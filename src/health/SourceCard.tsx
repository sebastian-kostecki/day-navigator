import { useId, type ReactNode } from "react";
import { StatusPill } from "../components";
import type { SourceStatus } from "./types";
import {
  GLYPH,
  cardMessage,
  pillState,
  pillText,
  toneOf,
  type CardSpec,
  type SourceFact,
} from "./sourceView";

type Props = {
  spec: CardSpec;
  /** Absent while the source has not reported yet (for example GitLab before its first round). */
  source: SourceStatus | undefined;
  /** Only facts backed by data; an empty list renders no `<dl>`. */
  facts: SourceFact[];
  /** Optional controls under the facts (for example a button). */
  actions?: ReactNode;
};

/** A big source card: head with glyph, name, kind and pill; message; facts. */
export function SourceCard({ spec, source, facts, actions }: Props) {
  const headingId = useId();
  const tone = source ? toneOf(source) : "calm";
  const message = source
    ? cardMessage(source)
    : { text: "Brak danych — źródło nie zgłosiło jeszcze stanu." };

  return (
    <section className={`health-card is-${tone}`} aria-labelledby={headingId}>
      <div className="health-card-head">
        <span className={`health-glyph health-card-glyph is-${tone}`} aria-hidden="true">
          {GLYPH[tone]}
        </span>
        <div className="health-card-title">
          <h2 id={headingId} className="health-card-name">
            {source?.label ?? spec.fallbackName}
          </h2>
          <div className="health-card-kind">{spec.kindLabel}</div>
        </div>
        {source ? (
          <StatusPill state={pillState(source)} glyph={false}>
            {pillText(source)}
          </StatusPill>
        ) : null}
      </div>
      <p className="health-card-message">{message.text}</p>
      {message.note ? <p className="health-card-note">{message.note}</p> : null}
      {facts.length > 0 ? (
        <dl className="health-card-facts">
          {facts.map((f) => (
            <div key={f.label} className="health-card-fact">
              <dt>{f.label}</dt>
              <dd className={f.tone === "warn" ? "is-warn" : undefined}>{f.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {actions ? <div className="health-card-actions">{actions}</div> : null}
    </section>
  );
}
