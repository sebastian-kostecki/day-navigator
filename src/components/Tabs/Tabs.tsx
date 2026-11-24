import { useRef, type KeyboardEvent } from "react";
import "./Tabs.css";

export type TabsProps<T extends string> = {
  tabs: { value: T; label: string; count?: number }[];
  value: T;
  onChange: (value: T) => void;
  /** Accessible name of the tab group. */
  label: string;
  /** Id of the tab panel these tabs control. */
  panelId: string;
  idPrefix: string;
};

/** A tablist with roving focus: arrows, Home and End move and select. */
export function Tabs<T extends string>({
  tabs,
  value,
  onChange,
  label,
  panelId,
  idPrefix,
}: TabsProps<T>) {
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
    <div role="tablist" aria-label={label} className="dn-tabs">
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
            className={on ? "dn-tab is-on" : "dn-tab"}
            onClick={() => onChange(t.value)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {t.label}
            {t.count !== undefined && <span className="dn-tab-count">{t.count}</span>}
          </button>
        );
      })}
    </div>
  );
}
