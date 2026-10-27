//! Public native request schemas. Acquisition and transport stay in platform;
//! these records never grant permission or create a tool call by themselves.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
#[derive(Clone)]
pub struct AntigravityPermission {
    pub id: String,
    pub session_id: String,
    pub view: Value,
    pub permission: Value,
    pub cascade_id: String,
    pub trajectory_id: String,
    pub step_index: u32,
}
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|key| (key.clone(), canonical(&object[key])))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        _ => value.clone(),
    }
}
fn record_hash(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(format!("{}:", part.len()));
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}
fn text(value: &Value) -> Option<&str> {
    value.as_str().filter(|s| !s.is_empty())
}
fn clipped(value: &str) -> String {
    value.chars().take(2048).collect()
}
fn index(value: &Value) -> Option<u32> {
    let n = value.as_f64()?;
    (n.is_finite() && n >= 0.0 && n <= u32::MAX as f64 && n.fract() == 0.0).then_some(n as u32)
}
pub fn antigravity_permissions(
    body: &Value,
    service_identity: &str,
    now_ms: i64,
) -> Result<Vec<AntigravityPermission>, String> {
    if !body.is_object() || service_identity.is_empty() {
        return Err("Antigravity permission summary is invalid".into());
    }
    let summaries = match body.get("trajectorySummaries") {
        None | Some(Value::Null) => return Ok(vec![]),
        Some(value) => value
            .as_object()
            .ok_or("Antigravity permission summaries are invalid")?,
    };
    let mut result = Vec::new();
    let mut keys = summaries.keys().collect::<Vec<_>>();
    keys.sort();
    for cascade in keys {
        if cascade.is_empty() {
            continue;
        }
        let summary = &summaries[cascade];
        for waiting in summary["waitingSteps"].as_array().into_iter().flatten() {
            let step = &waiting["step"];
            let metadata = &step["metadata"];
            let source = &metadata["sourceTrajectoryStepInfo"];
            if step["status"] != "CORTEX_STEP_STATUS_WAITING" && step["status"] != 9 {
                continue;
            }
            let Some(permission) = step["requestedInteraction"]["permission"].as_object() else {
                continue;
            };
            let (Some(trajectory), Some(action), Some(target)) = (
                text(&source["trajectoryId"]),
                text(&permission.get("resource").unwrap_or(&Value::Null)["action"]),
                text(&permission.get("resource").unwrap_or(&Value::Null)["target"]),
            ) else {
                continue;
            };
            let step_index = match source.get("stepIndex").filter(|v| v.is_number()) {
                Some(value) => match index(value) {
                    Some(index) => index,
                    None => continue,
                },
                None => 0,
            };
            if waiting
                .get("stepIndex")
                .filter(|v| v.is_number())
                .is_some_and(|value| index(value) != Some(step_index))
                || source
                    .get("cascadeId")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value != cascade)
            {
                continue;
            }
            let index = step_index;
            let workspace = summary["workspaces"][0]["workspaceFolderAbsoluteUri"]
                .as_str()
                .and_then(|s| url::Url::parse(s).ok())
                .filter(|url| url.scheme() == "file")
                .and_then(|url| url.to_file_path().ok())
                .map(|path| path.to_string_lossy().into_owned());
            let tool = metadata["toolCall"]["name"].as_str().unwrap_or(action);
            let description = permission
                .get("actionDescription")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(action);
            let permission = Value::Object(permission.clone());
            let identity=canonical(&json!({"permission":permission,"toolCall":metadata["toolCall"],"workspace":workspace,"executionId":metadata["executionId"]})).to_string();
            let id = format!(
                "antigravity:{service_identity}:{cascade}:{trajectory}:{index}:{}",
                record_hash(&[&identity])
            );
            let session_id = format!("antigravity:{cascade}");
            let summary = clipped(description);
            let detail = clipped(target);
            let view = json!({"id":id,"source":"antigravity","vendor":"Antigravity","sessionId":session_id,"title":summary,"summary":summary,"toolName":tool,"kind":"permission","input":detail,"detail":detail,"cwd":workspace,"createdAt":crate::iso(now_ms),"allowsAlways":false,"questions":[],"answersQuestion":false,"answersPlan":false,"plan":null,"suggestions":[]});
            result.push(AntigravityPermission {
                id,
                session_id,
                view,
                permission,
                cascade_id: cascade.clone(),
                trajectory_id: trajectory.into(),
                step_index: index,
            });
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}
impl AntigravityPermission {
    pub fn response(&self, decision: &Value) -> Result<Option<Value>, String> {
        let allow = match decision["kind"].as_str() {
            Some("leave") => return Ok(None),
            Some("allow") => true,
            Some("deny") => false,
            _ => return Err("Choose this permission option in Antigravity".into()),
        };
        Ok(Some(
            json!({"cascadeId":self.cascade_id,"interaction":{"trajectoryId":self.trajectory_id,"stepIndex":self.step_index,"permission":{"allow":allow,"scope":"PERMISSION_SCOPE_ONCE"}}}),
        ))
    }
}
