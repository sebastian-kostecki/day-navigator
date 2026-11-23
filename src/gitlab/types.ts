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

/** Items the filter removed, per section. Mirrors `gitlab::FilteredOut`. */
export type FilteredOut = {
  mergeRequests: number;
  issues: number;
  hiddenIssues: number;
};

/** Recently closed items. Mirrors `gitlab::ClosedSection`. */
export type ClosedSection = {
  days: number;
  items: GitlabItem[];
  filteredOut: number;
  /** Fixed Polish text when the closed fetch failed; `items` is then empty. */
  error: string | null;
};

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
  /** True when `gitlabRefresh.minutes` is invalid and the default interval is used. */
  refreshSettingInvalid: boolean;
  /** True when a `gitlabFilter` with at least one constraint is in effect. */
  filterActive: boolean;
  /** True when the `gitlabFilter` block is invalid and everything is shown. */
  filterInvalid: boolean;
  filteredOut: FilteredOut;
  /** Recently closed items; null when `closedDays` is off or the filter is invalid. */
  closed: ClosedSection | null;
};

/** Mirrors `hidden_issues::HiddenIssueError`. */
export type HiddenIssueError =
  | { kind: "Store"; detail: StoreError }
  | { kind: "NotFound" }
  | { kind: "NotInList" }
  | { kind: "ListUnavailable" };
