//! Codex Desktop approvals on Windows: not implemented.
//!
//! On macOS and Linux the observer speaks Codex Desktop's IPC over a Unix
//! socket (`desktop.rs`, `desktop_io.rs`). A Windows adapter needs
//! the client's own Windows transport; until then this observer starts inert,
//! so Codex Desktop requests stay in the client and the HUD shows none.
use crate::hooks::Wake;
use agent_hud_core::requests::Requests;
use serde_json::Value;
use std::path::PathBuf;

pub struct CodexDesktopObserver;

/// The Codex daemon's control socket is a Unix socket; none is read on Windows.
pub(crate) fn daemon_tui_origin(
    _thread_id: &str,
    _home: &std::path::Path,
    _env: &std::collections::HashMap<String, String>,
) -> Option<String> {
    None
}

impl CodexDesktopObserver {
    pub fn start(
        _data_directory: PathBuf,
        _requests: Requests,
        _wake: Wake,
    ) -> Result<Self, String> {
        Ok(Self)
    }
    pub fn update_sessions(&self, _sessions: &[Value]) {}
}
