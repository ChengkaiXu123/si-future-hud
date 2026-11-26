//! Local build: SI Future HUD's messages through a client's own hooks
//! (`agent_hud_agents::hooks::Relay`), as Cursor's version-1 hooks take them.
//! Messages for a conversation wait in `<data>/<client>-channel/prompts/<id>/`
//! (the channel Claude Code's plugin shares, `ClaudeChannel`). They go in
//! when the conversation's turn ends, as the next message Cursor sends there
//! by itself (`stop` → `followup_message`), or with the user's next prompt
//! when it is idle (`beforeSubmitPrompt` → `additional_context`). The HUD's
//! send tool is told which conversation calls it (`preToolUse` →
//! `updated_input.from_session`). Each hook answers at once, and anything
//! unexpected gets the answer that changes nothing in Cursor.
use crate::claude_plugin::ClaudeChannel;
use agent_hud_agents::hooks::Relay;
use serde_json::{json, Value};
use std::{path::Path, time::SystemTime};

/// The folder a client's messages wait in, in the app's data folder.
pub fn channel_directory(data: &Path, source: &str) -> std::path::PathBuf {
    data.join(format!("{source}-channel"))
}

fn conversation(payload: &Value) -> Option<&str> {
    payload["conversation_id"]
        .as_str()
        .filter(|id| crate::claude_plugin::valid_name(id))
}

/// What waits for the payload's conversation, taken now, as one message.
fn take(data: &Path, source: &str, payload: &Value) -> Option<String> {
    let id = conversation(payload)?;
    let messages =
        ClaudeChannel::new(channel_directory(data, source)).take_messages(id, SystemTime::now());
    (!messages.is_empty()).then(|| messages.join("\n\n"))
}

/// The completion hook's follow-up: what waits for the conversation, sent
/// as its next message when a turn ended normally. A turn the user stopped,
/// or one past the follow-ups allowed in a row, leaves them for later.
pub fn followup(data: &Path, source: &str, relay: &Relay, payload: &Value) -> Option<Value> {
    if payload["hook_event_name"]
        .as_str()
        .is_some_and(|name| name != "stop")
        || payload["status"]
            .as_str()
            .is_some_and(|status| status != "completed")
        || payload["loop_count"].as_u64().unwrap_or(0) >= relay.loop_limit
    {
        return None;
    }
    take(data, source, payload).map(|words| json!({"followup_message": words}))
}

/// The prompt hook's answer: the prompt always goes; what waits for the
/// conversation rides along as context.
pub fn prompt(data: &Path, source: &str, payload: &Value) -> Value {
    match take(data, source, payload) {
        Some(words) => json!({"continue": true, "additional_context": words}),
        None => json!({"continue": true}),
    }
}

/// Whether the tool hook's `tool_name` is the HUD's send tool: Cursor names
/// an MCP tool `MCP:<tool>`, which may carry the HUD's server name in front.
fn sends(relay: &Relay, name: &str) -> bool {
    let name = name.strip_prefix("MCP:").unwrap_or(name);
    name == relay.tool
        || ["agent-hud", "agent_hud"].iter().any(|server| {
            ["__", "-", ":", "."].iter().any(|join| {
                name.strip_suffix(relay.tool)
                    .and_then(|front| front.strip_suffix(join))
                    .is_some_and(|front| {
                        front == *server || front.ends_with(&format!("__{server}"))
                    })
            })
        })
}

/// The tool hook's answer: the HUD's send tool is let through, told which
/// conversation sends (whatever the call said); any other tool gets nothing,
/// which decides nothing.
pub fn tool(relay: &Relay, payload: &Value) -> Value {
    let Some(id) = conversation(payload) else {
        return json!({});
    };
    if !payload["tool_name"]
        .as_str()
        .is_some_and(|name| sends(relay, name))
    {
        return json!({});
    }
    // Cursor merges these keys into the call's arguments.
    let mut input = payload["tool_input"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    input.insert("from_session".into(), json!(id));
    json!({"permission": "allow", "updated_input": input})
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONVERSATION: &str = "5c1f2e0a-0000-4000-8000-00000000c0de";

    fn relay() -> &'static Relay {
        agent_hud_agents::hooks::hooks_by_id("cursor")
            .and_then(|hooks| hooks.relay.as_ref())
            .unwrap()
    }

    fn stop(loop_count: u64) -> Value {
        json!({"hook_event_name":"stop","status":"completed","loop_count":loop_count,"conversation_id":CONVERSATION,"generation_id":"g1"})
    }

    fn post(data: &Path, text: &str) {
        ClaudeChannel::new(channel_directory(data, "cursor"))
            .post_message(CONVERSATION, text, SystemTime::now())
            .unwrap();
    }

    #[test]
    fn a_turns_end_takes_what_waits_once_and_in_order() {
        let data = tempfile::tempdir().unwrap();
        assert_eq!(followup(data.path(), "cursor", relay(), &stop(0)), None);
        post(data.path(), "first");
        std::thread::sleep(std::time::Duration::from_millis(5));
        post(data.path(), "second");
        assert_eq!(
            followup(data.path(), "cursor", relay(), &stop(0)),
            Some(json!({"followup_message":"first\n\nsecond"}))
        );
        // Taken: never sent again, by either hook.
        assert_eq!(followup(data.path(), "cursor", relay(), &stop(1)), None);
        assert_eq!(
            prompt(
                data.path(),
                "cursor",
                &json!({"conversation_id":CONVERSATION})
            ),
            json!({"continue":true})
        );
        // Another conversation's messages stay for it.
        let other = json!({"hook_event_name":"stop","status":"completed","loop_count":0,"conversation_id":"another"});
        post(data.path(), "third");
        assert_eq!(followup(data.path(), "cursor", relay(), &other), None);
    }

    #[test]
    fn past_the_loop_limit_or_after_a_stopped_turn_messages_wait() {
        let data = tempfile::tempdir().unwrap();
        post(data.path(), "hello");
        assert_eq!(followup(data.path(), "cursor", relay(), &stop(10)), None);
        assert_eq!(followup(data.path(), "cursor", relay(), &stop(11)), None);
        let mut aborted = stop(0);
        aborted["status"] = json!("aborted");
        assert_eq!(followup(data.path(), "cursor", relay(), &aborted), None);
        // Still waiting: the next prompt takes it.
        assert_eq!(
            prompt(
                data.path(),
                "cursor",
                &json!({"prompt":"hi","conversation_id":CONVERSATION})
            ),
            json!({"continue":true,"additional_context":"hello"})
        );
        assert_eq!(
            prompt(
                data.path(),
                "cursor",
                &json!({"prompt":"hi","conversation_id":CONVERSATION})
            ),
            json!({"continue":true})
        );
        assert_eq!(followup(data.path(), "cursor", relay(), &stop(9)), None);
    }

    /// The hook as Cursor runs it: `<app> <flag> cursor`, JSON in, JSON out.
    fn run(data: &Path, flag: &str, payload: &Value) -> Value {
        let arguments = ["agent-hud-next", flag, "cursor"].map(str::to_owned);
        let mut output = Vec::new();
        let code = crate::hooks::handle_with_io(
            &arguments,
            data,
            &mut payload.to_string().as_bytes(),
            &mut output,
        );
        assert_eq!(code, Some(0));
        serde_json::from_slice(&output).unwrap()
    }

    #[test]
    fn cursors_hooks_answer_through_the_apps_own_entry() {
        let data = tempfile::tempdir().unwrap();
        // Nothing waits: the completion hook answers as before.
        assert_eq!(run(data.path(), "--completion-hook", &stop(0)), json!({}));
        post(data.path(), "from Claude");
        assert_eq!(
            run(data.path(), "--completion-hook", &stop(0)),
            json!({"followup_message":"from Claude"})
        );
        post(data.path(), "from Codex");
        assert_eq!(
            run(
                data.path(),
                "--relay-prompt-hook",
                &json!({"hook_event_name":"beforeSubmitPrompt","prompt":"go on","conversation_id":CONVERSATION})
            ),
            json!({"continue":true,"additional_context":"from Codex"})
        );
        assert_eq!(
            run(
                data.path(),
                "--relay-tool-hook",
                &json!({"hook_event_name":"preToolUse","tool_name":"MCP:hud_send_message","tool_input":{"to":"codex","text":"hi"},"conversation_id":CONVERSATION})
            ),
            json!({"permission":"allow","updated_input":{"to":"codex","text":"hi","from_session":CONVERSATION}})
        );
        assert_eq!(
            run(
                data.path(),
                "--relay-tool-hook",
                &json!({"hook_event_name":"preToolUse","tool_name":"Shell","tool_input":{"command":"ls"},"conversation_id":CONVERSATION})
            ),
            json!({})
        );
        // A client without the relay gets no answer from these flags.
        let mut output = Vec::new();
        let arguments = ["agent-hud-next", "--relay-prompt-hook", "claude"].map(str::to_owned);
        crate::hooks::handle_with_io(&arguments, data.path(), &mut b"{}".as_slice(), &mut output);
        assert!(output.is_empty());
    }

    #[test]
    fn a_prompt_with_nothing_waiting_or_no_conversation_just_goes() {
        let data = tempfile::tempdir().unwrap();
        assert_eq!(
            prompt(data.path(), "cursor", &json!({})),
            json!({"continue":true})
        );
        assert_eq!(
            prompt(
                data.path(),
                "cursor",
                &json!({"conversation_id":"../escape"})
            ),
            json!({"continue":true})
        );
    }

    #[test]
    fn only_the_send_tool_is_told_its_conversation() {
        let call = |name: &str, input: Value| {
            tool(
                relay(),
                &json!({"tool_name":name,"tool_input":input,"conversation_id":CONVERSATION,"tool_use_id":"t1"}),
            )
        };
        let input = json!({"to":"claude","text":"hi","from_session":"forged"});
        let expected = json!({"permission":"allow","updated_input":{"to":"claude","text":"hi","from_session":CONVERSATION}});
        assert_eq!(call("MCP:hud_send_message", input.clone()), expected);
        assert_eq!(
            call("MCP:agent-hud-hud_send_message", input.clone()),
            expected
        );
        assert_eq!(
            call("mcp__agent-hud__hud_send_message", input.clone()),
            expected
        );
        // Anything else decides nothing.
        for name in [
            "Shell",
            "MCP:hud_sessions",
            "MCP:other-hud_send_message",
            "MCP:not_hud_send_message",
        ] {
            assert_eq!(call(name, input.clone()), json!({}), "{name}");
        }
        // Without a conversation there is nothing to tell.
        assert_eq!(
            tool(
                relay(),
                &json!({"tool_name":"MCP:hud_send_message","tool_input":{}})
            ),
            json!({})
        );
        // Arguments that aren't an object still get the sender.
        assert_eq!(
            call("MCP:hud_send_message", json!("odd")),
            json!({"permission":"allow","updated_input":{"from_session":CONVERSATION}})
        );
    }
}
