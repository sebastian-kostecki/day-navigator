import { useState } from "react";
import {
  Button,
  Flag,
  Icon,
  ItemRow,
  Notice,
  Panel,
  RELATIVE_TICK_MS,
  SourceState,
  formatAge,
  formatAgo,
  formatDay,
  plural,
  retryText,
  useNow,
} from "../components";
import { ErrorState } from "../components/ErrorState";
import type { ChangeReason } from "../gitlab/types";
import { githubKey } from "../todo/keys";
import { TodoAddButton } from "../todo/TodoAddButton";
import { TodoPicksNotices } from "../todo/TodoPicksNotices";
import { useTodoPicks, type TodoPicks } from "../todo/useTodoPicks";
import { openGithubIssue } from "./api";
import type { GithubIssue, GithubState, RepoView } from "./types";
import { useGithub } from "./useGithub";
import "./GithubView.css";

// The same texts as `CHANGE_LABEL` in `src/gitlab/GitlabView.tsx` (kept apart on purpose).
const CHANGE_LABEL: Record<ChangeReason, string> = {
  status: "zmiana statusu",
  comment: "nowy komentarz",
  returned: "wróciło na listę",
};

type RowActions = {
  busy: ReadonlySet<string>;
  hide: (reference: string) => void;
  unhide: (reference: string) => void;
  dismiss: (reference: string) => void;
};

function formatIssueAge(iso: string, now: number): string {
  const ms = Date.parse(iso);
  return Number.isNaN(ms) ? "—" : formatAge(ms, now);
}

function formatUpdated(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pl-PL");
}

function IssueRow({
  issue,
  now,
  todo,
  actions,
  hidden = false,
  isNew = false,
}: {
  issue: GithubIssue;
  now: number;
  todo: TodoPicks;
  actions: RowActions;
  hidden?: boolean;
  isNew?: boolean;
}) {
  const url = issue.webUrl;
  const found = todo.find(githubKey(issue.reference));
  const working = actions.busy.has(issue.reference);
  const changed = issue.changed;
  return (
    <ItemRow
      dimmed={hidden}
      refId={`#${issue.number}`}
      title={issue.title}
      flag={found ? "W Do zrobienia" : undefined}
      flagTone={found ? "change" : undefined}
      extra={
        <>
          {hidden && (
            <span className="github-hidden-at code">
              {issue.hiddenAtMs != null ? `ukryte ${formatDay(issue.hiddenAtMs, now)}` : "ukryte"}
            </span>
          )}
          {isNew && <Flag tone="new">Nowe</Flag>}
          {changed && (
            <span className="github-flag" title={CHANGE_LABEL[changed]}>
              <Flag tone="change">
                Zmienione<span className="dn-sr-only">: {CHANGE_LABEL[changed]}</span>
              </Flag>
            </span>
          )}
        </>
      }
      labels={issue.labels}
      meta={
        <span title={formatUpdated(issue.updatedAt)}>{formatIssueAge(issue.updatedAt, now)}</span>
      }
    >
      {changed && (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="check-circle"
          disabled={working}
          aria-label={`OK, zauważone: ${issue.title}`}
          title="OK, zauważone"
          onClick={() => actions.dismiss(issue.reference)}
        />
      )}
      {hidden ? (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="eye"
          disabled={working}
          aria-label={`Pokaż: ${issue.title}`}
          title="Pokaż na liście"
          onClick={() => actions.unhide(issue.reference)}
        />
      ) : (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="eye-off"
          disabled={working}
          aria-label={`Ukryj: ${issue.title}`}
          title="Ukryj do zmiany"
          onClick={() => actions.hide(issue.reference)}
        />
      )}
      {(!hidden || found !== undefined) && (
        <TodoAddButton
          label={issue.title}
          onList={found !== undefined}
          atLimit={todo.atLimit}
          limit={todo.limit}
          disabled={todo.buttonDisabled}
          disabledHint={todo.buttonHint}
          onAdd={() => void todo.addGithub(issue.reference)}
          onRemove={found ? () => void todo.remove(found.id) : undefined}
        />
      )}
      {url && (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="external"
          aria-label={`Otwórz na GitHubie: ${issue.title}`}
          title="Otwórz na GitHubie"
          onClick={() => void openGithubIssue(url)}
        />
      )}
    </ItemRow>
  );
}

function RepoGroup({
  repo,
  now,
  todo,
  actions,
  showHidden,
  newRefs,
}: {
  repo: RepoView;
  now: number;
  todo: TodoPicks;
  actions: RowActions;
  showHidden: boolean;
  newRefs: ReadonlySet<string>;
}) {
  return (
    <Panel
      title={
        <span className="github-repo-title">
          <Icon name="repo" className="github-repo-icon" />
          <span className="github-repo-name">{repo.repo}</span>
          {!repo.error && <span className="github-repo-count">[{repo.issues.length}]</span>}
        </span>
      }
    >
      {repo.error ? (
        <Notice tone="error" inset>
          {repo.error.message}
        </Notice>
      ) : repo.issues.length === 0 && repo.hidden.length === 0 ? (
        <p className="github-repo-empty">Brak otwartych zgłoszeń przypisanych do Ciebie.</p>
      ) : (
        <>
          {repo.issues.length === 0 && !showHidden && (
            <p className="github-repo-empty">Wszystkie przypisane zgłoszenia są ukryte.</p>
          )}
          {repo.issues.map((issue) => (
            <IssueRow
              key={issue.reference}
              issue={issue}
              now={now}
              todo={todo}
              actions={actions}
              isNew={newRefs.has(issue.reference)}
            />
          ))}
          {showHidden &&
            repo.hidden.map((issue) => (
              <IssueRow
                key={issue.reference}
                issue={issue}
                now={now}
                todo={todo}
                actions={actions}
                hidden
              />
            ))}
        </>
      )}
    </Panel>
  );
}

function SourceError({
  state,
  lastSuccess,
  retryNote,
  onRetry,
}: {
  state: Extract<GithubState, { state: "error" }>;
  lastSuccess: string;
  /** The interval and next try; `null` when no round will come. */
  retryNote: string | null;
  onRetry: () => void;
}) {
  // "LoggedOut" mirrors `GithubError::kind` in src-tauri/src/github/mod.rs.
  if (state.kind === "LoggedOut") {
    return (
      <ErrorState
        title="GitHub: brak sesji"
        message={
          <>
            Narzędzie gh nie jest zalogowane na tym komputerze. Zaloguj się w terminalu poleceniem{" "}
            <code>gh auth login</code>, potem odśwież.
            {retryNote !== null && ` ${retryNote}`}
          </>
        }
        onRetry={onRetry}
        lastSuccess={lastSuccess}
      />
    );
  }
  return (
    <ErrorState
      title="Błąd GitHuba"
      message={
        retryNote !== null
          ? `${state.message}${/[.!?…]$/.test(state.message) ? "" : "."} ${retryNote}`
          : state.message
      }
      onRetry={onRetry}
      lastSuccess={lastSuccess}
    />
  );
}

export function GithubView() {
  const {
    snapshot,
    loadError,
    refresh,
    refreshError,
    retry,
    actionError,
    busy,
    hide,
    unhide,
    dismiss,
  } = useGithub();
  const todo = useTodoPicks();
  const now = useNow(RELATIVE_TICK_MS);
  // In memory only: the toggle is not remembered.
  const [showHidden, setShowHidden] = useState(false);

  if (loadError) {
    return (
      <main className="container">
        <h1>Projekty (GitHub)</h1>
        <ErrorState
          title="Nie można odczytać listy z GitHuba"
          message={loadError}
          onRetry={retry}
        />
      </main>
    );
  }

  if (!snapshot || (snapshot.state.state === "starting" && snapshot.repos.length === 0)) {
    return (
      <main className="container">
        <h1>Projekty (GitHub)</h1>
        <p className="dn-loading">Pobieram…</p>
      </main>
    );
  }

  const { state, repos, lastSuccessMs, refreshing } = snapshot;
  const newRefs = new Set(snapshot.newReferences);
  const actions: RowActions = { busy, hide, unhide, dismiss };
  // The tick can lag a fresh snapshot; never show a refresh "in the future".
  const nowMs = Math.max(now, lastSuccessMs ?? 0);
  const lastSuccess =
    lastSuccessMs != null
      ? `ostatnie udane odświeżenie: ${formatAgo(lastSuccessMs, nowMs)}`
      : "brak udanego odświeżenia od uruchomienia aplikacji";
  const retryNote = retryText(snapshot.refreshMinutes, snapshot.nextRefreshAtMs, refreshing, nowMs);
  const showList = state.state !== "notConfigured" && repos.length > 0;
  const open = repos.reduce((n, r) => n + r.issues.length, 0);
  const hiddenCount = repos.reduce((n, r) => n + (r.error ? 0 : r.hidden.length), 0);
  const allEmpty =
    state.state === "ok" && open === 0 && hiddenCount === 0 && repos.every((r) => !r.error);

  let summary: string | null = null;
  if (showList) {
    summary = `${open} ${plural(open, "otwarte", "otwarte", "otwartych")} · ${repos.length} ${plural(repos.length, "repozytorium", "repozytoria", "repozytoriów")}`;
  } else if (state.state === "unavailable") {
    summary = "źródło niedostępne";
  } else if (state.state === "error") {
    summary = "błąd źródła";
  }

  return (
    <main className="container">
      <div className="dn-view-head">
        <h1>Projekty (GitHub)</h1>
        {summary && <span className="dn-view-head-summary">{summary}</span>}
        <span className="dn-spacer" />
        {lastSuccessMs != null && (
          <span className="dn-view-head-note">odświeżone {formatAgo(lastSuccessMs, nowMs)}</span>
        )}
        <Button
          size="sm"
          variant="quiet"
          pressed={showHidden}
          disabled={hiddenCount === 0 && !showHidden}
          onClick={() => setShowHidden((s) => !s)}
        >
          Pokaż ukryte ({hiddenCount})
        </Button>
        <Button
          size="sm"
          variant="quiet"
          icon="refresh"
          disabled={refreshing}
          onClick={() => void refresh()}
        >
          {refreshing ? "Odświeżam…" : "Odśwież"}
        </Button>
      </div>
      {state.state === "notConfigured" && (
        <SourceState kind="calm" title="GitHub nie jest skonfigurowany.">
          Dodaj{" "}
          <code>
            &quot;github&quot;: {"{"} &quot;repos&quot;: [&quot;właściciel/nazwa&quot;] {"}"}
          </code>{" "}
          do settings.json.
        </SourceState>
      )}
      {state.state === "unavailable" && (
        <SourceState
          kind="calm"
          title="GitHub niedostępny"
          lastOk={lastSuccess}
          action="Sprawdź teraz"
          onAction={() => void refresh()}
          actionDisabled={refreshing}
        >
          {retryNote}
        </SourceState>
      )}
      {state.state === "error" && (
        <SourceError
          state={state}
          lastSuccess={lastSuccess}
          // "ThreadDown" mirrors `GithubError::kind` in src-tauri/src/github/mod.rs: no round ever comes.
          retryNote={state.kind === "ThreadDown" ? null : retryNote}
          onRetry={() => void refresh()}
        />
      )}
      {/* "1–14" and "10 min" mirror REFRESH_MINUTES_MAX and DEFAULT_REFRESH_MINUTES in src-tauri/src/settings.rs. */}
      {snapshot.refreshSettingInvalid && (
        <Notice tone="warn">
          Nieprawidłowe ustawienie <code>githubRefresh.minutes</code> (dozwolone 1–14) — odświeżam
          co 10 min
        </Notice>
      )}
      {snapshot.hiddenError && (
        <Notice tone="warn">
          Lista pokazuje wszystkie zgłoszenia, bo nie udało się odczytać ukrytych.{" "}
          {snapshot.hiddenError}
        </Notice>
      )}
      {snapshot.hiddenCheckFailed > 0 && (
        <Notice tone="warn">
          Nie udało się sprawdzić {snapshot.hiddenCheckFailed} ukrytych — spróbuję przy następnym
          odświeżeniu
        </Notice>
      )}
      {actionError && <Notice tone="error">{actionError}</Notice>}
      <TodoPicksNotices todo={todo} />
      {showList && allEmpty && (
        <SourceState kind="empty" title="Nic przypisanego">
          Brak otwartych zgłoszeń przypisanych do Ciebie w {repos.length}{" "}
          {repos.length === 1 ? "repozytorium" : "repozytoriach"} z ustawień.
        </SourceState>
      )}
      {showList && !allEmpty && (
        <div className={state.state !== "ok" ? "dn-columns github-stale" : "dn-columns"}>
          <div className="dn-col-main github-repos">
            {repos.map((repo) => (
              <RepoGroup
                key={repo.repo}
                repo={repo}
                now={nowMs}
                todo={todo}
                actions={actions}
                showHidden={showHidden}
                newRefs={newRefs}
              />
            ))}
          </div>
          <aside className="dn-col-side">
            <Panel title="Zakres">
              <div className="github-scope">
                {/* Mirrors the fixed `gh issue list --assignee @me --state open` call in src-tauri/src/cli. */}
                <p>
                  Tylko otwarte zgłoszenia przypisane do Ciebie, z repozytoriów wskazanych w
                  ustawieniach. Reszta zostaje backlogiem na GitHubie.
                </p>
                {/* Mirrors `plan_round` in src-tauri/src/hidden_issues/rules.rs. */}
                <p>
                  Ukryte zgłoszenie wraca samo, gdy ktoś inny doda komentarz albo zgłoszenie wróci
                  na listę po zamknięciu i ponownym otwarciu.
                </p>
                <p>Aplikacja tylko czyta — odhaczenie w „Do zrobienia” nie zamyka zgłoszenia.</p>
              </div>
            </Panel>
          </aside>
        </div>
      )}
      {refreshError && (
        <ErrorState
          title="Nie udało się odświeżyć"
          message={refreshError}
          onRetry={() => void refresh()}
        />
      )}
    </main>
  );
}
