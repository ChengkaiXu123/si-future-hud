use std::{os::unix::process::CommandExt, process::Command};

/// Becomes the MCP server instead of waiting on it as a child. Clients stop a server
/// by signalling the process they started; a waiting relay would die alone and leave
/// the server running under launchd, still holding the client's pipes.
pub(super) fn serve(mut command: Command) -> i32 {
    let error = command.exec();
    eprintln!("The MCP relay could not start: {error}");
    1
}
