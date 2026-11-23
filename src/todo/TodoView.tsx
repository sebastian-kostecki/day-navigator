import { useEffect, useState } from "react";
import { ErrorState } from "../components/ErrorState";
import { openGitlabItem } from "../gitlab/api";
import { openGithubIssue } from "../github/api";
import type { GithubIssue } from "../github/types";
import { useGithub } from "../github/useGithub";
import type { GitlabItem } from "../gitlab/types";
import { useGitlab } from "../gitlab/useGitlab";
import { listOwnTasks, ownTaskErrorMessage } from "../own-tasks/api";
import type { OwnTask, Space } from "../own-tasks/types";
import { todoErrorMessage } from "./api";
import type { TodoEntry, TodoItem, TodoList } from "./types";
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

// Picker keys carry the source, so a GitHub `owner/repo#N` can never collide
// with a GitLab reference.
function gitlabKey(kind: string, reference: string): string {
  return `gitlab:${kind}:${reference}`;
}

function githubKey(reference: string): string {
  return `github:${reference}`;
}

function entryKey(entry: TodoEntry): string {
  switch (entry.kind) {
    case "own":
      return `own:${entry.taskId}`;
    case "gitlab":
      return gitlabKey(entry.itemKind, entry.reference);
    case "github":
      return githubKey(entry.reference);
  }
}

/** `owner/name` part of a `owner/name#N` reference. */
function repoOf(reference: string): string {
  const at = reference.lastIndexOf("#");
  return (at < 0 ? reference : reference.slice(0, at)).toLowerCase();
}

type OwnLoad =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "loaded"; tasks: OwnTask[] };

export default function TodoView() {
  const {
    load,
    lastSuccessAt,
    reloadError,
    retry,
    refresh,
    addOwn,
    addGitlab,
    addGithub,
    setTicked,
    remove,
  } = useTodo();
  // The picker only reads the stored GitLab and GitHub snapshots: a visit to
  // the default tab must not start a `glab` or `gh` round.
  const {
    snapshot,
    loadError: gitlabLoadError,
    retry: retryGitlab,
  } = useGitlab({ refreshOnMount: false });
  const {
    snapshot: githubSnapshot,
    loadError: githubLoadError,
    retry: retryGithub,
  } = useGithub({ refreshOnMount: false });
  const [ownLoad, setOwnLoad] = useState<OwnLoad>({ state: "loading" });
  const [ownLoadedAt, setOwnLoadedAt] = useState<Date | null>(null);
  // Bumped by the own-task error's retry button.
  const [ownAttempt, setOwnAttempt] = useState(0);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Reload the candidates whenever the list was refreshed, so a task done in
  // "Moje zadania" or a fresh pick never leaves a stale candidate.
  useEffect(() => {
    let cancelled = false;
    listOwnTasks().then(
      (tasks) => {
        if (cancelled) return;
        setOwnLoad({ state: "loaded", tasks });
        setOwnLoadedAt(new Date());
      },
      (e: unknown) => {
        if (!cancelled) setOwnLoad({ state: "error", message: ownTaskErrorMessage(e) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [lastSuccessAt, ownAttempt]);

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
    onList.add(entryKey(item.entry));
  }

  const gitlabState = snapshot?.state.state ?? null;
  // Only a round that ended without a list makes the stored GitLab entries
  // unverified. While the first round is still running (no snapshot yet, or
  // "starting") nothing is known, so nothing is claimed.
  const gitlabDown =
    gitlabLoadError !== null ||
    gitlabState === "unavailable" ||
    gitlabState === "error" ||
    gitlabState === "notConfigured";
  const unverified = gitlabDown && list.items.some((i) => !i.ticked && i.entry.kind === "gitlab");

  const githubState = githubSnapshot?.state.state ?? null;
  const githubDown =
    githubLoadError !== null ||
    githubState === "unavailable" ||
    githubState === "error" ||
    githubState === "notConfigured";
  // An unticked GitHub entry is also unverified when its repo did not load in
  // the last successful round (it has an error, or it is no longer in
  // settings): reconcile never ticks such an entry.
  const githubUnverified = list.items.some((i) => {
    if (i.ticked || i.entry.kind !== "github") return false;
    if (githubDown) return true;
    if (githubState !== "ok") return false;
    const repo = repoOf(i.entry.reference);
    const view = githubSnapshot?.repos.find((r) => r.repo.toLowerCase() === repo);
    return !view || view.error !== null;
  });

  function renderRow(item: TodoItem) {
    const { entry } = item;
    const title = entry.title;
    const url = entry.kind === "own" ? null : entry.webUrl;
    const openUrl = entry.kind === "github" ? openGithubIssue : openGitlabItem;
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
              <button type="button" className="link-button" onClick={() => void openUrl(url)}>
                {title}
              </button>
            ) : (
              title
            )}
          </span>
          <span className="todo-meta">
            {entry.kind === "own" ? (
              <span className={`own-task-badge ${entry.space}`}>{SPACE_LABEL[entry.space]}</span>
            ) : entry.kind === "gitlab" ? (
              <span className="badge todo-source">
                {entry.itemKind === "mergeRequest" ? "MR" : "Zgłoszenie"} {entry.reference}
              </span>
            ) : (
              <span className="badge todo-source">Zgłoszenie GitHub {entry.reference}</span>
            )}
            {item.carriedOver && <span className="badge">od {dayMonth(item.pickedOn)}</span>}
            {entry.kind === "gitlab" && entry.closedInSource && (
              <span className="badge todo-closed">zamknięte w GitLabie</span>
            )}
            {entry.kind === "github" && entry.closedInSource && (
              <span className="badge todo-closed">zamknięte w GitHubie</span>
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

      {githubUnverified && (
        <p className="gitlab-notice" role="status">
          GitHub niedostępny lub repozytorium niezaładowane — stan pozycji z GitHuba
          niezweryfikowany (ostatnie odświeżenie{" "}
          {githubSnapshot?.lastSuccessMs != null
            ? formatClock(githubSnapshot.lastSuccessMs)
            : "brak"}
          )
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
        gitlabOk={gitlabState === "ok"}
        gitlabFilteredOut={
          snapshot?.filterActive
            ? snapshot.filteredOut.mergeRequests + snapshot.filteredOut.issues
            : null
        }
        gitlabLoadError={gitlabLoadError}
        gitlabLastSuccess={
          snapshot?.lastSuccessMs != null
            ? `Ostatnie udane odświeżenie: ${formatClock(snapshot.lastSuccessMs)}`
            : lastRefreshLabel(null)
        }
        onRetryGitlab={retryGitlab}
        ownLastSuccess={lastRefreshLabel(ownLoadedAt)}
        onRetryOwn={() => setOwnAttempt((n) => n + 1)}
        gitlabStatus={
          gitlabLoadError
            ? null
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
        githubIssues={
          githubSnapshot
            ? githubSnapshot.repos.filter((r) => !r.error).flatMap((r) => r.issues)
            : []
        }
        githubOk={githubState === "ok"}
        githubRepoErrors={
          githubState === "ok" && githubSnapshot
            ? repoErrorSpread(githubSnapshot.repos.map((r) => r.error !== null))
            : "none"
        }
        githubLoadError={githubLoadError}
        githubLastSuccess={
          githubSnapshot?.lastSuccessMs != null
            ? `Ostatnie udane odświeżenie: ${formatClock(githubSnapshot.lastSuccessMs)}`
            : lastRefreshLabel(null)
        }
        onRetryGithub={retryGithub}
        githubStatus={
          githubLoadError
            ? null
            : !githubSnapshot || githubState === "starting"
              ? "Pobieram przypisane z GitHuba…"
              : githubState === "notConfigured"
                ? "GitHub nie jest skonfigurowany."
                : githubState === "unavailable"
                  ? "GitHub niedostępny (brak sieci) — lista może być nieaktualna."
                  : githubState === "error"
                    ? "Błąd GitHuba — lista może być nieaktualna."
                    : null
        }
        onAddOwn={(id) => void act(() => addOwn(id))}
        onAddGitlab={(item) => void act(() => addGitlab(item.kind, item.reference))}
        onAddGithub={(issue) => void act(() => addGithub(issue.reference))}
      />
    </main>
  );
}

/** How many GitHub repos failed in an Ok round: none, some or every one. */
type RepoErrorSpread = "none" | "some" | "all";

function repoErrorSpread(failed: boolean[]): RepoErrorSpread {
  const n = failed.filter(Boolean).length;
  return n === 0 ? "none" : n === failed.length ? "all" : "some";
}

function Picker({
  list,
  onList,
  atLimit,
  busy,
  ownLoad,
  gitlabItems,
  gitlabOk,
  gitlabFilteredOut,
  gitlabLoadError,
  gitlabLastSuccess,
  onRetryGitlab,
  ownLastSuccess,
  onRetryOwn,
  gitlabStatus,
  githubIssues,
  githubOk,
  githubRepoErrors,
  githubLoadError,
  githubLastSuccess,
  onRetryGithub,
  githubStatus,
  onAddOwn,
  onAddGitlab,
  onAddGithub,
}: {
  list: TodoList;
  onList: Set<string>;
  atLimit: boolean;
  busy: boolean;
  ownLoad: OwnLoad;
  gitlabItems: GitlabItem[];
  gitlabOk: boolean;
  /** Items the filter removed from the GitLab list; null when no filter is active. */
  gitlabFilteredOut: number | null;
  gitlabLoadError: string | null;
  gitlabLastSuccess: string;
  onRetryGitlab: () => void;
  ownLastSuccess: string;
  onRetryOwn: () => void;
  gitlabStatus: string | null;
  githubIssues: GithubIssue[];
  githubOk: boolean;
  githubRepoErrors: RepoErrorSpread;
  githubLoadError: string | null;
  githubLastSuccess: string;
  onRetryGithub: () => void;
  githubStatus: string | null;
  onAddOwn: (taskId: number) => void;
  onAddGitlab: (item: GitlabItem) => void;
  onAddGithub: (issue: GithubIssue) => void;
}) {
  const gitlabCandidates = gitlabItems.filter((i) => !onList.has(gitlabKey(i.kind, i.reference)));
  const githubCandidates = githubIssues.filter((i) => !onList.has(githubKey(i.reference)));
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
      {gitlabLoadError && (
        <ErrorState
          title="Nie udało się wczytać listy z GitLaba"
          message={gitlabLoadError}
          onRetry={onRetryGitlab}
          lastSuccess={gitlabLastSuccess}
        />
      )}
      {gitlabStatus && (
        <p className="gitlab-notice" role="status">
          {gitlabStatus}
        </p>
      )}
      {gitlabCandidates.length === 0 ? (
        gitlabItems.length > 0 ? (
          <p className="empty">Wszystko z GitLaba jest już na liście.</p>
        ) : (
          gitlabOk && (
            <p className="empty">
              {gitlabFilteredOut !== null && gitlabFilteredOut > 0
                ? `Brak pasujących do filtra (ukryte filtrem: ${gitlabFilteredOut})`
                : "Brak przypisanych w GitLabie."}
            </p>
          )
        )
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

      <h3>GitHub</h3>
      {githubLoadError && (
        <ErrorState
          title="Nie udało się wczytać listy z GitHuba"
          message={githubLoadError}
          onRetry={onRetryGithub}
          lastSuccess={githubLastSuccess}
        />
      )}
      {githubStatus && (
        <p className="gitlab-notice" role="status">
          {githubStatus}
        </p>
      )}
      {githubRepoErrors === "some" && (
        <p className="gitlab-notice" role="status">
          Część repozytoriów GitHub nie załadowała się — ich zgłoszeń nie ma na liście.
        </p>
      )}
      {githubRepoErrors === "all" && (
        <p className="gitlab-notice" role="status">
          Żadne repozytorium GitHub nie załadowało się — nie ma czego dodać.
        </p>
      )}
      {githubCandidates.length === 0 ? (
        githubIssues.length > 0 ? (
          <p className="empty">Wszystko z GitHuba jest już na liście.</p>
        ) : githubOk && githubRepoErrors === "none" ? (
          <p className="empty">Brak przypisanych w GitHubie.</p>
        ) : (
          githubOk &&
          githubRepoErrors === "some" && (
            <p className="empty">Brak przypisanych w repozytoriach, które się załadowały.</p>
          )
        )
      ) : (
        <ul className="todo-list">
          {githubCandidates.map((issue) => (
            <li key={githubKey(issue.reference)} className="todo-row">
              <div className="todo-main">
                <span className="todo-title">{issue.title}</span>
                <span className="todo-meta">
                  <span className="badge todo-source">Zgłoszenie GitHub {issue.reference}</span>
                </span>
              </div>
              <button
                type="button"
                disabled={disabled}
                title={disabledHint}
                aria-label={`Dodaj: ${issue.title}`}
                onClick={() => onAddGithub(issue)}
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
        <ErrorState
          title="Nie udało się wczytać zadań"
          message={ownLoad.message}
          onRetry={onRetryOwn}
          lastSuccess={ownLastSuccess}
        />
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
