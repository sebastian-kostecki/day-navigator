import { ErrorState } from "../components/ErrorState";
import { openGithubIssue } from "./api";
import type { GithubIssue, RepoView } from "./types";
import { useGithub } from "./useGithub";

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
    <li className="gitlab-item">
      <span className="gitlab-ref">#{issue.number}</span>
      <span className="gitlab-title-line">
        {url ? (
          <button type="button" className="link-button" onClick={() => void openGithubIssue(url)}>
            {issue.title}
          </button>
        ) : (
          <span>{issue.title}</span>
        )}
      </span>
      <span className="gitlab-updated">{formatUpdated(issue.updatedAt)}</span>
    </li>
  );
}

function RepoGroup({ repo }: { repo: RepoView }) {
  return (
    <section className="gitlab-section">
      <h2>
        {repo.repo}
        {!repo.error && ` (${repo.issues.length})`}
      </h2>
      {repo.error ? (
        <p className="task-error" role="alert">
          {repo.error.message}
        </p>
      ) : repo.issues.length === 0 ? (
        <p className="empty">Brak przypisanych otwartych zgłoszeń</p>
      ) : (
        <ul className="gitlab-list">
          {repo.issues.map((issue) => (
            <IssueRow key={issue.reference} issue={issue} />
          ))}
        </ul>
      )}
    </section>
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
        <p>Pobieram…</p>
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
      <h1>Projekty (GitHub)</h1>
      {state.state === "notConfigured" && (
        <p className="notice" role="status">
          GitHub nie jest skonfigurowany. Dodaj{" "}
          <code>
            &quot;github&quot;: {"{"} &quot;repos&quot;: [&quot;właściciel/nazwa&quot;] {"}"}
          </code>{" "}
          do settings.json.
        </p>
      )}
      {state.state === "unavailable" && (
        <p className="gitlab-notice" role="status">
          GitHub niedostępny —{" "}
          {lastSuccessMs != null
            ? `lista z ${formatClock(lastSuccessMs)}`
            : "brak udanego odświeżenia od uruchomienia aplikacji"}
        </p>
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
        <div className={state.state !== "ok" ? "gitlab-lists stale" : "gitlab-lists"}>
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
      <div className="actions">
        <button type="button" disabled={refreshing} onClick={() => void refresh()}>
          {refreshing ? "Odświeżam…" : "Odśwież"}
        </button>
        {lastSuccessMs != null && state.state !== "error" && (
          <span className="checked-at">
            Ostatnie udane odświeżenie: {formatClock(lastSuccessMs)}
          </span>
        )}
      </div>
    </main>
  );
}
