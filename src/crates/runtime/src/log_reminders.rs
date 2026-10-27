//! Mirrors public LogReminders: one immediate read, then three signature looks
//! 500ms apart. The existing service waits for next_look; no timer/collector is
//! created here, and an unchanged follow-up never asks Core to read a log.
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
const FOLLOW_UPS: u8 = 3;
const INTERVAL: Duration = Duration::from_millis(500);
#[derive(PartialEq)]
struct Signature {
    modified: SystemTime,
    size: u64,
}
fn signature(path: &Path) -> Option<Signature> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(Signature {
        modified: metadata.modified().ok()?,
        size: metadata.len(),
    })
}
struct FollowUp {
    signature: Option<Signature>,
    looks_left: u8,
    due: Instant,
}
#[derive(Default)]
pub(super) struct LogReminders {
    fresh: HashSet<PathBuf>,
    follow_ups: HashMap<PathBuf, FollowUp>,
}
impl LogReminders {
    pub fn remind(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.fresh.extend(
            paths
                .into_iter()
                .map(|path| path.canonicalize().unwrap_or(path)),
        );
    }
    pub fn next_look(&self) -> Option<Instant> {
        if !self.fresh.is_empty() {
            return Some(Instant::now());
        }
        self.follow_ups.values().map(|look| look.due).min()
    }
    pub fn due(&mut self, now: Instant) -> Vec<PathBuf> {
        let mut due = HashSet::new();
        self.follow_ups.retain(|path, look| {
            if look.due > now || self.fresh.contains(path) {
                return true;
            }
            let current = signature(path);
            if current != look.signature {
                due.insert(path.clone());
            }
            if look.looks_left <= 1 {
                return false;
            }
            look.signature = current;
            look.looks_left -= 1;
            look.due = now + INTERVAL;
            true
        });
        for path in self.fresh.drain() {
            self.follow_ups.insert(
                path.clone(),
                FollowUp {
                    signature: signature(&path),
                    looks_left: FOLLOW_UPS,
                    due: now + INTERVAL,
                },
            );
            due.insert(path);
        }
        due.into_iter().collect()
    }
}
