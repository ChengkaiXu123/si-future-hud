//! Windows launch forms with explicit documented argv/shell contracts. A client
//! whose Windows launch is unsupported is refused instead of getting a POSIX
//! command there.
use super::configuration::{configuration, patch_configuration};
use super::{hooks_of, Launch};
use serde_json::{json, Value};

/// Render one handler using the client's Windows launch contract. Both public
/// callbacks and product-specific callbacks use this representation.
pub fn windows_hook_handler(source: &str, flag: &str, executable: &str) -> Result<Value, String> {
    let (source, hooks) = hooks_of(source)?;
    if executable.is_empty() || executable.contains(['\r', '\n', '\0']) {
        return Err("The Windows hook executable path is invalid".into());
    }
    if !flag.starts_with("--")
        || flag.len() <= 2
        || !flag
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
    {
        return Err("The Windows hook flag is invalid".into());
    }
    match hooks.launch.windows.value() {
        Some(Launch::CommandArgs) => Ok(json!({
            "type": "command", "command": executable, "args": [flag, source]
        })),
        Some(Launch::ExecArgs) => Ok(json!({"exec": executable, "args": [flag, source]})),
        Some(Launch::PowerShell) => Ok(json!({
            "type": "command",
            "command": format!("'{}' {flag} {source}", executable.replace('\'', "'\\''")),
            "commandWindows": powershell(executable, flag, source)
        })),
        _ => Err(format!("{source} has no verified Windows hook launch contract; its configuration was not changed")),
    }
}

pub fn patch_configuration_for_windows(
    source: &str,
    flag: &str,
    current: &Value,
    executable: Option<&str>,
) -> Result<Value, String> {
    let description = configuration(source, flag)?;
    let source = description.source.as_str();
    let (_, hooks) = hooks_of(source)?;
    let launch = hooks.launch.windows.value();
    if launch.is_none() && executable.is_some() {
        return Err(format!("{source} has no verified Windows hook launch contract; its configuration was not changed"));
    }
    // A client launched with `exec` lists its handlers under the event; the
    // others group them.
    let exec = matches!(launch, Some(Launch::ExecArgs));
    let mut current = current.clone();
    // Reduce only our exact exec+args handlers to the same owned-command model
    // used for install/update/uninstall. Foreign exec handlers stay untouched.
    for event in &description.events {
        if let Some(groups) = current
            .get_mut("hooks")
            .and_then(|h| h.get_mut(event))
            .and_then(Value::as_array_mut)
        {
            if exec {
                for handler in groups {
                    normalize(handler, source, flag, "bash");
                }
            } else {
                for group in groups {
                    if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        for handler in handlers {
                            normalize(handler, source, flag, "command");
                        }
                    }
                }
            }
        }
    }
    let marker =
        executable.map(|path| format!("'{}' {flag} {source}", path.replace('\'', "'\\''")));
    let mut updated = patch_configuration(source, flag, &current, marker.as_deref())?;
    let Some(executable) = executable else {
        return Ok(updated);
    };
    let windows_handler = windows_hook_handler(source, flag, executable)?;
    for event in &description.events {
        if let Some(groups) = updated
            .get_mut("hooks")
            .and_then(|h| h.get_mut(event))
            .and_then(Value::as_array_mut)
        {
            if exec {
                for handler in groups {
                    if handler["bash"].as_str() == marker.as_deref() {
                        let fields = handler.as_object_mut().unwrap();
                        fields.remove("bash");
                        fields.remove("powershell");
                        fields.remove("command");
                        fields.extend(windows_handler.as_object().unwrap().clone());
                    }
                }
            } else {
                for group in groups {
                    if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        for handler in handlers {
                            if handler["command"].as_str() != marker.as_deref() {
                                continue;
                            }
                            handler
                                .as_object_mut()
                                .unwrap()
                                .extend(windows_handler.as_object().unwrap().clone());
                        }
                    }
                }
            }
        }
    }
    Ok(updated)
}
fn normalize(handler: &mut Value, source: &str, flag: &str, key: &str) {
    if handler["args"] != json!([flag, source]) {
        return;
    }
    let path = handler[if key == "bash" { "exec" } else { "command" }]
        .as_str()
        .filter(|p| !p.is_empty())
        .map(str::to_owned);
    let Some(path) = path else {
        return;
    };
    let Some(fields) = handler.as_object_mut() else {
        return;
    };
    fields.remove("exec");
    fields.remove("args");
    fields.insert(key.into(), json!(format!("'{path}' {flag} {source}")));
}
fn powershell(executable: &str, flag: &str, source: &str) -> String {
    use base64::Engine;
    let script = format!(
        "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); & '{}' {flag} {source}",
        executable.replace('\'', "''")
    );
    let bytes = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    format!(
        "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand {}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn remote_callbacks_use_the_same_windows_launch_contract() {
        let executable = r"C:\Users\O'Brien\Agent HUD.exe";
        let claude = windows_hook_handler("claude", "--remote-hook", executable).unwrap();
        assert_eq!(claude["command"], executable);
        assert_eq!(claude["args"], json!(["--remote-hook", "claude"]));
        let codex = windows_hook_handler("codex", "--remote-hook", executable).unwrap();
        let encoded = codex["commandWindows"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        let words = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes(*bytes))
            .collect::<Vec<_>>();
        let script = String::from_utf16(&words).unwrap();
        assert!(script.ends_with("& 'C:\\Users\\O''Brien\\Agent HUD.exe' --remote-hook codex"));
        assert!(windows_hook_handler("codex", "--remote-hook; exit", executable).is_err());
        assert!(windows_hook_handler("claude", "--remote-hook", "bad\npath").is_err());
    }
}
