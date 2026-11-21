// Mirrors the serde output of `HealthSnapshot` in src-tauri/src/health/mod.rs.

export type Overall = "starting" | "ok" | "error" | "notRunning";

/** `SourceState` is flattened into the source: `state` is the tag. */
export type SourceState =
  | { state: "ok" }
  | { state: "notConfigured" }
  | { state: "error"; kind: string; message: string };

/** `id` is a stable key such as `settings`, `vault:work`, `exclusions:private`. */
export type SourceStatus = {
  id: string;
  label: string;
} & SourceState;

export type HealthSnapshot = {
  overall: Overall;
  sources: SourceStatus[];
  /** Wall-clock time of the last finished check, for display only. */
  checkedAtMs: number | null;
  /** Bumped on every stored change; snapshots are ordered by this. */
  seq: number;
};
