//! Local build: leaves words for an open Codex Desktop thread as its user's
//! next message, through the desktop app's IPC (`desktop_io`): a new turn
//! (`thread-follower-start-turn`), only while the thread is idle.
use super::desktop_io::{request_id, DesktopIpc, IpcError};
use serde_json::json;
use std::{collections::HashSet, path::Path, time::Duration};

const START: &str = "thread-follower-start-turn";
/// How long to wait for the window that has a thread: one that has it
/// answers within a tenth of a second (measured on two Macs, 26.1016).
const OWNER_WAIT: Duration = Duration::from_secs(2);
/// How long the rest of a send may take.
const SEND_WAIT: Duration = Duration::from_secs(5);

fn failed(error: IpcError) -> String {
    error.to_string()
}

fn not_open() -> String {
    agent_hud_core::l10n::text(
        "Codex 桌面版没有打开这个对话：在 Codex 里打开它，再发一次。",
        "Codex Desktop doesn't have this conversation open: open it in Codex, then send again.",
    )
}

/// A connection to the desktop app and the client id it gave this one.
fn connect(codex_home: &Path) -> Result<(DesktopIpc, String), String> {
    let mut connection =
        DesktopIpc::connect(&codex_home.join("ipc/ipc.sock"), SEND_WAIT).map_err(failed)?;
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
    Ok((connection, client))
}

/// Which client of the desktop app owns `thread` now, if the app has it open.
/// A window that has it answers at once; for one no window has, Desktop
/// (26.1016) says so only after its own 10-second search, past `OWNER_WAIT`,
/// so a wait that runs out reads as not open too.
fn owner(connection: &mut DesktopIpc, client: &str, thread: &str) -> Result<String, String> {
    connection.renew(OWNER_WAIT);
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
            IpcError::Rejected(_) | IpcError::TimedOut => not_open(),
            other => failed(other),
        })?;
    discovery["handledByClientId"]
        .as_str()
        .filter(|owner| !owner.is_empty())
        .map(str::to_owned)
        .ok_or_else(not_open)
}

/// Sends `text` into `thread` as a new turn. Ok("start") when Codex confirms
/// the turn; Ok("uncertain") when the request went out but its receipt did
/// not come back readable (it may have arrived: never offered again). A turn
/// already running is refused, so relayed words never cut into a task.
pub fn send(codex_home: &Path, thread: &str, text: &str) -> Result<&'static str, String> {
    let (mut connection, client) = connect(codex_home)?;
    let owner = owner(&mut connection, &client, thread)?;
    connection.renew(SEND_WAIT);
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

/// Which of `threads` (the most wanted first) a Desktop window has open,
/// asked about all at once. It stops waiting once the first open one is
/// known, so a later one may be missing; None when none could be asked.
pub fn open_among(codex_home: &Path, threads: &[String]) -> Option<HashSet<String>> {
    let (answer, answers) = std::sync::mpsc::channel();
    for (index, thread) in threads.iter().enumerate() {
        let (answer, home, thread) = (answer.clone(), codex_home.to_owned(), thread.clone());
        std::thread::spawn(move || {
            let open = connect(&home)
                .map(|(mut connection, client)| owner(&mut connection, &client, &thread).is_ok());
            let _ = answer.send((index, open.ok()));
        });
    }
    drop(answer);
    let mut known = vec![None; threads.len()];
    let mut open = HashSet::new();
    let mut asked = false;
    for (index, found) in answers {
        asked |= found.is_some();
        known[index] = Some(found == Some(true));
        if found == Some(true) {
            open.insert(threads[index].clone());
        }
        // Settled once every one before the first open one said it isn't.
        if known.iter().find(|answer| **answer != Some(false)) == Some(&Some(true)) {
            return Some(open);
        }
    }
    asked.then_some(open)
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
    fn open_among(&self, _source: &str, sessions: &[String]) -> Option<HashSet<String>> {
        open_among(&self.home, sessions)
    }
}

#[cfg(test)]
mod tests {
    /// A Desktop whose windows have the threads in `open`. For another one it
    /// answers `closed` (an error), or nothing within the wait when None.
    fn desktop(open: &'static [&'static str], closed: Option<&'static str>) -> tempfile::TempDir {
        use serde_json::{json, Value};
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join("ipc")).unwrap();
        let listener =
            std::os::unix::net::UnixListener::bind(home.path().join("ipc/ipc.sock")).unwrap();
        std::thread::spawn(move || {
            for socket in listener.incoming() {
                let Ok(mut socket) = socket else { return };
                std::thread::spawn(move || {
                    let receive = |socket: &mut UnixStream| {
                        let mut header = [0; 4];
                        socket.read_exact(&mut header).ok()?;
                        let mut bytes = vec![0; u32::from_le_bytes(header) as usize];
                        socket.read_exact(&mut bytes).ok()?;
                        serde_json::from_slice::<Value>(&bytes).ok()
                    };
                    let answer = |socket: &mut UnixStream, message: Value| {
                        let bytes = serde_json::to_vec(&message).unwrap();
                        let _ = socket.write_all(&(bytes.len() as u32).to_le_bytes());
                        let _ = socket.write_all(&bytes);
                    };
                    let Some(initialize) = receive(&mut socket) else {
                        return;
                    };
                    answer(
                        &mut socket,
                        json!({"type":"response","requestId":initialize["requestId"],"resultType":"success","method":"initialize","result":{"clientId":"hud"}}),
                    );
                    let Some(discovery) = receive(&mut socket) else {
                        return;
                    };
                    let thread = discovery["params"]["conversationId"].as_str().unwrap_or("");
                    match (open.contains(&thread), closed) {
                        (true, _) => answer(
                            &mut socket,
                            json!({"type":"response","requestId":discovery["requestId"],"resultType":"success","method":"thread-owner-discovery","handledByClientId":"window","result":{}}),
                        ),
                        (false, Some(error)) => answer(
                            &mut socket,
                            json!({"type":"response","requestId":discovery["requestId"],"resultType":"error","error":error}),
                        ),
                        // Holds on until the HUD gives up and hangs up.
                        (false, None) => {
                            let _ = socket.read(&mut [0; 1]);
                        }
                    }
                });
            }
        });
        home
    }

    #[test]
    fn a_conversation_no_window_has_open_is_named_so_whether_desktop_says_it_or_lets_the_wait_run_out(
    ) {
        for closed in [Some("no-client-found"), None] {
            let home = desktop(&[], closed);
            assert_eq!(
                super::send(home.path(), "thread", "hello"),
                Err(super::not_open()),
                "{closed:?}"
            );
        }
    }

    #[test]
    fn the_open_conversations_are_asked_about_at_once_and_the_first_open_one_ends_the_wait() {
        use std::time::{Duration, Instant};
        let threads = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        // The first open one answers at once: no wait for the closed one after it.
        let home = desktop(&["a"], None);
        let started = Instant::now();
        let open = super::open_among(home.path(), &threads(&["a", "b"])).unwrap();
        assert!(open.contains("a") && !open.contains("b"), "{open:?}");
        assert!(started.elapsed() < Duration::from_secs(1));
        // A closed one before it is waited for, once, not one after another.
        let home = desktop(&["c"], None);
        let started = Instant::now();
        let open = super::open_among(home.path(), &threads(&["a", "b", "c"])).unwrap();
        assert_eq!(open, std::collections::HashSet::from(["c".to_owned()]));
        assert!(started.elapsed() < super::OWNER_WAIT + Duration::from_secs(1));
        // None open: an empty answer, not "can't tell".
        let home = desktop(&[], Some("no-client-found"));
        assert_eq!(
            super::open_among(home.path(), &threads(&["a"])),
            Some(Default::default())
        );
        // Desktop not running: can't tell.
        let away = tempfile::tempdir().unwrap();
        assert_eq!(super::open_among(away.path(), &threads(&["a"])), None);
    }

    /// Read-only: which of CODEX_TEST_THREADS (comma-separated, the most
    /// wanted first) a real Codex Desktop has open, and how long asking took.
    /// Run by hand: `cargo test -p agent-hud-platform --lib
    /// codex_open_among_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn codex_open_among_live() {
        let threads = std::env::var("CODEX_TEST_THREADS").expect("CODEX_TEST_THREADS");
        let threads = threads.split(',').map(str::to_owned).collect::<Vec<_>>();
        let home = super::super::data_directory().unwrap();
        let started = std::time::Instant::now();
        let open = super::open_among(&home, &threads);
        println!("open {open:?} in {:?}", started.elapsed());
    }

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
