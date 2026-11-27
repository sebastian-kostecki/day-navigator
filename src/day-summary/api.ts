import { invoke } from "@tauri-apps/api/core";
import { storeErrorMessage } from "../components/storeError";
import type { DaySummary, DaySummaryError } from "./types";

export function daySummary(): Promise<DaySummary> {
  return invoke<DaySummary>("day_summary");
}

function asDaySummaryError(e: unknown): DaySummaryError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as DaySummaryError;
  }
  return null;
}

/** Polish message for anything thrown by the `day_summary` invoke call. */
export function daySummaryErrorMessage(e: unknown): string {
  const err = asDaySummaryError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeErrorMessage(err.detail);
    case "InvalidLocalTime":
      return "Zegar systemowy wskazuje czas, którego nie ma w strefie lokalnej — spróbuj za chwilę.";
    case "Background":
      return "Nie udało się wczytać podsumowania — spróbuj ponownie.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}
