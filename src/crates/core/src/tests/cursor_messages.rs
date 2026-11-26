//! Local build: messages to and from Cursor conversations, which the HUD lists
//! under the dashboard's account and Cursor's hooks name by their own id.
use super::*;
use std::sync::{Arc, Mutex};

const OLDER: &str = "0aaaaaaa-1111-4222-8333-000000000001";
const NEWER: &str = "0aaaaaaa-1111-4222-8333-000000000002";

fn listed(id: &str) -> String {
    format!("cursor-account:acct1:{id}")
}

/// Carries words for Cursor (and Claude) and records them; tells when they go in.
#[derive(Default)]
struct Fake {
    sent: Mutex<Vec<(String, String, String)>>,
}
impl crate::replies::SessionReplies for Fake {
    fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
        Ok(())
    }
    fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
        self.sent
            .lock()
            .unwrap()
            .push((source.into(), session.into(), words.into()));
        Ok(())
    }
    fn delivery(&self, source: &str, _: &str, running: bool) -> Option<String> {
        (source == "cursor").then(|| format!("[delivery running={running}]"))
    }
}

/// Two Cursor conversations, NEWER the most recently active.
fn cursor_backend(root: &Path) -> (AgentHudCore, Arc<Fake>) {
    let mut backend = AgentHudCore::new(vec![], root.join("hud"));
    let now = chrono::Utc::now().timestamp_millis();
    let row = |id: &str, title: &str, ago: i64| {
        json!({
            "source": "cursor", "id": listed(id), "title": title,
            "model": "claude-4.5-sonnet", "key": format!("{id}:0"), "at": now - ago,
            "input": 100, "output": 10, "read": 0, "write": 0,
            "parent": null, "archived": false,
        })
    };
    backend.set_account_projection(AccountProjection {
        session_records: vec![
            row(OLDER, "Older chat", 60_000),
            row(NEWER, "Newer chat", 1_000),
        ],
        ..Default::default()
    });
    let fake = Arc::new(Fake::default());
    backend.set_session_replies(Some(fake.clone()));
    backend.request("snapshot", json!({})).unwrap();
    (backend, fake)
}

fn send(backend: &mut AgentHudCore, params: Value) -> Result<Value, String> {
    let proposed = backend.request("agent_message", params)?;
    backend
        .request(
            "answer_request",
            json!({"id":proposed["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    Ok(proposed)
}

#[test]
fn a_message_for_cursor_goes_to_its_latest_conversation_and_says_when_it_goes_in() {
    let root = tempfile::tempdir().unwrap();
    let (mut backend, fake) = cursor_backend(root.path());
    let proposed = send(
        &mut backend,
        json!({"to":"Cursor","text":"hello Cursor","caller":{"client":"claude","sessionId":"c1"}}),
    )
    .unwrap();
    assert_eq!(proposed["to"]["agent"], "cursor");
    assert_eq!(proposed["to"]["session"], listed(NEWER));
    // The card and the sender hear when it goes in (an idle conversation).
    assert!(
        proposed["note"]
            .as_str()
            .unwrap()
            .contains("[delivery running=false]"),
        "{}",
        proposed["note"]
    );
    let sent = fake.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        (sent[0].0.as_str(), sent[0].1.as_str()),
        ("cursor", listed(NEWER).as_str())
    );
    assert!(sent[0].2.contains("hello Cursor"));
    // A conversation named by its own id, as Cursor's hooks name it, is the listed one.
    let named = send(
        &mut backend,
        json!({"to":"cursor","session":OLDER,"text":"to the older one","caller":{"client":"claude","sessionId":"c1"}}),
    )
    .unwrap();
    assert_eq!(named["to"]["session"], listed(OLDER));
}

#[test]
fn a_cursor_sender_is_known_by_its_hook_and_gets_replies_to_its_own_conversation() {
    let root = tempfile::tempdir().unwrap();
    let (mut backend, fake) = cursor_backend(root.path());
    // NEWER is the most recent, but it sends: the message goes to OLDER.
    let proposed = send(
        &mut backend,
        json!({"to":"cursor","text":"hi from a sibling","from_session":NEWER,"caller":{"client":"cursor","sessionId":null}}),
    )
    .unwrap();
    assert_eq!(proposed["to"]["session"], listed(OLDER));
    let words = fake.sent.lock().unwrap().last().unwrap().2.clone();
    // Verified: named by its title; the reply goes to its own id.
    assert!(words.contains("Newer chat"), "{words}");
    assert!(words.contains(&format!("\"{NEWER}\"")), "{words}");
    assert!(!words.contains("cursor-account:"), "{words}");
    // The relay may also name the session itself.
    let relayed = backend
        .request(
            "agent_message",
            json!({"to":"cursor","session":OLDER,"text":"again","caller":{"client":"cursor","sessionId":NEWER}}),
        )
        .unwrap();
    assert_eq!(relayed["status"], "pending");
    // Without the hook's word, Cursor can't say which conversation sends.
    let error = backend
        .request(
            "agent_message",
            json!({"to":"cursor","text":"who am I","caller":{"client":"cursor","sessionId":null}}),
        )
        .unwrap_err();
    assert!(
        error.contains("Client hooks") || error.contains("客户端回调"),
        "{error}"
    );
    // Another agent's caller can't borrow from_session.
    send(
        &mut backend,
        json!({"to":"cursor","session":OLDER,"text":"x","from_session":NEWER,"caller":{"client":"claude","sessionId":"c-unknown"}}),
    )
    .unwrap();
    let words = fake.sent.lock().unwrap().last().unwrap().2.clone();
    assert!(!words.contains(NEWER), "{words}");
}
