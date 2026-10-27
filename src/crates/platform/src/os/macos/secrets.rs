//! Generic passwords in the login keychain, read through /usr/bin/security.
use agent_hud_agents::Answer;
use std::{
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// The password of the generic keychain item named `service`, read within two
/// seconds and at most 32 KiB. macOS may ask the user to allow the read.
pub(crate) fn stored_secret(service: &str) -> Answer<String> {
    let Ok(mut child) = Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", service, "-w"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Answer::Failed;
    };
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // 44 is errSecItemNotFound.
                if status.code() == Some(44) {
                    return Answer::Missing;
                }
                if !status.success() {
                    return Answer::Failed;
                }
                let mut bytes = vec![];
                if let Some(stdout) = child.stdout.take() {
                    let _ = stdout.take(32768).read_to_end(&mut bytes);
                }
                return String::from_utf8(bytes).map_or(Answer::Failed, Answer::Found);
            }
            Err(_) => return Answer::Failed,
            Ok(None) => {
                if Instant::now() >= until {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Answer::Failed;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
