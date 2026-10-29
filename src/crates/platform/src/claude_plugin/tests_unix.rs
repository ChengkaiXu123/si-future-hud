use super::*;

#[test]
fn posts_a_message_into_a_running_sessions_own_inbox() {
    use std::io::Read;
    use std::os::unix::net::UnixListener;
    // Short enough for a socket path.
    let root = tempfile::Builder::new()
        .prefix("hud")
        .tempdir_in("/tmp")
        .unwrap();
    let config = root.path().join("claude");
    std::fs::create_dir_all(config.join("sessions")).unwrap();
    let socket = root.path().join("s.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let pid = std::process::id();
    std::fs::write(
        config.join(format!("sessions/{pid}.json")),
        serde_json::json!({"pid":pid,"sessionId":"live-1","messagingSocketPath":socket})
            .to_string(),
    )
    .unwrap();
    // A record whose process is gone, and one that names another pid, never count.
    std::fs::write(
        config.join("sessions/999999.json"),
        serde_json::json!({"pid":999999,"sessionId":"gone","messagingSocketPath":socket})
            .to_string(),
    )
    .unwrap();
    std::fs::write(
        config.join("sessions/1.json"),
        serde_json::json!({"pid":2,"sessionId":"odd","messagingSocketPath":socket}).to_string(),
    )
    .unwrap();
    let inboxes = native_inboxes(&config);
    assert_eq!(inboxes, vec![("live-1".to_owned(), socket.clone())]);
    post_to_inbox(&socket, "live-1", "hello from HUD").unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    let mut line = String::new();
    stream.read_to_string(&mut line).unwrap();
    let posted: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(posted["type"], "user");
    assert_eq!(posted["message"]["content"], "hello from HUD");
    assert_eq!(posted["session_id"], "live-1");
    assert_eq!(posted["from"], "SI Future HUD");
    assert!(posted.get("token").is_none());
}
