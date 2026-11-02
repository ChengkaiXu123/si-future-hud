use super::*;
use crate::SourceRoot;
use rusqlite::{params, Connection};

fn core(root: &Path) -> AgentHudCore {
    AgentHudCore::new(
        vec![SourceRoot {
            path: root.join("opencode.db"),
            provider: "opencode",
        }],
        root.join("hud"),
    )
}

fn source(root: &Path) -> Connection {
    let db = Connection::open(root.join("opencode.db")).unwrap();
    db.execute_batch(
        "CREATE TABLE session(id TEXT PRIMARY KEY,title TEXT,directory TEXT,time_archived INTEGER,parent_id TEXT);
         CREATE TABLE message(id TEXT PRIMARY KEY,session_id TEXT,data TEXT);
         INSERT INTO session VALUES('s','old title','/repo',NULL,NULL);",
    )
    .unwrap();
    db
}

fn message(input: u64, output: u64) -> String {
    json!({
        "role":"assistant",
        "time":{"created":1_792_470_000_000i64,"completed":1_792_470_001_000i64},
        "modelID":"gpt-5","providerID":"openai",
        "tokens":{"input":input,"output":output}
    })
    .to_string()
}

fn finish(core: &mut AgentHudCore, first: Value) -> Value {
    let mut snapshot = first;
    for _ in 0..10 {
        if core.reads.pending.is_empty() {
            return snapshot;
        }
        snapshot = core.request("snapshot", json!({})).unwrap();
    }
    panic!("OpenCode fixture did not finish indexing");
}

fn refresh(core: &mut AgentHudCore) -> Value {
    let first = core.request("refresh", json!({})).unwrap();
    finish(core, first)
}

#[test]
fn opencode_reloads_updated_messages_and_session_metadata() {
    let root = tempfile::tempdir().unwrap();
    let db = source(root.path());
    db.execute("INSERT INTO message VALUES('m','s',?1)", [message(10, 1)])
        .unwrap();
    let mut backend = core(root.path());
    assert_eq!(refresh(&mut backend)["totals"]["total"], 11);
    drop(backend);

    // The persisted cursor must also reread an UPDATE after an app restart.
    db.execute(
        "UPDATE message SET data=?1 WHERE id='m'",
        [message(100, 20)],
    )
    .unwrap();
    db.execute("UPDATE session SET title='new title' WHERE id='s'", [])
        .unwrap();
    let mut backend = core(root.path());
    let snapshot = refresh(&mut backend);
    assert_eq!(snapshot["totals"]["total"], 120);
    let sessions = backend.request("sessions", json!({"scope":"all"})).unwrap();
    assert_eq!(sessions["items"][0]["title"], "new title");
    let detail = backend
        .request("session_detail", json!({"id":"opencode:s"}))
        .unwrap();
    assert_eq!(detail["usage"]["total"], 120);
    assert_eq!(detail["calls"].as_array().unwrap().len(), 1);

    db.execute("UPDATE session SET time_archived=1 WHERE id='s'", [])
        .unwrap();
    assert_eq!(refresh(&mut backend)["totals"]["total"], 120);
    let sessions = backend.request("sessions", json!({"scope":"all"})).unwrap();
    assert_eq!(sessions["total"], 0);
}

#[test]
fn opencode_new_table_replaces_existing_legacy_events_for_its_sessions() {
    let root = tempfile::tempdir().unwrap();
    let db = source(root.path());
    db.execute_batch("INSERT INTO session VALUES('other','other','/other',NULL,NULL)")
        .unwrap();
    db.execute("INSERT INTO message VALUES('old','s',?1)", [message(10, 1)])
        .unwrap();
    db.execute(
        "INSERT INTO message VALUES('other','other',?1)",
        [message(6, 1)],
    )
    .unwrap();
    let mut backend = core(root.path());
    assert_eq!(refresh(&mut backend)["totals"]["total"], 18);

    // The new table names the same reply with a different ID. It owns s only;
    // the other session still belongs to message.
    db.execute_batch(
        "CREATE TABLE session_message(id TEXT PRIMARY KEY,session_id TEXT,type TEXT,data TEXT)",
    )
    .unwrap();
    db.execute(
        "INSERT INTO session_message VALUES('new','s','assistant',?1)",
        [message(10, 1)],
    )
    .unwrap();
    assert_eq!(refresh(&mut backend)["totals"]["total"], 18);
    let detail = backend
        .request("session_detail", json!({"id":"opencode:s"}))
        .unwrap();
    assert_eq!(detail["usage"]["total"], 11);
    assert_eq!(detail["calls"].as_array().unwrap().len(), 1);

    db.execute(
        "UPDATE session_message SET data=?1 WHERE id='new'",
        [message(100, 20)],
    )
    .unwrap();
    assert_eq!(refresh(&mut backend)["totals"]["total"], 127);
}

#[test]
fn opencode_rebuilds_an_old_reader_checkpoint_without_a_source_change() {
    let root = tempfile::tempdir().unwrap();
    let db = source(root.path());
    db.execute_batch(
        "CREATE TABLE session_message(id TEXT PRIMARY KEY,session_id TEXT,type TEXT,data TEXT)",
    )
    .unwrap();
    db.execute("INSERT INTO message VALUES('old','s',?1)", [message(10, 1)])
        .unwrap();
    db.execute(
        "INSERT INTO session_message VALUES('new','s','assistant',?1)",
        [message(10, 1)],
    )
    .unwrap();
    let mut backend = core(root.path());
    assert_eq!(refresh(&mut backend)["totals"]["total"], 11);

    // Simulate the previous reader's completed checkpoint and duplicate ledger.
    // The source file itself stays untouched during the upgrade.
    let path = root.path().join("opencode.db");
    let vpath = virtual_path(&path, "opencode:s");
    let (mut physical, _) = backend.load_parser(&path).unwrap();
    let (mut session, _) = backend.load_parser(&vpath).unwrap();
    physical.reader_version = 1;
    physical.source_state =
        json!({"generation":"old-reader","cursor":null,"watermark":{"stage":0,"s0":1,"s1":1}});
    session.source_state["generation"] = json!("old-reader");
    session.count_source(
        "opencode:old".into(),
        "gpt-5#openai",
        1_792_470_000_000,
        Usage {
            input: 10,
            output: 1,
            ..Usage::default()
        },
        false,
    );
    let updated = backend.next_timestamp_ms();
    let ledger = backend.database.as_mut().unwrap();
    let transaction = ledger.transaction().unwrap();
    for log in [&physical, &session] {
        let path = log.path.to_string_lossy();
        checkpoints::upsert_log(
            &transaction,
            &path,
            &serde_json::to_string(log).unwrap(),
            &serde_json::to_string(&LogHead::from_log(log)).unwrap(),
            updated,
        )
        .unwrap();
        metering::write(
            &transaction,
            &path,
            "opencode",
            &log.meter_events.values().cloned().collect::<Vec<_>>(),
        )
        .unwrap();
    }
    transaction.commit().unwrap();
    drop(backend);

    let mut backend = core(root.path());
    assert_eq!(refresh(&mut backend)["totals"]["total"], 11);
    let detail = backend
        .request("session_detail", json!({"id":"opencode:s"}))
        .unwrap();
    assert_eq!(detail["callsTotal"], 1);
    assert_eq!(backend.logs[&path].reader_version, 2);
}

#[test]
fn opencode_generation_paging_resumes_and_unchanged_snapshots_do_no_source_writes() {
    let root = tempfile::tempdir().unwrap();
    let mut db = source(root.path());
    let transaction = db.transaction().unwrap();
    for index in 0..1001 {
        transaction
            .execute(
                "INSERT INTO message VALUES(?1,'s',?2)",
                params![format!("m{index}"), message(1, 1)],
            )
            .unwrap();
    }
    transaction.commit().unwrap();
    let mut backend = core(root.path());
    let first = backend.request("snapshot", json!({})).unwrap();
    assert_eq!(first["totals"]["total"], 2000);
    assert_eq!(backend.reads.pending.len(), 1);
    drop(backend);

    let mut backend = core(root.path());
    assert_eq!(refresh(&mut backend)["totals"]["total"], 2002);
    db.execute("UPDATE message SET data=?1 WHERE id='m0'", [message(3, 1)])
        .unwrap();
    assert_eq!(refresh(&mut backend)["totals"]["total"], 2004);
    let committed = backend.last_synced_at_ms;
    assert_eq!(
        backend.request("snapshot", json!({})).unwrap()["totals"]["total"],
        2004
    );
    assert_eq!(backend.last_synced_at_ms, committed);
}

/// Each call's (output, duration) in the ledger, by output.
fn timed_calls(backend: &AgentHudCore) -> Vec<(i64, Option<i64>)> {
    let db = backend.database.as_ref().unwrap();
    let mut statement = db
        .prepare("SELECT output,duration_ms FROM usage_meter_event ORDER BY output")
        .unwrap();
    let rows = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    rows.map(Result::unwrap).collect()
}

#[test]
fn opencode_times_a_reply_only_when_it_called_no_tools() {
    let root = tempfile::tempdir().unwrap();
    let db = source(root.path());
    let created = chrono::Utc::now().timestamp_millis() - 60_000;
    let reply = |finish: &str, output: u64| {
        json!({
            "role":"assistant",
            "time":{"created":created,"completed":created + 4_000},
            "finish":finish,
            "modelID":"gpt-5","providerID":"openai",
            "tokens":{"input":10,"output":output}
        })
        .to_string()
    };
    db.execute(
        "INSERT INTO message VALUES('a','s',?1),('b','s',?2)",
        [reply("stop", 100), reply("tool-calls", 50)],
    )
    .unwrap();
    let mut backend = core(root.path());
    refresh(&mut backend);
    assert_eq!(timed_calls(&backend), [(50, None), (100, Some(4_000))]);
}

#[test]
fn grok_inference_records_keep_the_model_calls_own_time() {
    let root = tempfile::tempdir().unwrap();
    let logs = root.path().join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let at = |offset: i64| {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
            chrono::Utc::now().timestamp_millis() - 60_000 + offset,
        )
        .unwrap()
        .to_rfc3339()
    };
    let lines = [
        json!({"msg":"shell.turn.inference_done","sid":"g1","ts":at(0),"event_id":"e1","ctx":{"prompt_tokens":1000,"completion_tokens":200,"model_elapsed_ms":4734,"elapsed_since_turn_start_ms":13684}}),
        json!({"msg":"shell.turn.inference_done","sid":"g1","ts":at(9_000),"event_id":"e2","ctx":{"prompt_tokens":1200,"completion_tokens":300}}),
    ];
    std::fs::write(
        logs.join("unified.jsonl"),
        lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: logs,
            provider: "grok-cli",
        }],
        root.path().join("hud"),
    );
    refresh(&mut backend);
    assert_eq!(timed_calls(&backend), [(200, Some(4_734)), (300, None)]);
}

#[test]
fn copilot_chat_spans_keep_their_own_time() {
    let root = tempfile::tempdir().unwrap();
    let otel = root.path().join("otel");
    std::fs::create_dir_all(&otel).unwrap();
    let start = chrono::Utc::now().timestamp() - 60;
    let span = |id: &str, end: Option<[i64; 2]>, output: u64| {
        let mut span = json!({
            "type":"span","name":"chat gpt-5","traceId":"t1","spanId":id,
            "startTime":[start,0],
            "attributes":{
                "gen_ai.conversation.id":"c1","gen_ai.operation.name":"chat",
                "gen_ai.response.model":"gpt-5",
                "gen_ai.usage.input_tokens":100,"gen_ai.usage.output_tokens":output
            }
        });
        if let Some(end) = end {
            span["endTime"] = json!(end);
        }
        span
    };
    let lines = [
        span("a", Some([start + 3, 250_000_000]), 200),
        span("b", None, 300),
    ];
    std::fs::write(
        otel.join("spans.jsonl"),
        lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: otel,
            provider: "copilot",
        }],
        root.path().join("hud"),
    );
    refresh(&mut backend);
    assert_eq!(timed_calls(&backend), [(200, Some(3_250)), (300, None)]);
}

#[test]
fn chengxiaobang_runs_drive_waiting_tools_and_the_reply() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    let iso = |ago_ms: i64| {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
            chrono::Utc::now().timestamp_millis() - ago_ms,
        )
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    };
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         CREATE TABLE runs(id TEXT,session_id TEXT,status TEXT,created_at TEXT,updated_at TEXT);
         CREATE TABLE tool_calls(id TEXT,run_id TEXT,name TEXT,status TEXT,args_json TEXT,started_at TEXT,finished_at TEXT);
         CREATE TABLE messages(id TEXT,session_id TEXT,role TEXT,kind TEXT,content TEXT,created_at TEXT);
         INSERT INTO sessions VALUES('s1','Plan the trip');",
    )
    .unwrap();
    db.execute(
        "INSERT INTO usage_cost_entries VALUES('u1','s1','auto(free)',100,10,40,?1)",
        [iso(60_000)],
    )
    .unwrap();
    db.execute(
        "INSERT INTO runs VALUES('r1','s1','running',?1,?2)",
        [iso(61_000), iso(30_000)],
    )
    .unwrap();
    db.execute(
        "INSERT INTO tool_calls VALUES('c1','r1','AskUserQuestion','pending_approval','{\"questions\":[{\"question\":\"Which day?\",\"options\":[{\"label\":\"Mon\"}]}]}',?1,NULL)",
        [iso(30_000)],
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: path.clone(),
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let session = |backend: &mut AgentHudCore| {
        refresh(backend);
        let sessions = backend.request("sessions", json!({"scope":"all"})).unwrap();
        sessions["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "chengxiaobang:s1")
            .cloned()
            .unwrap()
    };
    let first = session(&mut backend);
    assert_eq!(first["state"], "waiting");

    db.execute_batch("UPDATE tool_calls SET status='completed';")
        .unwrap();
    db.execute(
        "INSERT INTO tool_calls VALUES('c2','r1','Shell','running',NULL,?1,NULL)",
        [iso(5_000)],
    )
    .unwrap();
    assert_eq!(session(&mut backend)["state"], "running");

    // Ended after this run of the app began, as a completion pops up.
    std::thread::sleep(std::time::Duration::from_millis(20));
    db.execute(
        "UPDATE tool_calls SET status='completed',finished_at=?1 WHERE id='c2'",
        [iso(0)],
    )
    .unwrap();
    db.execute(
        "INSERT INTO messages VALUES('m1','s1','assistant',NULL,'All booked.',?1)",
        [iso(0)],
    )
    .unwrap();
    db.execute("UPDATE runs SET status='completed',updated_at=?1", [iso(0)])
        .unwrap();
    let done = session(&mut backend);
    assert_eq!(done["state"], "idle");
    let snapshot = backend.request("snapshot", json!({})).unwrap();
    let completion = snapshot["islandEvents"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|e| e["kind"] == "completion" && e["sessionId"] == "chengxiaobang:s1");
    assert_eq!(
        completion.map(|e| e["message"].clone()),
        Some(json!("All booked.")),
        "{}",
        snapshot["islandEvents"]
    );
}

#[test]
fn chengxiaobang_questions_become_cards_and_answers_reach_the_plugin() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push((source.into(), session.into(), words.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    let iso = |ago_ms: i64| {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
            chrono::Utc::now().timestamp_millis() - ago_ms,
        )
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    };
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         CREATE TABLE runs(id TEXT,session_id TEXT,status TEXT,created_at TEXT,updated_at TEXT);
         CREATE TABLE tool_calls(id TEXT,run_id TEXT,name TEXT,status TEXT,args_json TEXT,started_at TEXT,finished_at TEXT,created_at TEXT);
         INSERT INTO sessions VALUES('s1','Held'),('s2','Native');",
    )
    .unwrap();
    for (s, r, status) in [("s1", "r1", "running"), ("s2", "r2", "pending_approval")] {
        db.execute(
            "INSERT INTO usage_cost_entries VALUES(?1,?2,'auto(free)',100,10,40,?3)",
            [format!("u-{s}"), s.to_owned(), iso(60_000)],
        )
        .unwrap();
        db.execute(
            "INSERT INTO runs VALUES(?1,?2,'running',?3,?4)",
            [r.to_owned(), s.to_owned(), iso(61_000), iso(20_000)],
        )
        .unwrap();
        db.execute(
            "INSERT INTO tool_calls VALUES(?1,?2,'AskUserQuestion',?3,'{\"questions\":[{\"question\":\"Which day?\",\"options\":[{\"label\":\"Mon\"},{\"label\":\"Tue\"}]}]}',NULL,NULL,?4)",
            [format!("c-{s}"), r.to_owned(), status.to_owned(), iso(20_000)],
        )
        .unwrap();
    }
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: path.clone(),
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    let snapshot = refresh(&mut backend);
    let approvals = snapshot["approvals"].as_array().unwrap().clone();
    let held = approvals
        .iter()
        .find(|a| a["sessionId"] == "chengxiaobang:s1")
        .expect("a card for the held question");
    assert!(held["id"]
        .as_str()
        .unwrap()
        .starts_with("chengxiaobang-ask:reply:"));
    assert_eq!(held["answersQuestion"], true);
    assert_eq!(held["questions"][0]["question"], "Which day?");
    let native = approvals
        .iter()
        .find(|a| a["sessionId"] == "chengxiaobang:s2")
        .expect("a card for the question in 程小帮's window");
    assert_eq!(native["answersQuestion"], false);
    assert!(!native["answerNote"].as_str().unwrap().is_empty());

    let key = held["questions"][0]["answerKey"]
        .as_str()
        .unwrap()
        .to_owned();
    backend
        .request(
            "answer_request",
            json!({"id":held["id"],"decision":{"kind":"answer","answers":{key:{"selected":["Tue"]}}}}),
        )
        .unwrap();
    assert_eq!(
        fake.0.lock().unwrap().clone(),
        vec![(
            "chengxiaobang".to_owned(),
            "chengxiaobang:s1".to_owned(),
            "Tue".to_owned()
        )]
    );
}

#[test]
fn an_agent_message_waits_for_send_then_reaches_the_session_and_flies() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, source: &str, session: &str) -> Result<(), crate::replies::Unreachable> {
            if source == "chengxiaobang" && session == "chengxiaobang:s1" {
                Ok(())
            } else {
                Err(crate::replies::Unreachable::new("not loaded".into(), None))
            }
        }
        fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push((source.into(), session.into(), words.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Trip plan'),('s2','Other');",
    )
    .unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    for s in ["s1", "s2"] {
        db.execute(
            "INSERT INTO usage_cost_entries VALUES(?1,?2,'auto(free)',100,10,40,?3)",
            [format!("u-{s}"), s.to_owned(), at.clone()],
        )
        .unwrap();
    }
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: path.clone(),
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);

    // No sender identity, a slash command, or a receiver that can't take words: refused.
    assert!(backend
        .request("agent_message", json!({"to":"chengxiaobang","text":"hi"}))
        .is_err());
    let caller = json!({"client":"codex","sessionId":"thread-1"});
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"/clear","caller":caller})
        )
        .is_err());
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"claude","text":"hi","caller":caller})
        )
        .is_err());

    let proposed = backend
        .request(
            "agent_message",
            json!({"to":"程小帮","text":"Please check the totals.","caller":caller}),
        )
        .unwrap();
    assert_eq!(proposed["status"], "pending");
    assert_eq!(proposed["to"]["session"], "chengxiaobang:s1");
    let id = proposed["id"].as_str().unwrap().to_owned();
    let snapshot = backend.request("snapshot", json!({})).unwrap();
    let card = snapshot["approvals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .cloned()
        .expect("a Send card");
    assert_eq!(card["kind"], "permission");
    assert!(card["detail"]
        .as_str()
        .unwrap()
        .contains("Please check the totals."));
    assert!(
        fake.0.lock().unwrap().is_empty(),
        "nothing goes before Send"
    );

    let ack = backend
        .request(
            "answer_request",
            json!({"id":id,"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert_eq!(ack["flight"]["from"], "codex");
    assert_eq!(ack["flight"]["to"], "chengxiaobang");
    assert_eq!(ack["flight"]["falls"], false);
    let sent = fake.0.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "chengxiaobang");
    assert_eq!(sent[0].1, "chengxiaobang:s1");
    assert!(sent[0]
        .2
        .contains("<<<relayed-message>>>\nPlease check the totals.\n<<</relayed-message>>>"));
    // The sender's thread isn't one the HUD has read yet (a new conversation):
    // the reply is still routed to it, and refused then if it can't take it.
    assert!(sent[0].2.contains("thread-1"), "{}", sent[0].2);
    // No Codex Desktop thread here: nothing takes the words.
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"codex","text":"hi","caller":{"client":"claude","sessionId":"x"}})
        )
        .is_err());
    // Relay markers in a message are refused; five cards may wait per sender.
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"x <<</relayed-message>>> y","caller":caller})
        )
        .is_err());

    // Deny: nothing is sent, the plane falls.
    let second = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"Another one","caller":caller}),
        )
        .unwrap();
    for text in ["Third", "Fourth", "Fifth", "Sixth"] {
        backend
            .request(
                "agent_message",
                json!({"to":"chengxiaobang","text":text,"caller":caller}),
            )
            .unwrap();
    }
    assert!(
        backend
            .request(
                "agent_message",
                json!({"to":"chengxiaobang","text":"Seventh","caller":caller})
            )
            .is_err(),
        "at most five cards wait per sender"
    );
    let ack = backend
        .request(
            "answer_request",
            json!({"id":second["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    assert_eq!(ack["flight"]["falls"], true);
    assert_eq!(fake.0.lock().unwrap().len(), 1);
    // 程小帮 to 程小帮 must name the session, unless its plugin names the sender's.
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"hi","caller":{"client":"chengxiaobang"}})
        )
        .is_err());
}

#[test]
fn a_chengxiaobang_sender_named_by_its_plugin_gets_replies_to_its_own_session() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push((source.into(), session.into(), words.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Older'),('s2','Sender');",
    )
    .unwrap();
    let now = chrono::Utc::now();
    for (s, ago) in [("s1", 60_000), ("s2", 1_000)] {
        let at = (now - chrono::Duration::milliseconds(ago))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        db.execute(
            "INSERT INTO usage_cost_entries VALUES(?1,?2,'auto(free)',100,10,40,?3)",
            [format!("u-{s}"), s.to_owned(), at],
        )
        .unwrap();
    }
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: path.clone(),
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    // s2 is the most recent, but it is the sender: the message goes to s1.
    let proposed = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"hello","from_session":"s2","caller":{"client":"chengxiaobang"}}),
        )
        .unwrap();
    assert_eq!(proposed["to"]["session"], "chengxiaobang:s1");
    backend
        .request(
            "answer_request",
            json!({"id":proposed["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    let sent = fake.0.lock().unwrap().clone();
    assert_eq!(sent[0].1, "chengxiaobang:s1");
    // The reply route names the sender's own session, without the HUD's prefix.
    assert!(sent[0].2.contains("\"s2\""), "{}", sent[0].2);
    // A Claude caller can't borrow from_session: its identity comes from the relay.
    let other = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"x","from_session":"s1","caller":{"client":"claude","sessionId":"c-unknown"}}),
        )
        .unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":other["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    let words = fake.0.lock().unwrap().last().unwrap().2.clone();
    assert!(!words.contains("\"s1\""), "{words}");
}

#[test]
fn a_codex_desktop_thread_can_take_a_message() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, source: &str, session: &str, _: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((source.into(), session.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    for (thread, originator) in [
        ("01aaaaaa-0000-7000-8000-000000000001", "Codex Desktop"),
        ("01aaaaaa-0000-7000-8000-000000000002", "codex_cli_rs"),
    ] {
        let lines = [
            json!({"timestamp":at,"type":"session_meta","payload":{"id":thread,"timestamp":at,"cwd":"/tmp","originator":originator,"cli_version":"0.1.0","source":if originator == "codex_cli_rs" {"cli"} else {"vscode"}}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"hello"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0,"total_tokens":110},"last_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0,"total_tokens":110}}}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_complete"}}),
        ];
        let text = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(sessions.join(format!("rollout-{thread}.jsonl")), text).unwrap();
    }
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: sessions.clone(),
            provider: "codex",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    let proposed = backend
        .request(
            "agent_message",
            json!({"to":"codex","text":"hi Codex","caller":{"client":"claude","sessionId":"c1"}}),
        )
        .unwrap();
    // The Desktop thread is chosen; the CLI thread never is.
    assert_eq!(
        proposed["to"]["session"],
        "01aaaaaa-0000-7000-8000-000000000001"
    );
    backend
        .request(
            "answer_request",
            json!({"id":proposed["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert_eq!(
        fake.0.lock().unwrap().clone(),
        vec![(
            "codex".to_owned(),
            "01aaaaaa-0000-7000-8000-000000000001".to_owned()
        )]
    );
}

#[test]
fn a_command_line_sender_waits_for_the_reply_and_hears_a_denial() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, source: &str, session: &str) -> Result<(), crate::replies::Unreachable> {
            if source == "chengxiaobang" && session == "chengxiaobang:s1" {
                Ok(())
            } else {
                Err(crate::replies::Unreachable::new("not loaded".into(), None))
            }
        }
        fn reply(&self, source: &str, session: &str, words: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push((source.into(), session.into(), words.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Meetings');",
    )
    .unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    db.execute(
        "INSERT INTO usage_cost_entries VALUES('u1','s1','auto(free)',100,10,40,?1)",
        [at],
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: path.clone(),
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);

    // An old Codex thread the HUD doesn't list asks from its shell, and waits.
    let cli = json!({"client":"codex","sessionId":"t-cli","via":"cli","waits":true});
    let inbox = backend
        .request("agent_inbox", json!({"caller":cli,"waitMs":60_000}))
        .unwrap();
    assert_eq!(inbox["messages"], json!([]));
    let asked = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"Any meetings today?","caller":cli}),
        )
        .unwrap();
    let card = backend.request("snapshot", json!({})).unwrap()["approvals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == asked["id"])
        .cloned()
        .unwrap();
    // Not a thread the HUD lists: unverified, and marked as the command line's.
    assert!(card["title"].as_str().unwrap().contains("Codex"));
    backend
        .request(
            "answer_request",
            json!({"id":asked["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    let words = fake.0.lock().unwrap()[0].2.clone();
    // The receiver is told where the reply goes: the waiting thread.
    assert!(words.contains("t-cli"), "{words}");

    // 程小帮 replies to that thread: the waiting command line takes it, no carrier.
    let reply = backend
        .request(
            "agent_message",
            json!({"to":"codex","session":"t-cli","text":"Two: 13:00 and 17:30.","from_session":"s1","caller":{"client":"chengxiaobang"}}),
        )
        .unwrap();
    assert_eq!(reply["to"]["session"], "t-cli");
    backend
        .request(
            "answer_request",
            json!({"id":reply["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert_eq!(fake.0.lock().unwrap().len(), 1, "the carrier is not used");
    let inbox = backend
        .request("agent_inbox", json!({"caller":cli,"waitMs":60_000}))
        .unwrap();
    let messages = inbox["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert!(messages[0]
        .as_str()
        .unwrap()
        .contains("Two: 13:00 and 17:30."));

    // A denied message tells the waiting command line.
    let denied = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"One more","caller":cli}),
        )
        .unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":denied["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    let inbox = backend
        .request("agent_inbox", json!({"caller":cli,"waitMs":0}))
        .unwrap();
    assert!(inbox["messages"][0]
        .as_str()
        .unwrap()
        .contains("SI Future HUD"));
    // Once it stops waiting, a reply to that thread has nowhere to go.
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"codex","session":"t-cli","text":"late","from_session":"s1","caller":{"client":"chengxiaobang"}}),
        )
        .is_err());

    // Five cards may wait per sender session, not per agent.
    for session in ["a", "b"] {
        for n in 0..5 {
            backend
                .request(
                    "agent_message",
                    json!({"to":"chengxiaobang","text":format!("{session}{n}"),"caller":{"client":"claude","sessionId":session}}),
                )
                .unwrap();
        }
    }
    assert!(backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"a5","caller":{"client":"claude","sessionId":"a"}}),
        )
        .is_err());
}

#[test]
fn words_for_a_codex_thread_mid_turn_wait_for_the_turn_to_end() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, source: &str, session: &str, _: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((source.into(), session.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let thread = "01aaaaaa-0000-7000-8000-0000000000aa";
    let file = sessions.join(format!("rollout-{thread}.jsonl"));
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let line = |value: Value| value.to_string() + "\n";
    let mut text = String::new();
    for value in [
        json!({"timestamp":at,"type":"session_meta","payload":{"id":thread,"timestamp":at,"cwd":"/tmp","originator":"Codex Desktop","cli_version":"0.1.0","source":"vscode"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"build it"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_started"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0,"total_tokens":110},"last_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":0,"total_tokens":110}}}}),
    ] {
        text.push_str(&line(value));
    }
    std::fs::write(&file, &text).unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: sessions.clone(),
            provider: "codex",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    let proposed = backend
        .request(
            "agent_message",
            json!({"to":"codex","text":"Totals are in.","caller":{"client":"claude","sessionId":"c1"}}),
        )
        .unwrap();
    assert_eq!(proposed["to"]["session"], thread);
    backend
        .request(
            "answer_request",
            json!({"id":proposed["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert!(
        fake.0.lock().unwrap().is_empty(),
        "nothing goes in while the turn runs"
    );
    // The turn ends: the next pass sends the words.
    text.push_str(&line(
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_complete"}}),
    ));
    std::fs::write(&file, &text).unwrap();
    refresh(&mut backend);
    backend.request("snapshot", json!({})).unwrap();
    assert_eq!(
        fake.0.lock().unwrap().clone(),
        vec![("codex".to_owned(), thread.to_owned())]
    );
    // The same words asked twice wait on one card.
    let caller = json!({"client":"claude","sessionId":"c1"});
    let first = backend
        .request(
            "agent_message",
            json!({"to":"codex","text":"again","caller":caller}),
        )
        .unwrap();
    let second = backend
        .request(
            "agent_message",
            json!({"to":"codex","text":"again","caller":caller}),
        )
        .unwrap();
    assert_eq!(first["id"], second["id"]);
}

#[test]
fn a_message_naming_no_session_goes_to_the_one_the_user_looked_at_last() {
    use std::collections::HashMap;
    use std::sync::Arc;
    // A scheduled task ran in one thread a minute ago; the user last read the other.
    struct Fake(Option<HashMap<String, i64>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn viewed_at(&self, _: &str) -> Option<HashMap<String, i64>> {
            self.0.clone()
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let scheduled = "01aaaaaa-0000-7000-8000-0000000000a1";
    let read = "01aaaaaa-0000-7000-8000-0000000000a2";
    let now = chrono::Utc::now();
    for (thread, ago) in [(scheduled, 60), (read, 3600)] {
        let at = (now - chrono::Duration::seconds(ago))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let text = [
            json!({"timestamp":at,"type":"session_meta","payload":{"id":thread,"timestamp":at,"cwd":"/tmp","originator":"Codex Desktop","cli_version":"0.1.0","source":"vscode"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"hello"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_started"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_complete"}}),
        ]
        .iter()
        .map(|value| value.to_string() + "\n")
        .collect::<String>();
        std::fs::write(sessions.join(format!("rollout-{thread}.jsonl")), text).unwrap();
    }
    let target = |viewed: Option<HashMap<String, i64>>| {
        let mut backend = AgentHudCore::new(
            vec![SourceRoot {
                path: sessions.clone(),
                provider: "codex",
            }],
            root.path().join(format!("hud-{}", viewed.is_some())),
        );
        backend.set_session_replies(Some(Arc::new(Fake(viewed))));
        refresh(&mut backend);
        let proposed = backend
            .request(
                "agent_message",
                json!({"to":"codex","text":"hi","caller":{"client":"claude","sessionId":"c1"}}),
            )
            .unwrap();
        proposed["to"]["session"].as_str().unwrap().to_owned()
    };
    // Without a record of viewing, the most recently updated one, as before.
    assert_eq!(target(None), scheduled);
    let viewed = HashMap::from([
        (read.to_owned(), now.timestamp_millis() - 120_000),
        (scheduled.to_owned(), now.timestamp_millis() - 86_400_000),
    ]);
    assert_eq!(target(Some(viewed)), read);
}

#[test]
fn the_candidate_cap_never_cuts_a_session_running_the_carrier() {
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    // Forty threads the user read lately can't take words; the one that can
    // runs the carrier now, though nobody has looked at it.
    struct Fake {
        live: String,
        viewed: HashMap<String, i64>,
    }
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, session: &str) -> Result<(), crate::replies::Unreachable> {
            if session == self.live {
                Ok(())
            } else {
                Err(crate::replies::Unreachable::new("not loaded".into(), None))
            }
        }
        fn reply(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn live_sessions(&self, _: &str) -> Option<HashSet<String>> {
            Some(HashSet::from([self.live.clone()]))
        }
        fn viewed_at(&self, _: &str) -> Option<HashMap<String, i64>> {
            Some(self.viewed.clone())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let now = chrono::Utc::now();
    let at = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let thread = |n: u32| format!("01aaaaaa-0000-7000-8000-{n:012}");
    for n in 0..41 {
        let id = thread(n);
        let text = [
            json!({"timestamp":at,"type":"session_meta","payload":{"id":id,"timestamp":at,"cwd":"/tmp","originator":"Codex Desktop","cli_version":"0.1.0","source":"vscode"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"hello"}}),
        ]
        .iter()
        .map(|value| value.to_string() + "\n")
        .collect::<String>();
        std::fs::write(sessions.join(format!("rollout-{id}.jsonl")), text).unwrap();
    }
    let live = thread(40);
    let viewed = (0..40)
        .map(|n| (thread(n), now.timestamp_millis() - i64::from(n)))
        .collect();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: sessions,
            provider: "codex",
        }],
        root.path().join("hud"),
    );
    backend.set_session_replies(Some(Arc::new(Fake {
        live: live.clone(),
        viewed,
    })));
    refresh(&mut backend);
    let proposed = backend
        .request(
            "agent_message",
            json!({"to":"codex","text":"hi","caller":{"client":"claude","sessionId":"c1"}}),
        )
        .unwrap();
    assert_eq!(proposed["to"]["session"], live.as_str());
}

#[test]
fn a_dead_wait_lets_go_and_a_wait_hears_only_about_its_own_card() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<String>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, source: &str, session: &str) -> Result<(), crate::replies::Unreachable> {
            if source == "chengxiaobang" && session == "chengxiaobang:s1" {
                Ok(())
            } else {
                Err(crate::replies::Unreachable::new("not loaded".into(), None))
            }
        }
        fn reply(&self, _: &str, _: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(words.into());
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Meetings');",
    )
    .unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    db.execute(
        "INSERT INTO usage_cost_entries VALUES('u1','s1','auto(free)',100,10,40,?1)",
        [at],
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path,
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    backend.set_session_replies(Some(Arc::new(Fake(Mutex::new(vec![])))));
    refresh(&mut backend);
    let cli = json!({"client":"codex","sessionId":"t-cli","via":"cli","waits":true});
    let now = chrono::Utc::now().timestamp_millis();

    // A `send --wait 1800` killed twenty seconds ago no longer holds the thread.
    backend
        .agent_inbox(&json!({"caller":cli,"waitMs":1_800_000}), now - 20_000)
        .unwrap();
    assert!(
        backend
            .request(
                "agent_message",
                json!({"to":"codex","session":"t-cli","text":"late","from_session":"s1","caller":{"client":"chengxiaobang"}}),
            )
            .is_err(),
        "no wait is left to take it"
    );

    // Two cards from that thread; its command line waits on the second.
    let first = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"one","caller":cli}),
        )
        .unwrap();
    let second = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"two","caller":cli}),
        )
        .unwrap();
    let wait = json!({"caller":cli,"card":second["id"],"waitMs":60_000});
    backend.request("agent_inbox", wait.clone()).unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":first["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    let inbox = backend.request("agent_inbox", wait.clone()).unwrap();
    assert_eq!(
        inbox["messages"],
        json!([]),
        "the first card's denial isn't its answer"
    );
    backend
        .request(
            "answer_request",
            json!({"id":second["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    let inbox = backend.request("agent_inbox", wait).unwrap();
    assert!(inbox["messages"][0]
        .as_str()
        .unwrap()
        .contains("SI Future HUD"));

    // The same words for another named session are not the waiting card again.
    let caller = json!({"client":"claude","sessionId":"c9"});
    let to_s1 = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s1","text":"same","caller":caller}),
        )
        .unwrap();
    let elsewhere = backend.request(
        "agent_message",
        json!({"to":"chengxiaobang","session":"s2","text":"same","caller":caller}),
    );
    assert!(elsewhere.map_or(true, |other| other["id"] != to_s1["id"]));
}

#[test]
fn a_new_conversation_not_listed_yet_takes_words_and_a_new_sender_gets_its_reply() {
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    // 程小帮's "fresh" runs the plugin and was just opened, but has no usage yet.
    struct Fake(Mutex<Vec<(String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, session: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((session.into(), words.into()));
            Ok(())
        }
        fn live_sessions(&self, source: &str) -> Option<HashSet<String>> {
            (source == "chengxiaobang").then(|| {
                HashSet::from([
                    "chengxiaobang:s1".to_owned(),
                    "chengxiaobang:fresh".to_owned(),
                ])
            })
        }
        fn viewed_at(&self, source: &str) -> Option<HashMap<String, i64>> {
            let now = chrono::Utc::now().timestamp_millis();
            (source == "chengxiaobang").then(|| {
                HashMap::from([
                    ("chengxiaobang:fresh".to_owned(), now),
                    ("chengxiaobang:s1".to_owned(), now - 3_600_000),
                ])
            })
        }
        fn session_title(&self, _: &str, session: &str) -> Option<String> {
            (session == "chengxiaobang:fresh").then(|| "HUD test".to_owned())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Meetings');
         INSERT INTO sessions VALUES('fresh','HUD test');",
    )
    .unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    db.execute(
        "INSERT INTO usage_cost_entries VALUES('u1','s1','auto(free)',100,10,40,?1)",
        [at],
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path,
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    // A Claude session the HUD hasn't read yet sends, naming no receiver.
    let sent = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","text":"Hello","caller":{"client":"claude","sessionId":"c-new"}}),
        )
        .unwrap();
    assert_eq!(sent["to"]["session"], "chengxiaobang:fresh");
    assert_eq!(sent["to"]["title"], "HUD test");
    backend
        .request(
            "answer_request",
            json!({"id":sent["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    let (session, words) = fake.0.lock().unwrap()[0].clone();
    assert_eq!(session, "chengxiaobang:fresh");
    // The reply is pointed at the sender itself, not at whichever Claude
    // session happens to be most recent.
    assert!(words.contains("c-new"), "{words}");
}

#[test]
fn super_root_sends_verified_messages_at_once_a_limited_number_at_a_time() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, session: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((session.into(), words.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Meetings');
         INSERT INTO sessions VALUES('s2','Reports');
         INSERT INTO sessions VALUES('s3','Notes');",
    )
    .unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    for (entry, session) in [("u1", "s1"), ("u2", "s2"), ("u3", "s3")] {
        db.execute(
            "INSERT INTO usage_cost_entries VALUES(?1,?2,'auto(free)',100,10,40,?3)",
            [entry, session, at.as_str()],
        )
        .unwrap();
    }
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path,
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    let send = |backend: &mut AgentHudCore, text: String, caller: Value| {
        backend
            .request(
                "agent_message",
                json!({"to":"chengxiaobang","session":"s2","text":text,"from_session":"s1","caller":caller}),
            )
            .unwrap()
    };
    let verified = json!({"client":"chengxiaobang"});
    // Off by default: the card waits for the click.
    assert_eq!(
        send(&mut backend, "first".into(), verified.clone())["status"],
        "pending"
    );
    backend
        .request("set_setting", json!({"key":"superRoot","value":true}))
        .unwrap();
    // On: a session the HUD lists sends at once, and a plane flies.
    let sent = send(&mut backend, "second".into(), verified.clone());
    assert_eq!(sent["status"], "sent");
    assert!(sent["flight"].is_object());
    let (session, words) = fake.0.lock().unwrap().last().unwrap().clone();
    assert_eq!(session, "chengxiaobang:s2");
    // The receiver is told no one confirmed it, never that the user agreed.
    assert!(words.contains("SuperRoot"), "{words}");
    assert!(!words.contains("only agreed"), "{words}");
    // One it can't verify still waits, and the card says why.
    let unverified = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s2","text":"who am I","caller":{"client":"claude","sessionId":"not-listed"}}),
        )
        .unwrap();
    assert_eq!(unverified["status"], "pending");
    let card = backend.request("snapshot", json!({})).unwrap()["approvals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == unverified["id"])
        .cloned()
        .unwrap();
    assert!(card.to_string().contains("SuperRoot"), "{card}");
    // The same words for another session are another message.
    let elsewhere = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s3","text":"second","from_session":"s1","caller":verified}),
        )
        .unwrap();
    assert_ne!(elsewhere["id"], sent["id"]);
    assert_eq!(fake.0.lock().unwrap().last().unwrap().0, "chengxiaobang:s3");
    // Past ten in ten minutes from one session, the click is asked again,
    // and stays asked until the user sends one.
    for n in 0..8 {
        assert_eq!(
            send(&mut backend, format!("more {n}"), verified.clone())["status"],
            "sent"
        );
    }
    let eleventh = send(&mut backend, "eleventh".into(), verified.clone());
    assert_eq!(eleventh["status"], "pending");
    assert_eq!(
        send(&mut backend, "twelfth".into(), verified.clone())["status"],
        "pending"
    );
    backend
        .request(
            "answer_request",
            json!({"id":eleventh["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert_eq!(
        send(&mut backend, "resumed".into(), verified.clone())["status"],
        "sent"
    );
    // The same short words twice are two messages (agents answer "OK" often).
    let delivered = fake.0.lock().unwrap().len();
    let first_ok = send(&mut backend, "OK".into(), verified.clone());
    // (A card's id carries the millisecond it was made.)
    std::thread::sleep(std::time::Duration::from_millis(2));
    let second_ok = send(&mut backend, "OK".into(), verified);
    assert_eq!(second_ok["status"], "sent");
    assert_ne!(first_ok["id"], second_ok["id"]);
    assert_eq!(fake.0.lock().unwrap().len(), delivered + 2);
}

#[test]
fn a_reply_for_a_wait_that_was_stopped_goes_into_the_conversation() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<(String, String)>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, session: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push((session.into(), words.into()));
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         INSERT INTO sessions VALUES('s1','Meetings');
         INSERT INTO sessions VALUES('s2','Reports');",
    )
    .unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    for (entry, session) in [("u1", "s1"), ("u2", "s2")] {
        db.execute(
            "INSERT INTO usage_cost_entries VALUES(?1,?2,'auto(free)',100,10,40,?3)",
            [entry, session, at.as_str()],
        )
        .unwrap();
    }
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path,
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    // s2's command line waits; a reply for it comes, then the command is stopped.
    let waiting = json!({"client":"chengxiaobang","sessionId":"s2","via":"cli","waits":true});
    backend
        .request("agent_inbox", json!({"caller":waiting,"waitMs":600_000}))
        .unwrap();
    let reply = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s2","text":"Done.","from_session":"s1","caller":{"client":"chengxiaobang"}}),
        )
        .unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":reply["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert!(
        fake.0.lock().unwrap().is_empty(),
        "the waiting command takes it"
    );
    // Twenty seconds without a poll: the wait lapses and the reply goes into
    // s2's conversation, as it would have with nobody waiting.
    let later = chrono::Utc::now().timestamp_millis() + 20_000;
    backend
        .agent_inbox(
            &json!({"caller":{"client":"claude","sessionId":"other"},"waitMs":0}),
            later,
        )
        .unwrap();
    // (程小帮's plugin queues words itself: no turn holds them back.)
    let delivered = fake.0.lock().unwrap().clone();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].0, "chengxiaobang:s2");
    assert!(delivered[0].1.contains("Done."));

    // A denial for the card a wait is on is kept for its command line, even
    // when that comes back only after the wait lapsed (the Mac slept).
    let card = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s1","text":"Shall I?","caller":waiting}),
        )
        .unwrap();
    let wait = json!({"caller":waiting,"card":card["id"],"waitMs":600_000});
    backend.request("agent_inbox", wait.clone()).unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":card["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    let back = chrono::Utc::now().timestamp_millis() + 40_000;
    backend
        .agent_inbox(
            &json!({"caller":{"client":"claude","sessionId":"other"},"waitMs":0}),
            back,
        )
        .unwrap();
    let inbox = backend.agent_inbox(&wait, back).unwrap();
    assert!(inbox["messages"][0]
        .as_str()
        .unwrap()
        .contains("SI Future HUD"));

    // That command never cleared its wait; a new one starts in the same
    // session: it isn't held to the old card, and hears about its own.
    let denied = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s1","text":"And this?","caller":waiting}),
        )
        .unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":denied["id"],"decision":{"kind":"allow"}}),
        )
        .ok();
    let stale = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s1","text":"Old card","caller":waiting}),
        )
        .unwrap();
    backend
        .agent_inbox(
            &json!({"caller":waiting,"card":stale["id"],"waitMs":600_000}),
            back,
        )
        .unwrap();
    let later = back + 40_000;
    backend
        .request(
            "answer_request",
            json!({"id":stale["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    // The new command registers (no card yet), proposes, and is denied
    // before its first poll naming its card.
    backend
        .agent_inbox(&json!({"caller":waiting,"waitMs":600_000}), later)
        .unwrap();
    let fresh = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s1","text":"Fresh","caller":waiting}),
        )
        .unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":fresh["id"],"decision":{"kind":"deny"}}),
        )
        .unwrap();
    let inbox = backend
        .agent_inbox(
            &json!({"caller":waiting,"card":fresh["id"],"waitMs":600_000}),
            later + 1_000,
        )
        .unwrap();
    assert!(
        inbox["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|words| words.as_str().unwrap().contains("SI Future HUD")),
        "{inbox}"
    );
}

#[test]
fn a_second_message_right_after_the_first_waits_for_the_turn_it_started() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<String>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, _: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(words.into());
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let thread = "01aaaaaa-0000-7000-8000-0000000000cc";
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // An idle Codex Desktop thread: its last turn has ended.
    let text = [
        json!({"timestamp":at,"type":"session_meta","payload":{"id":thread,"timestamp":at,"cwd":"/tmp","originator":"Codex Desktop","cli_version":"0.1.0","source":"vscode"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"hello"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_started"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_complete"}}),
    ]
    .iter()
    .map(|value| value.to_string() + "\n")
    .collect::<String>();
    std::fs::write(sessions.join(format!("rollout-{thread}.jsonl")), text).unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: sessions,
            provider: "codex",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    let send = |backend: &mut AgentHudCore, text: &str| {
        let card = backend
            .request(
                "agent_message",
                json!({"to":"codex","text":text,"caller":{"client":"claude","sessionId":"c1"}}),
            )
            .unwrap();
        backend
            .request(
                "answer_request",
                json!({"id":card["id"],"decision":{"kind":"allow"}}),
            )
            .unwrap();
    };
    send(&mut backend, "first");
    // The log can't show the turn "first" started until the next pass.
    send(&mut backend, "second");
    assert_eq!(fake.0.lock().unwrap().len(), 1, "the second waits");
    let later = chrono::Utc::now().timestamp_millis() + 20_000;
    backend.deliver_deferred_messages(later);
    let sent = fake.0.lock().unwrap().clone();
    assert_eq!(sent.len(), 2);
    assert!(sent[1].contains("second"));
}

#[test]
fn words_for_a_session_waiting_on_the_user_go_in_once_it_is_answered() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<String>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, _: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(words.into());
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("chengxiaobang.sqlite");
    let db = Connection::open(&path).unwrap();
    let iso = |ago_ms: i64| {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
            chrono::Utc::now().timestamp_millis() - ago_ms,
        )
        .unwrap()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    };
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT,title TEXT);
         CREATE TABLE usage_cost_entries(id TEXT,session_id TEXT,model TEXT,prompt_tokens INTEGER,completion_tokens INTEGER,cached_prompt_tokens INTEGER,entry_created_at TEXT);
         CREATE TABLE runs(id TEXT,session_id TEXT,status TEXT,created_at TEXT,updated_at TEXT);
         CREATE TABLE tool_calls(id TEXT,run_id TEXT,name TEXT,status TEXT,args_json TEXT,started_at TEXT,finished_at TEXT);
         CREATE TABLE messages(id TEXT,session_id TEXT,role TEXT,kind TEXT,content TEXT,created_at TEXT);
         INSERT INTO sessions VALUES('s1','Plan the trip');",
    )
    .unwrap();
    db.execute(
        "INSERT INTO usage_cost_entries VALUES('u1','s1','auto(free)',100,10,40,?1)",
        [iso(60_000)],
    )
    .unwrap();
    db.execute(
        "INSERT INTO runs VALUES('r1','s1','running',?1,?2)",
        [iso(61_000), iso(30_000)],
    )
    .unwrap();
    db.execute(
        "INSERT INTO tool_calls VALUES('c1','r1','AskUserQuestion','pending_approval','{\"questions\":[{\"question\":\"Which day?\",\"options\":[{\"label\":\"Mon\"}]}]}',?1,NULL)",
        [iso(30_000)],
    )
    .unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: path.clone(),
            provider: "chengxiaobang",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    // A reply named for s1 while s1 waits on the user: no longer refused.
    let reply = backend
        .request(
            "agent_message",
            json!({"to":"chengxiaobang","session":"s1","text":"Here are the dates.","caller":{"client":"claude","sessionId":"c1"}}),
        )
        .unwrap();
    assert_eq!(reply["to"]["session"], "chengxiaobang:s1");
    backend
        .request(
            "answer_request",
            json!({"id":reply["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    assert!(
        fake.0.lock().unwrap().is_empty(),
        "nothing cuts into the question"
    );
    // The user answers; the next pass sends the words in.
    db.execute_batch(
        "UPDATE tool_calls SET status='completed'; UPDATE runs SET status='completed';",
    )
    .unwrap();
    refresh(&mut backend);
    let sent = fake.0.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Here are the dates."));
}

#[test]
fn a_wait_back_from_a_pause_takes_the_reply_that_came_meanwhile() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<String>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, _: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(words.into());
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let thread = "01aaaaaa-0000-7000-8000-0000000000dd";
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // A Codex Desktop thread mid-turn: its turn runs `send --wait`.
    let text = [
        json!({"timestamp":at,"type":"session_meta","payload":{"id":thread,"timestamp":at,"cwd":"/tmp","originator":"Codex Desktop","cli_version":"0.1.0","source":"vscode"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"ask Claude"}}),
        json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_started"}}),
    ]
    .iter()
    .map(|value| value.to_string() + "\n")
    .collect::<String>();
    std::fs::write(sessions.join(format!("rollout-{thread}.jsonl")), text).unwrap();
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: sessions,
            provider: "codex",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    let now = chrono::Utc::now().timestamp_millis();
    let waiting = json!({"client":"codex","sessionId":thread,"via":"cli","waits":true});
    backend
        .agent_inbox(&json!({"caller":waiting,"waitMs":600_000}), now)
        .unwrap();
    let reply = backend
        .request(
            "agent_message",
            json!({"to":"codex","session":thread,"text":"Two options.","caller":{"client":"claude","sessionId":"c1"}}),
        )
        .unwrap();
    backend
        .request(
            "answer_request",
            json!({"id":reply["id"],"decision":{"kind":"allow"}}),
        )
        .unwrap();
    // The Mac sleeps; meanwhile another request clears lapsed waits away.
    backend
        .agent_inbox(
            &json!({"caller":{"client":"claude","sessionId":"other"},"waitMs":0}),
            now + 16_000,
        )
        .unwrap();
    // Back, the command line polls again: the reply is there, not stuck
    // behind the turn that waits for it.
    let inbox = backend
        .agent_inbox(&json!({"caller":waiting,"waitMs":600_000}), now + 17_000)
        .unwrap();
    assert!(inbox["messages"][0]
        .as_str()
        .unwrap()
        .contains("Two options."));
    assert!(fake.0.lock().unwrap().is_empty());
}

#[test]
fn turning_super_root_off_stops_the_words_it_was_still_holding() {
    use std::sync::{Arc, Mutex};
    struct Fake(Mutex<Vec<String>>);
    impl crate::replies::SessionReplies for Fake {
        fn reaches(&self, _: &str, _: &str) -> Result<(), crate::replies::Unreachable> {
            Ok(())
        }
        fn reply(&self, _: &str, _: &str, words: &str) -> Result<(), String> {
            self.0.lock().unwrap().push(words.into());
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let sessions = root.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // Two Codex Desktop threads: the sender, and a receiver mid-turn.
    let thread = |id: &str, ended: bool| {
        let mut lines = vec![
            json!({"timestamp":at,"type":"session_meta","payload":{"id":id,"timestamp":at,"cwd":"/tmp","originator":"Codex Desktop","cli_version":"0.1.0","source":"vscode"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"user_message","message":"work"}}),
            json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_started"}}),
        ];
        if ended {
            lines.push(
                json!({"timestamp":at,"type":"event_msg","payload":{"type":"task_complete"}}),
            );
        }
        let text = lines
            .iter()
            .map(|v| v.to_string() + "\n")
            .collect::<String>();
        std::fs::write(sessions.join(format!("rollout-{id}.jsonl")), text).unwrap();
    };
    let sender = "01aaaaaa-0000-7000-8000-0000000000e1";
    let receiver = "01aaaaaa-0000-7000-8000-0000000000e2";
    thread(sender, true);
    thread(receiver, false);
    let mut backend = AgentHudCore::new(
        vec![SourceRoot {
            path: sessions,
            provider: "codex",
        }],
        root.path().join("hud"),
    );
    let fake = Arc::new(Fake(Mutex::new(vec![])));
    backend.set_session_replies(Some(fake.clone()));
    refresh(&mut backend);
    backend
        .request("set_setting", json!({"key":"superRoot","value":true}))
        .unwrap();
    let sent = backend
        .request(
            "agent_message",
            json!({"to":"codex","session":receiver,"text":"Keep going.","caller":{"client":"codex","sessionId":sender}}),
        )
        .unwrap();
    assert_eq!(sent["status"], "sent");
    assert!(fake.0.lock().unwrap().is_empty(), "held for the turn");
    // The user turns SuperRoot off to stop the agents, then the turn ends.
    backend
        .request("set_setting", json!({"key":"superRoot","value":false}))
        .unwrap();
    backend.deliver_deferred_messages(chrono::Utc::now().timestamp_millis() + 60_000);
    assert!(fake.0.lock().unwrap().is_empty(), "nothing it held goes in");
}
