#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Local build: the app's own executable once it is installed (dragged into
/// Applications, ~/Applications or any folder) enables MCP setup (copy
/// config, adding the HUD tools) and makes client hooks installable. Not
/// while it runs from its disk image or from App Translocation (the random
/// read-only copy macOS runs a quarantined app from): entries written then
/// would point at a path that goes away.
fn installed_executable() -> Option<std::path::PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let path = executable.to_string_lossy();
    let in_bundle = path.contains(".app/Contents/MacOS/");
    let transient = path.starts_with("/Volumes/")
        || path.contains("/AppTranslocation/")
        || path.starts_with("/private/var/folders/");
    (cfg!(target_os = "macos") && in_bundle && !transient).then_some(executable)
}

fn main() {
    agent_hud_host::run_with_options(
        tauri::generate_context!(),
        None,
        agent_hud_host::HostOptions {
            canonical_executable: installed_executable(),
        },
    );
}
