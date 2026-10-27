//! 程小帮 on AgentHudCore: the questions its sessions ask, as cards, and how
//! their answers go back (as Claude's desktop questions do, in
//! `claude_desktop`). A question Agent HUD's 程小帮 plugin holds is answered
//! here; one waiting in 程小帮's own window shows, answered there.
use crate::*;
use agent_hud_agents::chengxiaobang::{transcript as cxb, AGENT as CXB, REPLIES};
use agent_hud_agents::replies::{AnswerFix, Unreachable};
use agent_hud_agents::transcript::live::DesktopAsk;

/// The cards of 程小帮's questions, by request id (their question's key and
/// the ticket that keeps the card answerable), and the keys answered or put
/// away, or reached, while they stand.
#[derive(Default)]
pub(crate) struct ChengxiaobangAsks {
    asks: HashMap<String, (String, crate::requests::Ticket)>,
    settled: HashSet<String>,
    reached: HashSet<String>,
}

impl crate::AgentHudCore {
    fn chengxiaobang_answers_off(&self) -> Option<Unreachable> {
        (self.settings[REPLIES.setting] == false).then(|| {
            Unreachable::new(
                crate::l10n::text(
                    "在 SI Future HUD 里回答程小帮的提问已关闭。",
                    "Answering 程小帮's questions in SI Future HUD is off.",
                ),
                Some(AnswerFix::TurnOnAnswers),
            )
        })
    }

    fn chengxiaobang_ask_way(&self, session: &str, ask: &DesktopAsk) -> Result<(), Unreachable> {
        if ask.kind != "question" {
            return Err(Unreachable::new(
                crate::l10n::text(
                    "这个提问在程小帮窗口里等你回答。",
                    "This question waits for you in 程小帮's own window.",
                ),
                None,
            ));
        }
        if let Some(off) = self.chengxiaobang_answers_off() {
            return Err(off);
        }
        match &self.session_replies {
            Some(replies) => replies.reaches(CXB.id, session),
            None => Err(Unreachable::new(
                crate::l10n::text(
                    "SI Future HUD 不能把回答发进程小帮会话。",
                    "SI Future HUD can't send answers into 程小帮 sessions.",
                ),
                None,
            )),
        }
    }

    /// Puts each standing 程小帮 question among the requests the user is
    /// asked about, answerable when Agent HUD's plugin holds it for a session
    /// it reaches (`sync_claude_desktop_asks` for 程小帮).
    pub(crate) fn sync_chengxiaobang_asks(&mut self, now: i64) {
        let mut active = HashMap::new();
        let mut standing = HashSet::new();
        if self.enabled.get(CXB.id).copied().unwrap_or(true) {
            let candidates = self
                .logs
                .values()
                .filter(|log| log.source == CXB.id && !log.gone && !log.archived)
                .filter_map(|log| {
                    let ask = log.live.activity.as_ref()?.desktop_ask.as_ref()?;
                    if !ask.is_worded()
                        || now - ask.at >= 1_800_000
                        || log.live.session_state(now, &log.source, None) != "waiting"
                    {
                        return None;
                    }
                    let key = crate::grok_cache::hash(&[&log.id, &ask.id]);
                    Some((key, log.id.clone(), log.cwd.clone(), ask.clone()))
                })
                .collect::<Vec<_>>();
            for (key, session, cwd, ask) in candidates {
                standing.insert(key.clone());
                if self.chengxiaobang_asks.settled.contains(&key) {
                    continue;
                }
                let way = if let Some(off) = self.chengxiaobang_answers_off() {
                    Err(off)
                } else if self.chengxiaobang_asks.reached.contains(&key) {
                    Ok(())
                } else {
                    self.chengxiaobang_ask_way(&session, &ask)
                };
                if way.is_ok() {
                    self.chengxiaobang_asks.reached.insert(key.clone());
                }
                let req_id = format!(
                    "chengxiaobang-ask:{}{key}",
                    if way.is_ok() { "reply:" } else { "" }
                );
                let view = cxb::ask_view(&req_id, &session, cwd.as_deref(), &ask, way.err());
                match self.requests.register_native(req_id.clone(), view.clone(), now) {
                    Some(ticket) => {
                        active.insert(req_id, (key, ticket));
                    }
                    None => {
                        self.requests.refresh_native(&req_id, view);
                        if let Some(kept) = self.chengxiaobang_asks.asks.remove(&req_id) {
                            active.insert(req_id, kept);
                        }
                    }
                }
            }
        }
        for old_id in self.chengxiaobang_asks.asks.keys() {
            if !active.contains_key(old_id) {
                self.requests.settle_native(old_id);
            }
        }
        self.chengxiaobang_asks.asks = active;
        let asks = &mut self.chengxiaobang_asks;
        asks.settled.retain(|key| standing.contains(key));
        asks.reached.retain(|key| standing.contains(key));
    }

    /// Gives the answer to 程小帮's question `id`, if it is one, before its
    /// card goes: the words reach its session through the host (the plugin
    /// hands them to the question it holds), and a refusal leaves the card
    /// waiting with the host's reason.
    pub(crate) fn answer_chengxiaobang_ask(
        &mut self,
        id: &str,
        decision: &Value,
        now: i64,
    ) -> Result<(), String> {
        let Some((key, _)) = self.chengxiaobang_asks.asks.get(id) else {
            return Ok(());
        };
        let key = key.clone();
        let view = self.requests.checked(id, decision, now)?;
        if let Some(words) = cxb::ask_words(&view, decision) {
            if let Some(off) = self.chengxiaobang_answers_off() {
                return Err(off.note);
            }
            let replies = self.session_replies.as_ref().ok_or_else(|| {
                crate::l10n::text(
                    "SI Future HUD 不能把回答发进程小帮会话。",
                    "SI Future HUD can't send answers into 程小帮 sessions.",
                )
            })?;
            replies.reply(
                CXB.id,
                view["sessionId"].as_str().unwrap_or_default(),
                &words,
            )?;
        }
        self.chengxiaobang_asks.settled.insert(key);
        Ok(())
    }
}
