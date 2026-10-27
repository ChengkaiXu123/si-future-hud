//! Claude Code background jobs (`claude --bg`): which sessions a job runs and
//! who else holds them, from Claude Code's own records: `jobs/<job>/state.json`
//! names the job's session, and every live Claude Code process writes
//! `sessions/<pid>.json` with its session and `kind` (`bg` for a job). A job
//! has no window to return to; returning to one moves it into the desktop app.
//! Only a collection pass reads them, like the desktop app's session index.
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Job records read in one pass, newest first; older jobs are left out.
const JOBS: usize = 256;

#[derive(Clone, Default)]
pub struct ClaudeJobs {
    /// Session → the job that ran it.
    jobs: HashMap<String, String>,
    /// Sessions a live desktop-app process holds: it took the job in.
    desktop: HashSet<String>,
    /// Sessions another live process holds, a terminal or an attached client.
    other: HashSet<String>,
}

/// Where a background job's session is, for its way back.
#[derive(Debug, PartialEq)]
pub enum JobPlace {
    /// Still the job's: returning moves it into the desktop app.
    Job(Value),
    /// The desktop app runs it now.
    Desktop,
    /// Another client runs it; hooks and the transcript say which.
    Elsewhere,
}

impl ClaudeJobs {
    pub fn read(provider_roots: &[(PathBuf, &'static str)]) -> Self {
        let mut configs = Vec::new();
        for (root, source) in provider_roots {
            if *source == "claude" {
                if let Some(config) = root.parent() {
                    if !configs.contains(&config.to_path_buf()) {
                        configs.push(config.to_path_buf());
                    }
                }
            }
        }
        let mut read = Self::default();
        for config in configs {
            read.read_config(&config);
        }
        read
    }
    fn read_config(&mut self, config: &Path) {
        let mut states = Vec::new();
        if let Ok(entries) = fs::read_dir(config.join("jobs")) {
            for entry in entries.flatten() {
                let Some(job) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if !job_name(&job) {
                    continue;
                }
                let path = entry.path().join("state.json");
                let Some(modified) = small_file(&path).and_then(|meta| meta.modified().ok()) else {
                    continue;
                };
                states.push((modified, job, path));
            }
        }
        states.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        for (_, job, path) in states.into_iter().take(JOBS) {
            let Some(state) = read_json(&path) else {
                continue;
            };
            if let Some(session) = state["sessionId"].as_str().filter(|s| uuid(s)) {
                self.jobs.entry(session.to_owned()).or_insert(job);
            }
        }
        if let Ok(entries) = fs::read_dir(config.join("sessions")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json")
                    || small_file(&path).is_none()
                {
                    continue;
                }
                let Some(process) = read_json(&path) else {
                    continue;
                };
                let Some(session) = process["sessionId"].as_str().filter(|s| uuid(s)) else {
                    continue;
                };
                // `bg` is the job itself; the background service's own
                // processes say `daemon` or `daemon-worker`.
                match (process["kind"].as_str(), process["entrypoint"].as_str()) {
                    (Some("bg" | "daemon" | "daemon-worker") | None, _) => {}
                    (_, Some("claude-desktop" | "claude-desktop-3p")) => {
                        self.desktop.insert(session.to_owned());
                    }
                    _ => {
                        self.other.insert(session.to_owned());
                    }
                }
            }
        }
    }
    /// Where the session of a background job is; none for a session no job ran.
    pub fn place(&self, session: &str) -> Option<JobPlace> {
        let job = self.jobs.get(session)?;
        Some(if self.other.contains(session) {
            JobPlace::Elsewhere
        } else if self.desktop.contains(session) {
            JobPlace::Desktop
        } else {
            JobPlace::Job(json!({"kind":"claude-background-job","id":session,"job":job}))
        })
    }
}

fn small_file(path: &Path) -> Option<fs::Metadata> {
    fs::symlink_metadata(path)
        .ok()
        .filter(|meta| meta.is_file() && meta.len() <= 1024 * 1024)
}
fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}
fn job_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, value: Value) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }

    #[test]
    fn job_records_name_their_session_and_its_holders() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join(".claude");
        let roots = vec![(config.join("projects"), "claude")];
        let id = "11111111-2222-4333-8444-555555555555";
        assert_eq!(ClaudeJobs::read(&roots).place(id), None);
        write(
            &config.join("jobs/abc123/state.json"),
            json!({"state":"done","sessionId":id}),
        );
        write(
            &config.join("jobs/bad name/state.json"),
            json!({"sessionId":"66666666-7777-4888-9999-aaaaaaaaaaaa"}),
        );
        write(
            &config.join("jobs/nosession/state.json"),
            json!({"state":"done"}),
        );
        let read = ClaudeJobs::read(&roots);
        assert_eq!(read.jobs.len(), 1);
        assert_eq!(
            read.place(id),
            Some(JobPlace::Job(
                json!({"kind":"claude-background-job","id":id,"job":"abc123"})
            ))
        );
        // The background service's own processes are the job's, not a client.
        write(
            &config.join("sessions/7.json"),
            json!({"pid":7,"sessionId":id,"kind":"daemon-worker"}),
        );
        assert!(matches!(
            ClaudeJobs::read(&roots).place(id),
            Some(JobPlace::Job(_))
        ));
    }
}
