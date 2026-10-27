//! Legacy OpenCode message records; SQLite is preferred via `session_mapping`.
//! Only the metadata/counters are deserialized, never message bodies.
use crate::{
    transcript::records::{Page, Record},
    Usage,
};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    id: String,
    #[serde(rename = "sessionID")]
    session: String,
    role: Option<String>,
    #[serde(default)]
    time: Value,
    #[serde(default)]
    tokens: Value,
    #[serde(rename = "modelID")]
    model_id: Option<String>,
    #[serde(rename = "providerID")]
    provider: Option<String>,
    #[serde(default)]
    model: Value,
    #[serde(default)]
    path: Value,
    #[serde(default)]
    finish: Value,
}

pub fn accepts(path: &Path) -> bool {
    path.extension().is_some_and(|s| s == "json")
        && path.components().any(|p| p.as_os_str() == "message")
}

pub fn read(path: &Path) -> rusqlite::Result<Page> {
    let file = fs::File::open(path).map_err(|_| rusqlite::Error::InvalidQuery)?;
    if file
        .metadata()
        .map_err(|_| rusqlite::Error::InvalidQuery)?
        .len()
        > 2 * 1024 * 1024
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let v: Message = serde_json::from_reader(std::io::BufReader::new(file))
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    if v.role.as_deref() != Some("assistant") || !v.tokens.is_object() {
        return Ok(Page {
            records: vec![],
            cursor: None,
            watermark: None,
        });
    }
    let at = v.time["created"]
        .as_i64()
        .filter(|n| *n > 0)
        .ok_or(rusqlite::Error::InvalidQuery)?;
    let model = v
        .model_id
        .as_deref()
        .or_else(|| v.model["id"].as_str())
        .unwrap_or("Unknown");
    let provider = v
        .provider
        .as_deref()
        .or_else(|| v.model["providerID"].as_str())
        .unwrap_or("Unknown");
    let number = |s: &Value| s.as_u64().unwrap_or(0);
    let cwd = v.path["root"].as_str().map(str::to_owned);
    let record = Record {
        id: format!("opencode:{}", v.session),
        title: None,
        title_fallback: None,
        cwd,
        model: format!("{model}#{provider}"),
        key: format!("opencode:{}", v.id),
        at,
        usage: Usage {
            input: number(&v.tokens["input"]),
            output: number(&v.tokens["output"]),
            read: number(&v.tokens["cache"]["read"]),
            write: number(&v.tokens["cache"]["write"]),
            reasoning: number(&v.tokens["reasoning"]),
        },
        parent: None,
        archived: false,
        updated: v.time["completed"].as_i64(),
        duration_ms: super::transcript::reply_time(&v.finish, &v.time),
    };
    Ok(Page {
        records: vec![record],
        cursor: None,
        watermark: None,
    })
}
