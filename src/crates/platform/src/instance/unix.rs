//! macOS and Linux: a Unix socket beside the lock, `instance.sock`. A second
//! launch writes `show` and the holder answers `shown` once its window is
//! asked to come forward.
use super::OnShow;
use std::{
    fs,
    io::{Read, Write},
    net::Shutdown,
    os::unix::{
        fs::{FileTypeExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    time::Duration,
};

const LIMIT: u64 = 16;

pub(super) struct Endpoint {
    _path: PathBuf,
}

fn socket(lock: &Path) -> PathBuf {
    lock.with_extension("sock")
}

pub(super) fn listen(lock: &Path, on_show: OnShow) -> Result<Endpoint, String> {
    let path = socket(lock);
    // Only the lock's holder gets here, so a socket already there was left
    // by a copy that ended.
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if !metadata.file_type().is_socket() {
            return Err("the path is occupied by another file".into());
        }
        fs::remove_file(&path).map_err(|error| error.to_string())?;
    }
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .map_err(|error| error.to_string())?;
    std::thread::Builder::new()
        .name("agenthud-instance".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                let mut request = Vec::new();
                if (&mut stream).take(LIMIT).read_to_end(&mut request).is_ok() && request == b"show"
                {
                    on_show();
                    let _ = stream.write_all(b"shown");
                }
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(Endpoint { _path: path })
}

pub(super) fn hand_over(lock: &Path) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket(lock)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let mut response = Vec::new();
    stream.write_all(b"show").is_ok()
        && stream.shutdown(Shutdown::Write).is_ok()
        && stream.take(LIMIT).read_to_end(&mut response).is_ok()
        && response == b"shown"
}
