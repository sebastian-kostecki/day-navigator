import { invoke } from "@tauri-apps/api/core";
import type { HealthSnapshot, OpenBackupsError } from "./types";

/** The stored snapshot (`starting` before the first check). */
export function healthStatus(): Promise<HealthSnapshot> {
  return invoke<HealthSnapshot>("health_status");
}

/** Wakes the check thread; the result arrives as a `health-changed` event. */
export function checkHealthNow(): Promise<void> {
  return invoke<void>("check_health_now");
}

/** Opens the backups folder of the app data in the default program. */
export function openBackupsFolder(): Promise<void> {
  return invoke<void>("open_backups_folder");
}

/** Polish message for anything thrown by `openBackupsFolder`. */
export function openBackupsErrorMessage(e: unknown): string {
  if (typeof e === "object" && e !== null && "kind" in e) {
    const err = e as OpenBackupsError;
    switch (err.kind) {
      case "DataDirUnresolved":
        return "Nie można ustalić folderu danych aplikacji.";
      case "Missing":
        return "Folder kopii nie istnieje.";
      case "OpenFailed":
        return "Nie udało się otworzyć folderu kopii.";
      default:
        return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
    }
  }
  return `Nieoczekiwany błąd: ${String(e)}`;
}
