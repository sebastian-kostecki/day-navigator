import { invoke } from "@tauri-apps/api/core";
import { storeErrorMessage } from "../components/storeError";
import type { ItemKind } from "../gitlab/types";
import type { TodoError, TodoList } from "./types";

export function listTodo(): Promise<TodoList> {
  return invoke<TodoList>("list_todo");
}

export function addTodoOwnTask(taskId: number): Promise<TodoList> {
  return invoke<TodoList>("add_todo_own_task", { taskId });
}

export function addTodoGitlabItem(itemKind: ItemKind, reference: string): Promise<TodoList> {
  return invoke<TodoList>("add_todo_gitlab_item", { itemKind, reference });
}

export function setTodoTicked(id: number, ticked: boolean): Promise<TodoList> {
  return invoke<TodoList>("set_todo_ticked", { id, ticked });
}

export function removeTodo(id: number): Promise<TodoList> {
  return invoke<TodoList>("remove_todo", { id });
}

function asTodoError(e: unknown): TodoError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as TodoError;
  }
  return null;
}

/** True when the shown list no longer matches the stored one, so it must be reloaded. */
export function isTodoStale(e: unknown): boolean {
  const kind = asTodoError(e)?.kind;
  return (
    kind === "NotFound" ||
    kind === "AlreadyOnList" ||
    kind === "TaskDone" ||
    kind === "NotInGitlabList"
  );
}

/** Polish message for anything thrown by a "To do" invoke call or carried as `limitError`. */
export function todoErrorMessage(e: unknown): string {
  const err = asTodoError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeErrorMessage(err.detail);
    case "LimitReached":
      return `Limit ${err.detail.limit} otwartych pozycji — odhacz lub zdejmij coś, żeby dodać`;
    case "SettingsInvalid":
      return 'Nieprawidłowy blok "todo" w settings.json — oczekiwano "todo": { "limit": 1–20 }.';
    case "ConfigDirUnresolved":
      return "Nie można ustalić folderu konfiguracji aplikacji, więc limit nie jest znany.";
    case "NotInGitlabList":
    case "TaskDone":
    case "AlreadyOnList":
      return "Lista jest nieaktualna — odśwież i spróbuj ponownie.";
    case "NotFound":
      return "Pozycja nie istnieje — lista została odświeżona.";
    case "Background":
      return "Zadanie w tle nie powiodło się — spróbuj ponownie.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}
