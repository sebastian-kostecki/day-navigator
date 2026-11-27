import { StatStrip, type Stat } from "../components";
import { ErrorState } from "../components/ErrorState";
import { formatClock, formatDaysAfterToday, formatDaysOff, formatHm, formatSigned } from "./format";
import type { MonthReport as Report } from "./types";
import type { MonthReportState } from "./useMonthReport";

/** The month tiles that fit the month's phase, after the "Dziś" tile. */
function stats(report: Report): Stat[] {
  const worked: Stat = {
    label: "Miesiąc",
    value: formatHm(report.workedMs),
    sub: `z ${formatHm(report.normMs)} normy`,
  };
  const daysOff: Stat = {
    label: "Dni wolne",
    value: formatDaysOff(report.daysOff.length, report.halfDaysOff.length),
    sub:
      report.halfDaysOff.length > 0
        ? `oznaczone ręcznie, w tym ${report.halfDaysOff.length} × pół dnia`
        : "oznaczone ręcznie",
    tone: "soft",
  };
  if (report.phase === "future") return [worked, daysOff];

  const deviation: Stat = {
    label: "Odchylenie",
    value: formatSigned(report.deviationMs),
    sub: report.phase === "past" ? "miesiąc zamknięty" : "do dziś włącznie",
    tone: report.deviationMs < 0 ? "warn" : "info",
  };
  if (report.phase === "past") return [worked, deviation, daysOff];

  // `neededPerDayMs` is null exactly when no workday is left after today.
  let neededSub: string;
  if (report.remainingMs === 0) {
    neededSub = "norma miesiąca wyrobiona";
  } else if (report.neededPerDayMs === null) {
    neededSub = `brak dni roboczych, brakuje ${formatHm(report.remainingMs)}`;
  } else {
    neededSub = formatDaysAfterToday(report.workdaysAfterToday, report.halfDaysAfterToday);
  }
  return [
    worked,
    deviation,
    {
      label: "Średnio / dzień",
      value: report.neededPerDayMs === null ? "—" : formatHm(report.neededPerDayMs),
      sub: neededSub,
    },
    daysOff,
  ];
}

export function MonthReport(props: {
  state: MonthReportState;
  /** Today's total, shown first whatever month is shown. */
  today: Stat;
  onRetry: () => void;
}) {
  const { state, today, onRetry } = props;
  let content: React.ReactNode;
  if (state.state === "loading") {
    content = <p className="dn-loading">Ładowanie…</p>;
  } else if (state.state === "error") {
    content = (
      <ErrorState
        title="Nie udało się wczytać raportu miesiąca"
        message={state.message}
        lastSuccess={
          state.lastOkAt === null ? undefined : `ostatnio OK ${formatClock(state.lastOkAt)}`
        }
        onRetry={onRetry}
      />
    );
  } else {
    const { report } = state;
    content = <StatStrip items={[today, ...stats(report)]} />;
  }
  return (
    <section className="wt-month" aria-label="Raport miesiąca">
      {content}
    </section>
  );
}
