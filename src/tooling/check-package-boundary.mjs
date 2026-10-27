#!/usr/bin/env node
// Keeps each front-end package behind its name. A file may reach another
// package only as `@agent-hud/<name>/<subpath>` through that package's
// `exports`, and only a package it lists in its own package.json. Relative
// paths stay inside the file's package, and no tsconfig or Vite alias points
// into another package's sources.
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..");
const groups = [join(root, "src", "apps"), join(root, "src", "packages")];
const skipped = new Set(["node_modules", "dist", "target", "src-tauri"]);
const source = /\.(ts|tsx|mjs|css)$/;
const specifier =
  /(?:\bfrom\s*|\bimport\s*\(?\s*|@import\s+|new URL\(\s*)(["'])([^"'\n]+)\1/g;

const packages = groups.flatMap((group) =>
  readdirSync(group, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(group, entry.name))
    .filter((directory) => existsSync(join(directory, "package.json"))),
);
const manifests = new Map(
  packages.map((directory) => [
    directory,
    JSON.parse(readFileSync(join(directory, "package.json"), "utf8")),
  ]),
);
const byName = new Map(
  [...manifests].map(([directory, manifest]) => [manifest.name, directory]),
);

function* files(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (skipped.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else if (source.test(entry.name)) yield path;
  }
}

function exported(manifest, subpath) {
  const exports = manifest.exports ?? {};
  if (subpath in exports) return true;
  return Object.keys(exports).some((key) => {
    const star = key.indexOf("*");
    return (
      star >= 0 &&
      subpath.startsWith(key.slice(0, star)) &&
      subpath.endsWith(key.slice(star + 1))
    );
  });
}

const violations = [];
for (const directory of packages) {
  const manifest = manifests.get(directory);
  const declared = new Set([
    ...Object.keys(manifest.dependencies ?? {}),
    ...Object.keys(manifest.devDependencies ?? {}),
    ...Object.keys(manifest.peerDependencies ?? {}),
  ]);
  for (const path of files(directory)) {
    const text = readFileSync(path, "utf8");
    for (const match of text.matchAll(specifier)) {
      const spec = match[2];
      const line = text.slice(0, match.index).split("\n").length;
      const where = `${relative(root, path)}:${line}`;
      if (spec.startsWith(".")) {
        const target = resolve(dirname(path), spec);
        if (!target.startsWith(directory + sep))
          violations.push(`${where}: ${spec} leaves the package`);
      } else if (spec.startsWith("@/")) {
        violations.push(
          `${where}: ${spec} uses an alias; import by package name`,
        );
      } else if (spec.startsWith("@agent-hud/")) {
        const [scope, name, ...rest] = spec.split("/");
        const target = `${scope}/${name}`;
        const own = target === manifest.name;
        if (!byName.has(target)) {
          violations.push(`${where}: ${target} is not a workspace package`);
          continue;
        }
        if (!own && !declared.has(target))
          violations.push(
            `${where}: ${target} is not in ${relative(root, directory)}/package.json`,
          );
        const subpath = rest.length ? `./${rest.join("/")}` : ".";
        if (!exported(manifests.get(byName.get(target)), subpath))
          violations.push(`${where}: ${target} does not export ${subpath}`);
      }
    }
  }
  const tsconfig = join(directory, "tsconfig.json");
  if (existsSync(tsconfig)) {
    const config = JSON.parse(readFileSync(tsconfig, "utf8"));
    for (const entry of config.include ?? []) {
      if (!resolve(directory, entry).startsWith(directory + sep))
        violations.push(
          `${relative(root, tsconfig)}: include ${entry} leaves the package`,
        );
    }
    for (const targets of Object.values(config.compilerOptions?.paths ?? {})) {
      for (const target of targets) {
        if (!resolve(directory, target).startsWith(directory + sep))
          violations.push(
            `${relative(root, tsconfig)}: path ${target} leaves the package`,
          );
      }
    }
  }
}

if (violations.length) {
  for (const violation of violations) console.error(violation);
  console.error(
    `\n${violations.length} import(s) cross a package boundary. Import another package by its name and an exported subpath, and list it in package.json; see CONTRIBUTING.md.`,
  );
  process.exit(1);
}
console.log(
  `Package boundary: ${packages.length} packages import each other only by name.`,
);
