// How the rim moves, as agent-hud-native-open's GlowWindowController and
// GlowAnimator decide it. Pure, so it can be tested apart from the renderer.
import { glowFrameRate } from "./glow-frame-rate.ts";

export interface GlowMotion {
  /** The whole layer breathes in opacity, which the compositor runs; the
   * shader draws one resting frame. */
  pulse: boolean;
  /** Frames the shader draws a second for a moving pattern; 0 when it only
   * redraws on a change of size or colour. */
  framesPerSecond: number;
}

/** Open's soft glow breathes as one Core Animation opacity pulse
 * (`applyBreathing`). Every other effect, and breathing in a grid style,
 * changes the pattern itself (GlowMotion's per-cell gain), so the shader draws
 * it frame by frame at GlowAnimator's rate. Nothing moves without readings,
 * under Reduce Motion, or while the page is hidden; without a GPU only the
 * pulse remains, over the CPU still. */
export function glowMotion(
  style: string,
  effect: string,
  periodSeconds: number,
  moves: boolean,
  gpu: boolean,
): GlowMotion {
  if (!moves) return { pulse: false, framesPerSecond: 0 };
  if (style === "blur" && effect === "breathe")
    return { pulse: true, framesPerSecond: 0 };
  return {
    pulse: false,
    framesPerSecond: gpu ? glowFrameRate(periodSeconds) : 0,
  };
}

/** Peak to trough: the layer keeps the brightness the shader drew and dims to
 * 1 − amplitude of it (Open: trough = peak · (1 − amplitude)). */
export function breathFrames(amplitude: number): Keyframe[] {
  const depth = Math.min(1, Math.max(0, amplitude));
  return [{ opacity: 1 }, { opacity: 1 - depth }];
}

/** Half a period each way, from the peak, eased, for as long as it runs:
 * Open's CABasicAnimation with autoreverses and easeInEaseOut, which is CSS's
 * ease-in-out. Periods below Open's 0.5 s floor are held at it. */
export function breathTiming(periodSeconds: number): KeyframeAnimationOptions {
  return {
    duration: halfBreath(periodSeconds),
    direction: "alternate",
    iterations: Infinity,
    easing: "ease-in-out",
  };
}

/** Milliseconds from peak to trough. */
export function halfBreath(periodSeconds: number): number {
  return Math.max(0.5, periodSeconds) * 500;
}

/** Where a running breath continues when its period changes (a session
 * starts or the last one ends): the same point of the same peak, trough, peak
 * cycle, so the glow keeps its brightness and only its pace changes. */
export function retimedBreath(
  currentTime: number,
  fromHalf: number,
  toHalf: number,
): number {
  if (!(fromHalf > 0) || !Number.isFinite(currentTime)) return 0;
  const cycle = 2 * fromHalf;
  const into = ((currentTime % cycle) + cycle) % cycle;
  return (into / fromHalf) * toHalf;
}
