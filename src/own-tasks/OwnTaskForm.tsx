import { useState } from "react";
import { Button, Field, Notice, Segmented } from "../components";
import { PRIORITY_OPTIONS, SPACE_OPTIONS, priorityFromLabel, priorityToLabel } from "./labels";
import type { OwnTaskInput, Priority, Space } from "./types";
import "./OwnTasks.css";

type Props = {
  /** The preselected space; changing it here never resets it after an add. */
  initialSpace: Space;
  /** Why the space is preselected, shown before the hint; omitted when unknown. */
  spaceReason?: string;
  /** Resolves to an error message to show inline, or `null` on success. */
  onSubmit: (input: OwnTaskInput) => Promise<string | null>;
};

/** The quick-add bar: a title is enough; the note is set later in Edycja. */
export function OwnTaskForm({ initialSpace, spaceReason, onSubmit }: Props) {
  const [space, setSpace] = useState<Space>(initialSpace);
  const [title, setTitle] = useState("");
  const [due, setDue] = useState("");
  const [priority, setPriority] = useState<Priority | "">("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setPending(true);
    const result = await onSubmit({
      space,
      title,
      note: "",
      due: due || null,
      priority: priority || null,
    });
    setPending(false);
    if (result !== null) {
      // Keep the typed input so the user can fix it.
      setError(result);
    } else {
      // The space stays: several tasks in a row usually go to the same one.
      setTitle("");
      setDue("");
      setPriority("");
    }
  }

  const hint = [spaceReason, "Termin i priorytet są opcjonalne."].filter(Boolean).join(" ");

  return (
    <form className="own-quick" aria-label="Dodaj zadanie" onSubmit={submit}>
      <fieldset className="own-quick-fields" disabled={pending}>
        <Field
          grow
          label="Nowe zadanie"
          value={title}
          onChange={(e) => setTitle(e.currentTarget.value)}
          placeholder="Tytuł — wystarczy sam tytuł"
        />
        <div className="own-space-field">
          {/* The group itself is named by Segmented's aria-label. */}
          <span className="label-caps" aria-hidden="true">
            Przestrzeń
          </span>
          <Segmented
            label="Przestrzeń"
            size="md"
            options={SPACE_OPTIONS}
            value={space}
            onChange={(value) => setSpace(value as Space)}
          />
        </div>
        <Field
          className="own-quick-due"
          label="Termin"
          type="date"
          value={due}
          onChange={(e) => setDue(e.currentTarget.value)}
        />
        <Field
          className="own-quick-prio"
          label="Priorytet"
          options={PRIORITY_OPTIONS}
          value={priorityToLabel(priority)}
          onChange={(e) => setPriority(priorityFromLabel(e.currentTarget.value))}
        />
        <Button type="submit" variant="primary">
          Dodaj
        </Button>
      </fieldset>
      <p className="meta own-hint">{hint}</p>
      {error && (
        <Notice tone="error" compact>
          {error}
        </Notice>
      )}
    </form>
  );
}
