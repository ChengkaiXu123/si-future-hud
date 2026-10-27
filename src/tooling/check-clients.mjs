#!/usr/bin/env node
// Keeps what Agent HUD knows about each client in src/crates/agents
// (docs/agent-contract.md). It reports where code outside that
// crate still names a client by its id, against the baseline in
// client-literals.json, and refuses operating-system conditions inside the
// crate, which holds every OS's facts as data instead.
//
//   node src/tooling/check-clients.mjs            report; fails only on OS conditions in agents
//   node src/tooling/check-clients.mjs --update   write today's counts as the baseline
//   node src/tooling/check-clients.mjs --strict   also fail where a file names clients more than its baseline
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..");
const agents = join(root, "src", "crates", "agents");
const baselinePath = join(root, "src", "tooling", "client-literals.json");
const skipped = new Set(["node_modules", "target", "dist", "gen", "assets"]);
const tests =
  /(^|[/\\])(tests?|fixtures)([/\\]|$)|(^|_)tests?\.rs$|\.test\.ts$/;

function* files(directory, pattern) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (skipped.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* files(path, pattern);
    else if (pattern.test(entry.name)) yield path;
  }
}

// The ids come from the descriptors themselves.
const ids = new Set();
for (const path of files(join(agents, "src"), /^mod\.rs$/)) {
  for (const [, id] of readFileSync(path, "utf8").matchAll(
    /^\s*id: "([^"]+)",/gm,
  ))
    ids.add(id);
}
const literal = new RegExp(
  `["'](${[...ids].map((id) => id.replace(/[-]/g, "\\-")).join("|")})["']`,
  "g",
);

const counts = {};
for (const base of ["src/crates", "src/apps", "src/packages"]) {
  for (const path of files(join(root, base), /\.(rs|ts|tsx|mjs)$/)) {
    const name = relative(root, path).split(sep).join("/");
    if (path.startsWith(agents + sep) || tests.test(name)) continue;
    const found = readFileSync(path, "utf8").match(literal)?.length ?? 0;
    if (found) counts[name] = found;
  }
}

const problems = [];
for (const path of files(join(agents, "src"), /\.rs$/)) {
  const text = readFileSync(path, "utf8");
  if (
    /#\[cfg(_attr)?\s*\([^\]]*(target_os|unix|windows|target_family)|cfg!\s*\(|std::env::consts::OS/.test(
      text,
    )
  )
    problems.push(
      `${relative(root, path)}: agents holds OS facts as data, not OS conditions`,
    );
}

const args = process.argv.slice(2);
if (args.includes("--update")) {
  const sorted = Object.fromEntries(Object.entries(counts).sort());
  writeFileSync(baselinePath, JSON.stringify(sorted, null, 2) + "\n");
}
const baseline = JSON.parse(readFileSync(baselinePath, "utf8"));
// A crate or package and its first folder or file, such as src/crates/core/src/naming.
const area = (name) =>
  name
    .split("/")
    .slice(0, 5)
    .join("/")
    .replace(/\.(rs|tsx?)$/, "");
const byArea = {};
let total = 0;
for (const [name, count] of Object.entries(counts)) {
  byArea[area(name)] = (byArea[area(name)] ?? 0) + count;
  total += count;
  if (args.includes("--strict") && count > (baseline[name] ?? 0))
    problems.push(
      `${name}: names clients ${count} times, baseline ${baseline[name] ?? 0}`,
    );
}
const was = Object.values(baseline).reduce((sum, count) => sum + count, 0);
console.log(
  `Client ids named outside src/crates/agents: ${total} (baseline ${was}), by area:`,
);
for (const [name, count] of Object.entries(byArea).sort((a, b) => b[1] - a[1]))
  console.log(`  ${String(count).padStart(4)}  ${name}`);
if (problems.length) {
  for (const problem of problems) console.error(problem);
  process.exit(1);
}
