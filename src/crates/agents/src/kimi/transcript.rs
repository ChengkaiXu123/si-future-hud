//! Kimi JSONL session & child-session reducer.
use crate::transcript::{at, n, text};
use crate::{Log, Usage};
use serde_json::Value;
use std::path::Path;

impl Log {
    pub fn kimi(&mut self, v: &Value) {
        let owned_path = self.path.clone();
        let dir = owned_path.parent().unwrap_or(Path::new(""));
        let modern = dir
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|s| s == "agents");
        let agent = if modern {
            dir.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            "main".into()
        };
        let folder = if modern {
            dir.parent().and_then(Path::parent).unwrap_or(dir)
        } else {
            dir
        };
        self.own_id(&format!(
            "{}:{agent}",
            folder.file_name().unwrap_or_default().to_string_lossy()
        ));
        self.live.is_subagent = agent != "main";
        if modern && agent != "main" {
            self.live.parent_id = Some(format!(
                "kimi:{}:main",
                folder.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        let kind = text(&v["type"]).unwrap_or("");
        if modern {
            if kind == "llm.request" {
                if let Some(model) = text(&v["model"]).filter(|s| !s.starts_with("__")) {
                    self.model = model.into();
                }
                // A call is timed from its request to its usage line. The
                // step's own timing comes after the usage line, in `step.end`.
                self.call_timing.request_at[0] = at(&v["time"]);
                return;
            }
            if let Some(at) = at(&v["time"]) {
                self.clock(at);
                if kind == "context.append_loop_event" && agent == "main" {
                    let e = &v["event"];
                    if let Some(turn) = text(&e["turnId"]) {
                        if e["type"] == "step.begin" {
                            self.live.observe_source(turn, true, Some(at), at, None);
                        } else {
                            self.live.touch_source(turn, at);
                        }
                    }
                }
                if kind == "turn.ended" && agent == "main" {
                    let id = v["turnId"]
                        .as_i64()
                        .map(|n| n.to_string())
                        .or_else(|| text(&v["turnId"]).map(str::to_owned));
                    if let Some(id) = id {
                        let outcome = if !v["error"].is_null() {
                            Some("failed")
                        } else if v["reason"] == "completed" {
                            Some("completed")
                        } else if v["reason"] == "cancelled" {
                            Some("interrupted")
                        } else {
                            None
                        };
                        self.live.observe_source(&id, false, None, at, outcome);
                    }
                }
            }
            if kind != "usage.record" || v["usageScope"] != "turn" {
                return;
            }
            let Some(at) = at(&v["time"]) else {
                return;
            };
            let u = &v["usage"];
            let model = text(&v["model"])
                .filter(|s| !s.starts_with("__"))
                .unwrap_or(&self.model)
                .to_owned();
            let duration_ms = self.call_timing.since_request(0, at);
            self.call_timing.request_at[0] = None;
            self.count_timed_source(
                format!("{}:o{}", self.id, self.meter_line_start),
                &format!("{model}#kimi-code"),
                at,
                Usage {
                    input: n(&u["inputOther"]),
                    output: n(&u["output"]),
                    read: n(&u["inputCacheRead"]),
                    write: n(&u["inputCacheCreation"]),
                    reasoning: 0,
                },
                false,
                duration_ms,
            );
        } else {
            let p = &v["message"]["payload"];
            if v["message"]["type"] != "StatusUpdate" || !p["token_usage"].is_object() {
                return;
            }
            let Some(at) = v["timestamp"]
                .as_f64()
                .filter(|n| *n > 0.0)
                .map(|n| (n * 1000.0) as i64)
            else {
                return;
            };
            let u = &p["token_usage"];
            let key = text(&p["message_id"])
                .map(|id| format!("{}:{id}", self.id))
                .unwrap_or_else(|| format!("{}:o{}", self.id, self.meter_line_start));
            self.count_source(
                key,
                "Unknown#Unknown",
                at,
                Usage {
                    input: n(&u["input_other"]),
                    output: n(&u["output"]),
                    read: n(&u["input_cache_read"]),
                    write: n(&u["input_cache_creation"]),
                    reasoning: 0,
                },
                true,
            );
        }
    }
}

/// A Kimi turn starts at the loop's `step.begin` event.
pub static TURNS: crate::transcript::turns::Turns = crate::transcript::turns::Turns {
    start: |value| {
        if value["type"] != "context.append_loop_event" || value["event"]["type"] != "step.begin" {
            return None;
        }
        let id = value["event"]["turnId"].as_str()?;
        Some(crate::transcript::turns::TurnStart::whole(
            Some(id.to_owned()),
            value.get("time"),
        ))
    },
    may_name: |line| crate::transcript::turns::has(line, b"step.begin"),
};
