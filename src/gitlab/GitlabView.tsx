import { useState } from "react";
import {
  Button,
  DataTable,
  DataTableCell,
  DataTableRow,
  Flag,
  Notice,
  Panel,
  RELATIVE_TICK_MS,
  Segmented,
  SourceChip,
  SourceState,
  formatAge,
  formatAgo,
  plural,
  useNow,
} from "../components";
import { ErrorState } from "../components/ErrorState";
import { gitlabKey } from "../todo/keys";
import { TodoAddButton } from "../todo/TodoAddButton";
import { TodoPicksNotices } from "../todo/TodoPicksNotices";
import { useTodoPicks, type TodoPicks } from "../todo/useTodoPicks";
import { openGitlabItem } from "./api";
import type { ChangeReason, ClosedSection, GitlabItem, GitlabSnapshot } from "./types";
import { useGitlab } from "./useGitlab";
import "./GitlabView.css";

function formatUpdated(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pl-PL");
}

/** Today: "12 min temu" / "2 godz. temu"; older: "1 dzień", "3 dni", "tydz.". */
function formatChange(iso: string, now: number): string {
  const ms = Date.parse(iso);
  if (Number.isNaN(ms)) return "—";
  return formatAge(ms, now) === "dziś" ? formatAgo(ms, now) : formatAge(ms, now);
}

/** `group/sub/project#12` → nr `#12`, project `project`, path `group/sub/project`. */
function splitReference(reference: string): { nr: string; project: string; path: string } {
  const at = Math.max(reference.lastIndexOf("#"), reference.lastIndexOf("!"));
  if (at <= 0) return { nr: reference, project: "", path: "" };
  const path = reference.slice(0, at);
  return { nr: reference.slice(at), project: path.slice(path.lastIndexOf("/") + 1), path };
}

/** Newest first, ties by reference: the order the backend uses inside each list. */
function byUpdated(a: GitlabItem, b: GitlabItem): number {
  const diff = (Date.parse(b.updatedAt) || 0) - (Date.parse(a.updatedAt) || 0);
  return diff !== 0 ? diff : a.reference.localeCompare(b.reference);
}

const CHANGE_LABEL: Record<ChangeReason, string> = {
  status: "zmiana statusu",
  comment: "nowy komentarz",
  returned: "wróciło na listę",
};

type KindFilter = "all" | "issue" | "mergeRequest";

const KIND_OPTIONS = [
  { value: "all", label: "Wszystko" },
  { value: "issue", label: "Zgłoszenia" },
  { value: "mergeRequest", label: "MR-y" },
];

function matchesKind(item: GitlabItem, kind: KindFilter): boolean {
  return kind === "all" || item.kind === kind;
}

type RowActions = {
  /** The "Do zrobienia" list, for the per-row "+" button. */
  todo: TodoPicks;
  busy: ReadonlySet<number>;
  hide: (id: number) => void;
  unhide: (id: number) => void;
  dismiss: (id: number) => void;
};

type TableRow = {
  item: GitlabItem;
  hidden?: boolean;
  /** The time the row shows; the last update when not given. */
  at?: string;
};

function RowButtons({
  item,
  hidden,
  actions,
}: {
  item: GitlabItem;
  hidden: boolean;
  actions: RowActions;
}) {
  const working = actions.busy.has(item.id);
  return (
    <>
      {item.changed && (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="check-circle"
          disabled={working}
          aria-label={`OK, zauważone: ${item.title}`}
          title="OK, zauważone"
          onClick={() => actions.dismiss(item.id)}
        />
      )}
      {hidden ? (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="eye"
          disabled={working}
          aria-label={`Pokaż: ${item.title}`}
          title="Pokaż na liście"
          onClick={() => actions.unhide(item.id)}
        />
      ) : (
        <Button
          variant="quiet"
          size="sm"
          iconOnly
          icon="eye-off"
          disabled={working}
          aria-label={`Ukryj: ${item.title}`}
          title="Ukryj do zmiany"
          onClick={() => actions.hide(item.id)}
        />
      )}
    </>
  );
}

function GitlabRow({
  row,
  now,
  actions,
}: {
  row: TableRow;
  now: number;
  /** Hide/dismiss buttons only on issue rows; every row gets the "Do zrobienia" button. */
  actions?: RowActions;
}) {
  const { item, hidden = false } = row;
  const todoFound = actions?.todo.find(gitlabKey(item.kind, item.reference));
  const at = row.at ?? item.updatedAt;
  const { nr, project, path } = splitReference(item.reference);
  const url = item.webUrl;
  const labels = item.labels.join(", ");
  return (
    <DataTableRow className={hidden ? "gitlab-row is-hidden" : "gitlab-row"}>
      <DataTableCell>
        <SourceChip source={item.kind === "mergeRequest" ? "mr" : "iss"} />
      </DataTableCell>
      <DataTableCell tone="faint" className="gitlab-col-nr code">
        {nr}
      </DataTableCell>
      <DataTableCell className="gitlab-col-title">
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
        {hidden && <span className="dn-sr-only"> (ukryte)</span>}
        {item.changed && (
          <span className="gitlab-flag" title={CHANGE_LABEL[item.changed]}>
            <Flag tone="change">
              Zmienione<span className="dn-sr-only">: {CHANGE_LABEL[item.changed]}</span>
            </Flag>
          </span>
        )}
        {item.draft && <Flag tone="change">Draft</Flag>}
        {todoFound && <Flag tone="change">W Do zrobienia</Flag>}
      </DataTableCell>
      <DataTableCell tone="muted" truncate className="gitlab-col-project" title={path || undefined}>
        {project}
      </DataTableCell>
      <DataTableCell
        tone="faint"
        truncate
        className="gitlab-col-labels code"
        title={labels || undefined}
      >
        {labels}
      </DataTableCell>
      <DataTableCell tone="faint" className="gitlab-col-when code" title={formatUpdated(at)}>
        {formatChange(at, now)}
      </DataTableCell>
      <DataTableCell className="gitlab-col-actions">
        {actions && item.kind === "issue" && (
          <RowButtons item={item} hidden={hidden} actions={actions} />
        )}
        {actions && (todoFound || !hidden) && (
          <TodoAddButton
            label={item.title}
            onList={todoFound !== undefined}
            atLimit={actions.todo.atLimit}
            limit={actions.todo.limit}
            disabled={actions.todo.buttonDisabled}
            disabledHint={actions.todo.buttonHint}
            onAdd={() => void actions.todo.addGitlab(item.kind, item.reference)}
            onRemove={todoFound ? () => void actions.todo.remove(todoFound.id) : undefined}
          />
        )}
      </DataTableCell>
    </DataTableRow>
  );
}

function GitlabTable({
  label,
  rows,
  now,
  whenHeader = "Zmiana",
  actions,
}: {
  label: string;
  rows: TableRow[];
  now: number;
  whenHeader?: string;
  actions?: RowActions;
}) {
  return (
    <DataTable label={label} className="gitlab-table">
      <DataTableRow head>
        <DataTableCell header>Typ</DataTableCell>
        <DataTableCell header tone="faint" className="gitlab-col-nr">
          Nr
        </DataTableCell>
        <DataTableCell header>Tytuł</DataTableCell>
        <DataTableCell header tone="muted" truncate className="gitlab-col-project">
          Projekt
        </DataTableCell>
        <DataTableCell header tone="faint" truncate className="gitlab-col-labels">
          Etykiety
        </DataTableCell>
        <DataTableCell header tone="faint" className="gitlab-col-when">
          {whenHeader}
        </DataTableCell>
        <DataTableCell header className="gitlab-col-actions">
          {actions ? "Akcje" : ""}
        </DataTableCell>
      </DataTableRow>
      {rows.map((row) => (
        <GitlabRow
          key={`${row.item.kind}:${row.item.reference}`}
          row={row}
          now={now}
          actions={actions}
        />
      ))}
    </DataTable>
  );
}

function filteredNote(count: number): string | undefined {
  return count > 0 ? `Ukryte filtrem: ${count}` : undefined;
}

function emptyText(kind: KindFilter, filterActive: boolean): string {
  if (kind === "issue") {
    return filterActive ? "Brak zgłoszeń pasujących do filtra." : "Brak przypisanych zgłoszeń.";
  }
  if (kind === "mergeRequest") {
    return filterActive
      ? "Brak merge requestów pasujących do filtra."
      : "Brak przypisanych merge requestów.";
  }
  return filterActive
    ? "Brak zgłoszeń i merge requestów pasujących do filtra."
    : "Brak przypisanych zgłoszeń i merge requestów.";
}

function ClosedBlock({
  closed,
  kind,
  now,
}: {
  closed: ClosedSection;
  kind: KindFilter;
  now: number;
}) {
  const oneDay = closed.days === 1;
  const items = closed.items.filter((i) => matchesKind(i, kind));
  return (
    <Panel
      title={oneDay ? "Zamknięte (ostatni dzień)" : `Zamknięte (ostatnie ${closed.days} dni)`}
      meta={closed.error ? undefined : items.length}
      footer={filteredNote(closed.filteredOut)}
    >
      {closed.error ? (
        <Notice tone="warn" inset>
          {closed.error}
        </Notice>
      ) : items.length === 0 ? (
        <p className="dn-empty">
          {oneDay
            ? "Brak zamkniętych w ostatnim dniu."
            : `Brak zamkniętych w ostatnich ${closed.days} dniach.`}
        </p>
      ) : (
        <GitlabTable
          label="Zamknięte w GitLabie"
          rows={items.map((item) => ({ item, at: item.closedAt }))}
          now={now}
          whenHeader="Zamknięte"
        />
      )}
    </Panel>
  );
}

/** UI-only list state, kept in the view so it survives the list being hidden offline. */
type ListView = {
  kind: KindFilter;
  setKind: (kind: KindFilter) => void;
  showHidden: boolean;
  setShowHidden: (update: (on: boolean) => boolean) => void;
};

function Lists({
  snapshot,
  stale,
  actions,
  now,
  view,
}: {
  snapshot: GitlabSnapshot & { items: NonNullable<GitlabSnapshot["items"]> };
  stale: boolean;
  actions: RowActions;
  now: number;
  view: ListView;
}) {
  const { kind, setKind, showHidden, setShowHidden } = view;
  const { items, filterActive, filteredOut, closed } = snapshot;

  const visible = [...items.mergeRequests, ...items.issues]
    .filter((i) => matchesKind(i, kind))
    .sort(byUpdated);
  const hidden = showHidden ? items.hiddenIssues.filter((i) => matchesKind(i, kind)) : [];
  const rows: TableRow[] = [
    ...visible.map((item) => ({ item })),
    ...hidden.map((item) => ({ item, hidden: true })),
  ];
  const outByFilter =
    (kind !== "issue" ? filteredOut.mergeRequests : 0) +
    (kind !== "mergeRequest" ? filteredOut.issues : 0) +
    (showHidden && kind !== "mergeRequest" ? filteredOut.hiddenIssues : 0);
  const hiddenCount = items.hiddenIssues.length;

  return (
    <div className={stale ? "gitlab-lists is-stale" : "gitlab-lists"}>
      <div className="gitlab-toolbar">
        <h2 className="dn-sr-only">Przypisane</h2>
        <span className="dn-spacer" />
        <Segmented
          label="Typ"
          options={KIND_OPTIONS}
          value={kind}
          onChange={(v) => setKind(v as KindFilter)}
        />
        <Button
          size="sm"
          variant="quiet"
          pressed={showHidden}
          disabled={hiddenCount === 0 && !showHidden}
          onClick={() => setShowHidden((s) => !s)}
        >
          Pokaż ukryte ({hiddenCount})
        </Button>
      </div>
      <div className="gitlab-main">
        <Panel footer={filteredNote(outByFilter)}>
          {rows.length === 0 ? (
            <p className="dn-empty">{emptyText(kind, filterActive)}</p>
          ) : (
            <GitlabTable label="Przypisane z GitLaba" rows={rows} now={now} actions={actions} />
          )}
        </Panel>
        <p className="meta gitlab-footnote">
          {/* Mirrors plan_round in src-tauri/src/hidden_issues/rules.rs. */}
          Ukryte zgłoszenie wraca samo, gdy zmieni się jego status (etykieta z „::”),{" "}
          {snapshot.ownCommentsKnown
            ? "ktoś inny doda komentarz"
            : "pojawi się nowy komentarz"}{" "}
          albo zgłoszenie wróci na listę po zniknięciu z niej.
        </p>
      </div>
      {closed && <ClosedBlock closed={closed} kind={kind} now={now} />}
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
  const todo = useTodoPicks();
  const [kind, setKind] = useState<KindFilter>("all");
  const [showHidden, setShowHidden] = useState(false);
  const now = useNow(RELATIVE_TICK_MS);

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
  // The tick can lag a fresh snapshot; never show a refresh "in the future".
  const nowMs = Math.max(now, lastSuccessMs ?? 0);
  const actions: RowActions = { todo, busy, hide, unhide, dismiss };
  // Filtered-out hidden issues are still checked for comments, so they count.
  const hasHidden = (items?.hiddenIssues.length ?? 0) + snapshot.filteredOut.hiddenIssues > 0;
  const lastSuccess =
    lastSuccessMs != null
      ? `ostatnie udane odświeżenie: ${formatAgo(lastSuccessMs, nowMs)}`
      : "brak udanego odświeżenia od uruchomienia aplikacji";
  const offline = state.state === "unavailable";

  let summary: string | null = null;
  if (offline) {
    summary = "źródło niedostępne";
  } else if (items && state.state !== "notConfigured") {
    const shown = items.mergeRequests.length + items.issues.length;
    const hid = items.hiddenIssues.length;
    summary = `${shown} ${plural(shown, "widoczne", "widoczne", "widocznych")} · ${hid} ${plural(hid, "ukryte", "ukryte", "ukrytych")}`;
  } else if (state.state === "error") {
    summary = "błąd źródła";
  }

  return (
    <main className="container">
      <div className="dn-view-head">
        <h1>Praca (GitLab)</h1>
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
        <SourceState kind="calm" title="GitLab nie jest skonfigurowany.">
          Dodaj{" "}
          <code>
            &quot;gitlab&quot;: {"{"} &quot;host&quot;: &quot;…&quot; {"}"}
          </code>{" "}
          do settings.json (opcjonalnie <code>&quot;user&quot;</code>: login, gdy glab jest
          zalogowany na konto serwisowe).
        </SourceState>
      )}
      {offline && (
        <SourceState
          kind="calm"
          title="GitLab niedostępny"
          lastOk={lastSuccess}
          action="Sprawdź teraz"
          onAction={() => void refresh()}
          actionDisabled={refreshing}
        >
          Brak połączenia z siecią firmową (VPN) albo serwer GitLaba nie odpowiada. To normalny stan
          poza pracą — aplikacja sprawdzi ponownie przy następnym automatycznym odświeżeniu.
        </SourceState>
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
        <Notice tone="warn">
          Nieprawidłowe ustawienie <code>gitlabRefresh.minutes</code> (dozwolone 1–14) — odświeżam
          co 10 min
        </Notice>
      )}
      {snapshot.filterInvalid && (
        <Notice tone="warn">
          Nieprawidłowy blok gitlabFilter w settings.json — pokazuję wszystkie przypisane
        </Notice>
      )}
      {snapshot.hiddenError && (
        <Notice tone="warn">
          Lista pokazuje wszystkie zgłoszenia, bo nie udało się odczytać ukrytych.{" "}
          {snapshot.hiddenError}
        </Notice>
      )}
      {hiddenCheckFailed > 0 && (
        <Notice tone="warn">
          Nie udało się sprawdzić {hiddenCheckFailed} ukrytych — spróbuję przy następnym odświeżeniu
        </Notice>
      )}
      {hasHidden && !ownCommentsKnown && (
        <Notice tone="warn">
          Ustaw <code>&quot;user&quot;</code> w bloku <code>&quot;gitlab&quot;</code> w
          settings.json, aby Twoje własne komentarze nie przywracały ukrytych zgłoszeń.
        </Notice>
      )}
      {actionError && <Notice tone="error">{actionError}</Notice>}
      <TodoPicksNotices todo={todo} />
      {items && state.state !== "notConfigured" && !offline && (
        <Lists
          snapshot={{ ...snapshot, items }}
          stale={state.state !== "ok"}
          actions={actions}
          now={nowMs}
          view={{ kind, setKind, showHidden, setShowHidden }}
        />
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
