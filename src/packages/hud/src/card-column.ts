import type { CSSProperties } from "react";

/** Rows stacked in a HUD card, each exactly as wide as the card's content.
 *
 * A grid left to size its own column makes it as wide as the narrowest its widest row can get. A line cut with "…"
 * rather than wrapped (an option's description, a path, a summary) cannot get narrower than the whole line, so a long
 * one made the column wider than the card. The card has a fixed width and clips what sticks out, so what sits at the
 * right (the count, the times, Dismiss) went out of sight at its border, and the row squeezed its other words to
 * make room. A column that starts at zero is the card's width whatever it holds, as a VStack in Open's fixed-width
 * card is; such a line is then cut at the card's edge instead. */
export function cardColumn(gap: number): CSSProperties {
  return { display: "grid", gridTemplateColumns: "minmax(0, 1fr)", gap };
}
