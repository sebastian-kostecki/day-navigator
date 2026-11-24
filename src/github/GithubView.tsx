import { useEffect, useState } from "react";
import {
  Button,
  Icon,
  ItemRow,
  Notice,
  Panel,
  SourceState,
  formatAge,
  formatAgo,
} from "../components";
import { ErrorState } from "../components/ErrorState";
import { openGithubIssue } from "./api";
import type { GithubIssue, GithubState, RepoView } from "./types";
import { useGithub } from "./useGithub";
import "./GithubView.css";

/** How often the relative labels ("2 min temu", "3 dni") are recomputed. */
const NOW_TICK_MS = 30_000;

/** Polish count form: 1, 2–4 (not 12–14), everything else. */
function plural(n: number, one: string, few: string, many: string): string {
  if (n === 1) return one;
  const d = n % 10;
  const h = n % 100;
  return d >= 2 && d <= 4 && (h < 12 || h > 14) ? few : many;
}

function formatIssueAge(iso: string, now: number): string {
  const ms = Date.parse(iso);
  return Number.isNaN(ms) ? "—" : formatAge(ms, now);
}

function formatUpdated(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pl-PL");
}

function IssueRow({ issue, now }: { issue: GithubIssue; now: number }) {
  const url = issue.webUrl;
  return (
    <ItemRow
      refId={`#${issue.number}`}
      title={issue.title}
      meta={
        <span title={formatUpdated(issue.updatedAt)}>{formatIssueAge(issue.updatedAt, now)}</span>
      }
    >
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

function RepoGroup({ repo, now }: { repo: RepoView; now: number }) {
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
      ) : repo.issues.length === 0 ? (
        <p className="github-repo-empty">Brak otwartych zgłoszeń przypisanych do Ciebie.</p>
      ) : (
        repo.issues.map((issue) => <IssueRow key={issue.reference} issue={issue} now={now} />)
      )}
    </Panel>
  );
}

function SourceError({
  state,
  lastSuccess,
  onRetry,
}: {
  state: Extract<GithubState, { state: "error" }>;
  lastSuccess: string;
  onRetry: () => void;
}) {
  // "LoggedOut" mirrors `GithubError::kind` in src-tauri/src/github/mod.rs.
  if (state.kind === "LoggedOut") {
    return (
      <ErrorState
        title="GitHub: brak sesji"
        message="Narzędzie gh nie jest zalogowane na tym komputerze. Zaloguj się w terminalu poleceniem „gh auth login”, potem odśwież."
        onRetry={onRetry}
        lastSuccess={lastSuccess}
      />
    );
  }
  return (
    <ErrorState
      title="Błąd GitHuba"
      message={state.message}
      onRetry={onRetry}
      lastSuccess={lastSuccess}
    />
  );
}

export function GithubView() {
  const { snapshot, loadError, refresh, refreshError, retry } = useGithub();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const handle = setInterval(() => setNow(Date.now()), NOW_TICK_MS);
    return () => clearInterval(handle);
  }, []);

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
  // The tick can lag a fresh snapshot; never show a refresh "in the future".
  const nowMs = Math.max(now, lastSuccessMs ?? 0);
  const lastSuccess =
    lastSuccessMs != null
      ? `ostatnie udane odświeżenie: ${formatAgo(lastSuccessMs, nowMs)}`
      : "brak udanego odświeżenia od uruchomienia aplikacji";
  const showList = state.state !== "notConfigured" && repos.length > 0;
  const open = repos.reduce((n, r) => n + r.issues.length, 0);
  const allEmpty = state.state === "ok" && open === 0 && repos.every((r) => !r.error);

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
        />
      )}
      {state.state === "error" && (
        <SourceError state={state} lastSuccess={lastSuccess} onRetry={() => void refresh()} />
      )}
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
              <RepoGroup key={repo.repo} repo={repo} now={nowMs} />
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
