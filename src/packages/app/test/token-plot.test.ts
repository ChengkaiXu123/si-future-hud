import { test } from "node:test";
import assert from "node:assert/strict";
import { tokenPlot } from "../src/lib/statistics.ts";

const hour = 3_600_000;
const at = (iso: string) => Date.parse(iso);
const buckets = (first: string, count: number, step: number) =>
  Array.from({ length: count }, (_, index) => ({
    timestamp: new Date(at(first) + index * step).toISOString(),
  }));

test("the plot runs from the first bucket's start to the last bucket's end, so edge buckets are drawn whole", () => {
  // A 24 h range ending at 16:40, in hour buckets: 16:00 on the first day to 17:00 on the next.
  const plot = tokenPlot(buckets("2026-10-20T16:00:00Z", 25, hour), {
    from: "2026-10-20T16:40:00Z",
    to: "2026-10-21T16:40:00Z",
    bucketMinutes: 60,
    timeZone: "UTC",
  });
  assert.equal(plot.start, at("2026-10-20T16:00:00Z"));
  assert.equal(plot.end, at("2026-10-21T17:00:00Z"));
  assert.equal(plot.columns.length, 25);
  for (const column of plot.columns)
    assert.equal(
      column.end - column.start,
      hour,
      "every column a whole hour wide",
    );
  assert.deepEqual(
    plot.columns.map((column) => column.partial),
    [true, ...Array(23).fill(false), true],
    "only the buckets the range cuts are partial",
  );
  assert.equal(
    plot.from,
    40 / 1500,
    "the range starts 40 minutes into the first bucket",
  );
  assert.equal(plot.to, 1480 / 1500, "and ends 40 minutes into the last");
});

test("a range on bucket boundaries is the plot itself", () => {
  const plot = tokenPlot(buckets("2026-10-21T12:00:00Z", 20, 15 * 60_000), {
    from: "2026-10-21T12:00:00Z",
    to: "2026-10-21T17:00:00Z",
    bucketMinutes: 15,
    timeZone: "UTC",
  });
  assert.equal(plot.start, at("2026-10-21T12:00:00Z"));
  assert.equal(plot.end, at("2026-10-21T17:00:00Z"));
  assert.equal(plot.from, 0);
  assert.equal(plot.to, 1);
  assert.ok(plot.columns.every((column) => !column.partial));
});

test("a day bucket ends at the next local midnight, 25 hours later on the day the clocks go back", () => {
  // London leaves summer time on Sunday 25 October 2026; its days start at 23:00 UTC before and 00:00 after.
  const plot = tokenPlot(
    [
      { timestamp: "2026-10-23T23:00:00Z" },
      { timestamp: "2026-10-24T23:00:00Z" },
    ],
    {
      from: "2026-10-24T09:00:00Z",
      to: "2026-10-25T09:00:00Z",
      bucketMinutes: 1440,
      timeZone: "Europe/London",
    },
  );
  assert.equal(
    plot.columns[0].end,
    at("2026-10-24T23:00:00Z"),
    "a column ends where the next starts",
  );
  assert.equal(plot.end, at("2026-10-26T00:00:00Z"));
  assert.equal(plot.columns[1].end - plot.columns[1].start, 25 * hour);
  const shanghai = tokenPlot([{ timestamp: "2026-10-20T16:00:00Z" }], {
    from: "2026-10-20T20:00:00Z",
    to: "2026-10-21T08:40:00Z",
    bucketMinutes: 1440,
    timeZone: "Asia/Shanghai",
  });
  assert.equal(shanghai.end, at("2026-10-21T16:00:00Z"));
});

test("without buckets the plot is the selected range", () => {
  const plot = tokenPlot([], {
    from: "2026-10-20T16:40:00Z",
    to: "2026-10-21T16:40:00Z",
    bucketMinutes: 60,
    timeZone: "UTC",
  });
  assert.deepEqual(plot, {
    start: at("2026-10-20T16:40:00Z"),
    end: at("2026-10-21T16:40:00Z"),
    columns: [],
    from: 0,
    to: 1,
  });
});
