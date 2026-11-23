import { useEffect, useState } from "react";
import { ErrorState } from "../components/ErrorState";
import { openGitlabItem } from "../gitlab/api";
import type { GitlabItem } from "../gitlab/types";
import { useGitlab } from "../gitlab/useGitlab";
import { listOwnTasks, ownTaskErrorMessage } from "../own-tasks/api";
import type { OwnTask, Space } from "../own-tasks/types";
import { todoErrorMessage } from "./api";
import type { TodoItem, TodoList } from "./types";
import { useTodo } from "./useTodo";

const SPACE_LABEL: Record<Space, string> = { work: "Praca", private: "Prywatne" };

function lastRefreshLabel(at: Date | null): string {
  return `Ostatnie udane odświeżenie: ${at ? at.toLocaleTimeString("pl-PL") : "brak"}`;
}

function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
}

/** `YYYY-MM-DD` to `DD.MM`. */
function dayMonth(day: string): string {
  const [, month, date] = day.split("-");
  return `${date}.${month}`;
}

function gitlabKey(kind: string, reference: string): string {
  return `${kind}:${reference}`;
}

type OwnLoad =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "loaded"; tasks: OwnTask[] };

export default function TodoView() {
  const { load, lastSuccessAt, reloadError, retry, refresh, addOwn, addGitlab, setTicked, remove } =
    useTodo();
  // The picker only reads the stored GitLab snapshot: a visit to the default
  // tab must not start a `glab` round.
  const { snapshot, loadError: gitlabLoadError } = useGitlab({ refreshOnMount: false });
  const [ownLoad, setOwnLoad] = useState<OwnLoad>({ state: "loading" });
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Reload the candidates whenever the list was refreshed, so a task done in
  // "Moje zadania" or a fresh pick never leaves a stale candidate.
  useEffect(() => {
    let cancelled = false;
    listOwnTasks().then(
      (tasks) => {
        if (!cancelled) setOwnLoad({ state: "loaded", tasks });
      },
      (e: unknown) => {
        if (!cancelled) setOwnLoad({ state: "error", message: ownTaskErrorMessage(e) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [lastSuccessAt]);

  async function act(run: () => Promise<string | null>) {
    setNotice(null);
    setBusy(true);
    const result = await run();
    if (result !== null) setNotice(result);
    setBusy(false);
  }

  if (load.state === "loading") {
    return (
      <main className="container">
        <h1>Do zrobienia</h1>
        <p>Ładowanie…</p>
      </main>
    );
  }

  if (load.state === "error") {
    return (
      <main className="container">
        <h1>Do zrobienia</h1>
        <ErrorState
          title="Nie udało się wczytać listy"
          message={load.message}
          onRetry={retry}
          lastSuccess={lastRefreshLabel(lastSuccessAt)}
        />
      </main>
    );
  }

  const { list } = load;
  const open = list.items.filter((i) => !i.ticked);
  const done = list.items.filter((i) => i.ticked);
  const atLimit = list.limit === null || list.openCount >= list.limit;

  const onList = new Set<string>();
  for (const item of list.items) {
    onList.add(
      item.entry.kind === "own"
        ? `own:${item.entry.taskId}`
        : gitlabKey(item.entry.itemKind, item.entry.reference),
    );
  }

  const gitlabState = snapshot?.state.state ?? null;
  const unverified =
    gitlabState !== "ok" && list.items.some((i) => !i.ticked && i.entry.kind === "gitlab");

  function renderRow(item: TodoItem) {
    const { entry } = item;
    const title = entry.title;
    const url = entry.kind === "gitlab" ? entry.webUrl : null;
    return (
      <li key={item.id} className={item.ticked ? "todo-row is-done" : "todo-row"}>
        <input
          type="checkbox"
          checked={item.ticked}
          disabled={busy}
          onChange={() => void act(() => setTicked(item.id, !item.ticked))}
          aria-label={item.ticked ? `Odznacz: ${title}` : `Zrobione: ${title}`}
        />
        <div className="todo-main">
          <span className="todo-title">
            {url ? (
              <button
                type="button"
                className="link-button"
                onClick={() => void openGitlabItem(url)}
              >
                {title}
              </button>
            ) : (
              title
            )}
          </span>
          <span className="todo-meta">
            {entry.kind === "own" ? (
              <span className={`own-task-badge ${entry.space}`}>{SPACE_LABEL[entry.space]}</span>
            ) : (
              <span className="badge todo-source">
                {entry.itemKind === "mergeRequest" ? "MR" : "Zgłoszenie"} {entry.reference}
              </span>
            )}
            {item.carriedOver && <span className="badge">od {dayMonth(item.pickedOn)}</span>}
            {entry.kind === "gitlab" && entry.closedInSource && (
              <span className="badge todo-closed">zamknięte w GitLabie</span>
            )}
          </span>
        </div>
        <button
          type="button"
          disabled={busy}
          aria-label={`Zdejmij: ${title}`}
          onClick={() => void act(() => remove(item.id))}
        >
          Zdejmij
        </button>
      </li>
    );
  }

  return (
    <main className="container">
      <h1>Do zrobienia</h1>

      <p className="todo-counter" role="status">
        {list.limit !== null
          ? `Otwarte ${list.openCount}/${list.limit}`
          : `Otwarte ${list.openCount}`}
      </p>
      {list.limitError && (
        <p className="own-task-error" role="alert">
          {todoErrorMessage(list.limitError)}
        </p>
      )}

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

      {unverified && (
        <p className="gitlab-notice" role="status">
          GitLab niedostępny — stan pozycji z GitLaba niezweryfikowany (ostatnie odświeżenie{" "}
          {snapshot?.lastSuccessMs != null ? formatClock(snapshot.lastSuccessMs) : "brak"})
        </p>
      )}

      {open.length === 0 && done.length === 0 && (
        <p className="empty">Nic na dziś — dodaj z list poniżej</p>
      )}
      {open.length > 0 && <ul className="todo-list">{open.map(renderRow)}</ul>}

      {done.length > 0 && (
        <>
          <h2>Zrobione dziś</h2>
          <ul className="todo-list">{done.map(renderRow)}</ul>
        </>
      )}

      <Picker
        list={list}
        onList={onList}
        atLimit={atLimit}
        busy={busy}
        ownLoad={ownLoad}
        gitlabItems={
          snapshot?.items ? [...snapshot.items.mergeRequests, ...snapshot.items.issues] : []
        }
        gitlabStatus={
          gitlabLoadError
            ? gitlabLoadError
            : !snapshot || gitlabState === "starting"
              ? "Pobieram przypisane z GitLaba…"
              : gitlabState === "notConfigured"
                ? "GitLab nie jest skonfigurowany."
                : gitlabState === "unavailable"
                  ? "GitLab niedostępny (brak VPN lub sieci) — lista może być nieaktualna."
                  : gitlabState === "error"
                    ? "Błąd GitLaba — lista może być nieaktualna."
                    : null
        }
        onAddOwn={(id) => void act(() => addOwn(id))}
        onAddGitlab={(item) => void act(() => addGitlab(item.kind, item.reference))}
      />
    </main>
  );
}

function Picker({
  list,
  onList,
  atLimit,
  busy,
  ownLoad,
  gitlabItems,
  gitlabStatus,
  onAddOwn,
  onAddGitlab,
}: {
  list: TodoList;
  onList: Set<string>;
  atLimit: boolean;
  busy: boolean;
  ownLoad: OwnLoad;
  gitlabItems: GitlabItem[];
  gitlabStatus: string | null;
  onAddOwn: (taskId: number) => void;
  onAddGitlab: (item: GitlabItem) => void;
}) {
  const gitlabCandidates = gitlabItems.filter((i) => !onList.has(gitlabKey(i.kind, i.reference)));
  const ownCandidates =
    ownLoad.state === "loaded"
      ? ownLoad.tasks.filter((t) => t.doneAt === null && !onList.has(`own:${t.id}`))
      : [];
  const disabled = busy || atLimit;
  const disabledHint =
    list.limit !== null && atLimit ? `Limit ${list.limit} otwartych pozycji` : undefined;

  return (
    <section className="todo-picker">
      <h2>Dodaj z list</h2>

      <h3>GitLab</h3>
      {gitlabStatus && (
        <p className="gitlab-notice" role="status">
          {gitlabStatus}
        </p>
      )}
      {gitlabCandidates.length === 0 ? (
        gitlabItems.length > 0 && <p className="empty">Wszystko z GitLaba jest już na liście.</p>
      ) : (
        <ul className="todo-list">
          {gitlabCandidates.map((item) => (
            <li key={gitlabKey(item.kind, item.reference)} className="todo-row">
              <div className="todo-main">
                <span className="todo-title">{item.title}</span>
                <span className="todo-meta">
                  <span className="badge todo-source">
                    {item.kind === "mergeRequest" ? "MR" : "Zgłoszenie"} {item.reference}
                  </span>
                </span>
              </div>
              <button
                type="button"
                disabled={disabled}
                title={disabledHint}
                aria-label={`Dodaj: ${item.title}`}
                onClick={() => onAddGitlab(item)}
              >
                Dodaj
              </button>
            </li>
          ))}
        </ul>
      )}

      <h3>Moje zadania</h3>
      {ownLoad.state === "loading" && <p>Ładowanie…</p>}
      {ownLoad.state === "error" && (
        <ErrorState title="Nie udało się wczytać zadań" message={ownLoad.message} />
      )}
      {ownLoad.state === "loaded" &&
        (ownCandidates.length === 0 ? (
          <p className="empty">Brak otwartych zadań do dodania.</p>
        ) : (
          <ul className="todo-list">
            {ownCandidates.map((task) => (
              <li key={task.id} className="todo-row">
                <div className="todo-main">
                  <span className="todo-title">{task.title}</span>
                  <span className="todo-meta">
                    <span className={`own-task-badge ${task.space}`}>
                      {SPACE_LABEL[task.space]}
                    </span>
                  </span>
                </div>
                <button
                  type="button"
                  disabled={disabled}
                  title={disabledHint}
                  aria-label={`Dodaj: ${task.title}`}
                  onClick={() => onAddOwn(task.id)}
                >
                  Dodaj
                </button>
              </li>
            ))}
          </ul>
        ))}
    </section>
  );
}
