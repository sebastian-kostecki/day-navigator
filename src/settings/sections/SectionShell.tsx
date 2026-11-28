import type { FormEvent, ReactNode } from "react";
import { Button, Notice } from "../../components";
import { unmappedErrors } from "../fieldErrors";
import type { SectionForms, SectionId } from "../types";
import type { SectionState } from "../useSettings";

type Props<K extends SectionId> = {
  id: K;
  section: SectionState<SectionForms[K]>;
  hidden: boolean;
  /** Some section is being saved or reloaded. */
  busy: boolean;
  onSave: (replaceInvalid: boolean) => void;
  onReload: () => void;
  onCancelReplace: () => void;
  children: ReactNode;
};

/** One section: its messages on top, the inputs, and its own "Zapisz". */
export function SectionShell<K extends SectionId>({
  id,
  section,
  hidden,
  busy,
  onSave,
  onReload,
  onCancelReplace,
  children,
}: Props<K>) {
  const { form, invalidBlocks, failure, fieldErrors, pending, saved, confirmReplace } = section;
  const unmapped = unmappedErrors(id, form, fieldErrors);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    // While asking, only the question's own buttons decide.
    if (!confirmReplace) onSave(false);
  };

  return (
    <form className="dn-settings-section" hidden={hidden} onSubmit={submit} noValidate>
      {invalidBlocks.length > 0 && (
        <Notice
          tone="warn"
          compact
          action="Wczytaj ponownie"
          actionDisabled={busy}
          onAction={onReload}
        >
          Nieprawidłowy blok w settings.json: {invalidBlocks.join(", ")}. Pola pokazują dla niego
          wartości domyślne.
          {id === "notices" && " Do czasu naprawy wszystkie powiadomienia są włączone."}
        </Notice>
      )}
      {failure && (
        <Notice
          tone={failure.reload ? "warn" : "error"}
          role="alert"
          compact
          action={failure.reload ? "Wczytaj ponownie" : undefined}
          actionDisabled={busy}
          onAction={onReload}
        >
          {failure.message}
        </Notice>
      )}
      {unmapped.map((e, i) => (
        <Notice key={i} tone="error" compact role={null}>
          {e.field}
          {e.index === null ? "" : ` (pozycja ${e.index + 1})`}: {e.message}
        </Notice>
      ))}
      <div className="dn-settings-scroll">
        <fieldset className="dn-settings-fields" disabled={pending}>
          {children}
        </fieldset>
      </div>
      {confirmReplace && (
        <Notice tone="warn" compact role="alert">
          Zastąpić nieprawidłowy blok? Obecna zawartość w pliku przepadnie:{" "}
          {invalidBlocks.join(", ")}.
        </Notice>
      )}
      <div className="dn-settings-actions">
        {confirmReplace ? (
          // "Wróć" sits where "Zapisz" was, so a double click replaces nothing.
          <>
            <Button key="back" autoFocus onClick={onCancelReplace}>
              Wróć
            </Button>
            <Button key="replace" disabled={busy} onClick={() => onSave(true)}>
              Zastąp
            </Button>
          </>
        ) : (
          <Button key="save" type="submit" variant="primary" disabled={busy}>
            {pending ? "Zapisywanie…" : "Zapisz"}
          </Button>
        )}
        {/* Always mounted, so the live region announces "Zapisano." when it appears. */}
        <div role="status">
          {saved ? (
            <Notice tone="info" compact role={null}>
              Zapisano.
            </Notice>
          ) : null}
        </div>
      </div>
    </form>
  );
}
