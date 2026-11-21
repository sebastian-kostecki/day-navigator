import { useCallback, useEffect, useRef, useState } from "react";
import {
  anyErrorMessage,
  createWorkTask,
  getWorkTaskSettings,
  listWorkTasks,
  needsReload,
  setWorkTaskStatus,
  vaultErrorMessage,
  type TaskRow,
  type TaskSettingsView,
} from "./api";

type Load =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "loaded"; settings: TaskSettingsView; rows: TaskRow[] };

export default function TasksView() {
  const [load, setLoad] = useState<Load>({ state: "loading" });
  const [showDone, setShowDone] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<Set<string>>(new Set());

  const [title, setTitle] = useState("");
  const [priority, setPriority] = useState("");
  const [due, setDue] = useState("");
  const [creating, setCreating] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  // Only the latest reload may set the list: an older listing resolving late
  // would bring back stale hashes and a spurious conflict on the next edit.
  const latestLoad = useRef(0);
  const reload = useCallback(async () => {
    const id = ++latestLoad.current;
    try {
      const [settings, rows] = await Promise.all([
        getWorkTaskSettings(),
        listWorkTasks(),
      ]);
      if (id === latestLoad.current) {
        setLoad({ state: "loaded", settings, rows });
      }
    } catch (e) {
      if (id === latestLoad.current) {
        setLoad({ state: "error", message: anyErrorMessage(e) });
      }
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function changeStatus(row: TaskRow, status: string) {
    if (!row.statusHash) return;
    setNotice(null);
    setBusy((b) => new Set(b).add(row.path));
    try {
      const result = await setWorkTaskStatus(
        row.path,
        status,
        row.statusHash,
        row.doneHash,
      );
      if (result === "Absent") {
        setNotice("Notatka nie ma pola Done — zmieniono tylko status");
      }
    } catch (e) {
      setNotice(anyErrorMessage(e));
      if (!needsReload(e)) {
        setBusy((b) => {
          const next = new Set(b);
          next.delete(row.path);
          return next;
        });
        return;
      }
    }
    await reload();
    setBusy((b) => {
      const next = new Set(b);
      next.delete(row.path);
      return next;
    });
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setFormError(null);
    setCreating(true);
    try {
      await createWorkTask({
        title,
        priority: priority || null,
        due: due || null,
      });
      setTitle("");
      setPriority("");
      setDue("");
      await reload();
    } catch (err) {
      setFormError(anyErrorMessage(err));
    } finally {
      setCreating(false);
    }
  }

  if (load.state === "loading") {
    return (
      <main className="container">
        <h1>Zadania</h1>
        <p>Ładowanie…</p>
      </main>
    );
  }

  if (load.state === "error") {
    return (
      <main className="container">
        <h1>Zadania</h1>
        <div className="error-banner" role="alert">
          <p>{load.message}</p>
          <button type="button" onClick={() => void reload()}>
            Spróbuj ponownie
          </button>
        </div>
      </main>
    );
  }

  const { settings, rows } = load;
  const visible = showDone
    ? rows
    : rows.filter((r) => r.status !== settings.doneStatus);

  return (
    <main className="container">
      <h1>Zadania</h1>

      <form className="new-task" onSubmit={submit}>
        <h2>Nowe zadanie</h2>
        <fieldset disabled={creating}>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.currentTarget.value)}
            placeholder="Tytuł"
            required
          />
          {settings.priorities.length > 0 && (
            <select
              value={priority}
              onChange={(e) => setPriority(e.currentTarget.value)}
              aria-label="Priorytet"
            >
              <option value="">Priorytet</option>
              {settings.priorities.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          )}
          <input
            type="date"
            value={due}
            onChange={(e) => setDue(e.currentTarget.value)}
            aria-label="Termin"
          />
          <button type="submit">Dodaj</button>
        </fieldset>
        {formError && (
          <p className="error" role="alert">
            {formError}
          </p>
        )}
      </form>

      <label className="toggle">
        <input
          type="checkbox"
          checked={showDone}
          onChange={(e) => setShowDone(e.currentTarget.checked)}
        />
        Pokaż zakończone
      </label>

      {notice && (
        <p className="notice" role="status">
          {notice}
        </p>
      )}

      {visible.length === 0 ? (
        <p className="empty">
          {rows.length === 0
            ? `Brak zadań w folderze ${settings.folder}`
            : "Brak aktywnych zadań — zaznacz „Pokaż zakończone”, aby zobaczyć resztę"}
        </p>
      ) : (
        <ul className="task-list">
          {visible.map((row) => {
            const known =
              row.status !== null && settings.statuses.includes(row.status);
            return (
              <li key={row.path} className="task-row">
                <div className="task-main">
                  <span className="task-title">{row.title}</span>
                  <span className="task-meta">
                    {row.priority && <span>{row.priority}</span>}
                    {row.due && <span>{row.due}</span>}
                  </span>
                  {row.problem && (
                    <span className="error">
                      {vaultErrorMessage(row.problem)}
                    </span>
                  )}
                  {row.fieldProblems.map((p, i) => (
                    <span key={i} className="error">
                      {vaultErrorMessage(p)}
                    </span>
                  ))}
                </div>
                <select
                  value={row.status ?? ""}
                  disabled={
                    row.problem !== null ||
                    row.statusHash === null ||
                    busy.has(row.path)
                  }
                  onChange={(e) => void changeStatus(row, e.currentTarget.value)}
                  aria-label={`Status: ${row.title}`}
                >
                  {!known && (
                    <option value={row.status ?? ""} disabled>
                      {row.status ?? "—"}
                    </option>
                  )}
                  {settings.statuses.map((s) => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
                </select>
              </li>
            );
          })}
        </ul>
      )}
    </main>
  );
}
