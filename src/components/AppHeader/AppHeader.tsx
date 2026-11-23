import type { ReactNode } from "react";
import "./AppHeader.css";

export type AppHeaderProps = {
  items: { id: string; label: string; active: boolean }[];
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
            onClick={() => onSelect(item.id)}
          >
            {item.label}
          </button>
        ))}
      </nav>
      {children}
    </header>
  );
}
