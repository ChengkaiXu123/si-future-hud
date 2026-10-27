//! Each reader's call times (`metering::Event::duration_ms`), from line
//! shapes the clients write today.
use crate::Log;
use serde_json::{json, Value};

/// Parses `lines` as consecutive lines of one log, each at its own offset.
fn read(path: &str, source: &'static str, lines: &[Value]) -> Log {
    let mut log = Log::new(path.into(), source);
    for (offset, line) in lines.iter().enumerate() {
        log.meter_line_start = offset as u64 * 1000;
        log.parse(line);
    }
    log
}

/// The calls' durations, oldest first.
fn durations(log: &Log) -> Vec<Option<u64>> {
    let mut events = log.meter_events.values().collect::<Vec<_>>();
    events.sort_by_key(|event| event.at);
    events.iter().map(|event| event.duration_ms).collect()
}

fn claude(kind: &str, at: &str, side: bool, message: Value) -> Value {
    json!({"type":kind,"sessionId":"s1","isSidechain":side,"timestamp":format!("2026-10-23T08:00:{at}Z"),"message":message})
}

#[test]
fn a_claude_response_runs_from_its_chains_newest_user_line_to_its_last_line() {
    let usage = json!({"input_tokens":10,"output_tokens":200});
    let reply = |id: &str, block: &str| json!({"id":id,"role":"assistant","model":"claude-opus-4-5","usage":usage,"content":[{"type":block}]});
    let log = read(
        "s1.jsonl",
        "claude",
        &[
            claude(
                "user",
                "00.000",
                false,
                json!({"role":"user","content":"go"}),
            ),
            // Each block is its own line with the same counts.
            claude("assistant", "03.000", false, reply("m1", "thinking")),
            claude("assistant", "05.000", false, reply("m1", "tool_use")),
            claude(
                "user",
                "09.000",
                false,
                json!({"role":"user","content":[{"type":"tool_result"}]}),
            ),
            // A side chain's request does not time the main chain's reply.
            claude(
                "user",
                "11.000",
                true,
                json!({"role":"user","content":"sub"}),
            ),
            claude("assistant", "12.000", false, reply("m2", "text")),
        ],
    );
    assert_eq!(durations(&log), [Some(5_000), Some(3_000)]);
    let first = log
        .meter_events
        .values()
        .min_by_key(|event| event.at)
        .unwrap();
    assert_eq!(
        first.at,
        crate::transcript::metering::date_ms("2026-10-23T08:00:03Z").unwrap(),
        "the call keeps its first line's time"
    );
}

#[test]
fn a_claude_response_with_no_user_line_before_it_is_not_timed() {
    let log = read(
        "s1.jsonl",
        "claude",
        &[claude(
            "assistant",
            "03.000",
            false,
            json!({"id":"m1","role":"assistant","model":"claude-opus-4-5","usage":{"input_tokens":1,"output_tokens":20},"content":[]}),
        )],
    );
    assert_eq!(durations(&log), [None]);
}

fn codex(at: &str, kind: &str, payload: Value) -> Value {
    json!({"timestamp":format!("2026-10-23T08:00:{at}Z"),"type":kind,"payload":payload})
}

fn count(input: u64, output: u64) -> Value {
    json!({"type":"token_count","info":{"total_token_usage":{"input_tokens":input,"output_tokens":output}}})
}

#[test]
fn a_codex_reply_ends_at_its_usage_record_not_at_the_count_after_its_tools() {
    let log = read(
        "rollout.jsonl",
        "codex",
        &[
            codex(
                "00.000",
                "event_msg",
                json!({"type":"task_started","turn_id":"t1"}),
            ),
            codex(
                "00.035",
                "response_item",
                json!({"type":"message","role":"user","content":[]}),
            ),
            codex(
                "05.996",
                "response_item",
                json!({"type":"function_call","name":"exec_command","call_id":"c1"}),
            ),
            codex("06.258", "token_usage_record", json!({"usage":{}})),
            codex(
                "09.213",
                "response_item",
                json!({"type":"function_call_output","call_id":"c1"}),
            ),
            codex("09.214", "event_msg", count(1_000, 200)),
            codex("19.000", "token_usage_record", json!({"usage":{}})),
            codex("19.001", "event_msg", count(2_000, 400)),
            // A count without a record of its own (an older rollout) is not timed.
            codex("30.000", "event_msg", count(3_000, 600)),
        ],
    );
    assert_eq!(durations(&log), [Some(6_223), Some(9_787), None]);
}

#[test]
fn a_kimi_call_runs_from_its_request_to_its_usage_line() {
    let line = |kind: &str, at: i64| json!({"type":kind,"time":1_792_470_000_000i64 + at,"model":"kimi-k2","usageScope":"turn","usage":{"inputOther":10,"output":33}});
    let log = read(
        "/h/.kimi-code/sessions/wd/s1/agents/main/wire.jsonl",
        "kimi",
        &[
            line("llm.request", 73),
            line("usage.record", 2_322),
            // The step's own timing comes after its usage line.
            json!({"type":"context.append_loop_event","time":1_792_470_002_325i64,"event":{"type":"step.end","turnId":"1","llmFirstTokenLatencyMs":1146,"llmStreamDurationMs":1103}}),
            // A usage line with no request of its own is not timed.
            line("usage.record", 9_000),
        ],
    );
    assert_eq!(durations(&log), [Some(2_249), None]);
}

#[test]
fn a_deepseek_call_runs_from_its_step_or_retry_to_its_settled_message() {
    let line = |kind: &str, at: &str, seq: u64, data: Value| json!({"type":kind,"time":format!("2026-10-23T08:00:{at}Z"),"seq":seq,"data":data});
    let message =
        |step: u64| json!({"turn":1,"step":step,"usage":{"inputTokens":100,"outputTokens":300}});
    let log = read(
        "session.v4.jsonl",
        "deepseek",
        &[
            json!({"type":"session","version":4,"id":"d1","createdAt":"2026-10-23T07:59:00Z"}),
            line("step/start", "01.000", 1, json!({"turn":1,"step":1})),
            line("assistant/message", "04.500", 2, message(1)),
            line("tool/call", "04.600", 3, json!({"turn":1,"step":1})),
            line("step/start", "10.000", 4, json!({"turn":1,"step":2})),
            line("llm/retry-started", "12.000", 5, json!({"turn":1,"step":2})),
            line("assistant/message", "15.000", 6, message(2)),
        ],
    );
    assert_eq!(durations(&log), [Some(3_500), Some(3_000)]);
}

#[test]
fn an_opencode_reply_is_timed_only_when_it_called_no_tools() {
    use crate::opencode::transcript::reply_time;
    let time = json!({"created":1_792_470_000_000i64,"completed":1_792_470_003_200i64});
    assert_eq!(reply_time(&json!("stop"), &time), Some(3_200));
    assert_eq!(
        reply_time(&json!("tool-calls"), &time),
        None,
        "its completion waits for its tools"
    );
    assert_eq!(reply_time(&Value::Null, &time), None, "an unfinished reply");
}

#[test]
fn a_devin_call_keeps_the_total_time_its_metrics_record() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE sessions(id TEXT PRIMARY KEY,title TEXT,working_directory TEXT);
         CREATE TABLE message_nodes(row_id INTEGER PRIMARY KEY,session_id TEXT,created_at REAL,chat_message TEXT);
         INSERT INTO sessions VALUES('s','t','/repo');",
    )
    .unwrap();
    let node = |metrics: Value| {
        json!({"message_id":"m","metadata":{"request_id":metrics["id"],"generation_model":"swe-1.5","created_at":"2026-10-23T08:00:00Z","metrics":metrics}}).to_string()
    };
    db.execute(
        "INSERT INTO message_nodes(session_id,created_at,chat_message) VALUES('s',1792470000,?1),('s',1792470001,?2)",
        [
            node(json!({"id":"r1","input_tokens":10,"output_tokens":100,"total_time_ms":2500,"ttft_ms":400})),
            node(json!({"id":"r2","input_tokens":10,"output_tokens":100})),
        ],
    )
    .unwrap();
    let page = crate::devin::transcript::read(&db, &Value::Null).unwrap();
    let times = page
        .records
        .iter()
        .map(|record| record.duration_ms)
        .collect::<Vec<_>>();
    assert_eq!(times, [Some(2_500), None]);
}
