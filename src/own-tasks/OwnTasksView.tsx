import { useEffect, useRef, useState } from "react";
import { Button, ItemRow, Panel, Segmented } from "../components";
import { ErrorState } from "../components/ErrorState";
import { ownTaskDefaultSpace, ownTaskErrorMessage } from "./api";
import { OwnTaskForm } from "./OwnTaskForm";
import type { OwnTask, Priority, Space } from "./types";
import { useOwnTasks } from "./useOwnTasks";
import "./OwnTasks.css";

type Filter = "all" | Space;

const FILTERS: { value: Filter; label: string }[] = [
  { value: "all", label: "Wszystkie" },
  { value: "work", label: "Praca" },
  { value: "private", label: "Prywatne" },
];

const SPACE_LABEL: Record<Space, string> = { work: "Praca", private: "Prywatne" };
const PRIORITY_LABEL: Record<Priority, string> = {
  high: "Wysoki",
  medium: "Średni",
  low: "Niski",
};

function lastRefreshLabel(at: Date | null): string {
  return `Ostatnie udane odświeżenie: ${at ? at.toLocaleTimeString("pl-PL") : "brak"}`;
}

/** Today as a local `YYYY-MM-DD`, comparable with a task's `due`. */
function todayIso(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** The note clamped to a few lines, with a toggle shown only when it overflows. */
function NotePreview({ note }: { note: string }) {
  const ref = useRef<HTMLParagraphElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [overflows, setOverflows] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el || expanded) return;
    // The observer fires once on observe and again on every resize, never
    // synchronously inside the effect.
    const observer = new ResizeObserver(() => {
      setOverflows(el.scrollHeight > el.clientHeight + 1);
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [note, expanded]);

  return (
    <div className="own-note-wrap">
      <p ref={ref} className={expanded ? "own-note is-expanded" : "own-note"}>
        {note}
      </p>
      {(overflows || expanded) && (
        <Button
          variant="quiet"
          size="sm"
          className="own-note-toggle"
          aria-expanded={expanded}
          onClick={() => setExpanded((x) => !x)}
        >
          {expanded ? "Zwiń notatkę" : "Pokaż całą notatkę"}
        </Button>
      )}
    </div>
  );
}

export default function OwnTasksView() {
  const { load, lastSuccessAt, reloadError, retry, refresh, add, edit, setDone } = useOwnTasks();
  const [filter, setFilter] = useState<Filter>("all");
  const [showDone, setShowDone] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<Set<number>>(new Set());
  const [defaultSpace, setDefaultSpace] = useState<Space | null>(null);
  const [defaultSpaceError, setDefaultSpaceError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    // State is set only after awaiting the backend, never synchronously.
    ownTaskDefaultSpace()
      .then((space) => {
        if (!cancelled) setDefaultSpace(space);
      })
      .catch((e) => {
        if (!cancelled) {
          setDefaultSpaceError(ownTaskErrorMessage(e));
          setDefaultSpace("private");
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function toggleDone(task: OwnTask) {
    setNotice(null);
    setBusy((b) => new Set(b).add(task.id));
    const result = await setDone(task.id, task.doneAt === null);
    if (result !== null) setNotice(result);
    setBusy((b) => {
      const next = new Set(b);
      next.delete(task.id);
      return next;
    });
  }

  if (load.state === "loading") {
    return (
      <main className="container">
        <h1>Moje zadania</h1>
        <p className="dn-loading">Ładowanie…</p>
      </main>
    );
  }

  if (load.state === "error") {
    return (
      <main className="container">
        <h1>Moje zadania</h1>
        <ErrorState
          title="Nie udało się wczytać zadań"
          message={load.message}
          onRetry={() => void retry()}
          lastSuccess={lastRefreshLabel(lastSuccessAt)}
        />
      </main>
    );
  }

  const { tasks } = load;
  const inFilter = tasks.filter((t) => filter === "all" || t.space === filter);
  const open = inFilter.filter((t) => t.doneAt === null);
  const done = inFilter.filter((t) => t.doneAt !== null);

  let emptyMessage: string | null = null;
  if (tasks.length === 0) {
    emptyMessage = "Nie masz jeszcze żadnych zadań.";
  } else if (open.length === 0) {
    emptyMessage =
      filter === "all" ? "Brak otwartych zadań." : "Brak otwartych zadań w tej przestrzeni.";
  }

  const today = todayIso();

  function renderTask(task: OwnTask) {
    if (editingId === task.id) {
      return (
        <div key={task.id} className="own-item is-editing">
          <OwnTaskForm
            heading="Edycja zadania"
            initial={{
              space: task.space,
              title: task.title,
              note: task.note,
              due: task.due,
              priority: task.priority,
            }}
            submitLabel="Zapisz"
            onSubmit={async (input) => {
              const result = await edit(task.id, input);
              if (result === null) setEditingId(null);
              return result;
            }}
            onCancel={() => setEditingId(null)}
          />
        </div>
      );
    }
    const isDone = task.doneAt !== null;
    const overdue = !isDone && task.due !== null && task.due < today;
    const dueToday = !isDone && task.due === today;
    return (
      <div key={task.id} className="own-item">
        <ItemRow
          id={`own-${task.id}`}
          done={isDone}
          onToggle={() => void toggleDone(task)}
          checkboxDisabled={busy.has(task.id)}
          checkboxLabel={isDone ? `Otwórz ponownie: ${task.title}` : `Zrobione: ${task.title}`}
          source="my"
          title={task.title}
          flag={dueToday ? "dziś" : undefined}
          flagTone="change"
          meta={
            <span className="own-meta">
              <span>{SPACE_LABEL[task.space]}</span>
              {task.due && (
                <span className={overdue ? "own-overdue" : undefined}>Termin: {task.due}</span>
              )}
              {task.priority && <span>Priorytet: {PRIORITY_LABEL[task.priority]}</span>}
            </span>
          }
        >
          <Button
            variant="quiet"
            size="sm"
            iconOnly
            icon="edit"
            aria-label={`Edytuj: ${task.title}`}
            onClick={() => setEditingId(task.id)}
          />
        </ItemRow>
        {task.note && <NotePreview note={task.note} />}
      </div>
    );
  }

  return (
    <main className="container">
      <h1>Moje zadania</h1>

      {defaultSpace !== null && (
        <div className="own-form-card">
          <OwnTaskForm
            heading="Dodaj zadanie"
            initial={{ space: defaultSpace, title: "", note: "", due: null, priority: null }}
            submitLabel="Dodaj"
            onSubmit={add}
            resetOnSuccess
          />
        </div>
      )}
      {defaultSpaceError && (
        <p className="own-error" role="alert">
          {defaultSpaceError}
        </p>
      )}

      <div className="own-controls">
        <Segmented
          label="Filtr przestrzeni"
          options={FILTERS}
          value={filter}
          onChange={(value) => setFilter(value as Filter)}
        />
        <label className="own-show-done">
          <input
            type="checkbox"
            checked={showDone}
            onChange={(e) => setShowDone(e.currentTarget.checked)}
          />
          Pokaż zrobione
        </label>
      </div>

      {notice && (
        <p className="own-error" role="alert">
          {notice}
        </p>
      )}

      {reloadError && (
        <ErrorState
          title="Nie udało się odświeżyć listy — może być nieaktualna"
          message={reloadError}
          onRetry={() => void refresh()}
          lastSuccess={lastRefreshLabel(lastSuccessAt)}
        />
      )}

      <Panel title="Otwarte" meta={open.length}>
        {emptyMessage && <p className="dn-empty">{emptyMessage}</p>}
        {open.map(renderTask)}
      </Panel>

      {/* With no tasks at all, the single "no tasks yet" message says it all. */}
      {showDone && tasks.length > 0 && (
        <Panel title="Zrobione" meta={done.length}>
          {done.length === 0 ? (
            <p className="dn-empty">Brak zrobionych zadań.</p>
          ) : (
            done.map(renderTask)
          )}
        </Panel>
      )}
    </main>
  );
}
