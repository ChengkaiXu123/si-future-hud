<p align="right"><a href="TROUBLESHOOTING.md">English</a> · <strong>简体中文</strong></p>

# 常见问题与注意事项

我们遇到过的每一个坑，原因和解决办法都在这里。大多数只要一条命令或点一下。

**跳到：**[安装](#安装) · [Claude Code](#claude-code) · [Codex](#codex) · [程小帮](#程小帮) · [Cursor](#cursor) · [Grok Bot](#grok-bot) · [互相传话](#互相传话) · [HUD 本身](#hud-本身) · [卸载](#卸载)

## 安装

<details open>
<summary><b>提示「Apple 无法检查其是否包含恶意软件」或「无法验证开发者」</b></summary>

**原因：** 安装包是临时签名，没有经过苹果公证。

**解决：** 关掉提示，打开「系统设置 → 隐私与安全性」，在「已阻止使用 SI Future HUD」旁边点「仍要打开」，再点「打开」。只需要做一次。
</details>

<details open>
<summary><b>提示「已损坏，无法打开」</b></summary>

**原因：** 下载的文件带着隔离标记，未公证的 App 过不了这一关。

**解决：**

```bash
xattr -dr com.apple.quarantine "/Applications/SI Future HUD.app"
```

然后重新打开。
</details>

<details>
<summary><b>什么都没自动配置：没有 Claude 插件，也没有 HUD 工具</b></summary>

**原因：** 三种可能之一。
1. 直接在 DMG 窗口里打开了 App，macOS 会把它放在临时的只读位置运行。
2. 没装 `claude` 或 `codex` 命令。
3. 装在了 HUD 找不到的位置。

**解决：**
- 把 App 拖进「应用程序」，退出后从那里重新打开。
- 装好命令行版。HUD 会在 `~/.local/bin`、`~/.bun/bin`、`~/.npm-global/bin`、`/opt/homebrew/bin`、`/usr/local/bin`、`/usr/bin`、`/bin` 和你的 `PATH` 里找。
- 重开 HUD。每次启动都会重试，直到配置成功。

在终端用 `claude mcp list` 和 `codex mcp list` 检查，两个都应该列出 `agent-hud`。
</details>

<details>
<summary><b>在我的 Mac 上根本打不开</b></summary>

安装包只支持 **Apple 芯片**（arm64）和 **macOS 12 以上**。Intel 芯片的 Mac 请在那台机器上[从源码构建](../../README.zh-CN.md#从源码构建)。
</details>

## Claude Code

<details open>
<summary><b>面板显示「Claude Code 已退出登录」，可桌面 App 明明登录着</b></summary>

**原因：** HUD 是通过**终端里的 `claude` 命令**读 Claude 额度的：它运行 `get_usage`，再查 `claude auth status`。Claude 桌面 App 里的 Claude Code 由桌面 App 自己管理登录，**和终端命令的登录不共用**。命令行版自动更新或登录过期，都可能让终端掉登录，而桌面 App 仍然是登录状态。

**检查：**

```bash
claude auth status
```

**解决：** 在终端运行 `claude`，用同一个账号登录（或输入 `/login`）。HUD 下一次读取时额度就回来了。
</details>

<details>
<summary><b>「当前登录方式没有订阅额度（API key 或第三方平台）」</b></summary>

终端里的 `claude` 是用 API key 或云服务商登录的，这类登录没有订阅额度可显示。想看额度，请用你的 Claude 订阅账号登录。
</details>

<details>
<summary><b>某个对话收不到传话，或者 HUD 回答不了它的提问</b></summary>

**原因：** 这个对话是在 HUD 装好插件之前打开的。

**解决：** 在那个对话里输入 `/reload-plugins`，或者新开一个对话。新对话会自动加载插件。
</details>

<details>
<summary><b>「Claude Code 版本太旧，无法识别 --marketplace」</b></summary>

先在终端运行 `claude update` 升级，再到「设置 → 智能体插件」里安装插件。
</details>

## Codex

<details open>
<summary><b>Codex 一直收不到传话</b></summary>

只有 **Codex 桌面版**的对话能收传话。命令行版 Codex 会在 HUD 上显示状态，但收不了消息。

HUD 通过 `~/.codex/ipc/ipc.sock` 和 Codex 桌面版通信，这个通道只在桌面版有对话正在运行或等待时才连得上。
</details>

<details>
<summary><b>「已拒收：对话正忙」</b></summary>

Codex 的对话正在进行一轮工作时会拒收新消息，HUD 不会中途插话。等它这一轮做完，再让助手重发。
</details>

<details>
<summary><b>Codex 的审批弹不到 HUD 上</b></summary>

Codex 需要信任一次 HUD 的回调：Codex 询问时选允许。同时确认「设置 → 通用 → 客户端回调」是打开的。
</details>

<details>
<summary><b>Codex 发不出传话</b></summary>

当时没找到 `codex` 命令，所以 Codex 里没加上 HUD 工具。装好 Codex 命令行版后重开 HUD，用 `codex mcp list` 检查。
</details>

## 程小帮

<details open>
<summary><b>程小帮没出现，或者一直收不到传话</b></summary>

- 在程小帮的「插件」页启用一次 **SI Future HUD** 插件。HUD 已经把它放在 `~/.chengxiaobang/plugins/agent-hud/`。
- **程小帮每次重启后**，要点开某个对话一次。它的插件接口是按对话生效的，重启后没点开过的对话收不了传话。
- **HUD 升级后**，重启一次程小帮。它正在运行的 MCP 中转会一直用旧的工具定义，直到重启。
</details>

<details>
<summary><b>程小帮只显示用量，没有额度进度条</b></summary>

这是正常的。程小帮没有可读的额度窗口，所以它那一块显示近 24 小时的 token 用量，是 HUD 从它的本地数据库里只读读取的。
</details>

<details>
<summary><b>在 HUD 上回答程小帮的提问</b></summary>

开关在「设置 → 智能体插件 → 程小帮 → 回答提问」。提问放着 10 分钟不答，会退回到程小帮自己的窗口。程小帮的审批卡和计划卡没法在 HUD 上回答。
</details>

## Cursor

<details>
<summary><b>Cursor 没有额度，或者显示 401 / 403</b></summary>

- 在 **Cursor 桌面版里**登录。在网页或其他工具里登录，不会更新 HUD 读取的那份登录信息。
- 公司（Enterprise）账号重新登录后仍然失败的话，可能是公司策略禁止读取用量，这不在 HUD 的控制范围内。
</details>

## Grok Bot

<details>
<summary><b>Grok Bot 的额度看起来没更新</b></summary>

HUD 从 Grok Bot 自己的缓存里读额度，在 Grok Bot 里刷新一下，HUD 就会跟上。Grok Bot 的提问卡片只能看，回答要回到 Grok Bot 里。
</details>

## 互相传话

<details open>
<summary><b>助手说已经发了，但对方没收到</b></summary>

1. **你点「发送」了吗？** 每条都会先停在 HUD 上。
2. **对方现在能收吗？** 这些情况收不了：
   - 对话已经关了；
   - Codex 对话正忙；
   - 程小帮重启后没点开过那个对话；
   - 对话正在等你回答提问。

   消息会被拒绝，不会排队。
3. **等待的太多了？** 每个发送方最多只能有 2 条消息在等你处理。

详见 [互相传话 → 安全规则](MESSAGING.zh-CN.md#安全规则)。
</details>

<details>
<summary><b>回信送到了别的对话</b></summary>

回信会送到消息里写明的那个对话。如果发送方没指定 `session`，消息会发到接收方最近活跃的对话。让助手按收到的消息里写的 `to` 和 `session` 回信即可。
</details>

## HUD 本身

<details>
<summary><b>改了设置没生效</b></summary>

每块屏幕各自保存外观和位置。先确认改的是你正在看的那块屏幕：「设置 → 显示器」里列出了每块屏幕。
</details>

<details>
<summary><b>审批卡片还没回答就消失了</b></summary>

10 分钟没回答，请求会交还给助手，你仍然可以在助手那边回答。
</details>

<details>
<summary><b>百分比看起来是反的</b></summary>

「设置 → 通用 → 用量显示」可以在**已用（0% → 100%）**和**剩余（100% → 0%）**之间切换。颜色始终按已用量算。
</details>

<details>
<summary><b>面板太容易弹开，或者太难弹开</b></summary>

在「设置 → 显示器 → 悬停交互」里调整「展开延迟」和「收起延迟」，或者打开「按 Option 才展开」。
</details>

## 卸载

<details open>
<summary><b>干净卸载，不留后患</b></summary>

1. **设置 → 通用 → 关掉「客户端回调」。** **这一步很重要**：不做的话，Claude Code、Codex 等工具里会留下指向已删除程序的回调条目。
2. 退出 SI Future HUD，拖进废纸篓。
3. 可选：删掉插件和工具：

   ```bash
   claude plugin uninstall agent-hud@agent-hud
   claude plugin marketplace remove agent-hud
   claude mcp remove --scope user agent-hud
   codex mcp remove agent-hud
   ```

   然后在程小帮里停用插件。
4. 可选：删掉 `~/Library/Application Support/app.agenthud.next`。
</details>

---

还是解决不了？请提一个 [issue](https://github.com/ChengkaiXu123/si-future-hud/issues)，附上 HUD 的截图，以及 `claude auth status`、`codex mcp list` 的输出。记得去掉令牌、邮箱和你不想公开的路径。
