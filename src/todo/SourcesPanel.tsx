import type { ReactNode } from "react";
import { Notice, Panel, RELATIVE_TICK_MS, formatAgo, useNow } from "../components";
import { ErrorState } from "../components/ErrorState";
import type { HealthSnapshot, SourceStatus } from "../health/types";
import { formatClock } from "../worktime/format";

const GLYPH: Record<SourceStatus["state"], string> = {
  ok: "●",
  unavailable: "○",
  notConfigured: "○",
  error: "▲",
};

const SUFFIX: Record<SourceStatus["state"], string | null> = {
  ok: null,
  unavailable: "niedostępny",
  notConfigured: "nie skonfigurowano",
  error: "błąd",
};

function detail(s: SourceStatus, lastSuccessMs: number | null, now: number): string | null {
  const parts: string[] = [];
  if (s.state === "error" || s.state === "unavailable") parts.push(s.message);
  if (lastSuccessMs !== null) {
    parts.push(
      s.state === "ok"
        ? `odświeżone ${formatAgo(lastSuccessMs, now)}`
        : `ostatnio ${formatClock(lastSuccessMs)}`,
    );
  }
  return parts.length > 0 ? parts.join(" · ") : null;
}

/** The mini "Stan źródeł" panel: one row per health source, read from the stored snapshot. */
export function SourcesPanel({
  snapshot,
  loadError,
  onRetry,
  gitlabLastSuccessMs,
  githubLastSuccessMs,
}: {
  snapshot: HealthSnapshot | null;
  loadError: string | null;
  onRetry: () => void;
  gitlabLastSuccessMs: number | null;
  githubLastSuccessMs: number | null;
}) {
  const now = useNow(RELATIVE_TICK_MS);

  let body: ReactNode;
  if (loadError !== null) {
    body = (
      <ErrorState title="Nie można odczytać stanu źródeł" message={loadError} onRetry={onRetry} />
    );
  } else if (!snapshot || snapshot.overall === "starting") {
    body = (
      <p className="dn-loading todo-group-note" role="status">
        Sprawdzam…
      </p>
    );
  } else if (snapshot.overall === "notRunning" && snapshot.checkedAtMs == null) {
    body = (
      <Notice tone="error" inset>
        Sprawdzanie stanu nie działa — pierwsze sprawdzenie źródeł nie zakończyło się.
      </Notice>
    );
  } else {
    body = (
      <>
        {snapshot.overall === "notRunning" && (
          <Notice tone="error" inset>
            Sprawdzanie stanu nie działa — wyniki mogą być nieaktualne.
          </Notice>
        )}
        <ul className="todo-sources">
          {snapshot.sources.map((s) => {
            const lastSuccessMs =
              s.id === "gitlab"
                ? gitlabLastSuccessMs
                : s.id === "github"
                  ? githubLastSuccessMs
                  : null;
            const text = detail(s, lastSuccessMs, now);
            const suffix = SUFFIX[s.state];
            return (
              <li key={s.id} className="todo-source">
                <span className={`todo-source-glyph is-${s.state}`} aria-hidden="true">
                  {GLYPH[s.state]}
                </span>
                <div className="todo-source-body">
                  <div>{suffix ? `${s.label} — ${suffix}` : s.label}</div>
                  {text && (
                    <div
                      className={
                        s.state === "error" ? "todo-source-detail is-error" : "todo-source-detail"
                      }
                    >
                      {text}
                    </div>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      </>
    );
  }

  return <Panel title="Stan źródeł">{body}</Panel>;
}
