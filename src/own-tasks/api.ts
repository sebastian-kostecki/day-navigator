import { invoke } from "@tauri-apps/api/core";
import { storeErrorMessage } from "../components/storeError";
import type { OwnTask, OwnTaskError, OwnTaskInput, Space } from "./types";

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
