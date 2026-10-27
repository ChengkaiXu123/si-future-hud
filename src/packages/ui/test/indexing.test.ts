import { test } from "node:test";
import assert from "node:assert/strict";
import { indexingText } from "../src/text/indexing.ts";

test("nothing shows outside a run", () => {
  assert.equal(indexingText(undefined, "en"), null);
  assert.equal(
    indexingText({ kind: null, bytesDone: 1, bytesTotal: 2 }, "en"),
    null,
  );
});

test("a run says what it does and how far it is, in bytes", () => {
  assert.deepEqual(
    indexingText({ kind: "reindex", bytesDone: 450, bytesTotal: 1_000 }, "en"),
    {
      fraction: 0.45,
      title: "Rebuilding usage history · 45%",
      detail: "Tokens and costs are incomplete until it finishes",
    },
  );
  assert.equal(
    indexingText({ kind: "first", bytesDone: 0, bytesTotal: 0 }, "zh")?.title,
    "正在建立用量记录 · 0%",
  );
});

test("a run shows less than 100% until the core ends it", () => {
  const text = indexingText(
    { kind: "reindex", bytesDone: 1_000, bytesTotal: 1_000 },
    "zh",
  );
  assert.equal(text?.fraction, 0.99);
  assert.equal(text?.title, "正在重新整理用量记录 · 99%");
});
