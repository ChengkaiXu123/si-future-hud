use std::{os::unix::process::CommandExt, process::Command};

/// Becomes the MCP server instead of waiting on it as a child. Clients stop a server
/// by signalling the process they started; a waiting relay would die alone and leave
/// the server running under launchd, still holding the client's pipes.
pub(super) fn serve(mut command: Command) -> i32 {
    let error = command.exec();
    eprintln!("The MCP relay could not start: {error}");
    1
}

/// This process's ancestors, nearest first: (pid, command), as `ps` names them.
pub(super) fn ancestors() -> Vec<(u32, String)> {
    let mut found = Vec::new();
    let mut pid = std::os::unix::process::parent_id();
    for _ in 0..32 {
        if pid <= 1 {
            break;
        }
        let Ok(output) = std::process::Command::new("/bin/ps")
            .args(["-o", "ppid=", "-o", "comm=", "-p", &pid.to_string()])
            .output()
        else {
            break;
        };
        let line = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let Some((parent, command)) = line.split_once(char::is_whitespace) else {
            break;
        };
        found.push((pid, command.trim().to_owned()));
        match parent.trim().parse::<u32>() {
            Ok(parent) if parent != pid => pid = parent,
            _ => break,
        }
    }
    found
}
