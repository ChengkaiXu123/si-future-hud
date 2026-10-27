//! Qwen Code JSONL session reducer.
use crate::transcript::{at, n, parts, text};
use crate::{Log, Usage};
use serde_json::Value;
use std::path::Path;

impl Log {
    pub fn qwen(&mut self, v: &Value) {
        let raw = text(&v["sessionId"]).map(str::to_owned).unwrap_or_else(|| {
            self.path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        });
        self.own_id(&raw);
        self.archived = self
            .path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "archive");
        if self.cwd.is_none() {
            self.cwd = text(&v["cwd"]).map(str::to_owned);
        }
        if let Some(at) = at(&v["timestamp"]) {
            self.clock(at);
        }
        if v["type"] == "system" && v["subtype"] == "custom_title" {
            self.naming
                .name(0, text(&v["systemPayload"]["customTitle"]));
        } else if v["type"] == "user" && v["subtype"].is_null() {
            if let Some(prompt) = parts(&v["message"]["parts"]) {
                self.title_from(prompt);
            }
        }
        if !v["forkedFrom"].is_null() || v["type"] != "system" || v["subtype"] != "ui_telemetry" {
            return;
        }
        let e = &v["systemPayload"]["uiEvent"];
        if e["event.name"] != "qwen-code.api_response" {
            return;
        }
        let Some(at) = at(&e["event.timestamp"]).or_else(|| at(&v["timestamp"])) else {
            return;
        };
        let input = n(&e["input_token_count"]);
        let read = n(&e["cached_content_token_count"]);
        if read > input {
            return;
        }
        let output = n(&e["output_token_count"]);
        let thought = n(&e["thoughts_token_count"]);
        let separate = thought > 0
            && n(&e["total_token_count"]) == input.saturating_add(output).saturating_add(thought);
        let key = text(&v["uuid"])
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{raw}:o{}", self.meter_line_start));
        self.count_source(
            key,
            text(&e["model"]).unwrap_or("Unknown"),
            at,
            Usage {
                input: input - read,
                output: if separate { output + thought } else { output },
                read,
                ..Usage::default()
            },
            false,
        );
    }
}
