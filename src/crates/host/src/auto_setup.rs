//! Local build: what Agent HUD sets up by itself on its first launch, as
//! 程小帮's integration is, so no click in Settings is needed: SI Future HUD's
//! Claude Code plugin, the HUD tools in Claude Code and Codex, the message
//! skill (`agent_skills`), and the
//! switches they need (the HUD tools, client hooks for HUD approvals). Each is done once and recorded in `auto-setup.json`:
//! one the user removes later in Settings is not put back, and one whose
//! client is not here yet is tried again on a later launch.
use serde_json::{json, Map, Value};
use std::{fs, sync::Arc};
use tauri::Manager;

pub(crate) fn on_launch(app: &tauri::AppHandle) {
    // The demo leaves the user's clients alone.
    if app.state::<super::AppState>().demo {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(data) = agent_hud_runtime::local_tools_directory() else {
            return;
        };
        let file = data.join("auto-setup.json");
        let mut done: Map<String, Value> = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let before = done.clone();
        let now = chrono::Utc::now().to_rfc3339();
        let mut step = |key: &str, run: &dyn Fn() -> Result<bool, String>| {
            if done.contains_key(key) {
                return;
            }
            match run() {
                Ok(true) => {
                    done.insert(key.into(), json!(now));
                }
                Ok(false) => {}
                Err(error) => eprintln!("SI Future HUD setup ({key}): {error}"),
            }
        };
        step("hudTools", &|| turn_on(&app, "hudTools"));
        step("claudePlugin", &|| super::claude_plugin::install_once(&app));
        step("claudeMcp", &|| {
            super::mcp_clients::add_once(&app, "claudeCode")
        });
        step("codexMcp", &|| super::mcp_clients::add_once(&app, "codex"));
        step("clientHooks", &|| turn_on(&app, "clientHooks"));
        // Every launch: the message skill for Claude Code and Codex, which
        // reaches windows already open (written once, then kept current).
        if let (Some(executable), Ok(home)) = (
            super::chengxiaobang::fixed_executable(&app),
            agent_hud_runtime::home_directory(),
        ) {
            if let Err(error) = super::agent_skills::ensure(&home, &executable, &mut done, &now) {
                eprintln!("SI Future HUD setup (skills): {error}");
            }
        }
        // Once on each Mac: Settings opens on Get started, the guide that
        // shows what was set up and the step left for the person.
        if !done.contains_key("guide") {
            super::show_settings_page(&app, "get-started".into());
            done.insert("guide".into(), json!(now));
        }
        // Every launch: a login item left at a moved or renamed app is bound to
        // this one (macOS keeps launching the bundle it was registered from).
        if let Some(bundle) = app
            .state::<super::HostOptions>()
            .canonical_executable
            .as_ref()
            .and_then(|path| path.ancestors().nth(3))
            .map(|bundle| bundle.to_string_lossy().into_owned())
        {
            let login = app
                .try_state::<super::Service>()
                .and_then(|service| {
                    service
                        .backend
                        .lock()
                        .ok()
                        .map(|backend| backend.runtime.launch_at_login_enabled())
                })
                .unwrap_or(false);
            if login && done.get("loginItemBundle").and_then(Value::as_str) != Some(bundle.as_str())
            {
                let system = app.state::<super::system::System>();
                // An item left by another bundle may not unregister from here;
                // registering this bundle is what counts, and is tried again on
                // the next launch until it holds.
                let _ = system.set_setting("launchAtLogin", &json!(false));
                match system.set_setting("launchAtLogin", &json!(true)) {
                    Ok(Some(Value::Bool(true))) => {
                        done.insert("loginItemBundle".into(), json!(bundle));
                    }
                    Ok(_) => eprintln!("SI Future HUD login item: not enabled yet"),
                    Err(error) => eprintln!("SI Future HUD login item: {error}"),
                }
            }
        }
        // Every launch: entries left at a moved or renamed app point here again.
        for client in ["claudeCode", "codex"] {
            if let Err(error) = super::mcp_clients::repoint(&app, client) {
                eprintln!("SI Future HUD setup ({client} MCP): {error}");
            }
        }
        if done != before {
            if let Ok(text) = serde_json::to_string_pretty(&done) {
                let _ = fs::write(&file, text);
            }
        }
    });
}

/// A switch the integrations need: the HUD tools answer agents only while
/// "Let agents arrange the HUD" is on; HUD approvals need client hooks on.
fn turn_on(app: &tauri::AppHandle, key: &str) -> Result<bool, String> {
    // Hooks point at the installed app only: from a disk image there is none.
    if key == "clientHooks" {
        if app
            .state::<super::HostOptions>()
            .canonical_executable
            .is_none()
        {
            return Ok(false);
        }
        let Some(service) = app.try_state::<super::Service>() else {
            return Ok(false);
        };
        let params = app
            .state::<super::system::System>()
            .set_setting("clientHooks", &json!(true))?
            .unwrap_or(json!(true));
        // As Settings turns it on: the hooks installed first, the switch saved
        // after; a client that refuses them leaves it off, tried again next launch.
        super::set_client_hooks(
            app,
            &service.hooks,
            &service.hook_actions,
            &service.stopped,
            &service.backend,
            json!({"key":"clientHooks","value":params}),
        )?;
        return Ok(true);
    }
    let Some(service) = app.try_state::<super::Service>() else {
        return Ok(false);
    };
    let backend = Arc::clone(&service.backend);
    let mut backend = backend
        .lock()
        .map_err(|_| "Runtime lock is unavailable".to_owned())?;
    backend.request(app, "set_setting", json!({"key":key,"value":true}))?;
    Ok(true)
}
