import { Button, StatStrip, type Stat } from "../components";
import { ErrorState } from "../components/ErrorState";
import { formatClock, formatHm, formatMonth, formatSigned, formatWorkdays } from "./format";
import type { MonthReport as Report } from "./types";
import type { MonthReportState } from "./useMonthReport";

function shortDay(day: string): string {
  return `${day.slice(8, 10)}.${day.slice(5, 7)}`;
}

/** The tiles that fit the month's phase (the same figures the text lines showed). */
function stats(report: Report): Stat[] {
  const worked: Stat = {
    label: "Przepracowane",
    value: formatHm(report.workedMs),
    sub: `z ${formatHm(report.normMs)} normy (${formatWorkdays(report.workdays)})`,
  };
  if (report.phase === "future") return [worked];

  const deviation: Stat = {
    label: "Odchylenie",
    value: formatSigned(report.deviationMs),
    sub: report.phase === "past" ? "miesiąc zamknięty" : "do dziś",
    tone: report.deviationMs < 0 ? "warn" : "info",
  };
  if (report.phase === "past") return [worked, deviation];

  // `neededPerDayMs` is null exactly when no workday is left after today.
  const done = report.remainingMs === 0;
  let neededSub: string | undefined;
  if (done) {
    neededSub = "norma miesiąca wyrobiona";
  } else if (report.neededPerDayMs === null) {
    neededSub = `brak dni roboczych, brakuje ${formatHm(report.remainingMs)}`;
  }
  return [
    worked,
    deviation,
    {
      label: "Średnio / dzień",
      value: report.neededPerDayMs === null ? "—" : formatHm(report.neededPerDayMs),
      sub: neededSub,
    },
    {
      label: "Dni robocze po dziś",
      value: String(report.workdaysAfterToday),
      sub: "bez dzisiejszego dnia",
    },
  ];
}

function Lines({ report, onPickDay }: { report: Report; onPickDay: (day: string) => void }) {
  return (
    <>
      <StatStrip items={stats(report)} />
      {report.daysOff.length === 0 ? (
        <p className="meta wt-muted">Brak dni wolnych</p>
      ) : (
        <p className="meta wt-days-off">
          <span className="label-caps">Dni wolne:</span>
          {report.daysOff.map((d) => (
            <Button key={d} variant="quiet" size="sm" onClick={() => onPickDay(d)}>
              {shortDay(d)}
            </Button>
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
    content = <p className="dn-loading">Ładowanie…</p>;
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
    <section className="wt-month">
      <h2 className="panel-title">Miesiąc: {formatMonth(month)}</h2>
      {content}
    </section>
  );
}
