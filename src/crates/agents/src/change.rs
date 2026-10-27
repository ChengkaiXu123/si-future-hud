//! What changed, as the engine's parts and the agents report it. A pass does
//! only the work its changes call for (docs/agent-contract.md).
use crate::HookEvent;
use std::path::PathBuf;

/// One change for the collector to act on.
pub enum Change {
    LogGrew {
        path: PathBuf,
    },
    LogGone {
        path: PathBuf,
    },
    AccountRead {
        agent: &'static str,
        observed_at: i64,
    },
    Hook {
        agent: &'static str,
        event: HookEvent,
    },
    RequestOpened {
        id: String,
    },
    RequestClosed {
        id: String,
    },
    Setting {
        key: String,
    },
    /// A time-based change falls due: one timer waits for the earliest.
    Due {
        at: i64,
        why: Why,
    },
}

/// Why something falls due at a time rather than on an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// A session goes quiet (2 minutes) or is abandoned (30 minutes).
    SessionState,
    RequestExpires,
    HeartbeatLapses,
    /// A session leaves the 1-day or 7-day list.
    LeavesList,
    QuotaResets,
    /// A forecast instant, such as when a quota runs out, is reached.
    Forecast,
    AccountReading,
}
