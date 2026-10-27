#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { constants } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const platforms = { darwin: "macos", win32: "windows", linux: "linux" };

const help = `Usage: pnpm run build:platform -- [options]

  --platform host|macos|windows|linux  Build on the matching host OS (default: host)
  --component desktop|engine        Component to compile (default: desktop)
  --debug / --release                Profile (default: release; last flag wins)
  --bundle                          Let Tauri produce host-platform bundles
  --print-plan                      Print commands without starting build tools
  --help                            Show this help

This entry point does not cross-compile, install prerequisites, launch the app,
change client hooks, or publish a release. Desktop defaults to --no-bundle.`;

export function parseOptions(args, host = process.platform) {
  const options = {
    platform: "host",
    component: "desktop",
    debug: false,
    bundle: false,
    printPlan: false,
    help: false,
  };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--") continue;
    if (arg === "--platform" || arg === "--component") {
      const value = args[++i];
      if (!value || value.startsWith("--"))
        throw new Error(`${arg} requires a value.`);
      options[arg.slice(2)] = value;
    } else if (arg === "--debug") options.debug = true;
    else if (arg === "--release") options.debug = false;
    else if (arg === "--bundle") options.bundle = true;
    else if (arg === "--print-plan") options.printPlan = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else
      throw new Error(
        `Unknown option: ${arg}. Use --help for supported options.`,
      );
  }
  if (options.help) return options;
  if (!["host", "macos", "windows", "linux"].includes(options.platform))
    throw new Error(
      `Unsupported platform: ${options.platform}. Choose macos, windows, linux, or host.`,
    );
  if (!["desktop", "engine"].includes(options.component))
    throw new Error(
      `Unsupported component: ${options.component}. Choose desktop or engine.`,
    );
  if (options.bundle && options.component !== "desktop")
    throw new Error("--bundle applies only to the desktop component.");
  const native = platforms[host];
  if (!native)
    throw new Error(
      `Unsupported host OS: ${host}. Use a macOS, Windows, or Linux build host.`,
    );
  if (options.platform === "host") options.platform = native;
  if (options.platform !== native)
    throw new Error(
      `Cannot build ${options.platform} on ${native}. This entry point does not cross-compile; run it on a ${options.platform} host with that system's prerequisites.`,
    );
  return options;
}

export function buildPlan(options, env = process.env, host = process.platform) {
  let command;
  let args;
  if (options.component === "engine") {
    command = "cargo";
    args = ["build", "--locked", "-p", "agent-hud-engine"];
    if (!options.debug) args.push("--release");
  } else {
    command = "pnpm";
    args = ["--filter", "@agent-hud/desktop", "exec", "tauri", "build"];
    if (!options.bundle) args.push("--no-bundle");
    if (options.debug) args.push("--debug");
  }
  return {
    platform: options.platform,
    component: options.component,
    profile: options.debug ? "debug" : "release",
    bundle: options.bundle,
    commands: [
      {
        command,
        args,
        cwd: root,
        shell: host === "win32" && command === "pnpm",
      },
    ],
  };
}

function checkRustHost(platform, env, run) {
  const result = run("rustc", ["-vV"], { cwd: root, encoding: "utf8", env });
  if (result.error)
    throw new Error(
      `Cannot run rustc: ${result.error.message}. Install the host Rust toolchain first.`,
    );
  if (result.status !== 0)
    throw new Error(
      "rustc -vV failed. Repair the host Rust toolchain before building.",
    );
  const target = /^host: (\S+)$/m.exec(result.stdout)?.[1];
  const rustPlatform = target?.includes("apple-darwin")
    ? "macos"
    : target?.includes("windows")
      ? "windows"
      : target?.includes("linux")
        ? "linux"
        : undefined;
  if (rustPlatform !== platform)
    throw new Error(
      `Rust compiler host ${target ?? "(unknown)"} does not match ${platform}. Cross-compilation is not supported by this entry point.`,
    );
  const configured = env.CARGO_BUILD_TARGET?.trim();
  if (configured && configured !== target)
    throw new Error(
      `CARGO_BUILD_TARGET=${configured} differs from the compiler host ${target}. Remove the cross target or build on the matching host; this entry point does not cross-compile.`,
    );
  return target;
}

export function executePlan(plan, run = spawnSync) {
  for (const item of plan.commands) {
    const result = run(item.command, item.args, {
      cwd: item.cwd,
      stdio: "inherit",
      shell: item.shell === true,
    });
    if (result.error)
      throw new Error(
        `Could not start ${item.command}: ${result.error.message}`,
      );
    if (result.signal) return 128 + (constants.signals[result.signal] ?? 1);
    if (result.status !== 0) return result.status ?? 1;
  }
  return 0;
}

export function main(args = process.argv.slice(2)) {
  try {
    const options = parseOptions(args);
    if (options.help) {
      process.stdout.write(`${help}\n`);
      return 0;
    }
    const plan = buildPlan(options);
    if (options.printPlan) {
      process.stdout.write(`${JSON.stringify(plan, null, 2)}\n`);
      return 0;
    }
    const target = checkRustHost(options.platform, process.env, spawnSync);
    process.stdout.write(
      `Building ${options.component} for ${options.platform} (${target}), ${plan.profile}${options.bundle ? ", bundles enabled" : ", executable only"}.\n`,
    );
    return executePlan(plan);
  } catch (error) {
    process.stderr.write(
      `Build error: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    return 2;
  }
}

if (
  process.argv[1] &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url
)
  process.exitCode = main();
