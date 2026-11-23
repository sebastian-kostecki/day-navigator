// Mirrors the serde output of `GitlabSnapshot` in src-tauri/src/gitlab/mod.rs.

import type { StoreError } from "../components/storeError";

export type ItemKind = "issue" | "mergeRequest";

/** Why a hidden issue came back to the list. Mirrors `gitlab::ChangeReason`. */
export type ChangeReason = "status" | "comment" | "returned";

export type GitlabItem = {
  kind: ItemKind;
  /** GitLab's global id: the key for hide, unhide and dismiss. */
  id: number;
  /** `group/project#12` or `group/project!34`. */
  reference: string;
  title: string;
  /** Only set for a link on the configured host. */
  webUrl: string | null;
  /** RFC 3339 text as GitLab sent it. */
  updatedAt: string;
  labels: string[];
  draft: boolean;
  /** Set on a visible issue that carries a "changed" mark. */
  changed: ChangeReason | null;
};

export type AssignedItems = {
  mergeRequests: GitlabItem[];
  /** Issues that are not hidden; a returned one carries `changed`. */
  issues: GitlabItem[];
  /** Issues the user has hidden. */
  hiddenIssues: GitlabItem[];
};

/** `GitlabState` is tagged by `state` and nested under the snapshot's `state`. */
export type GitlabState =
  | { state: "starting" }
  | { state: "ok" }
  | { state: "notConfigured" }
  /** Calm "cannot reach it right now" (for example no VPN); not an error. */
  | { state: "unavailable"; message: string }
  | { state: "error"; kind: string; message: string };

export type GitlabSnapshot = {
  state: GitlabState;
  /** Last successful list in this session (memory only). */
  items: AssignedItems | null;
  lastSuccessMs: number | null;
  checkedAtMs: number | null;
  refreshing: boolean;
  /** Bumped on every stored change; snapshots are ordered by this. */
  seq: number;
  /** Hidden issues whose comment check failed in the last round. */
  hiddenCheckFailed: number;
  /** True when `gitlab.user` is set, so the user's own comments are ignored. */
  ownCommentsKnown: boolean;
  /** Fixed Polish text when hidden state could not be read; the list shows every issue. */
  hiddenError: string | null;
};

/** Mirrors `hidden_issues::HiddenIssueError`. */
export type HiddenIssueError =
  | { kind: "Store"; detail: StoreError }
  | { kind: "NotFound" }
  | { kind: "NotInList" }
  | { kind: "ListUnavailable" };
