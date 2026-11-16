//! Claude's plan limits: Claude Code's own engine answers get_usage (Open's
//! ClaudeEngineUsageClient), and `claude auth status` tells a signed-out
//! engine from an API-key login. The signed-in account is the one Claude
//! Code's .claude.json names, read before and after.
use crate::account::{
    account_id, client_home_key, legacy_account, legacy_unresolved, read_json, status, variable,
    EngineStart, Host, Reading,
};
use crate::l10n;
use crate::naming::window_names;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// What Claude Code's engine gets on top of the user's environment.
fn engine_env() -> [(&'static str, String); 1] {
    [("CLAUDE_CODE_ENTRYPOINT", "agent-hud".into())]
}
/// Claude Code's configuration folder: `CLAUDE_CONFIG_DIR`, or `~/.claude`.
fn config_dir(host: &dyn Host) -> PathBuf {
    variable(host, "CLAUDE_CONFIG_DIR").unwrap_or_else(|| host.home().join(".claude"))
}
fn profile(host: &dyn Host) -> Value {
    let path = variable(host, "CLAUDE_CONFIG_DIR")
        .unwrap_or_else(|| host.home().to_path_buf())
        .join(".claude.json");
    read_json(host, &path).unwrap_or(Value::Null)["oauthAccount"].clone()
}
fn identity(value: &Value) -> Option<String> {
    let user = value["accountUuid"].as_str().unwrap_or_default();
    let organization = value["organizationUuid"].as_str().unwrap_or_default();
    (!user.is_empty() || !organization.is_empty())
        .then(|| account_id("claude", &[user, organization]))
}
fn plan(profile: &Value, fallback: Option<&str>) -> Option<String> {
    let fallback = fallback?;
    let mapped = match profile["organizationType"].as_str() {
        Some("claude_max") => "max",
        Some("claude_pro") => "pro",
        Some("claude_team") => "team",
        Some("claude_enterprise") => "enterprise",
        _ => fallback,
    };
    let value = if mapped == "max" && profile["organizationType"] == "claude_max" {
        match profile["userRateLimitTier"]
            .as_str()
            .or_else(|| profile["organizationRateLimitTier"].as_str())
        {
            Some("default_claude_max_5x") => "max_5x",
            Some("default_claude_max_20x") => "max_20x",
            _ => mapped,
        }
    } else {
        mapped
    };
    Some(value.into())
}
/// Engine failures in Open's words (agent-hud-native-open ClaudeDataError).
fn engine_failed(reason: &str) -> String {
    if l10n::chinese() {
        format!("Claude Code 引擎查询失败：{reason}")
    } else {
        format!("Claude Code engine query failed: {reason}")
    }
}
fn malformed() -> String {
    l10n::text(
        "Claude Code 引擎返回了无法识别的额度数据",
        "The Claude Code engine returned unrecognised usage data",
    )
}
pub fn read(host: &dyn Host) -> Result<Reading, String> {
    let executable = host.executable("claude").ok_or_else(|| {
        l10n::text(
            "未找到 Claude Code 引擎（claude 命令）",
            "Claude Code engine (claude command) not found",
        )
    })?;
    let before = profile(host);
    let mut process = host.start_engine(&EngineStart {
        executable: &executable,
        args: &[
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--settings",
            r#"{"disableAllHooks":true}"#,
        ],
        folder: "claude",
        env: &engine_env(),
    })?;
    process.send(&json!({"type":"control_request","request_id":"agent-hud-usage","request":{"subtype":"get_usage","skip_behaviors":true}}))?;
    let deadline = Instant::now() + Duration::from_secs(40);
    while let Some(message) = process.next(deadline) {
        if message["type"] != "control_response" {
            continue;
        }
        if message["response"]["subtype"] == "error" {
            return Err(engine_failed(
                message["response"]["error"]
                    .as_str()
                    .unwrap_or("unknown error"),
            ));
        }
        let payload = &message["response"]["response"];
        if !payload.is_object() {
            return Err(engine_failed("empty get_usage response"));
        }
        let after = profile(host);
        if identity(&before) != identity(&after) {
            return Err(l10n::text(
                "读取额度时 Claude 登录账户发生了变化，稍后自动重试",
                "The Claude account changed during the quota query; retrying shortly",
            ));
        }
        // Plan limits apply but the engine came back without numbers: its own
        // usage lookup failed (refused for asking too often, or an error) and
        // it had nothing seen earlier to answer with. That is a failed read,
        // so the last reading stays, not a login without plan limits.
        if payload["rate_limits_available"] == true && !payload["rate_limits"].is_object() {
            return Err(l10n::text(
                "这次没有读到 Claude 额度，稍后自动重试",
                "Couldn't read the Claude quota this time; retrying shortly",
            ));
        }
        let mut reading = Reading::empty("claude");
        if payload["rate_limits_available"] != true {
            // get_usage reports no plan limits both for an API-key login and
            // for no login at all; only `auth status` tells the two apart.
            let signed_in = host
                .start_engine(&EngineStart {
                    executable: &executable,
                    args: &["auth", "status", "--json"],
                    folder: "claude",
                    env: &engine_env(),
                })
                .ok()
                .and_then(|mut engine| engine.joined(Instant::now() + Duration::from_secs(10)))
                .and_then(|val| val["loggedIn"].as_bool());
            if let Some(reported) = reported(host, signed_in == Some(false)) {
                return Ok(reported);
            }
            reading.notice = Some(if signed_in == Some(false) {
                l10n::text(
                    "终端的 claude 没登录（和桌面 App 的登录分开）。在终端运行 claude 登录后恢复额度",
                    "The terminal claude is signed out (separate from the desktop app's sign-in). Run claude in a terminal to sign in and the quota returns",
                )
            } else {
                l10n::text(
                    "当前登录方式没有订阅额度（API key 或第三方平台）",
                    "No plan limits for this login (API key or third-party platform)",
                )
            });
            return Ok(reading);
        }
        let windows = windows(&payload["rate_limits"]).ok_or_else(malformed)?;
        let plan = plan(&after, payload["subscription_type"].as_str());
        fill(&mut reading, host, &after, plan, windows, None);
        return Ok(reading);
    }
    let reason = process.stderr_preview(200).unwrap_or_else(|| {
        l10n::text(
            "引擎没有在 40 秒内返回用量",
            "The engine returned no usage within 40 s",
        )
    });
    Err(engine_failed(&reason))
}

/// The account and its windows on `reading`, the account as `profile` names
/// it; each window `observed` then, else when the platform checked.
fn fill(
    reading: &mut Reading,
    host: &dyn Host,
    profile: &Value,
    plan: Option<String>,
    windows: Vec<Window>,
    observed: Option<&str>,
) {
    let id = identity(profile).unwrap_or_else(|| {
        account_id(
            "claude",
            &["unresolved", config_dir(host).to_string_lossy().as_ref()],
        )
    });
    let label = profile["emailAddress"].as_str();
    let legacy = legacy_account(
        "Claude",
        profile["accountUuid"].as_str().unwrap_or(""),
        profile["organizationUuid"].as_str().unwrap_or(""),
    );
    let account_identity = legacy.unwrap_or_else(|| {
        legacy_unresolved(
            "Claude",
            &client_home_key(host, &config_dir(host), &host.home().join(".claude")),
        )
    });
    reading.account =
        Some(json!({"id":account_identity,"provider":"claude","label":label,"plan":plan}));
    for window in windows {
        let Window {
            key,
            name,
            percent,
            reset,
            family,
        } = window;
        // "5h", "Weekly", and a family's weekly window by the family alone.
        let short = match family.as_deref() {
            Some(_) => window_names::leading(name.trim_start_matches("Weekly limit · ")),
            None if key == "claude-session" => window_names::Period::FiveHours.short_name(),
            None => window_names::Period::Week.short_name(),
        };
        reading.quotas.push(json!({"id":format!("{id}:{key}"),"vendor":"claude","name":name,"shortLabel":short,"usedPercent":percent,"resetAt":reset,"status":status(percent),"account":label,"accountId":account_identity,"legacyWindowId":format!("{account_identity}/{key}"),"consumerSource":"claude","windowDurationSeconds":if key=="claude-session"{5*3600}else{7*86400},"allModels":family.is_none(),"modelFamilies":family.map(|family|vec![family]),"observedAt":observed}));
    }
}

/// Claude's plan windows as a Claude Code session running the HUD's plugin
/// last reported them (`$.session.usage()`): that session's own sign-in, so
/// the desktop app's sessions, signed in apart from the terminal `claude`,
/// count too. The 5-hour and weekly windows only, none of a model's own.
/// None when no session reported any.
fn reported(host: &dyn Host, signed_out: bool) -> Option<Reading> {
    let report = host.account_reports("claude").into_iter().next()?;
    let at = chrono::DateTime::from_timestamp_millis(report["at"].as_i64()?)?;
    let mut windows = Vec::new();
    for window in report["windows"].as_array()? {
        let (key, name) = match window["kind"].as_str() {
            Some("five_hour") => ("claude-session", "Current session"),
            Some("seven_day") => ("claude-weekly", "Weekly limit · All models"),
            _ => continue,
        };
        let Some(percent) = window["percentUsed"].as_f64().filter(|p| p.is_finite()) else {
            continue;
        };
        let reset = window["resetsAt"]
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.to_utc().to_rfc3339());
        windows.push(Window {
            key: key.into(),
            name: name.into(),
            percent,
            reset,
            family: None,
        });
    }
    if windows.is_empty() {
        return None;
    }
    // The session names its account where the desktop app started it; a
    // session that doesn't is the account Claude Code's .claude.json names.
    let saved = profile(host);
    let named = json!({
        "accountUuid": report["accountUuid"],
        "organizationUuid": report["organizationUuid"],
        "emailAddress": report["email"],
    });
    let account = match identity(&named) {
        Some(id) if identity(&saved).as_deref() != Some(id.as_str()) => named,
        _ => saved,
    };
    let plan = plan(&account, Some("")).filter(|plan| !plan.is_empty());
    let mut reading = Reading::empty("claude");
    let observed = at.to_rfc3339();
    fill(&mut reading, host, &account, plan, windows, Some(&observed));
    let local = at.with_timezone(&chrono::Local);
    let time = if local.date_naive() == chrono::Local::now().date_naive() {
        local.format("%H:%M")
    } else {
        local.format("%m-%d %H:%M")
    };
    reading.notice = Some(if signed_out {
        l10n::text(
            &format!("终端的 claude 没登录，这里是 Claude Code 会话在 {time} 报告的额度"),
            &format!("The terminal claude is signed out; these are the figures a Claude Code session reported at {time}"),
        )
    } else {
        l10n::text(
            &format!("终端 claude 的登录方式没有订阅额度，这里是 Claude Code 会话在 {time} 报告的额度"),
            &format!("The terminal claude's login has no plan limits; these are the figures a Claude Code session reported at {time}"),
        )
    });
    Some(reading)
}

/// One plan window as the engine reports it.
struct Window {
    key: String,
    name: String,
    percent: f64,
    reset: Option<String>,
    family: Option<String>,
}

/// The model families Claude names (agent-hud-native-open ClaudeModelInfo.families).
const FAMILIES: [&str; 5] = ["opus", "sonnet", "haiku", "fable", "mythos"];

/// A model id's or name's family, lowercased: the first of its words that
/// names one (agent-hud-native-open ClaudeModelInfo.parse).
fn family(model: &str) -> Option<&'static str> {
    model
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .find_map(|word| FAMILIES.into_iter().find(|family| *family == word))
}

/// The windows of a `rate_limits` object in the order Claude Code's own
/// /usage screen shows them: the session, the week across models, then each
/// model family's week, families in alphabetical order (agent-hud-native-open
/// ClaudeUsage.rows). A family's week comes from `seven_day_<family>` or a
/// `limits[]` entry of kind `weekly_scoped`, which wins. None when the
/// object has neither the session nor the weekly window, as Open reads it.
fn windows(limits: &Value) -> Option<Vec<Window>> {
    let window = |object: &Value, value_key: &str| {
        let percent = object[value_key]
            .as_f64()
            .filter(|percent| percent.is_finite())?;
        let reset = object["resets_at"]
            .as_str()
            .filter(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok())
            .map(str::to_owned);
        Some((percent, reset))
    };
    let session = window(&limits["five_hour"], "utilization");
    let week = window(&limits["seven_day"], "utilization");
    if session.is_none() && week.is_none() {
        return None;
    }
    // Open reads `seven_day_opus` and `seven_day_sonnet`; the other families'
    // keys are read too, so a family window the service adds is not missed.
    let mut families = std::collections::BTreeMap::<String, (f64, Option<String>)>::new();
    for family in FAMILIES {
        if let Some(found) = window(
            &limits[format!("seven_day_{family}").as_str()],
            "utilization",
        ) {
            families.insert(family.into(), found);
        }
    }
    for entry in limits["limits"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["kind"] == "weekly_scoped")
    {
        let model = &entry["scope"]["model"];
        let name = model["display_name"]
            .as_str()
            .or_else(|| model["id"].as_str());
        let Some(key) = name
            .and_then(family)
            .map(str::to_owned)
            .or_else(|| model["display_name"].as_str().map(str::to_lowercase))
        else {
            continue;
        };
        if let Some(found) = window(entry, "percent") {
            families.insert(key, found);
        }
    }
    let mut rows = Vec::new();
    for (key, name, found) in [
        ("claude-session", "Current session", session),
        ("claude-weekly", "Weekly limit · All models", week),
    ] {
        if let Some((percent, reset)) = found {
            rows.push(Window {
                key: key.into(),
                name: name.into(),
                percent,
                reset,
                family: None,
            });
        }
    }
    for (family, (percent, reset)) in families {
        let mut letters = family.chars();
        let capitalized = letters
            .next()
            .map(|first| first.to_uppercase().chain(letters).collect::<String>())
            .unwrap_or_default();
        rows.push(Window {
            key: format!("claude-weekly-{family}"),
            name: format!("Weekly limit · {capitalized}"),
            percent,
            reset,
            family: Some(family),
        });
    }
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::fake::FakeHost;

    fn installed(usage: Value) -> FakeHost {
        let mut host = FakeHost::new("/u").file(
            "/u/.claude.json",
            &json!({"oauthAccount": {
                "accountUuid": "user-1", "organizationUuid": "org-1",
                "emailAddress": "me@example.com", "organizationType": "claude_max",
                "userRateLimitTier": "default_claude_max_20x",
            }})
            .to_string(),
        );
        host.executables
            .insert("claude".into(), "/bin/claude".into());
        host.engine_output.insert(
            "-p".into(),
            vec![
                json!({"type": "system"}),
                json!({"type": "control_response", "response": usage}),
            ],
        );
        host
    }

    fn limits() -> Value {
        json!({"subtype": "success", "response": {
            "rate_limits_available": true, "subscription_type": "max",
            "rate_limits": {"five_hour": {"utilization": 12.0}, "seven_day": {"utilization": 40.0}},
        }})
    }

    #[test]
    fn the_engine_reports_the_plans_windows() {
        let host = installed(limits());
        let reading = read(&host).unwrap();
        let identity = legacy_account("Claude", "user-1", "org-1").unwrap();
        let account = reading.account.unwrap();
        assert_eq!(account["id"], identity.as_str());
        assert_eq!(account["label"], "me@example.com");
        assert_eq!(account["plan"], "max_20x");
        assert_eq!(reading.quotas.len(), 2);
        let id = account_id("claude", &["user-1", "org-1"]);
        assert_eq!(reading.quotas[0]["id"], format!("{id}:claude-session"));
        assert_eq!(reading.quotas[0]["windowDurationSeconds"], 18000);
        let started = host.started.borrow();
        assert_eq!(started[0].0, PathBuf::from("/bin/claude"));
        assert_eq!(started[0].2, "claude");
        assert!(started[0]
            .1
            .contains(&r#"{"disableAllHooks":true}"#.to_owned()));
        assert_eq!(
            started[0].3,
            [("CLAUDE_CODE_ENTRYPOINT".to_owned(), "agent-hud".to_owned())]
        );
        assert_eq!(
            host.engine_sent.borrow()[0]["request"]["subtype"],
            "get_usage"
        );
    }

    #[test]
    fn without_plan_limits_auth_status_tells_signed_out_from_api_keys() {
        let none = json!({"subtype": "success", "response": {"rate_limits_available": false}});
        let mut host = installed(none.clone());
        host.engine_output
            .insert("auth".into(), vec![json!({"loggedIn": false})]);
        assert_eq!(
            read(&host).unwrap().notice.as_deref(),
            Some("The terminal claude is signed out (separate from the desktop app's sign-in). Run claude in a terminal to sign in and the quota returns")
        );
        let mut host = installed(none);
        host.engine_output
            .insert("auth".into(), vec![json!({"loggedIn": true})]);
        assert_eq!(
            read(&host).unwrap().notice.as_deref(),
            Some("No plan limits for this login (API key or third-party platform)")
        );
        assert_eq!(host.started.borrow()[1].1, ["auth", "status", "--json"]);
    }

    #[test]
    fn a_terminal_that_cant_read_the_quota_shows_what_a_session_reported() {
        let none = json!({"subtype": "success", "response": {"rate_limits_available": false}});
        let report = |account: &str| {
            json!({
                "at": 1_793_700_000_000_i64, "session": "s1",
                "accountUuid": account, "organizationUuid": "org-1", "email": "desk@example.com",
                "windows": [
                    {"kind": "five_hour", "percentUsed": 23.5, "resetsAt": "2026-11-03T15:00:00+08:00"},
                    {"kind": "seven_day", "percentUsed": 41, "resetsAt": "2026-11-07T00:00:00Z"},
                    {"kind": "spend_limit", "percentUsed": 10},
                ],
            })
        };
        // The desktop app's session names the account .claude.json names: its plan too.
        let mut host = installed(none.clone());
        host.engine_output
            .insert("auth".into(), vec![json!({"loggedIn": false})]);
        host.account_reports
            .insert("claude".into(), vec![report("user-1")]);
        let reading = read(&host).unwrap();
        assert!(reading
            .notice
            .as_deref()
            .unwrap()
            .starts_with("The terminal claude is signed out; these are the figures a Claude Code session reported at "));
        assert_eq!(reading.account.as_ref().unwrap()["plan"], "max_20x");
        assert_eq!(reading.account.as_ref().unwrap()["label"], "me@example.com");
        let rows: Vec<_> = reading
            .quotas
            .iter()
            .map(|row| {
                (
                    row["name"].clone(),
                    row["usedPercent"].clone(),
                    row["resetAt"].clone(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                (
                    json!("Current session"),
                    json!(23.5),
                    json!("2026-11-03T07:00:00+00:00")
                ),
                (
                    json!("Weekly limit · All models"),
                    json!(41.0),
                    json!("2026-11-07T00:00:00+00:00")
                ),
            ]
        );
        assert_eq!(reading.quotas[0]["observedAt"], "2026-11-03T10:00:00+00:00");
        assert_eq!(reading.quotas[0]["shortLabel"], "5h");

        // Another account than .claude.json's: its own name, no plan known.
        let mut host = installed(none.clone());
        host.engine_output
            .insert("auth".into(), vec![json!({"loggedIn": true})]);
        host.account_reports
            .insert("claude".into(), vec![report("user-2")]);
        let reading = read(&host).unwrap();
        assert!(reading
            .notice
            .as_deref()
            .unwrap()
            .starts_with("The terminal claude's login has no plan limits; "));
        assert_eq!(
            reading.account.as_ref().unwrap()["label"],
            "desk@example.com"
        );
        assert_eq!(reading.account.as_ref().unwrap()["plan"], Value::Null);

        // A report with no plan window is no reading.
        let mut host = installed(none);
        host.engine_output
            .insert("auth".into(), vec![json!({"loggedIn": false})]);
        host.account_reports.insert(
            "claude".into(),
            vec![json!({"at": 1_793_700_000_000_i64, "windows": [{"kind": "spend_limit", "percentUsed": 10}]})],
        );
        let reading = read(&host).unwrap();
        assert!(reading.quotas.is_empty());
        assert!(reading
            .notice
            .unwrap()
            .starts_with("The terminal claude is signed out (separate"));
    }

    #[test]
    fn plan_limits_without_numbers_are_a_failed_read_not_a_login_without_limits() {
        // Claude Code 2.1.307 answers this way when its usage lookup fails and
        // it has no numbers seen earlier; `auth status` is not asked.
        let empty = json!({"subtype": "success", "response": {
            "rate_limits_available": true, "subscription_type": "max", "rate_limits": null,
        }});
        let host = installed(empty);
        assert_eq!(
            read(&host).err().unwrap(),
            "Couldn't read the Claude quota this time; retrying shortly"
        );
        assert_eq!(host.started.borrow().len(), 1);
    }

    #[test]
    fn engine_failures_are_worded_as_open_words_them() {
        let host = installed(json!({"subtype": "error", "error": "boom"}));
        assert_eq!(
            read(&host).err().unwrap(),
            "Claude Code engine query failed: boom"
        );
        let mut silent = installed(limits());
        silent.engine_output.clear();
        assert_eq!(
            read(&silent).err().unwrap(),
            "Claude Code engine query failed: The engine returned no usage within 40 s"
        );
        assert_eq!(
            read(&FakeHost::new("/u")).err().unwrap(),
            "Claude Code engine (claude command) not found"
        );
    }

    #[test]
    fn a_login_without_an_account_is_the_config_folders() {
        let mut host = installed(limits());
        host.files.clear();
        let reading = read(&host).unwrap();
        assert_eq!(
            reading.account.unwrap()["id"],
            legacy_unresolved("Claude", "").as_str()
        );
        let config = host.home().join(".claude");
        let id = account_id("claude", &["unresolved", &config.to_string_lossy()]);
        assert_eq!(reading.quotas[0]["id"], format!("{id}:claude-session"));
    }

    #[test]
    fn windows_follow_claude_codes_order_with_families_alphabetical() {
        let limits = json!({
            "five_hour": {"utilization": 12.0, "resets_at": "2026-10-21T15:00:00Z"},
            "seven_day": {"utilization": 40.0, "resets_at": "2026-10-25T00:00:00Z"},
            "seven_day_sonnet": {"utilization": 5.0},
            "seven_day_opus": {"utilization": 30.0},
            "limits": [
                {"kind": "weekly_scoped", "percent": 55.0, "scope": {"model": {"display_name": "Claude Fable 5"}}},
                {"kind": "weekly_scoped", "percent": 31.0, "scope": {"model": {"id": "claude-opus-5-1"}}},
                {"kind": "session", "percent": 99.0}
            ]
        });
        let rows = windows(&limits).unwrap();
        let keys = rows.iter().map(|row| row.key.as_str()).collect::<Vec<_>>();
        assert_eq!(
            keys,
            [
                "claude-session",
                "claude-weekly",
                "claude-weekly-fable",
                "claude-weekly-opus",
                "claude-weekly-sonnet"
            ]
        );
        assert_eq!(rows[2].name, "Weekly limit · Fable");
        assert_eq!(
            rows[3].percent, 31.0,
            "a weekly_scoped entry wins over seven_day_<family>"
        );
        assert_eq!(rows[0].reset.as_deref(), Some("2026-10-21T15:00:00Z"));
    }

    #[test]
    fn a_reading_without_the_session_or_weekly_window_is_unrecognised() {
        let only_family = json!({"seven_day_opus": {"utilization": 30.0}, "five_hour": null});
        assert!(windows(&only_family).is_none());
        let weekly_only = json!({"seven_day": {"utilization": 1}});
        assert_eq!(windows(&weekly_only).unwrap().len(), 1);
    }
}
