//! Agent HUD's 程小帮 plugin: a mod (程小帮 loads Claude Code mods) in its
//! plugins folder, and the channel it shares with Agent HUD. The channel is
//! Claude's (`ClaudeChannel`): sessions check in, words wait in `prompts/`,
//! and a question the plugin holds takes its answer from `answers/`.
use crate::claude_plugin::{ClaudeChannel, Refusal};
use agent_hud_core::l10n;
use agent_hud_core::replies::{AnswerFix, SessionReplies, Unreachable};
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

const SCRIPT: &str = include_str!("register.ts");
pub const PLUGIN_NAME: &str = "agent-hud";

/// 程小帮's folder, `~/.chengxiaobang`, where its plugins live.
pub fn chengxiaobang_directory(home: &Path) -> PathBuf {
    home.join(".chengxiaobang")
}

#[derive(Clone)]
pub struct ChengxiaobangPlugin {
    channel: ClaudeChannel,
    /// `~/.chengxiaobang/plugins/agent-hud`.
    folder: PathBuf,
}

/// Local build: what 程小帮 says of the plugin, as far as SI Future HUD can read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PluginState {
    /// `~/.chengxiaobang` is on this Mac.
    pub found: bool,
    /// The plugin's files are in 程小帮's plugins folder.
    pub installed: bool,
    /// Enabled on 程小帮's Plugins page; None when its settings can't be read.
    pub enabled: Option<bool>,
    /// The user stopped loading it in 程小帮.
    pub stopped: bool,
    /// 程小帮 runs now.
    pub running: bool,
}

/// 程小帮's own record of its plugins (read-only): whether ours is enabled,
/// and whether loading it was stopped.
fn plugin_settings(home: &Path) -> Option<(bool, bool)> {
    let rows = agent_hud_agents::sqlite::rows(
        &home.join("data/chengxiaobang.sqlite"),
        "SELECT key, value FROM settings WHERE key IN ('plugins.enabled', 'mods.stopped')",
        &[],
        Duration::from_millis(250),
    )
    .ok()?;
    let value = |key: &str| {
        rows.iter()
            .find(|row| row.first().and_then(serde_json::Value::as_str) == Some(key))
            .and_then(|row| row.get(1)?.as_str())
            .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
    };
    let enabled = value("plugins.enabled")?
        .as_array()?
        .iter()
        .any(|name| name.as_str() == Some(PLUGIN_NAME));
    let stopped = value("mods.stopped").is_some_and(|stopped| stopped.get(PLUGIN_NAME).is_some());
    Some((enabled, stopped))
}

/// Whether 程小帮's app runs now.
fn chengxiaobang_running() -> bool {
    std::process::Command::new("/usr/bin/pgrep")
        .args(["-f", "/程小帮.app/Contents/MacOS/"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
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

    /// `~/.chengxiaobang`.
    fn home(&self) -> Option<&Path> {
        self.folder.parent().and_then(Path::parent)
    }

    /// Local build: what 程小帮 says of the plugin now. Reads 程小帮's
    /// settings and its process list: not for every collection pass.
    pub fn state(&self) -> PluginState {
        let found = self.home().is_some_and(Path::is_dir);
        let settings = self.home().filter(|_| found).and_then(plugin_settings);
        PluginState {
            found,
            installed: self.installed(),
            enabled: settings.map(|(enabled, _)| enabled),
            stopped: settings.is_some_and(|(_, stopped)| stopped),
            running: found && chengxiaobang_running(),
        }
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
                json!({"name":PLUGIN_NAME,"displayName":"SI Future HUD","version":"1.1.0","description":"把其他智能体经 SI Future HUD 发来的消息送进这个对话，并让你在 HUD 上回答它的提问。","author":{"name":"SI Future HUD"}}).to_string(),
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

    /// The carrier for 程小帮's sessions. `executable` is the fixed installed
    /// app, which a plugin written later lists as 程小帮's HUD tools.
    pub fn replies(&self, executable: Option<PathBuf>) -> ChengxiaobangReplies {
        ChengxiaobangReplies {
            plugin: self.clone(),
            executable,
            state: Mutex::new(None),
        }
    }
}

pub struct ChengxiaobangReplies {
    plugin: ChengxiaobangPlugin,
    executable: Option<PathBuf>,
    /// The last read of 程小帮's plugin state, kept a few seconds: the
    /// reasons are asked of every collection pass.
    state: Mutex<Option<(Instant, PluginState)>>,
}

/// A 程小帮 session's name in the channel: its own id, without the HUD's prefix.
fn own(session: &str) -> &str {
    session.strip_prefix("chengxiaobang:").unwrap_or(session)
}

const STATE_LIFETIME: Duration = Duration::from_secs(5);

impl ChengxiaobangReplies {
    fn channel(&self) -> &ClaudeChannel {
        &self.plugin.channel
    }

    /// 程小帮's plugin state, read at most every few seconds. A plugin missing
    /// from a 程小帮 installed after SI Future HUD started is written now.
    fn state(&self) -> PluginState {
        let mut cached = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((at, state)) = *cached {
            if at.elapsed() < STATE_LIFETIME {
                return state;
            }
        }
        let mut state = self.plugin.state();
        if state.found && !state.installed && self.plugin.prepare().is_ok() {
            if let Some(executable) = &self.executable {
                let _ = self.plugin.ensure_mcp(executable);
            }
            state.installed = self.plugin.installed();
        }
        *cached = Some((Instant::now(), state));
        state
    }

    /// Whether words for `session`, which isn't running the plugin now, may
    /// wait for it: it ran the plugin before, and 程小帮 still loads it.
    fn queueable(&self, session: &str) -> bool {
        if !self.channel().checked_in_before(own(session)) {
            return false;
        }
        let state = self.state();
        state.installed && state.enabled != Some(false) && !state.stopped
    }

    /// Why words don't reach `session`, and what the user does about it.
    fn unreachable_for(&self, session: &str) -> Unreachable {
        let state = self.state();
        if !state.found {
            return Unreachable::new(
                l10n::text("这台 Mac 上没有程小帮。", "程小帮 isn't on this Mac."),
                None,
            );
        }
        if !state.installed {
            return Unreachable::new(
                l10n::text(
                    "程小帮还没有 SI Future HUD 插件。",
                    "程小帮 doesn't have SI Future HUD's plugin yet.",
                ),
                Some(AnswerFix::InstallPlugin),
            );
        }
        if state.enabled == Some(false) {
            return Unreachable::new(
                l10n::text(
                    "程小帮里还没启用 SI Future HUD 插件：打开程小帮 → 插件 → SI Future HUD → 启用（不用重启），再点开要收消息的对话。",
                    "SI Future HUD's plugin isn't enabled in 程小帮: open 程小帮 → Plugins → SI Future HUD → Enable (no restart needed), then open the conversation that should take the message.",
                ),
                None,
            );
        }
        if state.stopped {
            return Unreachable::new(
                l10n::text(
                    "程小帮停止加载了 SI Future HUD 插件：在程小帮 → 插件里重新加载它。",
                    "程小帮 stopped loading SI Future HUD's plugin: load it again in 程小帮 → Plugins.",
                ),
                None,
            );
        }
        if !state.running {
            return Unreachable::new(
                l10n::text(
                    "程小帮现在没在运行：打开程小帮，再点开要收消息的对话。",
                    "程小帮 isn't running: open it, then open the conversation that should take the message.",
                ),
                None,
            );
        }
        if self.channel().loaded(SystemTime::now()).is_empty() {
            return Unreachable::new(
                l10n::text(
                    "程小帮还没加载 SI Future HUD 插件的任何对话：在程小帮里点开要收消息的对话（刚启用插件或刚升级 HUD 时，重启一次程小帮）。",
                    "No 程小帮 conversation runs SI Future HUD's plugin yet: open the conversation that should take the message in 程小帮 (right after enabling the plugin or updating the HUD, restart 程小帮 once).",
                ),
                None,
            );
        }
        let _ = session;
        Unreachable::new(
            l10n::text(
                "程小帮这次启动后还没打开过这个对话：在程小帮里点开它一次就能收。",
                "程小帮 hasn't opened this conversation since it started: open it once in 程小帮 and it can take messages.",
            ),
            None,
        )
    }
}

impl SessionReplies for ChengxiaobangReplies {
    /// An answer needs the session running the plugin now.
    fn reaches(&self, _source: &str, session: &str) -> Result<(), Unreachable> {
        if self
            .channel()
            .loaded(SystemTime::now())
            .contains(own(session))
        {
            Ok(())
        } else {
            Err(self.unreachable_for(session))
        }
    }
    /// A message may also wait for a session that ran the plugin before.
    fn relays_to(&self, source: &str, session: &str) -> Result<(), Unreachable> {
        if self.queueable(session) {
            return Ok(());
        }
        self.reaches(source, session)
    }
    fn live_sessions(&self, _source: &str) -> Option<HashSet<String>> {
        Some(
            self.channel()
                .loaded(SystemTime::now())
                .into_iter()
                .map(|id| format!("chengxiaobang:{id}"))
                .collect(),
        )
    }
    fn relay(&self, _source: &str, session: &str, words: &str) -> Result<(), String> {
        let now = SystemTime::now();
        let left = if self.channel().loaded(now).contains(own(session)) {
            self.channel().hand_message(own(session), words, now)
        } else if self.queueable(session) {
            self.channel().queue_message(own(session), words, now)
        } else {
            Err(Refusal::NotLoaded)
        };
        left.map(drop).map_err(|refusal| match refusal {
            Refusal::NotLoaded => self.unreachable_for(session).note,
            _ => l10n::text(
                "消息没能交给程小帮插件。",
                "The message could not be left for 程小帮's plugin.",
            ),
        })
    }
    fn reply(&self, _source: &str, session: &str, words: &str) -> Result<(), String> {
        self.channel()
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
    fn live_sessions(&self, source: &str) -> Option<HashSet<String>> {
        self.carrier(source)?.live_sessions(source)
    }
    fn relays_to(&self, source: &str, session: &str) -> Result<(), Unreachable> {
        match self.carrier(source) {
            Some(replies) => replies.relays_to(source, session),
            None => self.reaches(source, session),
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
        let replies = plugin.replies(None);
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
    fn queues_a_message_for_a_session_that_checked_in_before() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("cxb");
        fs::create_dir_all(home.join("plugins")).unwrap();
        let plugin = ChengxiaobangPlugin::new(root.path().join("channel"), &home);
        plugin.prepare().unwrap();
        let replies = plugin.replies(None);
        // s_2 ran the plugin an hour ago, then 程小帮 restarted.
        let file = root.path().join("channel/sessions/s_2.json");
        fs::write(&file, json!({"ended":1}).to_string()).unwrap();
        fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();
        assert!(!replies
            .live_sessions("chengxiaobang")
            .unwrap()
            .contains("chengxiaobang:s_2"));
        // A message may wait for it; an answer may not.
        replies
            .relays_to("chengxiaobang", "chengxiaobang:s_2")
            .unwrap();
        assert!(replies
            .reaches("chengxiaobang", "chengxiaobang:s_2")
            .is_err());
        replies
            .relay("chengxiaobang", "chengxiaobang:s_2", "hello later")
            .unwrap();
        let left = fs::read_dir(root.path().join("channel/prompts/s_2"))
            .unwrap()
            .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
            .collect::<Vec<_>>();
        assert_eq!(left.len(), 1);
        assert!(left[0].contains("\"kind\":\"message\""), "{}", left[0]);
        // A session that never ran the plugin is not queued for.
        assert!(replies
            .relays_to("chengxiaobang", "chengxiaobang:s_9")
            .is_err());
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
