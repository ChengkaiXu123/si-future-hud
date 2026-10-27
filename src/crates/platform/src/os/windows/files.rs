//! Owner-only files on Windows: the owner SID and a protected per-user DACL.
use super::pipe;
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

/// A directory only this user can open, created when absent. A reparse point
/// or another user's directory is refused.
pub(crate) fn private_directory(path: &Path) -> Result<(), String> {
    pipe::private_directory(path)
}

pub(crate) fn owned_by_user(path: &Path, _metadata: &fs::Metadata) -> bool {
    pipe::owned_path(path)
}

/// Files inherit the private directory's protected DACL, so ownership is the check.
pub(crate) fn owner_only(path: &Path, metadata: &fs::Metadata) -> bool {
    owned_by_user(path, metadata)
}

/// New files have an explicit current-user owner even in elevated processes.
pub(crate) fn private_file(path: &Path, create_new: bool) -> io::Result<fs::File> {
    pipe::private_file(path, create_new)
}

/// A hook payload from stdin, or Err after five seconds or past `limit` bytes.
/// Windows console and pipe handles cannot be polled, so a thread reads.
pub(crate) fn read_stdin(bytes: &mut Vec<u8>, limit: usize) -> Result<(), ()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let ok = io::stdin()
            .lock()
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .is_ok();
        let _ = sender.send((ok, bytes));
    });
    let (ok, input) = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| ())?;
    if !ok || input.len() > limit {
        return Err(());
    }
    *bytes = input;
    Ok(())
}

/// The client's hook entry names the executable directly; no shell quoting.
pub(crate) fn hook_configuration(
    source: &str,
    flag: &str,
    current: &serde_json::Value,
    executable: Option<&Path>,
) -> Result<serde_json::Value, String> {
    agent_hud_agents::patch_configuration_for_windows(
        source,
        flag,
        current,
        executable.and_then(Path::to_str),
    )
}
