#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Local build: the app's own executable once it is installed, which enables
// MCP setup (copy config, adding the HUD tools) and makes client hooks
// installable. Each host decides what "installed" means.
#[cfg(target_os = "macos")]
#[path = "installed/macos.rs"]
mod installed;
#[cfg(target_os = "windows")]
#[path = "installed/windows.rs"]
mod installed;
#[cfg(target_os = "linux")]
#[path = "installed/linux.rs"]
mod installed;

fn main() {
    agent_hud_host::run_with_options(
        tauri::generate_context!(),
        None,
        agent_hud_host::HostOptions {
            canonical_executable: installed::installed_executable(),
        },
    );
}
