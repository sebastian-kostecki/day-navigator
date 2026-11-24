import { Button, ItemRow, Panel, SourceState } from "../components";
import { ErrorState } from "../components/ErrorState";
import { openGithubIssue } from "./api";
import type { GithubIssue, RepoView } from "./types";
import { useGithub } from "./useGithub";
import "./GithubView.css";

function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
}

function formatUpdated(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pl-PL");
}

function IssueRow({ issue }: { issue: GithubIssue }) {
  const url = issue.webUrl;
  return (
    <ItemRow
      source="gh"
      refId={`#${issue.number}`}
      title={
        url ? (
          <button type="button" className="link-button" onClick={() => void openGithubIssue(url)}>
            {issue.title}
          </button>
        ) : (
          issue.title
        )
      }
      meta={formatUpdated(issue.updatedAt)}
    />
  );
}

function RepoGroup({ repo }: { repo: RepoView }) {
  return (
    <Panel
      title={<span className="github-repo">{repo.repo}</span>}
      meta={repo.error ? undefined : repo.issues.length}
    >
      {repo.error ? (
        <p className="github-repo-error" role="alert">
          {repo.error.message}
        </p>
      ) : repo.issues.length === 0 ? (
        <p className="dn-empty">Brak przypisanych otwartych zgłoszeń</p>
      ) : (
        repo.issues.map((issue) => <IssueRow key={issue.reference} issue={issue} />)
      )}
    </Panel>
  );
}

export function GithubView() {
  const { snapshot, loadError, refresh, refreshError, retry } = useGithub();

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
  const lastSuccess =
    lastSuccessMs != null
      ? `Ostatnie udane odświeżenie: ${formatClock(lastSuccessMs)}`
      : "Brak udanego odświeżenia od uruchomienia aplikacji";

  return (
    <main className="container">
      <div className="dn-view-head">
        <h1>Projekty (GitHub)</h1>
        {lastSuccessMs != null && state.state !== "error" && (
          <span className="meta github-last-ok">
            Ostatnie udane odświeżenie: {formatClock(lastSuccessMs)}
          </span>
        )}
        <Button size="sm" disabled={refreshing} onClick={() => void refresh()}>
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
          lastOk={
            lastSuccessMs != null
              ? `Lista z ${formatClock(lastSuccessMs)}`
              : "Brak udanego odświeżenia od uruchomienia aplikacji"
          }
        />
      )}
      {state.state === "error" && (
        <ErrorState
          title="Błąd GitHuba"
          message={state.message}
          onRetry={() => void refresh()}
          lastSuccess={lastSuccess}
        />
      )}
      {state.state !== "notConfigured" && repos.length > 0 && (
        <div className={state.state !== "ok" ? "github-lists is-stale" : "github-lists"}>
          {repos.map((repo) => (
            <RepoGroup key={repo.repo} repo={repo} />
          ))}
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
