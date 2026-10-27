//! Antigravity SQLite session & usage reader.
use crate::{staging, Usage};
use agent_hud_agents::antigravity::proto as antigravity_proto;
use agent_hud_agents::transcript::records::{Page, Record};
use agent_hud_agents::transcript::sql_tables;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{fs, path::Path};

const PAGE: i64 = 500;

fn unique_time(
    db: &Connection,
    path: &str,
    generation: &str,
    t: &antigravity_proto::Turn,
) -> rusqlite::Result<Option<i64>> {
    let message = t.usage.as_ref().and_then(|u| u.message.as_deref());
    let (key, value) = if let Some(m) = message {
        ("message", m)
    } else if let Some(e) = t.execution.as_deref() {
        ("execution", e)
    } else {
        return Ok(None);
    };
    staging::proto_unique_step_time(
        db,
        path,
        generation,
        key,
        value,
        message.is_some(),
        t.execution.as_deref(),
    )
}

fn title(path: &Path) -> Option<String> {
    let file = path
        .parent()?
        .parent()?
        .join("annotations")
        .join(format!("{}.pbtxt", path.file_stem()?.to_str()?));
    if fs::metadata(&file).ok()?.len() > 65536 {
        return None;
    }
    let text = fs::read_to_string(file).ok()?;
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(p) = text[at..].find("title") {
        let start = at + p;
        at = start + 5;
        if start > 0 && !bytes[start - 1].is_ascii_whitespace() && bytes[start - 1] != b'{' {
            continue;
        }
        let Some(rest) = text[at..]
            .trim_start()
            .strip_prefix(':')
            .map(str::trim_start)
        else {
            continue;
        };
        let quote = rest.as_bytes().first().copied()?;
        if quote != b'\'' && quote != b'"' {
            continue;
        }
        // Text-format escapes are bytes: "\344\275\240" is the UTF-8 of 你.
        let mut output = Vec::<u8>::new();
        let mut chars = rest[1..].chars();
        while let Some(c) = chars.next() {
            if c == quote as char {
                return String::from_utf8(output).ok();
            }
            if c != '\\' {
                output.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                continue;
            }
            let c = chars.next()?;
            match c {
                'n' => output.push(b'\n'),
                'r' => output.push(b'\r'),
                't' => output.push(b'\t'),
                'x' => {
                    let h: String = chars.by_ref().take(2).collect();
                    output.push(u8::from_str_radix(&h, 16).ok()?);
                }
                'u' | 'U' => {
                    let h: String = chars.by_ref().take(if c == 'u' { 4 } else { 8 }).collect();
                    let scalar = char::from_u32(u32::from_str_radix(&h, 16).ok()?)?;
                    output.extend_from_slice(scalar.encode_utf8(&mut [0; 4]).as_bytes());
                }
                '0'..='7' => {
                    let mut h = String::from(c);
                    for _ in 0..2 {
                        if chars.clone().next().is_some_and(|c| matches!(c, '0'..='7')) {
                            h.push(chars.next()?)
                        } else {
                            break;
                        }
                    }
                    output.push(u8::from_str_radix(&h, 8).ok()?);
                }
                _ => output.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
            }
        }
    }
    None
}

pub(crate) fn read(
    client: &Connection,
    ledger: &Connection,
    path: &Path,
    generation: &str,
    cursor: &Value,
) -> rusqlite::Result<Page> {
    let path_text = path.to_string_lossy();
    let stage = cursor["stage"].as_i64().unwrap_or(0);
    let after = cursor["after"].as_i64().unwrap_or(i64::MIN);
    if cursor.is_null() {
        staging::clear_stale_proto_identity(ledger, &path_text, generation)?;
    }
    if stage == 1 && !sql_tables(client)?.contains("steps") {
        return Ok(Page {
            records: vec![],
            cursor: Some(json!({"stage":2})),
            watermark: None,
        });
    }
    let sql = if stage == 1 {
        "SELECT rowid,metadata FROM steps NOT INDEXED WHERE rowid>?1 ORDER BY rowid LIMIT ?2"
    } else {
        "SELECT idx,data FROM gen_metadata NOT INDEXED WHERE idx>?1 ORDER BY idx LIMIT ?2"
    };
    let mut stmt = client.prepare(sql)?;
    let mut rows = stmt.query(params![after, PAGE])?;
    let mut seen = 0;
    let mut last = after;
    let mut records = vec![];
    let raw = path.file_stem().unwrap_or_default().to_string_lossy();
    let session = format!("antigravity:{raw}");
    let caption = title(path);
    let fallback = {
        format!(
            "{} · {}",
            if path
                .components()
                .any(|c| c.as_os_str() == "antigravity-cli")
            {
                "Antigravity CLI"
            } else {
                "Antigravity"
            },
            raw.chars().take(8).collect::<String>()
        )
    };
    while let Some(row) = rows.next()? {
        seen += 1;
        last = row.get(0)?;
        let blob = row.get_ref(1)?;
        let rusqlite::types::ValueRef::Blob(blob) = blob else {
            continue;
        };
        if blob.len() > 1024 * 1024 {
            continue;
        }
        if stage == 1 {
            if let Some(s) = antigravity_proto::step(blob) {
                staging::upsert_proto_identity(
                    ledger,
                    &path_text,
                    generation,
                    1,
                    last,
                    s.message.as_deref(),
                    s.execution.as_deref(),
                    s.at,
                    None,
                    None,
                    None,
                )?;
            }
            continue;
        }
        let Some(turn) = antigravity_proto::turn(blob) else {
            continue;
        };
        if stage == 0 {
            staging::upsert_proto_identity(
                ledger,
                &path_text,
                generation,
                0,
                last,
                turn.usage.as_ref().and_then(|u| u.message.as_deref()),
                turn.execution.as_deref(),
                turn.at,
                turn.label.as_deref(),
                turn.model.as_deref(),
                turn.usage.as_ref().and_then(|u| u.response_id.as_deref()),
            )?;
            continue;
        }
        let Some(u) = turn.usage.as_ref() else {
            continue;
        };
        if u.input == 0
            && u.output == 0
            && u.read == 0
            && u.write == 0
            && u.thinking == 0
            && u.response == 0
        {
            continue;
        }
        let Some(at) = turn
            .at
            .or(unique_time(ledger, &path_text, generation, &turn)?)
        else {
            continue;
        };
        if let Some(response) = u.response_id.as_deref() {
            let first =
                staging::proto_first_response_idx(ledger, &path_text, generation, response)?;
            if first != last {
                continue;
            }
        }
        let mapped = if let Some(label) = turn.label.as_deref() {
            staging::proto_unique_label_model(ledger, &path_text, generation, label)?
        } else {
            None
        };
        let model = turn
            .model
            .clone()
            .or(mapped)
            .or(turn.label.clone())
            .or(u
                .model
                .or(turn.requested)
                .filter(|m| *m > 0)
                .map(|m| format!("Antigravity model {m}")))
            .unwrap_or_else(|| "Unknown".into());
        records.push(Record {
            id: session.clone(),
            title: caption.clone(),
            title_fallback: Some(fallback.clone()),
            cwd: None,
            model,
            key: format!(
                "{session}:{}",
                u.response_id
                    .clone()
                    .unwrap_or_else(|| format!("row-{last}"))
            ),
            at,
            usage: Usage {
                input: u.input,
                output: u.output.saturating_sub(u.thinking),
                read: u.read,
                write: u.write,
                reasoning: u.thinking.min(u.output),
            },
            parent: None,
            archived: false,
            updated: None,
            duration_ms: None,
        });
    }
    let cursor = if seen == PAGE {
        Some(json!({"stage":stage,"after":last}))
    } else if stage < 2 {
        Some(json!({"stage":stage+1}))
    } else {
        None
    };
    Ok(Page {
        records,
        cursor,
        watermark: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_titles_decode_escaped_utf8_and_skip_other_title_words() {
        let temp = tempfile::tempdir().unwrap();
        let conversations = temp.path().join("conversations");
        fs::create_dir_all(temp.path().join("annotations")).unwrap();
        fs::create_dir_all(&conversations).unwrap();
        fs::write(
            temp.path().join("annotations/c1.pbtxt"),
            "subtitle_hint { }\ntitle \"x\"\ntitle: \"\\344\\275\\240\\345\\245\\275 \\xe4\\xb8\\x96\\u754c\"\n",
        )
        .unwrap();
        assert_eq!(
            title(&conversations.join("c1.db")).as_deref(),
            Some("你好 世界")
        );
    }
}
