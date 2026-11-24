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
  /** First seen after the view's seen-until mark; set on MRs and visible issues. */
  isNew: boolean;
  /** When the user hid the issue (UTC ms); only on hidden issues. */
  hiddenAtMs: number | null;
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

/** A recently closed item. Mirrors `gitlab::ClosedItem` (the item is flattened). */
export type ClosedItem = GitlabItem & {
  /** When it was closed or merged (RFC 3339); orders the section. */
  closedAt: string;
};

/** Recently closed items. Mirrors `gitlab::ClosedSection`. */
export type ClosedSection = {
  days: number;
  items: ClosedItem[];
  filteredOut: number;
  /** Fixed Polish text when the closed fetch failed; `items` is then empty. */
  error: string | null;
};

/** Include and exclude entries of one filter dimension, lowercased. */
export type FilterRule = { include: string[]; exclude: string[] };

/** Mirrors `gitlab::filter::GitlabFilter`. */
export type GitlabFilterValues = {
  projects: FilterRule;
  labels: FilterRule;
  milestones: FilterRule;
  closedDays: number | null;
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
  /** The filter that shaped `items`; belongs to the items. */
  filter: GitlabFilterValues;
  /** The interval (minutes) the last round used. */
  refreshMinutes: number;
  /** When the wait after the last round ends (UTC ms); null before the first round. */
  nextRefreshAtMs: number | null;
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

/** Mirrors `settings_file::OpenSettingsError`. */
export type OpenSettingsError =
  { kind: "ConfigDirUnresolved" } | { kind: "Missing" } | { kind: "OpenFailed" };
