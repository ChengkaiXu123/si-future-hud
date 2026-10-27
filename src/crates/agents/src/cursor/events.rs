//! Cursor's dashboard usage events as sessions: the events' pages, Grok
//! Bot's daily spend, and Grok Bot routine runs linked to their Bot. Pure:
//! `sessions.rs` fetches what these read.
use crate::l10n;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

fn parse_ms(value: &Value) -> Option<i64> {
    let n = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))?;
    (n.is_finite() && n > 0.0 && n <= 253_402_300_799_999.0).then_some(n.round() as i64)
}

fn parse_counter(value: &Value, optional: bool) -> Result<u64, String> {
    if value.is_null() {
        return if optional {
            Ok(0)
        } else {
            Err(l10n::text(
                "用量数据格式无法识别",
                "Usage data has an unsupported format.",
            ))
        };
    }
    if let Some(n) = value.as_u64() {
        return Ok(n);
    }
    if let Some(f) = value.as_f64() {
        if f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f <= u64::MAX as f64 {
            return Ok(f as u64);
        }
    }
    Err(l10n::text(
        "用量数据格式无法识别",
        "Usage data has an unsupported format.",
    ))
}

fn parse_tokens(value: &Value) -> Option<u64> {
    if let Ok(n) = parse_counter(value, false) {
        return Some(n);
    }
    value.as_str().and_then(|s| s.parse::<u64>().ok())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RoutineRun {
    pub agent: String,
    pub name: String,
    pub started_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SessionEvent {
    pub key: String,
    pub model: String,
    pub at: i64,
    pub input: u64,
    pub output: u64,
    pub read: u64,
    pub write: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BilledConversation {
    pub id: String,
    pub raw_conversation: Option<String>,
    pub title: String,
    pub source: &'static str,
    pub archived: bool,
    pub parent: Option<String>,
    pub events: Vec<SessionEvent>,
}

pub(super) fn subagent_id(conversation: &str) -> Option<&str> {
    let rest = conversation.strip_prefix("sand-subagent-")?;
    crate::grok::cache::agent_id(rest).then_some(rest)
}

pub(super) fn reconcile_pages(
    pages: Vec<Vec<Value>>,
    expected: Option<usize>,
) -> Result<Vec<Value>, String> {
    let total: usize = pages.iter().map(Vec::len).sum();
    let Some(expected) = expected else {
        return Ok(pages.into_iter().flatten().collect());
    };
    if total < expected {
        return Err(l10n::text(
            "用量数据格式无法识别",
            "Usage data has an unsupported format.",
        ));
    }
    let mut remove = total - expected;
    let mut iter = pages.into_iter();
    let Some(first) = iter.next() else {
        return if expected == 0 {
            Ok(Vec::new())
        } else {
            Err(l10n::text(
                "用量数据格式无法识别",
                "Usage data has an unsupported format.",
            ))
        };
    };
    let mut previous = first.clone();
    let mut result = first;
    for current in iter {
        let mut overlap = 0;
        if remove > 0 {
            let max_n = previous.len().min(current.len()).min(remove);
            for n in (1..=max_n).rev() {
                if previous[previous.len() - n..] == current[..n] {
                    overlap = n;
                    break;
                }
            }
        }
        result.extend(current.iter().skip(overlap).cloned());
        remove -= overlap;
        previous = current;
    }
    if remove != 0 || result.len() != expected {
        return Err(l10n::text(
            "用量数据格式无法识别",
            "Usage data has an unsupported format.",
        ));
    }
    Ok(result)
}

pub(super) fn parse_events(
    rows: &[Value],
    account: &str,
) -> Result<Vec<BilledConversation>, String> {
    let mut sessions = BTreeMap::<String, BilledConversation>::new();
    let mut occurrences = HashMap::<String, usize>::new();
    for row in rows {
        let Some(usage) = row.get("tokenUsage").filter(|v| v.is_object()) else {
            continue;
        };
        let date = parse_ms(&row["timestamp"]).ok_or_else(|| {
            l10n::text(
                "用量数据格式无法识别",
                "Usage data has an unsupported format.",
            )
        })?;
        let model = row["model"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                l10n::text(
                    "用量数据格式无法识别",
                    "Usage data has an unsupported format.",
                )
            })?;
        let input = parse_counter(&usage["inputTokens"], false)?;
        let output = parse_counter(&usage["outputTokens"], false)?;
        let cache = parse_counter(&usage["cacheReadTokens"], true)?;
        let write = parse_counter(&usage["cacheWriteTokens"], true)?;
        if input.checked_add(write).is_none() {
            return Err(l10n::text(
                "用量数据格式无法识别",
                "Usage data has an unsupported format.",
            ));
        }
        let conversation = row["conversationId"].as_str();
        let identity = crate::grok::cache::hash(&[
            account,
            conversation.unwrap_or(""),
            &date.to_string(),
            model,
            &input.to_string(),
            &output.to_string(),
            &cache.to_string(),
            &write.to_string(),
        ]);
        let ordinal = *occurrences.get(&identity).unwrap_or(&0);
        occurrences.insert(identity.clone(), ordinal + 1);
        let suffix = conversation
            .map(str::to_owned)
            .unwrap_or_else(|| format!("unassigned-{identity}-{ordinal}"));
        let id = format!("cursor-account:{account}:{suffix}");
        let entry = sessions.entry(id.clone()).or_insert_with(|| {
            let subagent = conversation.and_then(subagent_id);
            let title = if let Some(sub) = subagent {
                format!(
                    "{} · {}",
                    l10n::text("Grok Bot 子代理", "Grok Bot subagent"),
                    &sub[..sub.len().min(8)]
                )
            } else if let Some(conv) = conversation {
                format!("Cursor · {}", &conv[..conv.len().min(8)])
            } else {
                l10n::text("Cursor 未归属请求", "Cursor unassigned request")
            };
            BilledConversation {
                id,
                raw_conversation: conversation.map(str::to_owned),
                title,
                source: if subagent.is_some() {
                    "grok-bot"
                } else {
                    "cursor"
                },
                archived: false,
                parent: None,
                events: Vec::new(),
            }
        });
        entry.events.push(SessionEvent {
            key: format!("{identity}:{ordinal}"),
            model: model.to_owned(),
            at: date,
            input,
            output,
            read: cache,
            write,
        });
    }
    Ok(sessions.into_values().collect())
}

pub(super) fn merge_grok_bot_spend(
    reply: &Value,
    account: &str,
    mut sessions: Vec<BilledConversation>,
) -> Vec<BilledConversation> {
    let billed: HashSet<String> = sessions.iter().map(|s| s.id.clone()).collect();
    let mut totals = BTreeMap::<String, BTreeMap<i64, u64>>::new();
    for row in reply["dailySpend"].as_array().into_iter().flatten() {
        let Some(category) = row["category"].as_str() else {
            continue;
        };
        let Some(day) = parse_ms(&row["day"]) else {
            continue;
        };
        let Some(tokens) = parse_tokens(&row["totalTokens"]).filter(|t| *t > 0) else {
            continue;
        };
        let conversation = if category == "Other" || category == "unattributed" {
            "other-bots"
        } else {
            category
        };
        if !crate::grok::cache::agent_id(conversation) {
            continue;
        }
        let id = format!("cursor-account:{account}:{conversation}");
        if billed.contains(&id) {
            continue;
        }
        *totals
            .entry(conversation.to_owned())
            .or_default()
            .entry(day)
            .or_insert(0) += tokens;
    }
    for (conversation, by_day) in totals {
        let events = by_day
            .into_iter()
            .rev()
            .map(|(day, tokens)| SessionEvent {
                key: format!("spend-{conversation}-{}", day / 1000),
                model: "grok-bot".into(),
                at: day,
                input: tokens,
                output: 0,
                read: 0,
                write: 0,
            })
            .collect();
        let subagent = subagent_id(&conversation);
        let is_subagent = subagent.is_some();
        let title = if let Some(sub) = subagent {
            format!(
                "{} · {}",
                l10n::text("Grok Bot 子代理", "Grok Bot subagent"),
                &sub[..sub.len().min(8)]
            )
        } else {
            format!("Cursor · {}", &conversation[..conversation.len().min(8)])
        };
        sessions.push(BilledConversation {
            id: format!("cursor-account:{account}:{conversation}"),
            raw_conversation: Some(conversation),
            title,
            source: if is_subagent { "grok-bot" } else { "cursor" },
            archived: false,
            parent: None,
            events,
        });
    }
    sessions
}

pub(super) fn parse_automations(reply: &Value, now_ms: i64) -> HashMap<String, RoutineRun> {
    let mut result = HashMap::<String, RoutineRun>::new();
    let mut ambiguous = HashSet::<String>::new();
    for agent in reply["agents"].as_array().into_iter().flatten() {
        let Some(owner) = agent["agentId"]
            .as_str()
            .filter(|id| crate::grok::cache::agent_id(id))
        else {
            continue;
        };
        for automation in agent["automations"].as_array().into_iter().flatten() {
            let Some(text) = automation["recordJson"].as_str() else {
                continue;
            };
            let Ok(record) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            let name = record["name"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    record["id"]
                        .as_str()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or("")
                .to_owned();
            for run in record["runs"].as_array().into_iter().flatten() {
                let Some(id) = run["id"]
                    .as_str()
                    .filter(|id| crate::grok::cache::agent_id(id))
                else {
                    continue;
                };
                if let Some(known) = result.get(id) {
                    if known.agent != owner {
                        ambiguous.insert(id.to_owned());
                    }
                }
                result.insert(
                    id.to_owned(),
                    RoutineRun {
                        agent: owner.to_owned(),
                        name: name.clone(),
                        started_at_ms: parse_ms(&run["startedAt"]).unwrap_or(now_ms),
                    },
                );
            }
        }
    }
    result.retain(|k, _| !ambiguous.contains(k));
    result
}

pub(super) fn build_session_records(
    sessions: Vec<BilledConversation>,
    grok_slot_account: Option<&str>,
    roster_agents: &HashSet<String>,
    routine_runs: &HashMap<String, RoutineRun>,
) -> Vec<Value> {
    let mut records = Vec::new();
    for session in sessions {
        let mut source = session.source;
        let mut id = session.id.clone();
        let mut title: Option<String> = Some(session.title.clone());
        let mut parent = session.parent.clone();
        if let (Some(slot), Some(conv)) = (grok_slot_account, session.raw_conversation.as_deref()) {
            if roster_agents.contains(conv) {
                source = "grok-bot";
                id = crate::grok::cache::session_id(slot, conv);
                title = None;
            } else if let Some(sub_id) = subagent_id(conv) {
                source = "grok-bot";
                if let Some(run) = routine_runs.get(sub_id) {
                    if roster_agents.contains(&run.agent) {
                        parent = Some(crate::grok::cache::session_id(slot, &run.agent));
                        title = Some(if run.name.is_empty() {
                            "Routine".into()
                        } else {
                            run.name.clone()
                        });
                    }
                }
            }
        }
        for event in session.events {
            records.push(json!({
                "source": source,
                "id": id,
                "title": title,
                "model": event.model,
                "key": event.key,
                "at": event.at,
                "input": event.input,
                "output": event.output,
                "read": event.read,
                "write": event.write,
                "parent": parent,
                "archived": session.archived,
            }));
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_events_grok_bot_spend_and_automations_correlate() {
        let p1 = vec![json!({"id": 1}), json!({"id": 2})];
        let p2 = vec![json!({"id": 2}), json!({"id": 3})];
        let reconciled = reconcile_pages(vec![p1, p2], Some(3)).unwrap();
        assert_eq!(reconciled.len(), 3);

        let account = crate::grok::cache::hash(&["user_123"]);
        let events = vec![
            json!({
                "timestamp": "1710000000000",
                "model": "claude-3-7-sonnet",
                "conversationId": "comp-1",
                "tokenUsage": {
                    "inputTokens": 120,
                    "outputTokens": 45,
                    "cacheReadTokens": 30,
                    "cacheWriteTokens": 10
                }
            }),
            json!({
                "timestamp": "1710000005000",
                "model": "grok-3",
                "conversationId": "sand-subagent-run_abc123",
                "tokenUsage": {
                    "inputTokens": 200,
                    "outputTokens": 80
                }
            }),
        ];
        let parsed = parse_events(&events, &account).unwrap();
        assert_eq!(parsed.len(), 2);

        let spend = json!({
            "dailySpend": [
                {
                    "category": "bot_main_1",
                    "day": "1710000000000",
                    "totalTokens": "500"
                },
                {
                    "category": "sand-subagent-run_abc123",
                    "day": "1710000000000",
                    "totalTokens": "999"
                }
            ]
        });
        let merged = merge_grok_bot_spend(&spend, &account, parsed);
        // `sand-subagent-run_abc123` was already billed by request-level events, so only `bot_main_1` is added.
        assert_eq!(merged.len(), 3);

        let automations = json!({
            "agents": [
                {
                    "agentId": "bot_main_1",
                    "automations": [
                        {
                            "recordJson": serde_json::to_string(&json!({
                                "id": "auto_1",
                                "name": "Morning Digest",
                                "runs": [
                                    { "id": "run_abc123", "startedAt": 1710000001000i64 }
                                ]
                            })).unwrap()
                        }
                    ]
                }
            ]
        });
        let runs = parse_automations(&automations, 1710000010000);
        assert_eq!(
            runs.get("run_abc123"),
            Some(&RoutineRun {
                agent: "bot_main_1".into(),
                name: "Morning Digest".into(),
                started_at_ms: 1710000001000,
            })
        );

        let roster_set: HashSet<String> = ["bot_main_1".to_owned()].into_iter().collect();
        let records = build_session_records(merged, Some("slot_acct"), &roster_set, &runs);
        assert_eq!(records.len(), 3);
        let parent_bot_id = crate::grok::cache::session_id("slot_acct", "bot_main_1");
        let routine_record = records
            .iter()
            .find(|r| r["parent"] == json!(parent_bot_id))
            .expect("routine subagent linked to parent Grok Bot session");
        assert_eq!(routine_record["source"], "grok-bot");
        assert_eq!(routine_record["title"], "Morning Digest");
        let bot_spend_record = records
            .iter()
            .find(|r| r["id"] == json!(parent_bot_id))
            .expect("dailySpend joined to roster Grok Bot session");
        assert_eq!(bot_spend_record["input"], 500);
    }
}
