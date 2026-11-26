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
  | { kind: "NotAWorkday" }
  | { kind: "Background" };

/**
 * Mirror of Rust `ReminderThresholds` (camelCase). `null` means that reminder
 * is off; `fromFile` is true when `settings.json` set the field (a value or
 * `false`).
 */
export interface ReminderThresholds {
  longRunning: { hours: number | null; fromFile: boolean };
  /** `time` is a zero-padded `HH:MM`. */
  notRunningAfter: { time: string | null; fromFile: boolean };
}

/** Mirror of Rust `ReminderSettingsQueryError` (tag "kind", content "detail"). */
export type ReminderSettingsError = { kind: "SettingsInvalid" } | { kind: "Background" };

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
  /** Deviation of the current month from its norm, at `asOfMs`. */
  monthDeviationMs: number;
  /** Server time the figures were computed at. */
  asOfMs: number;
}

export interface EntryRow {
  id: number;
  startMs: number;
  endMs: number | null;
  /** Local `YYYY-MM-DDTHH:MM`. */
  startLocal: string;
  endLocal: string | null;
  dayPortionMs: number;
  /** Trimmed one-line note; `""` = none. */
  note: string;
}

export interface DayEntries {
  /** `YYYY-MM-DD`. */
  day: string;
  dayKind: DayKind;
  entries: EntryRow[];
  closedMs: number;
  runningFromMs: number | null;
  dayEndMs: number;
}

export type DayKind = "workday" | "dayOff" | "weekend";

export type MonthPhase = "past" | "current" | "future";

export interface DayReport {
  /** `YYYY-MM-DD`. */
  day: string;
  kind: DayKind;
  workedMs: number;
  normMs: number;
  deviationMs: number | null;
  entryCount: number;
}

interface MonthReportBase {
  /** `YYYY-MM`. */
  month: string;
  asOfMs: number;
  workedMs: number;
  normMs: number;
  workdays: number;
  /** `YYYY-MM-DD`, sorted, this month only. */
  daysOff: string[];
  /** All days of the month, ascending. */
  days: DayReport[];
}

/**
 * Mirrors the flattened Rust `MonthReport` + `NormFigures`: which figures are
 * set depends on `phase` (`norm::figures`), so the union is discriminated on it.
 */
export type MonthReport =
  | (MonthReportBase & {
      phase: "past";
      normToDateMs: number;
      deviationMs: number;
      workdaysAfterToday: null;
      neededPerDayMs: null;
      remainingMs: null;
    })
  | (MonthReportBase & {
      phase: "current";
      normToDateMs: number;
      deviationMs: number;
      workdaysAfterToday: number;
      /** `null` when no workday is left after today. */
      neededPerDayMs: number | null;
      remainingMs: number;
    })
  | (MonthReportBase & {
      phase: "future";
      normToDateMs: null;
      deviationMs: null;
      workdaysAfterToday: null;
      neededPerDayMs: null;
      remainingMs: null;
    });
