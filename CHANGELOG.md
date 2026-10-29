# Changelog / 版本记录

## Unreleased / 未发布

**A first-run guide, and only your agents / 新手引导，只显示你在用的智能体**
- **Get started**: on its first launch the HUD opens Settings on a six-step guide with the product's own pictures: where the HUD is, how its panel and bots read, how agents message each other, what was set up by itself (checked live), and the one step left for you, enabling the plugin in 程小帮, which ticks itself once done. It stays in Settings → Get started. / 新增「开始使用」：第一次启动时自动打开 6 步引导，配产品图示，讲清 HUD 在哪、面板和小机器人怎么看、怎么互相传话、哪些已自动配好（实时检查），以及还差你的一步：在程小帮里启用插件，启用后自动打勾；以后在「设置 → 开始使用」随时重看。
- **The HUD shows the agents you use**: a client that left only files behind (an old config, a stray executable) no longer gets a bot; one needs sessions, usage or an account that answers. / HUD 只显示你在用的智能体：只剩残留配置或程序、从没用过的客户端不再显示，需要有会话、用量或能读的账号。

**Messaging works from every window / 任何窗口都能传话**
- **`agent-hud-next send`**: any agent window that can run a command can send, including ones opened before the HUD was installed. It finds its own conversation from the agent process it runs under, and `--wait` prints the reply. / 新增 `agent-hud-next send` 命令行：能跑命令的窗口都能发，包括 HUD 装好之前就开着的；它自己认出发件对话，加 `--wait` 会把回信直接打印出来。
- **The `si-future-hud` skill** is written for Claude Code (`~/.claude/skills`) and Codex (`~/.agents/skills`) on launch; open windows pick it up at once. / 启动时给 Claude Code 和 Codex 写入 `si-future-hud` 技能，已开着的窗口立刻能用。
- **Chengxiaobang**: messages for a conversation that took messages before but isn't open now wait in a queue until it opens (an hour), instead of being refused; errors now say exactly what is missing (plugin not enabled, stopped, Chengxiaobang not running, no conversation open), and Settings shows it with a button that opens Chengxiaobang. / 程小帮：以前收过消息、现在没开着的对话改为排队送达；报错会准确说明缺哪一步，设置页也会显示，并能一键打开程小帮。
- A Chengxiaobang installed after the HUD works without restarting the HUD. / 先装 HUD、后装程小帮，不用重启 HUD 也能传话。
- **Settings**: Codex's card gets a **Messages** row saying whether the Codex desktop app can take messages (Codex needs no plugin); every HUD tools row says it carries messages too, and reads "Messages only" while Let agents arrange the HUD is off. / 设置页：Codex 卡片新增「收消息」一行，显示 Codex 桌面版能不能收（Codex 不用插件）；三张卡的「HUD 工具」说明都写明可以传话，「让智能体排版 HUD」关着时显示「仅可传话」。
- **Claude Code conversations opened before the plugin take messages** through Claude Code's own inbox (cross-session messaging, 2.1.224+), under their own inbound rules. / 插件装好之前就开着的 Claude Code 对话，也能通过 Claude Code 自带的收件箱收消息（按它自己的入站规则）。
- **A busy Codex conversation gets the message when its turn ends** (within half an hour) instead of refusing it. / 发给正在忙的 Codex 对话的消息，等这一轮结束自动送进去，不再拒收。
- The same message sent twice (a retry after a timeout) waits on one card; a Codex sender in a brand-new conversation gets its reply. / 同一条消息重发只出一张卡；全新的 Codex 对话发出的消息也能收到回信。

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
- An MCP entry left at a deleted build is replaced instead of counting as set up; the automatic set-up and Settings no longer run Claude's or Codex's command at the same time; a plugin disabled in Claude Code is named as such. / 指向已删除程序的 MCP 条目会被修正，不再算"已配好"；自动配置和设置页不再同时调用命令；插件在 Claude Code 里被停用时会如实提示。
- Chengxiaobang's questions keep their words after a HUD restart; a Chengxiaobang run that hasn't changed no longer rewrites its session's record on every database write. / HUD 重启后程小帮的提问卡片不再丢字；程小帮的运行状态没变时不再反复重写会话记录。
- Cursor: a conversation the dashboard lists no longer flickers away when its transcript is deleted, a busy state database no longer splits a conversation in two, and old transcripts aren't read on the first scan. / Cursor：仪表盘列出的对话不再因删掉聊天记录而来回消失；数据库忙时不再把一个对话拆成两个；首次扫描不再读很旧的记录。
- Questions worded again after a restart are read from their files once, not on every refresh; a question left by a run that never ended stops blocking messages after 30 minutes. / 重启后补回的提问文字只读一次；没结束的旧提问 30 分钟后不再挡住传话。
- The MCP relay counts the 4000 characters as the app does (emoji included), says that `hud_session` returns conversation messages, and survives the engine stopping mid-call. / MCP 中继按"字"计 4000 字（含表情），说明 `hud_session` 会返回对话内容，引擎中途退出时不再整个崩掉。

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
