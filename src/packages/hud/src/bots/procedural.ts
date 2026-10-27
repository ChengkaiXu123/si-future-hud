import type { MarkState } from "./clawd";
import { BotMotion } from "./bot-motion";
import { botPhaseState, sameBotState } from "./bot-state";
import { drawBot } from "./bot-renderer";
import { paintSign, signCell } from "./bot-signs";
import type { BotForm } from "./bot-form";
export { makeForm, type BotForm, formOutline } from "./bot-form";
export { BotMotion, BotPace, SplitMix } from "./bot-motion";
export {
  BotRenderer,
  drawBot,
  renderBot,
  renderLogo,
  botRenderLook,
  applyBotPose,
} from "./bot-renderer";
export { botSpecs, type BotSpec } from "./bot-spec";
export { BotMoves } from "./bot-moves";
export { BotSigns } from "./bot-signs";
export { BotHairline } from "./bot-hairline";
export { BotStill } from "./bot-still";

/** Compatibility for the former four-phase canvas entry; the performer itself is the full BotMotion. */
export class ProceduralMotion {
  readonly motion = new BotMotion();
  private initialized = false;
  step(
    form: BotForm,
    state: MarkState,
    age: number,
    clock: number,
    _dt: number,
    morph: number,
    hovered: boolean,
    aim: readonly [number, number],
  ) {
    this.motion.fit(form.spec);
    const next = botPhaseState(state);
    next.hovered = hovered;
    if (!this.initialized || !sameBotState(this.motion.state, next)) {
      this.motion.set(next, clock);
      this.motion.since = clock - age;
      this.initialized = true;
    }
    this.motion.step(clock, aim);
    this.motion.morph = morph;
    return this.motion;
  }
}
export function drawProcedural(
  context: CanvasRenderingContext2D,
  form: BotForm,
  morph: number,
  state: MarkState,
  age: number,
  clock: number,
  _open: number,
  motion: ProceduralMotion,
  dt: number,
  hovered: boolean,
  aim: readonly [number, number],
) {
  drawBot(
    context,
    form,
    motion.step(form, state, age, clock, dt, morph, hovered, aim),
    { signs: false },
  );
}
export function signAbove(
  context: CanvasRenderingContext2D,
  side: number,
  extent: number,
  light: boolean,
) {
  const cell = signCell(side);
  paintSign(
    context,
    "exclamation",
    {
      x: Math.round(extent / 2 + 0.14 * side),
      y: Math.round(extent / 2 - 0.68 * side),
      width: cell * 5,
      height: cell * 7,
    },
    light,
  );
}
