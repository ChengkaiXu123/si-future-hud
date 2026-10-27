//! Grok CLI JSONL session reducer (`updates.jsonl`).
//! Multi-session inference records from `unified.jsonl` are mapped onto the same session.
use crate::transcript::{at, n, text};
use crate::{Log, Usage};
use serde_json::{json, Value};

impl Log {
    pub fn grok(&mut self, v: &Value) {
        if self
            .path
            .file_name()
            .is_none_or(|name| name != "updates.jsonl")
        {
            return;
        }
        if !matches!(
            text(&v["method"]),
            Some("session/update" | "_x.ai/session/update")
        ) {
            return;
        }
        let p = &v["params"];
        let Some(raw) = text(&p["sessionId"]) else {
            return;
        };
        self.own_id(raw);
        self.id = format!("grok:{raw}");
        let meta = &p["_meta"];
        let Some(at) = at(&meta["agentTimestampMs"]) else {
            return;
        };
        self.clock(at);
        let u = &p["update"];
        if let Some(model) = text(&u["_meta"]["modelId"]).or_else(|| text(&meta["modelId"])) {
            self.model = model.into();
        }
        let kind = text(&u["sessionUpdate"]);
        let event = text(&meta["eventId"])
            .map(str::to_owned)
            .unwrap_or_else(|| format!("o{}", self.meter_line_start));
        if kind == Some("user_message_chunk") {
            let turn = text(&meta["promptId"]).unwrap_or(&event);
            self.source_state["turn"] = json!(turn);
            self.live
                .observe_source(turn, true, meta["turnStartMs"].as_i64(), at, None);
        }
        if kind != Some("turn_completed") {
            return;
        }
        let turn = text(&u["prompt_id"])
            .or_else(|| text(&self.source_state["turn"]))
            .unwrap_or(&event)
            .to_owned();
        self.live.observe_source(
            &turn,
            false,
            None,
            at,
            if u["stop_reason"] == "end_turn" {
                Some("completed")
            } else if u["stop_reason"] == "cancelled" {
                Some("interrupted")
            } else {
                None
            },
        );
        if self.source_state["hasInference"] == true {
            return;
        }
        let usage = &u["usage"];
        let input = n(&usage["inputTokens"]);
        let read = n(&usage["cachedReadTokens"]
            .as_u64()
            .map(Value::from)
            .unwrap_or_else(|| usage["cacheReadTokens"].clone()));
        if read > input {
            return;
        }
        let model = usage["modelUsage"]
            .as_object()
            .filter(|o| o.len() == 1)
            .and_then(|o| o.keys().next())
            .unwrap_or(&self.model)
            .to_owned();
        self.count_source(
            format!("{}:{event}", self.id),
            &model,
            at,
            Usage {
                input: input - read,
                output: n(&usage["outputTokens"]),
                read,
                ..Usage::default()
            },
            false,
        );
    }
}
