# What SI Future HUD changes / SI Future HUD 改了什么

SI Future HUD is a modified version of **[Agent HUD](https://github.com/jazzenchen/agent-hud-open)**, at upstream commit [`428dc88`](https://github.com/jazzenchen/agent-hud-open/commit/428dc88) (*Show the last 24 hours for agents with no quota window or balance*). Both are licensed under the Apache License 2.0. As section 4 of that license asks, this page lists every file we changed. Most of the changes also carry a `Local build` comment in the code that explains them.

SI Future HUD 是 **[Agent HUD](https://github.com/jazzenchen/agent-hud-open)** 的修改版，基于上游提交 [`428dc88`](https://github.com/jazzenchen/agent-hud-open/commit/428dc88)。两者都采用 Apache License 2.0。按照许可证第 4 条，这里列出了所有改动过的文件。大部分改动在代码里还有 `Local build` 注释说明。

The first commit of this repository imports Agent HUD exactly as it was at `428dc88`, and our changes are the commits after it. Agent HUD's full history lives in [its own repository](https://github.com/jazzenchen/agent-hud-open/commits/main). / 本仓库的第一个提交原样导入 `428dc88` 时的 Agent HUD，之后的提交是我们的改动。Agent HUD 的完整历史见[它自己的仓库](https://github.com/jazzenchen/agent-hud-open/commits/main)。

## Keeping up with Agent HUD / 跟上游同步

This repository's first commit has exactly the files of upstream `428dc88`, so upstream's later commits apply on top of ours one by one. / 本仓库第一个提交的文件和上游 `428dc88` 完全一样，所以上游之后的提交可以逐个接到我们的提交后面。

```bash
git fetch upstream
git cherry-pick 428dc88..upstream/main   # resolve any conflict in the files listed below, then: git cherry-pick --continue
```

Then run the same checks as CI (`pnpm run format:check`, `pnpm run check:platform`, `pnpm run typecheck`, `pnpm run test:ts`, `pnpm run test:mcp`, `cargo clippy --workspace --tests --locked -- -D warnings`, `cargo test --workspace --locked`). Build the app, use it for a while, and only then release. / 然后跑一遍和 CI 相同的检查，打包后实际用一阵，再发版本。

## In short / 概要

| Change | 改动 |
| --- | --- |
| **Agent‑to‑agent messaging.** The `hud_send_message` MCP tool, the host's `send_message` arm and `clients/agent_message.rs`. A Send/Deny card for every message, paper‑plane flights, delivery into Claude Code (plugin), Codex desktop (IPC `thread-follower-start-turn`) and Chengxiaobang (plugin), replies routed to the exact session, and safeguards. | **助手之间互相传话**：MCP 工具 `hud_send_message`、宿主的 `send_message`、`clients/agent_message.rs`。每条消息都有「发送 / 拒绝」卡片和纸飞机动画，投递到 Claude Code（插件）、Codex 桌面版（IPC）和程小帮（插件），回信精确回到原对话，并有一套安全规则。 |
| **Chengxiaobang (程小帮).** A new agent: usage from its local SQLite (read‑only), live state from its runs and tool calls, questions answerable on the HUD, and a plugin plus MCP entry written on every launch. | **程小帮**：新增的助手。从本地 SQLite 只读读取用量，从运行记录和工具调用判断实时状态，提问可以在 HUD 上回答；每次启动自动写好插件和 MCP 条目。 |
| **Codex desktop.** An IPC observer that runs without client hooks, and message delivery into desktop threads. | **Codex 桌面版**：不依赖客户端回调的 IPC 监听，以及往桌面版对话投递消息。 |
| **Grok Bot & Cursor.** Live states from their transcripts, reply popups, display‑only question cards, and Grok Bot team‑seat quotas. | **Grok Bot 和 Cursor**：从会话记录读取实时状态、回复提醒、只读的提问卡片，以及 Grok Bot 团队席位的额度。 |
| **Distribution.** Renamed SI Future HUD. No hard‑coded paths (the installed location is found at run time), first‑launch set‑up, MCP entries and the login item re‑pointed when the app moves, and a release build that teammates can open (`scripts/build-release.sh`). | **分发**：改名 SI Future HUD；不写死任何路径（运行时识别安装位置）；首次启动自动配置；App 挪位置后自动改指 MCP 条目和开机自启；打出来的安装包组员能直接打开（`scripts/build-release.sh`）。 |
| **Upstream checks pass.** Formatting (rustfmt, Prettier), the platform boundary (OS‑specific code only in per‑host files, with stubs where a host has no adapter), clippy with `-D warnings` and the tests. CI checks macOS and Ubuntu; the Windows job and workflow are removed because SI Future HUD ships for macOS only. | **通过上游的检查**：格式化（rustfmt、Prettier）、平台边界（按系统区分的代码只放在各系统自己的文件里，没实现的系统给空实现）、开启 `-D warnings` 的 clippy 和测试。CI 只检查 macOS 和 Ubuntu；SI Future HUD 只发 macOS 版，所以去掉了 Windows 的检查和工作流。 |
| **Small fixes.** Compacting look, peek height, 24‑hour usage in peeks, the navigation target for Chengxiaobang, and artwork aliases for non‑ASCII agent names. | **小修复**：压缩上下文的动画、悬浮预览的高度、预览里的 24 小时用量、程小帮的跳转目标、非英文名助手的图标别名。 |

## Files / 文件

### Agent readers (quota, usage, live state) / 读取各助手的额度、用量、实时状态

- `src/crates/agents/src/account.rs`
- `src/crates/agents/src/agent.rs`
- `src/crates/agents/src/antigravity/hooks.rs`
- `src/crates/agents/src/chengxiaobang/mod.rs` — **new**
- `src/crates/agents/src/chengxiaobang/transcript.rs` — **new**
- `src/crates/agents/src/claude/mod.rs`
- `src/crates/agents/src/codex/account.rs`
- `src/crates/agents/src/coding_plans/discover.rs`
- `src/crates/agents/src/coding_plans/tests.rs`
- `src/crates/agents/src/cursor/live.rs` — **new**
- `src/crates/agents/src/cursor/mod.rs`
- `src/crates/agents/src/grok/account.rs`
- `src/crates/agents/src/grok/bot.rs`
- `src/crates/agents/src/grok/live.rs` — **new**
- `src/crates/agents/src/grok/mod.rs`
- `src/crates/agents/src/hooks/configuration.rs`
- `src/crates/agents/src/hooks/request.rs`
- `src/crates/agents/src/lib.rs`
- `src/crates/agents/src/naming/model_names.rs`
- `src/crates/agents/src/naming/session_title.rs`
- `src/crates/agents/src/transcript/live.rs`
- `src/crates/agents/tests/golden/hook_formats.json`

### Core: messaging, questions, Chengxiaobang client / 核心：传话、提问、程小帮

- `src/crates/core/src/clients/agent_message.rs` — **new**
- `src/crates/core/src/clients/chengxiaobang.rs` — **new**
- `src/crates/core/src/clients/claude_desktop.rs`
- `src/crates/core/src/clients/mod.rs`
- `src/crates/core/src/clients/window_asks.rs` — **new**
- `src/crates/core/src/hook_events.rs`
- `src/crates/core/src/lib.rs`
- `src/crates/core/src/log_files.rs`
- `src/crates/core/src/presentation.rs`
- `src/crates/core/src/replies.rs`
- `src/crates/core/src/requests.rs`
- `src/crates/core/src/sessions.rs`
- `src/crates/core/src/settings.rs`
- `src/crates/core/src/source_db.rs`
- `src/crates/core/src/source_db/tests.rs`
- `src/crates/core/src/tests/claude_desktop.rs`

### Platform: Codex desktop delivery, Chengxiaobang plugin, hooks / 平台：Codex 桌面版投递、程小帮插件、回调

- `src/crates/platform/src/accounts/mod.rs`
- `src/crates/platform/src/chengxiaobang_plugin/mod.rs` — **new**
- `src/crates/platform/src/chengxiaobang_plugin/register.ts` — **new**
- `src/crates/platform/src/claude_plugin/mod.rs`
- `src/crates/platform/src/clients/codex/desktop.rs`
- `src/crates/platform/src/clients/codex/desktop/daemon.rs`
- `src/crates/platform/src/clients/codex/desktop/tests.rs`
- `src/crates/platform/src/clients/codex/desktop_io.rs`
- `src/crates/platform/src/clients/codex/message.rs` — **new**
- `src/crates/platform/src/clients/codex/message_windows.rs` — **new** (stub)
- `src/crates/platform/src/clients/codex/mod.rs`
- `src/crates/platform/src/hooks/mod.rs`
- `src/crates/platform/src/hooks/statusline_install.rs`
- `src/crates/platform/src/instance/mod.rs`
- `src/crates/platform/src/json_text.rs`
- `src/crates/platform/src/lib.rs`
- `src/crates/platform/src/local_tools/unix.rs`
- `src/crates/platform/src/os/mod.rs`
- `src/crates/platform/src/os/windows/pipe.rs`

### Host: first-launch set-up, MCP clients, Claude plugin, Chengxiaobang / 宿主：首次启动配置、MCP、Claude 插件、程小帮

- `src/crates/host/src/auto_setup.rs` — **new**
- `src/crates/host/src/chengxiaobang.rs` — **new**
- `src/crates/host/src/claude_plugin.rs`
- `src/crates/host/src/coordinator.rs`
- `src/crates/host/src/hud/drag.rs`
- `src/crates/host/src/hud/input.rs`
- `src/crates/host/src/hud/macos.rs`
- `src/crates/host/src/hud/mod.rs`
- `src/crates/host/src/hud/plans.rs`
- `src/crates/host/src/instance.rs`
- `src/crates/host/src/lib.rs`
- `src/crates/host/src/mcp_clients.rs`
- `src/crates/host/src/system/linux.rs`
- `src/crates/host/src/system/macos.rs`
- `src/crates/host/src/tools.rs`
- `src/crates/host/src/tray.rs`

### HUD presentation rules / HUD 呈现规则

- `src/crates/hud/src/presentation/app_rules.rs`
- `src/crates/hud/src/presentation/rules.rs`
- `src/crates/hud/src/presentation/tools.rs`
- `src/crates/hud/src/vendors.rs`

### Runtime / 运行时

- `src/crates/runtime/src/lib.rs`

### LAN / 局域网

- `src/crates/lan/src/identity/certificate.rs`
- `src/crates/lan/src/identity/macos.rs`

### MCP server (hud_send_message) / MCP 服务

- `src/apps/mcp/src/index.ts`

### Desktop app (Tauri), name and bundle / 桌面 App、名称和打包

- `src/apps/desktop/index.html`
- `src/apps/desktop/package.json`
- `src/apps/desktop/src-tauri/src/main.rs`
- `src/apps/desktop/src-tauri/src/installed/macos.rs` — **new**
- `src/apps/desktop/src-tauri/src/installed/linux.rs` — **new** (stub)
- `src/apps/desktop/src-tauri/src/installed/windows.rs` — **new** (stub)
- `src/apps/desktop/src-tauri/tauri.conf.json`
- `src/apps/desktop/src-tauri/tauri.macos.conf.json`

### HUD UI (React) / HUD 界面

- `src/packages/hud/src/HUDScene.tsx`
- `src/packages/hud/src/agent-artwork.ts`
- `src/packages/hud/src/assets/chengxiaobang.png` — **new**
- `src/packages/hud/src/bots/bot-spec.ts`
- `src/packages/hud/src/permissions/PermissionToolCard.tsx`
- `src/packages/hud/src/scene/AgentPeek.tsx`
- `src/packages/hud/src/scene/HUDMarkStrip.tsx`
- `src/packages/hud/src/scene/useHUDLayout.ts`
- `src/packages/hud/test/agent-artwork.test.ts` — **new**

### Settings UI / 设置界面

- `src/packages/app/src/App.tsx`
- `src/packages/app/src/lib/agent-plugins.ts`
- `src/packages/app/src/settings/AgentPluginsPane.tsx`
- `src/packages/app/src/statistics/ProviderCard.tsx`
- `src/packages/app/test/agent-plugins.test.ts`

### Protocol types / 协议类型

- `src/packages/protocol/src/index.ts`

### Packaging / 打包工具

- `src/tooling/package-resources.mjs`

### Workspace manifests / 工作区清单

- `Cargo.lock`
- `Cargo.toml`
- `package.json`
### CI / 持续集成

- `.github/workflows/ci.yml`: Rust checks on macOS and Ubuntu only / Rust 检查只跑 macOS 和 Ubuntu
- `.github/workflows/windows.yml`: removed, no Windows build / 已删除，不出 Windows 版

## Added in this repository / 本仓库新增

- `README.md`, `README.zh-CN.md`: the SI Future HUD product page / 产品页
- `README.upstream.md`, `README.upstream.zh-CN.md`: Agent HUD's own README, unchanged except for their language links / Agent HUD 原来的说明，只改了语言切换链接
- `docs/si-future-hud/`: install guide, user guide, messaging, troubleshooting, this file, media / 安装、使用、传话、常见问题、本文件和配图
- `scripts/build-release.sh`: the release build / 发布构建脚本
- `CHANGELOG.md`
