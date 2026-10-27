//! Local build: leaves words for an open Codex Desktop thread as its user's
//! next message, through the desktop app's IPC (`desktop_io`): a new turn
//! (`thread-follower-start-turn`), only while the thread is idle.
use super::desktop_io::{request_id, DesktopIpc, IpcError};
use serde_json::json;
use std::{path::Path, time::Duration};

const START: &str = "thread-follower-start-turn";

fn failed(error: IpcError) -> String {
    error.to_string()
}

/// Which client of the desktop app owns `thread` now, if the app has it open.
fn owner(connection: &mut DesktopIpc, client: &str, thread: &str) -> Result<String, String> {
    let discovery = connection
        .request(
            "thread-owner-discovery",
            1,
            client,
            None,
            json!({"hostId":"local","conversationId":thread}),
            None,
        )
        .map_err(|error| match error {
            IpcError::Rejected(_) => agent_hud_core::l10n::text(
                "Codex 桌面版没有打开这个对话。",
                "Codex Desktop doesn't have this conversation open.",
            ),
            other => failed(other),
        })?;
    discovery["handledByClientId"]
        .as_str()
        .filter(|owner| !owner.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            agent_hud_core::l10n::text(
                "Codex 桌面版没有打开这个对话。",
                "Codex Desktop doesn't have this conversation open.",
            )
        })
}

/// Sends `text` into `thread` as a new turn. Ok("start") when Codex confirms
/// the turn; Ok("uncertain") when the request went out but its receipt did
/// not come back readable (it may have arrived: never offered again). A turn
/// already running is refused, so relayed words never cut into a task.
pub fn send(codex_home: &Path, thread: &str, text: &str) -> Result<&'static str, String> {
    let mut connection =
        DesktopIpc::connect(&codex_home.join("ipc/ipc.sock"), Duration::from_secs(5))
            .map_err(failed)?;
    let initialized = connection
        .request(
            "initialize",
            0,
            "initializing-client",
            None,
            json!({"clientType":"agent-hud"}),
            None,
        )
        .map_err(failed)?;
    let client = initialized["result"]["clientId"]
        .as_str()
        .ok_or_else(|| failed(IpcError::Protocol))?
        .to_owned();
    let owner = owner(&mut connection, &client, thread)?;
    let id = request_id().map_err(failed)?;
    let input = json!([{"type":"text","text":text,"text_elements":[]}]);
    let restore = json!({"id":id,"text":text,"context":{"prompt":text,"addedFiles":[],"fileAttachments":[],"ideContext":null,"imageAttachments":[]},"createdAt":chrono::Utc::now().timestamp_millis()});
    let mut wrote = false;
    let started = connection.request(
        START,
        2,
        &client,
        Some(&owner),
        json!({"conversationId":thread,"turnStart":{"request":{"threadId":thread,"input":input,"clientUserMessageId":id},"context":{"restoreMessage":restore,"attachments":[]}}}),
        Some(&mut || wrote = true),
    );
    match started {
        // The receipt names the method and the owner, and carries the turn it started.
        Ok(receipt)
            if receipt["method"] == START
                && receipt["handledByClientId"] == owner.as_str()
                && receipt["result"]["result"]["turn"]["id"].is_string() =>
        {
            Ok("start")
        }
        Ok(_) => Ok("uncertain"),
        Err(IpcError::Rejected(reason))
            if !matches!(
                reason.as_str(),
                "request-timeout" | "server-closed" | "client-disconnected"
            ) =>
        {
            Err(if agent_hud_core::l10n::chinese() {
                format!("Codex 没有接收这条消息（可能正在跑一轮，等它结束再点发送）：{reason}")
            } else {
                format!("Codex didn't take the message (it may be mid-turn; send again when it ends): {reason}")
            })
        }
        Err(_) if wrote => Ok("uncertain"),
        Err(other) => Err(failed(other)),
    }
}

/// Words for Codex Desktop threads: the app must be running (its IPC socket
/// is there); whether it has the thread open is known only when sending.
pub struct CodexReplies {
    home: std::path::PathBuf,
}

impl CodexReplies {
    pub fn new(home: std::path::PathBuf) -> Self {
        Self { home }
    }
}

/// The carrier for Codex Desktop threads, for the host's replies by source.
/// Every host file offers it; this one has one.
pub fn carrier(
    home: std::path::PathBuf,
) -> Option<std::sync::Arc<dyn agent_hud_core::replies::SessionReplies>> {
    Some(std::sync::Arc::new(CodexReplies::new(home)))
}

impl agent_hud_core::replies::SessionReplies for CodexReplies {
    fn reaches(
        &self,
        _source: &str,
        _session: &str,
    ) -> Result<(), agent_hud_core::replies::Unreachable> {
        if self.home.join("ipc/ipc.sock").exists() {
            Ok(())
        } else {
            Err(agent_hud_core::replies::Unreachable::new(
                agent_hud_core::l10n::text(
                    "Codex 桌面版没有在运行。",
                    "Codex Desktop isn't running.",
                ),
                None,
            ))
        }
    }
    fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
        self.relay(source, session, words)
    }
    fn relay(&self, _source: &str, session: &str, words: &str) -> Result<(), String> {
        send(&self.home, session, words).map(drop)
    }
}

#[cfg(test)]
mod tests {
    /// Sends one message into a real Codex Desktop thread: run by hand with
    /// CODEX_TEST_THREAD set, as `cargo test -p agent-hud-platform --lib
    /// codex_message_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn codex_message_live() {
        let thread = std::env::var("CODEX_TEST_THREAD").expect("CODEX_TEST_THREAD");
        let text = std::env::var("CODEX_TEST_TEXT").unwrap_or_else(|_| "SI Future HUD test".into());
        let home = super::super::data_directory().unwrap();
        let result = super::send(&home, &thread, &text);
        println!("codex message: {result:?}");
        assert!(result.is_ok(), "{result:?}");
    }

    /// Local build probe, read-only: follows one Codex Desktop thread as the
    /// HUD's stream does and writes every broadcast it gets (long strings cut)
    /// as JSON lines, to learn what Desktop sends while a thread compacts.
    /// Run by hand: CODEX_TEST_THREAD=<id> WATCH_SECS=600 WATCH_OUT=<file>
    /// cargo test -p agent-hud-platform --lib codex_stream_watch_live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn codex_stream_watch_live() {
        use super::super::desktop_io::DesktopIpc;
        use serde_json::json;
        use std::io::Write;
        use std::time::{Duration, Instant};
        let thread = std::env::var("CODEX_TEST_THREAD").expect("CODEX_TEST_THREAD");
        let seconds: u64 = std::env::var("WATCH_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);
        let mut out =
            std::fs::File::create(std::env::var("WATCH_OUT").expect("WATCH_OUT")).unwrap();
        let home = super::super::data_directory().unwrap();
        let mut connection =
            DesktopIpc::connect(&home.join("ipc/ipc.sock"), Duration::from_secs(5)).unwrap();
        let initialized = connection
            .request(
                "initialize",
                0,
                "initializing-client",
                None,
                json!({"clientType":"agent-hud-probe"}),
                None,
            )
            .unwrap();
        let client = initialized["result"]["clientId"]
            .as_str()
            .unwrap()
            .to_owned();
        let owner = super::owner(&mut connection, &client, &thread).unwrap();
        let follow = |connection: &mut DesktopIpc, following: bool| {
            connection.renew(Duration::from_secs(5));
            connection.post(json!({"type":"broadcast","method":"thread-stream-following-changed","version":1,"sourceClientId":client,"targetClientIds":[owner],"params":{"hostId":"local","conversationId":thread,"following":following}}))
        };
        follow(&mut connection, true).unwrap();
        println!("following {thread} via {owner} for {seconds}s");
        let end = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < end {
            if !connection
                .wait_for_message(Duration::from_millis(500))
                .unwrap_or(false)
            {
                continue;
            }
            connection.renew(Duration::from_secs(120));
            let Ok(message) = connection.receive_trimmed(1024 * 1024 * 1024, 2 * 1024) else {
                println!("receive failed; stopping");
                break;
            };
            connection.renew(Duration::from_secs(5));
            if connection.decline_discovery(&message).unwrap_or(false) {
                continue;
            }
            let at = chrono::Utc::now().to_rfc3339();
            writeln!(out, "{}", json!({"at": at, "message": message})).unwrap();
        }
        let _ = follow(&mut connection, false);
    }
}
