import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { eventErrorMessage, listEventsToday } from "./api";
import { EVENTS_CHANGED, type EventLog } from "./types";

export type EventLogLoad =
  { state: "loading" } | { state: "error"; message: string } | { state: "loaded"; log: EventLog };

type LogResult = { ok: true; log: EventLog } | { ok: false; message: string };

/** Awaits the read and never rejects. */
function settle(): Promise<LogResult> {
  return listEventsToday().then(
    (log) => ({ ok: true, log }),
    (e: unknown) => ({ ok: false, message: eventErrorMessage(e) }),
  );
}

/** Milliseconds until the next local midnight, plus a second of slack. */
function msToNextMidnight(): number {
  const now = new Date();
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 0, 0, 1);
  return Math.max(1000, next.getTime() - now.getTime());
}

/** Today's event log: live on `events-changed`, on focus and at local midnight. */
export function useEventLog() {
  const [load, setLoad] = useState<EventLogLoad>({ state: "loading" });
  const [lastSuccessAt, setLastSuccessAt] = useState<Date | null>(null);
  // A failed reload over a loaded log: the log stays readable but is stale.
  const [reloadError, setReloadError] = useState<string | null>(null);
  // Bumped by `retry` to re-run the subscribe-and-load effect.
  const [attempt, setAttempt] = useState(0);

  // Only the latest call may set the log: an older answer resolving late
  // would bring back stale rows.
  const latest = useRef(0);
  const apply = useCallback((id: number, result: LogResult) => {
    if (id !== latest.current) return;
    if (result.ok) {
      setLoad({ state: "loaded", log: result.log });
      setLastSuccessAt(new Date());
      setReloadError(null);
    } else {
      const { message } = result;
      setLoad((prev) => (prev.state === "loaded" ? prev : { state: "error", message }));
      setReloadError(message);
    }
  }, []);

  const reload = useCallback(async () => {
    const id = ++latest.current;
    apply(id, await settle());
  }, [apply]);

  // Subscribe first and load only once the listener is registered, so no
  // event between the two is lost. WebView timers do not advance while the
  // laptop sleeps, so also reload on visible/focus, and at the next midnight.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let midnight: ReturnType<typeof setTimeout> | undefined;

    const onVisible = () => {
      if (document.visibilityState === "visible") void reload();
    };
    const onFocus = () => void reload();
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", onFocus);

    const arm = () => {
      midnight = setTimeout(() => {
        void reload();
        if (!cancelled) arm();
      }, msToNextMidnight());
    };
    arm();

    listen(EVENTS_CHANGED, () => {
      if (!cancelled) void reload();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        void reload();
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        apply(++latest.current, { ok: false, message: eventErrorMessage(e) });
      });

    return () => {
      cancelled = true;
      unlisten?.();
      clearTimeout(midnight);
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [reload, apply, attempt]);

  const retry = useCallback(() => {
    setLoad({ state: "loading" });
    setAttempt((n) => n + 1);
  }, []);

  return { load, lastSuccessAt, reloadError, retry, refresh: reload };
}
