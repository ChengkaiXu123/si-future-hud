//! Cursor's sessions from its dashboard (Open's CursorClient): the last
//! seven days of usage events, Grok Bot's daily spend, local titles from
//! Cursor's database and chats, and Grok Bot routine runs remembered in
//! Agent HUD's own data.
use super::events::{
    build_session_records, merge_grok_bot_spend, parse_automations, parse_events, reconcile_pages,
    subagent_id, BilledConversation, RoutineRun,
};
use crate::account::{locate, Host, HttpFailure, HttpRequest};
use crate::grok::cache as bot;
use crate::l10n;
use chrono::Utc;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::{Duration, Instant},
};

/// Where Agent HUD remembers Grok Bot routine runs between readings.
const ROUTINE_RUNS: &str = "grok-bot-routine-runs.json";
/// A local chat's meta.json, or the remembered runs, read at most this long.
const LIMIT: u64 = 16 * 1024 * 1024;

fn auth_format() -> String {
    l10n::text(
        "Cursor 账户登录信息格式无法识别",
        "Cursor account authentication has an unsupported format.",
    )
}

pub(super) fn resolve_local_names(
    host: &dyn Host,
    sessions: &mut [BilledConversation],
    db_path: &Path,
    agent_folder: &Path,
) {
    let conversations: Vec<String> = sessions
        .iter()
        .filter_map(|s| s.raw_conversation.clone())
        .collect();
    if conversations.is_empty() {
        return;
    }
    let mut titles = HashMap::<String, String>::new();
    let mut archived = HashSet::<String>::new();
    if let Ok(list_json) = serde_json::to_string(&conversations) {
        if let Ok(rows) = host.sqlite_rows(
            db_path,
            "SELECT composerId, json_extract(value, '$.name'), CAST(json_extract(value, '$.isArchived') AS TEXT) FROM composerHeaders WHERE composerId IN (SELECT value FROM json_each(?1))",
            &[&list_json],
            Duration::from_secs(2),
        ) {
            for row in rows {
                let Some(id) = row[0].as_str() else {
                    continue;
                };
                if let Some(name) = row[1].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    titles.insert(id.to_owned(), name.to_owned());
                }
                if matches!(row[2].as_str(), Some("1" | "true")) {
                    archived.insert(id.to_owned());
                }
            }
        }
    }
    let mut chat_dirs = vec![agent_folder.join("acp-sessions")];
    chat_dirs.extend(
        host.list_dir(&agent_folder.join("chats"))
            .found()
            .unwrap_or_default(),
    );
    for conv in &conversations {
        if titles.contains_key(conv) {
            continue;
        }
        for folder in &chat_dirs {
            let meta = folder.join(conv).join("meta.json");
            let Some(bytes) = host
                .read_file(&meta, LIMIT)
                .found()
                .filter(|bytes| bytes.len() as u64 <= LIMIT)
            else {
                continue;
            };
            let Ok(json) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            if let Some(title) = json["title"]
                .as_str()
                .map(str::trim)
                .filter(|t| !t.is_empty() && *t != "New Agent")
            {
                titles.insert(conv.clone(), title.to_owned());
                break;
            }
        }
    }
    for session in sessions {
        let Some(conv) = session.raw_conversation.as_ref() else {
            continue;
        };
        if let Some(title) = titles.get(conv) {
            session.title = title.clone();
        }
        if archived.contains(conv) {
            session.archived = true;
        }
    }
}

/// Grok Bot's signed-in slot and its roster of Bots, from the Bot's cache.
fn grok_roster(host: &dyn Host) -> (Option<String>, Vec<String>) {
    let Ok(dir) = locate(host, &crate::grok::account::CACHE) else {
        return (None, Vec::new());
    };
    let Some(account) = bot::account_through(host, &dir) else {
        return (None, Vec::new());
    };
    let key = bot::roster_key(&account);
    let Some(roster) = bot::value_through(host, &dir, &key, 4) else {
        return (Some(account), Vec::new());
    };
    let mut seen = HashSet::new();
    let mut agents = Vec::new();
    for row in roster["rows"].as_array().into_iter().flatten() {
        if let Some(id) = row["id"].as_str().filter(|id| bot::agent_id(id)) {
            if seen.insert(id.to_owned()) {
                agents.push(id.to_owned());
            }
        }
    }
    if bot::account_through(host, &dir).as_deref() != Some(account.as_str()) {
        return (None, Vec::new());
    }
    (Some(account), agents)
}

fn fetch_events(
    host: &dyn Host,
    cookie: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<Value>, String> {
    let mut pages = Vec::new();
    let mut expected: Option<usize> = None;
    let mut complete = false;
    let deadline = Instant::now() + Duration::from_secs(25);
    for page in 1..=200 {
        if Instant::now() >= deadline {
            return Err(l10n::text(
                "Cursor 会话读取已达上限",
                "Cursor session read limit reached.",
            ));
        }
        let body = host
            .http_json(&HttpRequest {
                url: "https://cursor.com/api/dashboard/get-filtered-usage-events",
                secrets: &[("Cookie", cookie)],
                headers: &[("Origin", "https://cursor.com")],
                body: Some(&json!({
                    "page": page,
                    "pageSize": 1000,
                    "startDate": start_ms.to_string(),
                    "endDate": end_ms.to_string(),
                })),
                timeout: Duration::from_secs(15),
            })
            .map_err(|failure| match failure {
                HttpFailure::BadHeader => auth_format(),
                HttpFailure::Unreachable => l10n::text(
                    "无法连接 Cursor 用量记录",
                    "Cursor dashboard events could not be reached.",
                ),
                other => other.message(),
            })?;
        let count = body["totalUsageEventsCount"]
            .as_u64()
            .or_else(|| {
                body["totalUsageEventsCount"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .map(|c| c as usize);
        if let Some(count) = count {
            if expected.is_some_and(|exp| exp != count) {
                return Err(l10n::text(
                    "Cursor 用量记录的总数前后不一致",
                    "Cursor dashboard events have an inconsistent total count.",
                ));
            }
            expected = Some(count);
        }
        let events = if let Some(arr) = body["usageEventsDisplay"].as_array() {
            arr.clone()
        } else if body.as_object().is_some_and(|o| o.is_empty()) {
            if expected.is_some_and(|exp| exp != 0) {
                return Err(l10n::text(
                    "Cursor 用量记录格式无法识别",
                    "Cursor dashboard events have an unsupported format.",
                ));
            }
            expected = Some(0);
            Vec::new()
        } else if body
            .as_object()
            .is_some_and(|o| o.len() == 1 && o.contains_key("totalUsageEventsCount"))
            && count.is_some()
        {
            Vec::new()
        } else {
            return Err(l10n::text(
                "Cursor 用量记录格式无法识别",
                "Cursor dashboard events have an unsupported format.",
            ));
        };
        let len = events.len();
        pages.push(events);
        if len < 1000 {
            complete = true;
            break;
        }
    }
    if !complete {
        return Err(l10n::text(
            "Cursor 会话读取已达上限",
            "Cursor session read limit reached.",
        ));
    }
    reconcile_pages(pages, expected)
}

/// A POST to Cursor's API server with the access token.
fn connect(
    host: &dyn Host,
    token: &str,
    service: &str,
    body: &Value,
    seconds: u64,
) -> Result<Value, HttpFailure> {
    host.http_json(&HttpRequest {
        url: &format!("https://api2.cursor.sh/aiserver.v1.{service}"),
        secrets: &[("Authorization", &format!("Bearer {token}"))],
        headers: &[("Connect-Protocol-Version", "1")],
        body: Some(body),
        timeout: Duration::from_secs(seconds),
    })
}

fn fetch_daily_spend(host: &dyn Host, token: &str, start_ms: i64, end_ms: i64) -> Option<Value> {
    let mut body = json!({
        "periodStartMs": start_ms.to_string(),
        "periodEndMs": end_ms.to_string(),
        "groupBy": "GROK_BOT",
        "spendType": "ALL",
        "clientType": "grok-bot",
        "products": ["sand"],
        "includeCreditUsage": true,
    });
    match connect(host, token, "DashboardService/GetMe", &json!({}), 5) {
        Ok(me) => {
            if let Some(uid) = me["userId"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| me["userId"].as_i64().map(|n| n.to_string()))
            {
                body["userId"] = json!(uid);
            }
        }
        // A token the HTTP layer refuses is refused for the spend too.
        Err(HttpFailure::BadHeader) => return None,
        Err(_) => {}
    }
    connect(
        host,
        token,
        "DashboardService/GetDailySpendByCategory",
        &body,
        12,
    )
    .ok()
}

fn refresh_routine_runs(
    host: &dyn Host,
    token: &str,
    sessions: &[BilledConversation],
    roster_agents: &[String],
    now_ms: i64,
) -> HashMap<String, RoutineRun> {
    let mut runs: HashMap<String, RoutineRun> = host
        .read_own(ROUTINE_RUNS, LIMIT)
        .found()
        .filter(|bytes| bytes.len() as u64 <= LIMIT)
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let has_unknown_subagent = sessions
        .iter()
        .filter_map(|s| s.raw_conversation.as_deref().and_then(subagent_id))
        .any(|sub_id| !runs.contains_key(sub_id));
    if !has_unknown_subagent || roster_agents.is_empty() {
        return runs;
    }
    let Ok(reply) = connect(
        host,
        token,
        "GrokBotService/ListGrokBotAccountAutomations",
        &json!({ "agentIds": roster_agents }),
        12,
    ) else {
        return runs;
    };
    let cutoff = now_ms - 30 * 86_400_000;
    for (k, v) in parse_automations(&reply, now_ms) {
        runs.insert(k, v);
    }
    runs.retain(|_, v| v.started_at_ms >= cutoff);
    if let Ok(bytes) = serde_json::to_vec(&runs) {
        let _ = host.write_own(ROUTINE_RUNS, &bytes);
    }
    runs
}

pub(super) fn read_sessions(
    host: &dyn Host,
    database: &Path,
    user: &str,
    token: &str,
    cookie: &str,
) -> Result<Vec<Value>, String> {
    let now_ms = Utc::now().timestamp_millis();
    let start_ms = ((now_ms - 7 * 86_400_000) / 86_400_000) * 86_400_000;
    let account = bot::hash(&[user]);
    let events = fetch_events(host, cookie, start_ms, now_ms)?;
    let mut sessions = parse_events(&events, &account)?;
    if let Some(spend) = fetch_daily_spend(host, token, start_ms, now_ms) {
        sessions = merge_grok_bot_spend(&spend, &account, sessions);
    }
    resolve_local_names(host, &mut sessions, database, &host.home().join(".cursor"));
    let (slot, roster_agents) = grok_roster(host);
    let runs = refresh_routine_runs(host, token, &sessions, &roster_agents, now_ms);
    let roster_set: HashSet<String> = roster_agents.into_iter().collect();
    Ok(build_session_records(
        sessions,
        slot.as_deref(),
        &roster_set,
        &runs,
    ))
}
