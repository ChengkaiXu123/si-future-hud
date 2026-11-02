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
/// doesn't inherit it. Given up after three seconds, reading included: a
/// background job the shell's startup files leave running keeps the output
/// open after the shell has exited, so it returns once the PATH is in.
pub(super) fn login_shell_path() -> Option<std::ffi::OsString> {
    shell_path(&std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into()))
}

fn shell_path(shell: &std::ffi::OsStr) -> Option<std::ffi::OsString> {
    use std::io::Read;
    use std::os::unix::ffi::OsStringExt;
    use std::sync::mpsc::RecvTimeoutError;
    const MARK: &str = "__HUD_PATH__";
    let mut child = std::process::Command::new(shell)
        .args(["-ilc", "printf '\\n__HUD_PATH__%s__HUD_PATH__' \"$PATH\""])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (sent, got) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(read) = stdout.read(&mut chunk) {
            if read == 0 || sent.send(chunk[..read].to_vec()).is_err() {
                break;
            }
        }
    });
    // The text between the two marks, once the closing one has come.
    let path = |output: &[u8]| -> Option<String> {
        let text = String::from_utf8_lossy(output);
        let mut parts = text.split(MARK);
        parts.next()?;
        let path = parts.next()?;
        parts.next()?;
        Some(path.to_owned())
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut output = Vec::new();
    let found = loop {
        if let Some(found) = path(&output) {
            break Some(found);
        }
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match got.recv_timeout(left) {
            Ok(chunk) => output.extend(chunk),
            Err(RecvTimeoutError::Timeout) => break None,
            Err(RecvTimeoutError::Disconnected) => break path(&output),
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    found
        .filter(|path| !path.is_empty())
        .map(|path| std::ffi::OsString::from_vec(path.into_bytes()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_job_left_holding_the_shell_output_does_not_hold_the_path() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let shell = temp.path().join("shell");
        // Startup files that leave a job running with the shell's output.
        std::fs::write(
            &shell,
            "#!/bin/sh\n/bin/sleep 6 &\nprintf '\\n__HUD_PATH__/opt/a/bin:/usr/bin__HUD_PATH__'\n",
        )
        .unwrap();
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
        let started = std::time::Instant::now();
        let path = super::shell_path(shell.as_os_str());
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert_eq!(
            path.as_deref(),
            Some(std::ffi::OsStr::new("/opt/a/bin:/usr/bin"))
        );
    }
}
