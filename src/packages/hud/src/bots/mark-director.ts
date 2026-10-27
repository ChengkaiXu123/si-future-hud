import type { Session, SessionLiveFacts, Snapshot } from "@agent-hud/protocol";
import { vendorKey, vendorsFromSnapshot } from "../model";
import {
  botState,
  eventRank,
  isCalm,
  isWorking,
  type BotBadge,
  type BotEvent,
  type BotLook,
  type BotState,
} from "./bot-state";

// MarkDirector / MarkDirector+Wants / DockMemory. Director times are UTC seconds;
// a host converts a delivered cue to its own monotonic animation clock once.
export interface Direction {
  state: BotState;
  cue: { event: BotEvent; at: number } | null;
}
interface Request {
  id: string;
  sessionId: string;
  kind: "permission" | "question" | "plan";
  at: number;
}
interface Facts {
  id: string;
  phase: Session["state"];
  live: SessionLiveFacts | null;
  observedAt: number;
  lastEventAt: number;
  requests: Request[];
}
interface QueueMark {
  key: string;
  isWorking: boolean;
  sessions: Facts[];
  requests: Request[];
  exhaustedUntil: number | null;
}
interface Want {
  look: BotLook;
  owner: string | null;
  subagents: boolean;
  holds: boolean;
}
interface Mark {
  look: BotLook | null;
  owner: string | null;
  subagents: boolean;
  holds: boolean;
  since: number;
  wanted: { look: BotLook; since: number } | null;
  changes: number[];
  guarded: boolean;
  restingSince: number | null;
  ended: Map<string, number>;
  cue: Direction["cue"];
}
const time = (date: string | null | undefined): number | null => {
  if (!date) return null;
  const value = Date.parse(date) / 1000;
  return Number.isFinite(value) ? value : null;
};
const inflight = (session: Facts) =>
  session.phase === "running" || session.phase === "waiting";
const started = (session: Facts) => time(session.live?.turnStartedAt);
const ended = (session: Facts) => time(session.live?.endedAt);
const toolSeen = (session: Facts) => time(session.live?.activity?.toolAt);
const subagents = (session: Facts) =>
  session.live?.activity?.delegating ?? false;
const openCall = (session: Facts) => session.live?.activity?.openCall ?? null;
const compacting = (session: Facts, now: number) => {
  const at = time(session.live?.activity?.compactingSince);
  return at !== null && now - at < 300;
};
const limited = (session: Facts, now: number) =>
  session.live?.outcome === "limited" &&
  (time(session.live.limitedUntil) ?? Infinity) > now;
const holds = (session: Facts, now: number) =>
  Boolean(
    session.live?.activity?.retrying ||
    (session.live?.activity?.awaitsModel &&
      !openCall(session) &&
      !subagents(session) &&
      session.requests.length === 0 &&
      now - session.observedAt >= 180),
  );
function lookOf(session: Facts, now: number): BotLook {
  const activity = session.live?.activity;
  if (!activity) return "plain";
  if (activity.retrying) return "thinking";
  const call = activity.openCall;
  if (call?.work === "shell" && now - (time(call.at) ?? now) >= 20)
    return "tool-long-command";
  const kind = call?.work ?? activity.work;
  const tool =
    kind === "shell" || kind === "command"
      ? "tool-command"
      : kind === "edit"
        ? "tool-edit"
        : kind === "read"
          ? "tool-read"
          : "thinking";
  if (
    tool === "thinking" ||
    call ||
    now - (toolSeen(session) ?? -Infinity) < 10
  )
    return tool;
  return "thinking";
}
function ownerOf(sessions: Facts[], current: string | null): Facts | undefined {
  return (
    sessions.find((session) => session.id === current) ??
    [...sessions].sort(
      (a, b) =>
        b.observedAt - a.observedAt ||
        (started(b) ?? -Infinity) - (started(a) ?? -Infinity) ||
        a.id.localeCompare(b.id),
    )[0]
  );
}
function wantOf(mark: QueueMark, owner: string | null, now: number): Want {
  const request = [...mark.requests].sort(
    (a, b) =>
      ({ permission: 2, question: 1, plan: 0 })[b.kind] -
        { permission: 2, question: 1, plan: 0 }[a.kind] ||
      a.at - b.at ||
      a.id.localeCompare(b.id),
  )[0];
  if (request)
    return {
      look: request.kind,
      owner: request.sessionId,
      subagents: false,
      holds: false,
    };
  const compact = ownerOf(
    mark.sessions.filter((session) => compacting(session, now)),
    owner,
  );
  if (compact)
    return {
      look: "compacting",
      owner: compact.id,
      subagents: subagents(compact),
      holds: false,
    };
  const busy = mark.sessions.filter(inflight);
  let session = ownerOf(busy, owner);
  if (session) {
    let look = lookOf(session, now);
    if (look === "thinking") {
      const tooling = ownerOf(
        busy.filter((session) => lookOf(session, now).startsWith("tool-")),
        null,
      );
      if (tooling) {
        session = tooling;
        look = lookOf(tooling, now);
      }
    }
    return {
      look,
      owner: session.id,
      subagents: subagents(session),
      holds: holds(session, now),
    };
  }
  return {
    look: mark.isWorking
      ? "plain"
      : mark.sessions.some((session) => session.phase === "unverified")
        ? "no-news"
        : mark.sessions.every((session) => now - session.lastEventAt >= 3600)
          ? "asleep"
          : "idle",
    owner: null,
    subagents: false,
    holds: false,
  };
}
function badgeOf(
  mark: QueueMark,
  lookedAt: number,
  now: number,
): BotBadge | null {
  const prompted = Math.max(
    -Infinity,
    ...mark.sessions.map((session) => started(session) ?? -Infinity),
  );
  if (
    mark.sessions.some(
      (session) =>
        session.live?.outcome === "failed" &&
        now - (ended(session) ?? -Infinity) < 900 &&
        (ended(session) ?? -Infinity) > lookedAt &&
        prompted <= (ended(session) ?? -Infinity),
    )
  )
    return "failed";
  return mark.sessions.some(
    (session) =>
      limited(session, now) && prompted <= (ended(session) ?? Infinity),
  ) || (mark.exhaustedUntil ?? -Infinity) > now
    ? "limited"
    : null;
}
export function botQueue(
  snapshot: Snapshot,
  now = Date.now() / 1000,
): QueueMark[] {
  return vendorsFromSnapshot(snapshot, "logos").map((vendor) => {
    const requests: Request[] = snapshot.approvals
      .filter((item) => vendorKey(item.source) === vendor.key)
      .map((item) => ({
        id: item.id,
        sessionId: item.sessionId ?? item.id,
        kind: item.kind ?? "permission",
        at: time(item.createdAt) ?? 0,
      }));
    const sessions = vendor.sessions
      .filter(
        (session) =>
          (time(session.liveFacts?.lastEventAt ?? session.updatedAt) ?? 0) >=
          now - 86400,
      )
      .map((session) => {
        const live = session.liveFacts ?? null;
        const ownRequests = (live?.requests ?? []).map((item) => ({
          ...item,
          at: time(item.at) ?? 0,
        }));
        for (const request of requests.filter(
          (item) => item.sessionId === session.id,
        ))
          if (!ownRequests.some((item) => item.id === request.id))
            ownRequests.push(request);
        return {
          id: session.id,
          phase: session.state,
          live,
          observedAt: time(live?.observedAt ?? session.updatedAt) ?? 0,
          lastEventAt: time(live?.lastEventAt ?? session.updatedAt) ?? 0,
          requests: ownRequests,
        };
      });
    const merged = [
      ...new Map(
        [...requests, ...sessions.flatMap((session) => session.requests)].map(
          (item) => [item.id, item],
        ),
      ).values(),
    ];
    const resets = vendor.quotas
      .filter(
        (quota) => quota.status !== "stale" && (quota.usedPercent ?? 0) >= 99.5,
      )
      .map((quota) => time(quota.resetAt))
      .filter((value): value is number => value !== null);
    return {
      key: vendor.key,
      isWorking: vendor.working > 0,
      sessions,
      requests: merged,
      exhaustedUntil: resets.length ? Math.max(...resets) : null,
    };
  });
}
export class MarkDirector {
  readonly startedAt: number;
  directions: Record<string, Direction> = {};
  private queue: QueueMark[] = [];
  private marks = new Map<string, Mark>();
  private lookedAt = new Map<string, number>();
  constructor(startedAt: number) {
    this.startedAt = startedAt;
  }
  read(snapshot: Snapshot, now: number) {
    this.queue = botQueue(snapshot, now);
    this.advance(now);
  }
  looked(keys: string[] | null, now: number) {
    for (const key of keys ?? this.queue.map((mark) => mark.key))
      this.lookedAt.set(key, now);
    this.advance(now);
  }
  advance(now: number) {
    const kept = new Map<string, Mark>();
    const directions: Record<string, Direction> = {};
    for (const queue of this.queue) {
      const mark = this.marks.get(queue.key) ?? {
        look: null,
        owner: null,
        subagents: false,
        holds: false,
        since: -Infinity,
        wanted: null,
        changes: [],
        guarded: false,
        restingSince: null,
        ended: new Map(),
        cue: null,
      };
      const want = wantOf(queue, mark.owner, now),
        resting = mark.restingSince;
      const rested = this.follow(
        mark,
        want,
        now,
        Math.max(
          -Infinity,
          ...queue.sessions.map((session) => session.lastEventAt),
        ),
      );
      for (const session of queue.sessions) {
        const end = ended(session);
        if (
          end !== null &&
          session.live?.outcome &&
          end >
            Math.max(this.startedAt, mark.ended.get(session.id) ?? -Infinity)
        ) {
          mark.ended.set(session.id, end);
          if (session.live?.outcome === "failed")
            this.tell(mark, "failed", now);
          else if (session.live?.outcome === "limited")
            this.tell(mark, "limit-hit", now);
        }
      }
      const ids = new Set(queue.sessions.map((session) => session.id));
      for (const key of mark.ended.keys())
        if (!ids.has(key)) mark.ended.delete(key);
      const owned = queue.sessions.find((session) => session.id === mark.owner);
      if (
        rested &&
        queue.requests.length === 0 &&
        owned &&
        (started(owned) ?? -Infinity) >
          Math.max(this.startedAt, resting ?? -Infinity)
      )
        this.tell(mark, "start", now);
      const look = mark.look ?? "idle";
      directions[queue.key] = {
        state: botState({
          look,
          subagents: mark.subagents && isWorking(look),
          badge: badgeOf(queue, this.lookedAt.get(queue.key) ?? -Infinity, now),
          holds: mark.holds && look === "thinking",
        }),
        cue: mark.cue,
      };
      kept.set(queue.key, mark);
    }
    this.marks = kept;
    this.directions = directions;
  }
  nextChange(now: number): number | null {
    const times: number[] = [];
    const take = (value: number | null) => {
      if (value !== null && value > now && Number.isFinite(value))
        times.push(value);
    };
    for (const queue of this.queue) {
      const mark = this.marks.get(queue.key);
      if (mark?.wanted)
        take(
          Math.max(mark.wanted.since + 2, mark.since + (mark.guarded ? 12 : 6)),
        );
      for (const session of queue.sessions) {
        take(session.lastEventAt + 3600);
        const call = openCall(session);
        if (call?.work === "shell") take((time(call.at) ?? Infinity) + 20);
        take((time(session.live?.activity?.compactingSince) ?? Infinity) + 300);
        if (limited(session, now)) take(time(session.live?.limitedUntil));
        if (inflight(session)) {
          if (!call) take((toolSeen(session) ?? Infinity) + 10);
          take(session.observedAt + 180);
        }
        if (session.live?.outcome === "failed")
          take((ended(session) ?? Infinity) + 900);
      }
      take(queue.exhaustedUntil);
    }
    return times.length ? Math.min(...times) : null;
  }
  private follow(
    mark: Mark,
    want: Want,
    now: number,
    restingFrom: number,
  ): boolean {
    const shown = mark.look;
    if (shown === null) {
      this.show(mark, want, now);
      if (isCalm(want.look)) mark.restingSince = restingFrom;
      return false;
    }
    if (mark.guarded && now - (mark.changes.at(-1) ?? now) >= 30) {
      mark.guarded = false;
      mark.changes = [];
    }
    if (want.look === shown) {
      mark.wanted = null;
      this.show(mark, want, now);
      return false;
    }
    if (
      !isWorking(want.look) ||
      !isWorking(shown) ||
      want.look === "compacting" ||
      shown === "compacting" ||
      want.owner !== mark.owner
    ) {
      const rested =
        shown === "asleep" ||
        (shown === "idle" && now - (mark.restingSince ?? now) >= 300);
      this.show(mark, want, now);
      return isWorking(want.look) && rested;
    }
    if (mark.wanted?.look !== want.look)
      mark.wanted = { look: want.look, since: now };
    if (
      now - mark.wanted.since >= 2 &&
      now - mark.since >= (mark.guarded ? 12 : 6)
    )
      this.show(mark, want, now);
    else {
      mark.subagents = want.subagents;
      mark.holds = want.holds;
    }
    return false;
  }
  private show(mark: Mark, want: Want, now: number) {
    const before = mark.look;
    if (isWorking(want.look)) {
      mark.owner = want.owner;
      if (want.look !== before) {
        mark.since = now;
        mark.changes = mark.changes
          .filter((time) => now - time < 30)
          .concat(now);
        if (mark.changes.length >= 4) mark.guarded = true;
      }
    } else if (isCalm(want.look)) {
      mark.owner = null;
      if (before === null || !isCalm(before)) mark.restingSince = now;
    }
    mark.look = want.look;
    mark.wanted = null;
    mark.subagents = want.subagents;
    mark.holds = want.holds;
  }
  private tell(mark: Mark, event: BotEvent, now: number) {
    if (
      mark.cue &&
      now - mark.cue.at < 3 &&
      eventRank(mark.cue.event) > eventRank(event)
    )
      return;
    mark.cue = { event, at: now };
  }
}
export class DockMemory {
  private handed = new Map<string, Direction["cue"]>();
  private replays = new Map<string, Direction["cue"]>();
  cue(
    key: string,
    cue: Direction["cue"],
    badge: BotBadge | null,
    now: number,
  ): Direction["cue"] {
    if (!cue) return null;
    const before = this.handed.get(key);
    if (before?.at === cue.at && before.event === cue.event)
      return this.replays.get(key) ?? cue;
    this.handed.set(key, cue);
    this.replays.delete(key);
    if (
      now - cue.at > 3 &&
      ((cue.event === "failed" && badge === "failed") ||
        (cue.event === "limit-hit" && badge === "limited"))
    ) {
      const replay = { event: cue.event, at: now };
      this.replays.set(key, replay);
      return replay;
    }
    return cue;
  }
}
export function monotonicDirection(
  direction: Direction,
  epochOffset: number,
): Direction {
  return {
    ...direction,
    cue: direction.cue
      ? { ...direction.cue, at: direction.cue.at - epochOffset }
      : null,
  };
}
