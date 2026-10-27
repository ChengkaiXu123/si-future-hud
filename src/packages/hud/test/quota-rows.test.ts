import { test } from "node:test";
import assert from "node:assert/strict";
import type { Quota } from "@agent-hud/protocol";
import {
  inRowOrder,
  quotaRowHidden,
  quotaRowKey,
  withRowShown,
} from "../src/quota-rows.ts";

const account = "account:aaa";
const other = "account:bbb";
const row = (window: string, accountId = account): Quota =>
  ({
    id: `claude:${accountId}:${window}`,
    vendor: "claude",
    name: window,
    usedPercent: 0,
    status: "normal",
    accountId,
    legacyWindowId: `${accountId}/${window}`,
  }) as Quota;

const session = row("claude-session");
const weekly = row("claude-weekly");
const fable = row("claude-weekly-fable");
const otherFable = row("claude-weekly-fable", other);

test("a layout writes a row by its Swift window id", () => {
  assert.equal(quotaRowKey(fable), `${account}/claude-weekly-fable`);
});

test("a row is hidden by its id, its window id, or its window alone", () => {
  assert.ok(quotaRowHidden(fable, [fable.id]));
  assert.ok(quotaRowHidden(fable, [`${account}/claude-weekly-fable`]));
  assert.ok(quotaRowHidden(fable, ["claude-weekly-fable"]));
  assert.ok(!quotaRowHidden(weekly, ["claude-weekly-fable"]));
});

test("rows follow a layout's order whichever key names them", () => {
  const ordered = inRowOrder(
    [session, weekly, fable],
    [fable.id, `${account}/claude-weekly`],
  );
  assert.deepEqual(
    ordered.map((r) => r.name),
    ["claude-weekly-fable", "claude-weekly", "claude-session"],
  );
});

test("showing a row hidden by its window alone keeps the other account's row hidden", () => {
  const hidden = withRowShown(["claude-weekly-fable", session.id], fable, [
    session,
    fable,
    otherFable,
  ]);
  assert.deepEqual(hidden, [session.id, `${other}/claude-weekly-fable`]);
  assert.ok(!quotaRowHidden(fable, hidden));
  assert.ok(quotaRowHidden(otherFable, hidden));
});
