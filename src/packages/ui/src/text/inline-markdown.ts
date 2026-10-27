// A plan's Markdown as a card draws it: its lines as written, a heading in
// bold without its marks, and emphasis, code and links inline. Block syntax
// other than headings stays as typed. Follows agent-hud-native-open's
// PermissionPlanCard.rendered.

export type Segment =
  | { kind: "text" | "strong" | "emphasis" | "code"; text: string }
  | { kind: "link"; text: string; href: string };

const INLINE =
  /(`[^`\n]+`)|(\*\*[^*\n]+\*\*)|((?<![\p{L}\p{N}])__[^_\n]+__(?![\p{L}\p{N}]))|(\*[^*\s][^*\n]*\*)|((?<![\p{L}\p{N}])_[^_\s][^_\n]*_(?![\p{L}\p{N}]))|(\[[^\]\n]+\]\([^)\s]+\))/gu;

function inline(line: string, strong: boolean): Segment[] {
  if (strong) return [{ kind: "strong", text: line }];
  const segments: Segment[] = [];
  let at = 0;
  for (const match of line.matchAll(INLINE)) {
    const index = match.index ?? 0;
    if (index > at)
      segments.push({ kind: "text", text: line.slice(at, index) });
    const token = match[0];
    if (match[1]) segments.push({ kind: "code", text: token.slice(1, -1) });
    else if (match[2] || match[3])
      segments.push({ kind: "strong", text: token.slice(2, -2) });
    else if (match[4] || match[5])
      segments.push({ kind: "emphasis", text: token.slice(1, -1) });
    else {
      const close = token.indexOf("](");
      segments.push({
        kind: "link",
        text: token.slice(1, close),
        href: token.slice(close + 2, -1),
      });
    }
    at = index + token.length;
  }
  if (at < line.length) segments.push({ kind: "text", text: line.slice(at) });
  return segments;
}

/** Each line of `text` as its segments; an empty line is an empty list. */
export function inlineMarkdown(text: string): Segment[][] {
  return text.split("\n").map((line) => {
    const heading = /^(#{1,6}) (.*)$/.exec(line);
    if (heading) {
      const title = heading[2].trim();
      return title ? inline(title, true) : [];
    }
    return inline(line, false);
  });
}
