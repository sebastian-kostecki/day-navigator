/** Tauri event emitted after every successful work-time write (no payload). */
export const WORK_TIME_CHANGED = "work-time-changed";

/** Mirror of the store's `StoreError` (`{ kind, detail }`; detail varies). */
export type StoreError = { kind: string; detail?: unknown };

/** Mirror of Rust `WorkTimeError` (tag "kind", content "detail", camelCase). */
export type WorkTimeError =
  | { kind: "Store"; detail: StoreError }
  | { kind: "AlreadyRunning" }
  | { kind: "NotRunning" }
  | { kind: "NotFound" }
  | { kind: "EndNotAfterStart" }
  | { kind: "InFuture" }
  | { kind: "Overlap"; detail: { entryId: number } }
  | { kind: "InvalidLocalTime" }
  | { kind: "RunningEndEdit" }
  | { kind: "Background" };

export interface RunningEntry {
  id: number;
  startMs: number;
}

export interface WorkTimerStatus {
  running: RunningEntry | null;
  todayClosedMs: number;
  /** `max(start, dayStart)` of the running entry. */
  runningFromMs: number | null;
  dayStartMs: number;
  dayEndMs: number;
}

export interface EntryRow {
  id: number;
  startMs: number;
  endMs: number | null;
  /** Local `YYYY-MM-DDTHH:MM`. */
  startLocal: string;
  endLocal: string | null;
  dayPortionMs: number;
}

export interface DayEntries {
  /** `YYYY-MM-DD`. */
  day: string;
  entries: EntryRow[];
  closedMs: number;
  runningFromMs: number | null;
  dayEndMs: number;
}
