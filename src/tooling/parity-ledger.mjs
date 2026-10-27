#!/usr/bin/env node
// Recounts the parity ledger. Every table row in docs/parity/c*.md carries one
// status cell ("differs", or "same ×18" for a summary row); this counts them
// per chunk and rewrites the totals table in docs/parity/README.md, so the
// summary always matches the rows. --check fails when it is out of date.
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const directory = join(
  fileURLToPath(new URL(".", import.meta.url)),
  "..",
  "..",
  "docs",
  "parity",
);
const statuses = [
  "same",
  "deliberate",
  "better",
  "fixed",
  "missing",
  "differs",
  "unclear",
];

function count(file) {
  const totals = Object.fromEntries(statuses.map((status) => [status, 0]));
  for (const line of readFileSync(join(directory, file), "utf8").split("\n")) {
    if (!line.startsWith("|")) continue;
    const cells = line.split("|").map((cell) => cell.trim());
    // A summary row counts several behaviors: "same ×18".
    for (const cell of cells) {
      const match = cell.match(/^([a-z]+)(?: ×(\d+))?$/);
      if (match && statuses.includes(match[1])) {
        totals[match[1]] += Number(match[2] ?? 1);
        break;
      }
    }
  }
  return totals;
}

const readme = readFileSync(join(directory, "README.md"), "utf8");
const header = `| Chunk | Open scope | Rows | ${statuses.join(" | ")} |`;
const rule = `| --- | --- | ---: | ${statuses.map(() => "---:").join(" | ")} |`;
const sum = Object.fromEntries(statuses.map((status) => [status, 0]));
const rows = [];
for (const file of readdirSync(directory)
  .filter((name) => /^c\d.*\.md$/.test(name))
  .sort()) {
  const id = file.split("-")[0];
  const scope = readme
    .match(
      new RegExp(
        `\\| \\[${id}\\]\\(${file.replace(".", "\\.")}\\) \\| ([^|]+) \\|`,
      ),
    )?.[1]
    .trim();
  if (!scope) throw new Error(`README has no totals row for ${file}`);
  const totals = count(file);
  const rowsInFile = statuses.reduce(
    (total, status) => total + totals[status],
    0,
  );
  for (const status of statuses) sum[status] += totals[status];
  rows.push(
    `| [${id}](${file}) | ${scope} | ${rowsInFile} | ${statuses.map((s) => totals[s]).join(" | ")} |`,
  );
}
const all = statuses.reduce((total, status) => total + sum[status], 0);
rows.push(
  `| **Total** | | **${all.toLocaleString("en-US")}** | ${statuses.map((s) => `**${sum[s]}**`).join(" | ")} |`,
);
const table = [header, rule, ...rows].join("\n");
const updated = readme.replace(
  /\| Chunk \| Open scope \|[^\n]*\n(\|[^\n]*\n)+/,
  `${table}\n`,
);
if (process.argv.includes("--check")) {
  if (updated !== readme) {
    console.error(
      "docs/parity/README.md totals are out of date; run pnpm run parity:ledger",
    );
    process.exit(1);
  }
} else {
  writeFileSync(join(directory, "README.md"), updated);
}
console.log(
  `Parity ledger: ${all} behaviors, ${sum.same + sum.fixed} match or are fixed, ${sum.missing + sum.differs} open.`,
);
