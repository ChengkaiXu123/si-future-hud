//! Local build: questions that clients without a way in ask in their own
//! window (Grok Bot's question cards, Cursor's AskQuestion), shown as cards
//! the user answers there: the card says where, and opening the session
//! brings that window up. Nothing goes back through Agent HUD.
use crate::*;
use agent_hud_agents::transcript::live::DesktopAsk;

/// The clients whose questions only show, with their names.
const CLIENTS: [(&str, &str, &str); 2] = [
    ("grok-bot", "Grok Bot", "Grok Bot"),
    ("cursor", "Cursor", "Cursor"),
];

/// A question's words, read again from its client's files: they are kept
/// only while Agent HUD runs (`DesktopAsk`), so after a restart a standing
/// question is worded from the replica or transcript it came from.
fn reword(source: &str, session: &str, ask: &DesktopAsk, now: i64) -> Option<DesktopAsk> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME")?);
    let id = session.rsplit(':').next()?;
    let fresh = match source {
        "grok-bot" => {
            use agent_hud_agents::grok::{cache, live};
            let directory =
                home.join("Library/Application Support/Grok Bot/sand-client-persistence");
            let account = cache::account(&directory)?;
            let replica = cache::value(&directory, &cache::transcript_key(&account, id), 1)?;
            live::stand(id, &replica, now)?.ask
        }
        "cursor" => {
            use agent_hud_agents::cursor::live;
            live::stand(&live::transcript_for(&home, id)?, now)?.ask
        }
        _ => None,
    }?;
    (fresh.id == ask.id && fresh.is_worded()).then_some(fresh)
}

/// The cards standing, by request id.
#[derive(Default)]
pub(crate) struct WindowAsks {
    asks: HashMap<String, crate::requests::Ticket>,
}

fn view(
    req_id: &str,
    source: &str,
    vendor: &str,
    window: &str,
    session: &str,
    cwd: Option<&str>,
    ask: &DesktopAsk,
) -> Value {
    let options = ask
        .options
        .iter()
        .map(|option| json!({"label": option.label, "description": option.message.clone().unwrap_or_default()}))
        .collect::<Vec<_>>();
    let note = crate::l10n::text(
        &format!("这个提问在 {window} 窗口里等你回答。"),
        &format!("This question waits for you in {window}'s own window."),
    );
    json!({
        "id": req_id,
        "title": ask.question,
        "source": source,
        "vendor": vendor,
        "toolName": "AskUserQuestion",
        "input": "",
        "detail": null,
        "summary": ask.question,
        "sessionId": session,
        "createdAt": chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ask.at)
            .map(|at| at.to_rfc3339())
            .unwrap_or_default(),
        "allowsAlways": false,
        "kind": "question",
        "cwd": cwd,
        "path": null,
        "removed": null,
        "added": null,
        "suggestions": [],
        "alwaysAllowRule": null,
        "questions": [{
            "id": ask.id,
            "question": ask.question,
            "header": ask.recap.clone().unwrap_or_else(|| vendor.to_owned()),
            "options": options,
            "multiSelect": false,
            "answerKey": ask.id,
        }],
        "plan": null,
        "answersQuestion": false,
        "answerNote": note,
        "answerFix": null,
        "answersPlan": false,
    })
}

impl crate::AgentHudCore {
    /// Puts each standing window question of these clients among the
    /// requests the user is asked about (`sync_chengxiaobang_asks` without
    /// an answer path).
    pub(crate) fn sync_window_asks(&mut self, now: i64) {
        let mut active = HashMap::new();
        let candidates = self
            .logs
            .values()
            .filter(|log| !log.gone && !log.archived)
            .filter_map(|log| {
                let (source, vendor, window) =
                    CLIENTS.into_iter().find(|(id, ..)| *id == log.source)?;
                if !self.enabled.get(source).copied().unwrap_or(true) {
                    return None;
                }
                let standing = log.live.activity.as_ref()?.desktop_ask.as_ref()?;
                if now - standing.at >= 1_800_000
                    || log.live.session_state(now, &log.source, None) != "waiting"
                {
                    return None;
                }
                let ask = if standing.is_worded() {
                    standing.clone()
                } else {
                    reword(source, &log.id, standing, now)?
                };
                let ask = &ask;
                let key = crate::grok_cache::hash(&[&log.id, &ask.id]);
                Some((
                    format!("window-ask:{key}"),
                    source,
                    vendor,
                    window,
                    log.id.clone(),
                    log.cwd.clone(),
                    ask.clone(),
                ))
            })
            .collect::<Vec<_>>();
        for (req_id, source, vendor, window, session, cwd, ask) in candidates {
            let view = view(
                &req_id,
                source,
                vendor,
                window,
                &session,
                cwd.as_deref(),
                &ask,
            );
            match self
                .requests
                .register_native(req_id.clone(), view.clone(), now)
            {
                Some(ticket) => {
                    active.insert(req_id, ticket);
                }
                None => {
                    self.requests.refresh_native(&req_id, view);
                    if let Some(kept) = self.window_asks.asks.remove(&req_id) {
                        active.insert(req_id, kept);
                    }
                }
            }
        }
        for old_id in self.window_asks.asks.keys() {
            if !active.contains_key(old_id) {
                self.requests.settle_native(old_id);
            }
        }
        self.window_asks.asks = active;
    }
}
