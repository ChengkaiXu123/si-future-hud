import type { Register } from 'claude-code'

// Agent HUD's Claude Code plugin. ClaudeChannel::prepare_plugin substitutes a JSON-escaped absolute path.
// This is a hooks module: no Node imports, no new Claude process, no CLI -p.
const CHANNEL = __AGENT_HUD_CHANNEL_JSON__
const POLL_MS = 1000
const CHECK_IN_EVERY = 5

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const started = await next(e)
    const { version } = await $.session.version()
    let checking = false
    let ticks = 0
    let last = ''
    $.clock.every(POLL_MS, () => {
      if (checking) return
      checking = true
      void (async () => {
        try {
          // /clear and resume can change identity without another session.start.
          const id = await $.session.id()
          if (ticks++ % CHECK_IN_EVERY === 0 || id !== last) {
            last = id
            await $.fs.write(`${CHANNEL}/sessions/${id}.json`, JSON.stringify({ at: await $.clock.now(), version, channelVersion: 2 }))
          }
          const folder = `${CHANNEL}/prompts/${id}`
          if (!(await $.fs.exists(folder))) return
          const entries = (await $.fs.list(folder)).filter(entry => entry.kind === 'file' && entry.name.endsWith('.json'))
          const pending: { path: string, cancel: string, prompt: Record<string, unknown> }[] = []
          for (const entry of entries) {
            const path = `${folder}/${entry.name}`
            const prompt = JSON.parse(await $.fs.read(path))
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
            // Claim before submit, so reloads never submit a prompt twice. Taken is
            // deliberately not described as started: submit waits for session idle.
            await $.fs.write(path, JSON.stringify({ takenAt }))
            const became = async (what: Record<string, unknown>) => {
              try { await $.fs.write(path, JSON.stringify({ takenAt, ...what })) } catch {}
            }
            // A cancellation crossing the claim is acknowledged if submit has not
            // begun; after submit the service truthfully refuses to promise withdrawal.
            if (await $.fs.exists(cancel)) { await became({ cancelledAt: await $.clock.now() }); continue }
            const submitted = prompt.asUser === true
              ? $.prompt.submit({ text: String(prompt.text), asUser: true })
              : $.prompt.submit({ text: String(prompt.text) })
            // Never await a prompt waiting for idle: check-ins must stay alive.
            void submitted.then(async result => {
              const dropped = (result as { drop?: unknown }).drop
              await became(dropped === undefined ? { startedAt: await $.clock.now() } : { refused: String(dropped) })
            }, error => became({ refused: String(error) }))
          }
        } catch {
          // One failed pass retries on the next clock; expired check-in hides reachability.
        } finally { checking = false }
      })()
    })
    return started
  })
  on('session.end', async ($, e, next) => {
    try { await $.fs.write(`${CHANNEL}/sessions/${e.sessionId}.json`, JSON.stringify({ ended: await $.clock.now() })) } catch {}
    return next(e)
  })
}
