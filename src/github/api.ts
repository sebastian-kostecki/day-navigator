import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { storeErrorMessage } from "../components/storeError";
import type { HiddenIssueError } from "../gitlab/types";
import type { GithubSnapshot } from "./types";

export function getGithubSnapshot(): Promise<GithubSnapshot> {
  return invoke<GithubSnapshot>("github_snapshot");
}

/** Wakes the refresh thread; the result arrives as a `github-changed` event. */
export function refreshGithub(): Promise<void> {
  return invoke<void>("refresh_github");
}

/** Opens an issue in the default browser. */
export function openGithubIssue(url: string): Promise<void> {
  return openUrl(url);
}

/** Hides an issue of the current list; the new snapshot arrives as a `github-changed` event. */
export function hideGithubIssue(reference: string): Promise<void> {
  return invoke<void>("hide_github_issue", { reference });
}

/** Shows a hidden issue on the list again. */
export function unhideGithubIssue(reference: string): Promise<void> {
  return invoke<void>("unhide_github_issue", { reference });
}

/** Clears the "Zmienione" mark of a returned issue. */
export function dismissGithubChange(reference: string): Promise<void> {
  return invoke<void>("dismiss_github_change", { reference });
}

function asHiddenIssueError(e: unknown): HiddenIssueError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as HiddenIssueError;
  }
  return null;
}

/** True when the row the user acted on is no longer what the list showed. */
export function isGithubHiddenStale(e: unknown): boolean {
  const kind = asHiddenIssueError(e)?.kind;
  return kind === "NotFound" || kind === "NotInList";
}

/** Polish message for anything thrown by a hide, unhide or dismiss call. */
export function githubHiddenErrorMessage(e: unknown): string {
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
      return "Lista GitHuba nie jest jeszcze dostępna.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}
