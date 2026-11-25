import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import "./MultiSelect.css";

export type MultiSelectOption = { value: string; count: number; adornment?: ReactNode };

export type MultiSelectProps = {
  /** Button text and accessible name of the panel. */
  label: string;
  options: MultiSelectOption[];
  selected: readonly string[];
  onToggle: (value: string) => void;
  /** Hint on the disabled button when there are no options. */
  emptyText?: string;
};

/** A button with the number of chosen values that opens a list of checkboxes with counts. */
export function MultiSelect({ label, options, selected, onToggle, emptyText }: MultiSelectProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const panelId = useId();
  const disabled = options.length === 0;
  const isOpen = open && !disabled;

  useEffect(() => {
    if (!isOpen) return;
    const onPointerDown = (event: PointerEvent) => {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      setOpen(false);
      buttonRef.current?.focus();
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen]);

  return (
    <div className="dn-multiselect" ref={rootRef}>
      <button
        ref={buttonRef}
        type="button"
        className={selected.length > 0 ? "dn-multiselect-btn is-active" : "dn-multiselect-btn"}
        aria-haspopup="true"
        aria-expanded={isOpen}
        aria-controls={isOpen ? panelId : undefined}
        disabled={disabled}
        title={disabled ? emptyText : undefined}
        onClick={() => setOpen((o) => !o)}
      >
        {selected.length > 0 ? `${label} (${selected.length})` : label}
      </button>
      {isOpen && (
        <div className="dn-multiselect-panel" id={panelId} role="group" aria-label={label}>
          {options.map((o) => (
            <label key={o.value} className="dn-multiselect-option">
              <input
                type="checkbox"
                checked={selected.includes(o.value)}
                onChange={() => onToggle(o.value)}
              />
              <span className="dn-multiselect-value">{o.adornment ?? o.value}</span>
              <span className="meta">{o.count}</span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
}
