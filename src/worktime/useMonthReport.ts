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
  const lastOkAt = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    const id = ++issued.current;
    let value: Result;
    try {
      value = { state: "loaded", report: await workMonthReport(`${month}-01`) };
    } catch (e) {
      value = { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOkAt.current };
    }
    if (id < applied.current) return;
    applied.current = id;
    if (value.state === "loaded") lastOkAt.current = Date.now();
    setResult({ month, value });
  }, [month]);

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
          value: { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOkAt.current },
        });
      });

    return () => {
      cancelled = true;
      unlisten?.();
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [month, refresh, attempt]);

  const current =
    result !== null &&
    result.month === month &&
    result.value.state === "loaded" &&
    result.value.report.phase === "current";

  // The current month moves with the running counter and at midnight.
  useEffect(() => {
    if (!current) return;
    const handle = setInterval(() => void refresh(), REFRESH_MS);
    return () => clearInterval(handle);
  }, [current, refresh]);

  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  const report: MonthReportState =
    result !== null && result.month === month ? result.value : { state: "loading" };

  return { report, retry };
}
