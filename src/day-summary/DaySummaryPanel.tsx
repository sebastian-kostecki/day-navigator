import { useEffect } from "react";
import {
  Button,
  ItemRow,
  Panel,
  StatStrip,
  formatHm,
  type SourceKind,
  type Stat,
} from "../components";
import { ErrorState } from "../components/ErrorState";
import type { Space } from "../own-tasks/types";
import { shortRef } from "../todo/shortRef";
import type { TodoItem } from "../todo/types";
import { formatClock } from "../worktime/format";
import { useWorkTimer } from "../worktime/useWorkTimer";
import "./DaySummaryPanel.css";
import { useDaySummary } from "./useDaySummary";

/** Lowercase, for the ref column of an own task. */
const SPACE_REF: Record<Space, string> = { work: "praca", private: "prywatne" };

/** `YYYY-MM-DD` to `DD.MM` without parsing a date. */
function dayLabel(day: string): string {
  return `${day.slice(8, 10)}.${day.slice(5, 7)}`;
}

function SummaryRow({ item }: { item: TodoItem }) {
  const { entry } = item;
  const source: SourceKind = entry.kind === "own" ? "my" : entry.kind === "github" ? "gh" : "gl";
  const closed = entry.kind !== "own" && entry.closedInSource;
  return (
    <ItemRow
      source={source}
      refId={entry.kind === "own" ? SPACE_REF[entry.space] : shortRef(entry.reference)}
      title={<span title={entry.kind === "own" ? undefined : entry.reference}>{entry.title}</span>}
      flag={
        closed
          ? entry.kind === "gitlab"
            ? "zamknięte w GitLabie"
            : "zamknięte w GitHubie"
          : undefined
      }
      flagTone={closed ? "info" : undefined}
    />
  );
}

function List({ title, items, empty }: { title: string; items: TodoItem[]; empty: string }) {
  return (
    <div>
      <h3 className="dn-day-summary-head">{title}</h3>
      {items.length === 0 ? (
        <p className="dn-empty">{empty}</p>
      ) : (
        items.map((item) => <SummaryRow key={item.id} item={item} />)
      )}
    </div>
  );
}

export function DaySummaryPanel({ onClose }: { onClose: () => void }) {
  const { state, retry } = useDaySummary();
  const { timer } = useWorkTimer();
  const running = timer.state === "loaded" && timer.status.running !== null;

  // A later START (header or tray) removes a stale summary.
  useEffect(() => {
    if (running) onClose();
  }, [running, onClose]);

  const carryOver = state.state === "loaded" && state.summary.carryOver.length > 0;

  let body;
  if (state.state === "loading") {
    body = <p className="dn-empty">Wczytywanie podsumowania…</p>;
  } else if (state.state === "error") {
    body = (
      <ErrorState
        title="Podsumowanie dnia niedostępne"
        message={state.message}
        onRetry={retry}
        lastSuccess={state.lastOkAt !== null ? formatClock(state.lastOkAt) : undefined}
      />
    );
  } else {
    const { summary } = state;
    const stats: Stat[] = [
      { label: "Dziś", value: formatHm(summary.todayMs) },
      {
        label: "Odchylenie miesiąca",
        value: formatHm(summary.monthDeviationMs, true),
        sub: "do dziś",
        tone: summary.monthDeviationMs < 0 ? "warn" : "info",
      },
      { label: "Zrobione", value: String(summary.done.length) },
      { label: "Na jutro", value: String(summary.carryOver.length) },
    ];
    body = (
      <div className="dn-day-summary-body">
        <div role="status">
          <StatStrip items={stats} />
        </div>
        <List
          title="Zrobione dziś"
          items={summary.done}
          empty="Dziś nic nie odhaczono w „Do zrobienia”."
        />
        <List
          title="Przechodzi na jutro"
          items={summary.carryOver}
          empty="Nic nie przechodzi na jutro."
        />
      </div>
    );
  }

  return (
    <div className="container dn-day-summary">
      <Panel
        title="Podsumowanie dnia"
        meta={state.state === "loaded" ? dayLabel(state.summary.day) : undefined}
        footer={
          <div className="dn-day-summary-foot">
            <span>{carryOver ? "Nieodhaczone pozycje przejdą na jutro same." : null}</span>
            <Button onClick={onClose}>Zamknij</Button>
          </div>
        }
      >
        {body}
      </Panel>
    </div>
  );
}
