/** Complete BotSpec.swift dimensions; coordinates are x0,y0,x1,y1 in body radii. */
export interface Fin {
  root: Point;
  angle: number;
  length: number;
  base: number;
  tip: number;
  sweep: number;
  hand: number;
}
export type Box = [number, number, number, number]; // x0, y0, x1, y1, in body radii
export type Point = [number, number];
export type Shape =
  | { kind: "circle" }
  | { kind: "outline"; keep: number; smooth: number }
  | { kind: "polygon"; sides: number; rotation: number; round: number }
  | {
      kind: "rect";
      width: number;
      height: number;
      round: number;
      skew?: number;
    }
  | { kind: "cat" }
  | { kind: "bob" };
export type Paint =
  | string
  | { colors: string[]; from: Point; to: Point }
  | { colors: string[]; center: Point; inner: number; outer: number };
export type Decor =
  | {
      kind: "blocks" | "soft";
      boxes: Box[];
      color: string;
      round?: number;
      follows?: boolean;
    }
  | { kind: "notch"; points: Point[] }
  | { kind: "cut"; boxes: Box[] }
  | { kind: "line"; points: Point[]; width: number }
  | { kind: "fins"; fins: Fin[]; color: string }
  | { kind: "feelers"; lines: Point[][]; color: string; width: number };
export interface BotSpec {
  radius: number;
  center: Point;
  reach: number;
  shape: Shape;
  paint?: Paint;
  face: Point;
  eye: {
    height: number;
    width: number;
    round: number;
    spread: number;
    across?: number;
  };
  eyeColor?: string;
  spin?: number;
  settle?: number;
  decor?: Decor[];
  ring?: {
    radiusX: number;
    radiusY: number;
    drop: number;
    tilt: number;
    width: number;
  };
  patch?: { box: Box; round: number; paint: Paint };
  stroke?: string;
  pupil?: string;
  signAt?: Point;
  unpainted?: Box[];
  crossesEyes?: boolean;
  keeps?: Box;
  ownEyes?: Box[];
  limbs?: { arms: Box[]; legs: Box[]; pixel: number };
}
const basic: BotSpec = {
  radius: 0.36,
  center: [0.5, 0.5],
  reach: 1,
  shape: { kind: "circle" },
  face: [0, 0],
  eye: { height: 0.62, width: 0.25, round: 1, spread: 0.34 },
};
const spec = (
  value: Partial<Omit<BotSpec, "eye">> & { eye?: Partial<BotSpec["eye"]> },
): BotSpec => ({ ...basic, ...value, eye: { ...basic.eye, ...value.eye } });
const mono = (light: boolean, dark = "#ffffff") => (light ? "#1d1d1f" : dark);

/** Numeric dimensions, outlines, eye placement and decoration from the original BotSpec.table. */
export function botSpecs(light: boolean): Record<string, BotSpec> {
  const cat = (head: string, face: Paint) =>
    spec({
      center: [0.5, 0.55],
      reach: 0.5,
      shape: { kind: "cat" },
      paint: head,
      face: [0, 0.2],
      eye: { height: 0.84, width: 0.36 },
      eyeColor: "#ffffff",
      patch: { box: [-0.9, -0.52, 0.9, 1.3], round: 0.5, paint: face },
      crossesEyes: true,
    });
  return {
    claude: spec({
      radius: 0.22,
      center: [0.5, 0.445],
      reach: 0.7,
      shape: { kind: "rect", width: 0.66, height: 0.44, round: 0.008 },
      face: [0, -0.25],
      eye: { height: 0.64, width: 0.32, round: 0, across: 0.875 },
      limbs: {
        arms: [
          [-2, 0, -1.5, 0.5],
          [1.5, 0, 2, 0.5],
        ],
        legs: [
          [-1.25, 1, -1, 1.5],
          [-0.75, 1, -0.5, 1.5],
          [0.5, 1, 0.75, 1.5],
          [1, 1, 1.25, 1.5],
        ],
        pixel: 0.5,
      },
    }),
    openai: spec({
      radius: 0.37,
      shape: { kind: "outline", keep: 1.1, smooth: 5 },
      spin: 7,
      crossesEyes: true,
    }),
    antigravity: spec({
      reach: 0.75,
      shape: { kind: "outline", keep: 0.5, smooth: 16 },
      face: [0, -0.2],
      settle: 0.093,
    }),
    deepseek: spec({
      face: [0, -0.14],
      crossesEyes: true,
      decor: [
        {
          kind: "fins",
          color: "#3f5ae6",
          fins: [
            {
              root: [-0.5, 0.38],
              angle: 142,
              length: 0.7,
              base: 0.27,
              tip: 0.06,
              sweep: 0,
              hand: -1,
            },
            {
              root: [0.5, 0.38],
              angle: 38,
              length: 0.7,
              base: 0.27,
              tip: 0.06,
              sweep: 0,
              hand: 1,
            },
            {
              root: [-0.12, -0.7],
              angle: -100,
              length: 0.64,
              base: 0.34,
              tip: 0.15,
              sweep: 0.22,
              hand: 0,
            },
          ],
        },
        {
          kind: "soft",
          boxes: [[-0.86, 0.34, 0.86, 1.9]],
          color: "#c9d4ff",
          round: 1,
        },
      ],
    }),
    grok: spec({
      radius: 0.33,
      reach: 0.55,
      face: [0, -0.3],
      ring: { radiusX: 1.5, radiusY: 0.32, drop: 0.3, tilt: -20, width: 0.14 },
    }),
    "grok-bot": spec({
      radius: 0.37,
      paint: light ? "#000000" : "#ffffff",
      crossesEyes: true,
      eye: { height: 0.78, width: 0.3 },
      eyeColor: light ? "#f5f5f7" : undefined,
    }),
    cursor: spec({
      radius: 0.41,
      reach: 0.7,
      shape: { kind: "polygon", sides: 6, rotation: -90, round: 0.07 },
      face: [-0.12, 0.16],
      decor: [
        {
          kind: "notch",
          points: [
            [0.32, -0.954],
            [0.75, -0.704],
            [0.266, -0.361],
          ],
        },
      ],
      eyeColor: light ? "#f5f5f7" : undefined,
    }),
    opencode: spec({
      reach: 0.5,
      shape: { kind: "rect", width: 0.66, height: 0.66, round: 0.04 },
      paint: light ? "#211e1e" : "#f1ecec",
      face: [0, -0.02],
      eye: { height: 0.4, width: 0.24, round: 0.12 },
      eyeColor: light ? "#f5f5f7" : undefined,
      decor: [
        {
          kind: "blocks",
          boxes: [
            [-1, -1.06, 1, -0.56],
            [-1, -0.56, -0.62, -0.36],
            [-0.24, -0.56, 0.16, -0.42],
            [0.58, -0.56, 1, -0.3],
          ],
          color: light ? "#cfcecd" : "#4b4646",
        },
        {
          kind: "blocks",
          boxes: [[-0.6, 0.36, 0.6, 0.6]],
          color: light ? "#cfcecd" : "#4b4646",
          follows: true,
        },
      ],
    }),
    kimi: spec({
      radius: 0.32,
      center: [0.41, 0.59],
      shape: { kind: "outline", keep: 0.3, smooth: 22 },
      keeps: [0.78, 0.02, 0.995, 0.235],
      signAt: [0.44, 0.22],
      eyeColor: light ? "#f5f5f7" : undefined,
    }),
    glm: spec({
      shape: { kind: "rect", width: 0.74, height: 0.74, round: 0.16 },
      paint: "#28282b",
      eyeColor: "#ffffff",
      stroke: "#ffffff",
    }),
    pi: spec({
      center: [0.5, 0.31],
      reach: 0.3,
      shape: { kind: "rect", width: 0.9, height: 0.29, round: 0.02 },
      eye: { height: 0.3, width: 0.3, round: 0 },
      eyeColor: light ? "#f5f5f7" : undefined,
      decor: [
        {
          kind: "blocks",
          boxes: [
            [-0.8, 0.36, -0.3, 1.5],
            [0.3, 0.36, 0.8, 1.5],
          ],
          color: mono(light),
        },
      ],
    }),
    hermes: spec({
      crossesEyes: true,
      radius: 0.35,
      reach: 0.5,
      paint: mono(light, "#f8f8f9"),
      face: [0, 0.24],
      eye: { height: 0.72, width: 0.3, spread: 0.44 },
      eyeColor: light ? "#f5f5f7" : undefined,
      decor: [
        {
          kind: "soft",
          boxes: [
            [-1.1, -1.25, -0.2, -0.22],
            [-0.44, -1.3, 0.44, -0.3],
            [0.2, -1.25, 1.1, -0.22],
          ],
          color: "#b7b6c2",
          round: 1,
        },
      ],
    }),
    zcode: spec({
      reach: 0.8,
      shape: {
        kind: "rect",
        width: 0.66,
        height: 0.64,
        skew: 0.2,
        round: 0.05,
      },
      eyeColor: light ? "#f5f5f7" : undefined,
    }),
    qwen: spec({
      radius: 0.37,
      shape: { kind: "outline", keep: 0.55, smooth: 7 },
      spin: 7,
      crossesEyes: true,
    }),
    devin: spec({
      reach: 0.55,
      shape: { kind: "outline", keep: 1, smooth: 5 },
      paint: mono(light),
      eye: { across: 0.42 },
      eyeColor: light ? "#f5f5f7" : undefined,
    }),
    openclaw: spec({
      crossesEyes: true,
      center: [0.5, 0.54],
      reach: 0.7,
      paint: {
        from: [0.2, 0.1],
        to: [0.85, 0.95],
        colors: ["#ee4747", "#c22e2e"],
      },
      face: [0, 0.06],
      eye: { height: 0.46, width: 0.46, spread: 0.46 },
      eyeColor: "#050810",
      decor: [
        {
          kind: "feelers",
          lines: [
            [
              [-0.24, -0.9],
              [-0.34, -1.14],
              [-0.54, -1.22],
            ],
            [
              [0.24, -0.9],
              [0.34, -1.14],
              [0.54, -1.22],
            ],
          ],
          color: "#ea4a4a",
          width: 0.1,
        },
      ],
    }),
    codebuddy: cat(mono(light), {
      from: [0.2, 0.2],
      to: [0.85, 1],
      colors: ["#5570da", "#4587c1", "#369dab"],
    }),
    workbuddy: cat(light ? "#1b2622" : "#eafaf6", {
      from: [0.5, 0.55],
      to: [0.5, 1],
      colors: ["#00cd8c", "#7dd76d"],
    }),
    qoder: spec({
      crossesEyes: true,
      reach: 0.55,
      paint: light ? "#161616" : "#f8f8f8",
      face: [0, -0.24],
      eye: { height: 0.7, width: 0.3, spread: 0.42 },
      eyeColor: light ? "#f5f5f7" : undefined,
      decor: [
        {
          kind: "soft",
          boxes: [[-1.2, -0.3, 1.2, 1.2]],
          color: "#28d858",
          round: 0,
        },
        {
          kind: "soft",
          boxes: [[-1.4, -1.7, 1.4, 0.45]],
          color: "#5be584",
          round: 1,
        },
        {
          kind: "soft",
          boxes: [[-1.4, -1.7, 1.4, 0.34]],
          color: light ? "#161616" : "#f8f8f8",
          round: 1,
        },
      ],
    }),
    copilot: spec({
      ownEyes: [
        [0.355, 0.57, 0.434, 0.723],
        [0.562, 0.57, 0.645, 0.723],
      ],
    }),
  };
}
