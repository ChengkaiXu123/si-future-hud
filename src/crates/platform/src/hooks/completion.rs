use agent_hud_agents::HookEvent;
use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
/// The platform supplies only bounded local bytes; the client's descriptor
/// owns its final-message schema and completion identity (CodeBuddy's today).
pub(super) fn enrich(event: &mut HookEvent, payload: &Value) {
    let enriches = agent_hud_agents::hooks::hooks_by_id(&event.source)
        .and_then(|hooks| hooks.completion.as_ref())
        .is_some_and(|completion| completion.enrich.is_some());
    if !enriches || event.kind != "completion" {
        return;
    }
    let Some(path) = payload["transcript_path"].as_str() else {
        return;
    };
    let Ok(mut file) = File::open(path) else {
        return;
    };
    let Ok(metadata) = file.metadata() else {
        return;
    };
    if !metadata.is_file() {
        return;
    }
    if !crate::os::owned_by_user(std::path::Path::new(path), &metadata) {
        return;
    }
    let start = metadata.len().saturating_sub(1024 * 1024);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return;
    }
    let mut bytes = Vec::new();
    if file.take(1024 * 1024).read_to_end(&mut bytes).is_err() {
        return;
    }
    let tail = if start > 0 {
        bytes
            .iter()
            .position(|b| *b == b'\n')
            .map(|end| &bytes[end + 1..])
            .unwrap_or(&[])
    } else {
        &bytes
    };
    agent_hud_agents::enrich_codebuddy_completion(event, tail);
}
