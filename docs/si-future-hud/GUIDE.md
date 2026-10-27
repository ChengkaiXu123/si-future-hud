<p align="right"><strong>English</strong> · <a href="GUIDE.zh-CN.md">简体中文</a></p>

# Using SI Future HUD

Everything the HUD shows, and what you can do from it. To open the settings, click the SI Future HUD icon in the menu bar, then **Settings**.

<p align="center"><img src="media/notch-drop.webp" alt="Hovering the notch drops open the panel of every agent's quota" width="88%"></p>

## The three looks

Each display has its own look. Choose it in **Settings → Display** (*Set each display on its own*).

| Look | What you see | Good for |
| --- | --- | --- |
| **Notch** | A ring of light around the MacBook's notch. Hover it, and the notch drops open into the full panel. | MacBooks with a notch. It takes no screen space at all. |
| **Dynamic** | A row of agent logos on any screen edge, with the glow behind them. A logo becomes its **bot** while that agent works. | Seeing who is busy at a glance. |
| **Simple** | A small strip of **quota rings** on any edge, one per agent. Turn on **Show numbers** to see the percentages. | External displays and side edges. |

**Moving it.** Drag a Dynamic or Simple HUD to any of the four edges. It turns horizontal on the top and bottom and vertical on the sides. **Reset Position** puts it back.

<p align="center">
  <img src="media/looks.webp" alt="Dynamic mode: the logos beside the notch turn into working bots" width="49%">
  <img src="media/drag.webp" alt="Simple mode: the strip is dragged to the top edge and turns horizontal" width="49%">
</p>

## Reading it at a glance

- **Colour is quota.** Each agent's colour follows its tightest window. Under 70% used it is **green**, from 70% **yellow**, and from 90% **red**.
- **Motion is work.** The glow breathes faster while an agent works. The rings and logos move only for the agent that is busy.
- **Used or remaining.** *Settings → General → Usage display* switches every percentage between **Consumed (0% → 100%)** and **Remaining (100% → 0%)**. The colours always follow what is used.

### What the bots are doing

Each agent has its own bot. The descriptions below follow Claude's, **Clawd**. The others act out the same states in their own way, as in the grid below.

| State | Clawd |
| --- | --- |
| **Thinking** | thinks it over |
| **Reading files** | reads |
| **Editing code** | types at a laptop |
| **Running a command** | works at a desktop |
| **Compacting** its context | meditates |
| **Waiting for your approval** | points at you, with a `!` |
| **Asking you a question** | waves, with a `?` |
| **Done**: a reply is ready | celebrates |
| **Out of quota** | faints |

<p align="center"><img src="media/bots.webp" alt="Every agent's bot in each state" width="88%"></p>

## The panel

Hover the HUD (or the notch) and the panel opens. You can set it to open only while you hold **Option**, under *Settings → Display → Hover behavior*, where the hover and collapse delays are too.

Each agent has a block:

- **Name and account.** The signed‑in account and plan, marked *Current account*.
- **One row per quota window.** For example *Current session*, *Weekly* and *Weekly limit · Fable* for Claude, or *Included usage*, *Cursor Models* and *Other Models* for Cursor. Each row has a bar, a percentage and its reset time.
- **Five measures**, from the buttons at the top right of each block:

  | Button | Shows |
  | :---: | --- |
  | **%** | **Quota**: the share of each window |
  | 🔥 | **Burn rate**: percentage points per hour this cycle |
  | ◔ | **Reply rate**: tokens per second |
  | **Σ** | **Consumed**: tokens used this cycle |
  | **$** | **Est. cost** |

Below the blocks, optionally:
- **Token usage**: the last 24 hours, 7 days or 30 days, stacked by agent.
- **Session list**: recent conversations, the ones waiting for you first. Click one to open its details: turns, context, tokens and sub‑agents.

Turn these sections on or off in *Settings → Display → Expanded panel*. **Separate agents** gives each agent its own card instead of one shared panel.

<p align="center"><img src="media/usage-lens.webp" alt="The usage panel beside Claude, with a magnifier on the token count of a session" width="88%"></p>

## Approvals and questions

When an agent wants permission ("Run `swift test`?"), the request appears on the HUD with the full command:

- **Allow once**: let this one call through.
- **Deny**: refuse it. The agent is told.
- **Always allow**: allow this kind of call from now on, as the agent's own setting would.

Questions show their options, and you answer by clicking one. This includes the questions Claude asks in its desktop app.

- **You stay in charge.** A request you leave alone goes back to the agent after 10 minutes. The HUD never answers for you.
- **This needs Client hooks.** First launch turns it on, under *Settings → General*. Turn it off if you'd rather approve in each app.
- **Chengxiaobang's questions** can be answered on the HUD too. Toggle it in *Settings → Agent plugins → Chengxiaobang → Answer questions*.
- **Grok Bot's question cards** are shown on the HUD, but you answer them in Grok Bot.

<p align="center"><img src="media/approval.webp" alt="An approval card with the full command and Allow once" width="88%"></p>

## Replies

When an agent finishes, a **New reply** card slides out of the HUD. Click **Return to session** and that conversation comes to the front: its Claude or Codex thread, or its terminal tab. For Chengxiaobang, the app opens.

## Running low, running out, starting over

- **Before a window runs out,** the HUD warns you and the colour turns red.
- **When it is used up,** the agent's bot faints and the HUD says the quota is used up.
- **When the window resets,** a *Reset* notice appears, and the ring and colour go back to green.

<p align="center">
  <img src="media/quota-out.webp" alt="Claude's quota climbs to 100% and its bot faints" width="49%">
  <img src="media/reset.webp" alt="The quota resets and every ring empties" width="49%">
</p>

## Agents messaging each other

Claude Code, Codex (desktop) and Chengxiaobang can send each other messages through the HUD. Every message waits for your **Send**. → **[Messaging](MESSAGING.md)**

## More than one display

Every display keeps its own look and position, for example Notch on the MacBook and Simple on an external display. If a change "didn't apply", check that you edited the display you're looking at.

## Settings map

| Setting | Where |
| --- | --- |
| Look, size, glow, position, which sections the panel shows | Settings → **Display** (per display) |
| Consumed or remaining percentages, language, theme | Settings → **General** |
| **Client hooks** (approvals on the HUD) | Settings → **General → Clients** |
| **Launch at login** | Settings → **General → Startup** |
| Claude Code plugin, Chengxiaobang plugin, *Answer questions* | Settings → **Agent plugins** |
| HUD tools (MCP) for Claude Code and Codex, other agents | Settings → **Agent plugins** / **Other agents** |

---

Next: **[Messaging](MESSAGING.md)** · **[Troubleshooting](TROUBLESHOOTING.md)** · Back to the **[README](../../README.md)**
