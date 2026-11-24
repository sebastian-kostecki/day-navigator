import { useEffect, useId, useState } from "react";
import { Button, ItemRow, Panel } from "../components";
import { ErrorState } from "../components/ErrorState";
import { ownTaskDefaultSpace, ownTaskErrorMessage } from "./api";
import { formatDue, PRIORITY_LABEL, SPACE_LABEL, todayIso } from "./labels";
import { OwnTaskEditPanel } from "./OwnTaskEditPanel";
import { OwnTaskForm } from "./OwnTaskForm";
import { OwnTaskTabs, type OwnTaskTab } from "./OwnTaskTabs";
import type { OwnTask, OwnTaskInput, Space } from "./types";
import { useOwnTasks } from "./useOwnTasks";
import "./OwnTasks.css";

/** `own_task_default_space` is Work exactly while the work-time counter runs. */
const SPACE_REASON: Record<Space, string> = {
  work: "Licznik biegnie, więc domyślnie Praca.",
  private: "Licznik stoi, więc domyślnie Prywatne.",
};

function lastRefreshLabel(at: Date | null): string {
  return `Ostatnie udane odświeżenie: ${at ? at.toLocaleTimeString("pl-PL") : "brak"}`;
}

export default function OwnTasksView() {
  const { load, lastSuccessAt, reloadError, retry, refresh, add, edit, setDone } = useOwnTasks();
  // `null` until the user picks a tab: the list then follows the default space.
  const [pickedTab, setPickedTab] = useState<OwnTaskTab | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<Set<number>>(new Set());
  const [defaultSpace, setDefaultSpace] = useState<Space | null>(null);
  const [defaultSpaceError, setDefaultSpaceError] = useState<string | null>(null);
  const idPrefix = useId();

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

  async function addTask(input: OwnTaskInput) {
    const result = await add(input);
    // Show the list the new task landed in.
    if (result === null) setPickedTab(input.space);
    return result;
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
  const tab: OwnTaskTab = pickedTab ?? defaultSpace ?? "work";
  const isOpenIn = (space: Space) => (t: OwnTask) => t.doneAt === null && t.space === space;
  const doneTasks = tasks.filter((t) => t.doneAt !== null);
  const tabs = [
    { value: "work" as const, label: "Praca", count: tasks.filter(isOpenIn("work")).length },
    {
      value: "private" as const,
      label: "Prywatne",
      count: tasks.filter(isOpenIn("private")).length,
    },
    { value: "done" as const, label: "Zrobione", count: doneTasks.length },
  ];
  const shown = tab === "done" ? doneTasks : tasks.filter(isOpenIn(tab));
  const selected = tasks.find((t) => t.id === selectedId) ?? null;

  let emptyMessage: string | null = null;
  if (tasks.length === 0) {
    emptyMessage = "Nie masz jeszcze żadnych zadań.";
  } else if (shown.length === 0) {
    emptyMessage =
      tab === "done" ? "Brak zrobionych zadań." : "Brak otwartych zadań w tej przestrzeni.";
  }

  const today = todayIso();
  const panelId = `${idPrefix}-panel`;

  function renderTask(task: OwnTask) {
    const isDone = task.doneAt !== null;
    const overdue = !isDone && task.due !== null && task.due < today;
    const dueToday = !isDone && task.due === today;
    const dueClass = ["own-due", overdue && "is-overdue", dueToday && "is-today"]
      .filter(Boolean)
      .join(" ");
    return (
      <ItemRow
        key={task.id}
        id={`own-${task.id}`}
        selected={task.id === selectedId}
        done={isDone}
        onToggle={() => void toggleDone(task)}
        checkboxDisabled={busy.has(task.id)}
        checkboxLabel={isDone ? `Otwórz ponownie: ${task.title}` : `Zrobione: ${task.title}`}
        title={
          <>
            <span className="own-row-title">{task.title}</span>
            {task.note && <span className="own-row-note">{task.note}</span>}
          </>
        }
        meta={
          <span className="own-row-meta">
            {task.priority && (
              <span className={`own-prio own-prio-${task.priority}`}>
                <span className="dn-sr-only">Priorytet: </span>
                {PRIORITY_LABEL[task.priority]}
              </span>
            )}
            <span className={dueClass}>
              <span className="dn-sr-only">Termin: </span>
              {task.due ? (
                formatDue(task.due, today)
              ) : (
                <>
                  <span aria-hidden="true">—</span>
                  <span className="dn-sr-only">brak</span>
                </>
              )}
              {overdue && <span className="dn-sr-only">, po terminie</span>}
            </span>
          </span>
        }
      >
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="edit"
          aria-label={`Edytuj: ${task.title}`}
          onClick={() => setSelectedId(task.id)}
        />
      </ItemRow>
    );
  }

  return (
    <main className="container">
      <h1>Moje zadania</h1>

      {defaultSpace !== null && (
        <OwnTaskForm
          initialSpace={defaultSpace}
          spaceReason={defaultSpaceError === null ? SPACE_REASON[defaultSpace] : undefined}
          onSubmit={addTask}
        />
      )}
      {defaultSpaceError && (
        <p className="own-error" role="alert">
          {defaultSpaceError}
        </p>
      )}

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

      <div className="dn-columns">
        <div className="dn-col-main own-list-col">
          <Panel>
            <OwnTaskTabs
              tabs={tabs}
              value={tab}
              onChange={setPickedTab}
              panelId={panelId}
              idPrefix={idPrefix}
            />
            <div id={panelId} role="tabpanel" aria-labelledby={`${idPrefix}-${tab}`}>
              {emptyMessage && <p className="dn-empty">{emptyMessage}</p>}
              {shown.map(renderTask)}
            </div>
          </Panel>
        </div>

        <div className="dn-col-side own-edit-col">
          <Panel
            title="Edycja"
            meta={
              selected ? <span className="own-space-tag">{SPACE_LABEL[selected.space]}</span> : null
            }
          >
            {selected ? (
              <OwnTaskEditPanel
                key={selected.id}
                task={selected}
                onSave={(input) => edit(selected.id, input)}
                onToggleDone={() => void toggleDone(selected)}
                toggleBusy={busy.has(selected.id)}
              />
            ) : (
              <p className="dn-empty">Wybierz zadanie ikoną edycji, aby je zmienić.</p>
            )}
          </Panel>
        </div>
      </div>
    </main>
  );
}
