import { useEffect, useRef, useState } from "react";
import { ErrorState } from "../components/ErrorState";
import { ownTaskDefaultSpace, ownTaskErrorMessage } from "./api";
import { OwnTaskForm } from "./OwnTaskForm";
import type { OwnTask, Priority, Space } from "./types";
import { useOwnTasks } from "./useOwnTasks";

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
    <>
      <p ref={ref} className={expanded ? "own-task-note expanded" : "own-task-note"}>
        {note}
      </p>
      {(overflows || expanded) && (
        <button
          type="button"
          className="link-button own-task-note-toggle"
          aria-expanded={expanded}
          onClick={() => setExpanded((x) => !x)}
        >
          {expanded ? "Zwiń notatkę" : "Pokaż całą notatkę"}
        </button>
      )}
    </>
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
        <p>Ładowanie…</p>
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

  function renderTask(task: OwnTask) {
    if (editingId === task.id) {
      return (
        <li key={task.id} className="own-task-row editing">
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
        </li>
      );
    }
    const isDone = task.doneAt !== null;
    return (
      <li key={task.id} className={isDone ? "own-task-row is-done" : "own-task-row"}>
        <input
          type="checkbox"
          checked={isDone}
          disabled={busy.has(task.id)}
          onChange={() => void toggleDone(task)}
          aria-label={isDone ? `Otwórz ponownie: ${task.title}` : `Zrobione: ${task.title}`}
        />
        <div className="own-task-main">
          <span className="own-task-title">{task.title}</span>
          <span className="own-task-meta">
            <span className={`own-task-badge ${task.space}`}>{SPACE_LABEL[task.space]}</span>
            {task.due && <span>Termin: {task.due}</span>}
            {task.priority && <span>Priorytet: {PRIORITY_LABEL[task.priority]}</span>}
          </span>
          {task.note && <NotePreview note={task.note} />}
        </div>
        <button
          type="button"
          aria-label={`Edytuj: ${task.title}`}
          onClick={() => setEditingId(task.id)}
        >
          Edytuj
        </button>
      </li>
    );
  }

  return (
    <main className="container">
      <h1>Moje zadania</h1>

      {defaultSpace !== null && (
        <OwnTaskForm
          heading="Dodaj zadanie"
          initial={{ space: defaultSpace, title: "", note: "", due: null, priority: null }}
          submitLabel="Dodaj"
          onSubmit={add}
          resetOnSuccess
        />
      )}
      {defaultSpaceError && (
        <p className="own-task-error" role="alert">
          {defaultSpaceError}
        </p>
      )}

      <div className="own-task-filter" role="group" aria-label="Filtr przestrzeni">
        {FILTERS.map((f) => (
          <button
            key={f.value}
            type="button"
            className={filter === f.value ? "active" : undefined}
            aria-pressed={filter === f.value}
            onClick={() => setFilter(f.value)}
          >
            {f.label}
          </button>
        ))}
      </div>

      <label className="toggle">
        <input
          type="checkbox"
          checked={showDone}
          onChange={(e) => setShowDone(e.currentTarget.checked)}
        />
        Pokaż zrobione
      </label>

      {notice && (
        <p className="own-task-error" role="alert">
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

      {emptyMessage && <p className="empty">{emptyMessage}</p>}
      {open.length > 0 && <ul className="own-task-list">{open.map(renderTask)}</ul>}

      {/* With no tasks at all, the single "no tasks yet" message says it all. */}
      {showDone && tasks.length > 0 && (
        <>
          <h2>Zrobione</h2>
          {done.length === 0 ? (
            <p className="empty">Brak zrobionych zadań.</p>
          ) : (
            <ul className="own-task-list">{done.map(renderTask)}</ul>
          )}
        </>
      )}
    </main>
  );
}
