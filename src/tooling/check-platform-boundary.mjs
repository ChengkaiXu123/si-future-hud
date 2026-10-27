#!/usr/bin/env node
// Keeps operating-system code behind its adapters. Shared Rust code may name
// an operating system only on a module selector: a #[cfg(...)] attribute that
// chooses which `mod` or `use` to compile. Everything else that differs by OS
// lives in a file or folder named for it: macos, windows, linux or unix.
import { readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..");
const scanned = [join(root, "src", "crates"), join(root, "src", "apps")];
const skipped = new Set(["node_modules", "target", "dist", "gen", "vendor"]);
const hosts = ["macos", "windows", "linux", "unix"];
const platform = /\b(target_os|unix|windows|target_family)\b/;

function* rustFiles(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (skipped.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* rustFiles(path);
    else if (entry.name.endsWith(".rs")) yield path;
  }
}

function hostSpecific(path) {
  return relative(root, path)
    .split(sep)
    .some((part) => {
      const stem = part.replace(/\.rs$/, "");
      return hosts.some((host) => stem === host || stem.endsWith(`_${host}`));
    });
}

const selector = /^((pub(\([^)]*\))?\s+)?(mod|use)\s|compile_error!)/;
const violations = [];
for (const directory of scanned) {
  for (const path of rustFiles(directory)) {
    if (hostSpecific(path)) continue;
    const lines = readFileSync(path, "utf8").split("\n");
    lines.forEach((line, index) => {
      const code = line.replace(/\/\/.*$/, "");
      if (/cfg!\s*\(/.test(code) && platform.test(code)) {
        violations.push([path, index, "cfg!() on the operating system"]);
        return;
      }
      if (!/#\[cfg(_attr)?\s*\(/.test(code) || !platform.test(code)) return;
      let next = index + 1;
      while (next < lines.length && /^\s*(#\[|\/\/|$)/.test(lines[next]))
        next += 1;
      if (!selector.test((lines[next] ?? "").trim())) {
        violations.push([
          path,
          index,
          "platform #[cfg] that does not select a mod or use",
        ]);
      }
    });
  }
}

if (violations.length) {
  for (const [path, index, reason] of violations) {
    console.error(`${relative(root, path)}:${index + 1}: ${reason}`);
  }
  console.error(
    `\n${violations.length} platform condition(s) in shared code. Move the OS-specific part into the ` +
      "capability's macos/windows/linux file and call it from here; see docs/platform-architecture.md.",
  );
  process.exit(1);
}
console.log(
  "Platform boundary: shared code selects adapters only through mod/use selectors.",
);
