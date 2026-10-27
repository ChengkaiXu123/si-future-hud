//! Devin SQLite session reader (`sessions.db`).
use crate::transcript::{sql_column, sql_columns, sql_text, sql_time, unsigned};
use crate::{
    transcript::metering,
    transcript::records::{Page, Record},
    Usage,
};
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn read(db: &Connection, cursor: &Value) -> rusqlite::Result<Page> {
    let s = sql_columns(db, "sessions")?;
    let n = sql_columns(db, "message_nodes")?;
    if !s.contains("id") || !n.contains("row_id") || !n.contains("chat_message") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let after = cursor["after"]
        .as_i64()
        .or_else(|| cursor.as_i64())
        .unwrap_or(0);
    let metadata = ["title", "working_directory"].map(|key| sql_column(&s, "s", key, "NULL"));
    let mut statement = db.prepare(&format!("SELECT n.row_id,n.session_id,n.created_at,CASE WHEN json_valid(n.chat_message) THEN json_extract(n.chat_message,'$.message_id','$.metadata.request_id','$.metadata.generation_model','$.metadata.created_at','$.metadata.metrics') END,{},{} FROM message_nodes n LEFT JOIN sessions s ON s.id=n.session_id WHERE n.row_id>?1 ORDER BY n.row_id LIMIT 1000",metadata[0],metadata[1]))?;
    let mut rows = statement.query([after])?;
    let mut records = Vec::new();
    let mut last = after;
    let mut seen = 0;
    while let Some(row) = rows.next()? {
        seen += 1;
        last = row.get(0)?;
        let Some(raw) = sql_text(row, 1) else {
            continue;
        };
        let Some(v) = sql_text(row, 3).and_then(|s| serde_json::from_str::<Value>(&s).ok()) else {
            continue;
        };
        let Some(fields) = v.as_array().filter(|v| v.len() >= 5) else {
            continue;
        };
        let metrics = &fields[4];
        if !metrics.is_object() {
            continue;
        }
        let at = fields[3]
            .as_str()
            .and_then(metering::date_ms)
            .or_else(|| sql_time(row, 2, true));
        let Some(at) = at else {
            continue;
        };
        let usage =
            if metrics["input_tokens"].is_null() && !metrics["total_prompt_tokens"].is_null() {
                let input = unsigned(&metrics["total_prompt_tokens"]);
                let read = unsigned(&metrics["total_cached_tokens"]);
                if read > input {
                    continue;
                }
                Usage {
                    input: input - read,
                    output: unsigned(&metrics["total_completion_tokens"]),
                    read,
                    ..Usage::default()
                }
            } else {
                Usage {
                    input: unsigned(&metrics["input_tokens"]),
                    output: unsigned(&metrics["output_tokens"]),
                    read: unsigned(&metrics["cache_read_tokens"]),
                    write: unsigned(&metrics["cache_creation_tokens"]),
                    reasoning: 0,
                }
            };
        let key = fields[1]
            .as_str()
            .or_else(|| fields[0].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("row-{last}"));
        records.push(Record {
            id: format!("devin:{raw}"),
            title: sql_text(row, 4),
            title_fallback: None,
            cwd: sql_text(row, 5),
            model: fields[2].as_str().unwrap_or("Unknown").into(),
            key,
            at,
            usage,
            parent: None,
            archived: false,
            updated: None,
            // The call's own time: its wait for the first token and its
            // output (`ttft_ms` + per-token time).
            duration_ms: metrics["total_time_ms"].as_u64().filter(|ms| *ms > 0),
        });
    }
    Ok(Page {
        records,
        cursor: (seen == 1000).then(|| json!({"after": last})),
        watermark: (seen < 1000).then(|| json!({"after": last})),
    })
}
