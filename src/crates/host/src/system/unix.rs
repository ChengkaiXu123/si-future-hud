pub(super) fn client_command_line(arguments: &[String]) -> String {
    arguments
        .iter()
        .map(|argument| format!("'{}'", argument.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn configure_client_command(_command: &mut std::process::Command) {}

pub(super) fn stop_client(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(super) fn socket_answers(path: &std::path::Path) -> bool {
    std::os::unix::net::UnixStream::connect(path).is_ok()
}

/// The login shell's PATH. Version managers (nvm, volta, fnm, asdf, mise,
/// pnpm) put the clients' commands there, and an app opened from the Finder
/// doesn't inherit it. Given up after three seconds.
pub(super) fn login_shell_path() -> Option<std::ffi::OsString> {
    use std::io::Read;
    use std::os::unix::ffi::OsStringExt;
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
    let mut child = std::process::Command::new(shell)
        .args(["-ilc", "printf '\\n__HUD_PATH__%s__HUD_PATH__' \"$PATH\""])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while child.try_wait().ok()?.is_none() {
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let mut output = Vec::new();
    child.stdout.take()?.read_to_end(&mut output).ok()?;
    let text = String::from_utf8_lossy(&output);
    let path = text.split("__HUD_PATH__").nth(1)?;
    (!path.is_empty()).then(|| std::ffi::OsString::from_vec(path.as_bytes().to_vec()))
}
