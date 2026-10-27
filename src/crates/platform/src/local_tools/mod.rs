//! Local application tools channel, following the original public MCPAppChannel.
//! This is a per-user IPC adapter; handlers read the application's existing facts.
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

/// As large as a response: a tool's arguments can carry a whole text or file.
pub const REQUEST_LIMIT: u64 = 16 * 1024 * 1024;
pub const RESPONSE_LIMIT: u64 = 16 * 1024 * 1024;
pub type Handler = Arc<dyn Fn(Value, &AtomicBool) -> Value + Send + Sync>;

pub fn socket_path(data: &Path) -> PathBuf {
    data.join("tools/mcp.sock")
}
pub fn supported() -> bool {
    true
}

// A Unix socket, or a named pipe on Windows.
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{request, Server};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{request, Server};
