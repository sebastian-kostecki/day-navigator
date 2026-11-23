import { useState } from "react";
import type { OwnTaskInput, Priority, Space } from "./types";

type Props = {
  heading?: string;
  initial: OwnTaskInput;
  submitLabel: string;
  /** Resolves to an error message to show inline, or `null` on success. */
  onSubmit: (input: OwnTaskInput) => Promise<string | null>;
  /** Clear the fields after a successful submit (add form). */
  resetOnSuccess?: boolean;
  onCancel?: () => void;
};

const TITLE_MAX = 200;
const NOTE_MAX = 2000;

// No `maxLength` on purpose: the browser would silently cut pasted text and
// counts UTF-16 units, not characters. Rust is the only limit; the counters
// below count characters the same way.
const charCount = (value: string) => [...value].length;

export function OwnTaskForm({
  heading,
  initial,
  submitLabel,
  onSubmit,
  resetOnSuccess = false,
  onCancel,
}: Props) {
  const [space, setSpace] = useState<Space>(initial.space);
  const [title, setTitle] = useState(initial.title);
  const [note, setNote] = useState(initial.note);
  const [due, setDue] = useState(initial.due ?? "");
  const [priority, setPriority] = useState<Priority | "">(initial.priority ?? "");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setPending(true);
    const result = await onSubmit({
      space,
      title,
      note,
      due: due || null,
      priority: priority || null,
    });
    setPending(false);
    if (result !== null) {
      // Keep the typed input so the user can fix it.
      setError(result);
    } else if (resetOnSuccess) {
      setTitle("");
      setNote("");
      setDue("");
      setPriority("");
    }
  }

  return (
    <form className="own-task-form" onSubmit={submit}>
      {heading && <h2>{heading}</h2>}
      <fieldset disabled={pending}>
        <input
          type="text"
          className="own-task-title-input"
          value={title}
          onChange={(e) => setTitle(e.currentTarget.value)}
          placeholder="Tytuł"
          aria-label="Tytuł"
        />
        <span className="own-task-counter">
          {charCount(title)}/{TITLE_MAX}
        </span>
        <textarea
          className="own-task-note-input"
          value={note}
          onChange={(e) => setNote(e.currentTarget.value)}
          placeholder="Notatka (opcjonalnie)"
          aria-label="Notatka"
          rows={3}
        />
        <span className="own-task-counter">
          {charCount(note)}/{NOTE_MAX}
        </span>
        <select
          value={space}
          onChange={(e) => setSpace(e.currentTarget.value as Space)}
          aria-label="Przestrzeń"
        >
          <option value="work">Praca</option>
          <option value="private">Prywatne</option>
        </select>
        <input
          type="date"
          value={due}
          onChange={(e) => setDue(e.currentTarget.value)}
          aria-label="Termin"
        />
        <select
          value={priority}
          onChange={(e) => setPriority(e.currentTarget.value as Priority | "")}
          aria-label="Priorytet"
        >
          <option value="">—</option>
          <option value="high">Wysoki</option>
          <option value="medium">Średni</option>
          <option value="low">Niski</option>
        </select>
        <button type="submit">{submitLabel}</button>
        {onCancel && (
          <button type="button" onClick={onCancel}>
            Anuluj
          </button>
        )}
      </fieldset>
      {error && (
        <p className="own-task-error" role="alert">
          {error}
        </p>
      )}
    </form>
  );
}
