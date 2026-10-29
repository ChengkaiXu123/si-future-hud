// The rows of Settings → Agents → Agent plugins as data: each row's status
// chip, its one action, what its ⋯ menu offers and the next step under it, in
// English and Chinese. The page draws them; a product build's own rows take
// their words from here too, so both builds read the same.

export type Words = readonly [en: string, zh: string];

/** working: in use. installed: in place, not in use yet. attention: one step
 * left. off: not set up (or still being read). failed: it broke. */
export type Tone = "working" | "installed" | "attention" | "off" | "failed";

export interface RowAction {
  id: string;
  label: Words;
  /** Why it cannot run now; the button is drawn disabled with this as its tip. */
  unavailable?: Words;
}

export interface RowState {
  tone: Tone;
  status: Words;
  /** The one action that matters now, or none when the item is working. */
  action: RowAction | null;
  /** A command is running: its words replace the action. */
  busy: Words | null;
  /** The ⋯ menu; removing is always here, never the main button. */
  menu: { id: string; label: Words; destructive?: boolean }[];
  /** The next step, shown under the description only when there is one. A
   * command in it is drawn as code with a Copy button. */
  next: { words: Words; command?: string } | null;
}

const checking: Words = ["Checking…", "正在检查…"];

export function sessionCount(count: number): Words {
  return [
    `Loaded in ${count} ${count === 1 ? "session" : "sessions"}`,
    `已在 ${count} 个会话中加载`,
  ];
}

export const reloadCommand = "/reload-plugins";

/** The line under a row whose command failed: "Couldn't install: …". */
export function failure(action: string, message: string): Words {
  const verbs: Record<string, Words> = {
    install: ["install", "安装"],
    remove: ["remove", "移除"],
    add: ["add", "添加"],
    "turn-on": ["turn on", "打开"],
  };
  const [en, zh] = verbs[action] ?? ["finish", "完成"];
  const line = message.replace(/^Error:\s*/, "");
  return [`Couldn't ${en}: ${line}`, `没能${zh}：${line}`];
}

export interface PluginInput {
  /** null while it is being read. */
  installed: boolean | null;
  /** Sessions that loaded it and checked in lately. */
  sessions: number;
  /** Whether the client's command line is on this Mac; null when unknown. */
  clientFound: boolean | null;
  running: "install" | "remove" | null;
  /** Installed from this page since it opened. */
  justInstalled: boolean;
}

/** A plugin a client loads per session, such as Agent HUD's Claude Code plugin. */
export function pluginRow(
  input: PluginInput,
  client = "Claude Code",
): RowState {
  const base: Pick<RowState, "busy" | "menu" | "next"> = {
    busy: null,
    menu: [],
    next: null,
  };
  const busy: Words | null =
    input.running === "install"
      ? ["Installing…", "正在安装…"]
      : input.running === "remove"
        ? ["Removing…", "正在移除…"]
        : null;
  if (input.installed === null && !busy)
    return { ...base, tone: "off", status: checking, action: null };
  if (!input.installed)
    return {
      ...base,
      tone: "off",
      status: ["Not installed", "未安装"],
      busy,
      action: {
        id: "install",
        label: ["Install", "安装"],
        unavailable:
          input.clientFound === false
            ? [`Install ${client} first.`, `请先安装 ${client}。`]
            : undefined,
      },
    };
  const loaded = input.sessions > 0;
  return {
    tone: loaded ? "working" : "installed",
    status: loaded ? sessionCount(input.sessions) : ["Installed", "已安装"],
    action: null,
    busy,
    menu: [
      { id: "remove", label: ["Remove plugin", "移除插件"], destructive: true },
    ],
    next:
      !loaded || input.justInstalled
        ? {
            words: [
              `New ${client} sessions load it when they start. In a session that's already open, run ${reloadCommand}.`,
              `新开的 ${client} 会话会自动加载。已经打开的会话，在其中运行 ${reloadCommand}。`,
            ],
            command: reloadCommand,
          }
        : null,
  };
}

export interface McpInput {
  /** The client's display name. */
  client: string;
  /** Whether this app may add the server (the installed app with its relay); null while reading. */
  setupReady: boolean | null;
  clientFound: boolean;
  added: boolean;
  /** Whether the client could be given a command to run by hand. */
  hasCommand: boolean;
  /** The app-side switch (settings.hudTools). */
  toolsOn: boolean;
  /** Why the switch cannot be turned on here (the demo, an unsupported system), if it cannot. */
  toolsUnavailable: Words | null;
  /** Whether the tools channel is listening now. */
  listening: boolean;
  running: "add" | "remove" | null;
  justAdded: boolean;
  /** The command was copied from this row a moment ago. */
  copied: boolean;
}

/** Agent HUD's MCP server in one client, with the app-side switch it needs. */
export function mcpRow(input: McpInput): RowState {
  const { client } = input;
  const busy: Words | null =
    input.running === "add"
      ? ["Adding…", "正在添加…"]
      : input.running === "remove"
        ? ["Removing…", "正在移除…"]
        : null;
  const menu: RowState["menu"] = [];
  if (input.hasCommand)
    menu.push({ id: "copy", label: ["Copy command", "拷贝命令"] });
  if (input.added && input.setupReady)
    menu.push({
      id: "remove",
      label: [`Remove from ${client}`, `从 ${client} 移除`],
      destructive: true,
    });
  const copied = input.copied
    ? {
        words: [
          "Command copied. Paste it into Terminal.",
          "已拷贝命令，粘贴到终端运行。",
        ] as Words,
      }
    : null;
  if (input.setupReady === null && !busy)
    return {
      tone: "off",
      status: checking,
      action: null,
      busy: null,
      menu: [],
      next: null,
    };
  if (!input.added) {
    const unavailable: Words | undefined = !input.setupReady
      ? [
          "Drag SI Future HUD out of its disk image into Applications and open it from there to add HUD tools.",
          "先把 SI Future HUD 从安装盘拖进「应用程序」，再从那里打开，才能添加 HUD 工具。",
        ]
      : !input.clientFound
        ? [`Install ${client} first.`, `请先安装 ${client}。`]
        : undefined;
    return {
      tone: "off",
      status: input.setupReady
        ? ["Not added", "未添加"]
        : ["Unavailable", "不可用"],
      action: { id: "add", label: ["Add", "添加"], unavailable },
      busy,
      menu,
      next: copied,
    };
  }
  if (!input.toolsOn)
    return {
      tone: "attention",
      status: ["Tools off", "工具已关闭"],
      action: {
        id: "turn-on",
        label: ["Turn on", "打开"],
        unavailable: input.toolsUnavailable ?? undefined,
      },
      busy,
      menu,
      next: copied,
    };
  if (!input.listening)
    return {
      tone: "failed",
      status: ["Not running", "未运行"],
      action: null,
      busy,
      menu,
      next: copied,
    };
  return {
    tone: "working",
    status: ["Ready", "可以使用"],
    action: null,
    busy,
    menu,
    next:
      copied ??
      (input.justAdded
        ? {
            words: [
              `Start a new ${client} session and ask it to change the HUD.`,
              `在 ${client} 里新开一个会话，说“把 HUD 改成……”。`,
            ],
          }
        : null),
  };
}

export interface ToolsSwitchInput {
  supported: boolean;
  enabled: boolean;
  listening: boolean;
  notice: string | null;
  demo: boolean;
}

/** The app-side switch: why it cannot change, and a line only when something is wrong. */
export function toolsSwitch(input: ToolsSwitchInput): {
  unavailable: Words | null;
  problem: Words | null;
} {
  const unavailable: Words | null = input.demo
    ? ["Not available in the demo.", "演示模式下不可用。"]
    : !input.supported
      ? [
          "This system can't serve HUD tools yet.",
          "这个系统暂时不能提供 HUD 工具。",
        ]
      : null;
  const problem: Words | null =
    input.supported && input.enabled && !input.listening
      ? [
          `Not running${input.notice ? `: ${input.notice}` : "."}`,
          `未运行${input.notice ? `：${input.notice}` : "。"}`,
        ]
      : !input.supported && !input.demo
        ? unavailable
        : null;
  return { unavailable, problem };
}

/** Local build: Agent HUD's 程小帮 plugin, written on launch; 程小帮 loads it
 * once it is enabled on 程小帮's own Plugins page. */
export function chengxiaobangPluginRow(input: {
  found: boolean | null;
  installed: boolean | null;
  sessions: number;
  running: boolean;
  /** What 程小帮 says of the plugin, when SI Future HUD can read it. */
  enabled?: boolean | null;
  stopped?: boolean;
  /** Whether 程小帮 itself runs now. */
  appRunning?: boolean;
}): RowState {
  const base: Pick<RowState, "menu" | "next"> = { menu: [], next: null };
  const busy: Words | null = input.running
    ? ["Installing…", "正在安装…"]
    : null;
  if (input.installed === null && !busy)
    return { ...base, tone: "off", status: checking, action: null, busy };
  if (input.found === false)
    return {
      ...base,
      tone: "off",
      status: ["Not found on this Mac", "这台 Mac 上没有程小帮"],
      action: null,
      busy,
    };
  if (!input.installed)
    return {
      ...base,
      tone: "off",
      status: ["Not installed", "未安装"],
      action: { id: "install", label: ["Install", "安装"] },
      busy,
    };
  const open = { id: "open", label: ["Open 程小帮", "打开程小帮"] as Words };
  if (input.enabled === false)
    return {
      ...base,
      tone: "attention",
      status: ["Enable it in 程小帮", "等你在程小帮里启用"],
      action: open,
      busy,
      next: {
        words: [
          "In 程小帮 → Plugins, turn on SI Future HUD once (no restart needed), then open the conversations that should take messages.",
          "在程小帮 → 插件里启用 SI Future HUD（只需一次，不用重启），再点开要收消息的对话。",
        ],
      },
    };
  if (input.stopped)
    return {
      ...base,
      tone: "attention",
      status: ["Stopped in 程小帮", "程小帮停止加载了它"],
      action: open,
      busy,
      next: {
        words: [
          "程小帮 stopped loading SI Future HUD: load it again in 程小帮 → Plugins.",
          "程小帮停止加载了 SI Future HUD：在程小帮 → 插件里重新加载它。",
        ],
      },
    };
  if (input.sessions > 0)
    return {
      ...base,
      tone: "working",
      status: sessionCount(input.sessions),
      action: null,
      busy,
    };
  if (input.appRunning === false)
    return {
      ...base,
      tone: "off",
      status: ["程小帮 isn't running", "程小帮没在运行"],
      action: open,
      busy,
      next: {
        words: [
          "Messages for 程小帮 wait until you open it and the conversation they go to.",
          "发给程小帮的消息会排队，等你打开程小帮和对应的对话时送达。",
        ],
      },
    };
  return {
    ...base,
    tone: "attention",
    status: ["Open a conversation in 程小帮", "在程小帮里点开一个对话"],
    action: open,
    busy,
    next: {
      words: [
        "Conversations take messages once 程小帮 opens them; ones it opened before wait in a queue until you open them again.",
        "程小帮里点开过的对话才能收消息；以前收过消息的对话会先排队，等你再点开时送达。",
      ],
    },
  };
}

/** Local build: Agent HUD's MCP server in 程小帮's mcp.json. */
export function chengxiaobangMcpRow(input: {
  found: boolean | null;
  listed: boolean | null;
  running: boolean;
}): RowState {
  const base: Pick<RowState, "menu" | "next"> = { menu: [], next: null };
  const busy: Words | null = input.running ? ["Adding…", "正在添加…"] : null;
  if (input.listed === null && !busy)
    return { ...base, tone: "off", status: checking, action: null, busy };
  if (input.listed)
    return {
      ...base,
      tone: "working",
      status: ["Ready to use", "可以使用"],
      action: null,
      busy,
    };
  return {
    ...base,
    tone: "off",
    status: ["Not added", "未添加"],
    action: {
      id: "add",
      label: ["Add", "添加"],
      unavailable:
        input.found === false
          ? ["Install 程小帮 first.", "请先安装程小帮。"]
          : undefined,
    },
    busy,
  };
}
