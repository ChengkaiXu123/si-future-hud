//! Local build: other agents' words for a Cursor conversation. Cursor loads no
//! plugin SI Future HUD could write, so its own hooks take them
//! (`crate::hooks::relay`): a message waits in `<data>/cursor-channel/` and
//! goes in by itself as the conversation's next message when its turn ends,
//! or, while it is idle, with the next message the user sends there. Cursor
//! takes them once SI Future HUD's hooks are in its `hooks.json`, which the
//! "Client hooks" switch installs.
use crate::claude_plugin::{ClaudeChannel, Refusal};
use agent_hud_core::{
    l10n,
    replies::{SessionReplies, Unreachable},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

const SOURCE: &str = "cursor";
/// The largest hooks.json read; Cursor's own is a few hundred bytes.
const HOOKS_LIMIT: u64 = 1024 * 1024;

pub struct CursorReplies {
    channel: ClaudeChannel,
    /// Cursor's hooks settings, `~/.cursor/hooks.json`.
    hooks: PathBuf,
    /// The user's home, where Cursor writes its conversations' transcripts.
    home: PathBuf,
}

/// The carrier for Cursor's conversations, for the host's replies by source.
pub fn carrier(data: &Path, home: &Path) -> Arc<dyn SessionReplies> {
    let environment = std::env::vars().collect();
    let hooks = agent_hud_agents::hooks::hooks_by_id(SOURCE)
        .and_then(|hooks| hooks.settings)
        .and_then(|settings| {
            crate::os::resolve(home, &environment, &settings)
                .into_iter()
                .next()
        })
        .unwrap_or_else(|| home.join(".cursor/hooks.json"));
    Arc::new(CursorReplies::new(data, home.to_owned(), hooks))
}

/// A conversation's own id: the HUD lists it as `cursor-account:<account>:<id>`
/// (or `cursor:<id>`), and Cursor's hooks name it `<id>`.
pub fn own(session: &str) -> &str {
    session.rsplit(':').next().unwrap_or(session)
}

impl CursorReplies {
    pub fn new(data: &Path, home: PathBuf, hooks: PathBuf) -> Self {
        Self {
            channel: ClaudeChannel::new(crate::hooks::relay::channel_directory(data, SOURCE)),
            hooks,
            home,
        }
    }

    /// Whether Cursor runs SI Future HUD's completion hook, which takes the
    /// words when a turn ends.
    fn hooked(&self) -> bool {
        let Ok(metadata) = std::fs::metadata(&self.hooks) else {
            return false;
        };
        if metadata.len() > HOOKS_LIMIT {
            return false;
        }
        std::fs::read(&self.hooks)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|settings| {
                agent_hud_agents::hook_commands(SOURCE, "--completion-hook", &settings).ok()
            })
            .is_some_and(|commands| !commands.is_empty())
    }

    fn unhooked() -> Unreachable {
        Unreachable::new(
            l10n::text(
                "Cursor 还没装 SI Future HUD 的回调，收不到消息：在 SI Future HUD 设置 → 通用里打开「客户端回调」（已经开着的话，关掉再打开一次）。",
                "Cursor doesn't run SI Future HUD's hooks yet, so it can't take messages: turn on Client hooks in SI Future HUD → Settings → General (if it is on, turn it off and on again).",
            ),
            None,
        )
    }

    fn not_local() -> Unreachable {
        Unreachable::new(
            l10n::text(
                "这个 Cursor 会话不是这台 Mac 上 Cursor 里的对话（找不到它的对话记录）：HUD 只能把消息送进本机 Cursor 里的对话。",
                "This Cursor session isn't a conversation in Cursor on this Mac (its transcript isn't here): the HUD can only send words into Cursor's conversations on this Mac.",
            ),
            None,
        )
    }

    fn not_cursor() -> Unreachable {
        Unreachable::new(
            l10n::text(
                "这条路只能把消息送进 Cursor 的对话。",
                "This carrier only takes words into Cursor conversations.",
            ),
            None,
        )
    }
}

impl SessionReplies for CursorReplies {
    /// Cursor's questions are answered in its own window.
    fn reaches(&self, _source: &str, _session: &str) -> Result<(), Unreachable> {
        Err(Unreachable::new(
            l10n::text(
                "SI Future HUD 不能替你回答 Cursor 的提问：在 Cursor 里回答。",
                "SI Future HUD can't answer Cursor's questions: answer them in Cursor.",
            ),
            None,
        ))
    }
    fn reply(&self, source: &str, session: &str, _words: &str) -> Result<(), String> {
        self.reaches(source, session).map_err(|why| why.note)
    }
    /// A message reaches a conversation of Cursor's on this Mac (one with a
    /// transcript here, not a dashboard row) once Cursor runs SI Future
    /// HUD's hooks.
    fn relays_to(&self, source: &str, session: &str) -> Result<(), Unreachable> {
        if source != SOURCE {
            return Err(Self::not_cursor());
        }
        // Hooks first: without them no conversation takes words, which is
        // the reason to give whichever conversation is asked about first.
        if !self.hooked() {
            return Err(Self::unhooked());
        }
        let id = own(session);
        if !crate::claude_plugin::valid_name(id)
            || agent_hud_agents::cursor::live::transcript_for(&self.home, id).is_none()
        {
            return Err(Self::not_local());
        }
        Ok(())
    }
    fn relay(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
        self.relays_to(source, session).map_err(|why| why.note)?;
        let directory = self.channel.directory();
        crate::os::private_directory(directory)?;
        crate::os::private_directory(&directory.join("prompts"))?;
        self.channel
            .post_message(own(session), words, SystemTime::now())
            .map(drop)
            .map_err(|refusal| match refusal {
                Refusal::Rejected(_) => {
                    l10n::text("消息不能以 / 开头。", "A message can't start with /.")
                }
                _ => l10n::text(
                    "消息没能留给 Cursor。",
                    "The message could not be left for Cursor.",
                ),
            })
    }
    fn delivery(&self, source: &str, _session: &str, running: bool) -> Option<String> {
        (source == SOURCE).then(|| {
            if running {
                l10n::text(
                    "Cursor 的这个对话正在跑一轮：这一轮结束时，消息会作为它的下一条消息自动发进去。",
                    "This Cursor conversation is mid-turn: the message goes in by itself as its next message when the turn ends.",
                )
            } else {
                l10n::text(
                    "Cursor 的这个对话现在空闲：你下次在这个对话里发消息时，消息会一起带进去（一小时内有效）。",
                    "This Cursor conversation is idle: the message goes in with the next message you send there (within an hour).",
                )
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ID: &str = "0b0e4c2a-1111-4222-8333-444455556666";
    const LISTED: &str = "cursor-account:acct1:0b0e4c2a-1111-4222-8333-444455556666";

    /// A home with Cursor's transcript for ID, and its hooks.json.
    fn cursor_home(hooks: Option<Value>) -> (tempfile::TempDir, CursorReplies) {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let transcript = home
            .join(".cursor/projects/Users-me-repo/agent-transcripts")
            .join(ID);
        std::fs::create_dir_all(&transcript).unwrap();
        std::fs::write(transcript.join(format!("{ID}.jsonl")), "{}\n").unwrap();
        let file = home.join(".cursor/hooks.json");
        if let Some(hooks) = hooks {
            std::fs::write(&file, hooks.to_string()).unwrap();
        }
        let replies = CursorReplies::new(&root.path().join("data"), home, file);
        (root, replies)
    }

    fn installed() -> Value {
        json!({"version":1,"hooks":{"stop":[
            {"command":"git-ai stop"},
            {"command":"'/Applications/SI Future HUD.app/Contents/MacOS/agent-hud-next' --completion-hook cursor","timeout":5,"loop_limit":10}
        ]}})
    }

    #[test]
    fn words_wait_for_the_conversations_own_id_where_its_hooks_take_them() {
        let (root, replies) = cursor_home(Some(installed()));
        assert_eq!(replies.relays_to("cursor", LISTED), Ok(()));
        replies.relay("cursor", LISTED, "hello Cursor").unwrap();
        // The hook names the conversation by its own id.
        let channel = ClaudeChannel::new(root.path().join("data/cursor-channel"));
        assert_eq!(
            channel.take_messages(ID, SystemTime::now()),
            vec!["hello Cursor".to_owned()]
        );
        assert!(channel.take_messages(ID, SystemTime::now()).is_empty());
        // Words starting with a slash would run as a command.
        assert!(replies.relay("cursor", LISTED, "/clear").is_err());
    }

    #[test]
    fn without_the_hooks_or_a_local_conversation_words_dont_go() {
        let (_root, replies) = cursor_home(None);
        let why = replies.relays_to("cursor", LISTED).unwrap_err();
        assert!(why.note.contains("Client hooks") || why.note.contains("客户端回调"));
        assert!(replies.relay("cursor", LISTED, "hi").is_err());
        // Only another tool's hooks: still not.
        let (_root, replies) = cursor_home(Some(
            json!({"version":1,"hooks":{"stop":[{"command":"git-ai stop"}]}}),
        ));
        assert!(replies.relays_to("cursor", LISTED).is_err());
        // A dashboard row with no conversation on this Mac.
        let (_root, replies) = cursor_home(Some(installed()));
        let why = replies
            .relays_to("cursor", "cursor-account:acct1:unassigned-abc-0")
            .unwrap_err();
        assert!(why.note.contains("transcript") || why.note.contains("对话记录"));
        assert!(replies.relays_to("claude", LISTED).is_err());
        // Cursor's questions stay in Cursor.
        assert!(replies.reaches("cursor", LISTED).is_err());
    }

    #[test]
    fn the_card_says_when_the_words_go_in() {
        let (_root, replies) = cursor_home(Some(installed()));
        let running = replies.delivery("cursor", LISTED, true).unwrap();
        let idle = replies.delivery("cursor", LISTED, false).unwrap();
        assert_ne!(running, idle);
        assert_eq!(replies.delivery("claude", LISTED, true), None);
    }
}
