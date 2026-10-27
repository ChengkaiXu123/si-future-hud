//! Grok Bot roster & session reader.
use crate::grok::cache as grok_cache;
use crate::naming::session_title;
use crate::{
    transcript::records::{Page, Record},
    Usage,
};
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

pub fn accepts(path: &Path) -> bool {
    grok_cache::key(path)
        .is_some_and(|k| k.starts_with(grok_cache::PREFIX) && k.ends_with(".roster.last-roster"))
}

pub fn read(path: &Path, cursor: &Value) -> rusqlite::Result<Page> {
    let dir = path.parent().ok_or(rusqlite::Error::InvalidQuery)?;
    let Some(account) = grok_cache::account(dir) else {
        return Ok(Page {
            records: vec![],
            cursor: None,
            watermark: None,
        });
    };
    let key = grok_cache::roster_key(&account);
    if grok_cache::key(path).as_deref() != Some(key.as_str()) {
        return Ok(Page {
            records: vec![],
            cursor: None,
            watermark: None,
        });
    }
    let roster = grok_cache::value(dir, &key, 4).ok_or(rusqlite::Error::InvalidQuery)?;
    let rows = roster["rows"]
        .as_array()
        .ok_or(rusqlite::Error::InvalidQuery)?;
    let offset = cursor.as_u64().unwrap_or(0) as usize;
    let mut records = vec![];
    let mut seen = HashSet::new();
    for row in rows.iter().take(offset + 500).skip(offset) {
        let Some(raw) = row["id"].as_str().filter(|s| grok_cache::agent_id(s)) else {
            continue;
        };
        if !seen.insert(raw) {
            continue;
        }
        let at = row["lastActivityAt"]
            .as_i64()
            .or_else(|| row["updatedAt"].as_i64())
            .or_else(|| row["createdAt"].as_i64())
            .unwrap_or(0);
        records.push(Record {
            id: grok_cache::session_id(&account, raw),
            title: row["title"]
                .as_str()
                .filter(|title| session_title::named(Some(title)).is_some())
                .or_else(|| row["name"].as_str())
                .map(str::to_owned),
            title_fallback: None,
            cwd: None,
            model: "Unknown".into(),
            key: format!("roster:{raw}"),
            at,
            usage: Usage::default(),
            parent: None,
            archived: false,
            updated: None,
            duration_ms: None,
        });
    }
    if grok_cache::account(dir).as_deref() != Some(account.as_str()) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(Page {
        records,
        cursor: (rows.len() > offset + 500).then(|| json!(offset + 500)),
        watermark: None,
    })
}
