import { formatHm, type Stat } from "../components";
import type { HealthSnapshot, SourceStatus } from "../health/types";
import { formatClock, formatDaysAfterToday } from "../worktime/format";
import { todayTotalMs } from "../worktime/total";
import type { MonthReportState } from "../worktime/useMonthReport";
import type { TimerState } from "../worktime/useWorkTimer";

/** Today's total: closed time plus the running span, counted as in `worktime/total.ts`. */
function todayTile(timer: TimerState, nowMs: number): Stat {
  if (timer.state === "loading") return { label: "Dziś", value: "…", sub: "wczytywanie" };
  if (timer.state === "error") {
    return { label: "Dziś", value: "—", sub: "czas pracy niedostępny", tone: "warn" };
  }
  const { status } = timer;
  const running = status.running;
  return {
    label: "Dziś",
    value: formatHm(todayTotalMs(status, nowMs)),
    sub: running !== null ? `start ${formatClock(running.startMs)}` : "licznik stoi",
  };
}

function monthTiles(state: MonthReportState): Stat[] {
  const labels = ["Miesiąc", "Odchylenie", "Średnio / dzień"];
  if (state.state === "loading") {
    return labels.map((label) => ({ label, value: "…", sub: "wczytywanie" }));
  }
  if (state.state === "error") {
    return labels.map((label) => ({
      label,
      value: "—",
      sub: "raport niedostępny",
      tone: "warn",
    }));
  }
  const { report } = state;
  const month: Stat = {
    label: "Miesiąc",
    value: formatHm(report.workedMs),
    sub: `z ${formatHm(report.normMs)} normy`,
  };
  const deviation: Stat =
    report.deviationMs === null
      ? { label: "Odchylenie", value: "—", sub: "miesiąc jeszcze nie trwa" }
      : {
          label: "Odchylenie",
          value: formatHm(report.deviationMs, true),
          sub: report.phase === "past" ? "miesiąc zamknięty" : "do dziś",
          tone: report.deviationMs < 0 ? "warn" : "info",
        };
  let perDay: Stat;
  if (report.phase !== "current") {
    perDay = { label: "Średnio / dzień", value: "—" };
  } else if (report.remainingMs === 0) {
    perDay = { label: "Średnio / dzień", value: "—", sub: "norma miesiąca wyrobiona" };
  } else if (report.neededPerDayMs === null) {
    // Null exactly when no workday is left after today.
    perDay = {
      label: "Średnio / dzień",
      value: "—",
      sub: `brak dni roboczych, brakuje ${formatHm(report.remainingMs)}`,
    };
  } else {
    perDay = {
      label: "Średnio / dzień",
      value: formatHm(report.neededPerDayMs),
      sub: formatDaysAfterToday(report.workdaysAfterToday, report.halfDaysAfterToday),
    };
  }
  return [month, deviation, perDay];
}

/** Worst first: an error, then a calm "unavailable", then "not configured". */
const SEVERITY: Record<SourceStatus["state"], number> = {
  error: 0,
  unavailable: 1,
  notConfigured: 2,
  ok: 3,
};

function shorten(text: string, max = 40): string {
  return text.length <= max ? text : `${text.slice(0, max - 1).trimEnd()}…`;
}

export function sourceProblem(s: SourceStatus): string {
  return s.state === "error" || s.state === "unavailable" ? s.message : "nie skonfigurowano";
}

function sourcesTile(snapshot: HealthSnapshot | null, loadError: string | null): Stat {
  if (loadError !== null) {
    return { label: "Źródła", value: "—", sub: "stan nieznany", tone: "warn" };
  }
  if (!snapshot || snapshot.overall === "starting") {
    return { label: "Źródła", value: "…", sub: "sprawdzam", tone: "soft" };
  }
  if (snapshot.overall === "notRunning" && snapshot.checkedAtMs == null) {
    return { label: "Źródła", value: "—", sub: "sprawdzanie nie działa", tone: "warn" };
  }
  // Same rule as the "Stan źródeł" view: sources not configured in settings.json
  // are switched off, so they count neither as available nor as missing.
  const sources = snapshot.sources.filter((s) => s.state !== "notConfigured");
  const ok = sources.filter((s) => s.state === "ok").length;
  const worst = [...sources].sort((a, b) => SEVERITY[a.state] - SEVERITY[b.state])[0];
  return {
    label: "Źródła",
    value: `${ok}/${sources.length}`,
    sub:
      worst && worst.state !== "ok"
        ? `${worst.label}: ${shorten(sourceProblem(worst))}`
        : "wszystkie OK",
    tone: "soft",
  };
}

/** The five tiles above the lists: today, month, deviation, per day, sources. */
export function summaryStats(args: {
  timer: TimerState;
  nowMs: number;
  month: MonthReportState;
  health: HealthSnapshot | null;
  healthError: string | null;
}): Stat[] {
  return [
    todayTile(args.timer, args.nowMs),
    ...monthTiles(args.month),
    sourcesTile(args.health, args.healthError),
  ];
}
