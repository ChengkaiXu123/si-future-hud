//! Grok Bot's personal quota, from the cache the Bot keeps itself. When the
//! Bot read it stays the observation time: reading the file now does not
//! make old data fresh.
use super::cache as bot;
use crate::account::{legacy_unresolved, locate, status, Host, Reading};
use crate::naming::window_names;
use crate::{l10n, Location, PerOs, Support};
use serde_json::json;

/// Grok Bot's desktop cache, where it is known.
pub const CACHE_LOCATION: Location = Location::app_data(&["Grok Bot"], "sand-client-persistence");
/// Grok Bot's desktop cache.
pub const CACHE: PerOs<Location> = PerOs {
    macos: Support::Verified(CACHE_LOCATION),
    windows: Support::Unsupported("Grok Bot's cache location on Windows is not known yet"),
    linux: Support::Unsupported("Grok Bot's cache location on Linux is not known yet"),
};

fn iso(ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d: chrono::DateTime<chrono::Utc>| d.to_rfc3339())
}

pub fn read(host: &dyn Host) -> Result<Reading, String> {
    let mut r = Reading::empty("grok-bot");
    let Ok(directory) = locate(host, &CACHE) else {
        r.notice = Some(l10n::text(
            "此系统暂不支持读取 Grok Bot 缓存",
            "Grok Bot cache acquisition is not implemented on this platform",
        ));
        return Ok(r);
    };
    let Some(slot) = bot::account_through(host, &directory) else {
        r.notice = Some(l10n::text(
            "登录 Grok Bot 后读取其缓存的个人额度",
            "Sign in to Grok Bot to expose its cached personal quota",
        ));
        return Ok(r);
    };
    let value =
        bot::value_through(host, &directory, &bot::quota_key(&slot), 2).ok_or_else(|| {
            l10n::text(
                "Grok Bot 缓存额度不可用，请在 Bot 中刷新",
                "Grok Bot cached quota is unavailable; refresh in Bot",
            )
        })?;
    let now = chrono::Utc::now().timestamp_millis();
    let (expires, observed) = (
        value["expiresAtMs"].as_i64(),
        value["reading"]["readAtMs"].as_i64(),
    );
    let u = &value["reading"]["usage"];
    if value["kind"] != "present"
        || !value["selectedTeamId"].is_null()
        || expires.is_none_or(|e| e <= now)
        || observed.is_none_or(|at| at > now || now - at >= 86_400_000)
        || expires <= observed
        || u["hasNonZeroIncludedLimit"] != true
        || !u["isSandTrial"].is_boolean()
        || u["isTeamSeat"] == true
    {
        return Err(l10n::text(
            "Grok Bot 的个人额度缓存已过期，或在当前团队选择下不可用",
            "Grok Bot personal quota cache is expired or unavailable for this team selection",
        ));
    }
    // The Swift app's account for this Bot slot; its rows are that app's `grok` windows.
    let id = legacy_unresolved("Grok", &format!("grok-bot:{}", bot::hash(&[&slot])));
    let used = u["percentUsed"]
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.)
        .map(|n| n.clamp(0., 100.));
    let reset = u["nextResetMs"].as_i64().and_then(iso);
    if !u["nextResetMs"].is_null() && reset.is_none() {
        return Err(l10n::text(
            "Grok Bot 缓存的重置时间无效",
            "Grok Bot cache reset time is invalid",
        ));
    }
    let trial = u["isSandTrial"] == true;
    r.quotas.push(json!({"id":format!("{id}/grok"),"vendor":"grok-bot","accountId":id,"legacyWindowId":format!("{id}/grok"),"name":if trial{"Trial usage limit"}else{"Weekly usage limit"},"shortLabel":if trial{"Trial".to_owned()}else{window_names::Period::Week.short_name()},"usedPercent":used,"resetAt":reset,"windowDurationSeconds":if trial{None}else{Some(604800)},"allModels":true,"observedAt":observed.and_then(iso),"status":used.map(status).unwrap_or("unknown")}));
    let cap = u["onDemand"]["limitCents"].as_f64();
    let extra = u["onDemand"]["usedCents"].as_f64();
    if cap.is_some_and(|n| n > 0.) {
        let used = cap.zip(extra).map(|(c, e)| (e / c * 100.).clamp(0., 100.));
        r.quotas.push(json!({"id":format!("{id}/grok:extra"),"vendor":"grok-bot","accountId":id,"legacyWindowId":format!("{id}/grok:extra"),"name":"Extra usage","shortLabel":"Extra","usedPercent":used,"resetAt":reset,"allModels":true,"observedAt":observed.and_then(iso),"status":used.map(status).unwrap_or("unknown")}));
    }
    r.account = Some(
        json!({"id":id,"provider":"grok-bot","identity":"unresolved","plan":u["grokPlanLabel"],"checkedAt":observed.and_then(iso)}),
    );
    r.notice = Some(l10n::text(
        "额度来自 Grok Bot 缓存；在 Bot 中刷新可更新读数",
        "Quota from Grok Bot cache; refresh in Bot to update",
    ));
    if bot::account_through(host, &directory) != Some(slot) {
        return Err(l10n::text(
            "读取缓存期间 Grok Bot 账户发生了变化",
            "Grok Bot account changed while reading its cache",
        ));
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{fake::FakeHost, FileInfo};
    use crate::Os;
    use serde_json::Value;

    const DIR: &str = "/u/AppData/Grok Bot/sand-client-persistence";

    fn entry(key: &str, schema: u64, value: Value) -> (String, String) {
        let file = bot::path(DIR.as_ref(), key);
        let text = format!(
            "{}\n{}",
            serde_json::to_string(key).unwrap(),
            json!({"schemaVersion": schema, "value": value})
        );
        (file.to_string_lossy().into_owned(), text)
    }

    fn host_with(quota: Value) -> FakeHost {
        let (account_file, account) = entry(bot::ACCOUNT_KEY, 1, json!("slot-1"));
        let (quota_file, quota) = entry(&bot::quota_key("slot-1"), 2, quota);
        FakeHost::new("/u")
            .file(&account_file, &account)
            .file(&quota_file, &quota)
    }

    fn fresh_quota(trial: bool) -> Value {
        let now = chrono::Utc::now().timestamp_millis();
        json!({
            "kind": "present",
            "selectedTeamId": null,
            "expiresAtMs": now + 3_600_000,
            "reading": {"readAtMs": now - 60_000, "usage": {
                "hasNonZeroIncludedLimit": true,
                "isSandTrial": trial,
                "percentUsed": 42.0,
                "nextResetMs": now + 86_400_000,
                "grokPlanLabel": "SuperGrok",
                "onDemand": {"limitCents": 1000, "usedCents": 950},
            }},
        })
    }

    #[test]
    fn the_cache_is_read_where_the_descriptor_puts_it() {
        let reading = read(&host_with(fresh_quota(false))).unwrap();
        let id = legacy_unresolved("Grok", &format!("grok-bot:{}", bot::hash(&["slot-1"])));
        assert_eq!(reading.quotas.len(), 2);
        assert_eq!(reading.quotas[0]["id"], format!("{id}/grok"));
        // The Swift app's account for slot-1, as its quota history keys it.
        let open = "account:481bd5386e9b3de64d1ae81e84810325bcac7510e46cea250c975c2fa4fb3623";
        assert_eq!(id, open);
        assert_eq!(reading.quotas[0]["legacyWindowId"], format!("{open}/grok"));
        assert_eq!(
            reading.quotas[1]["legacyWindowId"],
            format!("{open}/grok:extra")
        );
        assert_eq!(reading.quotas[0]["usedPercent"], 42.0);
        assert_eq!(reading.quotas[0]["windowDurationSeconds"], 604800);
        assert_eq!(reading.quotas[1]["usedPercent"], 95.0);
        assert_eq!(reading.quotas[1]["status"], "critical");
        assert_eq!(reading.account.unwrap()["plan"], "SuperGrok");
    }

    #[test]
    fn a_trial_has_no_weekly_window() {
        let reading = read(&host_with(fresh_quota(true))).unwrap();
        assert_eq!(reading.quotas[0]["name"], "Trial usage limit");
        assert!(reading.quotas[0]["windowDurationSeconds"].is_null());
    }

    #[test]
    fn other_systems_say_the_cache_is_not_supported_yet() {
        let mut host = host_with(fresh_quota(false));
        host.os = Os::Windows;
        let reading = read(&host).unwrap();
        assert_eq!(
            reading.notice.as_deref(),
            Some("Grok Bot cache acquisition is not implemented on this platform")
        );
        assert!(reading.quotas.is_empty());
    }

    #[test]
    fn stale_or_linked_caches_are_refused() {
        let mut quota = fresh_quota(false);
        quota["expiresAtMs"] = json!(1);
        assert!(read(&host_with(quota)).is_err());

        let mut host = host_with(fresh_quota(false));
        let file = bot::path(DIR.as_ref(), bot::ACCOUNT_KEY);
        host.info.insert(
            file,
            FileInfo {
                len: 10,
                modified_ms: None,
                symlink: true,
            },
        );
        let reading = read(&host).unwrap();
        assert!(
            reading.quotas.is_empty(),
            "a linked account file is not read"
        );
    }
}
