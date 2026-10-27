import { test } from "node:test";
import assert from "node:assert/strict";
import { inlineMarkdown } from "../src/text/inline-markdown.ts";

test("headings lose their marks and read bold; other lines stay as written", () => {
  assert.deepEqual(inlineMarkdown("## Plan\n\n- step one\n#hashtag"), [
    [{ kind: "strong", text: "Plan" }],
    [],
    [{ kind: "text", text: "- step one" }],
    [{ kind: "text", text: "#hashtag" }],
  ]);
});

test("emphasis, code and links are drawn inline", () => {
  assert.deepEqual(
    inlineMarkdown(
      "Run `make` then **ship** it, *carefully*, see [docs](https://example.com).",
    ),
    [
      [
        { kind: "text", text: "Run " },
        { kind: "code", text: "make" },
        { kind: "text", text: " then " },
        { kind: "strong", text: "ship" },
        { kind: "text", text: " it, " },
        { kind: "emphasis", text: "carefully" },
        { kind: "text", text: ", see " },
        { kind: "link", text: "docs", href: "https://example.com" },
        { kind: "text", text: "." },
      ],
    ],
  );
});

test("underscores inside words are not emphasis", () => {
  assert.deepEqual(inlineMarkdown("rename snake_case_name and _this_"), [
    [
      { kind: "text", text: "rename snake_case_name and " },
      { kind: "emphasis", text: "this" },
    ],
  ]);
});
