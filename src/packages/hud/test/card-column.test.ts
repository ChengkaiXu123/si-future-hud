import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { cardColumn } from "../src/card-column.ts";

const hud = fileURLToPath(new URL("../src/", import.meta.url));

test("a card's column starts at zero, so no line in it widens the card", () => {
  assert.deepEqual(cardColumn(9), {
    display: "grid",
    gridTemplateColumns: "minmax(0, 1fr)",
    gap: 9,
  });
});

// A HUD card has a fixed width and clips what sticks out. A grid that sizes its own column makes it as wide as its
// widest uncut line, which pushed the count, the times and the buttons past the card's edge, so every grid in a
// request or alert card says how wide its columns are.
test("every grid in the request and alert cards sets its columns", () => {
  const files = [
    "RequestActions.tsx",
    ...readdirSync(`${hud}permissions`).map((name) => `permissions/${name}`),
    ...readdirSync(`${hud}alerts`).map((name) => `alerts/${name}`),
  ].filter((name) => name.endsWith(".tsx"));
  const loose: string[] = [];
  for (const name of files) {
    const source = readFileSync(`${hud}${name}`, "utf8");
    for (const match of source.matchAll(/display:\s*"grid"/g)) {
      const start = source.lastIndexOf("{", match.index);
      const end = source.indexOf("}", match.index);
      const style = source.slice(start, end);
      // A box that centres one icon holds no line.
      if (!/gridTemplateColumns|placeItems/.test(style)) {
        loose.push(
          `${name}:${source.slice(0, match.index).split("\n").length}`,
        );
      }
    }
  }
  assert.deepEqual(loose, []);
});
