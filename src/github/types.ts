// Mirrors the serde output of `GithubSnapshot` in src-tauri/src/github/mod.rs.

import type { ChangeReason } from "../gitlab/types";

export type GithubIssue = {
  /** `owner/name` as written in settings. */
  repo: string;
  number: number;
  /** `owner/name#number`, owner and name lowercase: the stable key of the issue. */
  reference: string;
  title: string;
  /** Only set for a link under the repository's `/issues/`. */
  webUrl: string | null;
  /** RFC 3339 text as GitHub sent it. */
  updatedAt: string;
  /** Label names in the order GitHub sent them; empty when there are none. */
  labels: string[];
  /** Why a hidden issue came back; set only on a marked, visible issue. */
  changed: ChangeReason | null;
  /** When the issue was hidden (UTC epoch ms); set only on a hidden issue. */
  hiddenAtMs: number | null;
};

/** Why one repository has no list. Fixed Polish text, no repo name. */
export type RepoError = {
  kind: string;
  message: string;
};

export type RepoView = {
  /** `owner/name` as written in settings. */
  repo: string;
  /** The visible issues; a marked one carries `changed`. */
  issues: GithubIssue[];
  /** The repo's hidden issues in list order. */
  hidden: GithubIssue[];
  error: RepoError | null;
};

/** `GithubState` is tagged by `state` and nested under the snapshot's `state`. */
export type GithubState =
  | { state: "starting" }
  | { state: "ok" }
  | { state: "notConfigured" }
  /** Calm "cannot reach it right now"; not an error. */
  | { state: "unavailable"; message: string }
  | { state: "error"; kind: string; message: string };

export type GithubSnapshot = {
  state: GithubState;
  /** Last successful round in this session (memory only), in settings order. */
  repos: RepoView[];
  lastSuccessMs: number | null;
  checkedAtMs: number | null;
  refreshing: boolean;
  /** Bumped on every stored change; snapshots are ordered by this. */
  seq: number;
  /** Hidden issues whose comment check failed this round. */
  hiddenCheckFailed: number;
  /** Fixed Polish text when the hide records could not be read or written. */
  hiddenError: string | null;
  /** Invalid `githubRefresh.minutes`: the default interval is in use. */
  refreshSettingInvalid: boolean;
  /** The interval (minutes) the last round used. */
  refreshMinutes: number;
  /** When the wait after the last round ends (UTC ms); `null` before the first round. */
  nextRefreshAtMs: number | null;
  /** References of visible issues new since the last visit to this view, in list order. */
  newReferences: string[];
};
