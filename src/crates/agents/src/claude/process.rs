//! How a Claude Code process is told apart: its command-line grammar, the
//! apps that host it and the entrypoint it reports, for the hooks' origin
//! and for headless runs.
use crate::process::{Caller, Origin};
use serde_json::json;
use std::path::Path;

/// The bundle of Claude Code's own native app.
const CODE_BUNDLE: &str = "com.anthropic.claude-code";
/// The bundle of the Claude desktop app, which hosts Claude Code sessions.
const DESKTOP_BUNDLE: &str = "com.anthropic.claudefordesktop";

/// The arguments of the Claude Code process that runs the hook: the nearest
/// caller, when it is the installed program, Claude Code's app, or node
/// running the installed script.
fn arguments(caller: &Caller) -> Option<Vec<String>> {
    let process = caller.ancestors.first()?;
    let path = process.resolved.as_ref()?;
    if (caller.installed)(path) || process.bundle.as_deref() == Some(CODE_BUNDLE) {
        return Some(process.arguments.clone());
    }
    if crate::process::native_program(path, "node")
        && process
            .arguments
            .get(1)
            .is_some_and(|script| (caller.installed)(Path::new(script)))
    {
        return Some(process.arguments[1..].to_vec());
    }
    None
}

/// Options that take the next argument as their value.
fn takes_value(option: &str) -> bool {
    matches!(
        option,
        "--agent"
            | "--agents"
            | "--append-system-prompt"
            | "--append-system-prompt-file"
            | "--effort"
            | "--fallback-model"
            | "--json-schema"
            | "--mcp-config"
            | "--model"
            | "--permission-mode"
            | "--plugin-dir"
            | "--plugin-url"
            | "--session-id"
            | "--settings"
            | "--setting-sources"
            | "--system-prompt"
            | "--system-prompt-file"
            | "--max-turns"
            | "--max-budget-usd"
            | "--name"
            | "-n"
            | "--input-format"
            | "--output-format"
            | "--permission-prompt-tool"
            | "--permission-prompts"
    )
}

/// Whether the arguments start an interactive terminal session.
fn interactive(arguments: &[String]) -> bool {
    let mut i = 1;
    while i < arguments.len() {
        let argument = &arguments[i];
        if argument == "--" {
            break;
        }
        let option = argument.split('=').next().unwrap_or("");
        if matches!(
            option,
            "-p" | "--print"
                | "--background"
                | "--bg"
                | "--init-only"
                | "--cloud"
                | "--remote"
                | "--sdk-url"
                | "--input-format"
                | "--output-format"
        ) {
            return false;
        }
        i += if !argument.contains('=') && takes_value(option) {
            2
        } else {
            1
        };
    }
    true
}

/// Where a Claude Code session started: the terminal, the Claude desktop
/// app or Cowork, from the entrypoint Claude Code reports and the app it
/// runs under.
pub fn origin(caller: &Caller, session: &str) -> Origin {
    let Some(arguments) = arguments(caller) else {
        return Origin::unsupported();
    };
    let entrypoint = caller.env("CLAUDE_CODE_ENTRYPOINT");
    let mut target = serde_json::Value::Null;
    let host = match entrypoint {
        Some("cli" | "sdk-cli") => "cli",
        Some("claude-desktop" | "claude-desktop-3p") if caller.under_bundle(DESKTOP_BUNDLE) => {
            if !session.is_empty() {
                target = json!({"kind":"claude-desktop","id":session});
            }
            "desktop"
        }
        Some("local-agent") if caller.under_bundle(DESKTOP_BUNDLE) => {
            if !session.is_empty() {
                target = json!({"kind":"claude-cowork","id":session});
            }
            "cowork"
        }
        _ => "unsupported",
    };
    Origin {
        host,
        target,
        interactive: entrypoint == Some("cli") && interactive(&arguments),
        ask_channel: false,
    }
}

/// Whether the hook comes from a headless run (`-p` and the like) nobody
/// answers: no permission prompt tool, or prompts turned off.
pub fn unattended(caller: &Caller) -> bool {
    if caller.env("CLAUDE_CODE_SESSION_ATTENDED") != Some("0")
        || caller
            .env("CLAUDE_CODE_SESSION_KIND")
            .is_some_and(|s| !s.is_empty())
    {
        return false;
    }
    let Some(arguments) = arguments(caller) else {
        return false;
    };
    let (mut printing, mut prompt, mut none) = (false, false, false);
    let mut i = 1;
    while i < arguments.len() {
        let argument = &arguments[i];
        if argument == "--" {
            break;
        }
        let (option, inline) = argument
            .split_once('=')
            .map_or((argument.as_str(), None), |(a, b)| (a, Some(b)));
        match option {
            "-p" | "--print" | "--input-format" | "--output-format" => printing = true,
            "--permission-prompt-tool" => prompt = true,
            "--permission-prompts" => {
                none = inline.or_else(|| arguments.get(i + 1).map(String::as_str)) == Some("none")
            }
            _ => {}
        }
        i += if inline.is_none() && takes_value(option) {
            2
        } else {
            1
        };
    }
    printing && (none || !prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Ancestor;
    use std::collections::HashMap;

    const PROGRAM: &str = "/opt/claude/bin/claude";

    fn run(arguments: &[&str], env: &[(&str, &str)], bundles: &[&str]) -> (Origin, bool) {
        let mut ancestors = vec![Ancestor {
            executable: PROGRAM.into(),
            resolved: Some(PROGRAM.into()),
            arguments: arguments.iter().map(|s| (*s).to_owned()).collect(),
            bundle: None,
        }];
        for bundle in bundles {
            ancestors.push(Ancestor {
                bundle: Some((*bundle).into()),
                ..Ancestor::default()
            });
        }
        let env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect();
        let installed = |path: &Path| path == Path::new(PROGRAM);
        let caller = Caller {
            ancestors: &ancestors,
            env: &env,
            installed: &installed,
        };
        (origin(&caller, "s-1"), unattended(&caller))
    }

    #[test]
    fn the_entrypoint_and_the_hosting_app_name_the_host() {
        let (cli, _) = run(&["claude"], &[("CLAUDE_CODE_ENTRYPOINT", "cli")], &[]);
        assert_eq!((cli.host, cli.interactive), ("cli", true));
        let (printed, _) = run(
            &["claude", "-p", "hi"],
            &[("CLAUDE_CODE_ENTRYPOINT", "cli")],
            &[],
        );
        assert!(!printed.interactive);
        let (desktop, _) = run(
            &["claude"],
            &[("CLAUDE_CODE_ENTRYPOINT", "claude-desktop")],
            &[DESKTOP_BUNDLE],
        );
        assert_eq!(desktop.host, "desktop");
        assert_eq!(desktop.target, json!({"kind":"claude-desktop","id":"s-1"}));
        let (claimed, _) = run(
            &["claude"],
            &[("CLAUDE_CODE_ENTRYPOINT", "claude-desktop")],
            &[],
        );
        assert_eq!(
            claimed.host, "unsupported",
            "a claim without the app is not trusted"
        );
        let (cowork, _) = run(
            &["claude"],
            &[("CLAUDE_CODE_ENTRYPOINT", "local-agent")],
            &[DESKTOP_BUNDLE],
        );
        assert_eq!(cowork.target["kind"], "claude-cowork");
    }

    #[test]
    fn a_process_that_is_not_claude_code_has_no_origin() {
        let ancestors = [Ancestor {
            executable: "/usr/bin/python3".into(),
            resolved: Some("/usr/bin/python3".into()),
            arguments: vec!["python3".into()],
            bundle: None,
        }];
        let env = HashMap::from([("CLAUDE_CODE_ENTRYPOINT".to_owned(), "cli".to_owned())]);
        let caller = Caller {
            ancestors: &ancestors,
            env: &env,
            installed: &|_: &Path| false,
        };
        assert_eq!(origin(&caller, "s"), Origin::unsupported());
    }

    #[test]
    fn windows_node_uses_the_installed_script_identity() {
        let script = Path::new(r"C:\Users\me\AppData\Roaming\npm\claude.js");
        let ancestors = [Ancestor {
            executable: r"C:\Program Files\nodejs\node.exe".into(),
            resolved: Some(r"C:\Program Files\nodejs\node.exe".into()),
            arguments: vec![
                "node.exe".into(),
                script.to_str().unwrap().into(),
                "-p".into(),
                "hello".into(),
            ],
            bundle: None,
        }];
        let env = HashMap::from([
            ("CLAUDE_CODE_ENTRYPOINT".into(), "cli".into()),
            ("CLAUDE_CODE_SESSION_ATTENDED".into(), "0".into()),
        ]);
        let caller = Caller {
            ancestors: &ancestors,
            env: &env,
            installed: &|path| path == script,
        };
        assert_eq!(origin(&caller, "s").host, "cli");
        assert!(!origin(&caller, "s").interactive);
        assert!(unattended(&caller));
    }

    #[test]
    fn headless_runs_without_a_prompt_tool_are_unattended() {
        let headless = [("CLAUDE_CODE_SESSION_ATTENDED", "0")];
        assert!(run(&["claude", "-p", "go"], &headless, &[]).1);
        assert!(
            !run(
                &["claude", "-p", "go", "--permission-prompt-tool", "mcp"],
                &headless,
                &[]
            )
            .1
        );
        assert!(
            run(
                &[
                    "claude",
                    "-p",
                    "go",
                    "--permission-prompt-tool",
                    "mcp",
                    "--permission-prompts",
                    "none"
                ],
                &headless,
                &[]
            )
            .1
        );
        assert!(
            !run(&["claude", "-p", "go"], &[], &[]).1,
            "attended unless it says 0"
        );
        assert!(
            !run(&["claude", "--model", "-p"], &headless, &[]).1,
            "-p here is the model"
        );
    }
}
