import { formatClock, formatDuration, formatSigned } from "./format";
import { useWorkTimer } from "./useWorkTimer";

export function WorkTimerHeader() {
  const { timer, nowMs, busy, actionError, start, stop, retry } = useWorkTimer();

  if (timer.state === "loading") {
    return <div className="work-timer">Czas pracy: wczytywanie…</div>;
  }

  if (timer.state === "error") {
    return (
      <div className="work-timer work-timer-error" role="alert">
        <span>
          Czas pracy niedostępny: {timer.message}
          {timer.lastOkAt !== null && ` (ostatnio OK ${formatClock(timer.lastOkAt)})`}
        </span>
        <button type="button" onClick={retry}>
          Spróbuj ponownie
        </button>
      </div>
    );
  }

  const { status } = timer;
  // Capped at the day end (a late refetch must not count into tomorrow) and
  // clamped at zero (a clock moved back must not shrink the closed total).
  const total =
    status.todayClosedMs +
    (status.running !== null && status.runningFromMs !== null
      ? Math.max(0, Math.min(nowMs, status.dayEndMs) - status.runningFromMs)
      : 0);

  // Same extrapolation from the status's own `asOfMs`, capped at the day end.
  const deviation =
    status.monthDeviationMs +
    (status.running !== null ? Math.max(0, Math.min(nowMs, status.dayEndMs) - status.asOfMs) : 0);

  return (
    <div className="work-timer">
      <span className="work-timer-total">Dziś {formatDuration(total)}</span>
      <span
        className="work-timer-month"
        title="Odchylenie miesiąca od normy (8 h × dni robocze do dziś)"
      >
        Miesiąc {formatSigned(deviation)}
      </span>
      <button
        type="button"
        className="work-timer-toggle"
        onClick={status.running ? stop : start}
        disabled={busy}
      >
        {status.running ? "STOP" : "START"}
      </button>
      {actionError && (
        <span className="task-error" role="alert">
          {actionError}
        </span>
      )}
    </div>
  );
}
