// Mirrors the serde output of `HealthSnapshot` in src-tauri/src/health/mod.rs.

export type Overall = "starting" | "ok" | "error" | "notRunning";

/** `SourceState` is flattened into the source: `state` is the tag. */
export type SourceState =
  | { state: "ok" }
  | { state: "notConfigured" }
  /** Calm "cannot reach it right now" (for example no VPN); not an error. */
  | { state: "unavailable"; message: string }
  | { state: "error"; kind: string; message: string };

/** `id` is a stable key such as `settings`, `app-data`, `gitlab`. */
export type SourceStatus = {
  id: string;
  label: string;
} & SourceState;

/** Summary of the `backups/` folder; `state` is the tag. */
export type BackupSummary =
  | {
      state: "listed";
      kept: number;
      keep: number;
      /** `date` is "YYYY-MM-DD" (from the file name), `atMs` the file's modified time. */
      last: { date: string; atMs: number | null } | null;
      /** No backup dated today. */
      stale: boolean;
    }
  | { state: "unreadable" };

/** Details for the "Dane aplikacji" card. */
export type AppDataDetails = {
  /** `null` while the store does not open. */
  schemaVersion: number | null;
  backups: BackupSummary;
};

/** Error of `open_backups_folder`; carries no path. */
export type OpenBackupsError =
  { kind: "DataDirUnresolved" } | { kind: "Missing" } | { kind: "OpenFailed" };

export type HealthSnapshot = {
  overall: Overall;
  sources: SourceStatus[];
  /** Wall-clock time of the last finished check, for display only. */
  checkedAtMs: number | null;
  /** Bumped on every stored change; snapshots are ordered by this. */
  seq: number;
  /** `null` before the first check or when the data folder is unresolved. */
  appData: AppDataDetails | null;
};
