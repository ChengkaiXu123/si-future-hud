import type { Register } from 'claude-code'

// Agent HUD's 程小帮 plugin (a mod: 程小帮 loads Claude Code mods).
// ChengxiaobangPlugin::prepare substitutes a JSON-escaped absolute path.
// It checks each session in with Agent HUD, takes the words Agent HUD leaves
// for it, and holds a single AskUserQuestion while Agent HUD runs: answered
// in Agent HUD or in this plugin's pane in 程小帮, whichever comes first, as a
// Claude question is; unanswered for 10 minutes, it goes to 程小帮's own window.
const CHANNEL = __AGENT_HUD_CHANNEL_JSON__
const POLL_MS = 1000
const CHECK_IN_EVERY = 5
const HOLD_MS = 10 * 60 * 1000
const SLICE_S = 5
const PANE = 'agent-hud'
// The HUD app itself (its hooks and MCP relays run with arguments or as another program).
const HUD = '\\.app/Contents/MacOS/agent-hud-next$'

// Sessions with a question held for Agent HUD: where its answer goes, and
// what it asks (for the pane in 程小帮).
const held = new Map<string, string>()
const asking = new Map<string, { question: string, header?: string, options: { label: string, description?: string }[], answer: string }>()

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const started = await next(e)
    // A client without this call still checks in.
    let version = ''
    try { ({ version } = await $.session.version()) } catch {}
    let checking = false
    let ticks = 0
    let last = ''
    $.clock.every(POLL_MS, () => {
      if (checking) return
      checking = true
      void (async () => {
        try {
          const id = await $.session.id()
          if (ticks++ % CHECK_IN_EVERY === 0 || id !== last) {
            last = id
            await $.fs.write(`${CHANNEL}/sessions/${id}.json`, JSON.stringify({ at: await $.clock.now(), version, channelVersion: 2, plugin: 2 }))
          }
          const folder = `${CHANNEL}/prompts/${id}`
          if (!(await $.fs.exists(folder))) return
          const entries = (await $.fs.list(folder)).filter(entry => entry.kind === 'file' && entry.name.endsWith('.json'))
          const pending: { path: string, cancel: string, prompt: Record<string, unknown> }[] = []
          for (const entry of entries) {
            const path = `${folder}/${entry.name}`
            // One unreadable file (half written, say) never holds the others back.
            let prompt: Record<string, unknown>
            try { prompt = JSON.parse(await $.fs.read(path)) } catch { continue }
            if (typeof prompt.text !== 'string' || prompt.takenAt !== undefined || prompt.cancelledAt !== undefined || prompt.expiredAt !== undefined) continue
            pending.push({ path, cancel: path.slice(0, -5) + '.cancel', prompt })
          }
          pending.sort((a, b) => Number(a.prompt.at) - Number(b.prompt.at) || String(a.prompt.id).localeCompare(String(b.prompt.id)))
          for (const { path, cancel, prompt } of pending) {
            const now = await $.clock.now()
            if (await $.fs.exists(cancel)) {
              await $.fs.write(path, JSON.stringify({ cancelledAt: now }))
              continue
            }
            if (typeof prompt.expiresAt === 'number' && prompt.expiresAt <= now) {
              await $.fs.write(path, JSON.stringify({ expiredAt: now }))
              continue
            }
            const takenAt = now
            await $.fs.write(path, JSON.stringify({ takenAt }))
            const answer = held.get(id)
            // A relayed message (kind "message") is never the held question's answer.
            if (answer && prompt.kind !== 'message') {
              // The question this session holds takes the words as its answer.
              await $.fs.write(answer, JSON.stringify({ text: String(prompt.text), at: now }))
              await $.fs.write(path, JSON.stringify({ takenAt, startedAt: await $.clock.now() }))
              continue
            }
            const became = async (what: Record<string, unknown>) => {
              try { await $.fs.write(path, JSON.stringify({ takenAt, ...what })) } catch {}
            }
            void $.prompt.submit({ text: String(prompt.text) }).then(async result => {
              const dropped = (result as { drop?: unknown } | undefined)?.drop
              await became(dropped === undefined ? { startedAt: await $.clock.now() } : { refused: String(dropped) })
            }, error => became({ refused: String(error) }))
          }
        } catch {
          // One failed pass retries on the next tick.
        } finally { checking = false }
      })()
    })
    return started
  })

  on('session.end', async ($, e, next) => {
    try { await $.fs.write(`${CHANNEL}/sessions/${await $.session.id()}.json`, JSON.stringify({ ended: await $.clock.now() })) } catch {}
    return next(e)
  })

  // Agent HUD's tools can't tell which 程小帮 session calls them: the relay
  // tool's call names it, so a reply comes back to this very session.
  on('tool.call', { tool: /hud_send_message$/ }, async ($, e, next) => {
    return next({ ...e, from_session: await $.session.id() })
  })

  on('tool.call', { tool: 'AskUserQuestion' }, async ($, e, next) => {
    const questions = (e as { questions?: unknown }).questions
    if (!Array.isArray(questions) || questions.length !== 1) return next(e)
    const first = questions[0] as { question?: unknown, header?: unknown, options?: unknown }
    if (typeof first?.question !== 'string' || !first.question.trim()) return next(e)
    const hud = await $.process.run(['/usr/bin/pgrep', '-f', HUD], { timeoutMs: 3000 }).catch(() => undefined)
    if (!hud || hud.exitCode !== 0) return next(e)
    const id = await $.session.id()
    const call = String((e as { tool_use_id?: unknown }).tool_use_id ?? '').replace(/[^A-Za-z0-9_-]/g, '')
    if (!call || held.has(id)) return next(e)
    const answer = `${CHANNEL}/answers/${call}.json`
    const options = (Array.isArray(first.options) ? first.options : [])
      .map(option => option as { label?: unknown, description?: unknown })
      .filter(option => typeof option?.label === 'string')
      .map(option => ({ label: String(option.label), description: typeof option.description === 'string' ? option.description : undefined }))
    held.set(id, answer)
    asking.set(id, { question: first.question, header: typeof first.header === 'string' ? first.header : undefined, options, answer })
    let placed = false
    try {
      const deadline = Date.now() + HOLD_MS
      while (Date.now() < deadline) {
        // While the session is on screen in 程小帮, the question shows in its
        // right pane too: an answer there or in Agent HUD, whichever comes first.
        if (!placed && (await $.session.surface()) === 'desktop') {
          const opened = await $.ui.open({ id: PANE, title: 'SI Future HUD', focus: true }).catch(() => undefined)
          placed = (opened as { isPlaced?: unknown } | undefined)?.isPlaced === true
          if (placed) $.ui.invalidate('ui.render')
        }
        const waited = await $.process.run(
          ['/bin/sh', '-c', 'i=0; while [ "$i" -lt "$1" ]; do [ -f "$0" ] && exit 0; sleep 0.5; i=$((i+1)); done; exit 1', answer, String(SLICE_S * 2)],
          { timeoutMs: (SLICE_S + 10) * 1000 },
        ).catch(() => undefined)
        if (waited && waited.exitCode === 0) {
          const { text } = JSON.parse(await $.fs.read(answer))
          return { result: `User has answered your questions: "${first.question}"="${String(text)}". You can now continue with the user's answers in mind.` }
        }
      }
      return next(e)
    } finally {
      held.delete(id)
      asking.delete(id)
      if (placed) {
        try { await $.ui.close({ id: PANE }) } catch {}
      }
    }
  })

  on('ui.render', { component: 'Pane' }, async ($, e, next) => {
    if ((e as { requestId?: unknown }).requestId !== PANE) return next(e)
    const { Box, Text, Button, Input } = $.ui.resolve(e)
    const ask = asking.get(await $.session.id())
    if (!ask) {
      return Box({ padding: 'md', children: [Text({ dimColor: true, children: ['没有等你回答的问题'] })] })
    }
    const reply = (text: string) => {
      void $.fs.write(ask.answer, JSON.stringify({ text, at: Date.now() }))
    }
    return Box({
      flexDirection: 'column',
      gap: 'md',
      padding: 'md',
      children: [
        Text({ size: 'caption', dimColor: true, children: [ask.header ?? '程小帮在问你'] }),
        Text({ size: 'heading', weight: 'semibold', children: [ask.question] }),
        Box({
          flexDirection: 'column',
          gap: 'sm',
          children: ask.options.map((option, index) => Box({
            key: `option-${index}`,
            flexDirection: 'column',
            gap: 'xs',
            children: [
              Button({ key: `choose-${index}`, label: option.label, ...(index === 0 ? { variant: 'primary' } : {}), onPress: () => reply(option.label) }),
              ...(option.description ? [Text({ size: 'caption', dimColor: true, children: [option.description] })] : []),
            ],
          })),
        }),
        Input({
          key: 'own-answer',
          label: '自己回答',
          placeholder: '输入你的回答，回车发送',
          value: '',
          onSubmit: (value: string) => {
            if (value.trim()) reply(value.trim())
          },
        }),
        Text({ size: 'caption', dimColor: true, children: ['也可以在 SI Future HUD 里回答'] }),
      ],
    })
  })
}
