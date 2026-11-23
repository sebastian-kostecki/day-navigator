// Mirrors the serde output of `GitlabSnapshot` in src-tauri/src/gitlab/mod.rs.

export type ItemKind = "issue" | "mergeRequest";

export type GitlabItem = {
  kind: ItemKind;
  /** `group/project#12` or `group/project!34`. */
  reference: string;
  title: string;
  /** Only set for a link on the configured host. */
  webUrl: string | null;
  /** RFC 3339 text as GitLab sent it. */
  updatedAt: string;
  labels: string[];
  draft: boolean;
};

export type AssignedItems = {
  mergeRequests: GitlabItem[];
  issues: GitlabItem[];
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
};
