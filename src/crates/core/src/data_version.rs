//! The ledger's data version: the release whose reading rules its logs were
//! read by (`agent_hud_agents::DATA_VERSIONS`). A ledger with no logs is
//! indexed; one recorded under another version is indexed again. Either run
//! is reported with its progress in bytes until every log the retention
//! keeps has been read by this build's rules, and then this build's version
//! is recorded. What logs past the retention, or removed, counted stays.
use super::*;
use crate::ingestion::within_retention;
use rusqlite::OptionalExtension;

/// Why the ledger is being read through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Run {
    /// A ledger with no logs yet.
    First,
    /// A ledger read under another data version.
    Again,
}

impl Run {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Run::First => "first",
            Run::Again => "reindex",
        }
    }
}

/// The data version the ledger records; None before one was recorded.
pub(crate) fn recorded(db: &Connection) -> Option<String> {
    db.query_row(
        "SELECT value FROM ingestion_meta WHERE key='data_version'",
        [],
        |row| row.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub(crate) fn record(db: &Connection, version: &str) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO ingestion_meta(key,value) VALUES('data_version',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [version],
    )
    .map(|_| ())
}

impl AgentHudCore {
    /// After a collection pass: once a pass has listed every log since launch
    /// and none waits to be read (a log pausing after a failed read does not
    /// hold the run open), the ledger holds this build's data version.
    pub(crate) fn finish_indexing_run(&mut self) {
        if self.reads.listing.reconciled {
            self.listed_once = true;
        }
        if !self.listed_once || self.waiting_reads() > 0 {
            return;
        }
        let Some(db) = &self.database else {
            return;
        };
        if self.data_version.as_deref() != Some(DATA_VERSION) {
            if record(db, DATA_VERSION).is_err() {
                return;
            }
            self.data_version = Some(DATA_VERSION.into());
        }
        self.indexing_run = None;
    }
}

/// The run a launch begins with. A ledger from before data versions were
/// recorded holds the rules its checkpoints name: it is read again only when
/// a log the retention keeps was read by other rules.
pub(crate) fn run_at_launch(
    recorded: Option<&str>,
    logs: &HashMap<PathBuf, LogHead>,
    now_ms: i64,
) -> Option<Run> {
    match recorded {
        Some(version) if version == DATA_VERSION => None,
        Some(_) => Some(Run::Again),
        None if logs.is_empty() => Some(Run::First),
        None => logs
            .values()
            .any(|head| {
                !head.gone
                    && head.parser_version != PARSER_VERSION
                    && within_retention(
                        &FileStamp {
                            source: "",
                            length: head.length,
                            modified: head.modified,
                        },
                        now_ms,
                    )
            })
            .then_some(Run::Again),
    }
}

/// How much of the logs the retention keeps this build's rules have read,
/// in bytes (decoded text for compressed logs): read and in all. A source
/// database is counted by its logs' heads, not its bytes, so it is left out.
pub(crate) fn progress(
    files: &HashMap<PathBuf, FileStamp>,
    logs: &HashMap<PathBuf, LogHead>,
    now_ms: i64,
) -> (u64, u64) {
    files
        .iter()
        .filter(|(path, file)| {
            within_retention(file, now_ms) && !source_db::accepts(path, file.source)
        })
        .fold((0, 0), |(read, all), (path, file)| {
            let done = logs
                .get(path)
                .filter(|head| head.parser_version == PARSER_VERSION)
                .map_or(0, |head| head.offset.min(file.length));
            (read + done, all + file.length)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn head(version: u32, offset: u64, length: u64) -> LogHead {
        let mut log = Log::new(PathBuf::from("/l.jsonl"), "claude");
        log.parser_version = version;
        log.offset = offset;
        log.length = length;
        log.modified = Some(SystemTime::now());
        LogHead::from_log(&log)
    }

    #[test]
    fn a_launch_indexes_a_new_ledger_and_reads_another_versions_again() {
        let now = Utc::now().timestamp_millis();
        let none = HashMap::new();
        let current = HashMap::from([(PathBuf::from("/a"), head(PARSER_VERSION, 10, 10))]);
        let older = HashMap::from([(PathBuf::from("/a"), head(PARSER_VERSION - 1, 10, 10))]);
        assert_eq!(run_at_launch(None, &none, now), Some(Run::First));
        assert_eq!(run_at_launch(Some(DATA_VERSION), &older, now), None);
        assert_eq!(
            run_at_launch(Some("0.5.0"), &current, now),
            Some(Run::Again)
        );
        assert_eq!(
            run_at_launch(None, &current, now),
            None,
            "a ledger from before versions were recorded holds this build's rules"
        );
        assert_eq!(run_at_launch(None, &older, now), Some(Run::Again));
        let mut old = head(PARSER_VERSION - 1, 10, 10);
        old.modified = Some(SystemTime::now() - Duration::from_secs(40 * 86_400));
        assert_eq!(
            run_at_launch(None, &HashMap::from([(PathBuf::from("/a"), old)]), now),
            None,
            "a log past the retention is not read again"
        );
    }

    #[test]
    fn progress_counts_the_bytes_this_builds_rules_read() {
        let now = Utc::now().timestamp_millis();
        let stamp = |length| FileStamp {
            source: "claude",
            length,
            modified: Some(SystemTime::now()),
        };
        let files = HashMap::from([
            (PathBuf::from("/done"), stamp(100)),
            (PathBuf::from("/half"), stamp(200)),
            (PathBuf::from("/older"), stamp(300)),
            (PathBuf::from("/new"), stamp(400)),
            (
                PathBuf::from("/past"),
                FileStamp {
                    source: "claude",
                    length: 500,
                    modified: Some(SystemTime::now() - Duration::from_secs(40 * 86_400)),
                },
            ),
        ]);
        let logs = HashMap::from([
            (PathBuf::from("/done"), head(PARSER_VERSION, 100, 100)),
            (PathBuf::from("/half"), head(PARSER_VERSION, 50, 200)),
            (PathBuf::from("/older"), head(PARSER_VERSION - 1, 300, 300)),
        ]);
        assert_eq!(progress(&files, &logs, now), (150, 1_000));
    }
}
