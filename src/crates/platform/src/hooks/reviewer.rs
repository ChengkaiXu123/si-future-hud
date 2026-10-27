//! A permission policy is not a reviewer. Only an explicit user reviewer of
//! this exact turn authorizes holding a Codex hook for the HUD.
use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

pub(super) fn is_user(
    requests: &agent_hud_core::requests::Requests,
    source: &str,
    payload: &Value,
) -> bool {
    let Some(turn) = payload
        .get("turn_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    if let Some(session) = payload.get("session_id").and_then(Value::as_str) {
        if let Some(reviewer) = requests.reviewer(source, session, turn, super::now_ms()) {
            return reviewer.as_deref() == Some("user");
        }
    }
    recorded(payload).as_deref() == Some("user")
}

fn recorded(payload: &Value) -> Option<String> {
    let path = payload.get("transcript_path")?.as_str()?;
    let turn = payload.get("turn_id")?.as_str()?;
    let mut file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }
    if !crate::os::owned_by_user(std::path::Path::new(path), &metadata) {
        return None;
    }
    // Same bounded reverse scan as public ProviderFiles.linesFromEnd: read at
    // most 256 MiB/3 seconds, retaining no line larger than 16 MiB. A long
    // current turn may have much more than 4 MiB after its context record.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut position = metadata.len();
    let mut total = 0;
    let mut carry: Option<Vec<u8>> = Some(Vec::new());
    while position > 0 && total < 256 * 1024 * 1024 && std::time::Instant::now() < deadline {
        let length = position.min(256 * 1024) as usize;
        position -= length as u64;
        total += length;
        file.seek(SeekFrom::Start(position)).ok()?;
        let mut block = vec![0; length];
        file.read_exact(&mut block).ok()?;
        let mut end = block.len();
        for start in (0..block.len()).rev().filter(|i| block[*i] == b'\n') {
            if let Some(mut suffix) = carry.take() {
                if end - start - 1 + suffix.len() <= 16 * 1024 * 1024 {
                    let mut line = block[start + 1..end].to_vec();
                    line.append(&mut suffix);
                    if let Some(reviewer) = context(&line, turn) {
                        return reviewer;
                    }
                }
            }
            carry = Some(Vec::new());
            end = start;
        }
        carry = carry.and_then(|mut suffix| {
            if end + suffix.len() > 16 * 1024 * 1024 {
                return None;
            }
            let mut line = block[..end].to_vec();
            line.append(&mut suffix);
            Some(line)
        });
    }
    if position == 0 {
        if let Some(line) = carry {
            if let Some(reviewer) = context(&line, turn) {
                return reviewer;
            }
        }
    }
    None
}
fn context(line: &[u8], turn: &str) -> Option<Option<String>> {
    if !line.windows(12).any(|part| part == b"turn_context") {
        return None;
    }
    let record: Value = serde_json::from_slice(line).ok()?;
    if record["type"] == "turn_context"
        && record.pointer("/payload/turn_id").and_then(Value::as_str) == Some(turn)
    {
        Some(
            record
                .pointer("/payload/approvals_reviewer")
                .and_then(Value::as_str)
                .map(str::to_owned),
        )
    } else {
        None
    }
}
