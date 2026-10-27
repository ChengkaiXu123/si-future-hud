import { useEffect, useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  monotonicDirection,
  type Direction,
} from "@agent-hud/hud/bots/mark-director";

interface Wire {
  startedAt: number;
  directions: Record<string, Direction>;
}
/** The app directs every screen's Bots; a HUD window draws its decisions. It
 * listens while on screen and reads the current ones when it is back. */
export function useBotDirections(enabled = true) {
  const epochOffset = useRef(Date.now() / 1000 - performance.now() / 1000);
  const [wire, setWire] = useState<Wire>({ startedAt: 0, directions: {} });
  useEffect(() => {
    if (!enabled || !isTauri()) return;
    let disposed = false;
    let revision = 0;
    const subscription = getCurrentWebviewWindow().listen<Wire>(
      "hud:bot-directions",
      ({ payload }) => {
        if (disposed) return;
        revision += 1;
        setWire(payload);
      },
    );
    void subscription
      .then(async () => {
        if (disposed) return;
        const before = revision;
        const value = await invoke<Wire>("hud_bot_directions");
        if (!disposed && before === revision && value) setWire(value);
      })
      .catch((error) => {
        if (!disposed) console.error("Bot directions could not be read", error);
      });
    return () => {
      disposed = true;
      void subscription.then((unlisten) => unlisten());
    };
  }, [enabled]);
  return useMemo(
    () =>
      Object.fromEntries(
        Object.entries(wire.directions).map(([key, direction]) => [
          key,
          monotonicDirection(direction, epochOffset.current),
        ]),
      ),
    [wire],
  );
}
