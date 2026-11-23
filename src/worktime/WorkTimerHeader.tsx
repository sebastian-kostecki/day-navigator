import { Button } from "../components";
import { formatClock, formatDuration, formatSigned } from "./format";
import { useWorkTimer } from "./useWorkTimer";
import "./WorkTimerHeader.css";

export function WorkTimerHeader() {
  const { timer, nowMs, busy, actionError, start, stop, retry } = useWorkTimer();

  if (timer.state === "loading") {
    return <div className="dn-timer dn-timer-note">Czas pracy: wczytywanie…</div>;
  }

  if (timer.state === "error") {
    return (
      <div className="dn-timer-error" role="alert">
        <span>
          Czas pracy niedostępny: {timer.message}
          {timer.lastOkAt !== null && ` (ostatnio OK ${formatClock(timer.lastOkAt)})`}
        </span>
        <Button variant="secondary" size="sm" onClick={retry}>
          Spróbuj ponownie
        </Button>
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
    <div className="dn-timer">
      <span className="dn-clock">Dziś {formatDuration(total)}</span>
      {status.running !== null && <span className="dn-rec">● REC</span>}
      <span
        className={`dn-timer-month${Math.trunc(deviation / 60000) < 0 ? " is-below" : ""}`}
        title="Odchylenie miesiąca od normy (8 h × dni robocze do dziś)"
      >
        Miesiąc {formatSigned(deviation)}
      </span>
      <Button
        variant={status.running ? "secondary" : "primary"}
        size="lg"
        onClick={status.running ? stop : start}
        disabled={busy}
      >
        {status.running ? "STOP" : "START"}
      </Button>
      {actionError && (
        <span className="dn-timer-action-error" role="alert">
          {actionError}
        </span>
      )}
    </div>
  );
}
