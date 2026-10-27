//! Quota window names in each vendor's own words, with their short names
//! (`window_names`). The account readers name their windows with these; each
//! follows the agent-hud-native-open function named on it, and the shared parity
//! fixtures check them against it.
use super::window_names::{leading, Period};

/// Codex: a window's name in the words of Codex's own client
/// (`CodexRateLimits.limitName`).
pub fn codex_limit_name(period: Option<Period>, secondary: bool) -> String {
    match period {
        Some(Period::FiveHours) => "5h limit".into(),
        Some(Period::Day) => "Daily limit".into(),
        Some(Period::Week) => "Weekly limit".into(),
        Some(Period::Month) => "Monthly limit".into(),
        Some(Period::Year) => "Annual limit".into(),
        Some(Period::Days(count)) => format!("{count}d limit"),
        Some(Period::Hours(count)) => format!("{count}h limit"),
        Some(Period::Minutes(count)) => format!("{count}m limit"),
        None if secondary => "Secondary usage limit".into(),
        None => "Usage limit".into(),
    }
}

/// Copilot: a window's names in GitHub's and VS Code's English words. Unknown
/// keys read as their words, kept whole up to eight characters in tight
/// places, else the first word (`CopilotClient.names`).
pub fn copilot_names(key: &str, credits: bool) -> (String, String) {
    if credits {
        return ("AI credits".into(), "Credits".into());
    }
    match key {
        "premium_interactions" => ("Premium requests".into(), "Premium".into()),
        "chat" => ("Chat messages".into(), "Chat".into()),
        "completions" => ("Code completions".into(), "Completions".into()),
        _ => {
            let name = capitalized(&key.replace('_', " "));
            let short = leading(&name);
            (name, short)
        }
    }
}

/// Kimi: the 5-hour quota and the weekly quota of Kimi Code's plans, and any
/// other length in the same words; the short name is the period
/// (`OpenAgentQuota.kimiName`).
pub fn kimi_name(period: Option<Period>) -> (String, String) {
    let full = match period {
        Some(Period::FiveHours) => "5-hour quota".into(),
        Some(Period::Week) => "Weekly quota".into(),
        Some(Period::Day) => "Daily quota".into(),
        Some(Period::Month) => "Monthly quota".into(),
        Some(Period::Year) => "Annual quota".into(),
        Some(Period::Days(count)) => format!("{count}-day quota"),
        Some(Period::Hours(count)) => format!("{count}-hour quota"),
        Some(Period::Minutes(count)) => format!("{count}-minute quota"),
        None => "Quota".into(),
    };
    (
        full,
        period.map_or_else(|| "Quota".into(), Period::short_name),
    )
}

/// GLM: Zhipu's names for a plan's limits: credits over 5 hours and a week on
/// plans that meter credits, the 5-hour and weekly limits of older plans, and
/// their monthly MCP usage. A limit of a unit Zhipu does not name keeps its
/// type (`OpenAgentQuota.glmName`).
pub fn glm_name(kind: &str, period: Option<Period>, monthly: bool) -> (String, String) {
    if kind == "TIME_LIMIT" {
        let full = if monthly {
            "MCP usage (1 month)"
        } else {
            "MCP usage"
        };
        return (full.into(), "MCP".into());
    }
    let credits = kind == "CREDIT_LIMIT";
    let Some(period) = period else {
        return (
            kind.into(),
            if credits { "Credits" } else { "Tokens" }.into(),
        );
    };
    let unit = if credits { "credits" } else { "limit" };
    let full = match period {
        Period::FiveHours => format!("5-hour {unit}"),
        Period::Week => format!("Weekly {unit}"),
        Period::Day => format!("Daily {unit}"),
        Period::Month => format!("Monthly {unit}"),
        Period::Year => format!("Annual {unit}"),
        Period::Days(count) => format!("{count}-day {unit}"),
        Period::Hours(count) => format!("{count}-hour {unit}"),
        Period::Minutes(count) => format!("{count}-minute {unit}"),
    };
    (full, period.short_name())
}

/// Antigravity: the word a group of models is known by in tight places: its
/// first word, as Gemini for Gemini Models, and third-party for Claude and
/// GPT models, as Antigravity's plans describe them
/// (`AntigravityClient.groupWord`).
pub fn antigravity_group_word(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("claude") || lower.contains("gpt") {
        return "3rd-party".into();
    }
    name.split(' ')
        .find(|word| !word.is_empty())
        .unwrap_or(name)
        .to_owned()
}

/// Each word with its first letter upper case and the rest lower case, as
/// Foundation's `capitalized` writes them.
fn capitalized(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(letters.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
