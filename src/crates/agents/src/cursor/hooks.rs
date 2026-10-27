//! Cursor's completion hook, in its version-1 settings.
use crate::hooks::{Completed, Completion, Handler, Hooks, Layout, POSIX_ONLY};
use crate::text;
use crate::Location;
use serde_json::Value;

pub static HOOKS: Hooks = Hooks {
    settings: Some(Location::home(&[], &[".cursor"], "hooks.json")),
    aliases: &[],
    layout: Layout::Handlers(Handler {
        command_key: "command",
        typed: false,
        timeout_key: "timeout",
    }),
    permission: None,
    completion: Some(Completion {
        event: "stop",
        timeout: 5,
        read: completed,
        enrich: None,
        reply: None,
    }),
    origin: None,
    attention: false,
    compaction: None,
    prefixed_sessions: true,
    launch: POSIX_ONLY,
};

/// A completed generation: the conversation and the generation that ended.
fn completed(payload: &Value, _now_ms: i64) -> Option<Completed> {
    if payload["hook_event_name"] != "stop" || payload["status"] != "completed" {
        return None;
    }
    let (Some(id), Some(turn)) = (
        text(&payload["conversation_id"]),
        text(&payload["generation_id"]),
    ) else {
        return None;
    };
    Some(Completed {
        session: id.into(),
        turn: turn.into(),
        workspace: payload["workspace_roots"][0].as_str().map(str::to_owned),
        model: payload["model_id"]
            .as_str()
            .or(payload["model"].as_str())
            .map(str::to_owned),
        transcript_path: None,
    })
}
