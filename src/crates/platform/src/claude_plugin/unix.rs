use std::{
    fs,
    io::Write,
    os::unix::{fs::FileTypeExt, net::UnixStream},
    path::Path,
    time::Duration,
};

pub(super) fn process_runs(pid: u32) -> bool {
    pid > 1 && unsafe { libc::kill(pid as libc::pid_t, 0) } == 0
}

pub(super) fn is_socket(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_socket())
}

/// Writes one line into the socket and closes its side.
pub(super) fn write_line(socket: &Path, line: &str) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket).map_err(|error| error.to_string())?;
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    stream
        .write_all(line.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|error| error.to_string())?;
    let _ = stream.shutdown(std::net::Shutdown::Write);
    Ok(())
}
