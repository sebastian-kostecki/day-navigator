import { useId, useState } from "react";
import { Button, Field, Segmented } from "../components";
import {
  NOTE_MAX,
  PRIORITY_OPTIONS,
  SPACE_OPTIONS,
  TITLE_MAX,
  charCount,
  priorityFromLabel,
  priorityToLabel,
} from "./labels";
import type { OwnTask, OwnTaskInput, Priority, Space } from "./types";
import "./OwnTasks.css";

type Props = {
  /** Mount with `key={task.id}`: the fields start from this task. */
  task: OwnTask;
  /** Resolves to an error message to show inline, or `null` on success. */
  onSave: (input: OwnTaskInput) => Promise<string | null>;
  /** The same done / reopen action as the row's checkbox. */
  onToggleDone: () => void;
  toggleBusy: boolean;
};

/** The Edycja form for the selected task; edit replaces every field, space included. */
export function OwnTaskEditPanel({ task, onSave, onToggleDone, toggleBusy }: Props) {
  const [space, setSpace] = useState<Space>(task.space);
  const [title, setTitle] = useState(task.title);
  const [note, setNote] = useState(task.note);
  const [due, setDue] = useState(task.due ?? "");
  const [priority, setPriority] = useState<Priority | "">(task.priority ?? "");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const titleCountId = useId();
  const noteCountId = useId();
  const isDone = task.doneAt !== null;

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSaved(false);
    setPending(true);
    const result = await onSave({
      space,
      title,
      note,
      due: due || null,
      priority: priority || null,
    });
    setPending(false);
    // On failure the typed input stays so the user can fix it.
    if (result !== null) setError(result);
    else setSaved(true);
  }

  return (
    <form className="own-edit" aria-label={`Edycja: ${task.title}`} onSubmit={submit}>
      <fieldset className="own-edit-fields" disabled={pending}>
        <div className="own-space-field">
          {/* The group itself is named by Segmented's aria-label. */}
          <span className="label-caps" aria-hidden="true">
            Przestrzeń
          </span>
          <Segmented
            label="Przestrzeń"
            options={SPACE_OPTIONS}
            value={space}
            onChange={(value) => {
              setSpace(value as Space);
              setSaved(false);
            }}
          />
        </div>
        <div className="own-edit-field">
          <Field
            label="Tytuł"
            value={title}
            onChange={(e) => {
              setTitle(e.currentTarget.value);
              setSaved(false);
            }}
            aria-describedby={titleCountId}
            autoFocus
          />
          <span id={titleCountId} className="meta own-counter">
            {charCount(title)}/{TITLE_MAX}
          </span>
        </div>
        <div className="own-edit-field">
          <Field
            multiline
            label="Notatka (bez formatowania)"
            value={note}
            onChange={(e) => {
              setNote(e.currentTarget.value);
              setSaved(false);
            }}
            aria-describedby={noteCountId}
            rows={4}
          />
          <span id={noteCountId} className="meta own-counter">
            {charCount(note)}/{NOTE_MAX}
          </span>
        </div>
        <div className="own-edit-pair">
          <Field
            label="Termin"
            type="date"
            value={due}
            onChange={(e) => {
              setDue(e.currentTarget.value);
              setSaved(false);
            }}
          />
          <Field
            label="Priorytet"
            options={PRIORITY_OPTIONS}
            value={priorityToLabel(priority)}
            onChange={(e) => {
              setPriority(priorityFromLabel(e.currentTarget.value));
              setSaved(false);
            }}
          />
        </div>
        <div className="own-edit-actions">
          <Button type="submit" variant="primary">
            Zapisz
          </Button>
          <Button onClick={onToggleDone} disabled={toggleBusy}>
            {isDone ? "Przywróć" : "Oznacz jako zrobione"}
          </Button>
          <span className="meta own-saved" role="status">
            {saved ? "Zapisano." : ""}
          </span>
        </div>
      </fieldset>
      {error && (
        <p className="own-error" role="alert">
          {error}
        </p>
      )}
      <p className="meta own-hint">
        Zadań się nie usuwa — dodane przez pomyłkę oznacz jako zrobione albo popraw. Odhaczenie tu i
        w „Do zrobienia” to ten sam stan.
      </p>
    </form>
  );
}
