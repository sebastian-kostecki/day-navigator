import { Button, formatHms } from "../components";
import { formatClock } from "./format";
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
  const running = status.running !== null;
  // Capped at the day end (a late refetch must not count into tomorrow) and
  // clamped at zero (a clock moved back must not shrink the closed total).
  const total =
    status.todayClosedMs +
    (running && status.runningFromMs !== null
      ? Math.max(0, Math.min(nowMs, status.dayEndMs) - status.runningFromMs)
      : 0);
  // HH:MM:SS, as in the mock: a fixed width while the day grows.
  const clock = formatHms(total).padStart(8, "0");

  return (
    <div className="dn-timer">
      <span className={running ? "dn-rec" : "dn-rec is-stopped"}>
        <span aria-hidden="true">{running ? "●" : "○"}</span> {running ? "REC" : "STOP"}
      </span>
      <span className="clock dn-clock">
        <span className="dn-sr-only">Dziś: </span>
        {clock}
      </span>
      <Button
        variant={running ? "secondary" : "primary"}
        size="lg"
        className="dn-btn-mono"
        onClick={running ? stop : start}
        disabled={busy}
      >
        {running ? "STOP" : "START"}
      </Button>
      {actionError && (
        <span className="dn-timer-action-error" role="alert">
          {actionError}
        </span>
      )}
    </div>
  );
}
