//! Cached account facts supplied by the platform account reader. The domain
//! engine never performs network or engine I/O while projecting local sessions.
use super::*;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Default, PartialEq)]
pub struct AccountProjection {
    pub quotas: Vec<Value>,
    pub billing: Vec<Value>,
    pub accounts: Vec<Value>,
    pub providers: HashMap<String, Value>,
    pub session_records: Vec<Value>,
    /// Which clients reach which vendors' plans and APIs, and on which
    /// platform (`{client, provider, product, accountId, region}`).
    pub services: Vec<Value>,
}

impl crate::AgentHudCore {
    pub fn set_account_projection(&mut self, accounts: AccountProjection) {
        if let Some(database) = &self.database {
            let mut changed = false;
            for row in &accounts.quotas {
                let Some(id) = row["id"].as_str() else {
                    continue;
                };
                let Some(observed) = row["observedAt"].as_str() else {
                    continue;
                };
                if self
                    .account_samples
                    .get(id)
                    .is_some_and(|previous| previous == observed)
                {
                    continue;
                }
                if row["status"] == "stale" {
                    continue;
                }
                let Some(at) = metering::date_ms(observed) else {
                    continue;
                };
                let Some(used) = row["usedPercent"].as_f64().filter(|v| v.is_finite()) else {
                    continue;
                };
                match metering::insert_quota_sample(
                    database,
                    id,
                    at,
                    (100.0 - used).clamp(0.0, 100.0),
                ) {
                    Ok(count) => {
                        changed |= count > 0;
                        self.account_samples.insert(id.into(), observed.into());
                    }
                    Err(_) => {
                        self.storage_issue = Some(l10n::text(
                            "额度读数暂时无法保存",
                            "Quota sample persistence is unavailable",
                        ))
                    }
                }
            }
            if let Some(path) = &self.legacy_quota_database {
                let pairs = accounts
                    .quotas
                    .iter()
                    .filter_map(|row| {
                        // The Swift app kept Grok Bot's quota under its `grok` provider.
                        let scope = match row["vendor"].as_str()? {
                            "grok-bot" => "grok",
                            vendor => vendor,
                        };
                        Some((
                            row["id"].as_str()?.into(),
                            scope.into(),
                            row["legacyWindowId"].as_str()?.into(),
                        ))
                    })
                    .collect::<Vec<_>>();
                match legacy_quota::import(database, path, &pairs) {
                    Ok(count) => changed |= count > 0,
                    Err(_) => {
                        self.storage_issue = Some(l10n::text(
                            "无法读取之前的额度记录",
                            "Previous quota history could not be read",
                        ))
                    }
                }
            }
            if changed {
                let _ = metering::prune_quota_samples(
                    database,
                    Utc::now().timestamp_millis() - 30 * 86_400_000,
                );
                self.meters.samples = None;
            }
        }
        if !accounts.session_records.is_empty()
            && self.accounts.session_records != accounts.session_records
        {
            let _ = self.ingest_account_sessions(&accounts.session_records);
        }
        let regions = pricing::PriceRegions::new(&accounts.services, &accounts.billing);
        if regions != self.price_regions {
            // Costs already shown were priced on the old platforms.
            self.price_regions = regions;
            self.meters.view = None;
            self.statistics.revision = self.statistics.revision.wrapping_add(1);
        }
        if self.accounts != accounts {
            self.render.accounts = self.render.accounts.wrapping_add(1);
            self.accounts = accounts;
        }
    }
}
