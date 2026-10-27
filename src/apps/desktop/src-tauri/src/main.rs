#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    agent_hud_host::run_with_context(tauri::generate_context!(), None);
}
