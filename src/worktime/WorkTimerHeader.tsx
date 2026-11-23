import { formatClock, formatDuration } from "./format";
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
  const live = status.running !== null && status.runningFromMs !== null;
  const total = status.todayClosedMs + (live ? nowMs - (status.runningFromMs ?? nowMs) : 0);

  return (
    <div className="work-timer">
      <span className="work-timer-total">Dziś {formatDuration(total)}</span>
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
