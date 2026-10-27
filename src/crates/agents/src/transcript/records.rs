//! What a database source yields: one record per call, a page at a time.
use crate::Usage;
use serde_json::Value;

pub struct Record {
    pub id: String,
    /// A name the client keeps for the session.
    pub title: Option<String>,
    /// The client's own last resort, such as "Hermes · telegram".
    pub title_fallback: Option<String>,
    pub cwd: Option<String>,
    pub model: String,
    pub key: String,
    pub at: i64,
    pub usage: Usage,
    pub parent: Option<String>,
    pub archived: bool,
    pub updated: Option<i64>,
    /// How long the call itself took, where the client records it
    /// (`metering::Event::duration_ms`).
    pub duration_ms: Option<u64>,
}

pub struct Page {
    pub records: Vec<Record>,
    pub cursor: Option<Value>,
    pub watermark: Option<Value>,
}
