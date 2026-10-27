//! Per-user Windows callback transport shared by public and private services.
//! Callers own their JSON protocol and byte limits; the platform owns pipe
//! access, deadlines and listener shutdown.
pub use crate::os::pipe::{request, Handler, Server};

/// Read at most `limit` bytes from hook stdin within five seconds.
pub fn read_stdin(bytes: &mut Vec<u8>, limit: usize) -> Result<(), String> {
    crate::os::read_stdin(bytes, limit)
        .map_err(|()| "The hook input exceeded its size or time limit".into())
}
