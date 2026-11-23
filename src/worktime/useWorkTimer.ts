import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  WORK_TIME_CHANGED,
  asWorkTimeError,
  startWork,
  stopWork,
  workTimeErrorMessage,
  workTimerStatus,
} from "./api";
import type { WorkTimerStatus } from "./types";

export type TimerState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; status: WorkTimerStatus };

/** Slack after the day end so the backend already reports the new day. */
const DAY_END_SLACK_MS = 500;

export function useWorkTimer() {
  const [timer, setTimer] = useState<TimerState>({ state: "loading" });
  const [nowMs, setNowMs] = useState(() => Date.now());
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  // Latest request wins: every status request takes a number and a response
  // older than the last applied one is dropped, so a slow read cannot show
  // "stopped" after START.
  const issued = useRef(0);
  const applied = useRef(0);
  const lastOkAt = useRef<number | null>(null);

  const apply = useCallback((id: number, next: TimerState) => {
    if (id < applied.current) return;
    applied.current = id;
    if (next.state === "loaded") lastOkAt.current = Date.now();
    setNowMs(Date.now());
    setTimer(next);
  }, []);

  const refresh = useCallback(async () => {
    const id = ++issued.current;
    try {
      apply(id, { state: "loaded", status: await workTimerStatus() });
    } catch (e) {
      apply(id, {
        state: "error",
        message: workTimeErrorMessage(e),
        lastOkAt: lastOkAt.current,
      });
    }
  }, [apply]);

  // Subscribe first and load only once the listener is registered, so no
  // event between the two is lost. The webview may be hidden for days, so also
  // refetch when it becomes visible or focused.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const onVisible = () => {
      if (document.visibilityState === "visible") void refresh();
    };
    const onFocus = () => void refresh();
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", onFocus);

    listen(WORK_TIME_CHANGED, () => {
      if (!cancelled) void refresh();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        void refresh();
      })
      .catch((e) => {
        if (cancelled) return;
        apply(++issued.current, {
          state: "error",
          message: workTimeErrorMessage(e),
          lastOkAt: lastOkAt.current,
        });
      });

    return () => {
      cancelled = true;
      unlisten?.();
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh, apply]);

  const dayEndMs = timer.state === "loaded" ? timer.status.dayEndMs : null;
  const running = timer.state === "loaded" && timer.status.running !== null;

  // Refetch when the local day ends, running or not, so "Dziś" resets.
  useEffect(() => {
    if (dayEndMs === null) return;
    const delay = Math.max(dayEndMs - Date.now(), 0) + DAY_END_SLACK_MS;
    const handle = setTimeout(() => void refresh(), delay);
    return () => clearTimeout(handle);
  }, [dayEndMs, refresh]);

  // Tick only while running.
  useEffect(() => {
    if (!running) return;
    const handle = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(handle);
  }, [running]);

  const act = useCallback(
    async (call: () => Promise<WorkTimerStatus>) => {
      setBusy(true);
      setActionError(null);
      const id = ++issued.current;
      try {
        apply(id, { state: "loaded", status: await call() });
      } catch (e) {
        const kind = asWorkTimeError(e)?.kind;
        if (kind === "AlreadyRunning" || kind === "NotRunning") {
          // Another action already changed the state: just show it.
          await refresh();
        } else {
          setActionError(workTimeErrorMessage(e));
        }
      } finally {
        setBusy(false);
      }
    },
    [apply, refresh],
  );

  const start = useCallback(() => act(startWork), [act]);
  const stop = useCallback(() => act(stopWork), [act]);
  const retry = useCallback(() => void refresh(), [refresh]);

  return { timer, nowMs, busy, actionError, start, stop, retry };
}
