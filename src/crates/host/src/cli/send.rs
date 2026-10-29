//! Local build: `agent-hud-next send`, the HUD's message tool for any agent
//! window that can run a command, however long it has been open (the MCP tool
//! reaches only sessions started after it was added). It finds the calling
//! session itself, never from its arguments: a Claude Code session from the
//! session file of the `claude` process it runs under, Codex and 程小帮 from
//! the thread or session their shells carry. The card on the HUD is the gate,
//! as it is for the MCP tool; with `--wait` the reply prints here.
use agent_hud_runtime::local_tools;
use serde_json::{json, Value};
use std::{
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const HELP: &str = "\
agent-hud-next send — 通过 SI Future HUD 给另一个智能体窗口发消息 / message another agent's window through SI Future HUD

  agent-hud-next send --to <claude|codex|chengxiaobang> [--session <id>] [--wait <秒/seconds>] --text \"…\"
  agent-hud-next send --to codex --wait 1800 <<'EOF'
  …长消息 / a long message…
  EOF

每条消息都要用户在 HUD 上点「发送」才会送出。--wait 会一直等到对方回信并把回信打印出来（最多等这么多秒）。
Every message waits for the user to press Send on the HUD. --wait prints the reply when it comes (up to that many seconds).
";

/// What a run prints and exits with.
fn fail(message: impl AsRef<str>) -> i32 {
    eprintln!("{}", message.as_ref());
    1
}

pub(super) fn run(arguments: &[String]) -> i32 {
    let mut to = None;
    let mut session = None;
    let mut wait = 0u64;
    let mut text: Option<String> = None;
    let mut words = Vec::new();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return 0;
            }
            "--to" => to = rest.next().cloned(),
            "--session" => session = rest.next().cloned(),
            "--text" => text = rest.next().cloned(),
            "--wait" => match rest.next().map(|value| value.parse::<u64>()) {
                Some(Ok(seconds)) => wait = seconds.min(3600),
                _ => return fail("--wait 要跟秒数 / --wait takes a number of seconds"),
            },
            other if other.starts_with("--") => {
                return fail(format!("不认识的参数 / unknown option: {other}\n\n{HELP}"))
            }
            other => words.push(other.to_owned()),
        }
    }
    let Some(to) = to else {
        return fail(format!("缺少 --to / --to is required\n\n{HELP}"));
    };
    let text = match text.as_deref() {
        Some("-") => None,
        Some(given) => Some(given.to_owned()),
        None if !words.is_empty() => Some(words.join(" ")),
        None => None,
    };
    let text = match text {
        Some(text) => text,
        None if !std::io::stdin().is_terminal() => {
            let mut read = String::new();
            if std::io::stdin().read_to_string(&mut read).is_err() {
                return fail("读不到标准输入 / could not read standard input");
            }
            read
        }
        None => return fail(format!("缺少消息内容 / no message given\n\n{HELP}")),
    };
    let caller = match caller() {
        Ok(caller) => caller,
        Err(error) => return fail(error),
    };
    let data = match agent_hud_runtime::local_tools_directory() {
        Ok(data) => data,
        Err(error) => return fail(error),
    };
    let ask = |method: &str, params: Value| -> Result<Value, String> {
        let answer = local_tools::request(&data, &json!({"id":1,"method":method,"params":params}))?;
        match answer.get("error") {
            Some(error) => Err(error["message"]
                .as_str()
                .unwrap_or("SI Future HUD refused the request")
                .to_owned()),
            None => Ok(answer["result"].clone()),
        }
    };
    let mut caller = caller;
    let waits = wait > 0 && caller["sessionId"].is_string();
    if waits {
        caller["waits"] = json!(true);
        // Waiting before the card exists: the reply can't come before it.
        if let Err(error) = ask(
            "agent_inbox",
            json!({"caller":caller,"waitMs":wait * 1000 + 60_000}),
        ) {
            return fail(error);
        }
    }
    let mut params = json!({"to":to,"text":text,"caller":caller});
    if let Some(session) = session {
        params["session"] = json!(session);
    }
    let sent = match ask("send_message", params) {
        Ok(sent) => sent,
        Err(error) => {
            if waits {
                let _ = ask("agent_inbox", json!({"caller":caller,"waitMs":0}));
            }
            return fail(error);
        }
    };
    let title = sent["to"]["title"].as_str().unwrap_or_default();
    println!(
        "{}",
        sent["note"].as_str().unwrap_or(
            "已提交，等用户在 HUD 上确认。/ Submitted; the user confirms it on the HUD."
        )
    );
    if !title.is_empty() {
        println!("收件 / to: {to} 「{title}」");
    }
    if !waits {
        if wait > 0 {
            println!("（认不出发件会话，不能在这里等回信。/ The sending session is unknown, so no reply can be awaited here.）");
        }
        return 0;
    }
    println!("正在等回信，最多 {wait} 秒… / Waiting for the reply, up to {wait} s…");
    let deadline = Instant::now() + Duration::from_secs(wait);
    // The card itself may wait for the user a while: the wait covers it.
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let polled = ask(
            "agent_inbox",
            json!({"caller":caller,"waitMs":if left.is_zero() {0} else {left.as_millis() as u64 + 5_000}}),
        );
        match polled {
            Ok(inbox) => {
                let messages = inbox["messages"].as_array().cloned().unwrap_or_default();
                if !messages.is_empty() {
                    let mut notice = false;
                    for message in &messages {
                        let words = message.as_str().unwrap_or_default();
                        notice |= words.starts_with("【SI Future HUD】")
                            || words.starts_with("[SI Future HUD]");
                        println!("\n{words}");
                    }
                    if !left.is_zero() {
                        let _ = ask("agent_inbox", json!({"caller":caller,"waitMs":0}));
                    }
                    return if notice { 3 } else { 0 };
                }
            }
            // The HUD restarted or quit: say so rather than wait in silence.
            Err(error) => return fail(error),
        }
        if left.is_zero() {
            println!("没等到回信。之后的回复只能进能收消息的会话。/ No reply came in time; a later reply reaches only sessions that can take messages.");
            return 2;
        }
        std::thread::sleep(Duration::from_millis(1500).min(left));
    }
}

/// The calling agent and its session: the nearest agent process above this
/// command decides, so a shell an agent runs in another agent's window counts
/// as the inner one.
fn caller() -> Result<Value, String> {
    let codex = std::env::var("CODEX_THREAD_ID")
        .ok()
        .filter(|id| !id.is_empty());
    let chengxiaobang = std::env::var("CHENGXIAOBANG_SESSION_ID")
        .ok()
        .filter(|id| !id.is_empty());
    let sessions = claude_sessions();
    for (pid, command) in ancestors() {
        if let Some(id) = sessions
            .as_deref()
            .and_then(|folder| claude_session(folder, pid))
        {
            return Ok(json!({"client":"claude","sessionId":id,"via":"cli"}));
        }
        let lower = command.to_lowercase();
        if let Some(id) = &codex {
            if lower.contains("codex") {
                return Ok(json!({"client":"codex","sessionId":id,"via":"cli"}));
            }
        }
        if let Some(id) = &chengxiaobang {
            if command.contains("程小帮") || lower.contains("chengxiaobang") {
                return Ok(json!({"client":"chengxiaobang","sessionId":id,"via":"cli"}));
            }
        }
    }
    // No agent process found above: what the shell says, which the card
    // shows as unverified unless the HUD lists that session.
    if let Some(id) = codex {
        return Ok(json!({"client":"codex","sessionId":id,"via":"cli"}));
    }
    if let Some(id) = chengxiaobang {
        return Ok(json!({"client":"chengxiaobang","sessionId":id,"via":"cli"}));
    }
    Err("认不出是哪个智能体在调用：这个命令要在 Claude Code、Codex 或程小帮的会话里运行。/ Can't tell which agent calls: run this command from a Claude Code, Codex or 程小帮 session.".into())
}

/// Claude Code's folder of running sessions, `<config>/sessions`.
fn claude_sessions() -> Option<PathBuf> {
    let config = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            agent_hud_runtime::home_directory()
                .ok()
                .map(|home| home.join(".claude"))
        })?;
    Some(config.join("sessions")).filter(|folder| folder.is_dir())
}

/// The session Claude Code's process `pid` runs now, from its own record
/// (which follows /clear and /resume).
fn claude_session(folder: &Path, pid: u32) -> Option<String> {
    let record: Value =
        serde_json::from_slice(&std::fs::read(folder.join(format!("{pid}.json"))).ok()?).ok()?;
    (record["pid"].as_u64() == Some(pid as u64))
        .then(|| record["sessionId"].as_str().map(str::to_owned))
        .flatten()
        .filter(|id| !id.is_empty())
}

/// This command's ancestor processes, nearest first: (pid, command).
fn ancestors() -> Vec<(u32, String)> {
    let mut found = Vec::new();
    #[cfg(unix)]
    {
        let mut pid = std::os::unix::process::parent_id();
        for _ in 0..32 {
            if pid <= 1 {
                break;
            }
            let Ok(output) = std::process::Command::new("/bin/ps")
                .args(["-o", "ppid=", "-o", "comm=", "-p", &pid.to_string()])
                .output()
            else {
                break;
            };
            let line = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            let Some((parent, command)) = line.split_once(char::is_whitespace) else {
                break;
            };
            found.push((pid, command.trim().to_owned()));
            match parent.trim().parse::<u32>() {
                Ok(parent) if parent != pid => pid = parent,
                _ => break,
            }
        }
    }
    found
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn reads_a_claude_session_record_only_for_its_own_process() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("42.json"),
            json!({"pid":42,"sessionId":"abc"}).to_string(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("43.json"),
            json!({"pid":99,"sessionId":"other"}).to_string(),
        )
        .unwrap();
        assert_eq!(claude_session(root.path(), 42).as_deref(), Some("abc"));
        assert_eq!(claude_session(root.path(), 43), None);
        assert_eq!(claude_session(root.path(), 44), None);
    }

    #[test]
    fn finds_this_process_ancestors() {
        let found = ancestors();
        assert!(!found.is_empty());
        assert_eq!(found[0].0, std::os::unix::process::parent_id());
    }
}
