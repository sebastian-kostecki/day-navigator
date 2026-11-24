import type { ReactNode } from "react";
import "./AppHeader.css";

export type AppHeaderProps = {
  /** key: the shortcut shown before the label ("1"…"6"), also set as aria-keyshortcuts. */
  items: { id: string; label: string; active: boolean; key?: string }[];
  onSelect: (id: string) => void;
  /** Right-hand slot (the work-time counter). */
  children?: ReactNode;
};

export function AppHeader({ items, onSelect, children }: AppHeaderProps) {
  return (
    <header className="dn-header">
      <span className="dn-brand">DAY/NAV</span>
      <nav className="dn-nav" aria-label="Widoki">
        {items.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`dn-nav-item${item.active ? " is-active" : ""}`}
            aria-current={item.active ? "page" : undefined}
            aria-keyshortcuts={item.key}
            data-view={item.id}
            onClick={() => onSelect(item.id)}
          >
            {item.key ? (
              <span className="dn-nav-key" aria-hidden="true">
                {item.key}
              </span>
            ) : null}
            {item.label}
          </button>
        ))}
      </nav>
      {children}
    </header>
  );
}
