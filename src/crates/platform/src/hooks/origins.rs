//! Kernel-reported ancestry, not a callback's self-reported host. Environment is
//! read only from this callback; parent process environments are never decoded.
//! Platform gathers the caller; each client's descriptor rules decide the host
//! (`agents::process`).
use agent_hud_agents::process::{Ancestor, Caller};
use serde_json::{json, Value};
use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
/// The callback's parent processes, nearest first, shells left out.
fn ancestors() -> Vec<Ancestor> {
    crate::os::parent_processes()
        .into_iter()
        .filter(|process| !crate::os::is_shell(&process.executable))
        .map(|process| Ancestor {
            resolved: process.executable.canonicalize().ok(),
            executable: process.executable,
            arguments: process.arguments,
            bundle: process.bundle,
        })
        .collect()
}
/// The client a hook names, by its id or an older name.
fn agent_of(source: &str) -> Option<&'static agent_hud_agents::Agent> {
    agent_hud_agents::by_id(agent_hud_agents::hooks::hooks_of(source).ok()?.0)
}
/// Runs `rule` on the hook's caller. Whether a path is the client's
/// installed program is worked out only if the rule asks.
fn with_caller<T>(
    agent: &agent_hud_agents::Agent,
    home: &Path,
    env: &HashMap<String, String>,
    rule: impl FnOnce(&Caller) -> T,
) -> T {
    let ancestors = ancestors();
    let candidates: OnceCell<HashSet<PathBuf>> = OnceCell::new();
    let installed = |path: &Path| {
        let candidates = candidates.get_or_init(|| {
            agent
                .program
                .map(|program| crate::os::engine_candidates(home, env, program))
                .unwrap_or_default()
                .into_iter()
                .filter_map(|path| path.canonicalize().ok())
                .collect()
        });
        path.canonicalize()
            .is_ok_and(|path| candidates.contains(&path))
    };
    let caller = Caller {
        ancestors: &ancestors,
        env,
        installed: &installed,
    };
    rule(&caller)
}
pub(super) fn origin(
    source: &str,
    session: &str,
    home: &Path,
    env: &HashMap<String, String>,
) -> Value {
    let Some((agent, rule)) =
        agent_of(source).and_then(|agent| Some((agent, agent.hooks?.origin?)))
    else {
        return json!({"hostVerified":true,"host":"unsupported","navigationTarget":null});
    };
    let mut origin = with_caller(agent, home, env, |caller| rule(caller, session));
    if origin.ask_channel {
        if let Some(target) = crate::clients::session_terminal(agent.id, session, home, env) {
            origin.target = target;
        }
    }
    if origin.host == "cli" && origin.interactive {
        if let Some(target) = iterm_session(env) {
            origin.target = target;
        }
    }
    json!({"hostVerified":true,"host":origin.host,"navigationTarget":origin.target})
}
/// The iTerm session a terminal process runs in, from its environment; none
/// inside tmux, whose pane iTerm cannot be told to show.
pub(crate) fn iterm_session(env: &HashMap<String, String>) -> Option<Value> {
    if env.get("TERM_PROGRAM").map(String::as_str) != Some("iTerm.app")
        || env.get("TMUX").is_some_and(|s| !s.is_empty())
    {
        return None;
    }
    let id = env.get("ITERM_SESSION_ID").filter(|s| !s.is_empty())?;
    Some(json!({"kind":"iterm-session","id":id}))
}
/// Whether the hook comes from a run nobody watches, by the client's rule.
pub(super) fn unattended(source: &str, home: &Path, env: &HashMap<String, String>) -> bool {
    let Some((agent, rule)) = agent_of(source).and_then(|agent| {
        let rule = agent.hooks?.permission.as_ref()?.unattended?;
        Some((agent, rule))
    }) else {
        return false;
    };
    with_caller(agent, home, env, rule)
}
pub(crate) fn is_uuid(text: &str) -> bool {
    text.len() == 36
        && text.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

pub(crate) fn loopback_port(origin: &str) -> Option<u16> {
    let rest = origin.strip_prefix("http://127.0.0.1:")?;
    let digits = rest.strip_suffix('/').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u16>().ok().filter(|port| *port > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_loopback_port_matches_source() {
        assert_eq!(loopback_port("http://127.0.0.1:54321"), Some(54321));
        assert_eq!(loopback_port("http://127.0.0.1:54321/"), Some(54321));
        assert_eq!(loopback_port("http://localhost:54321"), None);
        assert_eq!(loopback_port("http://127.0.0.1:0"), None);
    }

    #[test]
    fn iterm_is_a_target_outside_tmux() {
        let env = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<HashMap<_, _>>()
        };
        let iterm = [
            ("TERM_PROGRAM", "iTerm.app"),
            ("ITERM_SESSION_ID", "w0t0p0:1"),
        ];
        assert_eq!(
            iterm_session(&env(&iterm)),
            Some(json!({"kind":"iterm-session","id":"w0t0p0:1"}))
        );
        let mut tmux = iterm.to_vec();
        tmux.push(("TMUX", "/tmp/tmux"));
        assert_eq!(iterm_session(&env(&tmux)), None);
        assert_eq!(
            iterm_session(&env(&[("TERM_PROGRAM", "Apple_Terminal")])),
            None
        );
    }
}
