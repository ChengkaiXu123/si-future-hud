# c08-bots: procedural bots, the Clawd sprite player, bot direction memory, mark rendering

## Header

**Open files covered:** 26 files, 6,294 lines, all under `OPEN/Sources/AgentHUDDesktop/Bots`.
- Top level: BotMotion 929, BotRenderer 633, BotActor 543 (also holds BotClock, BotMark, StillMark and BotStageView), BotForm 395, BotSpec 328, BotState 222, BotMoves 171, BotSigns 167, BotHairline 123, BotStill 111, BotFins 103, BotCanvas 45.
- Clawd/: ClawdPlayer 573, ClawdSpriteTable 311, ClawdPerformer 235, ClawdBaker 213, ClawdActor 208, ClawdClip 160, ClawdClips 146, ClawdSheet 118, ClawdSprites 63, ClawdSheetSource 45.
- Direction/: MarkDirector 251, MarkDirector+Wants 110, MarkDirections 61, DockMemory 30.

Context read only, not audited here: AgentHUDCore `SessionLiveFacts.swift`, `SessionActivity.swift`, `ReportView+Marks.swift`, `Notch/Alerts/ReminderMark.swift`, `Notch/Island/LogoQueueView.swift` (bot call sites), `Components/AgentLogo.swift` (template marks), `IslandAnimation.swift`.

**Next files consulted:**
- `src/packages/ui/src/hud/bots/`: every file (bot-state, bot-signs, bot-motion, bot-moves, bot-fins, bot-spec, bot-form, bot-renderer, bot-hairline, bot-still, bot-actor, procedural, artwork, clawd, clawd-clips, clawd-player, clawd-performer, clawd-actor, mark-director).
- `src/packages/ui/src/hud/`: BotMark.tsx, agent-artwork.ts, alerts/ReminderMark.tsx, scene/HUDMarkStrip.tsx.
- `src/crates/hud/src/bots.rs` (the director, in the host since 2026-10-21), `src/crates/host/src/coordinator.rs` (its timer and looks), `src/apps/desktop/src/lib/use-bot-directions.ts`, `src/apps/desktop/src/HUDSurface.tsx` (`hud_bot_looked`).
- `src/packages/protocol/src/index.ts` (SessionLiveFacts).
- `src/crates/agents/src/transcript/live.rs` (how `openCall` is serialized).

## Summary

The pure engines are faithful line-for-line ports. That covers BotMotion, BotMoves, BotFins, BotSigns, BotSpec geometry, BotForm, BotRenderer, ClawdPlayer, ClawdPerformer, ClawdClips, ClawdBaker and MarkDirector. Their constants, curves, seeded random numbers (SplitMix, bit-exact through BigInt) and state transitions all match.

The problems sit at the edges: the hosts and call sites (BotMark/ReminderMark/HUDMarkStrip), one light-ground spec entry, the still-logo path, and the live-facts derivation that Next re-implements inside the director (`bots.rs` since 2026-10-21, mark-director.ts before).

**Status counts (49 rows):**

| Status | Rows |
|---|---|
| same | 34 |
| differs | 13 |
| missing | 0 |
| unclear | 1 |
| deliberate | 0 |
| better | 0 |

Row 36 is fixed (2026-10-21).

**Top 10 most user-visible problems:**

1. **Reminder bots skip their arrival.** Next's ReminderMark builds every reminder bot with `awake` and with `arrivesAfter` 0.
   - A request's procedural bot shows up already raised, with no logo phase, no morph and no hop.
   - Done and reset cards morph at once instead of holding the logo for 0.7 s.
   - Clawd appears without the logo-to-Clawd fade.
   - Row 30.
2. **A running sub-agent shows as thinking.** In Next, a turn that runs a sub-agent (Task/agents call) or a question call after a tool shows **thinking** instead of the kept tool look. The cause: Rust's `openCall` falls back to a non-shown call, and the TS director reads that call's `work`. Row 35.
3. **Still signs sit too low.** On still logos (bots off, Reduce Motion, the exhausted-quota reminder logo), Next places the sign from the bot body's top, not the logo's top. The sign sits lower and overlaps the artwork. Row 8.
4. **Grok's eyes vanish on a light HUD.** Next lacks the light-ground eye cut (#f5f5f7), so the eyes are drawn island-dark on a #1d1d1f body. Row 18.
5. **The no-news still logo is not faded.** With bots off or Reduce Motion, a no-news mark should show at half strength. Next shows it at full strength. Row 28.
6. **Vendors with no bundled artwork lose their sign.** Next shows only the HUDArtwork fallback, with no request (`!` `?` `≡`) or badge (`×` `⧗`) sign. Open shows StillMark with the sign. Row 29.
7. **Dynamic-dock bots differ from Open's logo queue.** In Next they have no 1-px hairline, their breathing ignores the "Working period" setting, and they replay the logo-to-bot morph whenever the dock is rebuilt (`awake` is never passed). Row 37.
8. **Reminder and sample bots aim at the pointer differently.** Next takes the aim only from pointer moves over the 17-pt mark itself, normalised by the mark's width (far too strong) and sticky after the pointer leaves. Open follows the global pointer with a d/(d+120) falloff. Next also marks these bots hovered on pointer-enter. Rows 32 and 33.
9. **Switching the HUD between light and dark is abrupt.** Open rebuilds the bot and cross-fades it (0.4 s). Next flashes the static HUDArtwork fallback, then pops the canvas. Row 31.
10. **Event cues expire on a different clock.** Next counts the 3-s window for an event to begin from the event's own time, not from when the mark heard it. Its per-window `epochOffset` (Date.now − performance.now) is captured once, so after a system sleep, cue times land in the future and pending events are not let go for sleep+3 s. Row 34.

The Clawd sign halo (row 26) is also off, but only faintly.

## Table

| # | Open (file:line) | Behavior | Next (file:line) | Status | Detail (what / when / user sees) |
|---|---|---|---|---|---|
| 1 | BotState.swift:7-55 | BotLook cases in merge order; isAttention/isWorking/isCalm/staysInPlace | bot-state.ts:2-16,85-94 | same | Tool looks are flattened to `tool-*` strings; the predicates are equivalent. |
| 2 | BotState.swift:57-113 | BotTool, BotEvent (freshness 3 s, rank failed>limit>start), BotBadge, BotCard (longTurn 600 s, `reply(from:to:)`) | bot-state.ts:17-20,45-46,95-96,153-157 | same | `cardForReply` takes seconds; ReminderMark.tsx:39-46 passes seconds. |
| 3 | BotState.swift:116-138 | BotFacing from dock edge; aim ±0.6 | bot-state.ts:141-152 | same | |
| 4 | BotState.swift:141-185,195-222 | BotState fields, phase↔state, `stillSign` | bot-state.ts:31-39,72-140 | same | |
| 5 | BotSigns.swift:7-53 | 7 glyphs on a 5×7 grid, winning order, badge→sign | bot-signs.ts:9-17; bot-state.ts:63-105 | same | |
| 6 | BotSigns.swift:56-117 | Cell = round(side/20) capped to fit 0.3·side; dark ground: 1-px edge at 60 %, faint dots at 0.35 (edge 0.21); light ground: #0d0d0f ink | bot-signs.ts:18-81 | same | The edges are drawn as one union path, as in Open. |
| 7 | BotSigns.swift:127-144 | Sign box over the bot: foot (centre.x+0.14, top−0.04), ceiling −0.28·side, kept 1 px inside the right edge | bot-signs.ts:82-107 | same | |
| 8 | BotSigns.swift:150-156; BotActor.swift:403,424 | The still logo's sign is measured from the **logo's** top (`logoTop−0.04`) | BotMark.tsx:225-233 → bot-renderer.ts:764-776 → bot-signs.ts:95-107 | differs | **What:** with a bot form, Next draws the still sign with `signBox` (the bot's `form.top`). The `logoTop` version (`stillSignImage`) is used only when there is no form (BotMark.tsx:242-249). **When:** bots off, Reduce Motion, the exhausted-quota reminder logo (`enabled={false}`). **User sees:** the `!` `?` `≡` `×` `⧗` sign sits lower and overlaps the logo. For Claude, the sign's foot moves about 0.19 of the side down. |
| 9 | BotMotion.swift:55-80 | BotPace (1/24, 1/12, 1/6, slack 4 ms; next/frame/onFrame) | bot-motion.ts:83-100 | same | |
| 10 | BotMotion.swift:84-356 | Motion state machine: morph 0.5 s, blend 0.3 s, done rests after 2.4 s, arrival delay, events pending ≤3 s then playing, greet hop after 0.3 s, acts, dt clamp 1/6 with gap >1/3 → 0 | bot-motion.ts:162-439 | same | Next also clears `settled` in `set`/`play`. That is not visible: Open's host steps at once anyway. |
| 11 | BotMotion.swift:371-419,421-487 | Frame scheduling (`wait`), visits (idle 20–35 s, asleep 180–300 s, 3–5 s long), `acts`, `greetsOnHover`, `turnsToCard`, hover handling | bot-motion.ts:451-601 | same | |
| 12 | BotMotion.swift:489-587 | Poses: request hop 0.45 s, raise easeOutBack, knock/tilt/nod, done pop, breathing, thinking tilt, act poses; event poses; fin flaps | bot-motion.ts:602-700 | same | |
| 13 | BotMotion.swift:591-739 | Face targets, blink ranges, eye/head easing rates (18/9/60), kept-piece pulse, sign choice and lit dot | bot-motion.ts:701-854 | same | |
| 14 | BotMotion.swift:749-903 | Clawd limb stance (spinner frames, typing taps, waits, dance, doze), hop path, event stances, act pool weights and lengths | bot-motion.ts:855-1041 | same | |
| 15 | BotMotion.swift:907-929 | easeInOut/easeOut/easeOutBack; SplitMix64 | bot-moves.ts:2-6; bot-motion.ts:101-120 | same | Bit-exact through BigInt, so seeded replays match. |
| 16 | BotMoves.swift:6-171 | Every curve and constant: ponder 2.5 s, beat 0.125, burst 3/4.5, busySpin 14, move schedule 1.2/2.6/8 after 120 s, knock/tilt/nod, hop 0.3, shake/sag/sigh, event lengths | bot-moves.ts:8-126 | same | |
| 17 | BotFins.swift:6-102 | BotFlap rounding, fin path, silhouette, paddle 14°, flap 32°, wave 0.9 s, cheer | bot-fins.ts:3-58; bot-moves.ts:127-145; bot-motion.ts:699 | same | |
| 18 | BotSpec.swift:156-164,171-183; AgentLogo.swift:53-54 | On a light ground, every **template** mark (Grok, Cursor, Kimi, Pi, Copilot, Hermes, ZCode) and every swapped mark gets eyeColor = lightCut #f5f5f7 | bot-spec.ts:184-189; bot-renderer.ts:321 | differs | **What:** Next's `grok` entry has no light `eyeColor`. Its artwork is tinted #1d1d1f on light (agent-artwork.ts:114-121), so its eyes fall back to ink #0d0d0f. **When:** Grok / Grok CLI mark on the light HUD theme. **User sees:** a dark ball with almost invisible eyes. The other 19 marks match. |
| 19 | BotSpec.swift:219-323 | Spec table: 20 marks' radius, centre, reach, shape, paint, eyes, decor, limbs, spin, keeps, ownEyes, settle, signAt; light-ink swaps | bot-spec.ts:78-392 | same | Checked entry by entry, including the OpenCode/Qoder/WorkBuddy/CodeBuddy/Hermes/Pi/Devin/Grok Bot swaps. |
| 20 | BotForm.swift:52-107 | One form per mark and ground; template artwork stencilled in the ground's ink; ChatGPT green; OpenCode/Qoder light and dark files | bot-form.ts:260-314; artwork.ts:7-37; agent-artwork.ts:97-123 | same | |
| 21 | BotForm.swift:136-285 | 360 reach samples at 256 px, body reach per shape (outline smoothing, polygon, rect/slant, cat, bob), morph outline blended per degree with turn, `top`/`logoTop` | bot-form.ts:3-128,279-339 | same | |
| 22 | BotForm.swift:292-394 | Colour field: 40→60 grid, uniform-colour swatch (<14), row/column spread, 3 box-blur passes | bot-form.ts:131-244 | same | Uint8Clamped rounds where Open truncates. That is at most ±1 per channel and not visible. |
| 23 | BotRenderer.swift:12-107,133-154 | Margin 0.3, Look quantisation (morph 60, turn 0.5, …), image/logo/bare frames, ownEyes plus-lighter, morph shares (solid /0.22, dressed 0.45–0.9) | bot-renderer.ts:18-89,432-443,666-688,739-776 | same | |
| 24 | BotRenderer.swift:158-404 | Still part (artwork fade /0.4, settle, ring two passes, feelers, fins, paint, patch, stroke), sub-agent copy (0.4 at +0.42/+0.18, 0.035 gap), limbs, spinner shapes | bot-renderer.ts:444-665,211-280 | same | |
| 25 | BotRenderer.swift:407-633 | Decor (blocks/cut/notch/line), soft wear, gradients, ball/flat eyes, crossed eyes, smile, pill | bot-renderer.ts:90-210,281-431 | same | |
| 26 | ClawdSprites.swift:60-62 → BotSigns.swift:93-103 | Clawd's sign painted by the shared painter: edge as one union path at 60 % | clawd.ts:332-362 | differs | **What:** Next's `drawSign` fills one 60 % edge rect per ink cell. Where neighbouring edges overlap, the alpha compounds to about 84 %. **When:** any Clawd sprite sign on a dark ground (`!` `?` `×` `⧗` `z`). **User sees:** faint darker specks in the sign's halo between cells. |
| 27 | BotHairline.swift:21-69; BotCanvas.swift; BotStill.swift | Hairline: max filter, alpha ×3, half strength behind, dark or light ink. Kept bitmaps. Still-part cache (36 slots, second identical frame) | bot-hairline.ts:2-37; bot-still.ts:15-88; bot-renderer.ts:728-738 | same | The output is equivalent. Next allocates a canvas per frame and runs the hairline in JS; that costs performance only. |
| 28 | BotActor.swift:403,415-421 | StillMark draws a no-news logo at 0.5 opacity; its sign stays at full strength | BotMark.tsx:172,212,258 | differs | **What:** in still mode Next sets the canvas opacity to 1. **When:** bots off or Reduce Motion while a session is unverified (no news). **User sees:** a full-strength logo where Open fades it. This is covered by Open's test `BotQueueTests.testNoNewsFadesTheStillLogoToo`. |
| 29 | BotActor.swift:398-404 | A mark with no BotForm (no bundled artwork) shows StillMark: AgentLogo plus the still sign | BotMark.tsx:122-123,394 | differs | **What:** `loadArtwork` returns null, so the effect returns before `ready`. Only `HUDArtwork` renders, with no sign. **When:** any agent whose mark is not among the 20 bundled ones, while a request waits or a badge stands. **User sees:** no `!` `?` `≡` `×` `⧗` over that agent's logo. |
| 30 | BotActor.swift:373,385-395,490; Alerts/ReminderMark.swift:40 | A reminder's BotMark defaults to staysBot = true, arrivesAfter = `IslandAnimation.duration+0.3` = 0.7 s, awake = false. It opens as the logo, then morphs (0.5 s) with the hop and raise | Alerts/ReminderMark.tsx:26-34,48-56,61-69; BotMark.tsx:44-46,64-71; bot-actor.ts:53-57 | differs | **What:** Next passes `awake` and leaves `arrivesAfter` at 0, so a procedural request bot is resumed: morph = 1, age 1.2 s, no hop, no raise animation. Done and reset cards morph immediately. Clawd's actor gets `resumes`, so it has no logo-to-Clawd fade. **When:** every request, reply, reset and credit reminder card. **User sees:** the bot is already there and raised as the card opens, instead of the logo turning into the bot after the card has opened. This contradicts Open's tests `BotMotionTests.testAReminderOpensAsItsLogoAndTurnsIntoItsBotOnceItArrives` and `BotQueueTests.testAReminderMarkOpensAsItsLogo…`. |
| 31 | BotActor.swift:480-496 | A ground change (light↔dark HUD) builds a new bot and cross-fades it in (fadeLook 0.4 s) | BotMark.tsx:109-113,347,394 | differs | **What:** Next keeps the same actor, sets `ready=false` (the HUDArtwork fallback shows), reloads the artwork, then shows the canvas. There is no cross-fade. **When:** switching the HUD appearance. **User sees:** a brief swap to the static logo, then a hard cut to the bot. |
| 32 | BotActor.swift:534-541; BotMotion.swift:360-365 | The aim is toward the **global** pointer from the mark's centre, scaled by d/(d+120) | BotMark.tsx:366-388,166; HUDMarkStrip.tsx:380-404 | differs | **What:** with no `aim` prop (reminders, Settings samples), Next uses the pointer's offset inside the mark divided by the mark's width, clamped to ±1. It updates only while the pointer moves over the mark and keeps the last value after it leaves. Docks pass a parent aim with the same formula as Open, so docks match. **When:** reminder bots looking at the pointer, e.g. a done card after its smile, or a hovered request. **User sees:** reminder bots don't follow the pointer around the screen; when hovered their eyes swing almost fully sideways. |
| 33 | BotState.swift:146-148 (hovered comes only from the caller) | Reminder states are never hovered | BotMark.tsx:74-87,362-365,389-392 | differs | **What:** if the caller passes no `hovered`, Next sets `hovered` on pointer-enter. **When:** hovering a reminder card's mark. **User sees:** the request bot's eyes jump to the local aim, and Clawd schedules a greeting, which it skips because the look isn't calm. Open's reminder bot keeps looking ahead. |
| 34 | BotActor.swift:502-506; LogoQueueView.swift:308-310 | Freshness gate on `cue.at`, then `play(at: now)`: the event has 3 s to begin from when the mark heard it | bot-actor.ts:74-85; clawd-performer.ts:135-147; use-bot-directions.ts:23,92-97 | differs | **What:** Next passes `cue.at` itself as the asked time, so the 3-s window starts at the event, not at delivery. `epochOffset` is captured once per window, so after macOS sleep (performance.now pauses, Date.now doesn't), later cues convert to future times and a pending event is not let go for sleep+3 s. **When:** late delivery; after a system sleep, an event that arrives during a request's look. **User sees:** usually nothing. After sleep, a failure/limit/start animation can play long after the moment has passed. |
| 35 | MarkDirector+Wants.swift:62-77; SessionLiveFacts.swift:109-123; SessionActivity.swift:138-141 | Look of a working session: `openCall` is the newest *shown* call, else the kept shown `work`; `hasOpenCall` counts any open call | bots.rs `look_of`; agents/src/transcript/live.rs:462-463 | differs | **What:** Rust sets `openCall` to the last shown call, *else the last call of any kind*. The director's `look_of` then uses that call's `work` (`agents`/`question`) and turns it into thinking. Open uses the kept shown kind (e.g. edit) with hasOpenCall = true. **When:** a Claude Code turn that used a tool and now waits on a Task/sub-agent (or AskUserQuestion) call with nothing else open. **User sees:** thinking (dots, or Clawd's Thinking clip) for the whole sub-agent run instead of the tool look (bobbing, or Clawd at the laptop/reading). |
| 36 | MarkDirector+Wants.swift:50-58,35-37; SessionLiveFacts.swift:38-40 | Owner and request tie-breaks compare ids with Swift `<` (code-point order) | bots.rs `owner_of`, `want_of` | fixed | **Fixed 2026-10-21:** the director moved to Rust and compares ids in code-point order, as Swift `<` does; the TypeScript one used `localeCompare`. |
| 37 | LogoQueueView.swift:208,225,242-243 (call site of BotActor) | Dynamic-dock bots get a 1-px hairline (`outline = 1`), breathe at `settings.breathSeconds`, and are built `awake` when the dock is laid out again | HUDMarkStrip.tsx:332-405 (no `outline`, no period, no `awake`); BotMark has no period prop | differs | **What:** Next's dock BotMark has outline 0, a fixed 3-s breath and is never awake. **When:** Dynamic (logos) dock on a wallpaper; a non-default "Working period"; a dock rebuilt (mode, screen or layout change). **User sees:** bots without the hairline that keeps them readable on busy wallpapers; breathing that ignores the slider; every bot morphing in from its logo again after a re-layout. This crosses into the LogoQueueView chunk. |
| 38 | BotActor.swift:179-207 | Drawn Clawd ↔ sprite Clawd hand-over fades the layer (ClawdActor.fade 0.3 s) | BotMark.tsx:173-178,198-200 | differs | **What:** Next switches renderer with no fade. On first load it waits for both resources, so this happens only on a size or scale change that crosses grain 2 (13 pt at 2×). **When:** resizing a Claude mark across the threshold. **User sees:** a hard cut. Low. |
| 39 | BotActor.swift:229-362 | BotClock: one shared timer; leaves out bots in occluded windows, while displays sleep or the session is away; resumes without replaying | BotMark.tsx:141-163,329-330; bot-motion.ts:300-301 | unclear | Next uses a per-mark setTimeout on the same frame grid and stops only on `document.hidden`. Resuming without replay is equivalent (dt=0 after a gap). The verdict turns on whether WKWebView sets `document.hidden` for occluded transparent HUD windows and during display sleep or a fast user switch. If not, timers keep firing. Not visible; it costs CPU. |
| 40 | Clawd/ClawdSheet.swift; ClawdSpriteTable.swift:17-81; ClawdSheetSource.swift; ClawdClips.swift:68-72 | Decode the sheet once, base-36 frames and deltas, anchors, canonical repeats (Walking first), the `fit` check against the frame counts | clawd.ts:39-159; clawd-clips.ts:308-327 | same | For the bundled JSON the result is identical. Next accepts JSON only and rejects the whole sheet if any sprite is bad, where Open skips that sprite and lets `fit` decide. This can only matter with a malformed file; then Next falls back to the drawn Clawd. |
| 41 | ClawdClip.swift; ClawdClips.swift:76-145 | All clips, shots, gaze sectors, pointsAtCard, signShifts, frameCounts; the ClawdTicks grid | clawd-clips.ts:68-329 | same | Checked frame by frame. |
| 42 | ClawdPlayer.swift:27-573 | Player: show/resume/play/visit/greet/endHover/look/card/pointAtCard/standAtCard/badge; switchPoint; intro/entry/loop/outro; exits; pace slowdowns; endHold; nextChange lookahead; sector rounding | clawd-player.ts:59-722 | same | The legacy `step()` adapter (lines 323-385, `Math.random`) is unused. |
| 43 | ClawdPerformer.swift:11-235 | Staying arrival on the grid, onStage, greet after 0.3 s, eyes follow by sector, visits (Swaying/Turning/Glancing or Dozing), stale-event release, look mapping (plain → command, held thinking → retrying) | clawd-performer.ts:38-273 | same | `playCue` adds dedupe only (see row 34 for the clock). |
| 44 | ClawdBaker.swift:74-213 | Grain floor(1.8·side·scale/24+0.05), sprites at grain ≥ 2, feet 0.375·side below the middle, n/2 blocks from the anchor, mirrored, mini at n/4, sign box, 64-frame LRU | clawd.ts:161-310; BotMark.tsx:168-180 | better | **Decided 2026-10-21:** Next rounds the grain to the nearest instead of down. Open leaves Clawd's body as little as seven tenths of the mark (12 × 7.5 pt in the 17 pt reminder on a Retina screen, a size smaller than the logo it replaces); Next draws it at about the mark's width (18 pt). Odd grains lay sprite pixels in runs of one and two device pixels, as Open's baker already does. Sprites still need Open's floor grain ≥ 2, so no smaller mark swaps its logo for Clawd. Claude's mark canvas gains 0.3·side of room on each side so props (desk, laptop, light bulb, spark, jumps) are not cut. |
| 45 | ClawdActor.swift:10-208 | Two-layer cross-fade 0.3 s on the media curve (0.2,0.8,0.2,1); no-news 0.5 opacity fade; logo frames from `BotRenderer.logo`; frames purged after 60 s as the logo | clawd-actor.ts:31-213; BotMark.tsx:259-290 | same | Next dates the logo-side fade from the player's tick (`appearanceChangedAt`), which is equivalent. |
| 46 | MarkDirector.swift:34-129,173-249 | Director: thinkingAfter 10, quietAfter 180, confirmation 2, hold 6, guardedHold 12 after 4 changes in 30 s, restBeforeStart 300, asleepAfter 3600, badgeLife 900; follow/show/hear/tell/looked/nextChange | bots.rs (`Director`, `follow`, `show`, `tell`) | same | Inputs are UTC seconds. Checked against the TypeScript director's own steps (bots/director-cases.json). |
| 47 | MarkDirector+Wants.swift:8-31,87-109 | Want priority (request > compacting > owner's work with tool takeover > plain > no news > asleep/idle), `holds`, badge rules | bots.rs (`want_of`, `holds`, `badge_of`) | same | Apart from rows 35 and 36. `awaitsModel` defaults differ for activity-less sessions, but that only affects `plain`, where `holds` is ignored. |
| 48 | DockMemory.swift:6-29 | Fresh cue as is; a missed failure/limit whose badge stands is replayed once, re-dated | mark-director.ts:465-491; HUDMarkStrip.tsx:368-378 | same | |
| 49 | MarkDirections.swift:8-61 | One director for every screen; timer at `nextChange` (tolerance 0.1); `onChange` only when directions change; `looked` | bots.rs; coordinator.rs; use-bot-directions.ts | same | The host runs it on its coordinator thread and sends the directions to the HUD windows on screen; see the architecture notes. |

**Count check:**

| Status | Rows |
|---|---|
| same | 1–7, 9–17, 19–25, 27, 40–49 |
| differs | 8, 18, 26, 28–38 |
| unclear | 39 |

## Tests

| Open test file | Covers | Portable to fixtures? |
|---|---|---|
| BotStateTests.swift | BotState/BotLook/BotPhase, BotMotion per look (waits, holds, resume, events, hover, doze, no news), BotMoves curves, Clawd beats | **Yes.** Pure clock + seed → pose/face/limbs/sign/wait. It needs only the BotSpec table and SplitMix, both ported bit-exact. This is the best first fixture set. |
| BotMotionTests.swift | BotMotion lifecycle: morph timing, visits, seeds, aim, reminder arrival, staying bots, `BotActor.transform` | Mostly yes: 25 of 27 tests are pure. The pose-to-transform test is a small math check that can be portable. A few use BotForm only for `fit(spec)`; pass the spec as data instead. |
| BotFinTests.swift | Fin geometry (BotFins.path), flap timing in BotMotion, fin pixels in frames | Partly. Flap timing and path vertices are portable as numeric fixtures. Pixel/coverage assertions are bound to CoreGraphics rasterisation. |
| BotSignTests.swift | Glyph rows, sign order, cell size, sign box placement, still-sign frame, pixel painting | Partly. Rows, order, cell and box math are portable and pure. The pixel tests need CG or Canvas byte comparison; they could be golden images with exact pixels, since there is no antialiasing. |
| BotFormTests.swift | Bundled marks have bots, shared marks share bots, reach arrays, Clawd spec, drawing at queue size, hairline growth, Copilot eyes, Look quantisation | Partly. The vendor→spec mapping and Look quantisation are portable. Reach arrays come from rasterised PNGs: portable only with a tolerance, since CG and Canvas antialiasing differ. |
| BotLevelTests.swift | Antigravity settle lines its body up with a ball | Tolerance fixture on `top`/reach. Needs the rasterised artwork. |
| BotFrameTests.swift | Kept still parts give byte-identical frames, sub-agent copy placement, outline/hairline equal to the earlier implementation, frame cost | No. Bound to CGContext bytes and timing. |
| BotFramesAsBefore.swift | Test helper: the legacy CG hairline | No (CG). |
| BotClockTests.swift | BotPace grid; BotClock timer, occlusion, sleep and session handling | Only `testEveryFrameFallsOnAWholeMultipleOfItsPace` is portable. The rest use NSWindow/Timer. |
| BotQueueTests.swift | Logo queue and BotStageView integration: bots on/off, reduce motion, no-news fade, reminder arrival, cost | No. AppKit/SwiftUI windows. Its assertions (no-news fade, reminder arrival, reduce motion) are exactly the Next gaps in rows 28 and 30, so they are worth re-expressing as UI tests. |
| ClawdActorTests.swift | Sprite actor in a CALayer: baked frames, fades, purge, awake, reminder card, stage | Mostly no (CALayer). The sequencing (which `Seen` when, fade windows, purge after 60 s) could be portable against Next's pure `ClawdActor.step()` frames. |
| ClawdBakerTests.swift | Grain, whole-pixel blocks, feet placement, offset, mirror, mini, sign box, hairline, 64-frame LRU | Yes for placement, blocks, mirror, mini and sign box: exact integer pixels, so byte fixtures work. Display colour-space test: no (CG). |
| ClawdBundledSheetTests.swift | The bundled sheet fits every clip | Yes (data check). |
| ClawdClipsTests.swift | Clip frame lists, one-offs, fit rules, sign order | Yes (pure data). |
| ClawdFixture.swift | Fixture builder for a synthetic sheet | Helper. Its synthetic sheet can be exported as JSON. |
| ClawdPerformerTests.swift | Performer: each look's clip, resume, at-card holds, badges, cards, staying arrival, visits, greet and watch, stale events, event rank | **Yes.** Pure clock + seed. Next's `ClawdPerformer` accepts `seed` or `random`. |
| ClawdPlayerTests.swift | Player transitions, hand-overs, exits, slowdowns, events, visits, greet/gaze sectors, cards, at-card, mini, badges, canonical frames, due times, switch points | **Yes.** Pure clock, deterministic. Next's `ClawdPlayer` takes the same inputs. |
| ClawdSheetTests.swift | Literal parser variants, deltas, colours, non-sheets, repeats, decoding the file | Partly. JSON-form cases, deltas, colours and repeats are portable. JS-literal variants (minified booleans, `.split()`, templates) are Open-only, since Next is JSON-only. |
| MarkDirectorTests.swift | Every director rule: requests, compaction, owners, tool takeover, thinking after 10 s, holds, confirmation/hold/guard, starts, events, badges, dock replay | **Yes, high value.** Pure facts → directions. Next takes a protocol `Snapshot` rather than `QueueMark`, so the fixtures need a thin adapter (or a QueueMark-shaped JSON input). A fixture with a closed edit plus an open `.agents` call would catch row 35. |

## Architecture notes

- **Business logic in UI/TS that Open keeps in Core.** `mark-director.ts` holds two pieces of logic that Open keeps in AgentHUDCore:
  - `botQueue()` rebuilds `ReportView.queueMarks`: the 24-h cutoff, request merging, and `exhaustedUntil` from quotas ≥ 99.5 %.
  - It inlines `SessionLiveFacts` derivations: `look(at:)`, `hasOpenCall`, `compacting(at:)` (300 s), `limited(at:)`, `nextChange(after:)`.

  In Open these live in AgentHUDCore (`ReportView+Marks.swift`, `SessionLiveFacts.swift`). The duplicate has already drifted, through Rust overloading `openCall` (row 35). It also drops Open's `liveStatus` filter, and its request merge adds HUD approvals whose session is not in the queue. Both are unclear without the ReportView chunk's input. The director itself (`MarkDirector`) is pure, but it runs in TypeScript in the UI package.
- **Director hosting.** Since 2026-10-21 the single director runs in the host (`bots.rs`, driven by `coordinator.rs`), no longer in the Settings webview, which can now be closed. A HUD window reads the directions with `hud_bot_directions` when it comes on screen and hears `hud:bot-directions` while on screen. `startedAt` and the director's memory (turn ends heard, cue, guard) last as long as the app. Each window converts UTC cue times to `performance.now` through an offset captured once (row 34).
- **Rust fact encoding.** `agents/src/transcript/live.rs:462` falls back `openCall` to a non-shown call so that "any call open" can be inferred. That overloads one field with two meanings (Open has `openCall` and `hasOpenCall` separately) and causes row 35. A `hasOpenCall` boolean in the protocol would remove the ambiguity.
- **Host component size.** `BotMark.tsx` mixes resource loading, renderer choice, scheduling, painting and pointer handling in one 230-line effect. `HUDMarkStrip` calls the side-effecting `DockMemory.cue()` during render. It is idempotent, but it is render-time mutation.
- **Dead code.**
  - `procedural.ts`: `ProceduralMotion`, `drawProcedural`, `signAbove`.
  - `ClawdPlayer.step()`: the legacy four-phase adapter, which uses an unseeded `Math.random`.
  - `clawd.ts`: `drawClawd`, `isClawdStanding`.
  - `bot-renderer.ts`: `drawBot`, used only by the dead `procedural.ts`.
- **Performance shape.** Each of the following was one shared mechanism in Open:
  - One `setTimeout` per mark instead of one shared clock. The frame grid is kept, so the wake-ups still coincide.
  - A new canvas per drawn frame; the BotCanvas reuse was not ported.
  - The hairline computed in JS at O(w·h·(2r+1)²) per frame.
  - BotStill using `getImageData`/`putImageData`, a GPU→CPU readback.
- **No OS-specific code** was found in the shared bot files.
- **No concurrency touches collection reads** in this chunk, which is UI only. The `loadArtwork`/`loadClawd` `Promise.all` loads assets, not provider data.
