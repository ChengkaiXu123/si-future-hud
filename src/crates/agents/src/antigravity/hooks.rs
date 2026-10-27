//! Antigravity's completion hook, kept in SI Future HUD's own `agent-hud` entry.
//! Its permissions are observed natively (`permissions.rs`), not through hooks.
use crate::hooks::{Completed, Completion, Hooks, Layout, POSIX_ONLY};
use crate::text;
use crate::Location;
use serde_json::Value;

pub static HOOKS: Hooks = Hooks {
    settings: Some(Location::home(
        &["GEMINI_CLI_HOME"],
        &[".gemini"],
        "config/hooks.json",
    )),
    aliases: &[],
    layout: Layout::OwnedEntry,
    permission: None,
    completion: Some(Completion {
        event: "Stop",
        timeout: 5,
        read: completed,
        enrich: None,
        reply: Some(r#"{"decision":"stop"}"#),
    }),
    origin: None,
    attention: false,
    compaction: None,
    prefixed_sessions: true,
    launch: POSIX_ONLY,
};

/// A conversation that stopped without a tool call, fully idle and without
/// an error. Its turn is the callback's time.
fn completed(payload: &Value, now_ms: i64) -> Option<Completed> {
    if payload["terminationReason"] != "NO_TOOL_CALL"
        || payload["fullyIdle"] != true
        || payload["error"].as_str().is_some_and(|s| !s.is_empty())
    {
        return None;
    }
    let id = text(&payload["conversationId"])?;
    Some(Completed {
        session: id.to_owned(),
        turn: format!("stop-{now_ms}"),
        workspace: payload["workspacePaths"][0].as_str().map(str::to_owned),
        model: payload["modelName"].as_str().map(str::to_owned),
        transcript_path: None,
    })
}
