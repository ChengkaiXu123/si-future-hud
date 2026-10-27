//! DeepSeek CLI JSONL session & child-session reducer.
use crate::transcript::{at, n, parts, text};
use crate::{Log, Usage};
use serde_json::{json, Value};

impl Log {
    pub fn deepseek(&mut self, v: &Value) {
        let kind = text(&v["type"]).unwrap_or("");
        if kind == "session" {
            let version = n(&v["version"]);
            if ![0, 2, 3, 4].contains(&version) {
                return;
            }
            if let Some(id) = text(&v["id"]) {
                self.own_id(id);
            }
            self.cwd = text(&v["cwd"]).map(str::to_owned);
            self.live.is_subagent = v["origin"] == "subagent" || n(&v["delegationDepth"]) > 0;
            if self.live.is_subagent {
                if let Some(p) = text(&v["parentId"]).or_else(|| text(&v["parentSessionId"])) {
                    self.live.parent_id = Some(format!("deepseek:{p}"));
                }
            }
            self.source_state["format"] = json!(version);
            self.source_state["seedLength"] = v.get("seedLength").cloned().unwrap_or_else(|| {
                json!(if version >= 2 && v["isSeeded"] == true {
                    u64::MAX
                } else {
                    0
                })
            });
            if let Some(at) = at(&v["createdAt"]) {
                self.source_state["createdAt"] = json!(at);
                self.clock(at);
            }
            return;
        }
        if !self.observed {
            return;
        }
        let d = &v["data"];
        if kind == "request/header" || kind == "request/context" {
            let config = if kind == "request/header" {
                &d["header"]["config"]
            } else {
                d
            };
            if let Some(model) = text(&config["model"]) {
                self.model = model.into();
            }
            if let Some(provider) = text(&config["provider"]) {
                self.source_state["provider"] = json!(provider);
            }
        }
        if matches!(
            kind,
            "text-chunks" | "reasoning-chunks" | "tool-call-chunks"
        ) {
            if let (Some(time), Some(gaps), Some(turn)) =
                (v["time0"].as_f64(), d["dt"].as_array(), d["turn"].as_i64())
            {
                if n(&v["seq0"]).saturating_add(gaps.len() as u64)
                    >= n(&self.source_state["seedLength"])
                {
                    let at = (time + gaps.iter().filter_map(Value::as_f64).sum::<f64>()) as i64;
                    self.live.touch_source(&turn.to_string(), at);
                    self.clock(at);
                }
            }
            return;
        }
        let Some(at) = at(&v["time"]) else {
            return;
        };
        let seq = n(&v["seq"]);
        let seed = n(&self.source_state["seedLength"]);
        if kind == "session/end-seed" && seq < seed && d["inherited"] == true {
            self.source_state["seedMarker"] = json!(seq);
            if at >= self.source_state["createdAt"].as_i64().unwrap_or(i64::MAX) {
                self.source_state["seedLength"] = json!(seq);
            }
        } else if seed == u64::MAX
            && n(&self.source_state["format"]) >= 4
            && at >= self.source_state["createdAt"].as_i64().unwrap_or(i64::MAX)
        {
            if let Some(marker) = self.source_state.get("seedMarker").cloned() {
                self.source_state["seedLength"] = marker;
            }
        }
        if seq < n(&self.source_state["seedLength"]) {
            return;
        }
        if !matches!(kind, "session/title" | "session/end-seed") {
            self.clock(at);
            if let Some(turn) = d["turn"].as_i64() {
                self.live.touch_source(&turn.to_string(), at);
            }
        }
        let turn = d["turn"].as_i64().map(|n| n.to_string());
        if kind == "turn/start" && !self.live.is_subagent {
            self.mark(Some(at), crate::transcript::metering::MarkKind::Prompt);
        }
        if kind == "turn/start" {
            if let Some(turn) = turn.as_deref() {
                self.live.observe_source(turn, true, Some(at), at, None);
            }
        }
        if kind == "turn/end" {
            if let Some(turn) = turn.as_deref() {
                let reason = &d["reason"];
                let outcome = match text(&reason["kind"]) {
                    Some("completed") => Some("completed"),
                    Some("error") => Some(if reason["error"]["code"] == "RATE_LIMIT" {
                        "limited"
                    } else {
                        "failed"
                    }),
                    _ => None,
                };
                self.live.observe_source(turn, false, None, at, outcome);
            }
        }
        if kind == "session/title" {
            self.naming.name(0, text(&d["title"]));
            return;
        }
        if kind == "user/message" && d["source"]["kind"] == "user" {
            if let Some(prompt) = parts(&d["content"]) {
                self.title_from(prompt);
            }
        }
        if kind == "llm/retry-started" {
            self.source_state["lastAttempt"] = Value::Null;
        }
        // A step's call is timed from its start, or its latest retry, to the
        // line that settles its message; tool runs follow that line.
        if matches!(kind, "step/start" | "llm/retry-started") {
            self.call_timing.request_at[0] = Some(at);
        }
        if !matches!(
            kind,
            "assistant/chunk" | "assistant/message" | "assistant/attempt"
        ) {
            return;
        }
        let u = d
            .get("usage")
            .filter(|u| u.is_object())
            .or_else(|| {
                d["stream"].as_array()?.iter().rev().find_map(|s| {
                    let c = &s["chunk"];
                    (c["type"] == "usage").then_some(&c["usage"])
                })
            })
            .or_else(|| (d["chunk"]["type"] == "usage").then_some(&d["chunk"]["usage"]));
        let Some(u) = u else {
            return;
        };
        if !u["inputTokens"].is_u64()
            || !u["outputTokens"].is_u64()
            || d["turn"].is_null()
            || d["step"].is_null()
        {
            return;
        }
        let same = kind != "assistant/attempt"
            && self.source_state["lastAttempt"]["turn"] == d["turn"]
            && self.source_state["lastAttempt"]["step"] == d["step"];
        let key = if same {
            self.source_state["lastAttempt"]["key"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        } else {
            let count = n(&self.source_state["attempts"]) + 1;
            self.source_state["attempts"] = json!(count);
            format!("a{count}")
        };
        self.source_state["lastAttempt"] = if kind == "assistant/attempt" {
            Value::Null
        } else {
            json!({"turn":d["turn"],"step":d["step"],"key":key})
        };
        let out = n(&u["outputTokens"]);
        let reasoning = n(&u["reasoningTokens"]).min(out);
        let model = format!(
            "{}#{}",
            self.model,
            self.source_state["provider"].as_str().unwrap_or("Unknown")
        );
        let duration_ms = self.call_timing.since_request(0, at);
        self.count_timed_source(
            key,
            &model,
            at,
            Usage {
                input: n(&u["inputTokens"]),
                output: out - reasoning,
                read: n(&u["cacheReadTokens"]),
                write: n(&u["cacheWriteTokens"]),
                reasoning,
            },
            false,
            duration_ms,
        );
    }
}

/// A DeepSeek turn starts at its `turn/start` line.
pub static TURNS: crate::transcript::turns::Turns = crate::transcript::turns::Turns {
    start: |value| {
        if value["type"] != "turn/start" {
            return None;
        }
        let turn = value["data"]["turn"]
            .as_i64()
            .map(|n| n.to_string())
            .or_else(|| value["data"]["turn"].as_str().map(str::to_owned))?;
        Some(crate::transcript::turns::TurnStart::whole(
            Some(turn),
            value.get("time").or_else(|| value.get("timestamp")),
        ))
    },
    may_name: |line| crate::transcript::turns::has(line, b"turn/start"),
};
