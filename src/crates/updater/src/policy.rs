//! When the updater checks, what counts as newer, and when it may restart
//! the app.
use crate::text::{text, Text};
use serde_json::Value;
use std::time::{Duration, SystemTime};

/// The first automatic check, this long after launch.
pub const FIRST_CHECK: Duration = Duration::from_secs(60);
/// Later automatic checks, this long after the one before.
pub const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// While a downloaded update waits for a quiet moment, how often the updater
/// looks again even when no new snapshot arrives.
pub const IDLE_RECHECK: Duration = Duration::from_secs(60);
/// The longest the worker waits at a time, so a check that fell due while
/// the Mac slept runs soon after it wakes.
pub const LONGEST_WAIT: Duration = Duration::from_secs(10 * 60);

/// How long until `due` by the wall clock, or None once it is due.
pub fn until(now: SystemTime, due: SystemTime) -> Option<Duration> {
    due.duration_since(now).ok().filter(|left| !left.is_zero())
}

/// Whether `candidate` is a newer release than `current` (semantic
/// versioning: 0.1.10 is newer than 0.1.9, and 0.2.0-beta.1 older than
/// 0.2.0).
pub fn newer(candidate: &str, current: &str) -> Result<bool, Text> {
    let parse = |version: &str| {
        semver::Version::parse(version).map_err(|_| {
            text(
                format!("版本号「{version}」无法比较"),
                format!("The version “{version}” cannot be compared"),
            )
        })
    };
    Ok(parse(candidate)? > parse(current)?)
}

/// Whether the app can restart without losing anything the user is asked
/// about: no card waits on the notch. Every request that waits on the user
/// is a card in the snapshot's `approvals` (Core's one owner of pending
/// requests): permission, question and plan cards, a message between agents
/// waiting for Send, and a client hook whose callback holds its connection
/// open until the card is answered. A snapshot that has not loaded is not
/// quiet.
pub fn idle(snapshot: &Value) -> bool {
    snapshot["approvals"]
        .as_array()
        .is_some_and(|cards| cards.is_empty())
}

/// What the updater may do with a downloaded, verified update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallFacts {
    /// The user pressed Install and Restart.
    pub asked: bool,
    /// "Install new versions automatically" is on.
    pub automatic: bool,
    /// This copy is an installed app the updater may replace.
    pub installable: bool,
    /// No card waits on the notch (`idle`).
    pub idle: bool,
    /// The last automatic install of this version failed; only the user
    /// tries it again.
    pub failed_before: bool,
    /// A demo, which never replaces the app.
    pub demo: bool,
}

/// Whether to install now. Asked by the user: at once. Automatically: only
/// when the notch is quiet, otherwise at the next quiet moment.
pub fn install_now(facts: InstallFacts) -> bool {
    if !facts.installable || facts.demo {
        return false;
    }
    facts.asked || (facts.automatic && facts.idle && !facts.failed_before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions_compare_as_semantic_versions() {
        assert!(newer("0.1.8", "0.1.7").unwrap());
        assert!(newer("0.1.10", "0.1.9").unwrap(), "not as text");
        assert!(newer("1.0.0", "0.9.9").unwrap());
        assert!(!newer("0.1.7", "0.1.7").unwrap(), "the same version");
        assert!(!newer("0.1.6", "0.1.7").unwrap(), "an older version");
        assert!(!newer("0.2.0-beta.1", "0.2.0").unwrap(), "a pre-release");
        assert!(newer("0.2.0", "0.2.0-beta.1").unwrap());
        assert!(newer("v0.1.8", "0.1.7").is_err());
        assert!(newer("0.1.8", "").is_err());
    }

    #[test]
    fn checks_fall_due_by_the_wall_clock() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        assert_eq!(until(now, now + FIRST_CHECK), Some(Duration::from_secs(60)));
        assert_eq!(until(now, now), None, "due now");
        // Six hours that passed while the Mac slept still count.
        assert_eq!(
            until(now + EVERY + Duration::from_secs(1), now + EVERY),
            None
        );
    }

    #[test]
    fn the_notch_is_quiet_only_with_no_card() {
        assert!(idle(&json!({"approvals": []})));
        assert!(!idle(
            &json!({"approvals": [{"id": "hook:claude:1:1", "kind": "permission"}]})
        ));
        assert!(!idle(
            &json!({"approvals": [{"id": "message:1", "kind": "question"}]})
        ));
        assert!(
            !idle(&json!({})),
            "a snapshot without cards read is not quiet"
        );
        assert!(!idle(&Value::Null));
    }

    fn facts() -> InstallFacts {
        InstallFacts {
            asked: false,
            automatic: true,
            installable: true,
            idle: true,
            failed_before: false,
            demo: false,
        }
    }

    #[test]
    fn automatic_installs_wait_for_a_quiet_notch() {
        assert!(install_now(facts()));
        assert!(
            !install_now(InstallFacts {
                idle: false,
                ..facts()
            }),
            "a card waits"
        );
        assert!(
            !install_now(InstallFacts {
                automatic: false,
                ..facts()
            }),
            "automatic install is off"
        );
        assert!(
            !install_now(InstallFacts {
                failed_before: true,
                ..facts()
            }),
            "it failed before"
        );
    }

    #[test]
    fn the_user_installs_at_once_but_never_into_a_copy_that_isnt_installed() {
        let asked = InstallFacts {
            asked: true,
            automatic: false,
            idle: false,
            failed_before: true,
            ..facts()
        };
        assert!(install_now(asked));
        assert!(!install_now(InstallFacts {
            installable: false,
            ..asked
        }));
        assert!(!install_now(InstallFacts {
            installable: false,
            ..facts()
        }));
        assert!(!install_now(InstallFacts {
            demo: true,
            ..asked
        }));
    }
}
