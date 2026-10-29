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
