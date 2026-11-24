import type { ReactNode } from "react";
import { Button, Flag, ItemRow, Panel, SourceState } from "../components";
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
import "./GitlabView.css";

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

function GitlabRow({
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
    <ItemRow
      source={item.kind === "mergeRequest" ? "mr" : "iss"}
      refId={item.reference}
      title={
        <span className="gitlab-title">
          {url ? (
            <button
              type="button"
              className="link-button gitlab-title-text"
              onClick={() => void openGitlabItem(url)}
            >
              {item.title}
            </button>
          ) : (
            <span className="gitlab-title-text">{item.title}</span>
          )}
          {item.draft && <Flag tone="change">Draft</Flag>}
          {item.labels.map((l) => (
            <span key={l} className="gitlab-label">
              {l}
            </span>
          ))}
        </span>
      }
      flag={item.changed ? `Zmienione: ${CHANGE_LABEL[item.changed]}` : undefined}
      flagTone="change"
      meta={formatUpdated(at ?? item.updatedAt)}
    >
      {actions && (
        <>
          {item.changed && (
            <Button
              variant="quiet"
              size="sm"
              disabled={working}
              aria-label={`OK, zauważone: ${item.title}`}
              onClick={() => actions.dismiss(item.id)}
            >
              OK
            </Button>
          )}
          {hidden ? (
            <Button
              variant="quiet"
              size="sm"
              disabled={working}
              aria-label={`Pokaż: ${item.title}`}
              onClick={() => actions.unhide(item.id)}
            >
              Pokaż
            </Button>
          ) : (
            <Button
              variant="quiet"
              size="sm"
              disabled={working}
              aria-label={`Ukryj: ${item.title}`}
              onClick={() => actions.hide(item.id)}
            >
              Ukryj
            </Button>
          )}
        </>
      )}
    </ItemRow>
  );
}

function filteredNote(count: number): ReactNode {
  return count > 0 ? `Ukryte filtrem: ${count}` : undefined;
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
    <Panel title={title} meta={items.length} footer={filteredNote(filteredOut)}>
      {items.length === 0 ? (
        <p className="dn-empty">{emptyText}</p>
      ) : (
        items.map((item) => (
          <GitlabRow key={`${item.kind}:${item.reference}`} item={item} actions={actions} />
        ))
      )}
    </Panel>
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
  const note = filteredNote(filteredOut);
  return (
    <details className="gitlab-hidden">
      <summary>Ukryte ({items.length})</summary>
      {note && <p className="meta gitlab-hidden-note">{note}</p>}
      {items.map((item) => (
        <GitlabRow key={`${item.kind}:${item.reference}`} item={item} actions={actions} hidden />
      ))}
    </details>
  );
}

function ClosedBlock({ closed }: { closed: ClosedSection }) {
  const oneDay = closed.days === 1;
  return (
    <Panel
      title={oneDay ? "Zamknięte (ostatni dzień)" : `Zamknięte (ostatnie ${closed.days} dni)`}
      meta={closed.error ? undefined : closed.items.length}
      footer={filteredNote(closed.filteredOut)}
    >
      {closed.error ? (
        <p className="gitlab-note" role="status">
          {closed.error}
        </p>
      ) : closed.items.length === 0 ? (
        <p className="dn-empty">
          {oneDay
            ? "Brak zamkniętych w ostatnim dniu."
            : `Brak zamkniętych w ostatnich ${closed.days} dniach.`}
        </p>
      ) : (
        closed.items.map((item) => (
          <GitlabRow key={`${item.kind}:${item.reference}`} item={item} at={item.closedAt} />
        ))
      )}
    </Panel>
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
    <div className={stale ? "gitlab-lists is-stale" : "gitlab-lists"}>
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
        <p className="dn-loading">Pobieram przypisane z GitLaba…</p>
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
      <div className="dn-view-head">
        <h1>Praca (GitLab)</h1>
        {lastSuccessMs != null && state.state !== "error" && (
          <span className="meta gitlab-last-ok">
            Ostatnie udane odświeżenie: {formatClock(lastSuccessMs)}
          </span>
        )}
        <Button size="sm" disabled={refreshing} onClick={() => void refresh()}>
          {refreshing ? "Odświeżam…" : "Odśwież"}
        </Button>
      </div>
      {state.state === "notConfigured" && (
        <SourceState kind="calm" title="GitLab nie jest skonfigurowany.">
          Dodaj{" "}
          <code>
            &quot;gitlab&quot;: {"{"} &quot;host&quot;: &quot;…&quot; {"}"}
          </code>{" "}
          do settings.json (opcjonalnie <code>&quot;user&quot;</code>: login, gdy glab jest
          zalogowany na konto serwisowe).
        </SourceState>
      )}
      {state.state === "unavailable" && (
        <SourceState
          kind="calm"
          title="GitLab niedostępny (brak VPN lub sieci)"
          lastOk={
            lastSuccessMs != null
              ? `Lista z ${formatClock(lastSuccessMs)}`
              : "Brak udanego odświeżenia od uruchomienia aplikacji"
          }
        />
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
        <p className="gitlab-note" role="status">
          Nieprawidłowe ustawienie <code>gitlabRefresh.minutes</code> (dozwolone 1–14) — odświeżam
          co 10 min
        </p>
      )}
      {snapshot.filterInvalid && (
        <p className="gitlab-note" role="status">
          Nieprawidłowy blok gitlabFilter w settings.json — pokazuję wszystkie przypisane
        </p>
      )}
      {snapshot.hiddenError && (
        <p className="gitlab-note" role="status">
          Lista pokazuje wszystkie zgłoszenia, bo nie udało się odczytać ukrytych.{" "}
          {snapshot.hiddenError}
        </p>
      )}
      {hiddenCheckFailed > 0 && (
        <p className="gitlab-note" role="status">
          Nie udało się sprawdzić {hiddenCheckFailed} ukrytych — spróbuję przy następnym odświeżeniu
        </p>
      )}
      {hasHidden && !ownCommentsKnown && (
        <p className="gitlab-note" role="status">
          Ustaw <code>&quot;user&quot;</code> w bloku <code>&quot;gitlab&quot;</code> w
          settings.json, aby Twoje własne komentarze nie przywracały ukrytych zgłoszeń.
        </p>
      )}
      {actionError && (
        <p className="gitlab-action-error" role="alert">
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
    </main>
  );
}
