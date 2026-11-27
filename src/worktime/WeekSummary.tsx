import {
  Button,
  DataTable,
  DataTableCell,
  DataTableRow,
  HoursBar,
  Panel,
  StatStrip,
} from "../components";
import type { Stat } from "../components";
import { ErrorState } from "../components/ErrorState";
import { liveFigures } from "./DaysTable";
import { formatClock, formatDayLabel, formatHm, formatSigned } from "./format";
import { todayTotalMs } from "./total";
import type { DayReport, WeekReport } from "./types";
import { localDate, useWeekReport } from "./useWeekReport";
import { useWorkTimer } from "./useWorkTimer";

const HOUR_MS = 3_600_000;

/** `DD.MM` of a `YYYY-MM-DD` day. */
function dm(day: string): string {
  return `${day.slice(8, 10)}.${day.slice(5, 7)}`;
}

/** `DD.MM – DD.MM.YYYY`; both dates carry the year when the years differ. */
function formatRange(monday: string, sunday: string): string {
  if (monday.slice(0, 4) === sunday.slice(0, 4)) {
    return `${dm(monday)} – ${dm(sunday)}.${sunday.slice(0, 4)}`;
  }
  return `${dm(monday)}.${monday.slice(0, 4)} – ${dm(sunday)}.${sunday.slice(0, 4)}`;
}

/** The note for a non-working day; a holiday name wins on any kind of day. */
function kindLabel(row: DayReport): string | null {
  if (row.holiday !== null) {
    return row.dayOffOwed ? `${row.holiday} · należy się inny dzień wolny` : row.holiday;
  }
  switch (row.kind) {
    case "workday":
      return null;
    case "weekend":
      return "weekend";
    case "dayOff":
      return "dzień wolny";
    case "holiday":
      // A holiday row always has a name.
      return null;
    default: {
      const unknown: never = row.kind;
      return unknown;
    }
  }
}

function stats(report: WeekReport): Stat[] {
  const worked: Stat = {
    label: "Tydzień",
    value: formatHm(report.workedMs),
    sub: `z ${formatHm(report.normMs)} normy`,
  };
  if (report.phase === "future") return [worked];

  const deviation: Stat = {
    label: "Odchylenie",
    value: formatSigned(report.deviationMs),
    sub:
      report.phase === "past"
        ? "tydzień zamknięty"
        : `wobec ${formatHm(report.normToDateMs)} do dziś`,
    tone: report.deviationMs < 0 ? "warn" : "info",
  };
  if (report.phase === "past") return [worked, deviation];

  return [
    worked,
    deviation,
    {
      label: "Zostało",
      value: formatHm(report.remainingMs),
      sub: report.remainingMs === 0 ? "norma tygodnia wyrobiona" : "do normy tygodnia",
    },
  ];
}

function WeekRow(props: {
  row: DayReport;
  today: string;
  live: boolean;
  liveTodayMs: number | null;
  onPickDay: (day: string) => void;
}) {
  const { row, today, live, liveTodayMs, onPickDay } = props;
  const isToday = row.day === today;
  const weekend = row.kind === "weekend";
  const { workedMs, deviationMs } = liveFigures(row, isToday && live, liveTodayMs);
  const textTone = weekend ? "muted" : undefined;

  const showSum = workedMs > 0 || (row.day <= today && row.normMs > 0);
  const showBar = row.holiday !== null ? workedMs > 0 : !weekend || workedMs > 0;
  const label = kindLabel(row);
  const holidayTitle =
    row.holiday === null
      ? undefined
      : row.dayOffOwed
        ? `${row.holiday} — Święto w sobotę: pracodawca wyznacza za nie inny dzień wolny — oznacz go w tabeli`
        : row.holiday;
  const barLabel =
    row.normMs > 0
      ? `${formatHm(workedMs)} na tle normy ${formatHm(row.normMs)}`
      : `${formatHm(workedMs)}, bez normy`;

  return (
    <DataTableRow current={isToday} hatched={row.kind === "dayOff" || row.kind === "holiday"}>
      <DataTableCell tone={textTone} className="code">
        <Button
          variant="quiet"
          size="sm"
          aria-label={`Pokaż dzień ${dm(row.day)}`}
          onClick={() => onPickDay(row.day)}
        >
          {formatDayLabel(row.day)}
        </Button>
      </DataTableCell>
      <DataTableCell title={holidayTitle} tone={row.holiday !== null ? textTone : undefined}>
        <span className="wt-days-note">
          {showBar && (
            <HoursBar
              hours={workedMs / HOUR_MS}
              norm={row.normMs / HOUR_MS}
              today={isToday}
              label={barLabel}
            />
          )}
          {label !== null && <span className="wt-days-holiday">{label}</span>}
        </span>
      </DataTableCell>
      <DataTableCell tone={showSum ? textTone : "faint"} className="wt-days-num code">
        {showSum ? formatHm(workedMs) : "—"}
      </DataTableCell>
      <DataTableCell tone={row.normMs > 0 ? textTone : "faint"} className="wt-days-num code">
        {row.normMs > 0 ? formatHm(row.normMs) : "—"}
      </DataTableCell>
      <DataTableCell
        tone={
          deviationMs === null
            ? "faint"
            : deviationMs > 0
              ? "info"
              : deviationMs < 0
                ? "warn"
                : textTone
        }
        className="wt-days-num code"
      >
        {deviationMs === null ? "—" : formatSigned(deviationMs)}
      </DataTableCell>
    </DataTableRow>
  );
}

/** The current Monday–Sunday week; every figure comes from the backend report. */
export function WeekSummary(props: { onPickDay: (day: string) => void }) {
  const { onPickDay } = props;
  const { report: state, retry } = useWeekReport();
  const { timer, nowMs } = useWorkTimer();
  const today = localDate();
  const liveTodayMs = timer.state === "loaded" ? todayTotalMs(timer.status, nowMs) : null;

  let meta: string | undefined;
  let body: React.ReactNode;
  if (state.state === "loading") {
    body = <p className="dn-loading wt-panel-body">Ładowanie…</p>;
  } else if (state.state === "error") {
    body = (
      <div className="wt-panel-body">
        <ErrorState
          title="Nie udało się wczytać tygodnia"
          message={state.message}
          lastSuccess={
            state.lastOkAt === null ? undefined : `ostatnio OK ${formatClock(state.lastOkAt)}`
          }
          onRetry={retry}
        />
      </div>
    );
  } else {
    const { report } = state;
    meta = formatRange(report.monday, report.sunday);
    body = (
      <>
        <div className="wt-panel-body">
          <StatStrip items={stats(report)} />
        </div>
        <DataTable label="Bieżący tydzień" className="wt-week">
          <DataTableRow head>
            <DataTableCell header>Dzień</DataTableCell>
            <DataTableCell header>Na tle normy</DataTableCell>
            <DataTableCell header className="wt-days-num">
              Suma
            </DataTableCell>
            <DataTableCell header className="wt-days-num">
              Norma
            </DataTableCell>
            <DataTableCell header className="wt-days-num">
              ±
            </DataTableCell>
          </DataTableRow>
          {report.days.map((row) => (
            <WeekRow
              key={row.day}
              row={row}
              today={today}
              live={report.phase === "current"}
              liveTodayMs={liveTodayMs}
              onPickDay={onPickDay}
            />
          ))}
        </DataTable>
      </>
    );
  }

  return (
    <Panel
      title="Bieżący tydzień"
      meta={meta}
      footer="Tydzień od poniedziałku do niedzieli. Norma jak w miesiącu: dni robocze minus święta i dni wolne; odchylenie liczone do dziś włącznie."
    >
      {body}
    </Panel>
  );
}
