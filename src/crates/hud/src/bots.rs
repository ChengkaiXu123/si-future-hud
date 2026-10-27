//! Bot directions: what each agent's Bot shows, decided once for every screen
//! (MarkDirector). Ported from src/packages/hud/src/bots/mark-director.ts.
//!
//! Director times are UTC seconds; a HUD window converts a delivered cue to its
//! own animation clock once.
use crate::{presentation::mark, vendors::vendors};
use serde_json::{json, Map, Value};
use std::{cmp::Ordering, collections::HashMap};

/// A request an agent waits on: a permission, a question or a plan.
#[derive(Clone)]
struct Request {
    id: String,
    session_id: String,
    kind: String,
    at: f64,
}

/// A session as the director reads it.
struct Facts {
    id: String,
    phase: String,
    /// The session's liveFacts; null when its client reports none.
    live: Value,
    observed_at: f64,
    last_event_at: f64,
    requests: Vec<Request>,
}

/// A mark's agent as a snapshot has it.
struct QueueMark {
    key: String,
    is_working: bool,
    sessions: Vec<Facts>,
    requests: Vec<Request>,
    exhausted_until: Option<f64>,
}

/// What a mark's facts ask it to show.
struct Want {
    look: String,
    owner: Option<String>,
    subagents: bool,
    holds: bool,
}

/// A working look waiting out the guard.
struct Wanted {
    look: String,
    since: f64,
}

/// An event a Bot plays once: "start", "failed" or "limit-hit".
#[derive(Clone, Copy)]
struct Cue {
    event: &'static str,
    at: f64,
}

/// What a mark shows, and since when.
struct Mark {
    look: Option<String>,
    owner: Option<String>,
    subagents: bool,
    holds: bool,
    since: f64,
    wanted: Option<Wanted>,
    /// When the working look changed in the last 30 s.
    changes: Vec<f64>,
    guarded: bool,
    resting_since: Option<f64>,
    /// The latest turn end told, by session.
    ended: HashMap<String, f64>,
    cue: Option<Cue>,
}

impl Default for Mark {
    fn default() -> Self {
        Self {
            look: None,
            owner: None,
            subagents: false,
            holds: false,
            since: f64::NEG_INFINITY,
            wanted: None,
            changes: vec![],
            guarded: false,
            resting_since: None,
            ended: HashMap::new(),
            cue: None,
        }
    }
}

/// Decides every mark's Bot from snapshots and time, for all HUD windows.
pub struct Director {
    started_at: f64,
    directions: Map<String, Value>,
    queue: Vec<QueueMark>,
    marks: HashMap<String, Mark>,
    looked_at: HashMap<String, f64>,
}

impl Director {
    /// A director that tells only of turns that end or begin after `started_at`.
    pub fn new(started_at: f64) -> Self {
        Self {
            started_at,
            directions: Map::new(),
            queue: vec![],
            marks: HashMap::new(),
            looked_at: HashMap::new(),
        }
    }

    /// Takes a new snapshot's agents and directs them at `now`.
    pub fn read(&mut self, snapshot: &Value, now: f64) {
        self.queue = bot_queue(snapshot, now);
        self.advance(now);
    }

    /// The user looked at these agents, or at every agent shown: a failure
    /// that ended before now wears no badge.
    pub fn looked(&mut self, keys: Option<&[String]>, now: f64) {
        match keys {
            Some(keys) => {
                for key in keys {
                    self.looked_at.insert(key.clone(), now);
                }
            }
            None => {
                for queue in &self.queue {
                    self.looked_at.insert(queue.key.clone(), now);
                }
            }
        }
        self.advance(now);
    }

    /// Brings every mark's direction up to `now`.
    pub fn advance(&mut self, now: f64) {
        let mut marks = std::mem::take(&mut self.marks);
        let mut directions = Map::new();
        for queue in &self.queue {
            let mut mark = marks.remove(&queue.key).unwrap_or_default();
            let want = want_of(queue, mark.owner.as_deref(), now);
            let resting = mark.resting_since;
            let resting_from = queue
                .sessions
                .iter()
                .map(|session| session.last_event_at)
                .fold(f64::NEG_INFINITY, f64::max);
            let rested = follow(&mut mark, &want, now, resting_from);
            for session in &queue.sessions {
                let outcome = session.live["outcome"].as_str().unwrap_or("");
                let Some(end) = ended(session) else { continue };
                let told = mark.ended.get(&session.id).copied();
                if !outcome.is_empty()
                    && end > self.started_at.max(told.unwrap_or(f64::NEG_INFINITY))
                {
                    mark.ended.insert(session.id.clone(), end);
                    match outcome {
                        "failed" => tell(&mut mark, "failed", now),
                        "limited" => tell(&mut mark, "limit-hit", now),
                        _ => {}
                    }
                }
            }
            mark.ended
                .retain(|id, _| queue.sessions.iter().any(|session| session.id == *id));
            let owned = queue
                .sessions
                .iter()
                .find(|session| mark.owner.as_ref() == Some(&session.id));
            let since = self.started_at.max(resting.unwrap_or(f64::NEG_INFINITY));
            if rested
                && queue.requests.is_empty()
                && owned.is_some_and(|owned| started(owned).unwrap_or(f64::NEG_INFINITY) > since)
            {
                tell(&mut mark, "start", now);
            }
            let look = mark.look.as_deref().unwrap_or("idle");
            let looked_at = self.looked_at.get(&queue.key).copied();
            directions.insert(
                queue.key.clone(),
                json!({
                    "state": {
                        "look": look,
                        "subagents": mark.subagents && is_working(look),
                        "badge": badge_of(queue, looked_at.unwrap_or(f64::NEG_INFINITY), now),
                        "hovered": false,
                        "holds": mark.holds && look == "thinking",
                        "card": "done",
                        "facing": null,
                    },
                    "cue": mark.cue.map(|cue| json!({"event": cue.event, "at": cue.at})),
                }),
            );
            self.marks.insert(queue.key.clone(), mark);
        }
        self.directions = directions;
    }

    /// When the directions next change with no new snapshot, if they will.
    pub fn next_change(&self, now: f64) -> Option<f64> {
        let mut times: Vec<f64> = vec![];
        let mut take = |value: f64| {
            if value > now && value.is_finite() {
                times.push(value);
            }
        };
        for queue in &self.queue {
            if let Some(mark) = self.marks.get(&queue.key) {
                if let Some(wanted) = &mark.wanted {
                    let guard = if mark.guarded { 12.0 } else { 6.0 };
                    take((wanted.since + 2.0).max(mark.since + guard));
                }
            }
            for session in &queue.sessions {
                take(session.last_event_at + 3600.0);
                let call = open_call(session);
                if let Some(call) = call.filter(|call| call["work"] == "shell") {
                    take(time(&call["at"]).unwrap_or(f64::INFINITY) + 20.0);
                }
                let compacting = time(&session.live["activity"]["compactingSince"]);
                take(compacting.unwrap_or(f64::INFINITY) + 300.0);
                if limited(session, now) {
                    take(time(&session.live["limitedUntil"]).unwrap_or(f64::INFINITY));
                }
                if inflight(session) {
                    if call.is_none() {
                        take(tool_seen(session).unwrap_or(f64::INFINITY) + 10.0);
                    }
                    take(session.observed_at + 180.0);
                }
                if session.live["outcome"] == "failed" {
                    take(ended(session).unwrap_or(f64::INFINITY) + 900.0);
                }
            }
            take(queue.exhausted_until.unwrap_or(f64::INFINITY));
        }
        times.into_iter().reduce(f64::min)
    }

    /// What the HUD windows draw: `{startedAt, directions: {mark: {state, cue}}}`.
    pub fn wire(&self) -> Value {
        json!({"startedAt": self.started_at, "directions": self.directions})
    }
}

/// UTC seconds of an RFC 3339 date, to the millisecond as Date.parse reads it;
/// none when absent or unreadable.
fn time(date: &Value) -> Option<f64> {
    let date = chrono::DateTime::parse_from_rfc3339(date.as_str()?).ok()?;
    Some(date.timestamp_millis() as f64 / 1000.0)
}

/// `value ?? fallback`.
fn coalesce<'a>(value: &'a Value, fallback: &'a Value) -> &'a Value {
    if value.is_null() {
        fallback
    } else {
        value
    }
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

/// A JavaScript comparator's number as an ordering: NaN, like 0, decides nothing.
fn order(difference: f64) -> Ordering {
    difference.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
}

fn inflight(session: &Facts) -> bool {
    matches!(session.phase.as_str(), "running" | "waiting")
}

fn started(session: &Facts) -> Option<f64> {
    time(&session.live["turnStartedAt"])
}

fn ended(session: &Facts) -> Option<f64> {
    time(&session.live["endedAt"])
}

fn tool_seen(session: &Facts) -> Option<f64> {
    time(&session.live["activity"]["toolAt"])
}

fn subagents(session: &Facts) -> bool {
    session.live["activity"]["delegating"] == true
}

fn open_call(session: &Facts) -> Option<&Value> {
    Some(&session.live["activity"]["openCall"]).filter(|call| !call.is_null())
}

fn compacting(session: &Facts, now: f64) -> bool {
    time(&session.live["activity"]["compactingSince"]).is_some_and(|at| now - at < 300.0)
}

fn limited(session: &Facts, now: f64) -> bool {
    session.live["outcome"] == "limited"
        && time(&session.live["limitedUntil"]).unwrap_or(f64::INFINITY) > now
}

/// The model is retrying, or the session awaits it with no call, sub-agent or
/// request and nothing observed for 3 minutes.
fn holds(session: &Facts, now: f64) -> bool {
    let activity = &session.live["activity"];
    activity["retrying"] == true
        || activity["awaitsModel"] == true
            && open_call(session).is_none()
            && !subagents(session)
            && session.requests.is_empty()
            && now - session.observed_at >= 180.0
}

/// bot-state.ts `isWorking`.
fn is_working(look: &str) -> bool {
    matches!(look, "compacting" | "thinking" | "plain") || look.starts_with("tool-")
}

/// bot-state.ts `isCalm`.
fn is_calm(look: &str) -> bool {
    matches!(look, "no-news" | "idle" | "asleep")
}

/// bot-state.ts `eventRank`.
fn event_rank(event: &str) -> u8 {
    match event {
        "failed" => 2,
        "limit-hit" => 1,
        _ => 0,
    }
}

/// The look a session's activity asks for: its open call's tool (a shell call
/// 20 s on is a long command), a tool seen under 10 s ago, else thinking.
fn look_of(session: &Facts, now: f64) -> &'static str {
    let activity = &session.live["activity"];
    if activity.is_null() {
        return "plain";
    }
    if activity["retrying"] == true {
        return "thinking";
    }
    let call = &activity["openCall"];
    if call["work"] == "shell" && now - time(&call["at"]).unwrap_or(now) >= 20.0 {
        return "tool-long-command";
    }
    let tool = match coalesce(&call["work"], &activity["work"]).as_str() {
        Some("shell" | "command") => "tool-command",
        Some("edit") => "tool-edit",
        Some("read") => "tool-read",
        _ => "thinking",
    };
    if tool == "thinking"
        || !call.is_null()
        || now - tool_seen(session).unwrap_or(f64::NEG_INFINITY) < 10.0
    {
        return tool;
    }
    "thinking"
}

/// The session a look follows: the current owner while it qualifies, else the
/// latest observed, then the latest turn, then the smaller id.
fn owner_of<'a>(sessions: &[&'a Facts], current: Option<&str>) -> Option<&'a Facts> {
    let owner = sessions
        .iter()
        .copied()
        .find(|session| Some(session.id.as_str()) == current);
    owner.or_else(|| {
        sessions.iter().copied().min_by(|a, b| {
            let turn = |session: &Facts| started(session).unwrap_or(f64::NEG_INFINITY);
            order(b.observed_at - a.observed_at)
                .then(order(turn(b) - turn(a)))
                .then_with(|| a.id.cmp(&b.id))
        })
    })
}

/// What a mark's facts ask it to show: the weightiest request, a compaction,
/// the owner's work (a tooling session's when the owner only thinks), else how
/// the agent rests.
fn want_of(mark: &QueueMark, owner: Option<&str>, now: f64) -> Want {
    let rank = |kind: &str| match kind {
        "permission" => 2.0,
        "question" => 1.0,
        "plan" => 0.0,
        _ => f64::NAN,
    };
    let request = mark.requests.iter().min_by(|a, b| {
        order(rank(&b.kind) - rank(&a.kind))
            .then(order(a.at - b.at))
            .then_with(|| a.id.cmp(&b.id))
    });
    if let Some(request) = request {
        return Want {
            look: request.kind.clone(),
            owner: Some(request.session_id.clone()),
            subagents: false,
            holds: false,
        };
    }
    let compacts: Vec<&Facts> = mark
        .sessions
        .iter()
        .filter(|s| compacting(s, now))
        .collect();
    if let Some(compact) = owner_of(&compacts, owner) {
        return Want {
            look: "compacting".into(),
            owner: Some(compact.id.clone()),
            subagents: subagents(compact),
            holds: false,
        };
    }
    let busy: Vec<&Facts> = mark.sessions.iter().filter(|s| inflight(s)).collect();
    if let Some(mut session) = owner_of(&busy, owner) {
        let mut look = look_of(session, now);
        if look == "thinking" {
            let tooling: Vec<&Facts> = busy
                .iter()
                .copied()
                .filter(|s| look_of(s, now).starts_with("tool-"))
                .collect();
            if let Some(tooling) = owner_of(&tooling, None) {
                session = tooling;
                look = look_of(tooling, now);
            }
        }
        return Want {
            look: look.into(),
            owner: Some(session.id.clone()),
            subagents: subagents(session),
            holds: holds(session, now),
        };
    }
    let look = if mark.is_working {
        "plain"
    } else if mark.sessions.iter().any(|s| s.phase == "unverified") {
        "no-news"
    } else if mark
        .sessions
        .iter()
        .all(|s| now - s.last_event_at >= 3600.0)
    {
        "asleep"
    } else {
        "idle"
    };
    Want {
        look: look.into(),
        owner: None,
        subagents: false,
        holds: false,
    }
}

/// A failure under 15 minutes old that ended after the last look, with no turn
/// begun since; else a limit with no turn begun since, or a window used up.
fn badge_of(mark: &QueueMark, looked_at: f64, now: f64) -> Option<&'static str> {
    let prompted = mark
        .sessions
        .iter()
        .map(|session| started(session).unwrap_or(f64::NEG_INFINITY))
        .fold(f64::NEG_INFINITY, f64::max);
    let failed = mark.sessions.iter().any(|session| {
        let end = ended(session).unwrap_or(f64::NEG_INFINITY);
        session.live["outcome"] == "failed"
            && now - end < 900.0
            && end > looked_at
            && prompted <= end
    });
    if failed {
        return Some("failed");
    }
    let limit = mark.sessions.iter().any(|session| {
        limited(session, now) && prompted <= ended(session).unwrap_or(f64::INFINITY)
    }) || mark.exhausted_until.unwrap_or(f64::NEG_INFINITY) > now;
    limit.then_some("limited")
}

/// Each mark's agent in a snapshot, in the Dynamic dock's order: its sessions
/// heard from in the last day, its requests, and when its used-up windows reset.
fn bot_queue(snapshot: &Value, now: f64) -> Vec<QueueMark> {
    vendors(snapshot, "logos", (now * 1000.0) as i64)
        .into_iter()
        .map(|vendor| {
            let requests: Vec<Request> = snapshot["approvals"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| mark(item["source"].as_str().unwrap_or("")) == vendor.key)
                .map(|item| Request {
                    id: text(&item["id"]),
                    session_id: text(coalesce(&item["sessionId"], &item["id"])),
                    kind: item["kind"].as_str().unwrap_or("permission").to_owned(),
                    at: time(&item["createdAt"]).unwrap_or(0.0),
                })
                .collect();
            let sessions: Vec<Facts> = vendor
                .sessions
                .iter()
                .filter(|session| {
                    let last =
                        coalesce(&session["liveFacts"]["lastEventAt"], &session["updatedAt"]);
                    time(last).unwrap_or(0.0) >= now - 86400.0
                })
                .map(|session| {
                    let id = text(&session["id"]);
                    let live = session["liveFacts"].clone();
                    let mut own: Vec<Request> = live["requests"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|item| Request {
                            id: text(&item["id"]),
                            session_id: text(&item["sessionId"]),
                            kind: text(&item["kind"]),
                            at: time(&item["at"]).unwrap_or(0.0),
                        })
                        .collect();
                    for request in requests.iter().filter(|request| request.session_id == id) {
                        if !own.iter().any(|item| item.id == request.id) {
                            own.push(request.clone());
                        }
                    }
                    Facts {
                        phase: text(&session["state"]),
                        observed_at: time(coalesce(&live["observedAt"], &session["updatedAt"]))
                            .unwrap_or(0.0),
                        last_event_at: time(coalesce(&live["lastEventAt"], &session["updatedAt"]))
                            .unwrap_or(0.0),
                        requests: own,
                        id,
                        live,
                    }
                })
                .collect();
            // Each request once, where it first appears, as its last listing has it.
            let mut merged: Vec<Request> = vec![];
            for item in requests
                .iter()
                .chain(sessions.iter().flat_map(|s| &s.requests))
            {
                match merged.iter_mut().find(|kept| kept.id == item.id) {
                    Some(kept) => *kept = item.clone(),
                    None => merged.push(item.clone()),
                }
            }
            let exhausted_until = vendor
                .quotas
                .iter()
                .filter(|quota| {
                    quota["status"] != "stale"
                        && quota["usedPercent"].as_f64().unwrap_or(0.0) >= 99.5
                })
                .filter_map(|quota| time(&quota["resetAt"]))
                .reduce(f64::max);
            QueueMark {
                key: vendor.key,
                is_working: vendor.working > 0,
                sessions,
                requests: merged,
                exhausted_until,
            }
        })
        .collect()
}

/// Moves a mark toward what its facts want. A change to or from attention, rest
/// or compaction, or to another session, shows at once; another working look
/// shows once wanted 2 s and 6 s after the last change, 12 s while 4 changes in
/// 30 s guard it. True when work follows a rest: asleep, or idle 5 minutes.
fn follow(mark: &mut Mark, want: &Want, now: f64, resting_from: f64) -> bool {
    let Some(shown) = mark.look.clone() else {
        show(mark, want, now);
        if is_calm(&want.look) {
            mark.resting_since = Some(resting_from);
        }
        return false;
    };
    if mark.guarded && now - mark.changes.last().copied().unwrap_or(now) >= 30.0 {
        mark.guarded = false;
        mark.changes.clear();
    }
    if want.look == shown {
        mark.wanted = None;
        show(mark, want, now);
        return false;
    }
    if !is_working(&want.look)
        || !is_working(&shown)
        || want.look == "compacting"
        || shown == "compacting"
        || want.owner != mark.owner
    {
        let rested = shown == "asleep"
            || shown == "idle" && now - mark.resting_since.unwrap_or(now) >= 300.0;
        show(mark, want, now);
        return is_working(&want.look) && rested;
    }
    let since = match &mark.wanted {
        Some(wanted) if wanted.look == want.look => wanted.since,
        _ => {
            mark.wanted = Some(Wanted {
                look: want.look.clone(),
                since: now,
            });
            now
        }
    };
    if now - since >= 2.0 && now - mark.since >= if mark.guarded { 12.0 } else { 6.0 } {
        show(mark, want, now);
    } else {
        mark.subagents = want.subagents;
        mark.holds = want.holds;
    }
    false
}

/// Shows what a mark wants. A new working look counts as a change; a calm look
/// after any other begins a rest.
fn show(mark: &mut Mark, want: &Want, now: f64) {
    if is_working(&want.look) {
        mark.owner = want.owner.clone();
        if mark.look.as_deref() != Some(want.look.as_str()) {
            mark.since = now;
            mark.changes.retain(|at| now - at < 30.0);
            mark.changes.push(now);
            if mark.changes.len() >= 4 {
                mark.guarded = true;
            }
        }
    } else if is_calm(&want.look) {
        mark.owner = None;
        if !mark.look.as_deref().is_some_and(is_calm) {
            mark.resting_since = Some(now);
        }
    }
    mark.look = Some(want.look.clone());
    mark.wanted = None;
    mark.subagents = want.subagents;
    mark.holds = want.holds;
}

/// Plays an event, unless a weightier one went out under 3 s ago.
fn tell(mark: &mut Mark, event: &'static str, now: f64) {
    if mark
        .cue
        .is_some_and(|cue| now - cue.at < 3.0 && event_rank(cue.event) > event_rank(event))
    {
        return;
    }
    mark.cue = Some(Cue { event, at: now });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Numbers within 1e-9; everything else equal.
    fn same(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => {
                (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() <= 1e-9
            }
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
            }
            (Value::Object(a), Value::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(key, a)| b.get(key).is_some_and(|b| same(a, b)))
            }
            _ => a == b,
        }
    }

    /// director-cases.json holds steps the TypeScript MarkDirector took, with
    /// what it published and when it would next change after each.
    #[test]
    fn directs_as_the_typescript_director_did() {
        let cases: Value = serde_json::from_str(include_str!("bots/director-cases.json")).unwrap();
        let mut steps = 0;
        for case in cases["cases"].as_array().unwrap() {
            let mut director = Director::new(case["startedAt"].as_f64().unwrap());
            for (index, step) in case["steps"].as_array().unwrap().iter().enumerate() {
                let now = step["now"].as_f64().unwrap();
                if let Some(snapshot) = step.get("read") {
                    director.read(snapshot, now);
                } else if let Some(keys) = step.get("looked") {
                    let keys: Option<Vec<String>> = serde_json::from_value(keys.clone()).unwrap();
                    director.looked(keys.as_deref(), now);
                } else {
                    director.advance(now);
                }
                let place = format!("{} step {index}", case["name"]);
                let wire = director.wire();
                assert!(
                    same(&wire, &step["wire"]),
                    "{place}: published\n{wire}\nnot\n{}",
                    step["wire"]
                );
                let next = director.next_change(now);
                assert!(
                    same(&json!(next), &step["nextChange"]),
                    "{place}: next change {next:?}, not {}",
                    step["nextChange"]
                );
                steps += 1;
            }
        }
        assert!(steps >= 150);
    }
}
