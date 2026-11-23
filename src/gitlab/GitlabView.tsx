import { ErrorState } from "../components/ErrorState";
import { openGitlabItem } from "./api";
import type {
  AssignedItems,
  ChangeReason,
  ClosedSection,
  GitlabItem,
  GitlabSnapshot,
} from "./types";
import { useGitlab } from "./useGitlab";

function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
}

function formatUpdated(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pl-PL");
}

const CHANGE_LABEL: Record<ChangeReason, string> = {
  status: "zmiana statusu",
  comment: "nowy komentarz",
  returned: "wróciło na listę",
};

type RowActions = {
  busy: ReadonlySet<number>;
  hide: (id: number) => void;
  unhide: (id: number) => void;
  dismiss: (id: number) => void;
};

function ItemRow({
  item,
  actions,
  hidden = false,
  at,
}: {
  item: GitlabItem;
  /** Only issue rows get actions; merge request rows stay as they are. */
  actions?: RowActions;
  hidden?: boolean;
  /** The time the row shows; the last update when not given. */
  at?: string;
}) {
  const url = item.webUrl;
  const working = actions?.busy.has(item.id) ?? false;
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
        {item.changed && (
          <span className="badge badge-changed">Zmienione: {CHANGE_LABEL[item.changed]}</span>
        )}
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
      <span className="gitlab-updated">{formatUpdated(at ?? item.updatedAt)}</span>
      {actions && (
        <span className="gitlab-actions">
          {item.changed && (
            <button
              type="button"
              disabled={working}
              aria-label={`OK, zauważone: ${item.title}`}
              onClick={() => actions.dismiss(item.id)}
            >
              OK
            </button>
          )}
          {hidden ? (
            <button
              type="button"
              disabled={working}
              aria-label={`Pokaż: ${item.title}`}
              onClick={() => actions.unhide(item.id)}
            >
              Pokaż
            </button>
          ) : (
            <button
              type="button"
              disabled={working}
              aria-label={`Ukryj: ${item.title}`}
              onClick={() => actions.hide(item.id)}
            >
              Ukryj
            </button>
          )}
        </span>
      )}
    </li>
  );
}

function FilteredCount({ count }: { count: number }) {
  if (count <= 0) return null;
  return <p className="checked-at">Ukryte filtrem: {count}</p>;
}

function Section({
  title,
  items,
  emptyText,
  filteredOut,
  actions,
}: {
  title: string;
  items: GitlabItem[];
  emptyText: string;
  filteredOut: number;
  actions?: RowActions;
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
            <ItemRow key={`${item.kind}:${item.reference}`} item={item} actions={actions} />
          ))}
        </ul>
      )}
      <FilteredCount count={filteredOut} />
    </section>
  );
}

function HiddenSection({
  items,
  filteredOut,
  actions,
}: {
  items: GitlabItem[];
  filteredOut: number;
  actions: RowActions;
}) {
  if (items.length === 0 && filteredOut <= 0) return null;
  return (
    <details className="gitlab-hidden">
      <summary>Ukryte ({items.length})</summary>
      <FilteredCount count={filteredOut} />
      <ul className="gitlab-list">
        {items.map((item) => (
          <ItemRow key={`${item.kind}:${item.reference}`} item={item} actions={actions} hidden />
        ))}
      </ul>
    </details>
  );
}

function ClosedBlock({ closed }: { closed: ClosedSection }) {
  const oneDay = closed.days === 1;
  return (
    <section className="gitlab-section">
      <h2>
        {oneDay ? "Zamknięte (ostatni dzień)" : `Zamknięte (ostatnie ${closed.days} dni)`}
        {!closed.error && ` (${closed.items.length})`}
      </h2>
      {closed.error ? (
        <p className="gitlab-notice" role="status">
          {closed.error}
        </p>
      ) : closed.items.length === 0 ? (
        <p className="empty">
          {oneDay
            ? "Brak zamkniętych w ostatnim dniu."
            : `Brak zamkniętych w ostatnich ${closed.days} dniach.`}
        </p>
      ) : (
        <ul className="gitlab-list">
          {closed.items.map((item) => (
            <ItemRow key={`${item.kind}:${item.reference}`} item={item} at={item.closedAt} />
          ))}
        </ul>
      )}
      <FilteredCount count={closed.filteredOut} />
    </section>
  );
}

function Lists({
  items,
  snapshot,
  stale,
  actions,
}: {
  items: AssignedItems;
  snapshot: GitlabSnapshot;
  stale: boolean;
  actions: RowActions;
}) {
  const { filterActive, filteredOut, closed } = snapshot;
  return (
    <div className={stale ? "gitlab-lists stale" : "gitlab-lists"}>
      <Section
        title="Merge requesty"
        items={items.mergeRequests}
        emptyText={
          filterActive
            ? "Brak merge requestów pasujących do filtra."
            : "Brak przypisanych merge requestów."
        }
        filteredOut={filteredOut.mergeRequests}
      />
      <Section
        title="Zgłoszenia"
        items={items.issues}
        emptyText={
          filterActive ? "Brak zgłoszeń pasujących do filtra." : "Brak przypisanych zgłoszeń."
        }
        filteredOut={filteredOut.issues}
        actions={actions}
      />
      <HiddenSection
        items={items.hiddenIssues}
        filteredOut={filteredOut.hiddenIssues}
        actions={actions}
      />
      {closed && <ClosedBlock closed={closed} />}
    </div>
  );
}

export function GitlabView() {
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
  } = useGitlab();

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

  const { state, items, lastSuccessMs, refreshing, hiddenCheckFailed, ownCommentsKnown } = snapshot;
  const actions: RowActions = { busy, hide, unhide, dismiss };
  // Filtered-out hidden issues are still checked for comments, so they count.
  const hasHidden = (items?.hiddenIssues.length ?? 0) + snapshot.filteredOut.hiddenIssues > 0;
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
      {/* "1–14" and "10 min" mirror REFRESH_MINUTES_MAX and DEFAULT_REFRESH_MINUTES in src-tauri/src/settings.rs. */}
      {snapshot.refreshSettingInvalid && (
        <p className="gitlab-notice" role="status">
          Nieprawidłowe ustawienie <code>gitlabRefresh.minutes</code> (dozwolone 1–14) — odświeżam
          co 10 min
        </p>
      )}
      {snapshot.filterInvalid && (
        <p className="gitlab-notice" role="status">
          Nieprawidłowy blok gitlabFilter w settings.json — pokazuję wszystkie przypisane
        </p>
      )}
      {snapshot.hiddenError && (
        <p className="gitlab-notice" role="status">
          Lista pokazuje wszystkie zgłoszenia, bo nie udało się odczytać ukrytych.{" "}
          {snapshot.hiddenError}
        </p>
      )}
      {hiddenCheckFailed > 0 && (
        <p className="gitlab-notice" role="status">
          Nie udało się sprawdzić {hiddenCheckFailed} ukrytych — spróbuję przy następnym odświeżeniu
        </p>
      )}
      {hasHidden && !ownCommentsKnown && (
        <p className="gitlab-notice" role="status">
          Ustaw <code>&quot;user&quot;</code> w bloku <code>&quot;gitlab&quot;</code> w
          settings.json, aby Twoje własne komentarze nie przywracały ukrytych zgłoszeń.
        </p>
      )}
      {actionError && (
        <p className="task-error" role="alert">
          {actionError}
        </p>
      )}
      {items && state.state !== "notConfigured" && (
        <Lists items={items} snapshot={snapshot} stale={state.state !== "ok"} actions={actions} />
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
