import { useRef, type KeyboardEvent } from "react";
import "./OwnTasks.css";

export type OwnTaskTab = "work" | "private" | "done";

type Props = {
  tabs: { value: OwnTaskTab; label: string; count: number }[];
  value: OwnTaskTab;
  onChange: (value: OwnTaskTab) => void;
  /** Id of the tab panel these tabs control. */
  panelId: string;
  idPrefix: string;
};

/** A tablist with roving focus: arrows, Home and End move and select. */
export function OwnTaskTabs({ tabs, value, onChange, panelId, idPrefix }: Props) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);

  function onKeyDown(e: KeyboardEvent, index: number) {
    let next: number;
    if (e.key === "ArrowRight") next = (index + 1) % tabs.length;
    else if (e.key === "ArrowLeft") next = (index - 1 + tabs.length) % tabs.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = tabs.length - 1;
    else return;
    e.preventDefault();
    onChange(tabs[next].value);
    refs.current[next]?.focus();
  }

  return (
    <div role="tablist" aria-label="Filtr zadań" className="own-tabs">
      {tabs.map((t, i) => {
        const on = t.value === value;
        return (
          <button
            key={t.value}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            role="tab"
            id={`${idPrefix}-${t.value}`}
            aria-selected={on}
            aria-controls={panelId}
            tabIndex={on ? 0 : -1}
            className={on ? "own-tab is-on" : "own-tab"}
            onClick={() => onChange(t.value)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {t.label}
            <span className="own-tab-count">{t.count}</span>
          </button>
        );
      })}
    </div>
  );
}
