//! 程小帮 (Chengxiaobang), Trip.com's AI workbench.
pub mod transcript;

use crate::replies::Replies;
use crate::Agent;
use crate::Discovery;
use crate::{Location, Source};

static SOURCES: [Source; 1] = [Source {
    reads_as: "chengxiaobang",
    location: Location::home(&[], &[".chengxiaobang"], "data"),
}];
static DISCOVERY: Discovery = Discovery {
    storage: &[Location::home(&[], &[".chengxiaobang"], "data/chengxiaobang.sqlite")],
    settings: &[Location::home(&[], &[".chengxiaobang"], "config.yaml")],
    sessions: &[Location::home(&[], &[".chengxiaobang"], "data/chengxiaobang.sqlite")],
    apps: &["程小帮"],
    bundles: &["com.chengxiaobang.desktop"],
    ..Discovery::NONE
};

/// Words reach a 程小帮 session through Agent HUD's 程小帮 plugin (a mod in
/// its plugins folder), as Claude Code's do through SI Future HUD's plugin.
pub static REPLIES: Replies = Replies {
    carrier: "SI Future HUD's 程小帮 plugin",
    setting: "chengxiaobangAnswers",
};

pub static AGENT: Agent = Agent {
    id: "chengxiaobang",
    name: "程小帮",
    sessions: true,
    hooks: None,
    observer: None,
    channel: None,
    program: None,
    replies: Some(&REPLIES),
    turns: None,
    items: None,
    sources: crate::everywhere(&SOURCES),
    discovery: &DISCOVERY,
};
