//! OpenClaw JSONL session reducer.
use crate::transcript::{at, n, text};
use crate::{Log, Usage};
use serde_json::Value;

impl Log {
    pub fn openclaw(&mut self, v: &Value) {
        let stem = self
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .split(".jsonl")
            .next()
            .unwrap_or("session")
            .to_owned();
        self.own_id(&stem);
        if let Some(agent) = self
            .path
            .parent()
            .and_then(|dir| dir.parent())
            .and_then(|dir| dir.file_name())
        {
            self.naming
                .fallback(format!("OpenClaw · {}", agent.to_string_lossy()));
        }
        self.archived = self
            .path
            .file_name()
            .is_some_and(|name| name != format!("{stem}.jsonl").as_str());
        if v["type"] == "session" {
            self.cwd = text(&v["cwd"]).map(str::to_owned);
            return;
        }
        if v["type"] == "model_change" {
            if let Some(model) = text(&v["modelId"]) {
                self.model = model.into();
            }
            return;
        }
        let m = &v["message"];
        if v["type"] != "message" || m["role"] != "assistant" || !m["usage"].is_object() {
            return;
        }
        if m["api"] == "openclaw-transcript"
            || text(&m["provider"]).is_some_and(|s| s.eq_ignore_ascii_case("claude-cli"))
            || (m["provider"] == "openclaw"
                && matches!(
                    text(&m["model"]),
                    Some("delivery-mirror" | "gateway-injected")
                ))
            || text(&m["idempotencyKey"]).is_some_and(|s| s.starts_with("codex-app-server:"))
        {
            return;
        }
        let Some(at) = at(&m["timestamp"]).or_else(|| at(&v["timestamp"])) else {
            return;
        };
        let u = &m["usage"];
        let model = text(&m["model"]).unwrap_or("Unknown");
        let provider = text(&m["provider"]).unwrap_or("Unknown");
        let key = text(&m["responseId"])
            .map(|id| format!("response:{provider}:{id}"))
            .or_else(|| text(&v["id"]).map(|id| format!("entry:{id}:{at}")))
            .unwrap_or_else(|| format!("{}:o{}", self.id, self.meter_line_start));
        let out = n(&u["output"]);
        let reasoning = 0;
        self.count_source(
            key,
            &format!("{model}#{provider}"),
            at,
            Usage {
                input: n(&u["input"]),
                output: out - reasoning,
                read: n(&u["cacheRead"]),
                write: n(&u["cacheWrite"]),
                reasoning,
            },
            false,
        );
    }
}
