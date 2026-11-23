import { invoke } from "@tauri-apps/api/core";
import type { OwnTask, OwnTaskError, OwnTaskInput, Space, StoreError } from "./types";

export function listOwnTasks(): Promise<OwnTask[]> {
  return invoke<OwnTask[]>("list_own_tasks");
}

export function addOwnTask(input: OwnTaskInput): Promise<OwnTask> {
  return invoke<OwnTask>("add_own_task", { input });
}

export function editOwnTask(id: number, input: OwnTaskInput): Promise<OwnTask> {
  return invoke<OwnTask>("edit_own_task", { id, input });
}

export function setOwnTaskDone(id: number, done: boolean): Promise<OwnTask> {
  return invoke<OwnTask>("set_own_task_done", { id, done });
}

export function ownTaskDefaultSpace(): Promise<Space> {
  return invoke<Space>("own_task_default_space");
}

function asOwnTaskError(e: unknown): OwnTaskError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as OwnTaskError;
  }
  return null;
}

/** True when the task the user acted on no longer exists, so the list is stale. */
export function isOwnTaskNotFound(e: unknown): boolean {
  return asOwnTaskError(e)?.kind === "NotFound";
}

function storeErrorMessage(e: StoreError): string {
  switch (e.kind) {
    case "DirUnavailable":
      return "Dane aplikacji: folder danych jest niedostępny.";
    case "Damaged":
      return "Dane aplikacji: plik danych jest uszkodzony.";
    case "Missing":
      return "Dane aplikacji: brakuje pliku danych, chociaż istnieją kopie zapasowe.";
    case "NewerSchema":
      return "Dane aplikacji pochodzą z nowszej wersji aplikacji.";
    case "MigrationFailed":
      return "Dane aplikacji: nie udało się zaktualizować struktury danych.";
    case "Busy":
      return "Dane aplikacji są chwilowo zajęte — spróbuj ponownie.";
    case "Io":
      return "Dane aplikacji: błąd odczytu lub zapisu.";
    case "Sql":
      return "Dane aplikacji: błąd bazy danych.";
    default:
      return "Dane aplikacji: nieoczekiwany błąd.";
  }
}

/** Polish message for anything thrown by an own-task invoke call. */
export function ownTaskErrorMessage(e: unknown): string {
  const err = asOwnTaskError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeErrorMessage(err.detail);
    case "TitleEmpty":
      return "Tytuł jest wymagany.";
    case "TitleTooLong":
      return `Tytuł może mieć najwyżej ${err.detail.max} znaków.`;
    case "TitleMultiline":
      return "Tytuł musi być w jednej linii.";
    case "NoteTooLong":
      return `Notatka może mieć najwyżej ${err.detail.max} znaków.`;
    case "InvalidDue":
      return "Nieprawidłowa data terminu.";
    case "NotFound":
      return "Zadanie nie istnieje — odśwież listę.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}
