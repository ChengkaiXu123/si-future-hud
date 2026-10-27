//! Windows: a named pipe for the lock's folder (`os::pipe`, channel
//! "instance"). A second launch sends "show" and the holder answers "shown"
//! once its window is asked to come forward.
use super::OnShow;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc, time::Duration};

const LIMIT: usize = 64;

pub(super) struct Endpoint {
    _server: crate::os::pipe::Server,
}

fn folder(lock: &Path) -> &Path {
    lock.parent().unwrap_or(lock)
}

pub(super) fn listen(lock: &Path, on_show: OnShow) -> Result<Endpoint, String> {
    crate::os::pipe::Server::start(
        folder(lock),
        "instance",
        LIMIT,
        LIMIT,
        Arc::new(move |request: Value, _| {
            (request == json!("show")).then(|| {
                on_show();
                json!("shown")
            })
        }),
    )
    .map(|server| Endpoint { _server: server })
}

pub(super) fn hand_over(lock: &Path) -> bool {
    crate::os::pipe::request(
        folder(lock),
        "instance",
        &json!("show"),
        LIMIT,
        LIMIT,
        Duration::from_secs(2),
        || false,
    )
    .ok()
    .flatten()
        == Some(json!("shown"))
}
