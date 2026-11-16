//! AI Coding Plan limits reported by the mod running inside Chengxiaobang.
use crate::account::{account_id, legacy_unresolved, status, Host, Reading};
use crate::l10n;
use serde_json::{json, Value};

fn percentage(value: &Value, limit_key: &str) -> Option<f64> {
    let remaining = value["remainingUsd"].as_f64();
    let used = value["usedUsd"].as_f64();
    let total = value[limit_key]
        .as_f64()
        .or_else(|| Some(used? + remaining?))?;
    let used = used.or_else(|| Some(total - remaining?))?;
    (total > 0.0 && used >= 0.0).then_some(used / total * 100.0)
}

pub fn read(host: &dyn Host) -> Result<Reading, String> {
    let mut reading = Reading::empty("chengxiaobang");
    let Some(report) = host.account_reports("chengxiaobang").into_iter().next() else {
        return Ok(reading);
    };
    let usage = &report["usage"];
    if usage["status"] == "signed_out" {
        reading.notice = Some(l10n::text(
            "程小帮未登录，登录后恢复额度",
            "Sign in to 程小帮 to see its quota again",
        ));
        return Ok(reading);
    }
    if usage["status"] != "available" {
        return Ok(reading);
    }
    let at = report["at"]
        .as_i64()
        .and_then(chrono::DateTime::from_timestamp_millis);
    let identity = legacy_unresolved("Chengxiaobang", "");
    let id = account_id(
        "chengxiaobang",
        &[
            "unresolved",
            &host.home().join(".chengxiaobang").to_string_lossy(),
        ],
    );
    reading.account =
        Some(json!({"id":identity,"provider":"chengxiaobang","plan":usage["planName"]}));
    let five = &usage["fiveHourWindow"];
    for (key, name, short, value, limit_key, reset, seconds) in [
        (
            "chengxiaobang-monthly",
            l10n::text("月度使用限额", "Monthly limit"),
            l10n::text("月度", "Monthly"),
            usage,
            "totalUsd",
            &usage["periodEnd"],
            None,
        ),
        (
            "chengxiaobang-5h",
            l10n::text("5 小时使用限额", "5-hour limit"),
            "5h".into(),
            five,
            "limitUsd",
            &five["windowEndAt"],
            Some(5 * 3600),
        ),
    ] {
        if value["enabled"] == false {
            continue;
        }
        let Some(percent) = percentage(value, limit_key) else {
            continue;
        };
        reading.quotas.push(json!({"id":format!("{id}:{key}"),"vendor":"chengxiaobang","name":name,"shortLabel":short,"usedPercent":percent,"resetAt":reset,"status":status(percent),"accountId":identity,"legacyWindowId":format!("{identity}/{key}"),"consumerSource":"chengxiaobang","windowDurationSeconds":seconds,"allModels":true,"observedAt":at.map(|at|at.to_rfc3339())}));
    }
    if let Some(at) =
        at.filter(|at| chrono::Utc::now().signed_duration_since(*at).num_seconds() > 30 * 60)
    {
        let local = at.with_timezone(&chrono::Local);
        let time = if local.date_naive() == chrono::Local::now().date_naive() {
            local.format("%H:%M")
        } else {
            local.format("%m-%d %H:%M")
        };
        reading.notice = Some(l10n::text(
            &format!("这是程小帮在 {time} 报告的额度"),
            &format!("As 程小帮 reported at {time}"),
        ));
    }
    Ok(reading)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::fake::FakeHost;
    use serde_json::{json, Value};
    fn report(usage: Value, at: i64) -> Reading {
        let mut host = FakeHost::new("/u");
        host.account_reports
            .insert("chengxiaobang".into(), vec![json!({"at":at,"usage":usage})]);
        read(&host).unwrap()
    }
    fn usage() -> Value {
        json!({"status":"available","planName":"AI Coding Plan","usedUsd":25,"totalUsd":100,"periodEnd":"2026-12-01T00:00:00Z","fiveHourWindow":{"usedUsd":2,"limitUsd":10,"windowEndAt":"2026-11-16T15:00:00Z"}})
    }
    #[test]
    fn two_windows() {
        let r = report(usage(), chrono::Utc::now().timestamp_millis());
        assert_eq!(r.account.unwrap()["plan"], "AI Coding Plan");
        assert_eq!(r.quotas.len(), 2);
        assert_eq!(r.quotas[0]["name"], "Monthly limit");
        assert_eq!(r.quotas[1]["name"], "5-hour limit");
        assert_eq!(r.quotas[0]["usedPercent"], 25.0);
        assert_eq!(r.quotas[0]["resetAt"], "2026-12-01T00:00:00Z");
        assert_eq!(r.quotas[1]["usedPercent"], 20.0);
        assert_eq!(r.quotas[1]["resetAt"], "2026-11-16T15:00:00Z");
        assert_eq!(r.quotas[1]["windowDurationSeconds"], 18000);
        assert!(r.notice.is_none());
    }
    #[test]
    fn optional_five_hour_window() {
        let mut u = usage();
        u.as_object_mut().unwrap().remove("fiveHourWindow");
        assert_eq!(report(u, 0).quotas.len(), 1);
        let mut u = usage();
        u["fiveHourWindow"]["enabled"] = json!(false);
        assert_eq!(report(u, 0).quotas.len(), 1);
    }
    #[test]
    fn derives_missing_total_or_used_from_remaining() {
        let r = report(
            json!({"status":"available","usedUsd":20,"remainingUsd":80,"fiveHourWindow":{"remainingUsd":8,"limitUsd":10}}),
            0,
        );
        assert_eq!(r.quotas[0]["usedPercent"], 20.0);
        assert_eq!(r.quotas[1]["usedPercent"], 20.0);
        assert!(report(json!({"status":"available","remainingUsd":8}), 0)
            .quotas
            .is_empty());
    }
    #[test]
    fn signed_out_and_stale_and_missing() {
        let r = report(json!({"status":"signed_out"}), 0);
        assert!(r.quotas.is_empty());
        assert!(r.notice.unwrap().contains("Sign in"));
        assert!(report(usage(), 0)
            .notice
            .unwrap()
            .contains("As 程小帮 reported at"));
        assert!(read(&FakeHost::new("/u")).unwrap().quotas.is_empty());
    }
}
