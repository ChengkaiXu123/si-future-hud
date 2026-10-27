//! Agent HUD's 程小帮 plugin: a mod (程小帮 loads Claude Code mods) in its
//! plugins folder, and the channel it shares with Agent HUD. The channel is
//! Claude's (`ClaudeChannel`): sessions check in, words wait in `prompts/`,
//! and a question the plugin holds takes its answer from `answers/`.
use crate::claude_plugin::{ClaudeChannel, Refusal};
use agent_hud_core::l10n;
use agent_hud_core::replies::{AnswerFix, SessionReplies, Unreachable};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

const SCRIPT: &str = include_str!("register.ts");
pub const PLUGIN_NAME: &str = "agent-hud";

/// 程小帮's folder, `~/.chengxiaobang`, where its plugins live.
pub fn chengxiaobang_directory(home: &Path) -> PathBuf {
    home.join(".chengxiaobang")
}

pub struct ChengxiaobangPlugin {
    channel: ClaudeChannel,
    /// `~/.chengxiaobang/plugins/agent-hud`.
    folder: PathBuf,
}

impl ChengxiaobangPlugin {
    pub fn new(channel: PathBuf, chengxiaobang: &Path) -> Self {
        Self {
            channel: ClaudeChannel::new(channel),
            folder: chengxiaobang.join("plugins").join(PLUGIN_NAME),
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Writes the plugin into 程小帮's plugins folder, pointing it at the
    /// channel, and makes the channel's folders, when 程小帮 is on this Mac.
    /// Only what differs is written. 程小帮 loads it once the user enables it
    /// on its Plugins page.
    pub fn prepare(&self) -> Result<bool, String> {
        let Some(plugins) = self.folder.parent() else {
            return Ok(false);
        };
        if !plugins.parent().is_some_and(Path::is_dir) {
            return Ok(false);
        }
        for name in ["", "sessions", "prompts", "answers"] {
            crate::os::private_directory(&self.channel.directory().join(name))?;
        }
        let channel = serde_json::to_string(&self.channel.directory().to_string_lossy())
            .map_err(|e| e.to_string())?;
        let files = [
            (
                ".claude-plugin/plugin.json",
                json!({"name":PLUGIN_NAME,"displayName":"SI Future HUD","version":"1.0.0","description":"Shows this session's questions in SI Future HUD and takes the answers given there.","author":{"name":"SI Future HUD"}}).to_string(),
            ),
            ("hooks/hooks.json", json!({"modules":["./register.ts"]}).to_string()),
            ("hooks/register.ts", SCRIPT.replace("__AGENT_HUD_CHANNEL_JSON__", &channel)),
        ];
        let mut changed = false;
        for (relative, text) in files {
            let file = self.folder.join(relative);
            if fs::read(&file).ok().as_deref() == Some(text.as_bytes()) {
                continue;
            }
            fs::create_dir_all(file.parent().ok_or("The plugin folder is unavailable")?)
                .map_err(|e| e.to_string())?;
            fs::write(&file, text.as_bytes()).map_err(|e| e.to_string())?;
            changed = true;
        }
        Ok(changed)
    }

    /// Lists Agent HUD's MCP server (`executable --mcp`) in 程小帮's
    /// `mcp.json`, as the HUD tools Claude Code and Codex are offered, keeping
    /// every other server as it is. Returns whether the file changed.
    pub fn ensure_mcp(&self, executable: &Path) -> Result<bool, String> {
        let Some(home) = self.folder.parent().and_then(Path::parent) else {
            return Ok(false);
        };
        if !home.is_dir() || !executable.is_file() {
            return Ok(false);
        }
        let file = home.join("mcp.json");
        let mut config: serde_json::Value = match fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|_| "程小帮's mcp.json is not valid JSON".to_owned())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => json!({}),
            Err(error) => return Err(error.to_string()),
        };
        if !config.is_object() {
            return Err("程小帮's mcp.json is not a JSON object".into());
        }
        // AGENT_HUD_CLIENT tells the HUD tools which agent calls them (as
        // Claude Code's session and Codex's thread do by themselves).
        let wanted = json!({"command": executable.to_string_lossy(), "args": ["--mcp"], "env": {"AGENT_HUD_CLIENT": "chengxiaobang"}});
        let servers = config
            .as_object_mut()
            .unwrap()
            .entry("mcpServers")
            .or_insert_with(|| json!({}));
        let Some(servers) = servers.as_object_mut() else {
            return Err("程小帮's mcpServers is not a JSON object".into());
        };
        if servers.get(PLUGIN_NAME) == Some(&wanted) {
            return Ok(false);
        }
        servers.insert(PLUGIN_NAME.into(), wanted);
        let mut text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        text.push('\n');
        fs::write(&file, text).map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Whether 程小帮's `mcp.json` lists SI Future HUD's MCP server.
    pub fn mcp_listed(&self) -> bool {
        self.folder
            .parent()
            .and_then(Path::parent)
            .and_then(|home| fs::read(home.join("mcp.json")).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|config| config["mcpServers"][PLUGIN_NAME].is_object())
    }

    /// Whether the plugin's files are in 程小帮's plugins folder.
    pub fn installed(&self) -> bool {
        self.folder.join("hooks/register.ts").is_file()
    }

    /// How many 程小帮 sessions run the plugin now (checked in lately).
    pub fn loaded(&self, now: SystemTime) -> usize {
        self.channel.loaded(now).len()
    }

    pub fn replies(&self) -> ChengxiaobangReplies {
        ChengxiaobangReplies {
            channel: self.channel.clone(),
            folder: self.folder.clone(),
        }
    }
}

pub struct ChengxiaobangReplies {
    channel: ClaudeChannel,
    folder: PathBuf,
}

/// A 程小帮 session's name in the channel: its own id, without the HUD's prefix.
fn own(session: &str) -> &str {
    session.strip_prefix("chengxiaobang:").unwrap_or(session)
}

impl ChengxiaobangReplies {
    fn unreachable_for(&self, session: &str) -> Unreachable {
        if self.folder.join("hooks/register.ts").is_file() && self.channel.ended(own(session)) {
            return Unreachable::new(
                l10n::text(
                    "这个程小帮会话已经关掉了（程小帮重启或会话被关）：在程小帮里重新点开它，再发一次。",
                    "This 程小帮 session was closed (程小帮 restarted or the session closed): open it again in 程小帮, then send again.",
                ),
                None,
            );
        }
        self.unreachable()
    }

    fn unreachable(&self) -> Unreachable {
        if !self.folder.join("hooks/register.ts").is_file() {
            return Unreachable::new(
                l10n::text(
                    "程小帮还没有 SI Future HUD 插件。",
                    "程小帮 doesn't have SI Future HUD's plugin yet.",
                ),
                Some(AnswerFix::InstallPlugin),
            );
        }
        Unreachable::new(
            l10n::text(
                "程小帮这次启动后还没打开过这个会话：在程小帮里点开它一次就能收（插件不用再设置）。",
                "程小帮 hasn't opened this session since it started: open it once in 程小帮 and it can receive (nothing else to set up).",
            ),
            None,
        )
    }
}

impl SessionReplies for ChengxiaobangReplies {
    fn reaches(&self, _source: &str, session: &str) -> Result<(), Unreachable> {
        if self
            .channel
            .loaded(SystemTime::now())
            .contains(own(session))
        {
            Ok(())
        } else {
            Err(self.unreachable_for(session))
        }
    }
    fn relay(&self, _source: &str, session: &str, words: &str) -> Result<(), String> {
        self.channel
            .hand_message(own(session), words, SystemTime::now())
            .map(drop)
            .map_err(|refusal| match refusal {
                Refusal::NotLoaded => self.unreachable_for(session).note,
                _ => l10n::text(
                    "消息没能交给程小帮插件。",
                    "The message could not be left for 程小帮's plugin.",
                ),
            })
    }
    fn reply(&self, _source: &str, session: &str, words: &str) -> Result<(), String> {
        self.channel
            .hand(own(session), words, SystemTime::now())
            .map(drop)
            .map_err(|refusal| match refusal {
                Refusal::NotLoaded => self.unreachable_for(session).note,
                Refusal::Rejected(_) => l10n::text(
                    "插件不能发送以 / 开头的话。",
                    "The plugin can't send words that start with /.",
                ),
                Refusal::InvalidInput(_) => {
                    l10n::text("没有要发送的话。", "There is nothing to send.")
                }
                _ => l10n::text(
                    "回答没能交给程小帮插件。",
                    "The answer could not be left for 程小帮's plugin.",
                ),
            })
    }
}

/// The host's carrier when more than one client takes words: each source's
/// own, and a refusal for a source no carrier serves.
pub struct RepliesBySource(pub Vec<(&'static str, Arc<dyn SessionReplies>)>);

impl RepliesBySource {
    fn carrier(&self, source: &str) -> Option<&Arc<dyn SessionReplies>> {
        self.0.iter().find(|(id, _)| *id == source).map(|(_, r)| r)
    }
}

impl SessionReplies for RepliesBySource {
    fn reaches(&self, source: &str, session: &str) -> Result<(), Unreachable> {
        match self.carrier(source) {
            Some(replies) => replies.reaches(source, session),
            None => Err(Unreachable::new(
                l10n::text(
                    "SI Future HUD 不能把回答发进这个客户端的会话。",
                    "SI Future HUD can't send answers into this client's sessions.",
                ),
                None,
            )),
        }
    }
    fn relay(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
        match self.carrier(source) {
            Some(replies) => replies.relay(source, session, words),
            None => Err(l10n::text(
                "SI Future HUD 不能把消息发进这个客户端的会话。",
                "SI Future HUD can't send words into this client's sessions.",
            )),
        }
    }
    fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
        match self.carrier(source) {
            Some(replies) => replies.reply(source, session, words),
            None => Err(l10n::text(
                "SI Future HUD 不能把回答发进这个客户端的会话。",
                "SI Future HUD can't send answers into this client's sessions.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepares_the_mod_and_carries_words_to_a_checked_in_session() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("cxb");
        fs::create_dir_all(home.join("plugins")).unwrap();
        let plugin = ChengxiaobangPlugin::new(root.path().join("channel"), &home);
        assert!(plugin.prepare().unwrap());
        assert!(!plugin.prepare().unwrap());
        let script = fs::read_to_string(plugin.folder().join("hooks/register.ts")).unwrap();
        assert!(script.contains(&*root.path().join("channel").to_string_lossy()));
        assert!(!script.contains("__AGENT_HUD_CHANNEL_JSON__"));
        let replies = plugin.replies();
        let refused = replies
            .reaches("chengxiaobang", "chengxiaobang:s_1")
            .unwrap_err();
        assert!(refused.fix.is_none());
        fs::write(
            root.path().join("channel/sessions/s_1.json"),
            json!({"at":1,"channelVersion":2}).to_string(),
        )
        .unwrap();
        replies
            .reaches("chengxiaobang", "chengxiaobang:s_1")
            .unwrap();
        replies
            .reply("chengxiaobang", "chengxiaobang:s_1", "Monday")
            .unwrap();
        let left = fs::read_dir(root.path().join("channel/prompts/s_1"))
            .unwrap()
            .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
            .collect::<Vec<_>>();
        assert_eq!(left.len(), 1);
        assert!(left[0].contains("\"text\":\"Monday\""));
    }

    #[test]
    fn lists_the_hud_tools_in_mcp_json_and_keeps_other_servers() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("cxb");
        fs::create_dir_all(home.join("plugins")).unwrap();
        fs::write(
            home.join("mcp.json"),
            r#"{"mcpServers":{"other":{"command":"x"}}}"#,
        )
        .unwrap();
        let exe = root.path().join("agent-hud-next");
        fs::write(&exe, "").unwrap();
        let plugin = ChengxiaobangPlugin::new(root.path().join("channel"), &home);
        assert!(!plugin.mcp_listed());
        assert!(plugin.ensure_mcp(&exe).unwrap());
        assert!(!plugin.ensure_mcp(&exe).unwrap());
        assert!(plugin.mcp_listed());
        let config: serde_json::Value =
            serde_json::from_slice(&fs::read(home.join("mcp.json")).unwrap()).unwrap();
        assert_eq!(config["mcpServers"]["other"]["command"], "x");
        assert_eq!(config["mcpServers"]["agent-hud"]["args"][0], "--mcp");
        assert_eq!(
            config["mcpServers"]["agent-hud"]["env"]["AGENT_HUD_CLIENT"],
            "chengxiaobang"
        );
    }

    #[test]
    fn writes_nothing_without_chengxiaobang() {
        let root = tempfile::tempdir().unwrap();
        let plugin =
            ChengxiaobangPlugin::new(root.path().join("channel"), &root.path().join("none"));
        assert!(!plugin.prepare().unwrap());
        assert!(!plugin.folder().exists());
    }
}
