//! Local build: Agent HUD's 程小帮 plugin and HUD tools for Settings →
//! Agent plugins, set up on launch as soon as 程小帮 is on this Mac.
use agent_hud_runtime::chengxiaobang_plugin::ChengxiaobangPlugin;
use serde_json::{json, Value};
use std::time::SystemTime;
use tauri::Manager;

pub(crate) struct Chengxiaobang {
    pub(crate) plugin: ChengxiaobangPlugin,
}

/// Writes the plugin and lists the HUD tools in 程小帮's `mcp.json`, from
/// the fixed installed app only (a build elsewhere would list itself).
pub(crate) fn set_up(app: &tauri::AppHandle, plugin: &ChengxiaobangPlugin) -> Result<(), String> {
    plugin.prepare()?;
    if let Some(executable) = app
        .state::<super::HostOptions>()
        .canonical_executable
        .clone()
        .filter(|fixed| std::env::current_exe().ok().as_ref() == Some(fixed))
    {
        plugin.ensure_mcp(&executable)?;
    }
    Ok(())
}

fn status(plugin: &ChengxiaobangPlugin) -> Value {
    json!({
        "found": plugin.folder().parent().and_then(|p| p.parent()).is_some_and(|home| home.is_dir()),
        "installed": plugin.installed(),
        "sessions": plugin.loaded(SystemTime::now()),
        "mcp": plugin.mcp_listed(),
    })
}

#[tauri::command]
pub async fn native_chengxiaobang_plugin(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    action: String,
) -> Result<Value, String> {
    if window.label() != "main" {
        return Err("The 程小帮 plugin is set up in Settings".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let Some(state) = app.try_state::<Chengxiaobang>() else {
            return Ok(json!({"found":false,"installed":false,"sessions":0,"mcp":false}));
        };
        match action.as_str() {
            "status" => {}
            "install" => set_up(&app, &state.plugin)?,
            _ => return Err("Unknown plugin action".into()),
        }
        Ok(status(&state.plugin))
    })
    .await
    .map_err(|e| e.to_string())?
}
