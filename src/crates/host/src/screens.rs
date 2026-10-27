//! Display changes wake the app's HUD window planner, which measures the screens
//! again. A host without a change notification is measured every `POLL`.
#[cfg(target_os = "macos")]
#[path = "screens/macos.rs"]
mod host;
#[cfg(target_os = "windows")]
#[path = "screens/windows.rs"]
mod host;
#[cfg(target_os = "linux")]
#[path = "screens/linux.rs"]
mod host;
pub use host::{install, shutdown, POLL};
