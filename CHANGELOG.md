# Changelog / 版本记录

## Unreleased / 未发布

**Messaging works from every window / 任何窗口都能传话**
- **`agent-hud-next send`**: any agent window that can run a command can send, including ones opened before the HUD was installed. It finds its own conversation from the agent process it runs under, and `--wait` prints the reply. / 新增 `agent-hud-next send` 命令行：能跑命令的窗口都能发，包括 HUD 装好之前就开着的；它自己认出发件对话，加 `--wait` 会把回信直接打印出来。
- **The `si-future-hud` skill** is written for Claude Code (`~/.claude/skills`) and Codex (`~/.agents/skills`) on launch; open windows pick it up at once. / 启动时给 Claude Code 和 Codex 写入 `si-future-hud` 技能，已开着的窗口立刻能用。
- **Chengxiaobang**: messages for a conversation that took messages before but isn't open now wait in a queue until it opens (an hour), instead of being refused; errors now say exactly what is missing (plugin not enabled, stopped, Chengxiaobang not running, no conversation open), and Settings shows it with a button that opens Chengxiaobang. / 程小帮：以前收过消息、现在没开着的对话改为排队送达；报错会准确说明缺哪一步，设置页也会显示，并能一键打开程小帮。
- A Chengxiaobang installed after the HUD works without restarting the HUD. / 先装 HUD、后装程小帮，不用重启 HUD 也能传话。

**Fixes / 修复**
- Messaging no longer depends on **Let agents arrange the HUD** or **Answer questions**. / 传话不再受「让智能体排版 HUD」和「回答提问」开关影响。
- A Claude Code sender is identified from its process's own session record, so `/clear` and `/resume` no longer send messages to itself or lose replies. / Claude Code 发件方改从进程的会话记录认人，`/clear`、`/resume` 后不再发给自己、回信也不会走丢。
- A receiver now prefers conversations that can take messages right now, however old. / 选收件对话时优先挑现在能收的，不再只看最近 7 天。
- A conversation waiting for your answer is reported as such first. / 对方在等你回答问题时，优先如实提示。
- At most two waiting messages per sending conversation, not per agent. / 等待确认的上限改为按发件对话计算。
- A relayed message is written in one go, so a plugin never reads it half-labelled. / 转达消息一次写完，插件不会读到半截。
- Plugins skip an unreadable message instead of stalling. / 插件遇到读不了的文件会跳过，不再卡住。
- Settings finds the `claude` that the Claude desktop app keeps, for users without the CLI. / 只装了 Claude 桌面版的用户，也能找到它自带的 `claude`。
- The MCP relay answers the client's handshake within two seconds, and accepts `null` for optional ids. / MCP 中继两秒内完成握手，可选参数传 `null` 也不报错。
- A Codex Desktop channel that fails to start no longer stops the app. / Codex 桌面版通道启动失败不再导致整个应用起不来。
- Chengxiaobang databases without a `kind` column no longer break its reading. / 程小帮数据库缺 `kind` 列时不再整体读取失败。

## v0.1.4 · 2026‑10‑26

The first SI Future HUD release, built on Agent HUD (open edition) at [`428dc88`](https://github.com/jazzenchen/agent-hud-open/commit/428dc88). / SI Future HUD 的第一个版本，基于 Agent HUD 开源版 `428dc88`。

**New / 新增**
- **Agent‑to‑agent messaging** between Claude Code, Codex desktop and Chengxiaobang, through `hud_send_message`. Every message waits for your Send, paper planes fly, and replies return to the exact conversation. / Claude Code、Codex 桌面版和程小帮之间互相传话：每条都要你点「发送」，配纸飞机动画，回信回到原对话。
- **Chengxiaobang (程小帮)**: usage, live state, questions on the HUD and messaging. Its plugin installs itself. / 接入程小帮：用量、实时状态、在 HUD 上回答提问、互相传话，插件自动安装。
- **Grok Bot and Cursor live states**, reply popups and question cards, plus Grok Bot team‑seat quotas. / Grok Bot 和 Cursor 的实时状态、回复提醒、提问卡片，以及 Grok Bot 团队席位的额度。
- **First‑launch set‑up**: the Claude Code plugin, HUD tools for Claude Code and Codex, and client hooks. / 首次启动自动配置：Claude Code 插件、Claude Code 和 Codex 的 HUD 工具、客户端回调。

**Distribution / 分发**
- Renamed **SI Future HUD**, with no hard‑coded paths. A whole ad‑hoc signature makes downloaded copies open on teammates' Macs. Apple silicon, macOS 12+. / 改名 SI Future HUD，不写死路径；完整的临时签名，组员下载后能直接打开。Apple 芯片，macOS 12 以上。

**Assets / 附件**
- `SI-Future-HUD_0.1.4_aarch64.dmg`
- `SI-Future-HUD-film-1080p60.mp4`: the product film, 2:25 / 产品宣传片
- `SHA256SUMS.txt`
