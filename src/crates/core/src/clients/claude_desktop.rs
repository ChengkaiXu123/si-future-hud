//! Claude's desktop app on AgentHudCore: the questions it asks in its own
//! window, as cards, and how their answers go back.
use crate::*;
use agent_hud_agents::claude::{transcript::*, AGENT as CLAUDE, REPLIES};
use agent_hud_agents::replies::{AnswerFix, Unreachable};
use agent_hud_agents::transcript::live::DesktopAsk;

impl crate::AgentHudCore {
    /// Gives each desktop question that would show a card, and that a
    /// checkpoint gave back without its words, its words again from its
    /// call's line, once: one whose line cannot be read is not tried again
    /// until the log changes, and has no card while its session still waits.
    pub(crate) fn reword_desktop_asks(&mut self, now: i64) {
        let unworded = self
            .logs
            .values()
            .filter(|log| log.source == CLAUDE.id && !log.gone)
            .filter_map(|log| {
                let ask = log.live.activity.as_ref()?.desktop_ask.as_ref()?;
                (!ask.is_worded() && ask.offset.is_some() && now - ask.at < 1_800_000).then(|| {
                    (
                        log.path.clone(),
                        log.origin_path.clone().unwrap_or_else(|| log.path.clone()),
                        ask.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();
        fn asked<'a>(head: &'a mut crate::LogHead, id: &str) -> Option<&'a mut DesktopAsk> {
            head.live
                .activity
                .as_mut()?
                .desktop_ask
                .as_mut()
                .filter(|kept| kept.id == id)
        }
        for (key, file, ask) in unworded {
            let worded = reworded_desktop_ask(&file, &ask);
            let Some(head) = self.logs.get_mut(&key) else {
                continue;
            };
            match worded {
                Some(worded) => {
                    let mut read = head.clone();
                    if let Some(kept) = asked(&mut read, &ask.id) {
                        *kept = worded;
                    }
                    head.keep_words(&read);
                }
                None => {
                    if let Some(kept) = asked(head, &ask.id) {
                        kept.offset = None;
                    }
                }
            }
        }
    }
}

impl crate::AgentHudCore {
    /// Why Claude's questions are not answered here: the user turned off
    /// "Answer questions" on the Claude Code plugin's row in Settings.
    pub(crate) fn claude_answers_off(&self) -> Option<Unreachable> {
        (self.settings[REPLIES.setting] == false).then(|| {
            Unreachable::new(
                crate::l10n::text(
                    "在 SI Future HUD 里回答 Claude 的提问已关闭（设置 → 智能体插件）。",
                    "Answering Claude's questions in SI Future HUD is off (Settings → Agent plugins).",
                ),
                Some(AnswerFix::TurnOnAnswers),
            )
        })
    }

    /// Whether a card for `ask` in the session `session` can take its answer
    /// here; Err gives why it can only be answered in the app. An offer's
    /// buttons are the app's own actions, which no words stand for; a box
    /// needs answering on and a host that reaches its session.
    pub(crate) fn desktop_ask_way(
        &self,
        session: &str,
        ask: &DesktopAsk,
    ) -> Result<(), Unreachable> {
        if ask.kind == "offer" {
            return Err(Unreachable::new(
                crate::l10n::text(
                    "它的按钮只能在 Claude 窗口里用。",
                    "Its buttons work only in Claude's own window.",
                ),
                None,
            ));
        }
        if let Some(off) = self.claude_answers_off() {
            return Err(off);
        }
        match &self.session_replies {
            Some(replies) => replies.reaches(CLAUDE.id, session),
            None => Err(Unreachable::new(
                crate::l10n::text(
                    "SI Future HUD 不能把回答发进 Claude 会话。",
                    "SI Future HUD can't send answers into Claude sessions.",
                ),
                None,
            )),
        }
    }

    /// Gives the answer to the Claude desktop app's question `id`, if it is
    /// one, before its card goes: the words go into its session through the
    /// host, and a refusal leaves the card waiting with the host's reason.
    /// Answered or put away, its question has no card again while it stands.
    pub(crate) fn answer_desktop_ask(
        &mut self,
        id: &str,
        decision: &Value,
        now: i64,
    ) -> Result<(), String> {
        let Some((key, _)) = self.claude_desktop.asks.get(id) else {
            return Ok(());
        };
        let key = key.clone();
        let view = self.requests.checked(id, decision, now)?;
        if let Some(words) = desktop_ask_words(&view, decision) {
            if let Some(off) = self.claude_answers_off() {
                return Err(off.note);
            }
            let replies = self.session_replies.as_ref().ok_or_else(|| {
                crate::l10n::text(
                    "SI Future HUD 不能把回答发进 Claude 会话。",
                    "SI Future HUD can't send answers into Claude sessions.",
                )
            })?;
            replies.reply(
                CLAUDE.id,
                view["sessionId"].as_str().unwrap_or_default(),
                &words,
            )?;
        }
        self.claude_desktop.settled.insert(key);
        Ok(())
    }
}

/// What the Claude desktop app knows that its logs don't say: its sessions
/// and archive, its scheduled jobs, and the questions it asks in its own window.
#[derive(Default)]
pub(crate) struct ClaudeDesktop {
    pub(crate) index: ClaudeDesktopIndex,
    pub(crate) jobs: crate::claude_jobs::ClaudeJobs,
    /// The cards of the app's questions, by request id: the question's key
    /// (its session and call) and the ticket that keeps the card answerable.
    pub(crate) asks: HashMap<String, (String, crate::requests::Ticket)>,
    /// The keys of questions answered or put away on the HUD or a phone: no
    /// card comes back for them, answerable or not, while they stand.
    pub(crate) settled: HashSet<String>,
    /// The keys of questions whose session the host has reached since they
    /// were asked: their cards stay answerable while they stand.
    pub(crate) reached: HashSet<String>,
}
