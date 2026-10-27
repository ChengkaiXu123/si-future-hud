//! Local build: Grok Bot's live facts, from the transcript replicas the
//! desktop app keeps for the Bots it has open (`transcript.replicas.<id>`).
//!
//! A replica has no turn lifecycle. A Bot owes a reply while its newest entry
//! is the user's: a message, an attachment, or a question card the user just
//! answered. Each finished Bot message is a reply of its own (a Bot posts
//! several per request, as its own notifications do). A card that ends the
//! replica unanswered (a question, running a command here, a secret,
//! connecting an account) waits on the user, answered in Grok Bot's window.
//! An owed reply never lapses into a reply: the live log lets it go quiet. A
//! command the Bot runs on this Mac (its local-exec daemon has one in flight)
//! is a tool call of the Bot that owes a reply and was active last.
use super::cache;
use crate::transcript::live::{
    newer, Activity, Call, DesktopAsk, DesktopAskOption, LiveLog, Turn, Work,
};
use serde_json::Value;
use std::path::Path;

/// A replica's newest entry that tells where the Bot stands.
pub struct BotRun {
    pub agent: String,
    /// The entry's own id (`t<turn>u` for the user's, `t<turn>s<step>` for the Bot's).
    pub id: String,
    pub at: i64,
    /// The Bot owes a reply or waits on the user.
    pub running: bool,
    /// The newest entry is a finished Bot message: a reply of its own.
    pub replied: bool,
    pub answer: Option<String>,
    pub ask: Option<DesktopAsk>,
    pub command: bool,
}

fn text(message: &Value) -> Option<String> {
    message["content"]
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn answered(entry: &Value) -> bool {
    !entry["respondedValue"].is_null() || entry["widgetSkipped"] == true
}

fn ask(id: &str, at: i64, question: String, options: Vec<DesktopAskOption>) -> DesktopAsk {
    DesktopAsk {
        id: id.to_owned(),
        response: None,
        kind: "window".into(),
        question,
        recap: None,
        options,
        at,
        offset: None,
    }
}

fn widget_ask(entry: &Value, id: &str, at: i64) -> Option<DesktopAsk> {
    let widget = &entry["message"]["widget"];
    let question = widget["prompt"].as_str()?.trim();
    if question.is_empty() {
        return None;
    }
    let options = widget["options"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|option| {
            Some(DesktopAskOption {
                label: option["label"].as_str()?.to_owned(),
                message: None,
            })
        })
        .collect();
    Some(ask(id, at, question.into(), options))
}

/// What a card the Bot waits on asks, for the cards Grok Bot shows in its
/// window: running a command here, a secret, connecting an account.
fn card_ask(entry: &Value, id: &str, at: i64) -> Option<DesktopAsk> {
    let message = &entry["message"];
    let question = match message["type"].as_str()? {
        "local-tool-permission" => {
            let status = message["ask"]["status"].as_str().unwrap_or("pending");
            if status != "pending" {
                return None;
            }
            let target = message["ask"]["target"].as_str().unwrap_or_default().trim();
            crate::l10n::text(
                &format!("Grok Bot 想在这台 Mac 上运行：{target}"),
                &format!("Grok Bot wants to run on this Mac: {target}"),
            )
        }
        "secret-request" => {
            if entry["secretProvided"] == true || message["secretProvided"] == true {
                return None;
            }
            crate::l10n::text(
                "Grok Bot 在等你填写一项密钥。",
                "Grok Bot waits for a secret from you.",
            )
        }
        "connector" => {
            let name = message["connector"]["name"]
                .as_str()
                .or(message["connector"].as_str())
                .unwrap_or("an account");
            crate::l10n::text(
                &format!("Grok Bot 请你在它的窗口里连接 {name}。"),
                &format!("Grok Bot asks you to connect {name} in its window."),
            )
        }
        _ => return None,
    };
    Some(ask(id, at, question, vec![]))
}

/// Where one Bot stands, from its replica's entries (any order: by `seq`).
pub fn stand(agent: &str, replica: &Value, _now: i64) -> Option<BotRun> {
    let mut entries: Vec<&Value> = replica["entries"].as_array()?.iter().collect();
    entries.sort_by_key(|entry| entry["seq"].as_u64().unwrap_or(0));
    for entry in entries.into_iter().rev() {
        let (Some(id), Some(at)) = (entry["id"].as_str(), entry["timestampMs"].as_i64()) else {
            continue;
        };
        let mut run = BotRun {
            agent: agent.to_owned(),
            id: id.to_owned(),
            at,
            running: false,
            replied: false,
            answer: None,
            ask: None,
            command: false,
        };
        let message = &entry["message"];
        match (entry["kind"].as_str(), message["type"].as_str()) {
            // The user spoke, or answered a card: the Bot owes a reply.
            (Some("message" | "user-attachment"), _) => run.running = true,
            (Some("send-message"), Some("widget")) if answered(entry) => run.running = true,
            (Some("send-message"), Some("widget")) => {
                run.ask = widget_ask(entry, id, at);
                run.running = run.ask.is_some();
            }
            // A message still being written is no reply yet.
            (Some("send-message"), Some("text"))
                if message["streaming"] == true || text(message).is_none() =>
            {
                run.running = true
            }
            (Some("send-message"), Some("text")) => {
                run.replied = true;
                run.answer = text(message);
            }
            (
                Some("send-message"),
                Some("local-tool-permission" | "secret-request" | "connector"),
            ) => {
                // A card the Bot waits on, or one dealt with (it works on).
                run.ask = card_ask(entry, id, at);
                run.running = true;
            }
            // Renames, notices and kinds not known here say nothing of the turn.
            _ => continue,
        }
        return Some(run);
    }
    None
}

/// Whether Grok Bot's local-exec daemon runs a command for a Bot now.
pub fn command_in_flight(home: &Path, now: i64) -> bool {
    let Ok(bytes) = std::fs::read(home.join(".grokbot/local-exec-daemon.json")) else {
        return false;
    };
    let Ok(daemon) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    // The daemon refreshes `servingAt` every few seconds while it serves.
    daemon["inflightCount"].as_u64().is_some_and(|n| n > 0)
        && daemon["servingAt"]
            .as_i64()
            .is_some_and(|at| (now - at).abs() < 30_000)
}

/// Each listed Bot's standing, from the replicas in `directory`; a command in
/// flight is laid on the Bot that owes a reply and was active last.
pub fn runs(
    directory: &Path,
    account: &str,
    agents: &[String],
    now: i64,
    command: bool,
) -> Vec<BotRun> {
    let mut runs: Vec<BotRun> = agents
        .iter()
        .filter_map(|agent| {
            let replica = cache::value(directory, &cache::transcript_key(account, agent), 1)?;
            stand(agent, &replica, now)
        })
        .collect();
    // A command is the work of a Bot that owes a reply: the one active last.
    if command {
        if let Some(last) = runs
            .iter_mut()
            .filter(|run| run.running && run.ask.is_none())
            .max_by_key(|run| run.at)
        {
            last.command = true;
        }
    }
    runs
}

/// The Bots the signed-in roster lists in the sidebar, by id.
pub fn listed(directory: &Path, account: &str) -> Vec<String> {
    let Some(roster) = cache::value(directory, &cache::roster_key(account), 4) else {
        return vec![];
    };
    roster["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["isHiddenFromSidebar"] != true && row["isRemovedFromSidebar"] != true)
        .filter_map(|row| row["id"].as_str().filter(|id| cache::agent_id(id)))
        .map(str::to_owned)
        .collect()
}

/// Whether `path` is a Bot's transcript replica.
pub fn is_replica(path: &Path) -> bool {
    cache::key(path)
        .is_some_and(|key| key.starts_with(cache::PREFIX) && key.contains(".transcript.replicas."))
}

/// What a roster's stamp also follows: its replicas' newest write, and the
/// local-exec daemon's while it runs a command (it rewrites its file every
/// few seconds, so it counts only then).
pub fn followed_writes(directory: &Path, home: &Path, now: i64) -> Vec<std::time::SystemTime> {
    let mut writes: Vec<std::time::SystemTime> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| is_replica(&entry.path()))
        .filter_map(|entry| entry.metadata().ok()?.modified().ok())
        .collect();
    if command_in_flight(home, now) {
        writes.extend(
            std::fs::metadata(home.join(".grokbot/local-exec-daemon.json"))
                .ok()
                .and_then(|meta| meta.modified().ok()),
        );
    }
    writes
}

/// Lays a Bot's standing on its session's live log (程小帮's `apply` for Grok Bot).
pub fn apply(live: &mut LiveLog, run: &BotRun, now: i64) {
    let observed = if run.command { now } else { run.at };
    newer(&mut live.last_event, observed);
    match live
        .turns
        .back_mut()
        .filter(|turn| turn.id.as_deref() == Some(run.id.as_str()))
    {
        Some(turn) => {
            turn.observed = turn.observed.max(observed);
            turn.running = run.running;
            turn.outcome = run.replied.then(|| "completed".to_owned());
            if run.answer.is_some() {
                turn.message = run.answer.clone();
            }
        }
        // Only a finished Bot message completes a turn (and pops up once).
        None => live.push(Turn {
            id: Some(run.id.clone()),
            started: Some(run.at),
            observed,
            running: run.running,
            outcome: run.replied.then(|| "completed".to_owned()),
            limited_until: None,
            message: run.answer.clone(),
        }),
    }
    let activity = live.activity.get_or_insert_with(Activity::default);
    activity.calls.clear();
    activity.standing_question = None;
    activity.desktop_ask = None;
    activity.retrying = false;
    activity.compacting_since = None;
    if !run.running {
        activity.latest = None;
        activity.line = Some("answer".into());
        return;
    }
    if let Some(ask) = &run.ask {
        let call = Call {
            id: ask.id.clone(),
            work: Some(Work::Question),
            at: ask.at,
            request: Some("question".into()),
        };
        activity.standing_question = Some(call.clone());
        activity.desktop_ask = Some(ask.clone());
        activity.open(call);
    }
    if run.command {
        activity.open(Call {
            id: format!("{}:command", run.id),
            work: Some(Work::Command),
            at: now,
            request: None,
        });
        activity.work = Some(Work::Command);
        activity.tool_at = Some(now);
    }
    activity.line = Some(
        if activity.calls.is_empty() {
            "prompt"
        } else {
            "call"
        }
        .into(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: i64 = 1_793_006_400_000;

    fn replica(entries: Value) -> Value {
        json!({"entries": entries, "persistedAt": NOW})
    }

    #[test]
    fn the_users_newest_message_is_owed_a_reply() {
        let r = replica(json!([
            {"id":"t1s0","kind":"send-message","seq":1,"timestampMs":NOW-60_000,"message":{"type":"text","content":"hi"}},
            {"id":"t2u","kind":"message","role":"user","seq":2,"timestampMs":NOW-5_000,"content":"do it"},
        ]));
        let run = stand("a", &r, NOW).unwrap();
        assert!(run.running && !run.replied);
        assert_eq!(run.id, "t2u");
        // An owed reply never turns into one, however long it waits.
        let late = stand("a", &r, NOW + 3_600_000).unwrap();
        assert!(late.running && !late.replied);
        let mut live = LiveLog::default();
        apply(&mut live, &late, NOW + 3_600_000);
        assert!(live.turns.back().unwrap().outcome.is_none());
    }

    #[test]
    fn cards_the_bot_waits_on_wait_and_unknown_kinds_are_skipped() {
        let permission = |status: &str| {
            replica(json!([
                {"id":"t1s0","kind":"send-message","seq":1,"timestampMs":NOW-9_000,"message":{"type":"text","content":"checking"}},
                {"id":"t1s1","kind":"send-message","seq":2,"timestampMs":NOW-5_000,"message":{"type":"local-tool-permission","ask":{"action":"run-command","target":"git pull","status":status}}},
                {"id":"n1","kind":"notice","seq":3,"timestampMs":NOW-1_000},
            ]))
        };
        let waiting = stand("a", &permission("pending"), NOW).unwrap();
        assert_eq!(waiting.id, "t1s1");
        assert!(waiting.running && !waiting.replied);
        assert!(waiting.ask.as_ref().unwrap().question.contains("git pull"));
        let approved = stand("a", &permission("always"), NOW).unwrap();
        assert!(approved.running && approved.ask.is_none());
        let secret = replica(json!([
            {"id":"t2s0","kind":"send-message","seq":1,"timestampMs":NOW-5_000,"message":{"type":"secret-request"}},
        ]));
        assert!(stand("a", &secret, NOW).unwrap().ask.is_some());
        let streaming = replica(json!([
            {"id":"t3s0","kind":"send-message","seq":1,"timestampMs":NOW-1_000,"message":{"type":"text","content":"","streaming":true}},
        ]));
        let run = stand("a", &streaming, NOW).unwrap();
        assert!(run.running && !run.replied);
    }

    #[test]
    fn a_command_lands_only_on_a_bot_that_owes_a_reply() {
        let home = std::env::temp_dir().join(format!("grok-runs-{}", std::process::id()));
        let dir = home.join("cache");
        std::fs::create_dir_all(&dir).unwrap();
        let write = |agent: &str, entries: Value| {
            let file = cache::path(&dir, &cache::transcript_key("slot", agent));
            std::fs::write(
                file,
                json!({"schemaVersion":1,"value":{"entries":entries}}).to_string(),
            )
            .unwrap();
        };
        write(
            "aaaaaaaa-0000-0000-0000-000000000001",
            json!([{"id":"t1s0","kind":"send-message","seq":1,"timestampMs":NOW-1_000,"message":{"type":"text","content":"done"}}]),
        );
        write(
            "aaaaaaaa-0000-0000-0000-000000000002",
            json!([{"id":"t2u","kind":"message","seq":1,"timestampMs":NOW-60_000,"content":"go"}]),
        );
        let agents = vec![
            "aaaaaaaa-0000-0000-0000-000000000001".to_owned(),
            "aaaaaaaa-0000-0000-0000-000000000002".to_owned(),
        ];
        let runs = runs(&dir, "slot", &agents, NOW, true);
        assert!(!runs[0].command, "a finished Bot is not reopened");
        assert!(runs[1].command && runs[1].running);
        std::fs::remove_dir_all(home).ok();
    }

    #[test]
    fn each_bot_message_is_a_reply_and_events_are_skipped() {
        let r = replica(json!([
            {"id":"t2u","kind":"message","role":"user","seq":1,"timestampMs":NOW-9_000,"content":"x"},
            {"id":"t2s0","kind":"send-message","seq":3,"timestampMs":NOW-2_000,"message":{"type":"text","content":"  done \n"}},
            {"id":"event-1","kind":"event","seq":4,"timestampMs":NOW-1_000,"event":{"type":"name-changed"}},
        ]));
        let run = stand("a", &r, NOW).unwrap();
        assert!(!run.running);
        assert_eq!(run.id, "t2s0");
        assert_eq!(run.answer.as_deref(), Some("done"));
    }

    #[test]
    fn an_unanswered_card_waits_and_an_answered_one_is_owed() {
        let card = json!({"id":"t3s1","kind":"send-message","seq":5,"timestampMs":NOW-3_000,
            "message":{"type":"widget","widget":{"prompt":"Which?","options":[{"label":"A","value":"a"},{"label":"B","value":"b"}]}}});
        let run = stand("a", &replica(json!([card.clone()])), NOW).unwrap();
        assert!(run.running);
        let ask = run.ask.unwrap();
        assert_eq!(ask.kind, "window");
        assert_eq!(ask.question, "Which?");
        assert_eq!(ask.options.len(), 2);
        let mut answered = card;
        answered["respondedValue"] = json!("a");
        let run = stand("a", &replica(json!([answered])), NOW).unwrap();
        assert!(run.running && run.ask.is_none());
    }

    #[test]
    fn standing_lays_turns_questions_and_commands() {
        let mut live = LiveLog::default();
        let card = replica(
            json!([{"id":"t3s1","kind":"send-message","seq":5,"timestampMs":NOW-3_000,
            "message":{"type":"widget","widget":{"prompt":"Which?","options":[]}}}]),
        );
        apply(&mut live, &stand("a", &card, NOW).unwrap(), NOW);
        let activity = live.activity.as_ref().unwrap();
        assert!(activity.standing_question.is_some());
        assert!(activity.desktop_ask.is_some());
        assert!(live.turns.back().unwrap().running);

        let reply = replica(
            json!([{"id":"t3s2","kind":"send-message","seq":6,"timestampMs":NOW-1_000,
            "message":{"type":"text","content":"ok"}}]),
        );
        let mut run = stand("a", &reply, NOW).unwrap();
        apply(&mut live, &run, NOW);
        let turn = live.turns.back().unwrap();
        assert_eq!(turn.outcome.as_deref(), Some("completed"));
        assert_eq!(turn.message.as_deref(), Some("ok"));
        assert!(live.activity.as_ref().unwrap().standing_question.is_none());

        run.command = true;
        run.running = true;
        apply(&mut live, &run, NOW);
        let activity = live.activity.as_ref().unwrap();
        assert!(activity
            .calls
            .iter()
            .any(|call| call.work == Some(Work::Command)));
        assert!(live.turns.back().unwrap().running);
    }

    #[test]
    fn a_daemon_in_flight_and_serving_runs_a_command() {
        let home = std::env::temp_dir().join(format!("grok-live-{}", std::process::id()));
        std::fs::create_dir_all(home.join(".grokbot")).unwrap();
        let file = home.join(".grokbot/local-exec-daemon.json");
        std::fs::write(
            &file,
            json!({"inflightCount":1,"servingAt":NOW-2_000}).to_string(),
        )
        .unwrap();
        assert!(command_in_flight(&home, NOW));
        std::fs::write(
            &file,
            json!({"inflightCount":0,"servingAt":NOW}).to_string(),
        )
        .unwrap();
        assert!(!command_in_flight(&home, NOW));
        std::fs::write(
            &file,
            json!({"inflightCount":2,"servingAt":NOW-60_000}).to_string(),
        )
        .unwrap();
        assert!(
            !command_in_flight(&home, NOW),
            "a stale daemon serves nothing"
        );
        std::fs::remove_dir_all(&home).ok();
    }
}
