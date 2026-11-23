import { ErrorState } from "../components/ErrorState";
import { formatClock, formatHm, formatMonth, formatSigned, formatWorkdays } from "./format";
import type { MonthReport as Report } from "./types";
import type { MonthReportState } from "./useMonthReport";

type CurrentReport = Extract<Report, { phase: "current" }>;

function shortDay(day: string): string {
  return `${day.slice(8, 10)}.${day.slice(5, 7)}`;
}

function workedLine(report: Report): string {
  return `Przepracowano ${formatHm(report.workedMs)} z ${formatHm(report.normMs)} (${formatWorkdays(report.workdays)})`;
}

function neededLine(report: CurrentReport): string {
  if (report.remainingMs === 0) return "Norma miesiąca wyrobiona";
  // `neededPerDayMs` is null exactly when no workday is left after today.
  if (report.workdaysAfterToday === 0 || report.neededPerDayMs === null) {
    return `Do wyrobienia normy brakuje ${formatHm(report.remainingMs)}`;
  }
  return `Średnio ${formatHm(report.neededPerDayMs)} dziennie przez ${formatWorkdays(report.workdaysAfterToday)}`;
}

function Lines({ report, onPickDay }: { report: Report; onPickDay: (day: string) => void }) {
  let lines: React.ReactNode;
  if (report.phase === "future") {
    lines = (
      <p>
        Norma {formatHm(report.normMs)} ({formatWorkdays(report.workdays)})
      </p>
    );
  } else if (report.phase === "past") {
    lines = (
      <>
        <p>{workedLine(report)}</p>
        <p>Odchylenie {formatSigned(report.deviationMs)} (miesiąc zamknięty)</p>
      </>
    );
  } else {
    lines = (
      <>
        <p>{workedLine(report)}</p>
        <p>Odchylenie do dziś {formatSigned(report.deviationMs)}</p>
        <p>{neededLine(report)}</p>
      </>
    );
  }

  return (
    <>
      {lines}
      {report.daysOff.length === 0 ? (
        <p className="empty">Brak dni wolnych</p>
      ) : (
        <p className="days-off">
          Dni wolne:{" "}
          {report.daysOff.map((d) => (
            <button key={d} type="button" className="day-off-link" onClick={() => onPickDay(d)}>
              {shortDay(d)}
            </button>
          ))}
        </p>
      )}
    </>
  );
}

export function MonthReport(props: {
  state: MonthReportState;
  month: string;
  onRetry: () => void;
  onPickDay: (day: string) => void;
}) {
  const { state, month, onRetry, onPickDay } = props;
  let content: React.ReactNode;
  if (state.state === "loading") {
    content = <p>Ładowanie…</p>;
  } else if (state.state === "error") {
    content = (
      <ErrorState
        title="Nie udało się wczytać raportu miesiąca"
        message={
          state.lastOkAt === null
            ? state.message
            : `${state.message} (ostatnio OK ${formatClock(state.lastOkAt)})`
        }
        onRetry={onRetry}
      />
    );
  } else {
    content = <Lines report={state.report} onPickDay={onPickDay} />;
  }
  return (
    <section className="month-report">
      <h2>Miesiąc: {formatMonth(month)}</h2>
      {content}
    </section>
  );
}
