use std::process::Command;

/// Windows has no exec, so the relay still waits on the server here.
pub(super) fn serve(mut command: Command) -> i32 {
    command
        .status()
        .map(|status| status.code().unwrap_or(1))
        .unwrap_or(1)
}
