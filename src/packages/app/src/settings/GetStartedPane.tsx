// Local build: Get started, the guide SI Future HUD opens on its first launch
// (and after an update that brings it). Six short steps with the product's own
// pictures: where the HUD is, how its panel and bots read, how agents message
// each other, what was set up by itself, and the one step it may need from the
// person (enabling the plugin in 程小帮), each checked live.
import { useEffect, useState, type ReactNode } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CircleAlert,
  Circle,
  ExternalLink,
} from "lucide-react";
import type { Snapshot } from "@agent-hud/protocol";
import { Button } from "@agent-hud/ui/components/button";
import { HUDArtwork } from "@agent-hud/hud/HUDArtwork";
import { inUse } from "@agent-hud/hud/resting-agents";
import { snapshotLanguage } from "@agent-hud/ui/format";
import { useOpenSettingsPage } from "./SettingsNavigation";
import heroImage from "../assets/guide/hero.jpg";
import notchImage from "../assets/guide/notch-drop.webp";
import planeImage from "../assets/guide/paper-plane.webp";
import endImage from "../assets/guide/endcard.jpg";

type Words = [string, string];

interface ClaudeStatus {
  found: boolean;
  installed: boolean;
  sessions: number;
  /** The `claude` version when it is too old to run the plugin. */
  tooOld?: string | null;
}
interface McpStatus {
  ready: boolean;
  clients: { id: string; available: boolean; installed: boolean }[];
}
interface CodexStatus {
  found: boolean;
  running: boolean;
}
interface ChengxiaobangStatus {
  found: boolean;
  installed: boolean;
  sessions: number;
  enabled?: boolean | null;
  stopped?: boolean;
  running?: boolean;
}
interface Statuses {
  claude: ClaudeStatus | null;
  mcp: McpStatus | null;
  codex: CodexStatus | null;
  chengxiaobang: ChengxiaobangStatus | null;
}

const ask = async <T,>(command: string, args?: Record<string, unknown>) =>
  isTauri() ? invoke<T>(command, args).catch(() => null) : null;

/** What the guide checks, read again every few seconds while a step that
 * shows it is on screen, so a step done elsewhere ticks here. */
function useStatuses(watching: boolean): Statuses {
  const [statuses, setStatuses] = useState<Statuses>({
    claude: null,
    mcp: null,
    codex: null,
    chengxiaobang: null,
  });
  useEffect(() => {
    if (!watching) return;
    let dead = false;
    const read = async () => {
      const [claude, mcp, codex, chengxiaobang] = await Promise.all([
        ask<ClaudeStatus>("native_claude_plugin", { action: "status" }),
        ask<McpStatus>("native_mcp_clients", {
          action: "status",
          client: null,
        }),
        ask<CodexStatus>("native_codex_messages"),
        ask<ChengxiaobangStatus>("native_chengxiaobang_plugin", {
          action: "status",
        }),
      ]);
      if (!dead) setStatuses({ claude, mcp, codex, chengxiaobang });
    };
    void read();
    const timer = setInterval(() => void read(), 3_000);
    window.addEventListener("focus", read);
    return () => {
      dead = true;
      clearInterval(timer);
      window.removeEventListener("focus", read);
    };
  }, [watching]);
  return statuses;
}

type Mark = "done" | "todo" | "none" | "wait";

/** A step's notes: a short lead in bold, then a sentence or two. */
function Tips({ zh, items }: { zh: boolean; items: [Words, Words][] }) {
  const t = ([en, cn]: Words) => (zh ? cn : en);
  return (
    <ul className="guide-tips">
      {items.map(([lead, text], index) => (
        <li key={index}>
          <strong>{t(lead)}</strong>
          {zh ? "：" : ": "}
          {t(text)}
        </li>
      ))}
    </ul>
  );
}

function CheckRow({
  mark,
  title,
  detail,
  zh,
  children,
}: {
  mark: Mark;
  title: string;
  detail: string;
  zh: boolean;
  children?: ReactNode;
}) {
  const Icon = mark === "done" ? Check : mark === "todo" ? CircleAlert : Circle;
  return (
    <div className="guide-check" data-mark={mark}>
      <span className="guide-check-mark" aria-hidden>
        <Icon size={13} strokeWidth={2.4} />
      </span>
      <div className="guide-check-text">
        <strong>{title}</strong>
        <span>{detail}</span>
        {children}
      </div>
      <span className="guide-check-state">
        {mark === "done"
          ? zh
            ? "好了"
            : "Done"
          : mark === "todo"
            ? zh
              ? "要你来"
              : "Your step"
            : mark === "wait"
              ? zh
                ? "检查中"
                : "Checking"
              : zh
                ? "不涉及"
                : "Not here"}
      </span>
    </div>
  );
}

export function GetStartedPane({ snapshot }: { snapshot: Snapshot }) {
  const zh = snapshotLanguage(snapshot) === "zh";
  const t = ([en, cn]: Words) => (zh ? cn : en);
  const openPage = useOpenSettingsPage();
  const light =
    typeof document !== "undefined" &&
    document.documentElement.dataset.theme === "light";
  const [step, setStep] = useState(0);
  const statuses = useStatuses(step >= 3);
  const agents = snapshot.providers.filter(inUse);

  const claude = statuses.claude;
  const mcp = (id: string) =>
    statuses.mcp?.clients.find((client) => client.id === id) ?? null;
  const cxb = statuses.chengxiaobang;
  const cxbMark: Mark = !cxb
    ? "wait"
    : !cxb.found
      ? "none"
      : cxb.installed && cxb.enabled !== false && !cxb.stopped
        ? cxb.enabled === true || cxb.sessions > 0
          ? "done"
          : "wait"
        : "todo";

  // The plugin's card as 程小帮's plugin market shows it: its switch pulses
  // while it is off, and stands on once it is enabled.
  const pluginCard = (on: boolean) => (
    <div className="guide-plugin" data-on={on ? "" : undefined} aria-hidden>
      <span className="guide-plugin-icon">SI</span>
      <span className="guide-plugin-text">
        <strong>
          SI Future HUD <em>V1.1.0</em>
        </strong>
        <span>
          {t([
            "Brings messages from other agents into this conversation…",
            "把其他智能体经 SI Future HUD 发来的消息送进这个对话…",
          ])}
        </span>
      </span>
      <span className="guide-plugin-switch" />
    </div>
  );

  const steps: {
    title: Words;
    text: Words;
    image?: string;
    extra?: ReactNode;
  }[] = [
    {
      title: ["Welcome to SI Future HUD", "欢迎使用 SI Future HUD"],
      text: [
        "It lives beside the notch at the top of your screen and keeps the coding agents you use in one place: their quotas, what they are doing, and whether they wait for you.",
        "它住在屏幕顶部的刘海旁边，把你在用的编程智能体放在一处：额度还剩多少、正在做什么、是不是在等你，一眼就能看到。",
      ],
      image: heroImage,
      extra:
        agents.length > 0 ? (
          <div className="guide-agents">
            <span>{t(["Found on this Mac:", "在这台 Mac 上找到："])}</span>
            {agents.map((agent) => (
              <span key={agent.id} className="guide-agent">
                <HUDArtwork source={agent.id} size={16} light={light} />
                {agent.name}
              </span>
            ))}
          </div>
        ) : null,
    },
    {
      title: ["Point at the notch", "鼠标移到刘海上"],
      text: [
        "Move the pointer onto the notch (or the HUD at the top of the screen) and the panel drops open: every agent's quota, recent sessions and usage. Each little bot shows what its agent is doing: thinking, running a tool, or waiting for your answer.",
        "鼠标移到刘海（或屏幕顶部的 HUD）上，面板就会展开：每个智能体的额度、最近的会话和用量都在里面。每个小机器人的动作，就是它的智能体正在做的事：在想、在调用工具，还是在等你回答。",
      ],
      image: notchImage,
      extra: (
        <>
          <Tips
            zh={zh}
            items={[
              [
                ["Quotas", "额度"],
                [
                  "how much each agent has left, and when it resets.",
                  "每个智能体还剩多少、什么时候重置。",
                ],
              ],
              [
                ["Recent sessions", "最近会话"],
                [
                  "what each agent worked on lately; click one to go back to that conversation.",
                  "每个智能体最近在做什么；点一下就能回到那个对话。",
                ],
              ],
              [
                ["Bots", "小机器人"],
                [
                  "thinking, using a tool, or raising a hand while it waits for your answer.",
                  "在想、在调用工具，或者举手等你回答。",
                ],
              ],
              [
                ["Several displays", "多块屏幕"],
                [
                  "each display can show the HUD or not, and has its own style.",
                  "每块屏幕都可以单独选择显不显示 HUD、用什么样式。",
                ],
              ],
            ]}
          />
          <Button
            variant="ghost"
            size="sm"
            className="guide-link"
            onClick={() => openPage?.("display")}
          >
            {t(["Choose displays and styles", "选择屏幕和样式"])}
            <ArrowRight size={13} />
          </Button>
        </>
      ),
    },
    {
      title: ["Let your agents talk", "让智能体互相传话"],
      text: [
        "In any Claude Code, Codex or 程小帮 window, just say it, for example: “Use the HUD to ask Codex how far the front end has got.”",
        "在 Claude Code、Codex 或程小帮的任意窗口里，直接说一句就行，比如：「用 HUD 问问 Codex，前端做到哪了」。",
      ],
      image: planeImage,
      extra: (
        <Tips
          zh={zh}
          items={[
            [
              ["You confirm first", "先确认再送出"],
              [
                "a card on the HUD says who it goes to and which conversation. Send delivers it, Deny drops it; left alone, it is dropped after 10 minutes (Settings → General).",
                "HUD 上会弹出一张卡片，写着发给谁、哪个对话。点「发送」才送出，点「拒绝」就作废；一直不管，默认 10 分钟后作废（可在设置 → 通用里改）。",
              ],
            ],
            [
              ["Which conversation", "送进哪个对话"],
              [
                "unless the agent names one: in 程小帮, the conversation you opened last; in Claude Code and Codex, the one active most recently.",
                "没指定时，程小帮送进你最近打开的对话，Claude Code 和 Codex 送进最近活动的对话；也可以让智能体指名某个对话。",
              ],
            ],
            [
              ["When it's busy", "对方在忙"],
              [
                "if Codex is in the middle of a turn, or the conversation waits for your answer, the message waits and goes in once it is free.",
                "Codex 正在跑一轮，或者那个对话在等你回答问题时，消息会排队，等它空下来再送进去。",
              ],
            ],
            [
              ["Replies", "回信"],
              [
                "come back into the conversation that asked.",
                "对方的回复会回到发问的那个对话。",
              ],
            ],
            [
              ["How many", "排队上限"],
              [
                "up to 5 messages from one conversation can wait for you at a time.",
                "每个对话最多有 5 条消息同时等你确认。",
              ],
            ],
          ]}
        />
      ),
    },
    {
      title: [
        "Rather not press Send each time? SuperRoot",
        "不想每次都点发送？SuperRoot 模式",
      ],
      text: [
        "With SuperRoot on, messages between agents go at once, without your confirmation, and the paper plane still flies. Use it when you trust those conversations and want them to work things out together.",
        "打开 SuperRoot 后，智能体之间的消息不用你确认，直接送出，纸飞机照样会飞。适合你信任这些对话、想让它们自己协作的时候。",
      ],
      extra: (
        <>
          <Tips
            zh={zh}
            items={[
              [
                ["Only conversations the HUD knows", "只放行 HUD 认得的对话"],
                [
                  "a sender the HUD doesn't list still waits for your Send.",
                  "发件方要是 HUD 会话列表里的智能体对话；认不出来的，仍然要你点「发送」。",
                ],
              ],
              [
                ["It can't run away", "不会停不下来"],
                [
                  "at most 10 messages from one conversation in 10 minutes. Past that it pauses, and resumes once you press Send on one of its messages.",
                  "同一个对话 10 分钟内最多自动发 10 条。超出后暂停，等你亲手点一次「发送」才恢复。",
                ],
              ],
              [
                ["Think before turning it on", "想清楚再打开"],
                [
                  "while it is on, your agents can direct each other without asking you.",
                  "开着的时候，你的智能体能不经你确认互相指挥。",
                ],
              ],
            ]}
          />
          <span className="guide-where">
            {snapshot.settings.superRoot === true
              ? t(["SuperRoot is on now.", "SuperRoot 现在是开着的。"])
              : t([
                  "SuperRoot is off now (the default).",
                  "SuperRoot 现在是关着的（默认关闭）。",
                ])}
          </span>
          <Button
            variant="ghost"
            size="sm"
            className="guide-link"
            onClick={() => openPage?.("general")}
          >
            {t([
              "Turn it on or off in General → Messaging",
              "在「通用 → 传话」里开关",
            ])}
            <ArrowRight size={13} />
          </Button>
        </>
      ),
    },
    {
      // Run from its disk image or a copy macOS translocated, the app sets
      // nothing up: moving it into Applications is the step left.
      title:
        statuses.mcp && !statuses.mcp.ready
          ? ["Move it into Applications first", "先把它拖进「应用程序」"]
          : ["Set up for you", "这些已经替你配好"],
      text:
        statuses.mcp && !statuses.mcp.ready
          ? [
              "SI Future HUD is running from its disk image or a temporary copy macOS made, so nothing could be set up. Quit it, drag SI Future HUD into Applications, and open it from there: it sets your agents up then.",
              "SI Future HUD 现在是从安装包或 macOS 的临时副本里运行的，什么都没法自动配置。先退出，把 SI Future HUD 拖进「应用程序」文件夹，再从那里打开，就会自动配好。",
            ]
          : [
              "On its first launch the HUD set up your agents by itself. Windows that were already open work too: they pick up how to message from a skill the HUD wrote.",
              "第一次启动时，HUD 已经自动替你的智能体配好了。之前就开着的窗口也能用：HUD 写入的使用说明会被它们自动读到。",
            ],
      extra: (
        <>
          <div className="guide-checks">
            <CheckRow
              zh={zh}
              mark={
                !claude
                  ? "wait"
                  : !claude.found
                    ? "none"
                    : claude.installed &&
                        (!claude.tooOld || claude.sessions > 0)
                      ? "done"
                      : "todo"
              }
              title={t(["Claude Code plugin", "Claude Code 插件"])}
              detail={
                claude?.tooOld && claude.sessions === 0
                  ? t([
                      `Claude Code ${claude.tooOld} is too old to run the plugin: run claude update in Terminal (or update the desktop app), then start a new session.`,
                      `Claude Code ${claude.tooOld} 太旧，运行不了插件：在终端运行 claude update（用桌面 App 的，把 App 更新到最新），再新开一个会话。`,
                    ])
                  : claude?.installed
                    ? t([
                        `Installed; ${claude.sessions} session(s) running it.`,
                        `已装好，${claude.sessions} 个会话已加载。`,
                      ])
                    : claude && !claude.found
                      ? t([
                          "Claude Code isn't on this Mac.",
                          "这台 Mac 上没有 Claude Code。",
                        ])
                      : t([
                          "Lets the HUD bring messages and answers into Claude Code.",
                          "让 HUD 能把消息和回答送进 Claude Code。",
                        ])
              }
            />
            {(["claudeCode", "codex"] as const).map((id) => {
              const entry = mcp(id);
              const name = id === "claudeCode" ? "Claude Code" : "Codex";
              return (
                <CheckRow
                  key={id}
                  zh={zh}
                  mark={
                    !statuses.mcp
                      ? "wait"
                      : !entry || !entry.available
                        ? "none"
                        : entry.installed
                          ? "done"
                          : "todo"
                  }
                  title={t([`HUD tools in ${name}`, `${name} 的 HUD 工具`])}
                  detail={t([
                    `Lets ${name} message other agents and read the HUD.`,
                    `让 ${name} 能给其他智能体发消息、读取 HUD。`,
                  ])}
                />
              );
            })}
            <CheckRow
              zh={zh}
              mark={
                !statuses.codex
                  ? "wait"
                  : !statuses.codex.found
                    ? "none"
                    : statuses.codex.running
                      ? "done"
                      : "todo"
              }
              title={t(["Codex takes messages", "Codex 收消息"])}
              detail={
                statuses.codex &&
                statuses.codex.found &&
                !statuses.codex.running
                  ? t([
                      "Open the Codex desktop app; its conversations take messages while it runs.",
                      "打开 Codex 桌面版，它运行时里面的对话就能收消息。",
                    ])
                  : t([
                      "Through the Codex desktop app itself: no plugin needed.",
                      "直接通过 Codex 桌面版，不用装插件。",
                    ])
              }
            />
            <Button
              variant="ghost"
              size="sm"
              className="guide-link"
              onClick={() => openPage?.("agent-plugins")}
            >
              {t([
                "Details and fixes in Agent plugins",
                "详情和修复在「智能体插件」页",
              ])}
              <ArrowRight size={13} />
            </Button>
          </div>
          <Tips
            zh={zh}
            items={[
              [
                ["Claude Code version", "Claude Code 版本"],
                [
                  "the plugin needs 2.1.287 or newer; an older one is marked above (run claude update in Terminal, or update the desktop app).",
                  "插件要 2.1.287 或更新的版本，太旧的会在上面标出来（终端里运行 claude update；用桌面 App 的，把 App 更新到最新）。",
                ],
              ],
              [
                ["Windows already open", "已经开着的窗口"],
                [
                  "in Claude Code, run /reload-plugins once or start a new session. A window without the HUD's tool still sends with the command line the HUD's skill teaches it.",
                  "Claude Code 里运行一次 /reload-plugins，或者新开一个会话。没有 HUD 工具的旧窗口，也会按 HUD 写入的说明用命令行发消息。",
                ],
              ],
              [
                ["After an update", "升级之后"],
                [
                  "the HUD updates its Claude Code plugin by itself; sessions already open run /reload-plugins once.",
                  "HUD 会自动更新它的 Claude Code 插件；已经开着的会话运行一次 /reload-plugins。",
                ],
              ],
            ]}
          />
        </>
      ),
    },
    {
      title: ["One step for you: 程小帮", "还差你一步：程小帮"],
      text: [
        "The HUD already installed its plugin in 程小帮; 程小帮 runs another app's plugin only once you turn it on in its plugin market, so this switch is yours.",
        "HUD 已经把插件装进了程小帮，只差在程小帮的插件市场里把它打开：程小帮只有在你亲手开启之后，才会运行别的应用装进来的插件，所以这个开关得你来按。",
      ],
      extra: (
        <div className="guide-checks">
          <CheckRow
            zh={zh}
            mark={cxbMark}
            title={t([
              "SI Future HUD plugin in 程小帮",
              "程小帮里的 SI Future HUD 插件",
            ])}
            detail={
              !cxb
                ? t(["Checking…", "正在检查…"])
                : !cxb.found
                  ? t([
                      "程小帮 isn't on this Mac: nothing to do here.",
                      "这台 Mac 上没有程小帮，这一步可以跳过。",
                    ])
                  : cxbMark === "done"
                    ? cxb.sessions > 0
                      ? t([
                          `Enabled; ${cxb.sessions} conversation(s) take messages.`,
                          `已启用，${cxb.sessions} 个对话能收消息。`,
                        ])
                      : t([
                          "Enabled. Conversations take messages once 程小帮 opens them.",
                          "已启用。程小帮里点开过的对话就能收消息。",
                        ])
                    : cxb.stopped
                      ? t([
                          "程小帮 stopped loading it: load it again on its Plugins page.",
                          "程小帮停止加载了它：在程小帮的「插件」页重新加载。",
                        ])
                      : t(["Not enabled yet.", "还没启用。"])
            }
          >
            {cxb?.found && cxbMark === "done" && (
              <>
                <span className="guide-where">
                  {t([
                    "You'll find it in 程小帮 under Plugins → Manage → Enabled.",
                    "在程小帮的「插件 → 管理 → 已开启」里能看到它。",
                  ])}
                </span>
                {pluginCard(true)}
              </>
            )}
            {cxb?.found && cxbMark !== "done" && (
              <>
                <ol className="guide-howto">
                  <li>
                    {t([
                      "Open 程小帮 and go to Plugins in its sidebar.",
                      "打开程小帮，点左侧的「插件」。",
                    ])}
                  </li>
                  <li>
                    {t([
                      "Choose Manage at the top. SI Future HUD is already installed: find it under My plugins (or under My creations while it is off).",
                      "点上方的「管理」。SI Future HUD 已经替你装好了：在「我的插件」里找到它（还没开启时，可能在「我的创建」里）。",
                    ])}
                  </li>
                  <li>
                    {t([
                      "Turn on the switch on its right. Once it is on, it shows under Enabled. No restart needed.",
                      "打开它右边的开关。开关变成开启、它出现在「已开启」里，就好了，不用重启。",
                    ])}
                  </li>
                </ol>
                {pluginCard(false)}
                <Button
                  size="sm"
                  className="guide-open"
                  onClick={() =>
                    void ask("native_chengxiaobang_plugin", { action: "open" })
                  }
                >
                  {t(["Open 程小帮", "打开程小帮"])}
                  <ExternalLink size={13} />
                </Button>
              </>
            )}
          </CheckRow>
        </div>
      ),
    },
    {
      title: ["A message didn't arrive?", "消息没送到？这样查"],
      text: [
        "Start with Settings → Agent plugins: each agent has a card that says what is missing and how to fix it, and the HUD's errors name the cause too. The usual ones:",
        "先看「设置 → 智能体插件」：每个智能体一张卡片，缺哪一步、怎么修都写在上面；HUD 的报错也会直接说出原因。常见情况有这些：",
      ],
      extra: (
        <>
          <Tips
            zh={zh}
            items={[
              [
                ["Claude Code", "Claude Code"],
                [
                  "the version must be 2.1.287 or newer, and a session opened before the plugin runs /reload-plugins once.",
                  "版本要 2.1.287 或更新；插件装好之前就开着的会话，运行一次 /reload-plugins。",
                ],
              ],
              [
                ["Codex", "Codex"],
                [
                  "only conversations open in the Codex desktop app take messages (not one in a terminal or an editor); mid-turn, the message waits for the turn to end.",
                  "只有在 Codex 桌面版里打开着的对话能收消息，终端或编辑器里开的收不到；它正在跑一轮时，消息会等这一轮结束再送。",
                ],
              ],
              [
                ["程小帮", "程小帮"],
                [
                  "the plugin must be on and the conversation opened once. If it still doesn't arrive, restart 程小帮 once. If Settings says an earlier 程小帮 still runs, quit it fully (⌘Q), end the remaining 程小帮 and bun processes in Activity Monitor, then open it again.",
                  "插件要开着，要收消息的对话要点开过一次。还收不到的话，重启一次程小帮。设置里提示「旧的程小帮没退干净」时，彻底退出程小帮（⌘Q），在「活动监视器」里结束剩下的程小帮和 bun 进程，再重新打开。",
                ],
              ],
              [
                ["The HUD itself", "HUD 本身"],
                [
                  "open it from the Applications folder. Run straight from the disk image, it can't set anything up.",
                  "要从「应用程序」文件夹里打开；直接从安装包里打开，什么都配不了。",
                ],
              ],
            ]}
          />
          <Button
            variant="ghost"
            size="sm"
            className="guide-link"
            onClick={() => openPage?.("agent-plugins")}
          >
            {t(["Open Agent plugins", "打开「智能体插件」"])}
            <ArrowRight size={13} />
          </Button>
        </>
      ),
    },
    {
      title: ["You're all set", "准备好了"],
      text: [
        "Come back to this guide any time in Settings → Get started. If something doesn't work, Agent plugins says what is missing and how to fix it.",
        "以后随时可以在「设置 → 开始使用」重看这份引导。哪里不工作，就去「智能体插件」页看缺哪一步、怎么修。",
      ],
      image: endImage,
    },
  ];
  const last = steps.length - 1;
  const current = steps[step];
  return (
    <div className="guide">
      <div className="guide-progress" role="tablist">
        {steps.map((item, index) => (
          <button
            key={index}
            type="button"
            role="tab"
            aria-selected={index === step}
            aria-label={t(item.title)}
            className="guide-dot"
            data-state={
              index === step ? "current" : index < step ? "seen" : "next"
            }
            onClick={() => setStep(index)}
          />
        ))}
      </div>
      <article className="guide-card" key={step}>
        {current.image && (
          <div className="guide-media">
            <img src={current.image} alt="" />
          </div>
        )}
        <div className="guide-body">
          <span className="guide-step">
            {zh
              ? `第 ${step + 1} 步，共 ${steps.length} 步`
              : `Step ${step + 1} of ${steps.length}`}
          </span>
          <h2>{t(current.title)}</h2>
          <p>{t(current.text)}</p>
          {current.extra}
        </div>
      </article>
      <div className="guide-actions">
        {step < last ? (
          <Button variant="ghost" size="sm" onClick={() => setStep(last)}>
            {t(["Skip the guide", "跳过引导"])}
          </Button>
        ) : (
          <span />
        )}
        <div className="guide-actions-main">
          {step > 0 && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => setStep(step - 1)}
            >
              <ArrowLeft size={13} />
              {t(["Back", "上一步"])}
            </Button>
          )}
          {step < last ? (
            <Button size="sm" onClick={() => setStep(step + 1)}>
              {t(["Next", "下一步"])}
              <ArrowRight size={13} />
            </Button>
          ) : (
            <Button size="sm" onClick={() => openPage?.("display")}>
              {t(["Start using it", "开始使用"])}
              <Check size={13} />
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
