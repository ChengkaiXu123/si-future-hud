//! Local build: leaves words for a Codex thread as its user's next message.
//!
//! Codex Desktop 26.11 (now inside the ChatGPT app) dropped the window IPC
//! this used (`thread-follower-start-turn` over `<codex home>/ipc/ipc.sock`).
//! Words now go into the thread's durable queue with Codex's own CLI,
//! `codex queue --thread <id> --message <text>`. Desktop reads that queue
//! itself (measured 11-24 on 26.1117): a thread it has open runs them within
//! about half a minute, live in its window, after a turn still running; a
//! thread it hasn't open keeps them until the user opens it there.
//! Deliberately without `--resume`: that runs the thread on Codex's separate
//! app-server daemon, a second writer beside Desktop's own when Desktop has
//! the thread open (Desktop won't share the daemon: it always starts its own
//! app-server with config overrides).
use crate::os::Environment;
use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// How long queueing may take (well under a second, measured 0.171).
const SEND_WAIT: Duration = Duration::from_secs(20);

fn missing() -> String {
    agent_hud_core::l10n::text(
        "找不到 Codex：没有装 ChatGPT（含 Codex）或 Codex 命令行。",
        "Codex isn't installed: neither the ChatGPT app with Codex nor the Codex CLI was found.",
    )
}

/// Codex's CLI: the one inside the desktop app first, as the daemon it
/// manages must match the app.
fn cli() -> Option<PathBuf> {
    let home = crate::os::home_dir().ok()?;
    let env = std::env::vars().collect::<Environment>();
    crate::os::engine_candidates(&home, &env, "codex")
        .into_iter()
        .find(|path| crate::os::engine_executable(path))
}

/// Runs `cli args…` with CODEX_HOME at `codex_home`, within `wait`.
/// Ok((stdout, stderr)) when it exits 0; Err with its last words otherwise.
fn run(
    cli: &Path,
    codex_home: &Path,
    args: &[&str],
    wait: Duration,
) -> Result<(String, String), String> {
    let path = crate::os::home_dir()
        .map(|home| crate::os::engine_path(&home, &std::env::vars().collect::<Environment>()))
        .unwrap_or_default();
    let mut command = Command::new(cli);
    command
        .args(args)
        .env("CODEX_HOME", codex_home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !path.is_empty() {
        command.env("PATH", path);
    }
    crate::os::configure_command(&mut command);
    let mut child = command.spawn().map_err(|e| format!("{}: {e}", missing()))?;
    // Drained on their own threads, so a chatty CLI never blocks on a full pipe.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_string(&mut text);
            }
            text
        })
    };
    let out = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = Instant::now() + wait;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let (out, err) = (
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    );
    match status {
        Some(status) if status.success() => Ok((out, err)),
        Some(_) => Err(last_words(&err).unwrap_or_else(|| last_words(&out).unwrap_or_default())),
        None => Err(agent_hud_core::l10n::text(
            "Codex 没有及时回应",
            "Codex did not answer in time",
        )),
    }
}

/// The last line that says something, without the CLI's log noise.
fn last_words(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .rfind(|line| {
            !line.is_empty()
                && !line.starts_with('{')
                && !line.contains(" ERROR codex_core::session")
        })
        .map(str::to_owned)
}

/// Sends `text` into `thread`'s queue. Ok("start") when Codex says it is
/// queued; Ok("uncertain") when the CLI exited cleanly without saying so.
/// The queue waits behind a running turn, never cuts into it.
pub fn send(codex_home: &Path, thread: &str, text: &str) -> Result<&'static str, String> {
    let cli = cli().ok_or_else(missing)?;
    send_with(&cli, codex_home, thread, text)
}

fn send_with(
    cli: &Path,
    codex_home: &Path,
    thread: &str,
    text: &str,
) -> Result<&'static str, String> {
    let (out, _) = run(
        cli,
        codex_home,
        &["queue", "--thread", thread, "--message", text],
        SEND_WAIT,
    )
    .map_err(|reason| {
        if agent_hud_core::l10n::chinese() {
            format!("Codex 没有收下这条消息：{reason}")
        } else {
            format!("Codex didn't take the message: {reason}")
        }
    })?;
    Ok(if out.contains("Queued message") {
        "start"
    } else {
        "uncertain"
    })
}

/// Which of `threads` a Desktop window has open. Desktop 26.11 no longer
/// says, so: can't tell.
pub fn open_among(_codex_home: &Path, _threads: &[String]) -> Option<HashSet<String>> {
    None
}

/// Words for Codex threads: Codex must be installed; the thread's queue
/// takes them whether or not a window has it open.
pub struct CodexReplies {
    home: std::path::PathBuf,
}

impl CodexReplies {
    pub fn new(home: std::path::PathBuf) -> Self {
        Self { home }
    }
}

/// The carrier for Codex threads, for the host's replies by source.
/// Every host file offers it; this one has one.
pub fn carrier(
    home: std::path::PathBuf,
) -> Option<std::sync::Arc<dyn agent_hud_core::replies::SessionReplies>> {
    Some(std::sync::Arc::new(CodexReplies::new(home)))
}

impl agent_hud_core::replies::SessionReplies for CodexReplies {
    fn reaches(
        &self,
        _source: &str,
        _session: &str,
    ) -> Result<(), agent_hud_core::replies::Unreachable> {
        if cli().is_some() {
            Ok(())
        } else {
            Err(agent_hud_core::replies::Unreachable::new(missing(), None))
        }
    }
    fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
        self.relay(source, session, words)
    }
    fn relay(&self, _source: &str, session: &str, words: &str) -> Result<(), String> {
        send(&self.home, session, words).map(drop)
    }
    fn open_among(&self, _source: &str, sessions: &[String]) -> Option<HashSet<String>> {
        open_among(&self.home, sessions)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    /// A stand-in `codex` that logs its arguments (one per line, blank line
    /// between calls) and answers `queue` as `body` says.
    fn fake_codex(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("codex");
        let script = format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{log}\"; done\necho >> \"{log}\"\n\
             echo \"CODEX_HOME=$CODEX_HOME\" >> \"{log}\"\n\
             if [ \"$1\" = queue ]; then\n{body}\nfi\nexit 0\n",
            log = dir.join("log").display()
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn a_message_goes_into_the_threads_queue_only_never_resumed_on_another_app_server() {
        let dir = tempfile::tempdir().unwrap();
        let codex = fake_codex(dir.path(), "echo \"Queued message m1 for thread $3.\"");
        let home = dir.path().join("codex-home");
        assert_eq!(
            super::send_with(&codex, &home, "t1", "你好\n第二行"),
            Ok("start")
        );
        let log = std::fs::read_to_string(dir.path().join("log")).unwrap();
        // One call, and it is the queue: no daemon start, no --resume.
        assert!(
            log.starts_with("queue\n--thread\nt1\n--message\n你好\n第二行\n\n"),
            "{log}"
        );
        assert_eq!(log.matches("CODEX_HOME=").count(), 1, "{log}");
        assert!(
            !log.contains("--resume") && !log.contains("daemon"),
            "{log}"
        );
        assert!(
            log.contains(&format!("CODEX_HOME={}", home.display())),
            "{log}"
        );
    }

    #[test]
    fn a_clean_exit_without_the_receipt_is_uncertain_and_a_refusal_is_an_error_with_codexs_words() {
        let dir = tempfile::tempdir().unwrap();
        let codex = fake_codex(dir.path(), "echo 'something else'");
        assert_eq!(
            super::send_with(&codex, dir.path(), "t1", "hi"),
            Ok("uncertain")
        );

        let dir = tempfile::tempdir().unwrap();
        let codex = fake_codex(dir.path(), "echo 'Error: thread t9 not found' >&2\nexit 1");
        let refused = super::send_with(&codex, dir.path(), "t9", "hi").unwrap_err();
        assert!(refused.contains("thread t9 not found"), "{refused}");
    }

    /// Sends one message into a real Codex thread: run by hand with
    /// CODEX_TEST_THREAD set, as `cargo test -p agent-hud-platform --lib
    /// codex_message_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn codex_message_live() {
        let thread = std::env::var("CODEX_TEST_THREAD").expect("CODEX_TEST_THREAD");
        let text = std::env::var("CODEX_TEST_TEXT").unwrap_or_else(|_| "SI Future HUD test".into());
        let home = super::super::data_directory().unwrap();
        let result = super::send(&home, &thread, &text);
        println!("codex message: {result:?}");
        assert!(result.is_ok(), "{result:?}");
    }
}
