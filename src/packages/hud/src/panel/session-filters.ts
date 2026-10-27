import type { PanelLayout } from "@agent-hud/protocol";

/** Whether the panel's empty Sessions section says that no session matches
 * rather than that there are none yet (Open's PanelSessionRows.noneMatch):
 * only the layout's own filters, its agents and projects, leave
 * sessions out, and only when the panel has sessions to leave out. An agent's
 * own panel lists that agent's sessions; that is what it shows, not a filter.
 * - `shown`: the rows the section lists.
 * - `own`: the sessions the panel lists without the layout's filters: all of
 *   them, or in an agent's own panel that agent's. */
export function noSessionsMatch(
  layout: PanelLayout["sessions"] = {},
  focused: boolean,
  shown: number,
  own: number,
): boolean {
  const filtered = Boolean(
    (!focused && layout.agents?.length) || layout.projects?.length,
  );
  return filtered && shown === 0 && own > 0;
}
