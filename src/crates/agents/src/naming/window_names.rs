//! Quota window names. A window's full name is in its vendor's own words; its
//! short name, for tight places, keeps only what tells it apart from the
//! account's other windows. Names stay in English in every language. Follows
//! agent-hud-native-open `WindowNames`.
use super::characters;

/// A window's length as the vendors' clients name it: 5 hours, a day, a week,
/// 30 days and 365 days within 5 %, else whole days, hours or minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    FiveHours,
    Day,
    Week,
    Month,
    Year,
    Days(i64),
    Hours(i64),
    Minutes(i64),
}

impl Period {
    /// None for a window without a length, or one shorter than a minute.
    pub fn from_seconds(seconds: Option<f64>) -> Option<Self> {
        let seconds = seconds.filter(|s| s.is_finite() && *s >= 60.0)?;
        let minutes = (seconds / 60.0).round();
        let near = |expected: f64| minutes >= expected * 0.95 && minutes <= expected * 1.05;
        Some(if near(300.0) {
            Self::FiveHours
        } else if near(1440.0) {
            Self::Day
        } else if near(10080.0) {
            Self::Week
        } else if near(43200.0) {
            Self::Month
        } else if near(525_600.0) {
            Self::Year
        } else if minutes % 1440.0 == 0.0 {
            Self::Days((minutes / 1440.0) as i64)
        } else if minutes % 60.0 == 0.0 {
            Self::Hours((minutes / 60.0) as i64)
        } else {
            Self::Minutes(minutes as i64)
        })
    }

    /// The short name of a window that is only its period: 5h, Daily, Weekly,
    /// Monthly and Annual, and any other length as its unit (3h, 30m, 3d).
    pub fn short_name(self) -> String {
        match self {
            Self::FiveHours => "5h".into(),
            Self::Day => "Daily".into(),
            Self::Week => "Weekly".into(),
            Self::Month => "Monthly".into(),
            Self::Year => "Annual".into(),
            Self::Days(count) => format!("{count}d"),
            Self::Hours(count) => format!("{count}h"),
            Self::Minutes(count) => format!("{count}m"),
        }
    }

    /// The period after a word that names a bucket or group: the week is 7d,
    /// since a word and Weekly would no longer fit.
    pub fn after_word(self) -> String {
        if self == Self::Week {
            "7d".into()
        } else {
            self.short_name()
        }
    }
}

/// Quota window names as a vendor's service reports them, the name shown and
/// the word a short name keeps of it (agent-hud-native-open `VendorCatalog`). A value
/// no entry names is shown as the vendor wrote it.
const CATALOG: [(&str, &str, &str, &str); 1] =
    [("codex", "gpt-reserve", "Luna Reserve", "Reserve")];

/// The name shown for a window the vendor's service reported as `value`.
pub fn catalog_window(vendor: &str, value: &str) -> String {
    CATALOG
        .iter()
        .find(|(v, reported, _, _)| *v == vendor && *reported == value)
        .map_or_else(|| value.to_owned(), |(_, _, name, _)| (*name).to_owned())
}

/// The word a short name keeps of that window: the catalog's, else the
/// distinctive word of the name shown (`word`).
pub fn catalog_window_word(vendor: &str, value: &str) -> String {
    CATALOG
        .iter()
        .find(|(v, reported, _, _)| *v == vendor && *reported == value)
        .map_or_else(
            || word(&catalog_window(vendor, value)),
            |(_, _, _, word)| (*word).to_owned(),
        )
}

/// Every quota row's short name, as the panel, the docks and the peek show it.
/// Rows of one account whose short names would read alike, and rows without
/// one, show their full names (agent-hud-native-open `ProviderData.scopedWindows`,
/// `AgentDescriptor.shortName`).
pub fn settle_short_names(rows: &mut [serde_json::Value]) {
    let account = |row: &serde_json::Value| {
        (
            row["vendor"].as_str().unwrap_or("").to_owned(),
            row["accountId"].as_str().unwrap_or("").to_owned(),
        )
    };
    let mut groups: std::collections::HashMap<(String, String), Vec<usize>> = Default::default();
    for (index, row) in rows.iter().enumerate() {
        groups.entry(account(row)).or_default().push(index);
    }
    for indices in groups.into_values() {
        let names = indices
            .iter()
            .map(|&index| {
                rows[index]["shortLabel"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        for (&index, short) in indices.iter().zip(distinct(&names)) {
            let full = rows[index]["name"].clone();
            rows[index]["shortLabel"] = short.map_or(full, serde_json::Value::from);
        }
    }
}

/// The word a long service name is known by where no catalog names it: without
/// version numbers and the words GPT, Codex and Claude, the last word left
/// (GPT-5.3-Codex-Spark → Spark); the name itself when nothing is left.
pub fn word(name: &str) -> String {
    name.split([' ', '-', '_'])
        .filter(|word| !word.is_empty())
        .filter(|word| !matches!(word.to_lowercase().as_str(), "gpt" | "codex" | "claude"))
        .rfind(|word| !is_version(word))
        .unwrap_or(name)
        .to_owned()
}

/// `^[vV]?[0-9]+(\.[0-9]+)*$`
fn is_version(word: &str) -> bool {
    let digits = word.strip_prefix(['v', 'V']).unwrap_or(word);
    !digits.is_empty()
        && digits
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// The short names of one account's windows, with None for any two that would
/// read alike, which then show their full names.
pub fn distinct(names: &[Option<String>]) -> Vec<Option<String>> {
    names
        .iter()
        .map(|name| {
            name.clone().filter(|name| {
                names
                    .iter()
                    .filter(|other| other.as_deref() == Some(name))
                    .count()
                    == 1
            })
        })
        .collect()
}

/// A name kept whole with at most eight characters, else its first word.
pub fn leading(name: &str) -> String {
    if characters(name) <= 8 {
        return name.to_owned();
    }
    name.split(' ')
        .find(|word| !word.is_empty())
        .unwrap_or(name)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn short_names_settle_per_account_and_fall_back_to_the_full_name() {
        let mut rows = vec![
            json!({"vendor":"glm","accountId":"a","name":"5-hour limit","shortLabel":"5h"}),
            json!({"vendor":"glm","accountId":"a","name":"5-hour credits","shortLabel":"5h"}),
            json!({"vendor":"glm","accountId":"a","name":"Weekly limit","shortLabel":"Weekly"}),
            // Another account may use the same short name.
            json!({"vendor":"glm","accountId":"b","name":"5-hour limit","shortLabel":"5h"}),
            // A reader that names no short form shows the full name.
            json!({"vendor":"hermes","accountId":"c","name":"Daily budget"}),
        ];
        settle_short_names(&mut rows);
        let shorts: Vec<_> = rows
            .iter()
            .map(|row| row["shortLabel"].as_str().unwrap())
            .collect();
        assert_eq!(
            shorts,
            [
                "5-hour limit",
                "5-hour credits",
                "Weekly",
                "5h",
                "Daily budget"
            ]
        );
    }
}
