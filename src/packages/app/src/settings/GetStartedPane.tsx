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
    },
    {
      title: ["Let your agents talk", "让智能体互相传话"],
      text: [
        "In any Claude Code, Codex or 程小帮 window, just say: “Use the HUD to ask Codex how far the front end has got.” A card shows on the HUD; nothing goes until you press Send. The reply comes back into the conversation that asked.",
        "在 Claude Code、Codex 或程小帮的任意窗口里说一句：「用 HUD 问问 Codex，前端做到哪了」。HUD 会弹出一张卡片，你点「发送」才会送出；回信会回到发问的那个对话。",
      ],
      image: planeImage,
    },
    {
      title: ["Set up for you", "这些已经替你配好"],
      text: [
        "On its first launch the HUD set up your agents by itself. Windows that were already open work too: they pick up how to message from a skill the HUD wrote.",
        "第一次启动时，HUD 已经自动替你的智能体配好了。之前就开着的窗口也能用：HUD 写入的使用说明会被它们自动读到。",
      ],
      extra: (
        <div className="guide-checks">
          <CheckRow
            zh={zh}
            mark={
              !claude
                ? "wait"
                : !claude.found
                  ? "none"
                  : claude.installed
                    ? "done"
                    : "todo"
            }
            title={t(["Claude Code plugin", "Claude Code 插件"])}
            detail={
              claude?.installed
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
              statuses.codex && statuses.codex.found && !statuses.codex.running
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
                <div className="guide-plugin" aria-hidden>
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
