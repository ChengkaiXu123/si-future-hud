//! Client-owned MCP installation, only after an explicit Settings action.
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::Manager;
#[derive(Default)]
pub struct Clients(Mutex<()>);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub(crate) fn home() -> Result<PathBuf, String> {
    agent_hud_runtime::home_directory()
}
fn client_command(client: &str) -> Result<&'static str, String> {
    match client {
        "claudeCode" => Ok("claude"),
        "codex" => Ok("codex"),
        _ => Err("Unknown MCP client".into()),
    }
}
fn paths(home: &std::path::Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        home.join(".bun/bin"),
        home.join(".local/bin"),
        home.join(".npm-global/bin"),
        "/opt/homebrew/bin".into(),
        "/usr/local/bin".into(),
        "/usr/bin".into(),
        "/bin".into(),
    ];
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs
}
pub(crate) fn executable(client: &str, home: &std::path::Path) -> Option<PathBuf> {
    let command = client_command(client).ok()?;
    super::system::cli_candidates(home, command, paths(home))
        .into_iter()
        .find(|path| path.is_file())
}
fn installed(client: &str, home: &std::path::Path) -> bool {
    let path = if client == "claudeCode" {
        std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.into())
            .join(".claude.json")
    } else {
        std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"))
            .join("config.toml")
    };
    let Ok(meta) = fs::metadata(&path) else {
        return false;
    };
    if meta.len() > 4 * 1024 * 1024 {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    if client == "claudeCode" {
        serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["mcpServers"].get("agent-hud").cloned())
            .is_some()
    } else {
        toml::from_str::<toml::Value>(&text)
            .ok()
            .and_then(|v| {
                v.get("mcp_servers")
                    .and_then(|v| v.get("agent-hud"))
                    .cloned()
            })
            .is_some()
    }
}
fn args(client: &str, install: bool, path: &std::path::Path) -> Vec<String> {
    let mut args = vec![
        "mcp".into(),
        if install {
            "add".into()
        } else {
            "remove".into()
        },
    ];
    if client == "claudeCode" {
        args.extend(["--scope".into(), "user".into()]);
    }
    args.push("agent-hud".into());
    if install {
        args.extend([
            "--".into(),
            path.to_string_lossy().into_owned(),
            "--mcp".into(),
        ]);
    }
    args
}
fn status(app: &tauri::AppHandle) -> Result<Value, String> {
    let home = home()?;
    let options = app.state::<super::HostOptions>();
    let path = options.canonical_executable.as_ref();
    let ready = path.is_some_and(|path| std::env::current_exe().ok().as_ref() == Some(path))
        && super::cli::resource("agent-hud-mcp").is_some()
        && super::cli::resource("agent-hud-engine").is_some();
    Ok(
        json!({"ready":ready,"notice":if ready{None}else{Some("MCP setup requires the fixed installed app and its packaged relay")},"json":path.map(|path|json!({"mcpServers":{"agent-hud":{"command":path,"args":["--mcp"]}}})),"clients":(["claudeCode","codex"].map(|client|json!({"id":client,"name":if client=="claudeCode"{"Claude Code"}else{"Codex"},"available":executable(client,&home).is_some(),"installed":installed(client,&home),"command":path.map(|p|super::system::client_command_line(&std::iter::once(client_command(client).unwrap().into()).chain(args(client,true,p)).collect::<Vec<_>>()))}))) }),
    )
}
/// Local build: the command `client` lists for Agent HUD's MCP server, if any.
fn listed_command(client: &str, home: &std::path::Path) -> Option<String> {
    let path = if client == "claudeCode" {
        std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.into())
            .join(".claude.json")
    } else {
        std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"))
            .join("config.toml")
    };
    if fs::metadata(&path).ok()?.len() > 4 * 1024 * 1024 {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    if client == "claudeCode" {
        serde_json::from_str::<Value>(&text).ok()?["mcpServers"]["agent-hud"]["command"]
            .as_str()
            .map(str::to_owned)
    } else {
        toml::from_str::<toml::Value>(&text)
            .ok()?
            .get("mcp_servers")?
            .get("agent-hud")?
            .get("command")?
            .as_str()
            .map(str::to_owned)
    }
}

/// Local build: when the app was moved or renamed, `client`'s entry for the
/// HUD tools still names the old place; it is pointed at this app again.
/// Only an entry that names this app's own executable elsewhere is touched.
pub(crate) fn repoint(app: &tauri::AppHandle, client: &str) -> Result<bool, String> {
    let home = home()?;
    let Some(path) = app.state::<super::HostOptions>().canonical_executable.clone() else {
        return Ok(false);
    };
    // A re-add that failed after its removal is finished first, so the
    // client never stays without the HUD tools.
    let pending = agent_hud_runtime::local_tools_directory()?.join(format!("mcp-repoint-{client}.pending"));
    let Some(listed) = listed_command(client, &home) else {
        if pending.exists() && executable(client, &home).is_some() {
            run(app, client, true)?;
            let _ = fs::remove_file(&pending);
            return Ok(true);
        }
        return Ok(false);
    };
    let ours = path.file_name().and_then(|name| name.to_str()).unwrap_or("agent-hud-next");
    if std::path::Path::new(&listed) == path
        || !listed.ends_with(&format!(".app/Contents/MacOS/{ours}"))
        || executable(client, &home).is_none()
    {
        return Ok(false);
    }
    fs::write(&pending, &listed).map_err(|error| error.to_string())?;
    run(app, client, false)?;
    run(app, client, true)?;
    let _ = fs::remove_file(&pending);
    Ok(true)
}

/// Local build: adds Agent HUD's MCP server to `client` when its command line
/// is here and it lists none. Ok(false) while the client is not here.
pub(crate) fn add_once(app: &tauri::AppHandle, client: &str) -> Result<bool, String> {
    let home = home()?;
    if executable(client, &home).is_none() {
        return Ok(false);
    }
    if !installed(client, &home) {
        run(app, client, true)?;
    }
    Ok(true)
}
fn run(app: &tauri::AppHandle, client: &str, install: bool) -> Result<Value, String> {
    let home = home()?;
    let options = app.state::<super::HostOptions>();
    let path = options
        .canonical_executable
        .as_ref()
        .ok_or("No fixed application executable is delegated")?;
    if std::env::current_exe().map_err(|e| e.to_string())? != *path {
        return Err("MCP setup is only available in the fixed installed app".into());
    }
    if install && super::cli::resource("agent-hud-mcp").is_none() {
        return Err("The packaged MCP relay is missing".into());
    }
    let executable =
        executable(client, &home).ok_or("The client's command line is not installed")?;
    run_client(
        &executable,
        &args(client, install, path),
        &home,
        Duration::from_secs(15),
    )?;
    status(app)
}
/// Runs a client's command line in `home` and waits up to `timeout`; Err with
/// the first line it printed when it fails.
pub(crate) fn run_client(
    executable: &std::path::Path,
    arguments: &[String],
    home: &std::path::Path,
    timeout: Duration,
) -> Result<(), String> {
    // A temporary file avoids waiting on descendant processes that keep an output pipe open.
    let output_path = std::env::temp_dir().join(format!(
        "agenthud-client-{}-{}.log",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    super::system::private_output(&mut options);
    let output = options.open(&output_path).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut command = Command::new(executable);
        super::system::configure_client_command(&mut command);
        command
            .args(arguments)
            .current_dir(home)
            .stdin(Stdio::null())
            .stdout(output.try_clone().map_err(|e| e.to_string())?)
            .stderr(output);
        if let Ok(path) = std::env::join_paths(paths(home)) {
            command.env("PATH", path);
        }
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let deadline = Instant::now() + timeout;
        let exit_status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if Instant::now() >= deadline {
                super::system::stop_client(&mut child);
                return Err(format!(
                    "The client did not finish within {} seconds",
                    timeout.as_secs()
                ));
            }
            std::thread::sleep(Duration::from_millis(30));
        };
        if !exit_status.success() {
            use std::io::Read;
            let mut bytes = vec![];
            fs::File::open(&output_path)
                .map_err(|e| e.to_string())?
                .take(65536)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            let line = String::from_utf8_lossy(&bytes)
                .lines()
                .find(|s| !s.trim().is_empty())
                .unwrap_or("The client's command failed")
                .chars()
                .take(160)
                .collect::<String>();
            return Err(line);
        }
        Ok(())
    })();
    let _ = fs::remove_file(output_path);
    result
}
#[tauri::command]
pub async fn native_mcp_clients(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    action: String,
    client: Option<String>,
) -> Result<Value, String> {
    if window.label() != "main" {
        return Err("MCP setup is owned by Settings".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let owner = app.state::<Clients>();
        let _guard = owner.0.lock().map_err(|_| "MCP client owner unavailable")?;
        match action.as_str() {
            "status" => status(&app),
            "install" | "remove" => run(
                &app,
                client.as_deref().ok_or("Client is required")?,
                action == "install",
            ),
            _ => Err("Unknown MCP setup action".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
