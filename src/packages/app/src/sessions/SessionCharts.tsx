import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type SVGProps,
} from "react";
import { Layers, RotateCw } from "lucide-react";
import {
  contextRuns,
  largestPositions,
  niceScale,
  stackTurn,
  turnTicks,
  type StackedBar,
} from "../lib/sessions";
import { moneyShort } from "@agent-hud/ui/format";
import {
  clipped,
  compact,
  kindColor,
  numeric,
  plain,
  type Entry,
  type Theme,
} from "./SessionTheme";

/** What the session page's turn chart measures: the new tokens of each turn, or what its calls would cost at list prices. */
export type Measure = "tokens" | "cost";

/** How strongly the sub-agents' part of a bar is drawn (Open's `subagentOpacity`). */
const subagentOpacity = 0.4;
/** Bars grow and shrink, never jump; the window's reduce-motion rule stills them. */
const grow =
  "y 240ms cubic-bezier(0.22, 1, 0.36, 1), height 240ms cubic-bezier(0.22, 1, 0.36, 1), opacity 160ms ease";
/** The axis the turn chart and the context chart share, so a compaction lines up with its turn. */
const axis = 44;
const labelFont = { fontSize: 10, fontFamily: "inherit" } as const;

/** The width an element has, kept as the window resizes. */
export function useWidth<T extends Element>() {
  const [width, setWidth] = useState(0);
  const observer = useRef<ResizeObserver | null>(null);
  const ref = useCallback((node: T | null) => {
    observer.current?.disconnect();
    if (!node) return;
    setWidth(node.getBoundingClientRect().width);
    observer.current = new ResizeObserver(([entry]) =>
      setWidth(entry.contentRect.width),
    );
    observer.current.observe(node);
  }, []);
  useEffect(() => () => observer.current?.disconnect(), []);
  return [ref, width] as const;
}

/** A label that stays readable over lines: a halo in the card's colour behind it. */
function Halo({
  theme,
  children,
  ...props
}: SVGProps<SVGTextElement> & { theme: Theme; children: ReactNode }) {
  return (
    <text
      {...labelFont}
      paintOrder="stroke"
      stroke={theme.surface}
      strokeWidth={3}
      strokeLinejoin="round"
      {...props}
    >
      {children}
    </text>
  );
}

/**
 * One stacked bar from `base` up: each part keeps its proportion, a bar that
 * added almost nothing still reaches `least` px, and the sub-agents' parts
 * are faded. Returns the rects and where the bar's top is.
 */
function stackedRects(
  bar: StackedBar,
  x: number,
  width: number,
  base: number,
  scale: number,
  least: number,
  theme: Theme,
  key: string,
): { rects: ReactNode[]; top: number } {
  if (bar.total <= 0) return { rects: [], top: base };
  const barScale = Math.max(scale, least / bar.total);
  let y = base;
  const rects = bar.parts.map((part, index) => {
    const height = part.value * barScale;
    y -= height;
    return (
      <rect
        key={`${key}-${index}`}
        x={x}
        width={width}
        fill={kindColor(theme, part.kind)}
        fillOpacity={part.subagent ? subagentOpacity : 1}
        style={{
          y: y + 0.4,
          height: Math.max(0.6, height - 0.8),
          transition: grow,
        }}
      />
    );
  });
  return { rects, top: y };
}

/**
 * A row's recent turns (the phone's `TurnSparkline`): each turn's new tokens
 * stacked by kind, the sub-agents' part faded, a compaction as an amber line
 * before its turn, and the largest turn labelled with its value so the bars
 * have a scale. No turn is marked as running: lists show no session state
 * (owner, 2026-10-26).
 */
export function TurnSparkline({
  bars,
  theme,
  height = 34,
  label,
}: {
  bars: StackedBar[];
  theme: Theme;
  height?: number;
  label: string;
}) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const headroom = 12;
  const slot = width / Math.max(bars.length, 12);
  const barWidth = Math.max(1.5, slot - 2);
  const peak = Math.max(1, ...bars.map((bar) => bar.total));
  const scale = (height - headroom - 1) / peak;
  const [largest] = largestPositions(
    bars.map((bar) => bar.total),
    1,
  );
  const tops: number[] = [];
  return (
    <div
      ref={ref}
      style={{ height, position: "relative", minWidth: 0 }}
      role="img"
      aria-label={label}
    >
      {width > 0 && (
        <svg
          width={width}
          height={height}
          style={{ display: "block", overflow: "visible" }}
        >
          {bars.map((bar, index) => {
            const x = index * slot + (slot - barWidth) / 2;
            const { rects, top } = stackedRects(
              bar,
              x,
              barWidth,
              height - 1,
              scale,
              1.5,
              theme,
              `b${index}`,
            );
            tops[index] = top;
            return (
              <g key={index}>
                {bar.compacted && (
                  <line
                    x1={index * slot}
                    x2={index * slot}
                    y1={headroom - 2}
                    y2={height - 1}
                    stroke={theme.compaction}
                    strokeWidth={1}
                    strokeDasharray="2 2"
                  />
                )}
                {rects}
              </g>
            );
          })}
          <rect
            x={0}
            y={height - 0.5}
            width={width}
            height={0.5}
            fill={theme.divider}
          />
          {largest !== undefined && (
            <Halo
              theme={theme}
              x={Math.min(Math.max(largest * slot + slot / 2, 12), width - 12)}
              y={Math.max(9, (tops[largest] ?? height) - 3)}
              textAnchor="middle"
              fill={theme.secondary}
              style={numeric}
            >
              {compact(bars[largest].total)}
            </Halo>
          )}
        </svg>
      )}
    </div>
  );
}

/** A value on the turn chart: tokens as "48k", money as "$1.20". */
function measureText(value: number, measure: Measure, zh: boolean) {
  return measure === "cost"
    ? moneyShort(value, "USD", zh ? "zh" : "en")
    : compact(value);
}

/** A turn's value in the chart's measure: what it added, or its list price. */
export function turnValue(entry: Entry, measure: Measure): number {
  if (measure === "cost") return entry.listCost ?? 0;
  return entry.newTokens ?? entry.input + entry.output;
}

/** A turn as a stacked bar: Core's own part and its sub-agents' (the page's parser fallback has neither). */
export function turnBar(entry: Entry): StackedBar {
  return stackTurn(
    entry.own ?? { input: entry.input, output: entry.output },
    entry.subagents,
    Boolean(entry.compacted),
  );
}

/**
 * The session page's turns, one bar each (the phone's `SessionBarsChart`):
 * new tokens stacked by kind with the sub-agents' part faded, or the list
 * price; on a round scale. The three largest turns carry their value on the
 * chart, a layers mark sits above a turn sub-agents helped and an amber
 * arrow above one whose cache had expired, a compaction is an amber dashed
 * line before its turn. No turn is marked as running: the page shows no
 * session state (owner, 2026-10-26).
 * Pointing shows `hover`; a click picks the turn.
 */
export function TurnBarsChart({
  entries,
  measure,
  selected,
  onSelect,
  hover,
  theme,
  height: total = 196,
}: {
  entries: Entry[];
  measure: Measure;
  selected: string | null;
  onSelect: (entry: Entry) => void;
  hover: (entry: Entry) => ReactNode;
  theme: Theme;
  height?: number;
}) {
  const zh = Boolean(theme.zh);
  const [ref, width] = useWidth<HTMLDivElement>();
  const [pointed, setPointed] = useState<number | null>(null);
  const labels = 18;
  const headroom = 30;
  const plot = Math.max(0, width - axis);
  const height = total - labels;
  const values = entries.map((entry) => turnValue(entry, measure));
  const scale = niceScale(Math.max(0, ...values));
  const yScale = (height - headroom) / scale.top;
  const slot = entries.length > 0 ? plot / entries.length : plot;
  const barWidth = Math.max(1, slot * 0.64);
  const picked = entries.findIndex((entry) => entry.id === selected);
  const largest = largestPositions(values, 3);
  const placed: number[] = [];
  const tops: number[] = [];
  const at = (clientX: number, element: Element) => {
    const x = clientX - element.getBoundingClientRect().left;
    if (x < 0 || x > plot || entries.length === 0) return null;
    return Math.min(entries.length - 1, Math.max(0, Math.floor(x / slot)));
  };
  const bars = entries.map((entry, index) => {
    const x = index * slot + (slot - barWidth) / 2;
    const bar =
      measure === "cost"
        ? {
            parts: [],
            total: values[index],
            compacted: Boolean(entry.compacted),
          }
        : turnBar(entry);
    let rects: ReactNode[];
    let top: number;
    if (measure === "cost") {
      const h = values[index] > 0 ? Math.max(2, values[index] * yScale) : 0;
      top = height - h;
      rects =
        h > 0
          ? [
              <rect
                key={`c${index}`}
                x={x}
                width={barWidth}
                fill={theme.accent}
                fillOpacity={0.8}
                style={{
                  y: top + 0.4,
                  height: Math.max(0.6, h - 0.8),
                  transition: grow,
                }}
              />,
            ]
          : [];
    } else {
      ({ rects, top } = stackedRects(
        bar,
        x,
        barWidth,
        height,
        yScale,
        2,
        theme,
        `t${index}`,
      ));
    }
    tops[index] = top;
    return (
      <g
        key={entry.id}
        opacity={picked < 0 || picked === index ? 1 : 0.45}
        style={{ transition: "opacity 160ms ease" }}
      >
        {rects}
      </g>
    );
  });
  const marks = entries.map((entry, index) => {
    const helped = (entry.subagentNew ?? 0) > 0 || Boolean(entry.subagentParts);
    const resent = entry.recached != null && entry.recached > 0;
    if (!helped && !resent) return null;
    const center = index * slot + slot / 2;
    const y = tops[index] - 11;
    const both = helped && resent;
    return (
      <g key={`m${entry.id}`} aria-hidden="true">
        {helped && (
          <Layers
            x={center - 4.5 - (both ? 4 : 0)}
            y={y}
            size={9}
            color={theme.secondary}
          />
        )}
        {resent && (
          <RotateCw
            x={center - 4.5 + (both ? 4 : 0)}
            y={y}
            size={9}
            color={theme.warning}
            strokeWidth={2.6}
          />
        )}
      </g>
    );
  });
  const valueLabels = largest.map((index) => {
    const center = Math.min(Math.max(index * slot + slot / 2, 16), plot - 16);
    if (placed.some((x) => Math.abs(x - center) < 36)) return null;
    placed.push(center);
    const hasMark = marks[index] !== null;
    return (
      <Halo
        key={`v${index}`}
        theme={theme}
        x={center}
        y={Math.max(10, tops[index] - (hasMark ? 14 : 4))}
        textAnchor="middle"
        fill={theme.text}
        fontWeight={600}
        style={numeric}
      >
        {measureText(values[index], measure, zh)}
      </Halo>
    );
  });
  const ticks = turnTicks(entries.length);
  const hovered = pointed !== null ? entries[pointed] : undefined;
  return (
    <div ref={ref} style={{ position: "relative", height: total, minWidth: 0 }}>
      {width > 0 && (
        <svg
          width={width}
          height={total}
          role="img"
          aria-label={
            measure === "cost"
              ? zh
                ? "每轮费用"
                : "Cost per turn"
              : zh
                ? "每轮新增 Token"
                : "New tokens per turn"
          }
          style={{
            display: "block",
            cursor: entries.length ? "pointer" : "default",
            overflow: "visible",
          }}
          onMouseMove={(event) =>
            setPointed(at(event.clientX, event.currentTarget))
          }
          onMouseLeave={() => setPointed(null)}
          onClick={(event) => {
            const index = at(event.clientX, event.currentTarget);
            if (index !== null) onSelect(entries[index]);
          }}
        >
          {picked >= 0 && (
            <rect
              x={picked * slot}
              y={0}
              width={slot}
              height={height}
              fill={theme.pick}
              rx={2}
            />
          )}
          {pointed !== null && pointed !== picked && (
            <rect
              x={pointed * slot}
              y={0}
              width={slot}
              height={height}
              fill={theme.pick}
              opacity={0.6}
              rx={2}
            />
          )}
          {scale.ticks.map((tick) => {
            const y = height - tick * yScale;
            return (
              <g key={`g${tick}`}>
                <rect
                  x={0}
                  y={y - 0.25}
                  width={plot}
                  height={0.5}
                  fill={tick === 0 ? theme.divider : theme.track}
                  opacity={tick === 0 ? 1 : 0.6}
                />
                <text
                  {...labelFont}
                  x={plot + 6}
                  y={y + 3.5}
                  fill={theme.secondary}
                  style={numeric}
                >
                  {tick === 0 ? "0" : measureText(tick, measure, zh)}
                </text>
              </g>
            );
          })}
          {entries.map((entry, index) =>
            entry.compacted ? (
              <line
                key={`k${entry.id}`}
                x1={index * slot}
                x2={index * slot}
                y1={4}
                y2={height}
                stroke={theme.compaction}
                strokeWidth={1.25}
                strokeDasharray="3 3"
              />
            ) : null,
          )}
          {bars}
          {marks}
          {valueLabels}
          {ticks.map((index) => {
            const center = index * slot + slot / 2;
            return (
              <text
                key={`x${index}`}
                {...labelFont}
                x={Math.min(Math.max(center, 10), plot - 10)}
                y={total - 4}
                textAnchor="middle"
                fill={theme.secondary}
                style={numeric}
              >
                {`T${entries[index].index}`}
              </text>
            );
          })}
        </svg>
      )}
      {hovered && pointed !== null && (
        <div
          role="tooltip"
          style={{
            position: "absolute",
            top: 0,
            ...(pointed * slot + slot / 2 > plot / 2
              ? { right: width - pointed * slot + 8 }
              : { left: (pointed + 1) * slot + 8 }),
            pointerEvents: "none",
            zIndex: 2,
            animation: "session-fade-in 140ms ease",
          }}
        >
          {hover(hovered)}
        </div>
      )}
    </div>
  );
}

/**
 * Each turn's context against the model's window, on the turn chart's axis
 * (the phone's `ContextWindowChart`): a line and its area, broken where the
 * client compacted, each compaction a labelled amber line; a whisker up to a
 * turn's peak when it ended lower; the window as a labelled dashed line over
 * a near-limit band; the last point named with its tokens and fill.
 */
export function ContextChart({
  entries,
  window,
  fill,
  selected,
  theme,
  height: total = 104,
}: {
  entries: Entry[];
  window: number | null | undefined;
  /** Core's fill of the latest call, for the last point's label. */
  fill: number | null | undefined;
  selected: string | null;
  theme: Theme;
  height?: number;
}) {
  const zh = Boolean(theme.zh);
  const [ref, width] = useWidth<HTMLDivElement>();
  const plot = Math.max(0, width - axis);
  const pad = 6;
  const height = total - 2;
  const peak = Math.max(
    0,
    ...entries.map((entry) => entry.peakContext ?? entry.context ?? 0),
  );
  const limit = window != null && window > 0 ? window : null;
  const top = limit ?? niceScale(peak, 2).top;
  const y = (tokens: number) =>
    height - (Math.min(tokens, top) / top) * (height - pad);
  const slot = entries.length > 0 ? plot / entries.length : plot;
  const center = (index: number) => index * slot + slot / 2;
  const runs = contextRuns(entries);
  const picked = entries.findIndex((entry) => entry.id === selected);
  const color = theme.cache;
  const lastIndex = runs.at(-1)?.at(-1);
  const last = lastIndex !== undefined ? entries[lastIndex] : undefined;
  return (
    <div ref={ref} style={{ height: total, minWidth: 0 }}>
      {width > 0 && (
        <svg
          width={width}
          height={total}
          role="img"
          aria-label={zh ? "上下文窗口" : "Context window"}
          style={{ display: "block", overflow: "visible" }}
        >
          {picked >= 0 && (
            <rect
              x={picked * slot}
              y={0}
              width={slot}
              height={height}
              fill={theme.pick}
              rx={2}
            />
          )}
          {limit !== null && (
            <>
              <rect
                x={0}
                y={y(top)}
                width={plot}
                height={y(top * 0.8) - y(top)}
                fill={theme.warning}
                opacity={0.09}
              />
              <line
                x1={0}
                x2={plot}
                y1={y(top)}
                y2={y(top)}
                stroke={theme.warning}
                strokeWidth={1}
                strokeDasharray="3 3"
              />
              <Halo
                theme={theme}
                x={4}
                y={y(top) + 12}
                fill={theme.warning}
                fontWeight={600}
              >
                {(zh ? "上限 " : "Limit ") + compact(limit)}
              </Halo>
            </>
          )}
          <rect
            x={0}
            y={y(top / 2) - 0.25}
            width={plot}
            height={0.5}
            fill={theme.track}
            opacity={0.6}
          />
          <rect
            x={0}
            y={height - 0.25}
            width={plot}
            height={0.5}
            fill={theme.divider}
          />
          {[top, top / 2, 0].map((tick) => (
            <text
              key={`y${tick}`}
              {...labelFont}
              x={plot + 6}
              y={tick === top ? y(tick) + 7 : y(tick) + 3.5}
              fill={
                tick === top && limit !== null ? theme.warning : theme.secondary
              }
              style={numeric}
            >
              {tick === 0 ? "0" : compact(tick)}
            </text>
          ))}
          {runs.map((run, index) => {
            const points = run.map(
              (position) =>
                [center(position), y(entries[position].context ?? 0)] as const,
            );
            const line = points
              .map(([px, py], at) => `${at ? "L" : "M"}${px},${py}`)
              .join("");
            const area = `M${points[0][0]},${height}${points.map(([px, py]) => `L${px},${py}`).join("")}L${points.at(-1)![0]},${height}Z`;
            return (
              <g key={`r${index}`}>
                <path d={area} fill={color} fillOpacity={0.2} />
                <path
                  d={line}
                  fill="none"
                  stroke={color}
                  strokeWidth={1.5}
                  strokeLinejoin="round"
                />
              </g>
            );
          })}
          {entries.map((entry, index) =>
            entry.peakContext != null && entry.context != null ? (
              <path
                key={`p${entry.id}`}
                d={`M${center(index)},${y(entry.context)}L${center(index)},${y(entry.peakContext)}M${center(index) - 2.5},${y(entry.peakContext)}L${center(index) + 2.5},${y(entry.peakContext)}`}
                stroke={color}
                strokeWidth={1}
                fill="none"
              />
            ) : null,
          )}
          {entries.map((entry, index) => {
            if (!entry.compacted) return null;
            const x = index * slot;
            const right = x > plot - 64;
            return (
              <g key={`c${entry.id}`}>
                <line
                  x1={x}
                  x2={x}
                  y1={pad}
                  y2={height}
                  stroke={theme.compaction}
                  strokeWidth={1.25}
                  strokeDasharray="3 3"
                />
                <Halo
                  theme={theme}
                  x={right ? x - 4 : x + 4}
                  y={height - 6}
                  textAnchor={right ? "end" : "start"}
                  fill={theme.compaction}
                  fontWeight={600}
                >
                  {zh ? "已压缩" : "Compacted"}
                </Halo>
              </g>
            );
          })}
          {last && lastIndex !== undefined && last.context != null && (
            <g>
              <circle
                cx={center(lastIndex)}
                cy={y(last.context)}
                r={3}
                fill={color}
                stroke={theme.surface}
                strokeWidth={1.5}
              />
              <Halo
                theme={theme}
                x={center(lastIndex) - 7}
                y={Math.max(12, y(last.context) - 6)}
                textAnchor="end"
                fill={theme.text}
                fontWeight={600}
                style={numeric}
              >
                {compact(last.context) +
                  (fill != null ? ` · ${Math.round(fill * 100)}%` : "")}
              </Halo>
            </g>
          )}
        </svg>
      )}
    </div>
  );
}

/** One row of a ranked list: a name, its value and share, and a bar for its share of the whole. */
export interface ShareRow {
  id: string;
  label: ReactNode;
  value: number;
  /** The text after the value: its share, or anything else. */
  note?: string;
  color?: string;
  title?: string;
}

/**
 * Labelled bars, one per row, each as long as its share of `whole`: the name
 * and the figures sit on the bar's line, so nothing needs a hover.
 */
export function ShareRows({
  rows,
  whole,
  theme,
  onSelect,
}: {
  rows: ShareRow[];
  whole: number;
  theme: Theme;
  onSelect?: (id: string) => void;
}) {
  return (
    <div style={{ display: "grid", gap: 7, minWidth: 0 }}>
      {rows.map((row) => {
        const content = (
          <>
            <span
              style={{
                display: "flex",
                alignItems: "baseline",
                gap: 8,
                minWidth: 0,
                fontSize: 12,
              }}
            >
              <span style={{ ...clipped, flex: 1 }}>{row.label}</span>
              <strong style={{ fontWeight: 600, ...numeric }}>
                {compact(row.value)}
              </strong>
              {row.note && (
                <span
                  style={{
                    color: theme.secondary,
                    fontSize: 11,
                    minWidth: 30,
                    textAlign: "right",
                    ...numeric,
                  }}
                >
                  {row.note}
                </span>
              )}
            </span>
            <span
              aria-hidden="true"
              style={{
                display: "block",
                height: 4,
                borderRadius: 2,
                background: theme.track,
                overflow: "hidden",
                marginTop: 3,
              }}
            >
              <span
                style={{
                  display: "block",
                  height: "100%",
                  width: `${whole > 0 ? Math.min(100, (row.value / whole) * 100) : 0}%`,
                  minWidth: row.value > 0 ? 2 : 0,
                  background: row.color ?? theme.secondary,
                  borderRadius: 2,
                  transition: "width 240ms cubic-bezier(0.22, 1, 0.36, 1)",
                }}
              />
            </span>
          </>
        );
        return onSelect ? (
          <button
            key={row.id}
            type="button"
            title={row.title}
            className="session-share-row"
            onClick={() => onSelect(row.id)}
            style={{ ...plain, display: "block", width: "100%", minWidth: 0 }}
          >
            {content}
          </button>
        ) : (
          <div key={row.id} title={row.title} style={{ minWidth: 0 }}>
            {content}
          </div>
        );
      })}
    </div>
  );
}
