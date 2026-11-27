import {
  Button,
  DataTable,
  DataTableCell,
  DataTableRow,
  HoursBar,
  Segmented,
  formatHm,
} from "../components";
import { formatDayLabel, formatEntryCount, formatSigned } from "./format";
import type { DayMark, DayReport, MonthReport } from "./types";
import type { DayOffToggle } from "./useDayOffToggle";

const HOUR_MS = 3_600_000;

const DAY_MARK_OPTIONS = [
  { value: "workday", label: "nie" },
  { value: "halfDayOff", label: "½" },
  { value: "dayOff", label: "tak" },
];

/** `DD.MM` of a `YYYY-MM-DD` day. */
function dm(day: string): string {
  return `${day.slice(8, 10)}.${day.slice(5, 7)}`;
}

/** Today's figures follow the running counter; every other day shows the report. */
function liveFigures(
  row: DayReport,
  isLiveToday: boolean,
  liveTodayMs: number | null,
): { workedMs: number; deviationMs: number | null } {
  if (!isLiveToday || liveTodayMs === null) {
    return { workedMs: row.workedMs, deviationMs: row.deviationMs };
  }
  return {
    workedMs: liveTodayMs,
    deviationMs: row.deviationMs === null ? null : row.deviationMs + (liveTodayMs - row.workedMs),
  };
}

function DayRow(props: {
  row: DayReport;
  today: string;
  /** Today's total from the shared timer; `null` when it is unavailable. */
  liveTodayMs: number | null;
  /** Only the current month has a "today" row to run live. */
  live: boolean;
  onPickDay: (day: string) => void;
  dayOff: DayOffToggle;
}) {
  const { row, today, liveTodayMs, live, onPickDay, dayOff } = props;
  const isToday = row.day === today;
  const weekend = row.kind === "weekend";
  const { workedMs, deviationMs } = liveFigures(row, isToday && live, liveTodayMs);
  const textTone = weekend ? "muted" : undefined;

  const showSum = workedMs > 0 || (row.day <= today && row.normMs > 0);
  const showBar = row.holiday !== null ? workedMs > 0 : !weekend || workedMs > 0;
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
      <DataTableCell tone={row.entryCount === 0 ? "faint" : textTone}>
        {row.entryCount === 0 ? "—" : formatEntryCount(row.entryCount)}
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
          {row.holiday !== null && (
            <span className="wt-days-holiday">
              {row.dayOffOwed ? `${row.holiday} · należy się inny dzień wolny` : row.holiday}
            </span>
          )}
          {row.kind === "halfDayOff" && <span className="wt-days-holiday">pół dnia wolnego</span>}
        </span>
      </DataTableCell>
      <DataTableCell tone={showSum ? textTone : "faint"} className="wt-days-num code">
        {showSum ? formatHm(workedMs) : "—"}
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
      <DataTableCell className="wt-days-switch">
        {(row.kind === "workday" || row.kind === "halfDayOff" || row.kind === "dayOff") && (
          <Segmented
            label={`Dzień wolny ${dm(row.day)}`}
            options={DAY_MARK_OPTIONS}
            value={dayOff.mark(row.day)}
            disabled={dayOff.busy(row.day)}
            onChange={(v) => void dayOff.set(row.day, v as DayMark)}
          />
        )}
      </DataTableCell>
    </DataTableRow>
  );
}

/** One row per day of the shown month; the figures come from the backend report. */
export function DaysTable(props: {
  report: MonthReport;
  /** `YYYY-MM-DD`, the local date. */
  today: string;
  liveTodayMs: number | null;
  onPickDay: (day: string) => void;
  dayOff: DayOffToggle;
}) {
  const { report, today, liveTodayMs, onPickDay, dayOff } = props;
  return (
    <DataTable label="Dni miesiąca" className="wt-days">
      <DataTableRow head>
        <DataTableCell header>Dzień</DataTableCell>
        <DataTableCell header>Wpisy</DataTableCell>
        <DataTableCell header>Na tle normy</DataTableCell>
        <DataTableCell header className="wt-days-num">
          Suma
        </DataTableCell>
        <DataTableCell header className="wt-days-num">
          ±
        </DataTableCell>
        <DataTableCell header className="wt-days-switch">
          Wolne
        </DataTableCell>
      </DataTableRow>
      {report.days.map((row) => (
        <DayRow
          key={row.day}
          row={row}
          today={today}
          liveTodayMs={liveTodayMs}
          live={report.phase === "current"}
          onPickDay={onPickDay}
          dayOff={dayOff}
        />
      ))}
    </DataTable>
  );
}
