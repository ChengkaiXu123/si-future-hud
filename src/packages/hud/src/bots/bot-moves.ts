import type { BotEvent, BotLook } from "./bot-state";
export const easeInOut = (x: number) =>
  x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2;
export const easeOut = (x: number) => 1 - (1 - x) ** 3;
export const easeOutBack = (x: number) =>
  1 + 2.70158 * (x - 1) ** 3 + 1.70158 * (x - 1) ** 2;
/** Direct BotMoves.swift / BotFins.swift curves, all offsets in mark-side shares. */
export const BotMoves = {
  ponderGaze: [0.4, -0.7] as const,
  ponderTilt: 5,
  ponder: 2.5,
  ponderSide: (now: number) =>
    Math.floor(now / 2.5 + 1e-6) % 2 === 0 ? 1 : -1,
  nextPonder: (now: number) => (Math.floor(now / 2.5 + 1e-6) + 1) * 2.5,
  beat: 0.125,
  burst: 3,
  cycle: 4.5,
  busySpin: 14,
  inBurst: (time: number) => Math.max(0, time) % 4.5 < 3 - 1e-6,
  bob(time: number) {
    return this.inBurst(time) &&
      Math.floor(Math.max(0, time) / 0.125 + 1e-6) % 2 === 1
      ? 0.04
      : 0;
  },
  nextBob(time: number) {
    time = Math.max(0, time);
    return this.inBurst(time)
      ? (Math.floor(time / 0.125 + 1e-6) + 1) * 0.125
      : (Math.floor(time / 4.5 + 1e-6) + 1) * 4.5;
  },
  fullWait: 8,
  firstMove: 1.2,
  move(age: number): { index: number; start: number | null; next: number } {
    const first = 1.2,
      often = 2.6,
      rarely = 8;
    if (age < first) return { index: -1, start: null, next: first };
    const frequent = Math.ceil((120 - first) / often),
      late = first + frequent * often;
    if (age < late - 1e-9) {
      const index = Math.floor((age - first) / often + 1e-9);
      return {
        index,
        start: first + index * often,
        next: first + (index + 1) * often,
      };
    }
    const index = Math.floor((age - late) / rarely + 1e-9);
    return {
      index: frequent + index,
      start: late + index * rarely,
      next: late + (index + 1) * rarely,
    };
  },
  moveLength: (look: BotLook) =>
    look === "question" ? 0.4 : look === "plan" ? 0.6 : 0.55,
  knock(time: number) {
    return time >= 0 && time < 0.55
      ? 8 * Math.sin((2 * Math.PI * 2.5 * time) / 0.55) * (1 - time / 0.55)
      : 0;
  },
  questionTilt(age: number) {
    const move = this.move(age);
    if (move.start === null) return 0;
    const side = move.index % 2 === 0 ? 1 : -1,
      from = move.index === 0 ? 0 : -12 * side,
      x = Math.min(1, (age - move.start) / 0.4);
    return from + (12 * side - from) * easeInOut(x);
  },
  nod(time: number) {
    if (time < 0 || time >= 0.6) return 0;
    const half = time / 0.3;
    return 0.03 * Math.sin(Math.PI * (half - Math.floor(half)));
  },
  hopSeconds: 0.3,
  length: (event: BotEvent, limbed: boolean) =>
    event === "start"
      ? limbed
        ? 1
        : 0.3
      : event === "failed"
        ? 1.5
        : limbed
          ? 1
          : 1.2,
  hop(time: number) {
    if (time < 0 || time >= 0.3) return 0;
    const x = time / 0.3;
    return -0.06 * 4 * x * (1 - x);
  },
  shake: (time: number) =>
    time >= 0 && time < 0.5
      ? 0.05 * Math.sin(2 * Math.PI * 6 * time) * (1 - time / 0.5)
      : 0,
  sag: (time: number) =>
    time < 0.5
      ? 0
      : time < 0.75
        ? easeInOut((time - 0.5) / 0.25)
        : time < 1.2
          ? 1
          : time < 1.5
            ? 1 - easeInOut((time - 1.2) / 0.3)
            : 0,
  sigh(time: number) {
    const u = Math.max(0, time) / 1.2;
    if (u < 0.4) {
      const k = easeInOut(u / 0.4);
      return { scaleX: 1 - 0.04 * k, scaleY: 1 + 0.07 * k, open: 1 - 0.7 * k };
    }
    if (u < 0.7) {
      const k = easeInOut((u - 0.4) / 0.3);
      return {
        scaleX: 0.96 + 0.1 * k,
        scaleY: 1.07 - 0.15 * k,
        open: 0.3 + 0.2 * k,
      };
    }
    const k = easeInOut(Math.min(1, (u - 0.7) / 0.3));
    return {
      scaleX: 1.06 - 0.06 * k,
      scaleY: 0.92 + 0.08 * k,
      open: 0.5 + 0.5 * k,
    };
  },
  paddle: 14,
  finWaveSeconds: 0.9,
  cheerStart: 0.3,
  cheerSeconds: 1.2,
  finFlap: (time: number) =>
    time >= 0 && time < 0.3 ? 32 * Math.sin((Math.PI * time) / 0.3) : 0,
  finWave(time: number) {
    if (time < 0 || time >= 0.9) return 0;
    const up =
      easeInOut(Math.min(1, time / 0.18)) *
      easeInOut(Math.min(1, (0.9 - time) / 0.18));
    return up * (34 + 12 * Math.sin((2 * Math.PI * (time - 0.18)) / 0.27));
  },
  cheer(age: number) {
    const time = age - 0.3;
    return time >= 0 && time < 1.2
      ? 30 * Math.abs(Math.sin((Math.PI * time) / 0.4))
      : 0;
  },
};
