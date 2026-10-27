//! Which platform a client's calls to a vendor went through, as far as this
//! computer can tell: the endpoint or plan the client is configured with, and
//! for DeepSeek, which bills one platform in either currency, the currency of
//! its balance. A client whose endpoints disagree, or one nothing names, is
//! priced on the international list (agent-hud-native-open `PriceRegions`).
use super::{model_for, Region};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PriceRegions {
    /// By the client's source id and the catalog's vendor.
    by_client: BTreeMap<String, BTreeMap<String, Region>>,
    by_vendor: BTreeMap<String, Region>,
}

impl PriceRegions {
    /// `services`: the account readers' `{client, provider, region}` rows;
    /// `billing`: their balances, whose DeepSeek currencies name its platform.
    pub fn new(services: &[Value], billing: &[Value]) -> Self {
        let mut clients = BTreeMap::<String, BTreeMap<String, BTreeSet<Region>>>::new();
        let mut vendors = BTreeMap::<String, BTreeSet<Region>>::new();
        for service in services {
            let Some(region) = region(&service["region"]) else {
                continue;
            };
            let Some(provider) = service["provider"].as_str() else {
                continue;
            };
            let vendor = vendor(provider);
            if let Some(client) = service["client"].as_str() {
                clients
                    .entry(client.to_lowercase())
                    .or_default()
                    .entry(vendor.clone())
                    .or_default()
                    .insert(region);
            }
            vendors.entry(vendor).or_default().insert(region);
        }
        for balance in billing.iter().filter(|row| row["vendor"] == "deepseek") {
            let currency = balance["currency"].as_str();
            if let Some(region) = [Region::International, Region::China]
                .into_iter()
                .find(|region| Some(region.currency()) == currency)
            {
                vendors.entry("DeepSeek".into()).or_default().insert(region);
            }
        }
        let single =
            |regions: BTreeSet<Region>| (regions.len() == 1).then(|| *regions.first().unwrap());
        Self {
            by_client: clients
                .into_iter()
                .map(|(client, vendors)| {
                    let vendors = vendors
                        .into_iter()
                        .filter_map(|(vendor, regions)| {
                            single(regions).map(|region| (vendor, region))
                        })
                        .collect();
                    (client, vendors)
                })
                .collect(),
            by_vendor: vendors
                .into_iter()
                .filter_map(|(vendor, regions)| single(regions).map(|region| (vendor, region)))
                .collect(),
        }
    }

    /// The list `model`, used through the client `source`, is priced on.
    pub fn region(&self, source: &str, model: &str) -> Region {
        let Some(listed) = model_for(&format!("{source}-model:{model}")) else {
            return Region::International;
        };
        self.by_client
            .get(&source.to_lowercase())
            .and_then(|vendors| vendors.get(&listed.vendor))
            .or_else(|| self.by_vendor.get(&listed.vendor))
            .copied()
            .unwrap_or_default()
    }
}

fn region(value: &Value) -> Option<Region> {
    match value.as_str()? {
        "china" => Some(Region::China),
        "international" => Some(Region::International),
        _ => None,
    }
}

/// Plans name their vendor by product; the catalog by the company that
/// prices the model.
fn vendor(provider: &str) -> String {
    match provider {
        "Kimi" | "kimi" => "Moonshot".into(),
        "glm" => "GLM".into(),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_client_on_one_platform_is_priced_there_and_disagreement_is_international() {
        let services = [
            json!({"client":"Claude","provider":"GLM","product":"plan","region":"china"}),
            json!({"client":"OpenCode","provider":"GLM","product":"plan","region":"china"}),
            json!({"client":"OpenCode","provider":"GLM","product":"api","region":"international"}),
        ];
        let regions = PriceRegions::new(&services, &[]);
        assert_eq!(regions.region("claude", "glm-5"), Region::China);
        assert_eq!(
            regions.region("opencode", "glm-5"),
            Region::International,
            "the client's endpoints disagree"
        );
        assert_eq!(
            regions.region("pi", "glm-5"),
            Region::International,
            "the vendor's platforms disagree too"
        );
        assert_eq!(
            regions.region("claude", "claude-sonnet-4-5"),
            Region::International
        );
    }

    #[test]
    fn a_regional_model_has_no_price_on_a_platform_that_does_not_sell_it() {
        let tokens = super::super::PriceTokens {
            tokens_in: 1000,
            ..Default::default()
        };
        let china = [json!({"client":"Claude","provider":"GLM","product":"plan","region":"china"})];
        let on_china = PriceRegions::new(&china, &[]).region("claude", "glm-5-turbo");
        assert_eq!(on_china, Region::China);
        let priced =
            super::super::cost_at("claude", "glm-5-turbo", &tokens, false, on_china, None).unwrap();
        assert_eq!(priced.currency, "CNY");
        let elsewhere = PriceRegions::default().region("claude", "glm-5-turbo");
        assert!(
            super::super::cost_at("claude", "glm-5-turbo", &tokens, false, elsewhere, None)
                .is_none()
        );
    }

    #[test]
    fn a_vendor_on_one_platform_prices_every_client_and_deepseek_follows_its_currency() {
        let services = [json!({"client":"Pi","provider":"Kimi","product":"plan","region":"china"})];
        let billing = [json!({"vendor":"deepseek","currency":"CNY"})];
        let regions = PriceRegions::new(&services, &billing);
        assert_eq!(regions.region("opencode", "kimi-k3"), Region::China);
        assert_eq!(regions.region("deepseek", "deepseek-v4-pro"), Region::China);
        let both = [
            json!({"vendor":"deepseek","currency":"CNY"}),
            json!({"vendor":"deepseek","currency":"USD"}),
        ];
        assert_eq!(
            PriceRegions::new(&[], &both).region("deepseek", "deepseek-v4-pro"),
            Region::International
        );
    }
}
