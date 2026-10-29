use std::path::Path;

pub(super) fn process_runs(_pid: u32) -> bool {
    false
}

pub(super) fn is_socket(_path: &Path) -> bool {
    false
}

pub(super) fn write_line(_socket: &Path, _line: &str) -> Result<(), String> {
    Err("Claude Code's session inbox is not reachable here".into())
}
