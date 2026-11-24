import { useEffect, useState } from "react";
import { Button, Field, Panel, formatHm, type Stat } from "../components";
import { workTimeErrorMessage } from "./api";
import { formatClock, formatMonth } from "./format";
import { MonthReport } from "./MonthReport";
import { EntriesPanel } from "./WorkTimeEntries";
import { useMonthReport } from "./useMonthReport";
import { useWorkEntries } from "./useWorkEntries";
import { useWorkTimer, type TimerState } from "./useWorkTimer";
import "./WorkTime.css";

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

function dateKey(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** The local calendar date (never UTC). */
function localToday(): string {
  return dateKey(new Date());
}

function shiftDay(day: string, delta: number): string {
  const [y, m, d] = day.split("-").map(Number);
  return dateKey(new Date(y, m - 1, d + delta));
}

/** Today when the target month is the current one, else the target's 1st. */
function shiftMonth(day: string, delta: number): string {
  const [y, m] = day.split("-").map(Number);
  const first = dateKey(new Date(y, m - 1 + delta, 1));
  const today = localToday();
  return first.slice(0, 7) === today.slice(0, 7) ? today : first;
}

/** Today's total from the timer status, live while the counter runs (as in the header). */
function todayStat(timer: TimerState, nowMs: number): Stat {
  if (timer.state === "loading") return { label: "Dziś", value: "…", sub: "wczytywanie" };
  if (timer.state === "error") {
    return { label: "Dziś", value: "—", sub: "licznik niedostępny", tone: "muted" };
  }
  const { status } = timer;
  const total =
    status.todayClosedMs +
    (status.running !== null && status.runningFromMs !== null
      ? Math.max(0, Math.min(nowMs, status.dayEndMs) - status.runningFromMs)
      : 0);
  return {
    label: "Dziś",
    value: formatHm(total),
    sub:
      status.running === null
        ? "licznik zatrzymany"
        : `start ${formatClock(status.running.startMs)} · trwa`,
  };
}

function HowCounted() {
  return (
    <Panel title="Jak liczę">
      <div className="wt-how body">
        <p>
          Norma = 8 h × dni robocze: poniedziałek–piątek minus dni oznaczone ręcznie jako wolne
          (święta, urlop).
        </p>
        <p>
          Odchylenie w bieżącym miesiącu to przepracowany czas minus norma za dni robocze do dziś
          włącznie: gdy dziś jest dniem roboczym, jego 8 h liczy się od rana. W zamkniętym miesiącu
          to przepracowany czas minus cała norma.
        </p>
        <p>
          Średnio / dzień: czas brakujący do normy miesiąca podzielony na dni robocze po dziś,
          zaokrąglony w górę do minuty. Wpis przez północ liczy się w każdym dniu po kawałku.
        </p>
      </div>
    </Panel>
  );
}

export default function WorkTimeView() {
  const [day, setDay] = useState(localToday);
  const { entries, add, update, remove, setDayOff, dayOffChecked, dayOffBusy, busyIds, retry } =
    useWorkEntries(day);
  const { report, retry: retryReport } = useMonthReport(day);
  const { timer, nowMs: timerNowMs } = useWorkTimer();
  const [nowMs, setNowMs] = useState(() => Date.now());
  // A refusal belongs to the day it happened on.
  const [dayOffError, setDayOffError] = useState<{ day: string; message: string } | null>(null);

  async function toggleDayOff(off: boolean) {
    setDayOffError(null);
    const err = await setDayOff(off);
    if (err) setDayOffError({ day, message: workTimeErrorMessage(err) });
  }

  // `runningFromMs` is set only while the running entry counts in a day that
  // has not ended, so this ticks on today's view (and on a view left open past
  // midnight, where the total stops at the day end).
  const ticking = entries.state === "loaded" && entries.data.runningFromMs !== null;

  // Tick only while a running entry counts in the shown day.
  useEffect(() => {
    if (!ticking) return;
    const handle = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(handle);
  }, [ticking]);

  function changeDay(value: string) {
    if (value) setDay(value);
  }

  const month = day.slice(0, 7);

  const dayOffControl =
    entries.state === "loaded" && entries.data.dayKind !== "weekend" ? (
      <div className="wt-day-off">
        <label className="wt-check">
          <input
            type="checkbox"
            checked={dayOffChecked}
            disabled={dayOffBusy}
            onChange={(e) => void toggleDayOff(e.currentTarget.checked)}
          />
          Dzień wolny
        </label>
        {dayOffError?.day === day && (
          <p className="wt-error" role="alert">
            {dayOffError.message}
          </p>
        )}
      </div>
    ) : null;

  return (
    <main className="container">
      <div className="dn-view-head">
        <h1>Czas pracy</h1>
        <div className="wt-month-switch">
          <Button
            variant="quiet"
            size="sm"
            iconOnly
            icon="chevron-left"
            aria-label="Poprzedni miesiąc"
            onClick={() => setDay(shiftMonth(day, -1))}
          />
          <span className="code wt-month-name" aria-live="polite">
            {formatMonth(month)}
          </span>
          <Button
            variant="quiet"
            size="sm"
            iconOnly
            icon="chevron-right"
            aria-label="Następny miesiąc"
            onClick={() => setDay(shiftMonth(day, 1))}
          />
        </div>
      </div>
      <MonthReport
        state={report}
        today={todayStat(timer, timerNowMs)}
        onRetry={retryReport}
        onPickDay={setDay}
      />
      <div className="dn-columns">
        <div className="dn-col-main wt-stack">
          <Panel title="Wybrany dzień">
            <div className="wt-controls">
              <div className="wt-picker">
                <Button
                  variant="quiet"
                  iconOnly
                  icon="chevron-left"
                  aria-label="Poprzedni dzień"
                  onClick={() => setDay(shiftDay(day, -1))}
                />
                <Field
                  label="Dzień"
                  type="date"
                  value={day}
                  onChange={(e) => changeDay(e.currentTarget.value)}
                />
                <Button
                  variant="quiet"
                  iconOnly
                  icon="chevron-right"
                  aria-label="Następny dzień"
                  onClick={() => setDay(shiftDay(day, 1))}
                />
              </div>
              {dayOffControl}
            </div>
          </Panel>
          <HowCounted />
        </div>
        <div className="dn-col-side">
          <EntriesPanel
            day={day}
            isToday={day === localToday()}
            entries={entries}
            nowMs={nowMs}
            busyIds={busyIds}
            add={add}
            update={update}
            remove={remove}
            retry={retry}
          />
        </div>
      </div>
    </main>
  );
}
