# c02c — TencentBuddy, Copilot, Qwen, ZCode, Hermes, OpenClaw (sub-audit of c02)

Open: 13 files, 1,414 lines. Counts (70 rows): same 29 · deliberate 0 · better 1 · missing 7 · differs 30 · unclear 3.

## Check of the earlier audit
- "WorkBuddy renamed-database metering": refuted (buddy.rs:303-347, ingestion.rs:973; also tries data/workbuddy.db).
- "Credit attribution": confirmed as a display gap — computed, then discarded except used/size (ingestion.rs:974-980).
- "429 ceilings": refuted — implemented (buddy.rs:44-223, lib.rs:787-801); remaining gaps rows 17-19.
- "GLM-plan deduplication": confirmed missing (row 18).
- "OpenClaw SQLite archives/live state": confirmed missing (row 63).

## Missing / differs / unclear
| # | Open | Behavior | Next | Status | What / when / user sees |
|---|---|---|---|---|---|
| 2 | TencentBuddy/CodeBuddySharedHome.swift:18-25 | Remove legacy CodeBuddy handlers from WorkBuddy settings.json | none | missing | Upgraders: WorkBuddy requests ask the HUD twice |
| 3 | TencentBuddySessions.swift:9,22,136-146 | Sub-agent tokens roll into the parent | buddy.rs:442-461 child sessions; session_breakdown.rs `Family::usage` | fixed | Lower parent totals in lists/HUD; children hidden by rootsOnly. **Fixed 2026-10-21:** the decision that made this deliberate was reversed that day (README); a parent's row, HUD row and page count its child sessions' tokens, and the children stay hidden from lists. |
| 4 | AdditionalLocalStore.swift:29-35 | Cross-file event dedup | none despite lib.rs:716 notice | unclear | Needs real transcripts |
| 5 | TencentBuddySessions.swift:24-79; SessionTitle.swift | Title precedence, first line ≤60 with …; fallbacks cwd folder, "<Vendor> · <8>" | buddy.rs:468-487 | fixed | Raw-id titles; long titles **Fixed 2026-10-20** (session titles). |
| 6 | TencentBuddySessions.swift:25-28 | First non-empty cwd | latest cwd | fixed | Different project if cwd changes **Fixed 2026-10-20** (session titles). |
| 8 | TencentBuddy/Qwen/ZCode | Excluded-record notice; malformed counters exclude the record | silent drop; malformed → 0 | missing | No notice; partial counts |
| 9 | TencentBuddySessions.swift:59-64 | Same response key: larger sum wins | per-field max | differs | Data only, small overcount |
| 11 | TencentBuddySessions.swift:50-56 | API-reached if any call used custom-local | latest route only | differs | "(WorkBuddy)" mark may flip |
| 12 | TencentBuddySessions.swift:187-192 | WorkBuddy models named "<catalog>（WorkBuddy）" | only on ceiling rows, raw id, ASCII | differs | Unmarked models in Statistics |
| 17 | WorkBuddyCeilings.swift:60-63; CopilotClient.swift:65-94 | Short labels; ceiling window duration | buddy.rs `ceiling_quotas`; copilot.rs; lib.rs (removes `windowDurationSeconds` from ceiling rows) | differs | Short labels **fixed 2026-10-20** ("glm-5 · 5h"; "Premium", "Chat", "Completions", "Credits"; unknown Copilot keys title-cased as Open does). Still open: the ceiling rows lose their window duration, so they have no pace. |
| 18 | WorkBuddyCeilings.swift:51-98 | Hide ceiling mapped to a read GLM plan | none | missing | Duplicate 100% row beside GLM plan row |
| 19 | WorkBuddyCeilings.swift:14-69 | Ceiling rows scoped to WorkBuddy account; retire account | no accountId | unclear | Needs HUD grouping for account-less rows |
| 22 | WorkBuddyMetering.swift:45-75; SessionDetailView | Session credits shown | computed and discarded | fixed | No credits anywhere. **Fixed 2026-10-21:** credits and the unattributed part show on the session page. |
| 23 | SessionDetailView.swift:150-156 | Metered context only in detail | contextPercent on rows | differs | Extra context % on WorkBuddy rows |
| 26 | CopilotClient.swift:6-18 | Without consent: silent, history forgotten | English notice; history kept | differs | Extra notice on Copilot source |
| 28 | CopilotClient.swift:19 | "Sign in to GitHub CLI, then refresh quota" (zh too) | different English error; stale rows | differs | Wording; stale rows |
| 31 | CopilotClient.swift:39-42 | Row order premium, chat, completions | JSON map order | differs | Chat/Completions above Premium |
| 32 | CopilotClient.swift:90-93 | Unknown keys capitalized | lowercase | differs | "foo bar" |
| 33 | CopilotClient.swift:69-78 | Free-plan fallback when no metered window | only when snapshots empty | differs | No Copilot rows for some Free plans |
| 35 | CopilotClient.swift:65-66 | No used/capacity | nativeUsage added | differs | Extra consumed figure |
| 36 | CopilotSessions.swift:25-42 | Only session-state/<id>/events.jsonl | walks all, stages other .jsonl as OTel | differs | Extra reads |
| 37 | CopilotSessions.swift:76-77,128-129,202-224 | Title/cwd from workspace.yaml | none | fixed | "copilot:<uuid>" titles, no project **Fixed 2026-10-20** (session titles). |
| 38 | CopilotSessions.swift:98-118 | One turn per prompt, continuation rules | new turn per step; shutdown rewrites outcome | differs | Timer resets each step; outcomes erased; >30 min sub-agent → idle |
| 42 | CopilotSessions.swift:58-59,189-192 | Duplicate span: larger wins across files | latest per file | differs | Data only |
| 43 | CopilotHookFormat.swift:7 | Hook file ~/.copilot/hooks | $COPILOT_HOME/hooks | differs | Possibly better; unverified |
| 48 | QwenSessions.swift:42-57 | Title rules and fallbacks | 100 chars, no skip of tags, id fallback | fixed | "<command-message>…" titles **Fixed 2026-10-20** (session titles). |
| 51 | ZCode/Hermes; ProviderData.swift:41-46 | 8-day window; ZCode 10k cap + notice | whole history, no cap | differs | Older usage in Statistics |
| 52 | ZCodeSessions.swift:23-29 | Re-query rows by completion time | rowid watermark | unclear | Depends on ZCode row timing |
| 54 | ZCode/Hermes | Bad row excluded with notice | whole source page aborts | differs | Source stops updating, English error |
| 55 | ZCodeSessions.swift:44-46 | Title fallbacks | id | fixed | "zcode:<id>" **Fixed 2026-10-20** (session titles). |
| 58 | HermesSessions.swift:62-66 | Title fallback; client label with profile | id; none | differs | "hermes:<id>"; profiles indistinguishable **Title part fixed 2026-10-20**; the rest is open. |
| 59 | WholeFileStore.swift:136-139 | Running-total growth dated when observed | replaced at first_seen | differs | All tokens at first use in Today/hourly/burn |
| 60 | HermesSessions.swift:45-58 | Activity includes last_seen | ignored | differs | Tokens frozen until session ends |
| 63 | OpenClawSessions.swift:33-35,106-172 | openclaw-agent.sqlite sessions, turns, completions | none | missing | Those sessions and usage absent |
| 65 | OpenClawSessions.swift:61,81 | model_change fallback | ignored; model#provider | differs | "Unknown" model; split rows |
| 66 | OpenClawSessions.swift:73-79 | Identity incl. counts, first wins | last wins | differs | Data only |
| 67 | OpenClawSessions.swift:93,99 | Title "OpenClaw · <agent>"; start from header | id | differs | "openclaw:<id>" **Title part fixed 2026-10-20**; the rest is open. |
| 68 | OpenClawSessions.swift:69-71 | Malformed line fails file with notice | skip / 0 | differs | No notice, partial usage |
| 69 | all | Per-session client labels | none | differs | Hermes profiles indistinguishable |
| 70 | ProviderData.swift:19,24-38 | Localized source descriptions; "Qwen" | English implementation notes; "Qwen Code" | differs | Settings source list text |

Better: 5小时 without space accepted. Same (29): roots, token math, model names, ceilings hold/expiry/rows, metering db, titles from db, entry attribution, hooks for CodeBuddy/Copilot/Qwen, Copilot token order/endpoints/snapshots/account/shutdown/OTel, Qwen roots/archive/telemetry, ZCode db/layout, Hermes roots/ids, OpenClaw roots/archives/bookkeeping.

Behaviors summarized above, counted for the ledger totals:

| Behaviors | Status |
| --- | --- |
| Matching behaviors listed under "Same" | same ×29 |
| "5小时" without a space is accepted | better ×1 |

## Tests
All provider tests portable to fixtures except Copilot request/token cases (need HTTP/env/keychain injection) and the consent case (SettingsStore); Hermes running-total dating needs a ledger-level fixture.

## Architecture
- **Serial-rule gap:** `ensure_usage_projection` (ingestion.rs:936-962) reads workbuddy.db and the Claude Desktop index; it is reached from UI requests such as `sessions` (sessions.rs:47-48) that take only the backend lock, not the collection gate.
- Ceiling persistence (non-atomic fs::write) inside snapshot presentation.
- Copilot keychain read spawned `/usr/bin/security` in a shared reader (moved behind os::stored_secret on 10-20).
- CodeBuddy shared-home rule written three times; OpenClaw profile-name rule twice with different first-character rules.
- Credits computed every projection rebuild, then discarded.
- Inaccurate notice lib.rs:716 (cross-file dedup claim).
