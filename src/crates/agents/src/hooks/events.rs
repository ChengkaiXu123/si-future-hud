use super::{configuration::configuration, hooks_by_id, hooks_of, session_id};
use crate::{text, trimmed, validate_payload};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    pub source: String,
    pub session_id: String,
    pub at_ms: i64,
    pub kind: String,
    pub data: Value,
}
fn event(source: &str, session: &str, now: i64, kind: &str, data: Value) -> HookEvent {
    HookEvent {
        source: source.into(),
        session_id: session_id(source, session),
        at_ms: now,
        kind: kind.into(),
        data,
    }
}
/// A callback's time is supplied by its host. Origin carries no self-declared
/// host/terminal identity: verified process ancestry belongs to the OS adapter.
pub fn parse_event(
    source_name: &str,
    flag: &str,
    payload: &Value,
    now_ms: i64,
) -> Result<Option<HookEvent>, String> {
    let (source, hooks) = hooks_of(source_name)?;
    validate_payload(payload)?;
    if hooks.compaction.is_some()
        && matches!(flag, "--session-origin-hook" | "--compaction-hook")
        && payload["hook_event_name"] == "PreCompact"
    {
        let Some(session) = text(&payload["session_id"]) else {
            return Ok(None);
        };
        if payload["agent_id"].is_string() {
            return Ok(None);
        }
        return Ok(Some(event(
            source,
            session,
            now_ms,
            "compaction",
            json!({"trigger":payload["trigger"].as_str()}),
        )));
    }
    configuration(source, flag)?;
    match flag {
        "--attention-hook" => {
            let Some(session) = text(&payload["session_id"]) else {
                return Ok(None);
            };
            Ok(Some(event(
                source,
                session,
                now_ms,
                "attention",
                json!({"message":trimmed(&payload["message"])}),
            )))
        }
        "--session-origin-hook" => {
            if !matches!(
                payload["hook_event_name"].as_str(),
                Some("SessionStart" | "UserPromptSubmit")
            ) {
                return Ok(None);
            }
            let Some(session) = text(&payload["session_id"]) else {
                return Ok(None);
            };
            Ok(Some(event(
                source,
                session,
                now_ms,
                "origin",
                json!({"eventName":payload["hook_event_name"],"cwd":payload["cwd"].as_str()}),
            )))
        }
        "--compaction-hook" => Ok(None),
        "--completion-hook" => {
            let completion = hooks
                .completion
                .as_ref()
                .ok_or("This client has no completion hook format")?;
            let Some(done) = (completion.read)(payload, now_ms) else {
                return Ok(None);
            };
            let mut data =
                json!({"turnId":done.turn,"workspace":done.workspace,"model":done.model});
            // A transcript the platform may read to enrich the event, for a
            // client whose payload omits its final message.
            if let Some(path) = done.transcript_path {
                data["transcriptPath"] = path;
            }
            Ok(Some(event(
                source,
                &done.session,
                now_ms,
                "completion",
                data,
            )))
        }
        _ => Err("This flag is not a reporting hook".into()),
    }
}

/// Fills in a completion event's final message from the transcript's tail,
/// for a client whose completion payload omits it (CodeBuddy).
pub fn enrich_codebuddy_completion(event: &mut HookEvent, transcript_tail: &[u8]) -> bool {
    match hooks_by_id(&event.source)
        .and_then(|hooks| hooks.completion.as_ref())
        .and_then(|completion| completion.enrich)
    {
        Some(enrich) => enrich(event, transcript_tail),
        None => false,
    }
}
