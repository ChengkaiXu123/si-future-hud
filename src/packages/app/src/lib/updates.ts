/** What SI Future HUD's updater (src/crates/updater) reports to Settings →
 * Updates, through the product extension's `updates.state` request and its
 * `product:event` events. Its words come in both languages. */
export interface UpdateText {
  zh: string;
  en: string;
}
export type UpdatePhase =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "failed";
export interface UpdateView {
  currentVersion: string;
  autoCheck: boolean;
  autoInstall: boolean;
  lastCheckAt: string | null;
  phase: UpdatePhase;
  status: UpdateText;
  progress: { done: number; total: number } | null;
  error: UpdateText | null;
  available: {
    version: string;
    pubDate?: string | null;
    notes: UpdateText;
    size: number;
  } | null;
  canCheck: boolean;
  canInstall: boolean;
  /** Why this copy only tells: a development build, a copy outside
   * Applications, the demo. */
  blocked: UpdateText | null;
  /** Downloaded, and waiting for no card on the notch. */
  waitingForIdle: boolean;
  /** The test feed (`SI_HUD_UPDATE_FEED`) while it is in use. */
  testFeed: string | null;
  lastInstall: { ok: boolean; version: string; message: UpdateText } | null;
}

/** The text in the language Settings reads in, else the other one. */
export function pick(text: UpdateText | null | undefined, zh: boolean): string {
  if (!text) return "";
  return (zh ? text.zh || text.en : text.en || text.zh) ?? "";
}

/** Whether a product event carries the updater's state. */
export function isUpdateEvent(
  payload: unknown,
): payload is { updates: UpdateView } {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "updates" in payload &&
    typeof (payload as { updates: unknown }).updates === "object" &&
    (payload as { updates: unknown }).updates !== null
  );
}

/** When the last check ran: how long ago within a day, else the date. */
export function lastChecked(
  at: string | null,
  zh: boolean,
  now: Date = new Date(),
): string {
  if (!at) return zh ? "还没有检查过" : "Never checked";
  const time = new Date(at);
  if (Number.isNaN(time.getTime())) return "";
  const minutes = Math.max(
    0,
    Math.floor((now.getTime() - time.getTime()) / 60_000),
  );
  if (minutes < 1) return zh ? "上次检查：刚刚" : "Last checked just now";
  if (minutes < 60)
    return zh
      ? `上次检查：${minutes} 分钟前`
      : `Last checked ${minutes} minute${minutes === 1 ? "" : "s"} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24)
    return zh
      ? `上次检查：${hours} 小时前`
      : `Last checked ${hours} hour${hours === 1 ? "" : "s"} ago`;
  const date = time.toLocaleString(zh ? "zh-CN" : "en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
  return zh ? `上次检查：${date}` : `Last checked ${date}`;
}

/** A download's size in megabytes. */
export function megabytes(bytes: number): string {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

/** How far a download is, 0–100. */
export function percent(progress: UpdateView["progress"]): number {
  if (!progress || progress.total <= 0) return 0;
  return Math.min(100, Math.floor((progress.done * 100) / progress.total));
}
