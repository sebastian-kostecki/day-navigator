import type { ReactNode } from "react";
import { Panel } from "../components";
import { ErrorState } from "../components/ErrorState";
import type { EventEntry } from "../events/types";
import { useEventLog } from "../events/useEventLog";
import { shortRef } from "./shortRef";

type Row = { label: string; subject: ReactNode; tone: "plain" | "error" | "ok" };

function subjectOf(reference: string | null, title: string): ReactNode {
  if (reference === null) return title;
  return (
    <>
      <span title={reference}>{shortRef(reference)}</span> — {title}
    </>
  );
}

function rowOf(e: EventEntry): Row {
  switch (e.kind) {
    case "assigned":
      return {
        label: "Nowe przypisanie",
        subject: subjectOf(e.reference, e.title),
        tone: "plain",
      };
    case "ticked":
      return { label: "Odhaczono", subject: subjectOf(e.reference, e.title), tone: "plain" };
    case "removed":
      return { label: "Zdjęto", subject: subjectOf(e.reference, e.title), tone: "plain" };
    case "closedInSource":
      return {
        label: "Zamknięte w źródle",
        subject: subjectOf(e.reference, e.title),
        tone: "plain",
      };
    case "workStarted":
      return { label: "START", subject: null, tone: "plain" };
    case "workStopped":
      return { label: "STOP", subject: null, tone: "plain" };
    case "sourceError":
      return { label: `${e.label}: błąd — ${e.reason}`, subject: null, tone: "error" };
    case "sourceRecovered":
      return { label: `${e.label}: znowu działa`, subject: null, tone: "ok" };
  }
}

function lastRefreshLabel(at: Date | null): string {
  if (at === null) return "Ostatnie udane odświeżenie: brak";
  return `Ostatnie udane odświeżenie: ${at.toLocaleTimeString("pl-PL")}`;
}

/** The "Dziennik zdarzeń" panel: today's events, newest first. */
export function EventLogPanel() {
  const { load, lastSuccessAt, reloadError, retry, refresh } = useEventLog();

  let body: ReactNode;
  let meta: ReactNode;
  let footer: ReactNode;
  if (load.state === "loading") {
    body = (
      <p className="dn-loading todo-group-note" role="status">
        Ładowanie…
      </p>
    );
  } else if (load.state === "error") {
    body = (
      <ErrorState
        title="Nie udało się wczytać dziennika"
        message={load.message}
        onRetry={retry}
        lastSuccess={lastRefreshLabel(lastSuccessAt)}
      />
    );
  } else {
    const { events, truncated } = load.log;
    meta = truncated ? "200+" : events.length;
    if (truncated) footer = "Pokazano 200 najnowszych";
    body = (
      <>
        {reloadError && (
          <ErrorState
            title="Nie udało się odświeżyć dziennika — może być nieaktualny"
            message={reloadError}
            onRetry={() => void refresh()}
            lastSuccess={lastRefreshLabel(lastSuccessAt)}
          />
        )}
        {events.length === 0 ? (
          <p className="dn-empty">Brak zdarzeń dzisiaj.</p>
        ) : (
          <ul className="todo-events">
            {events.map((e) => {
              const row = rowOf(e);
              const tone = row.tone === "plain" ? "" : ` is-${row.tone}`;
              return (
                <li key={e.id} className="todo-event">
                  <span className="todo-event-time">{e.at.slice(11, 16)}</span>
                  <div className="todo-event-body">
                    <div className={`todo-event-label${tone}`}>{row.label}</div>
                    {row.subject !== null && (
                      <div className="todo-event-subject">{row.subject}</div>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </>
    );
  }

  return (
    <Panel title="Dziennik zdarzeń" meta={meta} footer={footer}>
      {body}
    </Panel>
  );
}
