import type { WorkTimerStatus } from "./types";

/**
 * The running span up to now, capped at the day end (a late refetch must not
 * count into tomorrow) and clamped at zero (a clock moved back must not shrink
 * the closed total). `0` when nothing runs.
 */
export function liveSpanMs(runningFromMs: number | null, dayEndMs: number, nowMs: number): number {
  return runningFromMs === null ? 0 : Math.max(0, Math.min(nowMs, dayEndMs) - runningFromMs);
}

/** Today's total: closed time plus the running span (see `liveSpanMs`). */
export function todayTotalMs(status: WorkTimerStatus, nowMs: number): number {
  return (
    status.todayClosedMs +
    liveSpanMs(status.running === null ? null : status.runningFromMs, status.dayEndMs, nowMs)
  );
}
