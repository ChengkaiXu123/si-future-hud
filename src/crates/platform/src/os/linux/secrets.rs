//! The system credential store: not implemented on this host.
//!
//! Clients such as gh keep tokens in the Secret Service (libsecret). Until an adapter reads it,
//! those readers fall back to their plain configuration files.

use agent_hud_agents::Answer;

pub(crate) fn stored_secret(_service: &str) -> Answer<String> {
    Answer::Unsupported
}
