import { invoke } from "@tauri-apps/api/core";
import type {
  DayEntries,
  EntryRow,
  MonthReport,
  ReminderSettingsError,
  ReminderThresholds,
  WorkTimeError,
  WorkTimerStatus,
} from "./types";

export { WORK_TIME_CHANGED } from "./types";

export function workTimerStatus(): Promise<WorkTimerStatus> {
  return invoke<WorkTimerStatus>("work_timer_status");
}

/** `day` is `YYYY-MM-DD`. */
export function listWorkEntries(day: string): Promise<DayEntries> {
  return invoke<DayEntries>("list_work_entries", { day });
}

export function startWork(): Promise<WorkTimerStatus> {
  return invoke<WorkTimerStatus>("start_work");
}

export function stopWork(): Promise<WorkTimerStatus> {
  return invoke<WorkTimerStatus>("stop_work");
}

/** `start` and `end` are local `YYYY-MM-DDTHH:MM`. */
export function addWorkEntry(start: string, end: string, note: string): Promise<void> {
  return invoke<void>("add_work_entry", { start, end, note });
}

/** `end` is null for the running entry. */
export function updateWorkEntry(
  id: number,
  start: string,
  end: string | null,
  note: string,
): Promise<void> {
  return invoke<void>("update_work_entry", { id, start, end, note });
}

export function removeWorkEntry(id: number): Promise<void> {
  return invoke<void>("remove_work_entry", { id });
}

/** `day` is any `YYYY-MM-DD` within the month. */
export function workMonthReport(day: string): Promise<MonthReport> {
  return invoke<MonthReport>("work_month_report", { day });
}

/** `day` is any `YYYY-MM-DD` within the month; TSV for the clipboard. */
export function workMonthReportText(day: string): Promise<string> {
  return invoke<string>("work_month_report_text", { day });
}

/** `day` is `YYYY-MM-DD`; a weekend is refused with `NotAWorkday`, a holiday with `Holiday`. */
export function setDayOff(day: string, off: boolean): Promise<void> {
  return invoke<void>("set_day_off", { day, off });
}

/** Read-only: the reminder thresholds and whether `settings.json` set them. */
export function reminderSettings(): Promise<ReminderThresholds> {
  return invoke<ReminderThresholds>("reminder_settings");
}

// Same wording as the "Przypomnienia" health source (`REMINDERS_INVALID_PL` in
// src-tauri/src/health/probe.rs); a Rust test there keeps the two equal.
const REMINDERS_INVALID_PL =
  "Blok „reminders” w settings.json jest nieprawidłowy: longRunningHours to liczba godzin 1–24 albo false, notRunningAfter to godzina „GG:MM” albo false.";

/** Polish message for anything thrown by `reminderSettings`. */
export function reminderSettingsErrorMessage(e: unknown): string {
  if (typeof e === "object" && e !== null && "kind" in e) {
    const err = e as ReminderSettingsError;
    switch (err.kind) {
      case "SettingsInvalid":
        return `${REMINDERS_INVALID_PL} Przypomnienia są wstrzymane, dopóki go nie poprawisz.`;
      case "Background":
        return "Nie udało się wczytać ustawień przypomnień, spróbuj ponownie";
      default:
        return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
    }
  }
  return `Nieoczekiwany błąd (${String(e)})`;
}

export function asWorkTimeError(e: unknown): WorkTimeError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as WorkTimeError;
  }
  return null;
}

function hhmm(local: string): string {
  return local.length >= 16 ? local.slice(11, 16) : local;
}

function storeMessage(kind: string): string {
  switch (kind) {
    case "Busy":
      return "Dane aplikacji są zajęte, spróbuj ponownie";
    case "Damaged":
      return "Plik danych aplikacji jest uszkodzony";
    case "Missing":
      return "Brakuje pliku danych aplikacji, choć istnieją kopie zapasowe";
    case "NewerSchema":
      return "Dane aplikacji pochodzą z nowszej wersji programu";
    case "DirUnavailable":
      return "Folder danych aplikacji jest niedostępny";
    case "Io":
      return "Błąd odczytu lub zapisu danych aplikacji";
    default:
      return "Błąd bazy danych aplikacji";
  }
}

/**
 * Polish text for a work-time failure. `entriesById` (the loaded list) lets an
 * overlap name the other entry's times.
 */
export function workTimeErrorMessage(e: unknown, entriesById?: Map<number, EntryRow>): string {
  const err = asWorkTimeError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeMessage(err.detail.kind);
    case "AlreadyRunning":
      return "Czas pracy już trwa";
    case "NotRunning":
      return "Czas pracy nie jest uruchomiony";
    case "NotFound":
      return "Nie znaleziono wpisu";
    case "EndNotAfterStart":
      return "Koniec musi być później niż początek";
    case "InFuture":
      return "Czas nie może być w przyszłości";
    case "Overlap": {
      const other = entriesById?.get(err.detail.entryId);
      if (!other) return "Wpis nachodzi na inny wpis";
      const end = other.endLocal ? hhmm(other.endLocal) : "trwa";
      return `Wpis nachodzi na inny wpis (${hhmm(other.startLocal)}–${end})`;
    }
    case "InvalidLocalTime":
      return "Nieprawidłowy lub nieistniejący czas lokalny";
    case "RunningEndEdit":
      return "Trwający wpis nie ma końca — zakończ go przyciskiem STOP";
    case "Holiday":
      return "Święto nie jest dniem roboczym";
    case "SettingsInvalid":
      return "Nie można odczytać bloku „workTime” z settings.json: plik musi być poprawnym JSON-em z „version”: 1, a holidays to „pl” albo false.";
    case "NotAWorkday":
      return "Weekend nie jest dniem roboczym";
    case "NoteTooLong":
      return `Notatka może mieć najwyżej ${err.detail.max} znaków`;
    case "NoteMultiline":
      return "Notatka musi mieścić się w jednej linii";
    case "Background":
      return "Zadanie w tle nie powiodło się, spróbuj ponownie";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}

/**
 * Polish text for a refused START or STOP. Both take `now` from the system
 * clock, so `EndNotAfterStart` on STOP and `Overlap` on START (a closed entry
 * ending after now) mean the clock is behind the stored times.
 */
export function timerActionErrorMessage(action: "start" | "stop", e: unknown): string {
  const kind = asWorkTimeError(e)?.kind;
  if (action === "stop" && kind === "EndNotAfterStart") {
    return "Zegar systemowy wskazuje czas sprzed początku wpisu — STOP zadziała, gdy zegar go dogoni";
  }
  if (action === "start" && kind === "Overlap") {
    return "Zegar systemowy wskazuje czas sprzed końca innego wpisu — sprawdź zegar lub popraw wpis";
  }
  return workTimeErrorMessage(e);
}
