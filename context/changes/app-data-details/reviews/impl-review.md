<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Szczegóły „Dane aplikacji” w „Stanie źródeł”

- **Plan**: context/changes/app-data-details/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Summary of evidence

- Range reviewed: `8227f4f..HEAD` (5 commits: plan, p1 `5435829`, p2 `db6a36e`, epilogue, manual checks).
- Every planned file is changed; no unplanned code file. `context/foundation/roadmap.md` (S-07 → `in-progress`) is the `/10x-implement` roadmap sync, not scope creep.
- Plan items checked one by one (all MATCH):
  - `BackupSummary` (`tag = "state"`, camelCase) / `LastBackup { date, atMs }` and `summarize` in `store/backup.rs`: `NotFound` → `Listed { kept: 0, keep: 7, last: None, stale: true }`, other `read_dir` error → `Unreadable` with a `debug` log of `e.kind()` only, counts only `dated` names ≤ today (same rule as `prune`, `.tmp` and foreign files skipped), `stale = last.is_none_or(date < today)`.
  - `StoreHandle::backup_summary` / `backups_dir` read the plain `dir` field — no lock; `backups_dir` doc says never for logs/errors.
  - `Store::schema_version` + `StoreHandle::status -> Result<u32, StoreError>` read under the same lock after `check` and `backup_if_due`.
  - `health::store_probe` calls `handle.status(today)` first, then `handle.backup_summary(today)` (ordering per Critical Implementation Details); `schema_version` only on `Ok`; `None` details when the data dir is unresolved; `store_status` kept as a thin wrapper so existing `tests/health_probe.rs` cases are unchanged.
  - `HealthSnapshot.app_data` set in `check_round`, `None` in the start snapshot; the watchdog mutates only `overall`/`seq`, so it keeps the previous `app_data`.
  - `backups_folder.rs` mirrors `settings_file.rs` exactly (fixed path, `is_dir` check, nothing created, `OpenBackupsError` tagged `kind`/`detail`, `log::warn!("backups: open failed")` without path/system text); registered in `generate_handler!`; does not take the store lock.
  - TS mirror (`types.ts`) matches serde: `state: "listed" | "unreadable"`, `kept/keep/last{date, atMs}/stale`, `schemaVersion: number | null`, `appData: AppDataDetails | null`, `OpenBackupsError` union on `kind`. Rust type and TS mirror landed in the same commit (`5435829`).
  - `src/health/api.ts` is the only `invoke` importer in `src/health/`; `useHealth.ts` uses `healthStatus`/`checkHealthNow`, behaviour unchanged.
  - `appDataFacts`: order „Ostatnia kopia” (`formatAgo(atMs)` / `DD.MM.YYYY` / „brak”, `warn` when stale), „Kopie” `N z 7` (or „nie można odczytać folderu” in `warn`, without „Ostatnia kopia”), „Schemat” `vN` only when not `null`. Facts and button render for every source state when `appData !== null` — an unreadable folder renders a warning, never an empty list.
  - `SourceFact.tone`, `SourceCard.actions`, `OpenBackupsButton` (`size="sm" variant="quiet" icon="folder"`, `aria-disabled`/`aria-busy`, inline `role="alert"` error cleared on the next successful click).
  - CSS uses only tokens (`--warn`, `--danger-soft`, `--space-*`), no literal colours.
- Privacy: no path or row value in `BackupSummary`, `AppDataDetails`, `OpenBackupsError` or any new log line; `app_data_json_carries_no_folder_name_or_path` covers the JSON.
- Tests cover every case in Phase 1 §6 (missing folder, 3 copies with `set_modified`, 7 + future, yesterday → stale, foreign/temp files, file-in-place → `Unreadable`, `status` returns latest migration, damaged DB → `Damaged` + `schema_version: None` + `Listed`, JSON shape, `appData: null`, no path, `backups_folder_to_open` cases and error serialisation).
- Out-of-scope list respected: no migration, `schema.rs`, `Cargo.toml` or `capabilities/default.json` change; no new source kind, toast, tray state or event; backup rules (`backup_if_due`, `prune`, `KEEP`) unchanged.

## Success criteria

Re-run in this review:

| Command | Result |
|---|---|
| `cargo test` (src-tauri) | PASS — 456 unit + all integration suites green |
| `cargo fmt --check` | PASS |
| `npm run build` | PASS |
| `npm run lint` | PASS |
| `npm run format:check` | PASS |
| `invoke` import in `src/health` | only `src/health/api.ts` |
| `git diff --stat 8227f4f..HEAD` on `capabilities/default.json`, `Cargo.toml`, `store/schema.rs` | no changes |
| `#[0-9a-fA-F]{3,6}` in `src/health/Health.css` | no match |

`cargo clippy --all-targets -- -D warnings` was not re-run here (reported green in the implementing session for both phases).

Manual 2.4–2.7 are `[x]`; they are UI observations confirmed by the user in the running app, consistent with the diff (facts, button, missing-folder path that creates nothing).

## Findings

### F1 — A failing `PRAGMA user_version` turns a healthy store into an error source

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/mod.rs:281
- **Detail**: `status()` now ends with `store.schema_version()` and propagates its error. After a passed `check` and backup, a failure of this read-only detail query (in practice only `Busy`/IO) flips the „Dane aplikacji” source to `Error` (tray, toast, event log) for a round, and — unlike a failed `check` — does not drop the store for reopen. The plan's contract (`Result<u32, StoreError>`) asked for exactly this, so it is a plan-level choice; the realistic probability is very low because `check` just succeeded on the same connection under the same lock.
- **Fix**: Keep the source state independent of the detail: have `status` return `Ok(store.schema_version().ok())` (`Result<Option<u32>, StoreError>`) so a failed version read only hides „Schemat”.
- **Decision**: PENDING

### F2 — Pure `sourceView.ts` imports its fact type from the `SourceCard` component

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/health/sourceView.ts:5
- **Detail**: `sourceView.ts` ("pure display rules… no state") now does `import type { SourceFact } from "./SourceCard"`, while `SourceCard.tsx` imports from `sourceView.ts` — a type-only import cycle and a rules module depending on a component. Harmless at runtime (erased), lint/build pass.
- **Fix**: Move `SourceFact` into `sourceView.ts` (or `types.ts`) and import it from there in `SourceCard.tsx` and `HealthPanel.tsx`.
- **Decision**: PENDING
