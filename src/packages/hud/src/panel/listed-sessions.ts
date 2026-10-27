import { createContext, useContext } from "react";

/** Where the Statistics window opens: a page, or one session's usage. */
export type StatsTarget = { tab?: "tokens" | "sessions"; sessionId?: string };

/** What the panel's session rows do through the HUD (Open's ScreenHUD and
 * HoverPanelView+Sessions): the HUD collapses before it opens the
 * Statistics window on the session list or a session's page. */
export interface ListedSessions {
  openStats(target: StatsTarget): void;
}

export const ListedSessionsContext = createContext<ListedSessions | null>(null);
export const useListedSessions = () => useContext(ListedSessionsContext);
