//! What a log reports for the ledger: each call's usage (`Event`) and where
//! turns start or the client compacted (`Mark`).
use crate::Usage;
use chrono::DateTime;

/// Matches LedgerWriter's stable signed FNV-1a event identity.
pub fn event_key(value: &str) -> String {
    let hash = value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    (hash as i64).to_string()
}

#[derive(Clone, Default)]
pub struct Event {
    pub key: String,
    pub model: String,
    pub at: i64,
    pub usage: Usage,
    /// The context window the client reported for the call (Codex's
    /// `model_context_window`); None where the log does not say.
    pub context_window: Option<u64>,
    /// The tools the call asked for, in order; empty when it asked for none
    /// or its log names none (agent-hud-native-open `TurnCall.tools`).
    pub tools: Vec<String>,
    /// How long the call itself took, from its request to its last output,
    /// as the client's log records it; None where the log does not time the
    /// call. Tool runs and the person's time between calls are not in it.
    pub duration_ms: Option<u64>,
}

/// Where a session's turn starts, or where its client compacted the
/// conversation (agent-hud-native-open `UsageLedger.Mark`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkKind {
    Prompt = 0,
    Compaction = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    pub at: i64,
    pub kind: MarkKind,
}

pub fn date_ms(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.timestamp_millis())
}
