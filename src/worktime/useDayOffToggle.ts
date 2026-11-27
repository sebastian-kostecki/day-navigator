import { useState } from "react";
import { setDayOff, workTimeErrorMessage } from "./api";
import type { DayMark } from "./types";
import type { MonthReportState } from "./useMonthReport";

export type DayOffToggle = {
  /** The shown state: the pending request, else the stored one. */
  mark: (day: string) => DayMark;
  /** A request waits for the reloaded report to agree with it. */
  busy: (day: string) => boolean;
  set: (day: string, mark: DayMark) => Promise<void>;
  /** The last refusal; it belongs to a day of the shown month. */
  error: { day: string; message: string } | null;
};

/**
 * Day-state switches of the days table. A request stays pending per day until the
 * reloaded report shows the requested kind, so a switch never snaps back; the
 * report itself refreshes on `work-time-changed`.
 */
export function useDayOffToggle(report: MonthReportState): DayOffToggle {
  const [requests, setRequests] = useState<Record<string, DayMark>>({});
  const [error, setError] = useState<{ day: string; message: string } | null>(null);

  const stored = new Map<string, DayMark>();
  if (report.state === "loaded") {
    for (const d of report.report.days) {
      stored.set(d.day, d.kind === "dayOff" || d.kind === "halfDayOff" ? d.kind : "workday");
    }
  }

  // Drop the requests the report now agrees with (adjusting state while rendering).
  const settled = Object.keys(requests).filter((day) => stored.get(day) === requests[day]);
  if (settled.length > 0) {
    const rest = { ...requests };
    for (const day of settled) delete rest[day];
    setRequests(rest);
  }

  const mark = (day: string): DayMark => requests[day] ?? stored.get(day) ?? "workday";
  const busy = (day: string) => day in requests;

  async function set(day: string, next: DayMark) {
    setError(null);
    setRequests((r) => ({ ...r, [day]: next }));
    try {
      await setDayOff(day, next);
    } catch (e) {
      setRequests((r) => {
        const rest = { ...r };
        delete rest[day];
        return rest;
      });
      setError({ day, message: workTimeErrorMessage(e) });
    }
  }

  // A change of month clears the message.
  const month = report.state === "loaded" ? report.report.month : null;
  const shownError = error !== null && error.day.slice(0, 7) === month ? error : null;

  return { mark, busy, set, error: shownError };
}
