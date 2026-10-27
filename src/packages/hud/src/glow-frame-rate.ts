/** GlowAnimator.swift nominal cadence: period follows motion without changing its clock. */
export function glowFrameRate(periodSeconds: number): number {
  return Math.floor(
    Math.min(24, Math.max(8, 72 / Math.max(0.5, periodSeconds))),
  );
}
