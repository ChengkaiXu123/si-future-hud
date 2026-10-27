//! Local build: messages into Codex Desktop threads on Windows, not yet.
//! On macOS and Linux (`message.rs`) a message is a new turn started over
//! the desktop app's IPC socket. On Windows that IPC is a named pipe
//! (`desktop_io/windows.rs`), and its address and the turn request there are
//! untested, so no carrier is offered: Codex threads don't receive messages,
//! as before this build added them. An adapter would connect `DesktopIpc` to
//! Desktop's pipe and send `thread-follower-start-turn` as `message.rs` does.

/// No carrier on Windows yet (see above).
pub fn carrier(
    _home: std::path::PathBuf,
) -> Option<std::sync::Arc<dyn agent_hud_core::replies::SessionReplies>> {
    None
}
