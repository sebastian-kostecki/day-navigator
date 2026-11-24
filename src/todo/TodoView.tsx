import { useEffect, useState, type ReactNode } from "react";
import {
  Button,
  Flag,
  ItemRow,
  Notice,
  Panel,
  SourceState,
  StatStrip,
  type FlagProps,
  type SourceKind,
} from "../components";
import { ErrorState } from "../components/ErrorState";
import { openGitlabItem } from "../gitlab/api";
import { openGithubIssue } from "../github/api";
import type { GithubIssue } from "../github/types";
import { useGithub } from "../github/useGithub";
import type { GitlabItem } from "../gitlab/types";
import { useGitlab } from "../gitlab/useGitlab";
import { useHealth } from "../health/useHealth";
import { listOwnTasks, ownTaskErrorMessage } from "../own-tasks/api";
import type { OwnTask, Space } from "../own-tasks/types";
import { useMonthReport } from "../worktime/useMonthReport";
import { useWorkTimer } from "../worktime/useWorkTimer";
import { todoErrorMessage } from "./api";
import { entryKey, githubKey, gitlabKey } from "./keys";
import { SourcesPanel } from "./SourcesPanel";
import { plural, summaryStats } from "./summary";
import { TodoAddButton } from "./TodoAddButton";
import type { TodoItem, TodoList } from "./types";
import { useTodo } from "./useTodo";
import "./TodoView.css";

/** Lowercase, for the ref column of an own task. */
const SPACE_REF: Record<Space, string> = { work: "praca", private: "prywatne" };

/** "z <dnia>" per `Date.getDay()`; the Flag shows it uppercase. */
const FROM_WEEKDAY = [
  "z niedzieli",
  "z poniedziałku",
  "z wtorku",
  "ze środy",
  "z czwartku",
  "z piątku",
  "z soboty",
];

const DAY_MS = 86_400_000;

function lastRefreshLabel(at: Date | number | null): string {
  if (at === null) return "Ostatnie udane odświeżenie: brak";
  return `Ostatnie udane odświeżenie: ${new Date(at).toLocaleTimeString("pl-PL")}`;
}

function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
}

/** The GitLab/GitHub variant: `HH:MM` of the source's last successful round. */
function clockLabel(ms: number | null): string {
  return ms === null ? lastRefreshLabel(null) : `Ostatnie udane odświeżenie: ${formatClock(ms)}`;
}

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

/** Local `YYYY-MM-DD` of an instant. */
function localDay(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

/** Local midnight of a `YYYY-MM-DD`. */
function dayStart(day: string): Date {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** `YYYY-MM-DD` to `DD.MM`. */
function dayMonth(day: string): string {
  const [, month, date] = day.split("-");
  return `${date}.${month}`;
}

/** Carry-over flag: the weekday it was picked on within the last 6 days, else its date. */
function carriedLabel(pickedOn: string, today: string): string {
  const from = dayStart(pickedOn);
  const days = Math.round((dayStart(today).getTime() - from.getTime()) / DAY_MS);
  return days >= 1 && days <= 6 ? FROM_WEEKDAY[from.getDay()] : `z ${dayMonth(pickedOn)}`;
}

/** `!418` / `#27` out of `group/project!418` or `owner/name#27`. */
function shortRef(reference: string): string {
  const at = Math.max(reference.lastIndexOf("#"), reference.lastIndexOf("!"));
  return at < 0 ? reference : reference.slice(at);
}

/** `owner/name` part of a `owner/name#N` reference. */
function repoOf(reference: string): string {
  const at = reference.lastIndexOf("#");
  return (at < 0 ? reference : reference.slice(0, at)).toLowerCase();
}

/** "N wolne miejsce · …"; without a limit only the rules are shown. */
function footerText(list: TodoList): string {
  const rules = "nieodhaczone → jutro · zamknięte w źródle odhaczają się same";
  if (list.limit === null) return rules;
  const free = Math.max(0, list.limit - list.openCount);
  const head =
    free === 0
      ? "limit osiągnięty"
      : `${free} ${plural(free, "wolne miejsce", "wolne miejsca", "wolnych miejsc")}`;
  return `${head} · ${rules}`;
}

type OwnLoad =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "loaded"; tasks: OwnTask[] };

type RowFlag = { text: string; tone: FlagProps["tone"] };

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
  // Summary strip and source panel: read-only status calls and event
  // listeners; START/STOP and "check now" stay in the header and "Źródła".
  const { timer, nowMs } = useWorkTimer();
  const { report: month, retry: retryMonth } = useMonthReport(
    load.state === "loaded" ? load.list.day : localDay(new Date()),
  );
  const { snapshot: health, loadError: healthError, retry: retryHealth } = useHealth();
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

  const summary: ReactNode = (
    <>
      <StatStrip items={summaryStats({ timer, nowMs, month, health, healthError })} />
      {month.state === "error" && (
        <ErrorState
          title="Nie udało się wczytać raportu miesiąca"
          message={month.message}
          onRetry={retryMonth}
          lastSuccess={lastRefreshLabel(month.lastOkAt)}
        />
      )}
    </>
  );

  if (load.state === "loading") {
    return (
      <main className="container">
        {summary}
        <h1>Do zrobienia</h1>
        <p className="dn-loading">Ładowanie…</p>
      </main>
    );
  }

  if (load.state === "error") {
    return (
      <main className="container">
        {summary}
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
  const atLimit = list.limit === null || list.openCount >= list.limit;

  const onList = new Set<string>();
  for (const item of list.items) {
    onList.add(entryKey(item.entry));
  }

  // "ZMIENIONE" and "DZIŚ" come from data this view already holds: the stored
  // GitLab snapshot's change marks and the own tasks' due dates.
  const changedGitlab = new Set<string>();
  for (const i of snapshot?.items?.issues ?? []) {
    if (i.changed !== null) changedGitlab.add(gitlabKey(i.kind, i.reference));
  }
  const dueToday = new Set<number>();
  if (ownLoad.state === "loaded") {
    for (const t of ownLoad.tasks) {
      if (t.doneAt === null && t.due === list.day) dueToday.add(t.id);
    }
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
    const source: SourceKind = entry.kind === "own" ? "my" : entry.kind === "github" ? "gh" : "gl";

    const flags: RowFlag[] = [];
    if (item.carriedOver) {
      flags.push({ text: carriedLabel(item.pickedOn, list.day), tone: "change" });
    }
    if (!item.ticked && entry.kind === "gitlab" && changedGitlab.has(entryKey(entry))) {
      flags.push({ text: "zmienione", tone: "change" });
    }
    if (!item.ticked && entry.kind === "own" && dueToday.has(entry.taskId)) {
      flags.push({ text: "dziś", tone: "change" });
    }
    if (entry.kind === "gitlab" && entry.closedInSource) {
      flags.push({ text: "zamknięte w GitLabie", tone: "info" });
    } else if (entry.kind === "github" && entry.closedInSource) {
      flags.push({ text: "zamknięte w GitHubie", tone: "info" });
    }
    const [first, ...rest] = flags;

    return (
      <ItemRow
        key={item.id}
        id={`todo-item-${item.id}`}
        done={item.ticked}
        onToggle={() => void act(() => setTicked(item.id, !item.ticked))}
        checkboxDisabled={busy}
        checkboxLabel={item.ticked ? `Odznacz: ${title}` : `Zrobione: ${title}`}
        source={source}
        refId={entry.kind === "own" ? SPACE_REF[entry.space] : shortRef(entry.reference)}
        title={
          url ? (
            <button
              type="button"
              className="link-button"
              title={entry.kind === "own" ? undefined : entry.reference}
              onClick={() => void openUrl(url)}
            >
              {title}
            </button>
          ) : (
            <span title={entry.kind === "own" ? undefined : entry.reference}>{title}</span>
          )
        }
        flag={first?.text}
        flagTone={first?.tone}
        meta={
          rest.length > 0 ? (
            <span className="todo-flags">
              {rest.map((f) => (
                <Flag key={f.text} tone={f.tone}>
                  {f.text}
                </Flag>
              ))}
            </span>
          ) : undefined
        }
      >
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="close"
          disabled={busy}
          aria-label={`Zdejmij: ${title}`}
          title="Zdejmij"
          onClick={() => void act(() => remove(item.id))}
        />
      </ItemRow>
    );
  }

  return (
    <main className="container">
      {summary}

      {list.limitError && <Notice tone="error">{todoErrorMessage(list.limitError)}</Notice>}

      {notice && <Notice tone="error">{notice}</Notice>}

      {reloadError && (
        <ErrorState
          title="Nie udało się odświeżyć listy — może być nieaktualna"
          message={reloadError}
          onRetry={() => void refresh()}
          lastSuccess={lastRefreshLabel(lastSuccessAt)}
        />
      )}

      {unverified && (
        <Notice tone="calm">
          GitLab niedostępny — stan pozycji z GitLaba niezweryfikowany (ostatnie odświeżenie{" "}
          {snapshot?.lastSuccessMs != null ? formatClock(snapshot.lastSuccessMs) : "brak"})
        </Notice>
      )}

      {githubUnverified && (
        <Notice tone="calm">
          GitHub niedostępny lub repozytorium niezaładowane — stan pozycji z GitHuba
          niezweryfikowany (ostatnie odświeżenie{" "}
          {githubSnapshot?.lastSuccessMs != null
            ? formatClock(githubSnapshot.lastSuccessMs)
            : "brak"}
          )
        </Notice>
      )}

      <div className="dn-columns">
        <div className="dn-col-main">
          <Panel
            title="Do zrobienia"
            headingLevel={1}
            meta={
              <span role="status">
                <span className="dn-sr-only">Otwarte </span>
                {list.limit !== null ? `[${list.openCount}/${list.limit}]` : `[${list.openCount}]`}
              </span>
            }
            footer={footerText(list)}
          >
            {list.items.length === 0 && <p className="dn-empty">Nic na dziś — dodaj z list obok</p>}
            {list.items.map(renderRow)}
          </Panel>
        </div>

        <div className="todo-col-pick">
          <Picker
            list={list}
            onList={onList}
            atLimit={atLimit}
            busy={busy}
            ownLoad={ownLoad}
            gitlabItems={
              snapshot?.items ? [...snapshot.items.mergeRequests, ...snapshot.items.issues] : []
            }
            gitlabHidden={snapshot?.items?.hiddenIssues.length ?? 0}
            gitlabOk={gitlabState === "ok"}
            gitlabFilteredOut={
              snapshot?.filterActive
                ? snapshot.filteredOut.mergeRequests + snapshot.filteredOut.issues
                : null
            }
            gitlabLoadError={gitlabLoadError}
            gitlabLastSuccess={clockLabel(snapshot?.lastSuccessMs ?? null)}
            onRetryGitlab={retryGitlab}
            ownLastSuccess={lastRefreshLabel(ownLoadedAt)}
            onRetryOwn={() => setOwnAttempt((n) => n + 1)}
            gitlabStatus={
              gitlabLoadError
                ? null
                : !snapshot || gitlabState === "starting"
                  ? { kind: "loading", text: "Pobieram przypisane z GitLaba…" }
                  : gitlabState === "notConfigured"
                    ? { kind: "calm", text: "GitLab nie jest skonfigurowany." }
                    : gitlabState === "unavailable"
                      ? {
                          kind: "calm",
                          text: "GitLab niedostępny (brak VPN lub sieci) — lista może być nieaktualna.",
                        }
                      : gitlabState === "error"
                        ? { kind: "error", text: "Błąd GitLaba — lista może być nieaktualna." }
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
            githubLastSuccess={clockLabel(githubSnapshot?.lastSuccessMs ?? null)}
            onRetryGithub={retryGithub}
            githubStatus={
              githubLoadError
                ? null
                : !githubSnapshot || githubState === "starting"
                  ? { kind: "loading", text: "Pobieram przypisane z GitHuba…" }
                  : githubState === "notConfigured"
                    ? { kind: "calm", text: "GitHub nie jest skonfigurowany." }
                    : githubState === "unavailable"
                      ? {
                          kind: "calm",
                          text: "GitHub niedostępny (brak sieci) — lista może być nieaktualna.",
                        }
                      : githubState === "error"
                        ? { kind: "error", text: "Błąd GitHuba — lista może być nieaktualna." }
                        : null
            }
            onAddOwn={(id) => void act(() => addOwn(id))}
            onAddGitlab={(item) => void act(() => addGitlab(item.kind, item.reference))}
            onAddGithub={(issue) => void act(() => addGithub(issue.reference))}
          />
        </div>

        <div className="todo-col-side">
          <SourcesPanel
            snapshot={health}
            loadError={healthError}
            onRetry={retryHealth}
            gitlabLastSuccessMs={snapshot?.lastSuccessMs ?? null}
            githubLastSuccessMs={githubSnapshot?.lastSuccessMs ?? null}
          />
        </div>
      </div>
    </main>
  );
}

/** How many GitHub repos failed in an Ok round: none, some or every one. */
type RepoErrorSpread = "none" | "some" | "all";

function repoErrorSpread(failed: boolean[]): RepoErrorSpread {
  const n = failed.filter(Boolean).length;
  return n === 0 ? "none" : n === failed.length ? "all" : "some";
}

/** Status of one picker source: a first round still running, a calm "unavailable", a real error. */
type SourceStatus = { kind: "loading" | "calm" | "error"; text: string };

function SourceStatusView({ status }: { status: SourceStatus }) {
  if (status.kind === "loading") {
    return (
      <p className="dn-loading todo-group-note" role="status">
        {status.text}
      </p>
    );
  }
  return <SourceState kind={status.kind} title={status.text} />;
}

/** Caps name on the left, mono count on the right ("7 · 2 ukryte"). */
function GroupHead({ name, count }: { name: string; count: string | null }) {
  return (
    <h3 className="label-caps todo-group-head">
      <span>{name}</span>
      {count !== null && <span className="todo-group-count">{count}</span>}
    </h3>
  );
}

function Picker({
  list,
  onList,
  atLimit,
  busy,
  ownLoad,
  gitlabItems,
  gitlabHidden,
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
  /** Issues the user has hidden in the GitLab view (never offered here). */
  gitlabHidden: number;
  gitlabOk: boolean;
  /** Items the filter removed from the GitLab list; null when no filter is active. */
  gitlabFilteredOut: number | null;
  gitlabLoadError: string | null;
  gitlabLastSuccess: string;
  onRetryGitlab: () => void;
  ownLastSuccess: string;
  onRetryOwn: () => void;
  gitlabStatus: SourceStatus | null;
  githubIssues: GithubIssue[];
  githubOk: boolean;
  githubRepoErrors: RepoErrorSpread;
  githubLoadError: string | null;
  githubLastSuccess: string;
  onRetryGithub: () => void;
  githubStatus: SourceStatus | null;
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
  const total = gitlabCandidates.length + githubCandidates.length + ownCandidates.length;
  function addButton(label: string, onAdd: () => void) {
    return (
      <TodoAddButton
        className="todo-add"
        label={label}
        onList={false}
        atLimit={atLimit}
        limit={list.limit}
        disabled={busy}
        onAdd={onAdd}
      />
    );
  }

  return (
    <Panel
      title="Do wyboru"
      className="todo-pick"
      meta={`${total} ${plural(total, "pozycja", "pozycje", "pozycji")}`}
    >
      <GroupHead
        name="GitLab"
        count={
          gitlabHidden > 0
            ? `${gitlabCandidates.length} · ${gitlabHidden} ukryte`
            : String(gitlabCandidates.length)
        }
      />
      {gitlabLoadError && (
        <ErrorState
          title="Nie udało się wczytać listy z GitLaba"
          message={gitlabLoadError}
          onRetry={onRetryGitlab}
          lastSuccess={gitlabLastSuccess}
        />
      )}
      {gitlabStatus && <SourceStatusView status={gitlabStatus} />}
      {gitlabCandidates.length === 0 ? (
        gitlabItems.length > 0 ? (
          <p className="dn-empty">Wszystko z GitLaba jest już na liście.</p>
        ) : (
          gitlabOk && (
            <p className="dn-empty">
              {gitlabFilteredOut !== null && gitlabFilteredOut > 0
                ? `Brak pasujących do filtra (ukryte filtrem: ${gitlabFilteredOut})`
                : "Brak przypisanych w GitLabie."}
            </p>
          )
        )
      ) : (
        gitlabCandidates.map((item) => (
          <ItemRow key={gitlabKey(item.kind, item.reference)} title={item.title}>
            {addButton(item.title, () => onAddGitlab(item))}
          </ItemRow>
        ))
      )}

      <GroupHead name="GitHub" count={String(githubCandidates.length)} />
      {githubLoadError && (
        <ErrorState
          title="Nie udało się wczytać listy z GitHuba"
          message={githubLoadError}
          onRetry={onRetryGithub}
          lastSuccess={githubLastSuccess}
        />
      )}
      {githubStatus && <SourceStatusView status={githubStatus} />}
      {githubRepoErrors === "some" && (
        <Notice tone="error" inset>
          Część repozytoriów GitHub nie załadowała się — ich zgłoszeń nie ma na liście.
        </Notice>
      )}
      {githubRepoErrors === "all" && (
        <Notice tone="error" inset>
          Żadne repozytorium GitHub nie załadowało się — nie ma czego dodać.
        </Notice>
      )}
      {githubCandidates.length === 0 ? (
        githubIssues.length > 0 ? (
          <p className="dn-empty">Wszystko z GitHuba jest już na liście.</p>
        ) : githubOk && githubRepoErrors === "none" ? (
          <p className="dn-empty">Brak przypisanych w GitHubie.</p>
        ) : (
          githubOk &&
          githubRepoErrors === "some" && (
            <p className="dn-empty">Brak przypisanych w repozytoriach, które się załadowały.</p>
          )
        )
      ) : (
        githubCandidates.map((issue) => (
          <ItemRow key={githubKey(issue.reference)} title={issue.title}>
            {addButton(issue.title, () => onAddGithub(issue))}
          </ItemRow>
        ))
      )}

      <GroupHead
        name="Moje zadania"
        count={ownLoad.state === "loaded" ? String(ownCandidates.length) : null}
      />
      {ownLoad.state === "loading" && <p className="dn-loading todo-group-note">Ładowanie…</p>}
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
          <p className="dn-empty">Brak otwartych zadań do dodania.</p>
        ) : (
          ownCandidates.map((task) => (
            <ItemRow key={task.id} title={task.title}>
              {addButton(task.title, () => onAddOwn(task.id))}
            </ItemRow>
          ))
        ))}
    </Panel>
  );
}
