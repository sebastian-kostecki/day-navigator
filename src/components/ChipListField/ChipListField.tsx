import { useId, useRef, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import "../Field/Field.css";
import { Icon } from "../Icon/Icon";
import "./ChipListField.css";

export type ChipListFieldProps = {
  /** Always visible; linked to the add input. */
  label: string;
  values: string[];
  onChange: (values: string[]) => void;
  placeholder?: string;
  /** The most entries the list takes. */
  max?: number;
  /** Message per position in `values`; the caller drops them when the list changes. */
  errors?: Record<number, string>;
  disabled?: boolean;
};

/**
 * A list of strings as removable chips with an add input. Entries are only
 * trimmed here; every other rule belongs to Rust.
 */
export function ChipListField({
  label,
  values,
  onChange,
  placeholder,
  max,
  errors,
  disabled,
}: ChipListFieldProps) {
  const id = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const [draft, setDraft] = useState("");
  // Entries the last add left out because of `max`.
  const [skipped, setSkipped] = useState(0);

  /** Appends the new entries and returns how many did not fit. */
  function add(pieces: string[]): number {
    const next = [...values];
    let over = 0;
    for (const piece of pieces) {
      const entry = piece.trim();
      if (!entry || next.includes(entry)) continue;
      if (max !== undefined && next.length >= max) over += 1;
      else next.push(entry);
    }
    setSkipped(over);
    if (next.length !== values.length) onChange(next);
    return over;
  }

  function remove(index: number) {
    setSkipped(0);
    onChange(values.filter((_, i) => i !== index));
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.nativeEvent.isComposing) return;
    if (event.key === "Enter") {
      event.preventDefault();
      if (add([draft]) === 0) setDraft("");
    } else if (event.key === "Backspace" && draft === "" && !event.repeat && values.length > 0) {
      event.preventDefault();
      remove(values.length - 1);
    }
  }

  function onPaste(event: ClipboardEvent<HTMLInputElement>) {
    const text = event.clipboardData.getData("text");
    if (!/[\r\n,]/.test(text)) return;
    event.preventDefault();
    const { value, selectionStart, selectionEnd } = event.currentTarget;
    const start = selectionStart ?? value.length;
    const end = selectionEnd ?? value.length;
    add((value.slice(0, start) + text + value.slice(end)).split(/[\r\n,]+/));
    setDraft("");
  }

  const errorId = (index: number) => `${id}-error-${index}`;
  const badIds = values.flatMap((_, i) => (errors?.[i] ? [errorId(i)] : []));

  return (
    <div className="dn-chiplist">
      <label htmlFor={id} className="dn-field-label">
        {label}
      </label>
      {values.length > 0 && (
        <ul className="dn-chiplist-chips" aria-label={label}>
          {values.map((value, i) => {
            const error = errors?.[i];
            return (
              <li key={`${i}:${value}`} className="dn-chiplist-item">
                <span className={error ? "dn-chiplist-chip is-error" : "dn-chiplist-chip"}>
                  <span className="dn-chiplist-value">{value}</span>
                  <button
                    type="button"
                    className="dn-chiplist-remove"
                    aria-label={`Usuń ${value}`}
                    aria-describedby={error ? errorId(i) : undefined}
                    disabled={disabled}
                    onClick={() => {
                      remove(i);
                      // The button goes away with its chip; keep focus in the field.
                      inputRef.current?.focus();
                    }}
                  >
                    <Icon name="close" size={12} />
                  </button>
                </span>
                {error && (
                  <span id={errorId(i)} className="dn-chiplist-error">
                    {error}
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
      <input
        ref={inputRef}
        id={id}
        type="text"
        className="dn-input"
        value={draft}
        placeholder={placeholder}
        disabled={disabled}
        aria-invalid={badIds.length > 0 || undefined}
        aria-describedby={badIds.length > 0 ? badIds.join(" ") : undefined}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={onKeyDown}
        onPaste={onPaste}
      />
      <span role="status" className="dn-chiplist-note">
        {skipped > 0 ? `Limit pozycji: ${max}. Pominięto: ${skipped}.` : ""}
      </span>
    </div>
  );
}
