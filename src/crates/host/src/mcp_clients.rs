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
/// Local build: Cursor, whose MCP servers SI Future HUD lists itself.
const CURSOR: &str = "cursor";
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
    if let Some(path) = super::system::login_shell_path() {
        dirs.extend(std::env::split_paths(path));
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs
}
/// The PATH a client's command line runs with: the usual install folders first.
pub(crate) fn search_path(home: &std::path::Path) -> Option<std::ffi::OsString> {
    std::env::join_paths(paths(home)).ok()
}
pub(crate) fn executable(client: &str, home: &std::path::Path) -> Option<PathBuf> {
    let command = client_command(client).ok()?;
    super::system::cli_candidates(home, command, paths(home))
        .into_iter()
        .find(|path| path.is_file())
}
/// Local build: whether `client` lists Agent HUD's MCP server at a program
/// that is there (an entry left at a deleted build doesn't count).
fn installed(client: &str, home: &std::path::Path) -> bool {
    if client == CURSOR {
        return cursor::entry(&cursor::config(home)).is_some_and(|entry| cursor::current(&entry));
    }
    listed_command(client, home).is_some_and(|command| std::path::Path::new(&command).is_file())
        && listed(client, home)
}
/// Local build: whether `client` is on this Mac to take the HUD tools.
fn available(client: &str, home: &std::path::Path) -> bool {
    if client == CURSOR {
        return cursor::found(home);
    }
    executable(client, home).is_some()
}
fn listed(client: &str, home: &std::path::Path) -> bool {
    if client == CURSOR {
        return cursor::entry(&cursor::config(home)).is_some();
    }
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
        json!({"ready":ready,"notice":if ready{None}else{Some("MCP setup requires the fixed installed app and its packaged relay")},"json":path.map(|path|json!({"mcpServers":{"agent-hud":{"command":path,"args":["--mcp"]}}})),"clients":(["claudeCode","codex",CURSOR].map(|client|json!({"id":client,"name":match client{"claudeCode"=>"Claude Code","codex"=>"Codex",_=>"Cursor"},"available":available(client,&home),"installed":installed(client,&home),"command":client_command(client).ok().zip(path).map(|(command,p)|super::system::client_command_line(&std::iter::once(command.into()).chain(args(client,true,p)).collect::<Vec<_>>()))}))) }),
    )
}
/// Local build: the command `client` lists for Agent HUD's MCP server, if any.
fn listed_command(client: &str, home: &std::path::Path) -> Option<String> {
    if client == CURSOR {
        return cursor::entry(&cursor::config(home))?["command"]
            .as_str()
            .map(str::to_owned);
    }
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
    let Some(owner) = app.try_state::<Clients>() else {
        return Ok(false);
    };
    let _guard = owner.0.lock().map_err(|_| "MCP client owner unavailable")?;
    let home = home()?;
    let Some(path) = app
        .state::<super::HostOptions>()
        .canonical_executable
        .clone()
    else {
        return Ok(false);
    };
    // Local build: Cursor's entry is the HUD's own to write, in one step.
    if client == CURSOR {
        let ours = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agent-hud-next");
        let moved = listed_command(client, &home).is_some_and(|listed| {
            std::path::Path::new(&listed) != path
                && (!std::path::Path::new(&listed).is_file()
                    || listed.ends_with(&format!(".app/Contents/MacOS/{ours}")))
        });
        return if moved && std::env::current_exe().ok().as_ref() == Some(&path) {
            cursor::write(&cursor::config(&home), Some(&path))
        } else {
            Ok(false)
        };
    }
    // A re-add that failed after its removal is finished first, so the
    // client never stays without the HUD tools.
    let pending =
        agent_hud_runtime::local_tools_directory()?.join(format!("mcp-repoint-{client}.pending"));
    let Some(listed) = listed_command(client, &home) else {
        if pending.exists() && executable(client, &home).is_some() {
            run(app, client, true)?;
            let _ = fs::remove_file(&pending);
            return Ok(true);
        }
        return Ok(false);
    };
    let ours = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("agent-hud-next");
    // Ours moved or renamed, or any entry whose program is gone (a deleted
    // build): pointed here. A developer's own build that is there stays.
    let gone = !std::path::Path::new(&listed).is_file();
    if std::path::Path::new(&listed) == path
        || !(gone || listed.ends_with(&format!(".app/Contents/MacOS/{ours}")))
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
    // As Settings does: one client command at a time.
    let Some(owner) = app.try_state::<Clients>() else {
        return Ok(false);
    };
    let _guard = owner.0.lock().map_err(|_| "MCP client owner unavailable")?;
    let home = home()?;
    if !available(client, &home) {
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
    // Local build: Cursor has no command that adds an MCP server; the HUD
    // writes its entry into Cursor's mcp.json.
    if client == CURSOR {
        if install && !cursor::found(&home) {
            return Err("Cursor is not installed".into());
        }
        cursor::write(&cursor::config(&home), install.then_some(path.as_path()))?;
        return status(app);
    }
    let executable =
        executable(client, &home).ok_or("The client's command line is not installed")?;
    // An entry left at a program that is gone is replaced, not refused.
    if install && listed(client, &home) && !installed(client, &home) {
        let _ = run_client(
            &executable,
            &args(client, false, path),
            &home,
            Duration::from_secs(15),
        );
    }
    match run_client(
        &executable,
        &args(client, install, path),
        &home,
        Duration::from_secs(15),
    ) {
        Err(error) if install && error.contains("already exists") => {}
        other => other?,
    }
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
        if let Some(path) = search_path(home) {
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

/// Local build: whether Codex can take messages now, for its card in
/// Settings: Codex is here, and either its app-server daemon answers
/// (`<codex home>/app-server-control/app-server-control.sock`, which
/// messages go through since Desktop 26.11) or Codex's CLI is installed to
/// start it. The old window channel (`ipc/ipc.sock`) still counts for older
/// Desktops.
#[tauri::command]
pub async fn native_codex_messages(window: tauri::WebviewWindow) -> Result<Value, String> {
    if window.label() != "main" {
        return Err("Codex's messages are shown in Settings".into());
    }
    tauri::async_runtime::spawn_blocking(|| {
        let Ok(home) = agent_hud_runtime::codex_client::data_directory() else {
            return json!({"found":false,"running":false});
        };
        let found = home.is_dir();
        let user = std::env::var_os("HOME").map(std::path::PathBuf::from);
        let cli = std::iter::once(std::path::PathBuf::from("/Applications"))
            .chain(user.map(|user| user.join("Applications")))
            .any(|apps| {
                apps.join("ChatGPT.app/Contents/Resources/codex-cli/bin/codex")
                    .is_file()
            });
        let running = found
            && (crate::system::socket_answers(
                &home.join("app-server-control/app-server-control.sock"),
            ) || crate::system::socket_answers(&home.join("ipc/ipc.sock"))
                || cli);
        json!({"found":found,"running":running})
    })
    .await
    .map_err(|e| e.to_string())
}

/// Local build: Agent HUD's MCP server in Cursor's own list,
/// `~/.cursor/mcp.json`. Cursor has no command that adds one, so the HUD
/// writes its entry, keeping every other server as it is. The entry names the
/// client (`AGENT_HUD_CLIENT`), as 程小帮's does: Cursor tells an MCP server
/// nothing about who calls it.
mod cursor {
    use serde_json::{json, Value};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    const NAME: &str = "agent-hud";

    /// Whether Cursor is on this Mac: its folder, made the first time it runs.
    pub(super) fn found(home: &Path) -> bool {
        home.join(".cursor").is_dir()
    }

    pub(super) fn config(home: &Path) -> PathBuf {
        home.join(".cursor/mcp.json")
    }

    fn read(file: &Path) -> Result<Value, String> {
        match fs::read(file) {
            Ok(bytes) if bytes.len() > 4 * 1024 * 1024 => {
                Err("Cursor's mcp.json is too large".into())
            }
            Ok(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => Ok(json!({})),
            Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
                .ok()
                .filter(Value::is_object)
                .ok_or_else(|| "Cursor's mcp.json is not a JSON object".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Agent HUD's entry in Cursor's list, if there is one.
    pub(super) fn entry(file: &Path) -> Option<Value> {
        read(file).ok()?["mcpServers"].get(NAME).cloned()
    }

    /// Whether an entry runs a program that is there, for Cursor.
    pub(super) fn current(entry: &Value) -> bool {
        entry["command"]
            .as_str()
            .is_some_and(|command| Path::new(command).is_file())
            && entry["env"]["AGENT_HUD_CLIENT"] == "cursor"
    }

    /// The entry for `executable`.
    pub(super) fn wanted(executable: &Path) -> Value {
        json!({"command": executable.to_string_lossy(), "args": ["--mcp"], "env": {"AGENT_HUD_CLIENT": "cursor"}})
    }

    /// Lists Agent HUD's server at `executable`, or takes it out (None),
    /// every other server kept. Returns whether the file changed.
    pub(super) fn write(file: &Path, executable: Option<&Path>) -> Result<bool, String> {
        let mut config = read(file)?;
        let root = config
            .as_object_mut()
            .ok_or("Cursor's mcp.json is not a JSON object")?;
        let servers = root.entry("mcpServers").or_insert_with(|| json!({}));
        let servers = servers
            .as_object_mut()
            .ok_or("Cursor's mcpServers is not a JSON object")?;
        match executable {
            Some(executable) => {
                let wanted = wanted(executable);
                if servers.get(NAME) == Some(&wanted) {
                    return Ok(false);
                }
                servers.insert(NAME.into(), wanted);
            }
            None => {
                if servers.remove(NAME).is_none() {
                    return Ok(false);
                }
            }
        }
        let mut text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        text.push('\n');
        let folder = file.parent().ok_or("Cursor's folder is unavailable")?;
        fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        let temporary = folder.join(format!(".mcp.json.agent-hud-{}.tmp", std::process::id()));
        if let Ok(metadata) = fs::metadata(file) {
            fs::write(&temporary, &text).map_err(|e| e.to_string())?;
            let _ = fs::set_permissions(&temporary, metadata.permissions());
        } else {
            fs::write(&temporary, &text).map_err(|e| e.to_string())?;
        }
        fs::rename(&temporary, file).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            error.to_string()
        })?;
        Ok(true)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_hud_tools_join_cursors_list_once_and_leave_it_cleanly() {
            let home = tempfile::tempdir().unwrap();
            let file = config(home.path());
            assert!(!found(home.path()));
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            assert!(found(home.path()));
            let others =
                json!({"mcpServers":{"github":{"command":"npx","args":["gh-mcp"]}},"other":true});
            fs::write(&file, others.to_string()).unwrap();
            let exe = home
                .path()
                .join("SI Future HUD.app/Contents/MacOS/agent-hud-next");
            fs::create_dir_all(exe.parent().unwrap()).unwrap();
            fs::write(&exe, "").unwrap();
            assert!(write(&file, Some(&exe)).unwrap());
            let written: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
            assert_eq!(
                written["mcpServers"]["github"],
                others["mcpServers"]["github"]
            );
            assert_eq!(written["other"], true);
            assert_eq!(
                written["mcpServers"]["agent-hud"],
                json!({"command":exe.to_string_lossy(),"args":["--mcp"],"env":{"AGENT_HUD_CLIENT":"cursor"}})
            );
            assert!(entry(&file).is_some_and(|entry| current(&entry)));
            // Again: nothing to change.
            assert!(!write(&file, Some(&exe)).unwrap());
            // An entry left without the client's name is not Cursor's yet.
            let mut bare = written.clone();
            bare["mcpServers"]["agent-hud"]["env"] = json!({});
            fs::write(&file, bare.to_string()).unwrap();
            assert!(!entry(&file).is_some_and(|entry| current(&entry)));
            assert!(write(&file, Some(&exe)).unwrap());
            // Taken out, only the others are left.
            assert!(write(&file, None).unwrap());
            let left: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
            assert_eq!(left, others);
            assert!(!write(&file, None).unwrap());
        }

        #[test]
        fn a_missing_list_is_made_and_a_broken_one_is_left_alone() {
            let home = tempfile::tempdir().unwrap();
            let file = config(home.path());
            let exe = Path::new("/Applications/SI Future HUD.app/Contents/MacOS/agent-hud-next");
            assert!(write(&file, Some(exe)).unwrap());
            assert_eq!(entry(&file), Some(wanted(exe)));
            fs::write(&file, "{ not json").unwrap();
            assert!(write(&file, Some(exe)).is_err());
            assert_eq!(fs::read_to_string(&file).unwrap(), "{ not json");
        }
    }
}
