import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { daySummary, daySummaryErrorMessage } from "./api";
import type { DaySummary } from "./types";

const TODO_CHANGED = "todo-changed";

export type DaySummaryState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; summary: DaySummary };

export function useDaySummary(): { state: DaySummaryState; retry: () => void } {
  const [state, setState] = useState<DaySummaryState>({ state: "loading" });
  // Bumped by `retry` to redo the subscribe and the load together.
  const [attempt, setAttempt] = useState(0);
  // Latest request wins.
  const latest = useRef(0);
  const lastOkAt = useRef<number | null>(null);

  const load = useCallback(async () => {
    const id = ++latest.current;
    try {
      const summary = await daySummary();
      if (id !== latest.current) return;
      lastOkAt.current = Date.now();
      setState({ state: "loaded", summary });
    } catch (e) {
      if (id !== latest.current) return;
      setState({
        state: "error",
        message: daySummaryErrorMessage(e),
        lastOkAt: lastOkAt.current,
      });
    }
  }, []);

  // Subscribe first, load once the listener is registered, so no event is lost.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    listen(TODO_CHANGED, () => {
      if (!cancelled) void load();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        void load();
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        latest.current += 1;
        setState({
          state: "error",
          message: daySummaryErrorMessage(e),
          lastOkAt: lastOkAt.current,
        });
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [load, attempt]);

  const retry = useCallback(() => {
    setState({ state: "loading" });
    setAttempt((n) => n + 1);
  }, []);

  return { state, retry };
}
