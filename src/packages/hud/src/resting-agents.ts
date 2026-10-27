// Which agents a resting HUD shows, by mode, before a layout's own list
// narrows them. Kept free of the UI so the tests can run it.
import type { Snapshot } from "@agent-hud/protocol";
import { agentArtworkKey as vendorKey } from "./agent-artwork.ts";

type HUDMode = "notch" | "logos" | "simple";

/** Acquisition identity stays distinct from artwork (Codex shares its OpenAI mark). */
export function sourceID(snapshot: Snapshot, source: string): string {
  const normalize = (value: string) =>
    value.trim().toLowerCase().replace(/[ _]+/g, "-");
  const key = normalize(source);
  return (
    snapshot.providers.find(
      (provider) =>
        normalize(provider.id) === key || normalize(provider.name) === key,
    )?.id ?? key
  );
}

/** How long after its last event an agent still belongs on a Dynamic dock
 * (Open's ReportView.queueRecency). */
const queueRecency = 86_400_000;

/** The agents a resting HUD in `mode` shows without a list, one per mark, in
 * Open's order (ScreenAgents.available):
 * - notch, the glow's parts: each window whose reading shows a level, then
 *   each balance (ReportView.glowSegments);
 * - Dynamic dock: every window's agent in row order, then any agent with a
 *   session heard from in the last day whose live status is on, newest first
 *   (ReportView.queueVendors);
 * - Simple: every window's agent in row order, then the agents billed by
 *   balance alone (SimpleMeterItem.items).
 * Each mode also includes detected, enabled clients without a reading.
 * Also returns when a recent session ages out of the day. */
export function restingAgents(snapshot: Snapshot, mode: HUDMode, now: number) {
  const order: string[] = [];
  const add = (source: string) => {
    if (!order.some((s) => vendorKey(s) === vendorKey(source)))
      order.push(source);
  };
  let expiresAt = Infinity;
  if (mode === "notch") {
    snapshot.quotas
      .filter((q) => q.usedPercent != null && q.status !== "stale")
      .forEach((q) => add(q.vendor));
    snapshot.billing.forEach((b) => add(b.vendor));
  } else {
    snapshot.quotas.forEach((q) => add(q.vendor));
    if (mode === "simple") {
      snapshot.billing.forEach((b) => add(b.vendor));
    } else {
      const live = (source: string) =>
        snapshot.sourcePreferences?.[sourceID(snapshot, source)]?.live !==
        false;
      snapshot.sessions
        .map((s) => ({ source: s.source, at: Date.parse(s.updatedAt) }))
        .filter(({ source, at }) => {
          const expires = at + queueRecency;
          if (!Number.isFinite(expires) || now > expires || !live(source))
            return false;
          expiresAt = Math.min(expiresAt, expires);
          return true;
        })
        .sort((a, b) => b.at - a.at)
        .forEach(({ source }) => add(source));
    }
  }
  snapshot.providers
    .filter((provider) => provider.detected && provider.enabled)
    .forEach((provider) => add(provider.id));
  return { order, expiresAt };
}
