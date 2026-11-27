import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { WORK_TIME_CHANGED, workTimeErrorMessage, workWeekReport } from "./api";
import type { WeekReport } from "./types";

/** What the panel sees: loading, an error with the last good load, or the report. */
export type WeekReportState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; report: WeekReport };

const REFRESH_MS = 60_000;

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/** The local calendar date `YYYY-MM-DD` (never UTC). */
export function localDate(): string {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/**
 * The report of the week containing today. "Today" is read at each request, so
 * the Sunday to Monday rollover is picked up by the next poll, focus or change.
 */
export function useWeekReport() {
  const [report, setReport] = useState<WeekReportState>({ state: "loading" });
  // Bumped by retry to redo the subscribe and the load together.
  const [attempt, setAttempt] = useState(0);

  // Latest request wins, as in useMonthReport.
  const issued = useRef(0);
  const applied = useRef(0);
  const lastOk = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    const id = ++issued.current;
    let value: WeekReportState;
    try {
      value = { state: "loaded", report: await workWeekReport(localDate()) };
    } catch (e) {
      value = { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOk.current };
    }
    if (id < applied.current) return;
    applied.current = id;
    if (value.state === "loaded") lastOk.current = Date.now();
    setReport(value);
  }, []);

  // Subscribe first and load only once the listener is registered; the webview
  // may be hidden for days, so also refetch when it becomes visible or focused.
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
        applied.current = ++issued.current;
        setReport({
          state: "error",
          message: workTimeErrorMessage(e),
          lastOkAt: lastOk.current,
        });
      });

    return () => {
      cancelled = true;
      unlisten?.();
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh, attempt]);

  // The current week always moves (counter, midnight), so the poll runs in
  // every phase. An error redoes the subscribe and the load together.
  const poll: "refresh" | "resubscribe" = report.state === "error" ? "resubscribe" : "refresh";
  useEffect(() => {
    const handle = setInterval(() => {
      // Hidden: the visibilitychange/focus listeners refetch on return.
      if (document.visibilityState !== "visible") return;
      if (poll === "resubscribe") setAttempt((n) => n + 1);
      else void refresh();
    }, REFRESH_MS);
    return () => clearInterval(handle);
  }, [poll, refresh]);

  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  return { report, retry };
}
