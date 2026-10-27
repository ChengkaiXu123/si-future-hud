//! How a client's transcript names its turns, line by line: the part of the
//! transcript behaviour (docs/agent-contract.md) that pages of messages need.
//! Each client states its own rule on its `Agent`; the engine keeps where a
//! page stopped and looks back for a turn's start.
use serde_json::Value;

/// A line that names a turn.
pub struct TurnStart {
    pub turn: Option<String>,
    /// When the turn began, where this line says so.
    pub started: Option<Value>,
    /// Whether this line starts the turn. Codex names its turn on most lines
    /// and says once when it began; every other client names it only on the
    /// line that starts it.
    pub whole: bool,
}

impl TurnStart {
    pub fn whole(turn: Option<String>, started: Option<&Value>) -> Self {
        Self {
            turn,
            started: started.cloned(),
            whole: true,
        }
    }
}

pub struct Turns {
    /// The turn this line names, if it names one.
    pub start: fn(&Value) -> Option<TurnStart>,
    /// Whether a raw line may name a turn, so that looking back decodes only
    /// those lines. It may say yes to lines `start` then refuses.
    pub may_name: fn(&[u8]) -> bool,
}

/// The rule for clients that state none of their own yet: every client's
/// rule, first match wins, as all transcripts were read before each client
/// stated its own.
pub static SHARED: Turns = Turns {
    start: shared_start,
    may_name: shared_may_name,
};

const STATED: [&Turns; 5] = [
    &crate::codex::transcript::TURNS,
    &crate::claude::transcript::TURNS,
    &crate::deepseek::transcript::TURNS,
    &crate::copilot::transcript::TURNS,
    &crate::kimi::transcript::TURNS,
];

fn shared_start(value: &Value) -> Option<TurnStart> {
    STATED.iter().find_map(|turns| (turns.start)(value))
}

fn shared_may_name(line: &[u8]) -> bool {
    STATED.iter().any(|turns| (turns.may_name)(line))
}

/// Whether `line` holds `word`, without decoding it.
pub fn has(line: &[u8], word: &[u8]) -> bool {
    line.windows(word.len()).any(|w| w == word)
}

/// The rule of the client with this id, or the shared rule.
pub fn of(source: &str) -> &'static Turns {
    crate::by_id(source)
        .and_then(|agent| agent.turns)
        .unwrap_or(&SHARED)
}
