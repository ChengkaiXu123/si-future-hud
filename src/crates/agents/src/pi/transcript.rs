//! Pi JSONL session reducer.
use crate::transcript::{at, n, parts, text};
use crate::{Log, Usage};
use serde_json::Value;

impl Log {
    pub fn pi(&mut self, v: &Value) {
        let kind = text(&v["type"]).unwrap_or("");
        let time = at(&v["timestamp"]);
        if kind == "session" {
            if let Some(id) = text(&v["id"]) {
                self.own_id(id);
            }
            self.cwd = text(&v["cwd"]).map(str::to_owned);
            if let Some(at) = time {
                self.clock(at);
            }
            return;
        }
        if !self.observed {
            return;
        }
        if kind == "session_info" {
            // An empty name gives the session back its first prompt's title.
            match text(&v["name"]) {
                Some(name) => self.naming.name(0, Some(name)),
                None => self.naming.clear_name(),
            }
            return;
        }
        if kind == "model_change" {
            if let Some(model) = text(&v["modelId"]) {
                self.model = model.into();
            }
            return;
        }
        let message = &v["message"];
        if kind == "message" && message["role"] == "user" {
            if let Some(prompt) = parts(&message["content"]) {
                self.title_from(prompt);
            }
            return;
        }
        if kind != "message" || message["role"] != "assistant" || !message["usage"].is_object() {
            return;
        }
        let Some(at) = time.or_else(|| at(&message["timestamp"])) else {
            return;
        };
        let u = &message["usage"];
        let model = text(&message["model"]).unwrap_or("Unknown");
        let provider = text(&message["provider"]).unwrap_or("Unknown");
        let key = if let Some(response) = text(&message["responseId"]) {
            format!("pi:response:{provider}:{response}")
        } else if let Some(id) = text(&v["id"]) {
            format!("pi:entry:{id}:{at}:{provider}:{model}")
        } else {
            format!("{}:o{}", self.id, self.meter_line_start)
        };
        self.count_source(
            key,
            &format!("{model}#{provider}"),
            at,
            Usage {
                input: n(&u["input"]),
                output: n(&u["output"]),
                read: n(&u["cacheRead"]),
                write: n(&u["cacheWrite"]),
                reasoning: 0,
            },
            false,
        );
    }
}
