import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { GitlabSnapshot, HiddenIssueError, StoreError } from "./types";

export function getGitlabSnapshot(): Promise<GitlabSnapshot> {
  return invoke<GitlabSnapshot>("gitlab_snapshot");
}

/** Wakes the refresh thread; the result arrives as a `gitlab-changed` event. */
export function refreshGitlab(): Promise<void> {
  return invoke<void>("refresh_gitlab");
}

/** Opens an item in the default browser. */
export function openGitlabItem(url: string): Promise<void> {
  return openUrl(url);
}

/** Hides an issue of the current list; the new snapshot arrives as a `gitlab-changed` event. */
export function hideGitlabIssue(id: number): Promise<void> {
  return invoke<void>("hide_gitlab_issue", { id });
}

/** Shows a hidden issue again, unmarked. */
export function unhideGitlabIssue(id: number): Promise<void> {
  return invoke<void>("unhide_gitlab_issue", { id });
}

/** Dismisses the "changed" mark of a returned issue. */
export function dismissGitlabChange(id: number): Promise<void> {
  return invoke<void>("dismiss_gitlab_change", { id });
}

function asHiddenIssueError(e: unknown): HiddenIssueError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as HiddenIssueError;
  }
  return null;
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

/** Polish message for anything thrown by a hide, unhide or dismiss call. */
export function hiddenIssueErrorMessage(e: unknown): string {
  const err = asHiddenIssueError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeErrorMessage(err.detail);
    case "NotFound":
      return "Zgłoszenie nie jest już ukryte — odśwież listę.";
    case "NotInList":
      return "Zgłoszenia nie ma na liście — odśwież listę.";
    case "ListUnavailable":
      return "Lista GitLaba nie jest jeszcze dostępna.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}
