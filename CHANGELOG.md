# Changelog / 版本记录

## v0.1.6 · 2026‑11‑02

What changed since 0.1.5: fixes for what colleagues met in their first week: messages that seemed sent but never showed up, and setups that quietly didn't work. / 相对 0.1.5：修好同事第一周实际遇到的问题——显示已发送却没收到的消息，以及悄悄没配好的安装。

**SuperRoot mode / SuperRoot 模式**

- **New in Settings → General → Messaging**: with SuperRoot on, messages between agents go at once, without your Send on the HUD. Only from agent sessions the HUD knows, and at most 10 from one session in 10 minutes, so two agents can't go on answering each other; past that, or for a sender the HUD doesn't know, it asks you as before and the card says why. Your agents can then direct each other without you, so turn it on only when you trust their sessions. Off unless you turn it on. / **设置 → 通用 → 传话里新增 SuperRoot 模式**：打开后，智能体之间的消息不用你在 HUD 上点「发送」就直接送出。只放行 HUD 认得的智能体会话，同一个会话 10 分钟内最多自动发 10 条，防止两个智能体来回对话停不下来；超出的、或者 HUD 不认得发件方的，照旧请你确认，卡片上会说明原因。开启后你的智能体能不经你确认互相指挥，只在信任这些会话时打开。默认关闭。
- **Up to 5 messages per conversation can wait for you**, up from 2. / **每个对话最多可以有 5 条消息等你确认**，之前是 2 条。

**Messages reach the conversation you're looking at / 消息送进你正在看的对话**

- **程小帮: the conversation you opened last, not the busiest one.** A message that names no conversation used to go to the one updated last, and 程小帮's scheduled tasks update theirs every hour, so the words landed in a background conversation nobody reads. It now goes to the conversation you opened last (程小帮's own record), including a new one that hasn't answered yet. / **程小帮：发给你最近打开的对话，而不是最忙的那个。** 没指定对话时，以前会发给最近更新的那个；程小帮的定时任务每小时都在更新自己的对话，消息就进了没人看的后台对话。现在按程小帮自己记录的「最近打开」来挑，刚新建、还没回答过的对话也算。
- **A reply goes back to the conversation that asked**, even one the HUD hasn't read yet; it is never sent to another conversation of that agent instead. / **回信回到发问的那个对话**，HUD 还没收录的新对话也一样，不会改投给同一智能体的别的对话。
- **Waiting on the command line (`send --wait`)**: a wait that was stopped (a shell's two‑minute limit, Esc) lets go of its conversation within seconds; before, it held that conversation's messages for up to half an hour and they were lost. A wait hears only about its own message, and nothing that arrives at its start or end is dropped. / **命令行等回信**：被中断的等待（shell 两分钟超时、按了 Esc）几秒内就会释放；以前它会把这个对话的消息截留最多半小时然后丢掉。每次等待只接收自己那条消息的通知，开始和结束时到达的消息也不再丢。
- The same words for two different conversations are two messages, not one. / 同一句话发给两个不同的对话，算两条消息。

**Says what's actually wrong / 报错说出真正的原因**

- **Claude Code too old for the plugin.** Claude Code before 2.1.287 can't run plugin hooks (Claude Mods), so its sessions never take messages. The HUD now names the version and says to run `claude update`, in the error, Settings and Get started, instead of asking for `/reload-plugins`. / **Claude Code 版本太旧。** 2.1.287 之前的 Claude Code 运行不了插件钩子（Claude Mods），会话永远收不到消息。现在报错、设置页和新手引导都会写出版本号并提示 `claude update`，不再让你去 `/reload-plugins`。
- **An earlier 程小帮 still running.** When an old 程小帮 backend didn't quit, conversations opened now can't take messages; Settings, the error and the message card say so and how to fix it. / **旧的程小帮没退干净。** 旧后台还在跑时，新打开的对话收不到消息；设置页、报错和消息卡片都会说明并给出处理办法。
- After `/clear` or a resume, the conversation it was stops counting as open at once. / `/clear` 或恢复会话后，原来的会话立刻不再算作开着。
- A message held for a busy Codex conversation that fails for another reason says why; a terminal Codex session is named as such. / Codex 忙时延后的消息因别的原因失败时，会说明真正原因；在终端里开的 Codex 会话会直接说明收不了。

**Display / 显示**

- **Choose which displays show the HUD.** Settings → Display → Screens has _Show the HUD on this display_ for each one, when more than one is connected. / **可以选哪块屏幕显示 HUD。** 接了不止一块屏幕时，设置 → 显示 → 屏幕里，每块屏幕都有「在这块屏幕显示 HUD」开关。

**Setup / 安装与配置**

- **Upgrading now updates the Claude Code plugin.** Claude Code runs its own copy of the plugin, made when it installed it, and earlier versions never refreshed that copy, so their plugin fixes didn't reach Claude Code. The plugin's version now follows the app's, and on launch the HUD has Claude Code update an older copy (new sessions use it; run `/reload-plugins` in open ones). / **升级时会一并更新 Claude Code 插件。** Claude Code 运行的是它安装插件时复制的那份，之前的版本从不刷新它，所以插件脚本的修复都没到达 Claude Code。现在插件版本跟随 App 版本，启动时发现旧的就让 Claude Code 更新（新会话直接用上，已开的会话运行一次 `/reload-plugins`）。
- A second message to a Codex conversation right after the first waits for the turn the first one started, instead of cutting into it. / 紧接着发给同一个 Codex 对话的第二条消息，会等第一条开启的那一轮结束，不会插进去。
- A command-line wait keeps a reply that arrived while the Mac slept, and a stopped wait no longer hears about another card. / 命令行等待期间电脑睡着了，已到的回信不会丢；也不会收到别的卡片的通知。

- **Clients installed with nvm, volta, fnm, asdf, mise or pnpm are found**: the HUD reads your login shell's PATH once, which an app opened from the Finder doesn't inherit. / **用 nvm、volta、fnm、asdf、mise 或 pnpm 装的客户端也能找到**：HUD 会读一次登录 shell 的 PATH（从访达打开的 App 拿不到它）。
- **Run from the disk image or a temporary copy**, Get started says to move the app into Applications first, and opens again once it's there. / **从安装包或临时副本里运行时**，新手引导会提示先拖进「应用程序」，拖进去之后会再次打开。
- The MCP relay finds its engine beside itself, without `AGENTHUD_NEXT_ENGINE`. / MCP 中继自己就能找到同目录的引擎，不再需要 `AGENTHUD_NEXT_ENGINE`。
- The `si-future-hud` skill explains that `send` exit 0 means handed to the HUD (delivered only after Send), and how to keep a long `--wait` from being cut off; the command finds a Claude Code session from its environment where processes can't be read. / `si-future-hud` 技能说明了 `send` 退出码 0 只表示交给了 HUD（点发送后才送达），以及怎样不让长时间的 `--wait` 被截断；读不了进程信息时，命令会从环境变量认出 Claude Code 会话。

## v0.1.5 · 2026‑10‑29

What changed since 0.1.4: install it, and the agents on this Mac can message each other from every window, old or new. / 相对 0.1.4：装好之后，这台 Mac 上的智能体在新旧窗口里都能互相传话。

**A first-run guide, and only your agents / 新手引导，只显示你在用的智能体**

- **Get started**: on its first launch the HUD opens Settings on a six-step guide with the product's own pictures: where the HUD is, how its panel and bots read, how agents message each other, what was set up by itself (checked live), and the one step left for you, enabling the plugin in 程小帮, which ticks itself once done. It stays in Settings → Get started. / 新增「开始使用」：第一次启动时自动打开 6 步引导，配产品图示，讲清 HUD 在哪、面板和小机器人怎么看、怎么互相传话、哪些已自动配好（实时检查），以及还差你的一步：在程小帮里启用插件，启用后自动打勾；以后在「设置 → 开始使用」随时重看。
- **Agents reach the HUD sooner after its launch**: watching every log folder at once (macOS restarted its event stream for each of a few dozen folders, seven seconds on the main thread) and a channel that listens before the first data pass: measured from 10.5 s to 1.8–2.9 s after launch (inside Codex's 10-second MCP start limit). / 启动后更快能调用 HUD：一次性监视所有日志目录（之前 macOS 每加一个目录都重启一次监视流，主线程卡 7 秒），本地通道不再等第一轮数据采集；实测从启动后 10.5 秒降到 1.8–2.9 秒（不再超过 Codex 的 10 秒 MCP 启动时限）。
- **The 程小帮 step says where to click**: Plugins → Manage → My plugins, with a picture of the plugin's card and its switch; the plugin describes itself in Chinese and says it carries messages. / 程小帮那一步写清了在哪里点：插件 → 管理 → 我的插件，并配插件卡片和开关的示意；插件介绍改成中文、写明负责传话。
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
