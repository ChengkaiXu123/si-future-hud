//! Local build: Cursor's conversations and live facts, from the agent
//! transcripts it writes for each one
//! (`~/.cursor/projects/<project>/agent-transcripts/<id>/<id>.jsonl`).
//!
//! A line is a message (`{role, message: {content: [text | tool_use]}}`) or a
//! turn's end (`{type: "turn_ended", status}`). Lines carry no times, so the
//! file's last write stands for the latest. A conversation is listed under
//! the id Cursor's dashboard gives it (`cursor-account:<account>:<id>`), so its
//! usage, read from the dashboard, and these facts make one session.
use crate::transcript::live::{
    newer, Activity, Call, DesktopAsk, DesktopAskOption, LiveLog, Turn, Work,
};
use crate::transcript::records::{Page, Record};
use crate::Usage;
use base64::Engine;
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Conversations untouched for longer are left to the dashboard alone.
const RECENT_MS: i64 = 7 * 86_400_000;
/// An open turn quiet for longer is given up, as an abandoned turn is.
const OPEN_MS: i64 = 1_800_000;
/// How much of a transcript's end is read for its last turn.
const TAIL: u64 = 2 * 1024 * 1024;

/// Whether `path` is a conversation's transcript (not a subagent's).
pub fn accepts(path: &Path) -> bool {
    conversation(path).is_some()
}

/// The conversation a transcript belongs to: `agent-transcripts/<id>/<id>.jsonl`.
pub fn conversation(path: &Path) -> Option<&str> {
    let id = path.file_stem()?.to_str()?;
    let folder = path.parent()?;
    (path.extension()? == "jsonl"
        && folder.file_name()?.to_str()? == id
        && folder.parent()?.file_name()? == "agent-transcripts"
        && !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
    .then_some(id)
}

/// The transcript of conversation `id`, if Cursor wrote one.
pub fn transcript_for(home: &Path, id: &str) -> Option<PathBuf> {
    std::fs::read_dir(home.join(".cursor/projects"))
        .ok()?
        .flatten()
        .map(|project| {
            project
                .path()
                .join("agent-transcripts")
                .join(id)
                .join(format!("{id}.jsonl"))
        })
        .find(|path| path.is_file())
}

/// The account part of Cursor's dashboard ids: the signed-in user, hashed as
/// `cursor::sessions` does, from the login Cursor keeps (only its subject).
pub fn account(home: &Path) -> Option<String> {
    static CACHE: Mutex<Option<(Instant, Option<String>)>> = Mutex::new(None);
    let mut cache = CACHE.lock().ok()?;
    if let Some((at, account)) = cache.as_ref() {
        if at.elapsed() < Duration::from_secs(60) {
            return account.clone();
        }
    }
    let account = read_account(home);
    *cache = Some((Instant::now(), account.clone()));
    account
}

fn read_account(home: &Path) -> Option<String> {
    let database = home.join("Library/Application Support/Cursor/User/globalStorage/state.vscdb");
    let connection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let token: String = connection
        .query_row(
            "SELECT value FROM ItemTable WHERE key = 'cursorAuth/accessToken'",
            [],
            |row| row.get(0),
        )
        .ok()?;
    let claims = token.split('.').nth(1)?;
    let claims: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(claims.trim_end_matches('='))
            .ok()?,
    )
    .ok()?;
    let user = claims["sub"].as_str()?.rsplit('|').next()?;
    (!user.is_empty()).then(|| crate::grok::cache::hash(&[user]))
}

/// A conversation's model and project folder, as Cursor keeps them for it
/// (`composerData:<id>` in its state database), read in place.
fn composer(connection: &rusqlite::Connection, id: &str) -> (Option<String>, Option<String>) {
    connection
        .query_row(
            "SELECT json_extract(value, '$.modelConfig.modelName'), \
                    json_extract(value, '$.workspaceIdentifier.uri.fsPath') \
             FROM cursorDiskKV WHERE key = ?1",
            [format!("composerData:{id}")],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .map(|(model, folder)| {
            (
                model.filter(|m| !m.trim().is_empty() && m != "default"),
                folder.filter(|f| f.starts_with('/')),
            )
        })
        .unwrap_or((None, None))
}

fn conversation_facts(home: &Path, id: &str) -> (Option<String>, Option<String>) {
    let database = home.join("Library/Application Support/Cursor/User/globalStorage/state.vscdb");
    rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map(|connection| composer(&connection, id))
    .unwrap_or((None, None))
}

/// The session id a conversation is listed under.
pub fn session_id(account: Option<&str>, id: &str) -> String {
    match account {
        Some(account) => format!("cursor-account:{account}:{id}"),
        None => format!("cursor:{id}"),
    }
}

fn modified_ms(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64)
}

/// The words a user line asked, without Cursor's wrappers.
fn asked(text: &str) -> &str {
    let text = text
        .split_once("<user_query>")
        .map(|(_, rest)| rest.split("</user_query>").next().unwrap_or(rest))
        .unwrap_or(text);
    text.trim()
}

fn texts(line: &Value) -> impl Iterator<Item = &str> {
    line["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["type"] == "text")
        .filter_map(|item| item["text"].as_str())
}

/// The conversation's title from its first question, when it can be read.
fn first_question(path: &Path) -> Option<String> {
    let mut head = vec![0; 16 * 1024];
    let mut file = std::fs::File::open(path).ok()?;
    let read = file.read(&mut head).ok()?;
    head.truncate(read);
    let text = String::from_utf8_lossy(&head);
    let first: Value = serde_json::from_str(text.lines().next()?).ok()?;
    let words = asked(texts(&first).next()?);
    let line = words.lines().find(|line| !line.trim().is_empty())?.trim();
    Some(line.chars().take(60).collect())
}

/// The lines of the transcript's last turn and whether it ended (and how),
/// with where the turn starts in the file (its id while it runs).
fn last_turn(path: &Path) -> Option<(Vec<Value>, Option<String>, u64)> {
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let start = length.saturating_sub(TAIL);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    // Lines are cut on the file's own bytes, so each offset is where its line
    // starts in the file whatever the tail's first (possibly cut) character.
    let mut offset = start;
    let mut lines = Vec::new();
    for (index, raw) in bytes.split_inclusive(|byte| *byte == b'\n').enumerate() {
        let at = offset;
        offset += raw.len() as u64;
        // The first line of a tail may be cut.
        if index == 0 && start > 0 {
            continue;
        }
        if let Ok(line) = serde_json::from_slice::<Value>(raw.trim_ascii()) {
            lines.push((at, line));
        }
    }
    let ended = lines
        .last()
        .filter(|(_, line)| line["type"] == "turn_ended")
        .map(|(_, line)| line["status"].as_str().unwrap_or("success").to_owned());
    let body = if ended.is_some() {
        &lines[..lines.len() - 1]
    } else {
        &lines[..]
    };
    let begins = body
        .iter()
        .rposition(|(_, line)| line["type"] == "turn_ended")
        .map_or(0, |index| index + 1);
    let turn_at = body.get(begins).map_or(start, |(at, _)| *at);
    Some((
        body[begins..]
            .iter()
            .map(|(_, line)| line.clone())
            .collect(),
        ended,
        turn_at,
    ))
}

/// Where a conversation stands.
pub struct CursorRun {
    pub id: String,
    pub turn: String,
    pub at: i64,
    pub running: bool,
    pub outcome: Option<String>,
    pub answer: Option<String>,
    pub ask: Option<DesktopAsk>,
    pub tool: Option<String>,
}

fn work(name: &str) -> Option<Work> {
    match name {
        "Shell" | "AwaitShell" | "run_terminal_cmd" => Some(Work::Shell),
        "StrReplace" | "Write" | "ApplyPatch" | "Edit" | "MultiEdit" | "Delete"
        | "EditNotebook" | "edit_file" | "search_replace" => Some(Work::Edit),
        "Read" | "ReadFile" | "Grep" | "Glob" | "rg" | "LS" | "SemanticSearch"
        | "codebase_search" | "WebSearch" | "WebFetch" | "read_file" | "grep" | "list_dir"
        | "file_search" => Some(Work::Read),
        "Task" | "Subagent" => Some(Work::Agents),
        "AskQuestion" => Some(Work::Question),
        "TodoWrite" | "CreatePlan" | "GetDynamicTools" => None,
        _ => Some(Work::Command),
    }
}

/// Where conversation `id` stands, from its transcript at `path`.
pub fn stand(path: &Path, now: i64) -> Option<CursorRun> {
    let id = conversation(path)?.to_owned();
    let at = modified_ms(path)?;
    let (lines, ended, turn_at) = last_turn(path)?;
    let running = ended.is_none() && !lines.is_empty() && now - at < OPEN_MS;
    let last_tool = lines
        .iter()
        .filter(|line| line["role"] == "assistant")
        .flat_map(|line| line["message"]["content"].as_array().into_iter().flatten())
        .rfind(|item| item["type"] == "tool_use");
    // A tool is in flight while it is the turn's last word.
    let last_item = lines
        .iter()
        .rev()
        .find_map(|line| line["message"]["content"].as_array()?.last().cloned());
    let calling = running
        && last_item
            .as_ref()
            .is_some_and(|item| item["type"] == "tool_use");
    let ask = calling
        .then_some(last_item.as_ref())
        .flatten()
        .filter(|item| item["name"] == "AskQuestion")
        .and_then(|item| {
            let first = item["input"]["questions"].get(0)?;
            let question = first["prompt"].as_str()?.trim();
            (!question.is_empty()).then(|| DesktopAsk {
                id: first["id"].as_str().unwrap_or("question").to_owned(),
                response: None,
                kind: "window".into(),
                question: question.into(),
                recap: None,
                options: first["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|option| {
                        Some(DesktopAskOption {
                            label: option["label"].as_str()?.to_owned(),
                            message: None,
                        })
                    })
                    .collect(),
                at,
                offset: None,
            })
        });
    let answer = lines
        .iter()
        .rev()
        .filter(|line| line["role"] == "assistant")
        .flat_map(|line| texts(line).collect::<Vec<_>>().into_iter().rev())
        .map(str::trim)
        .find(|text| !text.is_empty())
        .map(str::to_owned);
    Some(CursorRun {
        id,
        turn: format!("o{turn_at}"),
        at,
        running,
        outcome: ended.map(|status| match status.as_str() {
            "success" | "completed" => "completed".to_owned(),
            "aborted" => "interrupted".to_owned(),
            other => other.to_owned(),
        }),
        answer,
        ask,
        tool: calling
            .then(|| {
                last_tool
                    .and_then(|item| item["name"].as_str())
                    .map(str::to_owned)
            })
            .flatten(),
    })
}

/// The conversation a transcript holds, as one record (no usage: the
/// dashboard has it), for transcripts written lately.
pub fn read(path: &Path, home: &Path, now: i64) -> Page {
    let empty = Page {
        records: vec![],
        cursor: None,
        watermark: None,
    };
    let (Some(id), Some(at)) = (conversation(path), modified_ms(path)) else {
        return empty;
    };
    if now - at >= RECENT_MS {
        return empty;
    }
    let fallback = first_question(path).or_else(|| {
        Some(format!(
            "Cursor · {}",
            id.chars().take(8).collect::<String>()
        ))
    });
    // What the completion card names: the model and the project folder.
    let (model, cwd) = conversation_facts(home, id);
    Page {
        records: vec![Record {
            id: session_id(account(home).as_deref(), id),
            title: None,
            title_fallback: fallback,
            cwd,
            model: model.unwrap_or_else(|| "Unknown".into()),
            key: format!("transcript:{id}"),
            at,
            usage: Usage::default(),
            parent: None,
            archived: false,
            updated: None,
            duration_ms: None,
        }],
        cursor: None,
        watermark: None,
    }
}

/// Lays a conversation's standing on its session's live log.
pub fn apply(live: &mut LiveLog, run: &CursorRun) {
    newer(&mut live.last_event, run.at);
    match live
        .turns
        .back_mut()
        .filter(|turn| turn.id.as_deref() == Some(run.turn.as_str()))
    {
        Some(turn) => {
            turn.observed = turn.observed.max(run.at);
            turn.running = run.running;
            turn.outcome = run.outcome.clone();
            if run.answer.is_some() && !run.running {
                turn.message = run.answer.clone();
            }
        }
        None => live.push(Turn {
            id: Some(run.turn.clone()),
            started: Some(run.at),
            observed: run.at,
            running: run.running,
            outcome: run.outcome.clone(),
            limited_until: None,
            message: (!run.running).then(|| run.answer.clone()).flatten(),
        }),
    }
    let activity = live.activity.get_or_insert_with(Activity::default);
    activity.calls.clear();
    activity.standing_question = None;
    activity.desktop_ask = None;
    activity.retrying = false;
    activity.compacting_since = None;
    if !run.running {
        activity.latest = None;
        activity.line = Some("answer".into());
        return;
    }
    if let Some(ask) = &run.ask {
        let call = Call {
            id: ask.id.clone(),
            work: Some(Work::Question),
            at: run.at,
            request: Some("question".into()),
        };
        activity.standing_question = Some(call.clone());
        activity.desktop_ask = Some(ask.clone());
        activity.open(call);
    } else if let Some(name) = &run.tool {
        activity.open(Call {
            id: format!("{}:{name}", run.turn),
            work: work(name),
            at: run.at,
            request: None,
        });
        activity.work = work(name);
        activity.tool_at = Some(run.at);
    }
    activity.line = Some(
        if activity.calls.is_empty() {
            "prompt"
        } else {
            "call"
        }
        .into(),
    );
}

/// For tests and callers that hold a time.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |at| at.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn transcript(lines: &[Value]) -> (PathBuf, PathBuf) {
        let home = std::env::temp_dir().join(format!(
            "cursor-live-{}-{}",
            std::process::id(),
            lines.len()
        ));
        let folder = home.join(".cursor/projects/p/agent-transcripts/abc-1");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("abc-1.jsonl");
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        std::fs::write(&path, text).unwrap();
        (home, path)
    }

    fn user(text: &str) -> Value {
        json!({"role":"user","message":{"content":[{"type":"text","text":format!("<timestamp>x</timestamp>\n<user_query>\n{text}\n</user_query>")}]}})
    }

    fn assistant(content: Value) -> Value {
        json!({"role":"assistant","message":{"content":content}})
    }

    #[test]
    fn a_conversation_names_its_model_and_folder() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value TEXT);")
            .unwrap();
        connection
            .execute(
                "INSERT INTO cursorDiskKV VALUES ('composerData:c-1', ?1)",
                [json!({"modelConfig":{"modelName":"claude-opus-5"},"workspaceIdentifier":{"uri":{"fsPath":"/Users/x/proj"}}}).to_string()],
            )
            .unwrap();
        let (model, folder) = composer(&connection, "c-1");
        assert_eq!(model.as_deref(), Some("claude-opus-5"));
        assert_eq!(folder.as_deref(), Some("/Users/x/proj"));
        assert_eq!(composer(&connection, "missing"), (None, None));
    }

    #[test]
    fn only_conversation_transcripts_are_accepted() {
        assert!(accepts(Path::new(
            "/h/.cursor/projects/p/agent-transcripts/abc-1/abc-1.jsonl"
        )));
        assert!(!accepts(Path::new(
            "/h/.cursor/projects/p/agent-transcripts/abc-1/subagents/x.jsonl"
        )));
        assert!(!accepts(Path::new(
            "/h/.cursor/projects/p/agent-transcripts/abc-1/other.jsonl"
        )));
    }

    #[test]
    fn an_ended_turn_has_its_answer_and_the_first_question_titles_it() {
        let (home, path) = transcript(&[
            user("Fix the build\nplease"),
            assistant(
                json!([{"type":"text","text":"Looking"},{"type":"tool_use","name":"Read","input":{}}]),
            ),
            assistant(json!([{"type":"text","text":"Fixed it."}])),
            json!({"type":"turn_ended","status":"success"}),
        ]);
        let run = stand(&path, now_ms()).unwrap();
        assert!(!run.running);
        assert_eq!(run.outcome.as_deref(), Some("completed"));
        assert_eq!(run.answer.as_deref(), Some("Fixed it."));
        assert_eq!(first_question(&path).as_deref(), Some("Fix the build"));
        let page = read(&path, &home, now_ms());
        assert_eq!(page.records.len(), 1);
        assert!(page.records[0].id.ends_with(":abc-1"));
        let mut live = LiveLog::default();
        apply(&mut live, &run);
        assert_eq!(
            live.turns.back().unwrap().message.as_deref(),
            Some("Fixed it.")
        );
        std::fs::remove_dir_all(home).ok();
    }

    #[test]
    fn an_open_turn_runs_its_last_tool_or_asks() {
        let (home, path) = transcript(&[
            user("one"),
            assistant(json!([{"type":"text","text":"ok"}])),
            json!({"type":"turn_ended","status":"success"}),
            user("two"),
            assistant(json!([{"type":"tool_use","name":"Shell","input":{"command":"ls"}}])),
        ]);
        let run = stand(&path, now_ms()).unwrap();
        assert!(run.running);
        assert_eq!(run.tool.as_deref(), Some("Shell"));
        let mut live = LiveLog::default();
        apply(&mut live, &run);
        assert!(live.turns.back().unwrap().running);
        assert!(live.activity.as_ref().unwrap().work == Some(Work::Shell));
        std::fs::remove_dir_all(home).ok();

        let (home, path) = transcript(&[
            user("pick"),
            assistant(
                json!([{"type":"tool_use","name":"AskQuestion","input":{"questions":[{"id":"q1","prompt":"Which one?","options":[{"id":"a","label":"A"},{"id":"b","label":"B"}]}]}}]),
            ),
            json!({"role":"assistant","message":{"content":[]}}),
        ]);
        let run = stand(&path, now_ms()).unwrap();
        let ask = run.ask.as_ref().unwrap();
        assert_eq!(ask.question, "Which one?");
        assert_eq!(ask.options.len(), 2);
        let mut live = LiveLog::default();
        apply(&mut live, &run);
        assert!(live.activity.as_ref().unwrap().standing_question.is_some());
        std::fs::remove_dir_all(home).ok();
    }
}
