//! 程小帮 SQLite session reader (`data/chengxiaobang.sqlite`).
//! Each row of `usage_cost_entries` is one model call; its prompt tokens
//! include the cached ones, and total = prompt + completion.
use crate::transcript::live::{
    newer, Activity, Call, DesktopAsk, DesktopAskOption, LiveLog, Turn, Work,
};
use crate::transcript::{
    metering, sql_column, sql_columns, sql_count, sql_model, sql_tables, sql_text,
};
use crate::{
    transcript::records::{Page, Record},
    Usage,
};
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn read(db: &Connection, cursor: &Value) -> rusqlite::Result<Page> {
    let usage = sql_columns(db, "usage_cost_entries")?;
    if ![
        "id",
        "session_id",
        "model",
        "prompt_tokens",
        "completion_tokens",
        "entry_created_at",
    ]
    .iter()
    .all(|key| usage.contains(*key))
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let s = sql_columns(db, "sessions")?;
    let join = s.contains("id");
    let session = |key: &str| {
        if join {
            sql_column(&s, "s", key, "NULL")
        } else {
            "NULL".into()
        }
    };
    let p = if join && s.contains("project_id") && sql_tables(db)?.contains("projects") {
        sql_columns(db, "projects")?
    } else {
        Default::default()
    };
    let projects = p.contains("id") && p.contains("path");
    let project_path = if projects { "p.path" } else { "NULL" };
    let gone = ["archived_at", "deleted_at"]
        .iter()
        .filter(|key| s.contains(**key))
        .map(|key| format!("s.{key} IS NOT NULL"))
        .chain(
            s.contains("hidden")
                .then(|| "COALESCE(s.hidden,0)<>0".to_owned()),
        )
        .collect::<Vec<_>>();
    let gone = if join && !gone.is_empty() {
        gone.join(" OR ")
    } else {
        "0".into()
    };
    let cached = sql_column(&usage, "u", "cached_prompt_tokens", "NULL");
    let recorded = sql_column(&usage, "u", "recorded_at", "NULL");
    let after = cursor["after"]
        .as_i64()
        .or_else(|| cursor.as_i64())
        .unwrap_or(0);
    let sql = format!(
        "SELECT u.rowid,u.id,u.session_id,u.model,u.entry_created_at,{recorded},u.prompt_tokens,u.completion_tokens,{cached},{},{},{project_path},{},({gone}) FROM usage_cost_entries u {} {} WHERE u.rowid>?1 ORDER BY u.rowid LIMIT 1000",
        session("title"),
        session("worktree_path"),
        session("subagent_parent_session_id"),
        if join { "LEFT JOIN sessions s ON s.id=u.session_id" } else { "" },
        if projects { "LEFT JOIN projects p ON p.id=s.project_id" } else { "" },
    );
    let mut statement = db.prepare(&sql)?;
    let mut rows = statement.query([after])?;
    let mut records = Vec::new();
    let mut last = after;
    let mut seen = 0;
    while let Some(row) = rows.next()? {
        seen += 1;
        last = row.get(0)?;
        let Some(raw) = sql_text(row, 2) else {
            continue;
        };
        let Some(at) = sql_text(row, 4)
            .and_then(|text| metering::date_ms(&text))
            .or_else(|| sql_text(row, 5).and_then(|text| metering::date_ms(&text)))
        else {
            continue;
        };
        let prompt = sql_count(row, 6)?;
        let output = sql_count(row, 7)?;
        let read = sql_count(row, 8)?;
        if read > prompt {
            continue;
        }
        records.push(Record {
            id: format!("chengxiaobang:{raw}"),
            title: sql_text(row, 9),
            title_fallback: None,
            cwd: sql_text(row, 10).or_else(|| sql_text(row, 11)),
            model: sql_model(sql_text(row, 3)),
            key: sql_text(row, 1).unwrap_or_else(|| format!("row-{last}")),
            at,
            usage: Usage {
                input: prompt - read,
                output,
                read,
                write: 0,
                reasoning: 0,
            },
            parent: sql_text(row, 12).map(|id| format!("chengxiaobang:{id}")),
            archived: row.get::<_, Option<i64>>(13)?.unwrap_or(0) != 0,
            updated: None,
            duration_ms: None,
        });
    }
    Ok(Page {
        records,
        cursor: (seen == 1000).then(|| json!({"after": last})),
        watermark: (seen < 1000).then(|| json!({"after": last})),
    })
}

/// One session's latest run, as 程小帮's own tables tell it: what the HUD
/// needs for a turn's lifecycle, its tools, what waits on the user and the
/// final answer (程小帮 has no turn events in its usage rows).
pub struct Run {
    /// The session, as `read` names it.
    pub session: String,
    pub id: String,
    pub status: String,
    pub started: i64,
    pub updated: i64,
    /// Calls still open: running, or held for an approval or an answer.
    pub open: Vec<OpenCall>,
    /// The latest call that finished, and when.
    pub last_tool: Option<(String, i64)>,
    /// The last thing the assistant said in this run, once it ended.
    pub answer: Option<String>,
    /// When the run's latest model call began, if it compacts the context.
    pub compacting: Option<i64>,
}

pub struct OpenCall {
    pub id: String,
    pub name: String,
    pub at: i64,
    /// Held for the user (`pending_approval`), as 程小帮's sidebar reads it,
    /// or a question Agent HUD's plugin holds for the HUD.
    pub waits: bool,
    /// The question is held for the HUD, so the HUD can answer it.
    pub held: bool,
    pub args: Option<Value>,
}

/// The work a tool name stands for on the HUD's bots.
pub fn work(name: &str) -> Option<Work> {
    match name {
        "Shell" | "Bash" => Some(Work::Shell),
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => Some(Work::Edit),
        "Read" | "Grep" | "Glob" | "LS" | "WebSearch" | "WebFetch" | "SessionSearch"
        | "ToolSearch" | "Memory" | "AnalyzeImage" | "Skill" => Some(Work::Read),
        "Task" | "Agent" => Some(Work::Agents),
        "AskUserQuestion" => Some(Work::Question),
        "TaskNotify" | "TodoWrite" | "ExitPlanMode" => None,
        _ => Some(Work::Command),
    }
}

/// What a held call asks of the user, as the HUD's requests name it.
pub fn request(name: &str) -> &'static str {
    match name {
        "AskUserQuestion" => "question",
        "ExitPlanMode" => "plan",
        _ => "permission",
    }
}

const RECENT_MS: i64 = 6 * 3_600_000;

/// Each session's latest run that is running or ended in the last hours.
pub fn runs(db: &Connection, now: i64) -> rusqlite::Result<Vec<Run>> {
    let tables = sql_tables(db)?;
    if !tables.contains("runs") {
        return Ok(vec![]);
    }
    let r = sql_columns(db, "runs")?;
    if !["id", "session_id", "status", "created_at", "updated_at"]
        .iter()
        .all(|key| r.contains(*key))
    {
        return Ok(vec![]);
    }
    let cutoff = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now - RECENT_MS)
        .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default();
    let mut statement = db.prepare(
        "SELECT r.session_id,r.id,r.status,r.created_at,r.updated_at FROM runs r JOIN (SELECT session_id,MAX(created_at) m FROM runs GROUP BY session_id) l ON l.session_id=r.session_id AND l.m=r.created_at WHERE r.status='running' OR r.updated_at>=?1",
    )?;
    let mut rows = statement.query([cutoff])?;
    let mut found = Vec::new();
    while let Some(row) = rows.next()? {
        let (Some(session), Some(id), Some(status)) =
            (sql_text(row, 0), sql_text(row, 1), sql_text(row, 2))
        else {
            continue;
        };
        let (Some(started), Some(updated)) = (date(row, 3), date(row, 4)) else {
            continue;
        };
        found.push(Run {
            session: format!("chengxiaobang:{session}"),
            id,
            status,
            started,
            updated,
            open: vec![],
            last_tool: None,
            answer: None,
            compacting: None,
        });
    }
    drop(rows);
    drop(statement);
    let calls = tables
        .contains("tool_calls")
        .then(|| sql_columns(db, "tool_calls"))
        .transpose()?
        .filter(|c| {
            ["id", "run_id", "name", "status"]
                .iter()
                .all(|key| c.contains(*key))
        });
    let messages = tables
        .contains("messages")
        .then(|| sql_columns(db, "messages"))
        .transpose()?
        .filter(|m| {
            ["session_id", "role", "content", "created_at"]
                .iter()
                .all(|key| m.contains(*key))
        });
    for run in &mut found {
        if let Some(c) = &calls {
            let pick = |keys: [&str; 2]| {
                let have = keys
                    .iter()
                    .filter(|key| c.contains(**key))
                    .map(|key| format!("tc.{key}"))
                    .collect::<Vec<_>>();
                match have.len() {
                    0 => "NULL".to_owned(),
                    1 => have[0].clone(),
                    _ => format!("COALESCE({})", have.join(",")),
                }
            };
            let started = pick(["started_at", "created_at"]);
            let finished = pick(["finished_at", "updated_at"]);
            let args = sql_column(c, "tc", "args_json", "NULL");
            let mut statement = db.prepare(&format!(
                "SELECT tc.id,tc.name,tc.status,{started},{finished},{args} FROM tool_calls tc WHERE tc.run_id=?1 ORDER BY {started}"
            ))?;
            let mut rows = statement.query([&run.id[..]])?;
            while let Some(row) = rows.next()? {
                let (Some(id), Some(name), Some(status)) =
                    (sql_text(row, 0), sql_text(row, 1), sql_text(row, 2))
                else {
                    continue;
                };
                let at = date(row, 3).unwrap_or(run.started);
                match status.as_str() {
                    "running" | "pending_approval" | "pending_smart_approval" => {
                        run.open.push(OpenCall {
                            // A question Agent HUD's plugin holds stays `running`
                            // (程小帮 defers its approval route while a mod has
                            // the call); its own window holds one `pending_approval`.
                            held: name == "AskUserQuestion" && status == "running",
                            waits: status == "pending_approval"
                                || (name == "AskUserQuestion" && status == "running"),
                            args: sql_text(row, 5)
                                .and_then(|text| serde_json::from_str(&text).ok()),
                            id,
                            name,
                            at,
                        })
                    }
                    _ => {
                        let done = date(row, 4).unwrap_or(at);
                        if run.last_tool.as_ref().is_none_or(|(_, last)| done >= *last) {
                            run.last_tool = Some((name, done));
                        }
                    }
                }
            }
        }
        if run.status == "running" && tables.contains("usage_cost_entries") {
            let u = sql_columns(db, "usage_cost_entries")?;
            if ["run_id", "purpose", "entry_created_at"]
                .iter()
                .all(|key| u.contains(*key))
            {
                run.compacting = db
                    .query_row(
                        "SELECT purpose,entry_created_at FROM usage_cost_entries WHERE run_id=?1 ORDER BY entry_created_at DESC LIMIT 1",
                        [&run.id[..]],
                        |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
                    )
                    .ok()
                    .filter(|(purpose, _)| purpose.as_deref() == Some("compaction"))
                    .and_then(|(_, at)| at.as_deref().and_then(metering::date_ms));
            }
        }
        if run.status != "running" && messages.is_some() {
            let mut statement = db.prepare(
                "SELECT content FROM messages WHERE session_id=?1 AND role='assistant' AND COALESCE(kind,'')='' AND COALESCE(content,'')<>'' AND created_at>=?2 AND created_at<=?3 ORDER BY created_at DESC LIMIT 1",
            )?;
            let session = run.session.trim_start_matches("chengxiaobang:").to_owned();
            let from = iso_text(run.started);
            let to = iso_text(run.updated + 1_000);
            run.answer = statement
                .query_row([session, from, to], |row| row.get::<_, Option<String>>(0))
                .ok()
                .flatten()
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty());
        }
    }
    Ok(found)
}

fn date(row: &rusqlite::Row<'_>, index: usize) -> Option<i64> {
    sql_text(row, index).and_then(|text| metering::date_ms(&text))
}

fn iso_text(at: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(at)
        .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

/// The question a waiting `AskUserQuestion` asks, as a card shows it: its
/// question, header and options. Kind `question` is held for the HUD and can
/// be answered there; `window` waits in 程小帮's own window, as does any call
/// asking more than one question (the plugin answers only single ones).
pub fn ask(call: &OpenCall) -> Option<DesktopAsk> {
    let questions = call.args.as_ref()?["questions"].as_array()?;
    let first = questions.first()?;
    let question = first["question"].as_str()?.trim();
    if question.is_empty() {
        return None;
    }
    Some(DesktopAsk {
        id: call.id.clone(),
        response: None,
        kind: if call.held && questions.len() == 1 {
            "question"
        } else {
            "window"
        }
        .into(),
        question: question.into(),
        recap: first["header"].as_str().map(str::to_owned),
        options: first["options"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|option| {
                Some(DesktopAskOption {
                    label: option["label"].as_str()?.to_owned(),
                    message: option["description"].as_str().map(str::to_owned),
                })
            })
            .collect(),
        at: call.at,
        offset: None,
    })
}

/// The card of a 程小帮 question (the shape of a Claude desktop question's).
pub fn ask_view(
    req_id: &str,
    session_id: &str,
    cwd: Option<&str>,
    ask: &DesktopAsk,
    unanswerable: Option<crate::replies::Unreachable>,
) -> Value {
    let options = ask
        .options
        .iter()
        .map(|opt| json!({"label": opt.label, "description": opt.message}))
        .collect::<Vec<_>>();
    let header = ask
        .recap
        .clone()
        .unwrap_or_else(|| crate::l10n::text("提问", "Question"));
    let question = json!({
        "id": ask.id,
        "question": ask.question,
        "header": header,
        "options": options,
        "multiSelect": false,
        "answerKey": ask.id,
    });
    let created_at = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ask.at)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();
    json!({
        "id": req_id,
        "title": ask.question,
        "source": super::AGENT.id,
        "vendor": super::AGENT.name,
        "toolName": "AskUserQuestion",
        "input": "",
        "detail": null,
        "summary": ask.question,
        "sessionId": session_id,
        "createdAt": created_at,
        "allowsAlways": false,
        "kind": "question",
        "cwd": cwd,
        "path": null,
        "removed": null,
        "added": null,
        "suggestions": [],
        "alwaysAllowRule": null,
        "questions": [question],
        "plan": null,
        "answersQuestion": unanswerable.is_none(),
        "answerNote": unanswerable.as_ref().map(|why| why.note.as_str()),
        "answerFix": unanswerable.and_then(|why| why.fix).map(|fix| fix.as_str()),
        "answersPlan": false,
    })
}

/// The words a card's answer gives: the user's own words, else the chosen
/// option's label (程小帮 records the label as the answer).
pub fn ask_words(view: &Value, decision: &Value) -> Option<String> {
    if decision["kind"] != "answer" {
        return None;
    }
    let question = view["questions"].get(0)?;
    let answer = decision["answers"].get(question["answerKey"].as_str()?)?;
    if let Some(own) = answer["custom"]
        .as_str()
        .map(str::trim)
        .filter(|own| !own.is_empty())
    {
        return Some(own.to_owned());
    }
    Some(answer["selected"].get(0)?.as_str()?.to_owned())
}

/// Lays a run's facts on a session's live log: its turn (running or ended,
/// with its outcome and answer), the calls it has open, the latest tool it
/// used, and the call that waits on the user as the standing question.
pub fn apply(live: &mut LiveLog, run: &Run) {
    let running = run.status == "running";
    let outcome = match run.status.as_str() {
        "running" => None,
        "completed" => Some("completed"),
        "aborted" | "interrupted" => Some("interrupted"),
        other => Some(other),
    };
    let observed = run
        .open
        .iter()
        .map(|call| call.at)
        .chain(run.last_tool.as_ref().map(|(_, at)| *at))
        .chain([run.updated])
        .max()
        .unwrap_or(run.updated);
    newer(&mut live.last_event, observed);
    match live
        .turns
        .back_mut()
        .filter(|turn| turn.id.as_deref() == Some(run.id.as_str()))
    {
        Some(turn) => {
            turn.observed = turn.observed.max(observed);
            turn.running = running;
            turn.outcome = outcome.map(str::to_owned);
            if run.answer.is_some() {
                turn.message = run.answer.clone();
            }
        }
        None => live.push(Turn {
            id: Some(run.id.clone()),
            started: Some(run.started),
            observed,
            running,
            outcome: outcome.map(str::to_owned),
            limited_until: None,
            message: run.answer.clone(),
        }),
    }
    let activity = live.activity.get_or_insert_with(Activity::default);
    activity.calls.clear();
    activity.standing_question = None;
    activity.desktop_ask = None;
    activity.retrying = false;
    activity.compacting_since = if running { run.compacting } else { None };
    if let Some((name, at)) = &run.last_tool {
        activity.work = work(name);
        activity.tool_at = Some(*at);
    }
    if !running {
        activity.latest = None;
        activity.line = Some("answer".into());
        return;
    }
    for call in &run.open {
        let opened = Call {
            id: call.id.clone(),
            work: work(&call.name),
            at: call.at,
            request: call.waits.then(|| request(&call.name).to_owned()),
        };
        if call.waits {
            activity.standing_question = Some(opened.clone());
            if call.name == "AskUserQuestion" {
                activity.desktop_ask = ask(call);
            }
        }
        activity.open(opened);
    }
    activity.line = Some(
        if run.open.is_empty() {
            "prompt"
        } else {
            "call"
        }
        .into(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_calls_with_cached_prompt_split_and_session_facts() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE projects(id TEXT,path TEXT);
             CREATE TABLE sessions(id TEXT,project_id TEXT,title TEXT,worktree_path TEXT,hidden INTEGER,archived_at TEXT,deleted_at TEXT,subagent_parent_session_id TEXT);
             CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,total_tokens INTEGER,entry_created_at TEXT,recorded_at TEXT);
             INSERT INTO projects VALUES('p1','/work/app');
             INSERT INTO sessions VALUES('s1','p1','Fix the build',NULL,0,NULL,NULL,NULL);
             INSERT INTO sessions VALUES('s2','p1','Helper','/work/tree',1,NULL,NULL,'s1');
             INSERT INTO usage_cost_entries VALUES('u1','s1','auto(free)',1000,50,800,1050,'2026-10-26T05:15:06.936Z','2026-10-26T05:15:08.710Z');
             INSERT INTO usage_cost_entries VALUES('u2','s2','auto(free)',100,10,NULL,110,'2026-10-26T05:16:00.000Z',NULL);
             INSERT INTO usage_cost_entries VALUES('u3','s1','auto(free)',10,1,20,11,'2026-10-26T05:17:00.000Z',NULL);",
        )
        .unwrap();
        let page = read(&db, &Value::Null).unwrap();
        // u3 claims more cached than prompt tokens and is skipped.
        assert_eq!(page.records.len(), 2);
        let first = &page.records[0];
        assert_eq!(first.id, "chengxiaobang:s1");
        assert_eq!(first.key, "u1");
        assert_eq!(first.title.as_deref(), Some("Fix the build"));
        assert_eq!(first.cwd.as_deref(), Some("/work/app"));
        assert_eq!(first.at, 1_792_991_706_936);
        assert_eq!(
            (first.usage.input, first.usage.read, first.usage.output),
            (200, 800, 50)
        );
        assert!(first.parent.is_none() && !first.archived);
        let second = &page.records[1];
        assert_eq!(second.cwd.as_deref(), Some("/work/tree"));
        assert_eq!(second.parent.as_deref(), Some("chengxiaobang:s1"));
        assert!(second.archived);
        assert_eq!((second.usage.input, second.usage.read), (100, 0));
        assert!(page.cursor.is_none());
        assert_eq!(page.watermark, Some(json!({"after": 3})));
        let next = read(&db, &json!({"after": 3})).unwrap();
        assert!(next.records.is_empty());
    }

    #[test]
    fn refuses_a_table_without_its_usage_columns() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE usage_cost_entries(id TEXT); CREATE TABLE sessions(id TEXT);",
        )
        .unwrap();
        assert!(read(&db, &Value::Null).is_err());
    }

    #[test]
    fn runs_lay_turns_tools_questions_and_answers_on_the_live_log() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE runs(id TEXT,session_id TEXT,status TEXT,created_at TEXT,updated_at TEXT);
             CREATE TABLE tool_calls(id TEXT,run_id TEXT,name TEXT,status TEXT,args_json TEXT,started_at TEXT,finished_at TEXT,created_at TEXT,updated_at TEXT);
             CREATE TABLE messages(id TEXT,session_id TEXT,role TEXT,kind TEXT,content TEXT,created_at TEXT);
             INSERT INTO runs VALUES('old','s1','completed','2026-10-26T05:00:00.000Z','2026-10-26T05:01:00.000Z');
             INSERT INTO runs VALUES('r1','s1','running','2026-10-26T05:15:00.000Z','2026-10-26T05:15:06.000Z');
             INSERT INTO runs VALUES('r2','s2','completed','2026-10-26T05:10:00.000Z','2026-10-26T05:12:00.000Z');
             INSERT INTO tool_calls VALUES('c1','r1','Edit','completed',NULL,'2026-10-26T05:15:01.000Z','2026-10-26T05:15:02.000Z',NULL,NULL);
             INSERT INTO tool_calls VALUES('c2','r1','AskUserQuestion','pending_approval','{\"questions\":[{\"question\":\"Which day?\",\"header\":\"Day\",\"options\":[{\"label\":\"Mon\",\"description\":\"Monday\"},{\"label\":\"Tue\"}]}]}','2026-10-26T05:15:05.000Z',NULL,NULL,NULL);
             INSERT INTO messages VALUES('m1','s2','assistant',NULL,'Booked it.','2026-10-26T05:11:59.000Z');
             INSERT INTO messages VALUES('m2','s2','assistant',NULL,'','2026-10-26T05:11:59.500Z');",
        )
        .unwrap();
        let now = metering::date_ms("2026-10-26T05:16:00.000Z").unwrap();
        let runs = runs(&db, now).unwrap();
        assert_eq!(runs.len(), 2);
        let r1 = runs.iter().find(|r| r.id == "r1").unwrap();
        assert_eq!(r1.session, "chengxiaobang:s1");
        assert_eq!(r1.open.len(), 1);
        assert!(r1.open[0].waits);
        assert_eq!(r1.last_tool.as_ref().unwrap().0, "Edit");
        let mut live = LiveLog::default();
        apply(&mut live, r1);
        assert_eq!(live.session_state(now, "chengxiaobang", None), "waiting");
        let facts = live
            .facts("chengxiaobang:s1", "chengxiaobang", now, None)
            .unwrap();
        assert_eq!(facts["requests"][0]["kind"], "question");
        assert_eq!(facts["activity"]["openCall"]["work"], "question");
        let ask = live
            .activity
            .as_ref()
            .unwrap()
            .desktop_ask
            .as_ref()
            .unwrap();
        assert_eq!(ask.question, "Which day?");
        assert_eq!(ask.options.len(), 2);

        let r2 = runs.iter().find(|r| r.id == "r2").unwrap();
        assert_eq!(r2.answer.as_deref(), Some("Booked it."));
        let mut done = LiveLog::default();
        apply(&mut done, r2);
        let facts = done
            .facts("chengxiaobang:s2", "chengxiaobang", now, None)
            .unwrap();
        assert_eq!(facts["outcome"], "completed");
        assert_eq!(facts["message"], "Booked it.");
        assert!(facts["endedAt"].is_string());
        assert_eq!(done.session_state(now, "chengxiaobang", None), "idle");

        // The question answered and the run working on: a tool is open.
        db.execute_batch(
            "UPDATE tool_calls SET status='completed',finished_at='2026-10-26T05:15:30.000Z' WHERE id='c2';
             INSERT INTO tool_calls VALUES('c3','r1','Shell','running',NULL,'2026-10-26T05:15:40.000Z',NULL,NULL,NULL);",
        )
        .unwrap();
        let runs_after = super::runs(&db, now).unwrap();
        apply(&mut live, runs_after.iter().find(|r| r.id == "r1").unwrap());
        assert_eq!(live.session_state(now, "chengxiaobang", None), "running");
        let facts = live
            .facts("chengxiaobang:s1", "chengxiaobang", now, None)
            .unwrap();
        assert_eq!(facts["activity"]["openCall"]["work"], "shell");
        assert!(facts["requests"].as_array().unwrap().is_empty());
        assert_eq!(live.turns.len(), 1);
    }

    #[test]
    fn a_compaction_call_marks_the_run_compacting() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE runs(id TEXT,session_id TEXT,status TEXT,created_at TEXT,updated_at TEXT);
             CREATE TABLE usage_cost_entries(id TEXT,run_id TEXT,session_id TEXT,purpose TEXT,entry_created_at TEXT);
             INSERT INTO runs VALUES('r1','s1','running','2026-10-26T05:15:00.000Z','2026-10-26T05:15:30.000Z');
             INSERT INTO usage_cost_entries VALUES('u1','r1','s1','assistant','2026-10-26T05:15:10.000Z');
             INSERT INTO usage_cost_entries VALUES('u2','r1','s1','compaction','2026-10-26T05:15:20.000Z');",
        )
        .unwrap();
        let now = metering::date_ms("2026-10-26T05:15:40.000Z").unwrap();
        let runs = runs(&db, now).unwrap();
        let mut live = LiveLog::default();
        apply(&mut live, &runs[0]);
        let facts = live
            .facts("chengxiaobang:s1", "chengxiaobang", now, None)
            .unwrap();
        assert!(facts["activity"]["compactingSince"]
            .as_str()
            .is_some_and(|at| at.starts_with("2026-10-26T05:15:20")));
        db.execute(
            "INSERT INTO usage_cost_entries VALUES('u3','r1','s1','assistant','2026-10-26T05:15:35.000Z')",
            [],
        )
        .unwrap();
        apply(&mut live, &super::runs(&db, now).unwrap()[0]);
        let facts = live
            .facts("chengxiaobang:s1", "chengxiaobang", now, None)
            .unwrap();
        assert!(facts["activity"]["compactingSince"].is_null());
    }
}
