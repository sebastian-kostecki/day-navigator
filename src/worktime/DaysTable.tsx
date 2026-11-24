import { Button, DataTable, DataTableCell, DataTableRow, HoursBar, formatHm } from "../components";
import { formatDayLabel, formatEntryCount, formatSigned } from "./format";
import type { DayReport, MonthReport } from "./types";
import type { DayOffToggle } from "./useDayOffToggle";

const HOUR_MS = 3_600_000;

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
  const showBar = !weekend || workedMs > 0;
  const barLabel =
    row.normMs > 0
      ? `${formatHm(workedMs)} na tle normy ${formatHm(row.normMs)}`
      : `${formatHm(workedMs)}, bez normy`;

  return (
    <DataTableRow current={isToday} hatched={row.kind === "dayOff"}>
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
      <DataTableCell>
        {showBar && (
          <HoursBar
            hours={workedMs / HOUR_MS}
            norm={row.normMs / HOUR_MS}
            today={isToday}
            label={barLabel}
          />
        )}
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
        {!weekend && (
          <label className="wt-check">
            <input
              type="checkbox"
              role="switch"
              aria-label={`Dzień wolny ${dm(row.day)}`}
              checked={dayOff.checked(row.day)}
              disabled={dayOff.busy(row.day)}
              onChange={(e) => void dayOff.toggle(row.day, e.currentTarget.checked)}
            />
          </label>
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
