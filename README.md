<p align="center">
  <a href="http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/v0.1.4/downloads/SI-Future-HUD-film-1080p60.mp4">
    <img src="docs/si-future-hud/media/hero.jpg" alt="SI Future HUD: your AI agents, living at the edge of your screen. The MacBook's notch dropped open into a panel listing every agent's quota: Claude, OpenAI, Chengxiaobang, Cursor and Grok Bot" width="100%">
  </a>
</p>

<h1 align="center">SI Future HUD</h1>

<p align="center">
  <strong>Your AI agents, living at the edge of your screen.</strong><br>
  Quotas, live states, approvals and agent‑to‑agent messages, right around your MacBook's notch.
</p>

<p align="center">
  <a href="http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/permalink/latest"><img src="https://img.shields.io/badge/release-v0.1.4-7c8cff?style=for-the-badge" alt="Release v0.1.4"></a>
  <img src="https://img.shields.io/badge/macOS-12%2B%20·%20Apple%20silicon-111?style=for-the-badge&logo=apple&logoColor=white" alt="macOS 12+ on Apple silicon">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-2ea44f?style=for-the-badge" alt="Apache-2.0"></a>
  <a href="https://github.com/jazzenchen/agent-hud-open"><img src="https://img.shields.io/badge/based%20on-Agent%20HUD-ff8a5c?style=for-the-badge" alt="Based on Agent HUD"></a>
</p>

<p align="center">
  <strong>English</strong> · <a href="README.zh-CN.md">简体中文</a>
  &nbsp;|&nbsp;
  <a href="http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/v0.1.4/downloads/SI-Future-HUD-film-1080p60.mp4">Watch the film</a> ·
  <a href="http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/permalink/latest">Download</a> ·
  <a href="docs/si-future-hud/INSTALL.md">Install</a> ·
  <a href="docs/si-future-hud/GUIDE.md">Guide</a> ·
  <a href="docs/si-future-hud/MESSAGING.md">Messaging</a> ·
  <a href="docs/si-future-hud/TROUBLESHOOTING.md">Troubleshooting</a>
</p>

---

You hand work to Claude Code, Codex, Cursor, Grok Bot and Chengxiaobang, then go back to your own. **SI Future HUD keeps all of them in the corner of your eye:**
- who is thinking and who is waiting for you;
- how much quota each one has left;
- what needs your OK.

When two of your agents need to compare notes, they can now send each other a message. Each one waits on the HUD until you press **Send**.

SI Future HUD is a customized build of the open‑source [Agent HUD](https://github.com/jazzenchen/agent-hud-open). It adds agent‑to‑agent messaging, the Chengxiaobang (程小帮) integration, live states for Cursor and Grok Bot, and an install that sets itself up.

<p align="center">
  <a href="http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/v0.1.4/downloads/SI-Future-HUD-film-1080p60.mp4">
    <img src="docs/si-future-hud/media/poster.jpg" alt="Still from the product film: the opened notch panel" width="72%">
  </a><br>
  <sub>▶ <b>Watch the 2½‑minute film</b> (1080p60, MP4, 34 MB) · sample data throughout</sub>
</p>

## Hover the notch. See everything.

Move the pointer onto the notch and its glowing rim drops open into one panel with every agent:
- your account and plan;
- every quota window with its bar and when it resets;
- five ways to read each block: quota, burn rate, reply rate, consumed, and estimated cost.

Move the pointer away and it folds back into a quiet ring of light.

<p align="center">
  <img src="docs/si-future-hud/media/notch-drop.webp" alt="The pointer touches the notch; the glowing notch drops open into a tall panel listing Claude, OpenAI, Chengxiaobang, Cursor and Grok Bot with their quota windows" width="88%">
</p>

The ring's colour comes from the tightest quota: green under 70% used, yellow from 70%, red from 90%. It breathes faster while your agents work.

## Your agents come alive.

Every agent has a little bot that shows what it is doing right now:
- **thinking**;
- **compacting** its context;
- **editing code**;
- **running a command**;
- **waiting for your approval**;
- **asking a question**;
- **done**;
- **out of quota**.

You don't have to open a window to know.

<p align="center">
  <img src="docs/si-future-hud/media/bots.webp" alt="A grid of bots, one row per agent and one column per state: thinking, compacting, editing, running a command, waiting for approval, asking, done and out of quota; shown on a dark and a light background" width="88%">
</p>

<table>
  <tr>
    <td width="50%"><img src="docs/si-future-hud/media/strip-arrival.webp" alt="Clawd leaps from the Claude window to the left edge of the screen and the strip of agents grows below it, ring by ring" width="100%"></td>
    <td width="50%"><img src="docs/si-future-hud/media/quota-out.webp" alt="Claude's ring fills to 100%, the HUD warns that the quota is running out, and Clawd faints" width="100%"></td>
  </tr>
  <tr>
    <td align="center"><sub><b>At home on the edge.</b> Each agent sits in a ring of its quota.</sub></td>
    <td align="center"><sub><b>Never run out by surprise.</b> A warning first, then the bot faints.</sub></td>
  </tr>
</table>

## Answer it right there.

When an agent asks to run a command, the request comes to the HUD. Choose **Allow once**, **Deny** or **Always allow** without leaving what you are doing. Questions work the same way, including the ones Claude asks in its desktop app.

A finished reply shows up as a **New reply** card. One click takes you back into that conversation. If you leave a request alone, it goes back to the agent after 10 minutes: the HUD never decides for you.

<table>
  <tr>
    <td width="50%"><img src="docs/si-future-hud/media/approval.webp" alt="Claude asks to run swift test; the HUD card shows the command and Allow once is clicked" width="100%"></td>
    <td width="50%"><img src="docs/si-future-hud/media/usage-lens.webp" alt="The usage panel opens beside Claude's ring; a magnifier shows 268M tokens for the main conversation" width="100%"></td>
  </tr>
  <tr>
    <td align="center"><sub><b>Approvals on the HUD.</b> The command in full, then one click.</sub></td>
    <td align="center"><sub><b>Details on demand.</b> Quotas, tokens and every session.</sub></td>
  </tr>
</table>

## Agents talk to each other, with your OK.

Claude Code, Codex (desktop) and Chengxiaobang can now message one another. Ask Claude to check something with Codex. Claude calls the HUD's `hud_send_message` tool, and the message waits on the HUD until you press **Send**. Then a paper plane carries it to Codex's open conversation. The reply flies back into the exact conversation that asked.

<table>
  <tr>
    <td width="50%"><img src="docs/si-future-hud/media/paper-plane.webp" alt="Claude says it will ask Codex; a paper plane flies from Claude's ring to Codex's, and Codex answers that it got it" width="100%"></td>
    <td width="50%"><img src="docs/si-future-hud/media/reply.webp" alt="Codex sends its reply; the paper plane flies back to Claude, who says it got the reply" width="100%"></td>
  </tr>
  <tr>
    <td align="center"><sub><b>Ask.</b> Claude → Codex, after you press Send.</sub></td>
    <td align="center"><sub><b>Reply.</b> Back to the exact conversation that asked.</sub></td>
  </tr>
</table>

Nothing is sent behind your back. Deny a message and its plane falls. → [How messaging works](docs/si-future-hud/MESSAGING.md)

## One more thing: your company's knowledge, for every agent.

**Chengxiaobang (程小帮)** is Trip.com's AI workbench. It is connected to the company's knowledge base, MCP servers and plugins. Through the HUD, your coding agents can ask it the business questions they can't answer from the code:
- *Is there a limit on travellers?*
- *How are children priced?*

Chengxiaobang looks it up and sends each agent its answer. The agents then compare notes until they agree.

<p align="center">
  <img src="docs/si-future-hud/media/ecosystem.jpg" alt="Claude and Codex on either side of Chengxiaobang, which is connected to the company knowledge base, MCP and plugins" width="88%">
</p>

<table>
  <tr>
    <td width="50%"><img src="docs/si-future-hud/media/ask-chengxiaobang.webp" alt="Claude asks Chengxiaobang a business question; the HUD shows the message for approval and a paper plane carries it" width="100%"></td>
    <td width="50%"><img src="docs/si-future-hud/media/debate.webp" alt="Claude and Codex exchange paper planes and short arguments in speech bubbles" width="100%"></td>
  </tr>
  <tr>
    <td align="center"><sub><b>Ask the one who knows.</b> Every message still waits for your Send.</sub></td>
    <td align="center"><sub><b>Then they agree.</b> Codex and Claude settle the details together.</sub></td>
  </tr>
</table>

<p align="center">
  <img src="docs/si-future-hud/media/conclusion.jpg" alt="The agreed conclusion: at most 8 travellers, checked on the server, a hint in the app, children priced separately" width="88%">
</p>

## Three looks. Any edge.

| | |
| --- | --- |
| **Notch** | A ring of light around the MacBook's notch. Hover it and the full panel drops open. |
| **Dynamic** | A row of agent logos on any edge. Each logo turns into its bot while that agent works. |
| **Simple** | A small strip of quota rings on any edge. It turns horizontal on the top or bottom edge and vertical on the sides. |

<table>
  <tr>
    <td width="50%"><img src="docs/si-future-hud/media/looks.webp" alt="Dynamic mode: agent logos beside the notch turn into working bots one after another" width="100%"></td>
    <td width="50%"><img src="docs/si-future-hud/media/drag.webp" alt="Simple mode: the strip of rings is dragged from the left edge to the top edge, where it turns horizontal, and back" width="100%"></td>
  </tr>
  <tr>
    <td align="center"><sub><b>Dynamic.</b> Logos that wake up when their agent works.</sub></td>
    <td align="center"><sub><b>Simple.</b> Drag it to any edge.</sub></td>
  </tr>
</table>

## Supported agents

| Agent | Quota | Live state | Approvals | Questions | Replies | Messaging |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Claude Code** | ✓ | ✓ | ✓ | ✓ | ✓ | send + receive |
| **Codex** | ✓ | ✓ | ✓ | ✓ | ✓ | send + receive (desktop app) |
| **Chengxiaobang 程小帮** | last 24 h of tokens | ✓ |  | ✓ | ✓ | send + receive |
| **Cursor** | ✓ | ✓ |  |  | ✓ |  |
| **Grok Bot** | ✓ | ✓ |  | display | ✓ |  |

Codex approvals need its hook trusted once in Codex. Grok Bot questions show on the HUD but are answered in Grok Bot.

Everything else Agent HUD supports comes along too, for example DeepSeek, GitHub Copilot CLI, OpenCode, Antigravity and Kimi. See the [upstream list](README.upstream.md#supported-agents). The five agents above are the ones this build was tested with.

## Get started

1. **Download** `SI-Future-HUD_0.1.4_aarch64.dmg` from the [latest release](http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/permalink/latest). You need an Apple silicon Mac with macOS 12 or newer.
2. **Drag** SI Future HUD into **Applications**, then open it from there.
3. **Allow it once.** In **System Settings → Privacy & Security**, click **Open Anyway**. The build is ad‑hoc signed, not notarized.

On first launch it sets itself up:
- installs its Claude Code plugin;
- adds the HUD tools to Claude Code and Codex;
- turns on client hooks so approvals reach the HUD;
- drops its plugin into Chengxiaobang.

→ Every step, and what each macOS prompt means: **[Install guide](docs/si-future-hud/INSTALL.md)**

> ⚠️ **Read this before your first day with it.**
> - **Install the command‑line tools.** The HUD connects to Claude Code and Codex through the `claude` and `codex` commands. If you only use the desktop apps, install them too, then reopen the HUD. Without them, Claude can't send or receive messages and Codex can't send.
> - **Claude's quota needs the terminal `claude` signed in.** The desktop app's sign‑in is separate. If the panel says *Claude Code is signed out*, run `claude` in a terminal and sign in.
> - **Codex receives messages only in Codex desktop conversations**, and only when that conversation isn't busy with a turn.
> - **Chengxiaobang:** enable the *SI Future HUD* plugin once on its Plugins page. After each restart, open a conversation once so that it can receive.
> - **Conversations opened before you installed the HUD** need `/reload-plugins` (Claude Code), or simply start a new one.
> - **Uninstall in order.** Turn **Client hooks** off first, then remove the app. Otherwise hook entries pointing at a missing app stay behind in your agents' settings.
>
> Every caveat, with the fix for each: **[Troubleshooting](docs/si-future-hud/TROUBLESHOOTING.md)**

## Privacy

- **Everything stays on your Mac.** The HUD reads your agents' local usage records, and checks quotas with the accounts you are already signed in to.
- **Nothing goes to any third‑party server.** There is no telemetry and no auto‑update.
- **Chengxiaobang's local database is opened read‑only.**
- **No message moves between agents until you press Send.**

## Build from source

```bash
pnpm install
pnpm run build:macos -- --bundle
```

The app lands in `target/release/bundle/macos/`. For builds to hand to other people, use [`scripts/build-release.sh`](scripts/build-release.sh). It does three things:
- rewrites the build machine's paths out of the binaries;
- signs the whole bundle ad‑hoc and verifies the signature;
- checks that a quarantined copy still opens.

Without a whole signature, a downloaded copy is reported as *damaged*. Toolchain details: [docs/development.md](docs/development.md).

## What's different from Agent HUD

- **Agent‑to‑agent messaging.** Claude Code, Codex desktop and Chengxiaobang can message each other through the `hud_send_message` MCP tool. Every message needs your Send, a paper plane flies for it, and replies go back to the exact conversation.
- **Chengxiaobang.** Usage from its local database, live states, questions answered on the HUD, and messaging. The plugin and MCP entry install themselves.
- **Codex desktop.** Messages are delivered into Codex desktop conversations over its local IPC socket.
- **Grok Bot and Cursor live states.** Replies and waiting cards for both, and Grok Bot team‑seat quotas.
- **Installs like a normal app.** No hard‑coded paths, set‑up on first launch, a login item that follows the app, and a release build that teammates can open.

The full list of changed files is in **[docs/si-future-hud/CHANGES.md](docs/si-future-hud/CHANGES.md)**. The release history is in [CHANGELOG.md](CHANGELOG.md).

## Credits & license

SI Future HUD is built on **[Agent HUD](https://github.com/jazzenchen/agent-hud-open)** by Jazzen Chen and contributors. Thank you for making it open. Agent HUD's own README is kept as [README.upstream.md](README.upstream.md).

Licensed under the [Apache License 2.0](LICENSE), like the project it is built on. Modified files are listed in [CHANGES.md](docs/si-future-hud/CHANGES.md).

Claude, Codex, Cursor, Grok and Chengxiaobang are trademarks of their owners. This project is not affiliated with them. The film and screenshots use sample data.

<p align="center"><sub>Made by <a href="http://git.dev.sh.ctripcorp.com/ckxu">Chengkai Xu (许成凯)</a> · SI Future</sub></p>
