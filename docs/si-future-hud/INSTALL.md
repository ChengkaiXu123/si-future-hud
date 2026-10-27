<p align="right"><strong>English</strong> · <a href="INSTALL.zh-CN.md">简体中文</a></p>

# Install SI Future HUD

About five minutes, most of it macOS asking whether you trust the app.

<p align="center"><img src="media/panel.jpg" alt="The opened notch panel with every agent's quota" width="80%"></p>

## Before you start

| You need | Why |
| --- | --- |
| **A Mac with Apple silicon** (M1, M2, M3, M4…) and **macOS 12 or newer** | The release is built for arm64 only. Check: Apple menu → **About This Mac** → *Chip* says *Apple M…*. On an Intel Mac, [build it from source](../../README.md#build-from-source). |
| **The `claude` command** (Claude Code CLI), if you use Claude Code | The HUD installs its plugin and tools into Claude Code through this command. **The desktop app alone is not enough.** |
| **The Codex desktop app**, if you use Codex | Nothing to install on top. The HUD uses the `codex` command built into the app (Codex.app, or the ChatGPT app that carries Codex) to add its tools, and messages *to* Codex go to the app. **You don't need the Codex CLI.** Only if you use Codex purely from a terminal does the HUD need a `codex` command from the list below. |
| **Chengxiaobang (程小帮)**, if you use it | Nothing to prepare. The HUD drops its plugin in by itself. |

**Installing the command‑line tools.** Follow each vendor's official instructions:
- Claude Code: [docs.claude.com → Claude Code → Set up](https://docs.claude.com/en/docs/claude-code/setup)
- Codex, only if you have no Codex desktop app: [github.com/openai/codex](https://github.com/openai/codex)

The HUD looks for them in `~/.local/bin`, `~/.bun/bin`, `~/.npm-global/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/bin` and your `PATH`.

You can install them later. The HUD tries again every time it starts.

## 1 · Download

The quickest: **[SI-Future-HUD.dmg](https://pages.release.ctripcorp.com/ckxu/si-future-hud/SI-Future-HUD.dmg)** (45 MB, no sign‑in). Or, from the [release page](http://git.dev.sh.ctripcorp.com/ckxu/si-future-hud/-/releases/permalink/latest) (company GitLab sign‑in), download:

- `SI-Future-HUD_0.1.4_aarch64.dmg`: the app
- `SHA256SUMS.txt`: checksums, if you want to verify the download

To verify, run this in Terminal in the download folder:

```bash
shasum -a 256 -c SHA256SUMS.txt --ignore-missing
```

You should see `SI-Future-HUD_0.1.4_aarch64.dmg: OK`.

## 2 · Move it into Applications

1. Double‑click the DMG.
2. In the window that opens, drag **SI Future HUD** onto **Applications**.

> ⚠️ **Don't open the app straight from the DMG window.**
>
> macOS would run it from a temporary, read‑only location, and the automatic set‑up (plugins, tools, the login item) can't work from there. Always open it from **Applications**.

## 3 · Open it, and let macOS trust it

Open **Applications → SI Future HUD**.

SI Future HUD is ad‑hoc signed, not notarized by Apple, so the first launch is blocked. You will see *"SI Future HUD" cannot be opened because Apple cannot check it for malicious software*, or *…from an unidentified developer*. To allow it once:

1. Click **Done** (or **Cancel**) to close the prompt.
2. Open **System Settings → Privacy & Security** and scroll down. You'll see *"SI Future HUD" was blocked to protect your Mac*.
3. Click **Open Anyway**, then **Open**. Enter your password if asked.

From now on it opens normally.

**If macOS says the app "is damaged and can't be opened",** the download kept a quarantine flag that this unsigned app can't pass. Run this in Terminal, then open the app again:

```bash
xattr -dr com.apple.quarantine "/Applications/SI Future HUD.app"
```

## 4 · What happens on first launch

You don't need to touch any setting. On first launch SI Future HUD:

| It does | So that |
| --- | --- |
| Shows the HUD at the top of the screen, around the notch where there is one | You see it right away |
| Installs its **Claude Code plugin** (`agent-hud@agent-hud`) | You can answer Claude's questions on the HUD, and messages can reach Claude |
| Adds **HUD tools** (the `agent-hud` MCP server) to **Claude Code** and **Codex** | Agents can call `hud_send_message` and read the HUD |
| Turns on **Client hooks** (Settings → General) | Permission requests from Claude Code, Codex, Cursor, Copilot and OpenCode come to the HUD |
| Writes its plugin into **Chengxiaobang** and adds the HUD tools to it | Chengxiaobang shows on the HUD and can send and receive messages |
| Keeps a **login item** you turned on pointed at the app, even after you move it | *Launch at login* stays off until you turn it on in Settings |

Each step runs once. **Nothing you remove later is put back.** If `claude` wasn't installed yet (or `codex`, when there is no Codex desktop app either), that step waits and runs on a later launch.

## 5 · Finish setting up each agent

| Agent | One last step |
| --- | --- |
| **Claude Code** | **Sign in to the terminal `claude` once:** run `claude` in Terminal and follow the sign‑in. The quota comes from the terminal command, and the desktop app's sign‑in is separate. In conversations that were already open before you installed the HUD, type `/reload-plugins`, or start a new conversation. |
| **Codex** | Use the **Codex desktop app** for conversations that should receive messages (the CLI shows status only). The first time Codex asks whether to trust the HUD's hook, allow it, or approvals won't reach the HUD. |
| **Chengxiaobang** | Open its **Plugins** page and enable **SI Future HUD** once. After every restart of Chengxiaobang, click into a conversation once, and that conversation can then receive messages. |
| **Cursor** | Sign in **inside the Cursor desktop app**. A web sign‑in doesn't count. |
| **Grok Bot** | Sign in. Quota and replies appear by themselves. |

## 6 · Check that everything is connected

- **Menu bar icon → Settings → Agent plugins (智能体插件).** The Claude Code plugin and the Chengxiaobang card should show as installed.
- **HUD tools.** In Settings, the HUD tools rows for Claude Code and Codex should be on. From a terminal you can also check:

  ```bash
  claude mcp list | grep agent-hud
  codex mcp list | grep agent-hud
  ```

  With only the Codex desktop app there is no `codex` in your terminal. The Settings row is enough, or run the app's copy: `/Applications/ChatGPT.app/Contents/Resources/codex-cli/bin/codex mcp list` or, if your app is Codex.app, `/Applications/Codex.app/Contents/Resources/codex mcp list`.

- **Hover the notch.** Every agent you use should be listed with its quota.
- **Try a message.** In a Claude Code conversation, ask:

  > *Ask Codex what it's working on, using the HUD.*

  A card appears on the HUD. Press **Send** and watch the paper plane fly.

## Update

Quit SI Future HUD, then drag the new version into **Applications** and replace the old one. Settings, plugins and tools carry over. If the app moved, the tool entries are re‑pointed on launch.

After an update, **restart Chengxiaobang once**. Its running MCP relay keeps the old tool definitions until it restarts.

## Uninstall

Do it in this order. Otherwise the hook entries in your agents' settings point at an app that no longer exists.

1. **Settings → General → turn off Client hooks.** This removes the HUD's hooks from Claude Code, Codex and the others.
2. Quit SI Future HUD from its menu bar icon.
3. Drag **Applications → SI Future HUD** to the Trash.
4. *(Optional)* Remove the plugin and tools it added:

   ```bash
   claude plugin uninstall agent-hud@agent-hud
   claude plugin marketplace remove agent-hud
   claude mcp remove --scope user agent-hud
   codex mcp remove agent-hud
   ```

   With only the Codex desktop app, run the last line with the app's copy of `codex` (see step 6 above).

   In Chengxiaobang, disable or delete the SI Future HUD plugin on its Plugins page.

The HUD's own data is in `~/Library/Application Support/app.agenthud.next`. Delete that folder too if you want nothing left.

---

Next: **[Guide](GUIDE.md)** · **[Messaging](MESSAGING.md)** · **[Troubleshooting](TROUBLESHOOTING.md)**
