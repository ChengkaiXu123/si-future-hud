<p align="right"><strong>English</strong> · <a href="TROUBLESHOOTING.zh-CN.md">简体中文</a></p>

# Troubleshooting & things to know

Every snag we know of, with its cause and fix. Most of them take one command or one click.

**Jump to:** [Installing](#installing) · [Claude Code](#claude-code) · [Codex](#codex) · [Chengxiaobang 程小帮](#chengxiaobang-程小帮) · [Cursor](#cursor) · [Grok Bot](#grok-bot) · [Messaging](#messaging) · [The HUD itself](#the-hud-itself) · [Removing it](#removing-it)

## Installing

<details open>
<summary><b>"Apple cannot check it for malicious software" / "unidentified developer"</b></summary>

**Why:** the release is ad‑hoc signed, not notarized by Apple.

**Fix:** close the prompt, then open **System Settings → Privacy & Security** and click **Open Anyway** next to *"SI Future HUD" was blocked*. Confirm with **Open**. You only do this once.
</details>

<details open>
<summary><b>"SI Future HUD is damaged and can't be opened"</b></summary>

**Why:** the downloaded copy carries a quarantine flag that an un‑notarized app can't pass.

**Fix:**

```bash
xattr -dr com.apple.quarantine "/Applications/SI Future HUD.app"
```

Then open it again.
</details>

<details>
<summary><b>Nothing got set up: no Claude plugin, no HUD tools</b></summary>

**Why:** one of three things.
1. The app was opened straight from the DMG window. macOS then runs it from a temporary read‑only location.
2. The `claude` or `codex` command isn't installed.
3. It is installed somewhere the HUD doesn't look.

**Fix:**
- Move the app into **Applications**, quit it and reopen it from there.
- Install the command‑line tools. The HUD looks in `~/.local/bin`, `~/.bun/bin`, `~/.npm-global/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/bin` and your `PATH`.
- Reopen the HUD. Set‑up is retried on every launch until it succeeds.

Check from a terminal with `claude mcp list` and `codex mcp list`. Both should list `agent-hud`.
</details>

<details>
<summary><b>It won't open on my Mac at all</b></summary>

The release is built for **Apple silicon** (arm64) and **macOS 12+**. On an Intel Mac, [build from source](../../README.md#build-from-source) on that Mac.
</details>

## Claude Code

<details open>
<summary><b>The panel says "Claude Code is signed out", but the desktop app is signed in</b></summary>

**Why:** the HUD reads Claude's quota through the **terminal `claude` command**. It runs `get_usage` and checks `claude auth status`. The Claude desktop app manages its own sign‑in for the Claude Code inside it, and that sign‑in is **not shared** with the terminal command. A CLI update or an expired token can sign the terminal out while the desktop app stays signed in.

**Check:**

```bash
claude auth status
```

**Fix:** run `claude` in Terminal and sign in (or type `/login`), using the same account. The quota returns on the HUD's next reading.
</details>

<details>
<summary><b>"No plan limits for this login (API key or third‑party platform)"</b></summary>

The terminal `claude` is signed in with an API key or through a cloud provider, and those logins have no subscription quota to show. Sign in with your Claude subscription account if you want the quota.
</details>

<details>
<summary><b>A conversation doesn't get messages, or the HUD can't answer its questions</b></summary>

**Why:** the conversation was opened before the HUD installed its plugin.

**Fix:** type `/reload-plugins` in that conversation, or start a new one. New conversations load the plugin by themselves.
</details>

<details>
<summary><b>"This Claude Code is too old to recognise --marketplace"</b></summary>

Update Claude Code with `claude update`, then install the plugin from **Settings → Agent plugins**.
</details>

## Codex

<details open>
<summary><b>Codex never receives messages</b></summary>

Only conversations in the **Codex desktop app** can receive. The Codex CLI shows its status on the HUD, but it can't take messages.

The HUD talks to Codex desktop over `~/.codex/ipc/ipc.sock`. That channel only connects while a desktop conversation is running or waiting.
</details>

<details>
<summary><b>"Refused: the conversation is busy"</b></summary>

A Codex conversation in the middle of a turn refuses new messages. The HUD doesn't "steer" a running turn. Wait until it finishes, then ask your agent to send again.
</details>

<details>
<summary><b>Codex approvals don't show on the HUD</b></summary>

Codex has to trust the HUD's hook once. When Codex asks whether to trust it, allow it. Also check that **Settings → General → Client hooks** is on.
</details>

<details>
<summary><b>Codex can't send messages</b></summary>

The HUD tools weren't added to Codex, because the `codex` command wasn't found. Install the Codex CLI and reopen the HUD. Check with `codex mcp list`.
</details>

## Chengxiaobang 程小帮

<details open>
<summary><b>Chengxiaobang doesn't appear, or never receives messages</b></summary>

- Enable the **SI Future HUD** plugin once on Chengxiaobang's **Plugins** page. The HUD wrote it into `~/.chengxiaobang/plugins/agent-hud/`.
- **After every restart of Chengxiaobang,** click into a conversation once. Its plugin API works per conversation, so a conversation you haven't opened since the restart can't receive.
- **After you update the HUD,** restart Chengxiaobang once. Its running MCP relay keeps the old tool definitions until then.
</details>

<details>
<summary><b>Chengxiaobang shows usage but no quota bar</b></summary>

That's expected. Chengxiaobang has no quota window to read, so its block shows your last 24 hours of tokens. The HUD reads them from its local database (read‑only).
</details>

<details>
<summary><b>Answering Chengxiaobang's questions on the HUD</b></summary>

Toggle it in **Settings → Agent plugins → Chengxiaobang → Answer questions**. A question left alone for 10 minutes falls back to Chengxiaobang's own window. Its approval and plan cards can't be answered from the HUD.
</details>

## Cursor

<details>
<summary><b>Cursor's quota is missing, or shows 401 / 403</b></summary>

- Sign in **inside the Cursor desktop app**. A sign‑in on the website, or in another tool, doesn't update what the HUD reads.
- If it still fails on a company (Enterprise) account, the organisation's policy may block usage reads. That is outside the HUD's control.
</details>

## Grok Bot

<details>
<summary><b>Grok Bot's quota looks out of date</b></summary>

The HUD reads Grok Bot's quota from Grok Bot's own cache. Refresh it in Grok Bot and the HUD follows. Grok Bot's question cards are display‑only: answer them in Grok Bot.
</details>

## Messaging

<details open>
<summary><b>My agent says it sent a message, but nothing arrived</b></summary>

1. **Did you press Send?** Every message waits on the HUD.
2. **Can the receiver take it right now?** It can't if:
   - it is a closed conversation;
   - it is a busy Codex conversation;
   - it is a Chengxiaobang conversation not opened since a restart;
   - it is a conversation waiting for your answer to a question.

   Messages are refused, not queued.
3. **Too many waiting?** Each sender can have at most 2 messages waiting for your decision.

More detail: [Messaging → Rules](MESSAGING.md#rules-that-keep-it-safe).
</details>

<details>
<summary><b>The reply went to the wrong conversation</b></summary>

Replies go to the conversation named in the message. If the sender didn't name a `session`, the message went to the receiver's most recently active conversation. Ask the agent to reply with the `to` and `session` its message names.
</details>

## The HUD itself

<details>
<summary><b>My setting change didn't apply</b></summary>

Each display keeps its own look and position. Check that you edited the display you're looking at: **Settings → Display** lists every screen.
</details>

<details>
<summary><b>An approval disappeared before I answered</b></summary>

After 10 minutes without an answer, the request goes back to the agent. You can still answer it there.
</details>

<details>
<summary><b>Percentages look upside down</b></summary>

**Settings → General → Usage display** switches between **Consumed (0% → 100%)** and **Remaining (100% → 0%)**. The colours always follow what is used.
</details>

<details>
<summary><b>The panel opens too easily, or not easily enough</b></summary>

Adjust **Hover delay** and **Collapse delay** in **Settings → Display → Hover behavior**, or set **Press Option to open**.
</details>

## Removing it

<details open>
<summary><b>Uninstall without leaving anything broken</b></summary>

1. **Settings → General → turn off Client hooks.** **This step matters.** Skip it and Claude Code, Codex and others keep hook entries that point at a missing app.
2. Quit SI Future HUD and drag it to the Trash.
3. Optionally remove the plugin and tools:

   ```bash
   claude plugin uninstall agent-hud@agent-hud
   claude plugin marketplace remove agent-hud
   claude mcp remove --scope user agent-hud
   codex mcp remove agent-hud
   ```

   Then disable the plugin in Chengxiaobang.
4. Optionally delete `~/Library/Application Support/app.agenthud.next`.
</details>

---

Still stuck? Open an [issue](https://github.com/ChengkaiXu123/si-future-hud/issues) with a screenshot of the HUD and the output of `claude auth status` / `codex mcp list`. Leave out tokens, emails and paths you'd rather keep private.
