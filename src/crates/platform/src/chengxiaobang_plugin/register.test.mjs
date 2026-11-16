import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { stripTypeScriptTypes } from 'node:module'

let serial = 0
async function harness(stalled = false) {
  const source = (await readFile(new URL('./register.ts', import.meta.url), 'utf8')).replace('__AGENT_HUD_CHANNEL_JSON__', '"/channel"')
  const { register } = await import(`data:text/javascript;base64,${Buffer.from(stripTypeScriptTypes(source) + `\n// ${serial++}`).toString('base64')}`)
  const handlers = new Map(), ticks = [], files = new Map()
  let now = 1000000, calls = 0, code = 200
  let usage = { status: 'available', totalUsd: 100, usedUsd: 25, token: 'must-not-save', fiveHourWindow: { limitUsd: 10, usedUsd: 2, secret: 'must-not-save' } }
  const env = { CHENGXIAOBANG_API_BASE: 'http://localhost:1234/', CHENGXIAOBANG_API_TOKEN: 'secret-token' }
  const $ = {
    env: { get: async key => env[key] },
    clock: { now: async () => now, every: (_ms, fn) => ticks.push(fn) },
    session: { version: async () => ({version: '1.11.1'}), id: async () => 's1' },
    fs: { read: async path => { if (!files.has(path)) throw Error('missing'); return files.get(path) }, write: async (path, body) => files.set(path, body), exists: async () => false },
    http: { fetch: async (url, options) => {
      calls++
      if (stalled) await new Promise(() => {})
      assert.equal(url, 'http://localhost:1234/api/auth/ada/usage')
      assert.equal(options.headers['x-chengxiaobang-token'], 'secret-token')
      return {status: code, ok: code === 200, text: JSON.stringify({usage})}
    } },
  }
  register((event, ...args) => handlers.set(event, args.at(-1)))
  const start = () => handlers.get('session.start')($, {}, async () => ({}))
  const tick = async () => { ticks.forEach(fn => fn()); await new Promise(resolve => setImmediate(resolve)) }
  return { start, tick, files, env, calls: () => calls, advance: () => { now += 300000 }, code: value => {code = value}, usage: value => {usage = value} }
}

test('quota polling shares a five-minute gate and saves only quota fields', async () => {
  const h = await harness()
  await h.start(); await h.start(); await h.tick()
  assert.equal(h.calls(), 1)
  const saved = h.files.get('/channel/usage.json')
  assert.equal(JSON.parse(saved).usage.usedUsd, 25)
  assert.ok(!saved.includes('secret') && !saved.includes('must-not-save'))
  await h.tick(); assert.equal(h.calls(), 1)
  h.advance(); h.code(500); await h.tick()
  assert.equal(h.files.get('/channel/usage.json'), saved)
  await h.tick(); assert.equal(h.calls(), 2)
  h.advance(); h.code(401); await h.tick()
  assert.equal(JSON.parse(h.files.get('/channel/usage.json')).usage.status, 'signed_out')
})

test('missing credentials and fresh persisted reports avoid queries', async () => {
  const h = await harness()
  delete h.env.CHENGXIAOBANG_API_TOKEN
  await h.start(); await h.tick(); assert.equal(h.calls(), 0)
  h.env.CHENGXIAOBANG_API_TOKEN = 'secret-token'
  h.files.set('/channel/usage.json', JSON.stringify({at: 1000000, usage: {status: 'available'}}))
  await h.tick(); assert.equal(h.calls(), 0)
  h.advance(); await h.tick(); assert.equal(h.calls(), 1)
  const saved = h.files.get('/channel/usage.json')
  h.advance(); h.usage({status:'unavailable'}); await h.tick()
  assert.equal(h.files.get('/channel/usage.json'), saved)
})


test("a pending quota request does not block session polling", async () => {
  const h = await harness(true);
  await h.start();
  await h.tick();
  assert.ok(h.files.has("/channel/sessions/s1.json"));
  h.files.delete("/channel/sessions/s1.json");
  for (let i = 0; i < 5; i++) await h.tick();
  assert.ok(h.files.has("/channel/sessions/s1.json"));
  assert.equal(h.calls(), 1);
});
