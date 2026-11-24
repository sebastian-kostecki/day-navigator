import { useState } from "react";
import { Button, Field, Segmented } from "../components";
import type { OwnTaskInput, Priority, Space } from "./types";
import "./OwnTasks.css";

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

const SPACE_OPTIONS: { value: Space; label: string }[] = [
  { value: "work", label: "Praca" },
  { value: "private", label: "Prywatne" },
];

// `Field` renders select options with the label as the value, so the
// priority select works on labels and these maps translate to the domain value.
const NO_PRIORITY = "—";
const PRIORITY_LABELS: Record<Priority, string> = {
  high: "Wysoki",
  medium: "Średni",
  low: "Niski",
};
const PRIORITY_OPTIONS = [NO_PRIORITY, ...Object.values(PRIORITY_LABELS)];

function priorityFromLabel(label: string): Priority | "" {
  const found = (Object.keys(PRIORITY_LABELS) as Priority[]).find(
    (p) => PRIORITY_LABELS[p] === label,
  );
  return found ?? "";
}

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
    <form className="own-form" onSubmit={submit}>
      {heading && <h2 className="panel-title">{heading}</h2>}
      <fieldset className="own-form-fields" disabled={pending}>
        <div className="own-form-space">
          <span className="label-caps">Przestrzeń</span>
          <Segmented
            label="Przestrzeń"
            options={SPACE_OPTIONS}
            value={space}
            onChange={(value) => setSpace(value as Space)}
          />
        </div>
        <Field
          grow
          label="Tytuł"
          value={title}
          onChange={(e) => setTitle(e.currentTarget.value)}
          placeholder="Tytuł"
        />
        <Field
          label="Termin"
          type="date"
          value={due}
          onChange={(e) => setDue(e.currentTarget.value)}
        />
        <Field
          label="Priorytet"
          options={PRIORITY_OPTIONS}
          value={priority === "" ? NO_PRIORITY : PRIORITY_LABELS[priority]}
          onChange={(e) => setPriority(priorityFromLabel(e.currentTarget.value))}
        />
        <Field
          multiline
          className="own-form-note"
          label="Notatka"
          value={note}
          onChange={(e) => setNote(e.currentTarget.value)}
          placeholder="Notatka (opcjonalnie)"
          rows={3}
        />
        <p className="meta own-form-counters">
          <span>
            Tytuł {charCount(title)}/{TITLE_MAX}
          </span>
          <span>
            Notatka {charCount(note)}/{NOTE_MAX}
          </span>
        </p>
        <div className="own-form-actions">
          <Button type="submit" variant="primary">
            {submitLabel}
          </Button>
          {onCancel && <Button onClick={onCancel}>Anuluj</Button>}
        </div>
      </fieldset>
      {error && (
        <p className="own-error" role="alert">
          {error}
        </p>
      )}
    </form>
  );
}
