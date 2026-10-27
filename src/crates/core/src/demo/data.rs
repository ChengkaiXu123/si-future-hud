//! The demo's sample, ported from agent-hud-native-open's `DemoData`: one Claude
//! account and one Codex account with their quota windows, every other
//! vendor's window, a DeepSeek balance, nine sessions and a month of tokens.
//! The sessions' turns come from the same seeded generator, so the figures
//! match Open's demo.

/// A quota window as last read: the share used, the seconds to its reset and
/// its length.
pub(crate) struct Reading {
    pub key: &'static str,
    pub used: f64,
    pub reset_in: i64,
    pub duration: i64,
}

pub(crate) const READINGS: [Reading; 6] = [
    Reading {
        key: "claude-session",
        used: 12.0,
        reset_in: 4320,
        duration: 5 * 3600,
    },
    Reading {
        key: "claude-weekly",
        used: 79.0,
        reset_in: 5 * 86400,
        duration: 7 * 86400,
    },
    Reading {
        key: "claude-weekly-fable",
        used: 60.0,
        reset_in: 5 * 86400,
        duration: 7 * 86400,
    },
    Reading {
        key: "codex",
        used: 49.0,
        reset_in: 6 * 86400,
        duration: 7 * 86400,
    },
    Reading {
        key: "codex:spark:primary",
        used: 18.0,
        reset_in: 7200,
        duration: 5 * 3600,
    },
    Reading {
        key: "codex:spark:secondary",
        used: 24.0,
        reset_in: 6 * 86400,
        duration: 7 * 86400,
    },
];

/// The other vendors the app shows: the vendor's key, its window's name, the
/// share remaining and the seconds to its reset.
pub(crate) const EXTRAS: [(&str, &str, f64, Option<i64>); 14] = [
    ("antigravity", "Agent", 91.0, Some(6 * 3600 + 40 * 60)),
    ("cursor", "Agent", 83.0, Some(3 * 3600 + 12 * 60)),
    ("copilot", "Agent", 12.0, Some(5 * 3600)),
    ("grok-cli", "Code", 66.0, Some(90 * 60)),
    ("kimi", "K3", 38.0, Some(2 * 3600)),
    ("glm", "Coding", 95.0, Some(8 * 3600)),
    ("opencode", "CLI", 51.0, None),
    ("openclaw", "Agent", 4.0, Some(33 * 60)),
    ("hermes", "Agent", 77.0, Some(4 * 3600 + 25 * 60)),
    ("zcode", "CLI", 29.0, Some(70 * 60)),
    ("codebuddy", "Agent", 88.0, Some(6 * 3600)),
    ("workbuddy", "Agent", 19.0, Some(45 * 60)),
    ("qwen", "Code", 47.0, Some(2 * 3600 + 30 * 60)),
    ("pi", "Agent", 60.0, Some(3 * 3600)),
];

/// A session of the sample: its client's source, model, what it works on and
/// where, its turns and their seed, and the minutes since it last did
/// anything.
pub(crate) struct Plan {
    pub id: &'static str,
    pub source: &'static str,
    pub model: &'static str,
    pub title: &'static str,
    pub project: &'static str,
    pub turns: usize,
    pub seed: u64,
    pub idle_minutes: f64,
    /// In a turn now, where no script says which sessions run.
    pub running: bool,
    /// The turn that came back after its cache had lapsed.
    pub lapse: Option<usize>,
    /// The days the session began before its turns.
    pub days: f64,
}

pub(crate) const PLANS: [Plan; 9] = [
    Plan {
        id: "s1",
        source: "claude",
        model: "claude-fable-5-1",
        title: "Build the iOS dashboard",
        project: "~/work/mobile-app",
        turns: 26,
        seed: 7,
        idle_minutes: 1.0,
        running: true,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s2",
        source: "codex",
        model: "gpt-6-astra",
        title: "Review App Store sandbox support",
        project: "~/work/orbit",
        turns: 30,
        seed: 51,
        idle_minutes: 2.0,
        running: true,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s3",
        source: "claude",
        model: "claude-opus-5-5",
        title: "Refine settings and colors",
        project: "~/work/orbit",
        turns: 12,
        seed: 33,
        idle_minutes: 38.0,
        running: false,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s4",
        source: "codex",
        model: "gpt-6.1-sol",
        title: "Run the regression suite",
        project: "~/work/ledger-api",
        turns: 9,
        seed: 12,
        idle_minutes: 3.0,
        running: false,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s5",
        source: "claude",
        model: "claude-sonnet-5-5",
        title: "Update the documentation",
        project: "~/work/handbook",
        turns: 17,
        seed: 19,
        idle_minutes: 62.0,
        running: false,
        lapse: Some(11),
        days: 9.0,
    },
    Plan {
        id: "s6",
        source: "claude",
        model: "claude-opus-5-5",
        title: "Package the Mac release",
        project: "~/work/orbit",
        turns: 14,
        seed: 61,
        idle_minutes: 125.0,
        running: false,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s7",
        source: "codex",
        model: "gpt-6.1-sol",
        title: "Check the release scripts",
        project: "~/work/orbit",
        turns: 5,
        seed: 23,
        idle_minutes: 4.0,
        running: false,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s8",
        source: "deepseek",
        model: "deepseek-v4-pro",
        title: "Review the API integration",
        project: "~/work/website",
        turns: 6,
        seed: 41,
        idle_minutes: 5.0,
        running: false,
        lapse: None,
        days: 0.0,
    },
    Plan {
        id: "s9",
        source: "codex",
        model: "gpt-6.1-sol",
        title: "Draft the release notes",
        project: "~/work/orbit",
        turns: 8,
        seed: 71,
        idle_minutes: 26.0 * 60.0,
        running: false,
        lapse: None,
        days: 0.0,
    },
];

/// The model a Claude session hands part of every fourth turn to.
pub(crate) const HELPER: &str = "claude-haiku-4-5";

/// Tokens by kind, as Open's `TokenKinds`: input and output exclude cache
/// writes and reasoning.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Kinds {
    pub cache_write: u64,
    pub input: u64,
    pub reasoning: u64,
    pub output: u64,
    pub cache_read: u64,
}

impl Kinds {
    fn part(self) -> Self {
        Self {
            cache_write: self.cache_write * 2 / 5,
            input: self.input * 2 / 5,
            reasoning: self.reasoning * 2 / 5,
            output: self.output * 2 / 5,
            cache_read: self.cache_read * 2 / 5,
        }
    }

    fn minus(self, other: Self) -> Self {
        Self {
            cache_write: self.cache_write - other.cache_write,
            input: self.input - other.input,
            reasoning: self.reasoning - other.reasoning,
            output: self.output - other.output,
            cache_read: self.cache_read - other.cache_read,
        }
    }

    /// One of `calls` equal shares; the last share takes the remainder.
    pub fn share(self, index: usize, calls: usize) -> Self {
        let calls = calls.max(1) as u64;
        let last = index as u64 + 1 == calls;
        let cut = |total: u64| {
            if last {
                total - total / calls * (calls - 1)
            } else {
                total / calls
            }
        };
        Self {
            cache_write: cut(self.cache_write),
            input: cut(self.input),
            reasoning: cut(self.reasoning),
            output: cut(self.output),
            cache_read: cut(self.cache_read),
        }
    }
}

/// One turn of a session: when it ran, its calls, what its own model spent
/// and what a sub-agent spent on the helper model.
pub(crate) struct DemoTurn {
    pub start: i64,
    pub end: i64,
    pub calls: usize,
    pub own: Kinds,
    pub helper: Option<Kinds>,
}

/// The session's span: when its last turn ended and when its first began.
pub(crate) fn span(plan: &Plan, now: i64) -> (i64, i64) {
    let last = now - (plan.idle_minutes * 60_000.0) as i64;
    let start = last - plan.turns as i64 * 150_000 - (plan.days * 86_400_000.0) as i64;
    (start, last)
}

/// The session's turns, from Open's generator with the plan's seed: a context
/// that grows by what each turn writes to the cache, reasoning on about half
/// of them, now and then a large paste, a compaction as the context nears its
/// window, a sub-agent on Haiku every fourth Claude turn, and at `lapse` a
/// turn that writes the whole context again.
pub(crate) fn turns(plan: &Plan, now: i64) -> Vec<DemoTurn> {
    let mut state = plan.seed * 9973 + 1;
    let mut random = || {
        state = state * 16807 % 2_147_483_647;
        state as f64 / 2_147_483_647.0
    };
    let claude = plan.source == "claude";
    let window: Option<u64> = match plan.source {
        "claude" => Some(1_000_000),
        "codex" => Some(258_400),
        _ => None,
    };
    let (start, end) = span(plan, now);
    let span = (end - start) as f64;
    let mut context: u64 = if claude { 140_000 } else { 24_000 };
    let mut turns = Vec::with_capacity(plan.turns);
    for index in 0..plan.turns {
        let calls = 3 + (random() * 12.0) as usize;
        let input = if random() < 0.18 {
            (7_000.0 + random() * 16_000.0) as u64
        } else {
            (150.0 + random() * 1_800.0) as u64
        };
        let output = (250.0 + random() * random() * 4_200.0) as u64;
        let reasoning = if random() < 0.55 {
            (400.0 + random() * 5_000.0) as u64
        } else {
            0
        };
        let mut write = (4_000.0 + random() * if claude { 26_000.0 } else { 9_000.0 }) as u64;
        let mut compacted = false;
        if let Some(window) = window {
            if context + write + input > window * 82 / 100 {
                compacted = true;
                context = 22_000;
                write = 22_000;
            }
        }
        let lapsed = plan.lapse == Some(index) && !compacted;
        let rewritten = if lapsed { context } else { 0 };
        context += write + input;
        let kinds = Kinds {
            cache_write: write + rewritten,
            input,
            reasoning,
            output,
            cache_read: context * calls as u64 * 3 / 5,
        };
        let turn_start =
            start + (span * (index as f64 + random() * 0.4) / plan.turns as f64) as i64;
        let helping = claude && index % 4 == 1 && !lapsed;
        let turn_end = end.min(turn_start + ((20.0 + random() * 150.0) * 1000.0) as i64);
        let (own, helper) = if helping {
            let part = kinds.part();
            (kinds.minus(part), Some(part))
        } else {
            (kinds, None)
        };
        turns.push(DemoTurn {
            start: turn_start,
            end: turn_end,
            calls,
            own,
            helper,
        });
    }
    turns
}

/// A month of tokens in quarter hours, 15 million a day before cache reads,
/// with a model mix that changes through the day: (start, source, model,
/// input, output, cache read).
pub(crate) fn history(now: i64) -> Vec<(i64, &'static str, &'static str, u64, u64, u64)> {
    const QUARTER: i64 = 900_000;
    let end = now.div_euclid(QUARTER) * QUARTER;
    let heights: [u64; 24] = [
        64, 16, 34, 4, 24, 48, 6, 6, 22, 24, 12, 20, 44, 62, 38, 4, 72, 16, 6, 24, 8, 44, 62, 72,
    ];
    let spenders: [(&str, &str); 6] = [
        ("claude", "claude-fable-5-1"),
        ("claude", "claude-opus-5-5"),
        ("deepseek", "deepseek-v4-flash"),
        ("codex", "codex-auto-review"),
        ("codex", "gpt-6.1-sol"),
        ("claude", "claude-sonnet-5-5"),
    ];
    let mixes: [[u64; 6]; 24] = [
        [5, 3, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [2, 0, 1, 0, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [3, 2, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [2, 3, 0, 1, 0, 0],
        [1, 4, 0, 3, 0, 0],
        [0, 3, 0, 2, 0, 0],
        [0, 1, 0, 0, 0, 0],
        [0, 2, 0, 1, 0, 0],
        [0, 1, 0, 0, 0, 0],
        [0, 0, 0, 0, 1, 0],
        [0, 0, 0, 0, 3, 1],
        [0, 0, 0, 0, 0, 1],
        [1, 3, 0, 1, 0, 0],
        [0, 3, 0, 1, 0, 0],
        [0, 2, 0, 1, 0, 0],
    ];
    let multiplier = 15_000_000.0 / (heights.iter().sum::<u64>() * 4) as f64;
    let count = 30 * 96;
    let mut buckets = Vec::new();
    for index in 0..count {
        let start = end - (count - index) as i64 * QUARTER;
        let hour = index / 4 % 24;
        let mix = mixes[hour];
        let weight: u64 = mix.iter().sum();
        for (position, (source, model)) in spenders.iter().enumerate() {
            let total =
                (heights[hour] as f64 * multiplier * mix[position] as f64 / weight as f64) as u64;
            if total == 0 {
                continue;
            }
            let input = total * 4 / 5;
            buckets.push((
                start,
                *source,
                *model,
                input,
                total - input,
                total * (position as u64 + 1),
            ));
        }
    }
    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generator_is_seeded_and_shaped_as_opens() {
        let now = 1_800_000_000_000;
        let first = turns(&PLANS[0], now);
        assert_eq!(first.len(), 26);
        assert_eq!(
            turns(&PLANS[0], now)[3].own,
            first[3].own,
            "the same seed gives the same turns"
        );
        // A Claude session hands part of every fourth turn to the helper.
        assert!(first[1].helper.is_some() && first[0].helper.is_none());
        let (start, end) = span(&PLANS[0], now);
        assert!(first
            .iter()
            .all(|turn| turn.start >= start && turn.end <= end));
        // Fifteen million tokens a day before cache reads, over thirty days.
        let total: u64 = history(now).iter().map(|row| row.3 + row.4).sum();
        assert!(
            (total as f64 / 30.0 - 15_000_000.0).abs() < 200_000.0,
            "{total}"
        );
    }
}
