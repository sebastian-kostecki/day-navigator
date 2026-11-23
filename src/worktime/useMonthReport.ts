import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { WORK_TIME_CHANGED, workMonthReport, workTimeErrorMessage } from "./api";
import type { MonthReport } from "./types";

type Result =
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; report: MonthReport };

/** The state the view sees; a result for another month counts as loading. */
export type MonthReportState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; report: MonthReport };

const REFRESH_MS = 60_000;

/** `day` is `YYYY-MM-DD`; the report is keyed on its month. */
export function useMonthReport(day: string) {
  const month = day.slice(0, 7);
  const [result, setResult] = useState<{ month: string; value: Result } | null>(null);
  // Bumped by retry to redo the subscribe and the load together.
  const [attempt, setAttempt] = useState(0);

  // Latest request wins, as in useWorkTimer.
  const issued = useRef(0);
  const applied = useRef(0);
  // The last successful load per month, so an error never shows another month's time.
  const lastOk = useRef<{ month: string; at: number } | null>(null);
  const lastOkAt = useCallback(
    () => (lastOk.current?.month === month ? lastOk.current.at : null),
    [month],
  );

  const refresh = useCallback(async () => {
    const id = ++issued.current;
    let value: Result;
    try {
      value = { state: "loaded", report: await workMonthReport(`${month}-01`) };
    } catch (e) {
      value = { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOkAt() };
    }
    if (id < applied.current) return;
    applied.current = id;
    if (value.state === "loaded") lastOk.current = { month, at: Date.now() };
    setResult({ month, value });
  }, [month, lastOkAt]);

  // Subscribe first and load only once the listener is registered. The webview
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
        const id = ++issued.current;
        applied.current = id;
        setResult({
          month,
          value: { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOkAt() },
        });
      });

    return () => {
      cancelled = true;
      unlisten?.();
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [month, refresh, lastOkAt, attempt]);

  // Poll every month except a loaded past one, which cannot change on its own:
  // the current month moves with the counter and at midnight, a future month
  // becomes current at midnight on the 1st, and a failed load must recover.
  // An error redoes the subscribe and the load together (as retry does).
  const shown = result !== null && result.month === month ? result.value : null;
  const poll: "refresh" | "resubscribe" | null =
    shown?.state === "error"
      ? "resubscribe"
      : shown?.state === "loaded" && shown.report.phase === "past"
        ? null
        : "refresh";

  useEffect(() => {
    if (poll === null) return;
    const handle = setInterval(() => {
      // Hidden: the visibilitychange/focus listeners refetch on return.
      if (document.visibilityState !== "visible") return;
      if (poll === "resubscribe") setAttempt((n) => n + 1);
      else void refresh();
    }, REFRESH_MS);
    return () => clearInterval(handle);
  }, [poll, refresh]);

  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  const report: MonthReportState =
    result !== null && result.month === month ? result.value : { state: "loading" };

  return { report, retry };
}
