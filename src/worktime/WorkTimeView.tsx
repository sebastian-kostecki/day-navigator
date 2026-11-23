import { useEffect, useState } from "react";
import { ErrorState } from "../components/ErrorState";
import { workTimeErrorMessage } from "./api";
import { formatClock, formatDuration } from "./format";
import type { EntryRow, WorkTimeError } from "./types";
import { useWorkEntries } from "./useWorkEntries";

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

/** `HH:MM`, prefixed with `DD.MM` when the instant is outside the shown day. */
function localLabel(local: string, day: string): string {
  const time = local.slice(11, 16);
  const date = local.slice(0, 10);
  return date === day ? time : `${date.slice(8, 10)}.${date.slice(5, 7)} ${time}`;
}

function rowRange(row: EntryRow, day: string): string {
  const end = row.endLocal === null ? "trwa" : localLabel(row.endLocal, day);
  return `${localLabel(row.startLocal, day)}–${end}`;
}

type SaveFn = (start: string, end: string | null) => Promise<WorkTimeError | null>;

function EntryForm(props: {
  initialStart: string;
  initialEnd: string | null;
  showEnd: boolean;
  submitLabel: string;
  onSave: SaveFn;
  onCancel?: () => void;
  entriesById: Map<number, EntryRow>;
  /** Clears the form after a successful add. */
  resetOnSave?: boolean;
}) {
  const { initialStart, initialEnd, showEnd, submitLabel, onSave, onCancel, entriesById } = props;
  const [start, setStart] = useState(initialStart);
  const [end, setEnd] = useState(initialEnd ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSaving(true);
    const err = await onSave(start, showEnd ? end : null);
    setSaving(false);
    if (err) {
      setError(workTimeErrorMessage(err, entriesById));
    } else if (props.resetOnSave) {
      setStart(initialStart);
      setEnd(initialEnd ?? "");
    }
  }

  return (
    <form className="entry-form" onSubmit={submit}>
      <fieldset disabled={saving}>
        <label>
          Początek
          <input
            type="datetime-local"
            step={60}
            value={start}
            onChange={(e) => setStart(e.currentTarget.value)}
            required
          />
        </label>
        {showEnd && (
          <label>
            Koniec
            <input
              type="datetime-local"
              step={60}
              value={end}
              onChange={(e) => setEnd(e.currentTarget.value)}
              required
            />
          </label>
        )}
        <button type="submit">{submitLabel}</button>
        {onCancel && (
          <button type="button" onClick={onCancel}>
            Anuluj
          </button>
        )}
      </fieldset>
      {error && (
        <p className="task-error" role="alert">
          {error}
        </p>
      )}
    </form>
  );
}

function EntryItem(props: {
  row: EntryRow;
  day: string;
  busy: boolean;
  entriesById: Map<number, EntryRow>;
  onUpdate: (id: number, start: string, end: string | null) => Promise<WorkTimeError | null>;
  onRemove: (id: number) => Promise<WorkTimeError | null>;
}) {
  const { row, day, busy, entriesById, onUpdate, onRemove } = props;
  const [mode, setMode] = useState<"view" | "edit" | "confirm">("view");
  const [removeError, setRemoveError] = useState<string | null>(null);

  async function confirmRemove() {
    setRemoveError(null);
    const err = await onRemove(row.id);
    if (err) {
      setRemoveError(workTimeErrorMessage(err, entriesById));
      setMode("view");
    }
  }

  if (mode === "edit") {
    return (
      <li className="entry-row">
        <EntryForm
          initialStart={row.startLocal}
          initialEnd={row.endLocal}
          showEnd={row.endMs !== null}
          submitLabel="Zapisz"
          entriesById={entriesById}
          onSave={async (start, end) => {
            const err = await onUpdate(row.id, start, end);
            if (!err) setMode("view");
            return err;
          }}
          onCancel={() => setMode("view")}
        />
      </li>
    );
  }

  return (
    <li className="entry-row">
      <div className="entry-main">
        <span className="entry-range">{rowRange(row, day)}</span>
        <span className="entry-duration">{formatDuration(row.dayPortionMs)}</span>
      </div>
      {mode === "confirm" ? (
        <span className="entry-actions">
          Usunąć wpis?
          <button type="button" disabled={busy} onClick={() => void confirmRemove()}>
            Tak
          </button>
          <button type="button" disabled={busy} onClick={() => setMode("view")}>
            Anuluj
          </button>
        </span>
      ) : (
        <span className="entry-actions">
          <button type="button" disabled={busy} onClick={() => setMode("edit")}>
            Edytuj
          </button>
          <button type="button" disabled={busy} onClick={() => setMode("confirm")}>
            Usuń
          </button>
        </span>
      )}
      {removeError && (
        <p className="task-error entry-error" role="alert">
          {removeError}
        </p>
      )}
    </li>
  );
}

export default function WorkTimeView() {
  const [day, setDay] = useState(localToday);
  const { entries, add, update, remove, busyIds, retry } = useWorkEntries(day);
  const [nowMs, setNowMs] = useState(() => Date.now());

  const isToday = day === localToday();
  const ticking = isToday && entries.state === "loaded" && entries.data.runningFromMs !== null;

  // Tick only on today's view with a running entry.
  useEffect(() => {
    if (!ticking) return;
    const handle = setInterval(() => setNowMs(Date.now()), 1000);
    return () => clearInterval(handle);
  }, [ticking]);

  function changeDay(value: string) {
    if (value) setDay(value);
  }

  const picker = (
    <div className="day-picker">
      <button type="button" aria-label="Poprzedni dzień" onClick={() => setDay(shiftDay(day, -1))}>
        ◀
      </button>
      <input
        type="date"
        value={day}
        onChange={(e) => changeDay(e.currentTarget.value)}
        aria-label="Dzień"
      />
      <button type="button" aria-label="Następny dzień" onClick={() => setDay(shiftDay(day, 1))}>
        ▶
      </button>
    </div>
  );

  let body: React.ReactNode;
  if (entries.state === "loading") {
    body = <p>Ładowanie…</p>;
  } else if (entries.state === "error") {
    body = (
      <ErrorState
        title="Nie udało się wczytać wpisów"
        message={
          entries.lastOkAt === null
            ? entries.message
            : `${entries.message} (ostatnio OK ${formatClock(entries.lastOkAt)})`
        }
        onRetry={retry}
      />
    );
  } else {
    const { data } = entries;
    const entriesById = new Map(data.entries.map((r) => [r.id, r]));
    const total =
      data.closedMs +
      (isToday && data.runningFromMs !== null ? Math.max(0, nowMs - data.runningFromMs) : 0);
    body = (
      <>
        {data.entries.length === 0 ? (
          <p className="empty">Brak wpisów w tym dniu</p>
        ) : (
          <ul className="entry-list">
            {data.entries.map((row) => (
              <EntryItem
                key={row.id}
                row={row}
                day={day}
                busy={busyIds.has(row.id)}
                entriesById={entriesById}
                onUpdate={update}
                onRemove={remove}
              />
            ))}
          </ul>
        )}
        <p className="entry-total">
          Razem: <strong>{formatDuration(total)}</strong>
        </p>
        <section className="new-entry">
          <h2>Dodaj wpis</h2>
          <EntryForm
            key={day}
            initialStart={`${day}T08:00`}
            initialEnd={`${day}T09:00`}
            showEnd
            submitLabel="Dodaj"
            entriesById={entriesById}
            resetOnSave
            onSave={(start, end) => add(start, end ?? "")}
          />
        </section>
      </>
    );
  }

  return (
    <main className="container">
      <h1>Czas pracy</h1>
      {picker}
      {body}
    </main>
  );
}
