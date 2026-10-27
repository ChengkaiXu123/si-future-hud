# Changelog / 版本记录

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
