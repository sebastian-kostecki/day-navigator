import { ErrorState } from "../components/ErrorState";
import type { SourceStatus } from "./types";
import { useHealth } from "./useHealth";

function stateText(s: SourceStatus): string {
  switch (s.state) {
    case "ok":
      return "działa";
    case "notConfigured":
      return "nie skonfigurowano";
    case "error":
      return "błąd";
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
      <h1>Stan źródeł</h1>
      {snapshot?.overall === "notRunning" && (
        <p className="notice">Sprawdzanie stanu nie działa.</p>
      )}
      {starting ? (
        <p>Sprawdzam…</p>
      ) : (
        <ul className="sources">
          {snapshot.sources.map((s) => (
            <li key={s.id} className={`source ${s.state}`}>
              <span className="source-label">{s.label}</span>
              <span className="source-state">{stateText(s)}</span>
              {s.state === "error" && <span className="source-message">{s.message}</span>}
            </li>
          ))}
        </ul>
      )}
      <div className="actions">
        <button type="button" onClick={checkNow}>
          Sprawdź teraz
        </button>
        {snapshot?.checkedAtMs != null && (
          <span className="checked-at">
            Ostatnie sprawdzenie: {formatTime(snapshot.checkedAtMs)}
          </span>
        )}
      </div>
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
