import { useTransition } from "react";
import {
  Button,
  Panel,
  RELATIVE_TICK_MS,
  StatusPill,
  formatAgo,
  useNow,
  type PillState,
} from "../components";
import { ErrorState } from "../components/ErrorState";
import { OpenBackupsButton } from "./OpenBackupsButton";
import { SourceCard, type SourceFact } from "./SourceCard";
import {
  CARD_IDS,
  appDataFacts,
  CARD_SOURCES,
  GLYPH,
  pillState,
  pillText,
  rowMessage,
  summaryText,
  toneOf,
} from "./sourceView";
import type { SourceStatus } from "./types";
import { useHealth } from "./useHealth";
import "./Health.css";

const LEGEND: { state: PillState; text: string; label: string }[] = [
  { state: "ok", label: "OK", text: "Źródło działa i odpowiada." },
  {
    state: "calm",
    label: "NIEDOSTĘPNY",
    text: "Spokojny stan, np. brak VPN do GitLaba. Bez alarmu i bez powiadomień.",
  },
  {
    state: "calm",
    label: "NIESKONFIGUROWANE",
    text: "Brak wpisu w settings.json — źródło jest pominięte.",
  },
  {
    state: "error",
    label: "BŁĄD",
    text: "Wymaga działania, np. wylogowane CLI, uszkodzona baza, błędne ustawienia. Czerwona ikona w zasobniku i powiadomienie.",
  },
];

function SourceRow({ source }: { source: SourceStatus }) {
  const tone = toneOf(source);
  const message = rowMessage(source);
  return (
    <li className="health-row">
      <span className={`health-glyph is-${tone}`} aria-hidden="true">
        {GLYPH[tone]}
      </span>
      <span className="health-row-name">{source.label}</span>
      <StatusPill state={pillState(source)} glyph={false}>
        {pillText(source)}
      </StatusPill>
      {message ? (
        <span className={`meta health-row-message${tone === "error" ? " is-error" : ""}`}>
          {message}
        </span>
      ) : null}
    </li>
  );
}

export function HealthPanel() {
  const { snapshot, loadError, checkError, checkNow, retry } = useHealth();
  const now = useNow(RELATIVE_TICK_MS);
  const [checking, startCheck] = useTransition();

  const runCheck = () => startCheck(() => checkNow());

  if (loadError) {
    return (
      <main className="container">
        <h1>Stan źródeł</h1>
        <ErrorState title="Nie można odczytać stanu źródeł" message={loadError} onRetry={retry} />
      </main>
    );
  }

  // Stalled before the first check finished: there are no rows to show, so
  // the failure takes their place instead of an empty list.
  if (snapshot?.overall === "notRunning" && snapshot.checkedAtMs == null) {
    return (
      <main className="container">
        <h1>Stan źródeł</h1>
        <ErrorState
          title="Sprawdzanie stanu nie działa"
          message={checkError ?? "Pierwsze sprawdzenie źródeł nie zakończyło się."}
          onRetry={runCheck}
        />
      </main>
    );
  }

  const starting = !snapshot || snapshot.overall === "starting";
  const checkedAtMs = snapshot?.checkedAtMs ?? null;
  const lastCheck =
    checkedAtMs != null ? `Ostatnie sprawdzenie: ${formatAgo(checkedAtMs, now)}` : undefined;

  const appData = snapshot?.appData ?? null;

  const cardFacts = (id: string): SourceFact[] => {
    if (id !== "app-data") return [];
    // The store is opened and checked by the health round itself, so the
    // round's time is this source's check time. GitLab and GitHub are read
    // from their own refresh rounds, whose time is not in the snapshot.
    const facts: SourceFact[] =
      checkedAtMs != null ? [{ label: "Sprawdzono", value: formatAgo(checkedAtMs, now) }] : [];
    return appData ? [...facts, ...appDataFacts(appData, now)] : facts;
  };

  const others = snapshot?.sources.filter((s) => !CARD_IDS.has(s.id)) ?? [];

  return (
    <main className="container">
      <div className="dn-view-head">
        <h1>Stan źródeł</h1>
        {!starting ? (
          <span className="dn-view-head-summary">{summaryText(snapshot.sources)}</span>
        ) : null}
        <div className="dn-spacer" />
        {checkedAtMs != null ? (
          <span className="dn-view-head-note">sprawdzone {formatAgo(checkedAtMs, now)}</span>
        ) : null}
        <Button
          size="sm"
          variant="quiet"
          icon="refresh"
          onClick={checking ? undefined : runCheck}
          aria-disabled={checking || undefined}
          aria-busy={checking || undefined}
        >
          Sprawdź wszystkie
        </Button>
      </div>
      {snapshot?.overall === "notRunning" && (
        <ErrorState
          title="Sprawdzanie stanu nie działa."
          message="Wyniki poniżej mogą być nieaktualne."
          onRetry={runCheck}
          lastSuccess={lastCheck}
        />
      )}
      {checkError && (
        <ErrorState
          title="Nie udało się uruchomić sprawdzenia"
          message={checkError}
          onRetry={runCheck}
          lastSuccess={lastCheck}
        />
      )}
      {starting ? (
        <p className="dn-loading" role="status">
          Sprawdzam…
        </p>
      ) : (
        <>
          <div className="health-cards">
            {CARD_SOURCES.map((spec) => (
              <SourceCard
                key={spec.id}
                spec={spec}
                source={snapshot.sources.find((s) => s.id === spec.id)}
                facts={cardFacts(spec.id)}
                actions={spec.id === "app-data" && appData ? <OpenBackupsButton /> : undefined}
              />
            ))}
          </div>
          <div className="dn-columns">
            <div className="dn-col-main">
              <Panel title="Pozostałe źródła" meta={others.length}>
                {others.length > 0 ? (
                  <ul className="health-rows">
                    {others.map((s) => (
                      <SourceRow key={s.id} source={s} />
                    ))}
                  </ul>
                ) : (
                  <p className="dn-empty">Brak innych źródeł w ostatnim sprawdzeniu.</p>
                )}
              </Panel>
            </div>
            <aside className="dn-col-side" aria-label="Co znaczą stany">
              <Panel title="Co znaczą stany">
                <dl className="health-legend">
                  {LEGEND.map((g) => (
                    <div key={g.label} className="health-legend-row">
                      <dt>
                        <StatusPill state={g.state} glyph={false}>
                          {g.label}
                        </StatusPill>
                      </dt>
                      <dd>{g.text}</dd>
                    </div>
                  ))}
                </dl>
              </Panel>
            </aside>
          </div>
        </>
      )}
    </main>
  );
}
