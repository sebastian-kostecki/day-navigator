import { Button, Panel, StatusPill, type PillState } from "../components";
import { ErrorState } from "../components/ErrorState";
import type { SourceStatus } from "./types";
import { useHealth } from "./useHealth";
import "./Health.css";

function pillState(s: SourceStatus): PillState {
  switch (s.state) {
    case "ok":
      return "ok";
    case "notConfigured":
    case "unavailable":
      return "calm";
    case "error":
      return "error";
  }
}

function pillText(s: SourceStatus): string {
  switch (s.state) {
    case "ok":
      return "OK";
    case "notConfigured":
      return "NIESKONFIGUROWANE";
    case "unavailable":
      return "NIEDOSTĘPNY";
    case "error":
      return "BŁĄD";
  }
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL");
}

export function HealthPanel() {
  const { snapshot, loadError, checkError, checkNow, retry } = useHealth();

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
          onRetry={checkNow}
        />
      </main>
    );
  }

  const starting = !snapshot || snapshot.overall === "starting";

  return (
    <main className="container">
      <div className="dn-view-head">
        <h1>Stan źródeł</h1>
        {snapshot?.checkedAtMs != null && (
          <span className="meta health-checked-at">
            Ostatnie sprawdzenie: {formatTime(snapshot.checkedAtMs)}
          </span>
        )}
        <Button size="sm" onClick={checkNow}>
          Sprawdź teraz
        </Button>
      </div>
      {snapshot?.overall === "notRunning" && (
        <ErrorState
          title="Sprawdzanie stanu nie działa."
          message="Wyniki poniżej mogą być nieaktualne."
          onRetry={checkNow}
          lastSuccess={
            snapshot.checkedAtMs != null
              ? `Ostatnie sprawdzenie: ${formatTime(snapshot.checkedAtMs)}`
              : undefined
          }
        />
      )}
      {starting ? (
        <p className="dn-loading">Sprawdzam…</p>
      ) : (
        <Panel title="Źródła" meta={snapshot.sources.length}>
          <ul className="health-sources">
            {snapshot.sources.map((s) => (
              <li key={s.id} className="health-source">
                <span className="health-source-label">{s.label}</span>
                <StatusPill state={pillState(s)}>{pillText(s)}</StatusPill>
                {(s.state === "error" || s.state === "unavailable") && (
                  <span
                    className={
                      s.state === "error"
                        ? "meta health-source-message is-error"
                        : "meta health-source-message"
                    }
                  >
                    {s.message}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </Panel>
      )}
      {checkError && (
        <ErrorState
          title="Nie udało się uruchomić sprawdzenia"
          message={checkError}
          onRetry={checkNow}
        />
      )}
    </main>
  );
}
