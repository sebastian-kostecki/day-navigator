import { useState } from "react";
import { Button, Field, Notice, Panel } from "../components";
import { ErrorState } from "../components/ErrorState";
import { workTimeErrorMessage } from "./api";
import { WEEKDAYS, formatClock, formatDuration } from "./format";
import { liveSpanMs } from "./total";
import type { DayEntries, EntryRow } from "./types";
import type { EntriesState } from "./useWorkEntries";

/** `DD.MM` of a `YYYY-MM-DD` day. */
export function shortDay(day: string): string {
  return `${day.slice(8, 10)}.${day.slice(5, 7)}`;
}

/** `DD.MM.YYYY pn` for the panel head. */
function dayMeta(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  return `${shortDay(day)}.${y} ${WEEKDAYS[new Date(y, m - 1, d).getDay()]}`;
}

/** `HH:MM`, prefixed with `DD.MM` when the instant is outside the shown day. */
function localLabel(local: string, day: string): string {
  const time = local.slice(11, 16);
  const date = local.slice(0, 10);
  return date === day ? time : `${shortDay(date)} ${time}`;
}

/** Resolves to `null` on success, or to the failure for `workTimeErrorMessage`. */
type SaveFn = (start: string, end: string | null) => Promise<unknown>;

function EntryForm(props: {
  initialStart: string;
  initialEnd: string | null;
  showEnd: boolean;
  submitLabel: string;
  onSave: SaveFn;
  onCancel: () => void;
  entriesById: Map<number, EntryRow>;
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
    if (err) setError(workTimeErrorMessage(err, entriesById));
  }

  return (
    <form className="wt-form" onSubmit={submit}>
      <fieldset className="wt-form-fields" disabled={saving}>
        <Field
          label="Początek"
          type="datetime-local"
          step={60}
          value={start}
          onChange={(e) => setStart(e.currentTarget.value)}
          required
        />
        {showEnd && (
          <Field
            label="Koniec"
            type="datetime-local"
            step={60}
            value={end}
            onChange={(e) => setEnd(e.currentTarget.value)}
            required
          />
        )}
        <div className="wt-form-actions">
          <Button type="submit" variant="secondary">
            {submitLabel}
          </Button>
          <Button variant="quiet" onClick={onCancel}>
            Anuluj
          </Button>
        </div>
      </fieldset>
      {error && (
        <Notice tone="error" compact>
          {error}
        </Notice>
      )}
    </form>
  );
}

function EntryItem(props: {
  row: EntryRow;
  day: string;
  /** The row's portion of the day, live for the running entry. */
  durationMs: number;
  busy: boolean;
  entriesById: Map<number, EntryRow>;
  onUpdate: (id: number, start: string, end: string | null) => Promise<unknown>;
  onRemove: (id: number) => Promise<unknown>;
}) {
  const { row, day, durationMs, busy, entriesById, onUpdate, onRemove } = props;
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
      <li className="wt-row is-editing">
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

  const start = localLabel(row.startLocal, day);
  const running = row.endLocal === null;
  const end = row.endLocal === null ? "biegnie" : localLabel(row.endLocal, day);
  const range = `${start} – ${end}`;

  return (
    <li className="wt-row">
      <span className="wt-range">
        {start} – <span className={running ? "wt-running" : undefined}>{end}</span>
      </span>
      {mode === "confirm" ? (
        <span className="wt-row-actions">
          <span className="body">Usunąć wpis?</span>
          <Button size="sm" disabled={busy} onClick={() => void confirmRemove()}>
            Tak
          </Button>
          <Button variant="quiet" size="sm" disabled={busy} onClick={() => setMode("view")}>
            Anuluj
          </Button>
        </span>
      ) : (
        <>
          <span className="wt-duration">{formatDuration(durationMs)}</span>
          <span className="wt-row-actions">
            <Button
              variant="quiet"
              size="sm"
              iconOnly
              icon="edit"
              aria-label={`Popraw wpis ${range}`}
              disabled={busy}
              onClick={() => setMode("edit")}
            />
            <Button
              variant="quiet"
              size="sm"
              iconOnly
              icon="close"
              aria-label={`Usuń wpis ${range}`}
              disabled={busy}
              onClick={() => setMode("confirm")}
            />
          </span>
        </>
      )}
      {removeError && (
        <Notice tone="error" compact className="wt-row-error">
          {removeError}
        </Notice>
      )}
    </li>
  );
}

function LoadedEntries(props: {
  data: DayEntries;
  day: string;
  nowMs: number;
  busyIds: Set<number>;
  add: (start: string, end: string) => Promise<unknown>;
  update: (id: number, start: string, end: string | null) => Promise<unknown>;
  remove: (id: number) => Promise<unknown>;
}) {
  const { data, day, nowMs, busyIds, add, update, remove } = props;
  // The add form belongs to the day it was opened on.
  const [addingDay, setAddingDay] = useState<string | null>(null);
  const adding = addingDay === day;
  const entriesById = new Map(data.entries.map((r) => [r.id, r]));
  const running = liveSpanMs(data.runningFromMs, data.dayEndMs, nowMs);

  return (
    <>
      {data.entries.length === 0 ? (
        <p className="dn-empty">Brak wpisów w tym dniu</p>
      ) : (
        <ul className="wt-entries">
          {data.entries.map((row) => (
            <EntryItem
              key={row.id}
              row={row}
              day={day}
              durationMs={
                row.endMs === null && data.runningFromMs !== null ? running : row.dayPortionMs
              }
              busy={busyIds.has(row.id)}
              entriesById={entriesById}
              onUpdate={update}
              onRemove={remove}
            />
          ))}
        </ul>
      )}
      <div className="wt-new">
        {adding ? (
          <EntryForm
            key={day}
            initialStart={`${day}T08:00`}
            initialEnd={`${day}T09:00`}
            showEnd
            submitLabel="Dopisz"
            entriesById={entriesById}
            onSave={async (start, end) => {
              const err = await add(start, end ?? "");
              if (!err) setAddingDay(null);
              return err;
            }}
            onCancel={() => setAddingDay(null)}
          />
        ) : (
          <Button variant="dashed" icon="plus" className="wt-add" onClick={() => setAddingDay(day)}>
            Dopisz brakujący wpis
          </Button>
        )}
      </div>
    </>
  );
}

export function EntriesPanel(props: {
  day: string;
  isToday: boolean;
  entries: EntriesState;
  nowMs: number;
  busyIds: Set<number>;
  add: (start: string, end: string) => Promise<unknown>;
  update: (id: number, start: string, end: string | null) => Promise<unknown>;
  remove: (id: number) => Promise<unknown>;
  retry: () => void;
}) {
  const { day, isToday, entries, nowMs, retry } = props;
  const title = isToday ? "Dziś — wpisy" : `${shortDay(day)} — wpisy`;

  let body: React.ReactNode;
  let footer: React.ReactNode = null;
  if (entries.state === "loading") {
    body = <p className="dn-loading wt-panel-body">Ładowanie…</p>;
  } else if (entries.state === "error") {
    body = (
      <div className="wt-panel-body">
        <ErrorState
          title="Nie udało się wczytać wpisów"
          message={entries.message}
          lastSuccess={
            entries.lastOkAt === null ? undefined : `ostatnio OK ${formatClock(entries.lastOkAt)}`
          }
          onRetry={retry}
        />
      </div>
    );
  } else {
    const { data } = entries;
    const total = data.closedMs + liveSpanMs(data.runningFromMs, data.dayEndMs, nowMs);
    footer = (
      <span className="wt-total">
        <span>Razem</span>
        <strong>{formatDuration(total)}</strong>
      </span>
    );
    body = (
      <LoadedEntries
        data={data}
        day={day}
        nowMs={nowMs}
        busyIds={props.busyIds}
        add={props.add}
        update={props.update}
        remove={props.remove}
      />
    );
  }

  return (
    <Panel title={title} meta={dayMeta(day)} footer={footer}>
      {body}
    </Panel>
  );
}
