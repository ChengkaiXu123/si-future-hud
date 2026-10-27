// What a window says while the usage ledger is read through: built for the
// first time ("first"), or again after a data-version change ("reindex").
// The share counts bytes of the logs the 31-day retention keeps, so one long
// log weighs as much as it takes to read. Nothing shows once the run ends.

/** The snapshot's `indexing`, as far as this text needs it. */
export interface IndexingFacts {
  kind?: string | null;
  bytesDone?: number;
  bytesTotal?: number;
}

export interface IndexingText {
  /** 0…1, short of 1 until the run ends. */
  fraction: number;
  /** The run and its share, e.g. "Rebuilding usage history · 45%". */
  title: string;
  /** What is incomplete meanwhile. */
  detail: string;
}

export function indexingText(
  indexing: IndexingFacts | null | undefined,
  language: string,
): IndexingText | null {
  const kind = indexing?.kind;
  if (kind !== "first" && kind !== "reindex") return null;
  const zh = language === "zh";
  const total = Math.max(0, indexing?.bytesTotal ?? 0);
  const done = Math.min(Math.max(0, indexing?.bytesDone ?? 0), total);
  // A run is not done until the core says so: 100% waits for that.
  const fraction = total > 0 ? Math.min(done / total, 0.99) : 0;
  const share = `${Math.floor(fraction * 100)}%`;
  const title =
    kind === "first"
      ? zh
        ? `正在建立用量记录 · ${share}`
        : `Building usage history · ${share}`
      : zh
        ? `正在重新整理用量记录 · ${share}`
        : `Rebuilding usage history · ${share}`;
  const detail = zh
    ? "完成前 Token 和费用不完整"
    : "Tokens and costs are incomplete until it finishes";
  return { fraction, title, detail };
}
