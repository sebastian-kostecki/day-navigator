import { useState } from "react";
import { Button } from "../components";
import { openBackupsErrorMessage, openBackupsFolder } from "./api";

/** Opens the backups folder; a failure shows inline until the next successful try. */
export function OpenBackupsButton() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const open = () => {
    if (busy) return;
    setBusy(true);
    openBackupsFolder()
      .then(() => setError(null))
      .catch((e: unknown) => setError(openBackupsErrorMessage(e)))
      .finally(() => setBusy(false));
  };

  return (
    <>
      <Button
        size="sm"
        variant="quiet"
        icon="folder"
        onClick={open}
        aria-disabled={busy || undefined}
        aria-busy={busy || undefined}
      >
        Otwórz folder kopii
      </Button>
      {error ? (
        <p className="health-card-note is-error" role="alert">
          {error}
        </p>
      ) : null}
    </>
  );
}
