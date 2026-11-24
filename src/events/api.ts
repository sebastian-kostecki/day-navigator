import { invoke } from "@tauri-apps/api/core";
import { storeErrorMessage } from "../components/storeError";
import type { EventError, EventLog } from "./types";

export function listEventsToday(): Promise<EventLog> {
  return invoke<EventLog>("list_events_today");
}

function asEventError(e: unknown): EventError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as EventError;
  }
  return null;
}

/** Polish message for anything thrown by an event-log invoke call. */
export function eventErrorMessage(e: unknown): string {
  const err = asEventError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeErrorMessage(err.detail);
    case "Background":
      return "Zadanie w tle nie powiodło się — spróbuj ponownie.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}
