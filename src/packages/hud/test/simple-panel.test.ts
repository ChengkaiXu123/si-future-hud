import { test } from "node:test";
import assert from "node:assert/strict";
import {
  outsideSimplePanel,
  simplePanelRect,
} from "../src/scene/simple-panel.ts";

const strip = { x: 1000, y: 300, width: 40, height: 120 };

test("the Simple panel is its strip without the margin on the parked side", () => {
  assert.deepEqual(simplePanelRect(strip, "right"), {
    x: 1000,
    y: 300,
    width: 37,
    height: 120,
  });
  assert.deepEqual(simplePanelRect(strip, "left"), {
    x: 1003,
    y: 300,
    width: 37,
    height: 120,
  });
  assert.deepEqual(simplePanelRect(strip, "top"), {
    x: 1000,
    y: 303,
    width: 40,
    height: 117,
  });
  assert.deepEqual(simplePanelRect(strip, "bottom"), {
    x: 1000,
    y: 300,
    width: 40,
    height: 117,
  });
});

test("what grows out of the Simple panel is cut where the panel is, in its own corners", () => {
  assert.equal(
    outsideSimplePanel(strip, "right", 12),
    'path(evenodd, "M-100000 -100000H100000V100000H-100000Z' +
      "M1012 300H1025A12 12 0 0 1 1037 312V408A12 12 0 0 1 1025 420H1012A12 12 0 0 1 1000 408V312A12 12 0 0 1 1012 300Z" +
      '")',
  );
  // A radius past half the panel's width rounds it into a capsule, as border-radius does.
  assert.match(
    outsideSimplePanel(strip, "right", 40),
    /M1018\.5 300H1018\.5A18\.5 18\.5 0 0 1 1037 318\.5/,
  );
});
