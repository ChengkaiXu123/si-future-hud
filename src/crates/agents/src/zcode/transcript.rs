//! ZCode SQLite session reader (`db.sqlite`).
use crate::transcript::{sql_column, sql_columns, sql_count, sql_model, sql_text, sql_time};
use crate::{
    transcript::records::{Page, Record},
    Usage,
};
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn read(db: &Connection, cursor: &Value) -> rusqlite::Result<Page> {
    let usage = sql_columns(db, "model_usage")?;
    if ![
        "id",
        "session_id",
        "model_id",
        "started_at",
        "input_tokens",
        "output_tokens",
    ]
    .iter()
    .all(|key| usage.contains(*key))
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let s = sql_columns(db, "session")?;
    let join = s.contains("id");
    let metadata = ["directory", "path", "title"].map(|key| {
        if join {
            sql_column(&s, "s", key, "NULL")
        } else {
            "NULL".into()
        }
    });
    let fields = [
        "completed_at",
        "reasoning_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
        "computed_total_tokens",
    ]
    .map(|key| sql_column(&usage, "mu", key, "NULL"));
    let after = cursor["after"]
        .as_i64()
        .or_else(|| cursor.as_i64())
        .unwrap_or(0);
    let sql = format!("SELECT mu.rowid,mu.id,mu.session_id,mu.model_id,mu.started_at,{},mu.input_tokens,mu.output_tokens,{},{},{},{},{},{},{} FROM model_usage mu {} WHERE mu.rowid>?1 ORDER BY mu.rowid LIMIT 1000",fields[0],fields[1],fields[2],fields[3],fields[4],metadata[0],metadata[1],metadata[2],if join{"LEFT JOIN session s ON s.id=mu.session_id"}else{""});
    let mut statement = db.prepare(&sql)?;
    let mut rows = statement.query([after])?;
    let mut records = Vec::new();
    let mut last = after;
    let mut seen = 0;
    while let Some(row) = rows.next()? {
        seen += 1;
        last = row.get(0)?;
        let Some(at) = sql_time(row, 5, false).or_else(|| sql_time(row, 4, false)) else {
            continue;
        };
        let input = sql_count(row, 6)?;
        let output = sql_count(row, 7)?;
        let reasoning = sql_count(row, 8)?;
        let read = sql_count(row, 9)?;
        let write = sql_count(row, 10)?;
        let total = row.get_ref(11)?;
        let inclusive = if total == rusqlite::types::ValueRef::Null {
            !usage.contains("computed_total_tokens")
        } else {
            sql_count(row, 11)? == input.saturating_add(output)
        };
        if inclusive && read > input {
            continue;
        }
        let fresh = if inclusive {
            input - read
        } else {
            input.saturating_add(write)
        };
        let out = if inclusive {
            output
        } else {
            output.saturating_add(reasoning)
        };
        let write = write.min(fresh);
        let reasoning = reasoning.min(out);
        let raw = sql_text(row, 2).unwrap_or_else(|| "unknown".into());
        records.push(Record {
            id: format!("zcode:{raw}"),
            title: sql_text(row, 14).filter(|s| s != "Untitled session"),
            title_fallback: None,
            cwd: sql_text(row, 12).or_else(|| sql_text(row, 13)),
            model: sql_model(sql_text(row, 3)),
            key: sql_text(row, 1).unwrap_or_else(|| format!("row-{last}")),
            at,
            usage: Usage {
                input: fresh - write,
                output: out - reasoning,
                read,
                write,
                reasoning,
            },
            parent: None,
            archived: false,
            updated: None,
            duration_ms: None,
        });
    }
    Ok(Page {
        records,
        cursor: (seen == 1000).then(|| json!({"after": last})),
        watermark: (seen < 1000).then(|| json!({"after": last})),
    })
}
