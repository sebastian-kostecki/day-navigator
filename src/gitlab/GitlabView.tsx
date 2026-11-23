import { ErrorState } from "../components/ErrorState";
import { openGitlabItem } from "./api";
import type { AssignedItems, GitlabItem } from "./types";
import { useGitlab } from "./useGitlab";

function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
}

function formatUpdated(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pl-PL");
}

function ItemRow({ item }: { item: GitlabItem }) {
  const url = item.webUrl;
  return (
    <li className="gitlab-item">
      <span className="gitlab-ref">{item.reference}</span>
      <span className="gitlab-title-line">
        {url ? (
          <button type="button" className="link-button" onClick={() => void openGitlabItem(url)}>
            {item.title}
          </button>
        ) : (
          <span>{item.title}</span>
        )}
        {item.draft && <span className="badge">Draft</span>}
      </span>
      {item.labels.length > 0 && (
        <span className="gitlab-labels">
          {item.labels.map((l) => (
            <span key={l} className="badge">
              {l}
            </span>
          ))}
        </span>
      )}
      <span className="gitlab-updated">{formatUpdated(item.updatedAt)}</span>
    </li>
  );
}

function Section({
  title,
  items,
  emptyText,
}: {
  title: string;
  items: GitlabItem[];
  emptyText: string;
}) {
  return (
    <section className="gitlab-section">
      <h2>
        {title} ({items.length})
      </h2>
      {items.length === 0 ? (
        <p className="empty">{emptyText}</p>
      ) : (
        <ul className="gitlab-list">
          {items.map((item) => (
            <ItemRow key={`${item.kind}:${item.reference}`} item={item} />
          ))}
        </ul>
      )}
    </section>
  );
}

function Lists({ items, stale }: { items: AssignedItems; stale: boolean }) {
  return (
    <div className={stale ? "gitlab-lists stale" : "gitlab-lists"}>
      <Section
        title="Merge requesty"
        items={items.mergeRequests}
        emptyText="Brak przypisanych merge requestów."
      />
      <Section title="Zgłoszenia" items={items.issues} emptyText="Brak przypisanych zgłoszeń." />
    </div>
  );
}

export function GitlabView() {
  const { snapshot, loadError, refresh, refreshError, retry } = useGitlab();

  if (loadError) {
    return (
      <main className="container">
        <h1>Praca (GitLab)</h1>
        <ErrorState
          title="Nie można odczytać listy z GitLaba"
          message={loadError}
          onRetry={retry}
        />
      </main>
    );
  }

  if (!snapshot || (snapshot.state.state === "starting" && !snapshot.items)) {
    return (
      <main className="container">
        <h1>Praca (GitLab)</h1>
        <p>Pobieram przypisane z GitLaba…</p>
      </main>
    );
  }

  const { state, items, lastSuccessMs, refreshing } = snapshot;
  const lastSuccess =
    lastSuccessMs != null
      ? `Ostatnie udane odświeżenie: ${formatClock(lastSuccessMs)}`
      : "Brak udanego odświeżenia od uruchomienia aplikacji";

  return (
    <main className="container">
      <h1>Praca (GitLab)</h1>
      {state.state === "notConfigured" && (
        <p className="notice" role="status">
          GitLab nie jest skonfigurowany. Dodaj{" "}
          <code>
            &quot;gitlab&quot;: {"{"} &quot;host&quot;: &quot;…&quot; {"}"}
          </code>{" "}
          do settings.json (opcjonalnie <code>&quot;user&quot;</code>: login, gdy glab jest
          zalogowany na konto serwisowe).
        </p>
      )}
      {state.state === "unavailable" && (
        <p className="gitlab-notice" role="status">
          GitLab niedostępny (brak VPN lub sieci) —{" "}
          {lastSuccessMs != null
            ? `lista z ${formatClock(lastSuccessMs)}`
            : "brak udanego odświeżenia od uruchomienia aplikacji"}
        </p>
      )}
      {state.state === "error" && (
        <ErrorState
          title="Błąd GitLaba"
          message={state.message}
          onRetry={() => void refresh()}
          lastSuccess={lastSuccess}
        />
      )}
      {items && state.state !== "notConfigured" && (
        <Lists items={items} stale={state.state !== "ok"} />
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
