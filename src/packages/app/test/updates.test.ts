import { test } from "node:test";
import assert from "node:assert/strict";
import {
  isUpdateEvent,
  lastChecked,
  megabytes,
  percent,
  pick,
} from "../src/lib/updates.ts";

test("the updater's words are read in Settings' language, else the other", () => {
  assert.equal(
    pick({ zh: "已是最新版本", en: "Up to date" }, true),
    "已是最新版本",
  );
  assert.equal(
    pick({ zh: "已是最新版本", en: "Up to date" }, false),
    "Up to date",
  );
  assert.equal(pick({ zh: "", en: "Only English" }, true), "Only English");
  assert.equal(pick(null, true), "");
});

test("the last check reads as minutes or hours ago within a day", () => {
  const now = new Date("2026-11-26T08:00:00Z");
  assert.equal(lastChecked(null, true, now), "还没有检查过");
  assert.equal(
    lastChecked("2026-11-26T07:59:40Z", true, now),
    "上次检查：刚刚",
  );
  assert.equal(
    lastChecked("2026-11-26T07:55:00Z", false, now),
    "Last checked 5 minutes ago",
  );
  assert.equal(
    lastChecked("2026-11-26T07:59:00Z", false, now),
    "Last checked 1 minute ago",
  );
  assert.equal(
    lastChecked("2026-11-26T02:00:00Z", true, now),
    "上次检查：6 小时前",
  );
  assert.match(lastChecked("2026-11-20T02:00:00Z", true, now), /^上次检查：/);
  assert.equal(lastChecked("garbage", true, now), "");
});

test("only the updater's events carry its state", () => {
  assert.ok(isUpdateEvent({ updates: { phase: "ready" } }));
  assert.ok(!isUpdateEvent({ updates: null }));
  assert.ok(!isUpdateEvent({ flight: {} }));
  assert.ok(!isUpdateEvent(null));
  assert.ok(!isUpdateEvent("updates"));
});

test("sizes and progress", () => {
  assert.equal(megabytes(47_185_920), "47.2 MB");
  assert.equal(percent({ done: 25, total: 100 }), 25);
  assert.equal(percent({ done: 99, total: 100 }), 99);
  assert.equal(percent({ done: 5, total: 0 }), 0);
  assert.equal(percent(null), 0);
});
