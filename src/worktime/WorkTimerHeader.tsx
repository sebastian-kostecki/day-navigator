import { useEffect, useRef, useState } from "react";
import { Button, Notice, formatHms } from "../components";
import { formatClock } from "./format";
import { todayTotalMs } from "./total";
import { useWorkTimer } from "./useWorkTimer";
import "./WorkTimerHeader.css";

/** The question gives up on its own after this long. */
const CONFIRM_TIMEOUT_MS = 6000;
/** Clicks on "Tak" this soon after the question appears are the tail of a double click. */
const CONFIRM_GUARD_MS = 400;

type Pending = { action: "start" | "stop"; at: number };

export function WorkTimerHeader({ onStopped }: { onStopped?: () => void }) {
  const { timer, nowMs, busy, actionError, start, stop, retry } = useWorkTimer();
  // START/STOP asks first, so a stray click doesn't open or close a work entry.
  const [asked, setPending] = useState<Pending | null>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const running = timer.state === "loaded" && timer.status.running !== null;
  // A question that no longer fits (the counter changed elsewhere, e.g. at the
  // day end) is dropped: "start" only while stopped, "stop" only while running.
  const pending =
    asked && timer.state === "loaded" && (asked.action === "stop") === running ? asked : null;

  // Focus "Anuluj" (it sits where START/STOP was), give up after a while or on Escape.
  useEffect(() => {
    if (!pending) return;
    cancelRef.current?.focus();
    const timeout = setTimeout(() => setPending(null), CONFIRM_TIMEOUT_MS);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setPending(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      clearTimeout(timeout);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [pending]);

  const confirm = () => {
    if (!pending || Date.now() - pending.at < CONFIRM_GUARD_MS) return;
    const action = pending.action;
    setPending(null);
    if (action === "start") void start();
    else
      void stop().then((ok) => {
        if (ok) onStopped?.();
      });
  };

  if (timer.state === "loading") {
    return <div className="dn-timer dn-timer-note">Czas pracy: wczytywanie…</div>;
  }

  if (timer.state === "error") {
    return (
      <Notice
        tone="error"
        compact
        className="dn-timer-error"
        action="Spróbuj ponownie"
        onAction={retry}
      >
        Czas pracy niedostępny: {timer.message}
        {timer.lastOkAt !== null && ` (ostatnio OK ${formatClock(timer.lastOkAt)})`}
      </Notice>
    );
  }

  const { status } = timer;
  const total = todayTotalMs(status, nowMs);
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
      {pending ? (
        <div className="dn-timer-confirm" role="group" aria-labelledby="dn-timer-confirm-q">
          <span id="dn-timer-confirm-q" className="dn-timer-confirm-q">
            {pending.action === "start" ? "Rozpocząć pracę?" : "Zakończyć pracę?"}
          </span>
          <Button
            variant={pending.action === "start" ? "primary" : "secondary"}
            size="lg"
            className="dn-btn-mono"
            onClick={confirm}
            disabled={busy}
          >
            {pending.action === "start" ? "TAK, START" : "TAK, STOP"}
          </Button>
          {/* Last, in START/STOP's place: a double click lands on "Anuluj". */}
          <Button ref={cancelRef} size="lg" onClick={() => setPending(null)}>
            Anuluj
          </Button>
        </div>
      ) : (
        <Button
          variant={running ? "secondary" : "primary"}
          size="lg"
          className="dn-btn-mono"
          onClick={() => setPending({ action: running ? "stop" : "start", at: Date.now() })}
          disabled={busy}
        >
          {running ? "STOP" : "START"}
        </Button>
      )}
      {actionError && (
        <Notice tone="error" compact className="dn-timer-action-error">
          {actionError}
        </Notice>
      )}
    </div>
  );
}
