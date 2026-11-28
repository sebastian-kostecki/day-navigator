import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { storeErrorMessage } from "../components/storeError";
import type { GitlabSnapshot, GitlabViewState, HiddenIssueError, ItemKind } from "./types";

export function getGitlabSnapshot(): Promise<GitlabSnapshot> {
  return invoke<GitlabSnapshot>("gitlab_snapshot");
}

/** Wakes the refresh thread; the result arrives as a `gitlab-changed` event. */
export function refreshGitlab(): Promise<void> {
  return invoke<void>("refresh_gitlab");
}

/** Ends a visit to the GitLab view: the "new" flags go out and the seen mark moves. */
export function markGitlabSeen(): Promise<void> {
  return invoke<void>("mark_gitlab_seen");
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

/** Hides a merge request (assigned or awaiting review); the new snapshot arrives as a `gitlab-changed` event. */
export function hideGitlabMergeRequest(id: number): Promise<void> {
  return invoke<void>("hide_gitlab_merge_request", { id });
}

/** Shows a hidden merge request again, unmarked. */
export function unhideGitlabMergeRequest(id: number): Promise<void> {
  return invoke<void>("unhide_gitlab_merge_request", { id });
}

/** Dismisses the "changed" mark of a returned merge request. */
export function dismissGitlabMergeRequestChange(id: number): Promise<void> {
  return invoke<void>("dismiss_gitlab_merge_request_change", { id });
}

function asHiddenIssueError(e: unknown): HiddenIssueError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as HiddenIssueError;
  }
  return null;
}

/** True when the row the user acted on is no longer what the list showed. */
export function isHiddenIssueStale(e: unknown): boolean {
  const kind = asHiddenIssueError(e)?.kind;
  return kind === "NotFound" || kind === "NotInList";
}

/** Polish message for anything thrown by a hide, unhide or dismiss call. `kind` picks the words for the row's item kind. */
export function hiddenIssueErrorMessage(e: unknown, kind: ItemKind = "issue"): string {
  const err = asHiddenIssueError(e);
  if (!err) return `Nieoczekiwany błąd: ${String(e)}`;
  switch (err.kind) {
    case "Store":
      return storeErrorMessage(err.detail);
    case "NotFound":
      return kind === "mergeRequest"
        ? "Merge request nie jest już ukryty — odśwież listę."
        : "Zgłoszenie nie jest już ukryte — odśwież listę.";
    case "NotInList":
      return kind === "mergeRequest"
        ? "Merge requesta nie ma na liście — odśwież listę."
        : "Zgłoszenia nie ma na liście — odśwież listę.";
    case "ListUnavailable":
      return "Lista GitLaba nie jest jeszcze dostępna.";
    default:
      return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
  }
}

/** The remembered type choice and hidden expansion. A failure rejects with `GitlabViewStateError`; Rust has already logged it. */
export function getGitlabViewState(): Promise<GitlabViewState> {
  return invoke<GitlabViewState>("gitlab_view_state");
}

/** Saves the type choice and hidden expansion. A failure rejects with `GitlabViewStateError`; Rust has already logged it. */
export function setGitlabViewState(view: GitlabViewState): Promise<void> {
  return invoke<void>("set_gitlab_view_state", { view });
}
