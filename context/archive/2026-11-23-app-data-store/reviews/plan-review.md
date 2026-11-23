<!-- PLAN-REVIEW-REPORT -->
# Plan Review: App Data Store Implementation Plan

- **Plan**: context/changes/app-data-store/plan.md
- **Mode**: Deep (code checked inline)
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical · 4 warnings · 4 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding
11/11 paths ✓, 6/6 symbols ✓ (probe, probe_round, VaultState::current, short_reason, spawn_blocking, SourceId), rusqlite 0.43.0 on crates.io ✓, brief↔plan ✓, Progress↔Phase ✓

## Findings

### F1 — Setting WAL before the checks can change a file the plan then refuses

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Critical Implementation Details "Open sequence"; Phase 1 §2, §5
- **Detail**: The open sequence set `journal_mode=WAL` before `quick_check` and the `user_version` check. WAL mode is stored in the file header, so switching a rollback-journal file to WAL is a write. That breaks "every refusal happens before anything writes" and would fail the NewerSchema byte-identity test with a rollback-journal fixture.
- **Fix**: Reorder to busy_timeout → quick_check → user_version → WAL/FULL/foreign keys → backup → migrate. The NewerSchema fixture is created in rollback-journal mode on purpose.
- **Decision**: FIXED

### F2 — A missing or empty database file is silently replaced by a fresh store

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 1 §2 (open), Migration Notes
- **Detail**: SQLite treats a 0-byte file as a valid empty database, and a missing file is created. Either case would show lost history as an empty store reported OK, the "silent reset" that the plan and the hard rule forbid.
- **Fix**: A 0-byte db → `Damaged`; a missing db while `backups/` holds a dated backup → new `Missing` variant; no file is created or changed in either case. Only an empty data dir is a first run. Tests and AGENTS.md "start over" step added.
  - Strength: Meets "never render empty in place of a failed source" for about two `metadata` calls.
  - Tradeoff: To start over on purpose, the user must also clear `backups/`.
  - Confidence: HIGH — SQLite documents that a 0-byte file is a valid empty database.
  - Blind spot: None significant.
- **Decision**: FIXED

### F3 — A failing backup logs a warning every 30 s and never reaches the user

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 2 §1, Phase 4 §1
- **Detail**: `status()` calls `backup_if_due` every 30 s. A backup that keeps failing would log ~2,880 warnings a day and push other diagnostics out of the rotated 3×1 MB log.
- **Fix A ⭐ Recommended**: `Store.last_backup_attempt: Option<NaiveDate>`; try and warn at most once per date.
  - Strength: No log spam; keeps "never blocks the store".
  - Tradeoff: Backup failures stay visible only in the log.
  - Confidence: HIGH — same once-per-transition idea as the health baseline.
  - Blind spot: A failure that clears later the same day waits until the next date or a restart.
- **Fix B**: Fix A plus Error `BackupFailed` on `app-data` after N days with no backup.
  - Strength: The user sees that the safety net is gone.
  - Tradeoff: A red tray for a store that works; new phrase and tests.
  - Confidence: MED — N and the tray semantics are not settled.
  - Blind spot: Whether the PRD wants non-blocking problems in the tray.
- **Decision**: FIXED (Fix A)

### F4 — How StoreHandle reopens and where it gets its input are not specified

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 §1–§3
- **Detail**: The plan didn't say whether a failed `check()` drops the open store. "Logged and stored" had no field to store into. `runtime::probe_round(settings_path)` has no access to the handle.
- **Fix**: A non-`Busy` `check()` error resets `inner` to `None`; the handle stores no error; `probe_round(settings_path, store, today)` is called from `check_round` with `app.state::<StoreHandle>()`.
- **Decision**: FIXED

### F5 — Later commands can block on the store lock during a backup

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 4 §1, §5
- **Detail**: The health thread holds the store mutex during reopen and `VACUUM INTO`. A sync command (which runs on the main thread) calling `with_store` would freeze the window.
- **Fix**: AGENTS.md block: store commands are `async` and call `with_store` inside `spawn_blocking`.
- **Decision**: FIXED

### F6 — The kill test proves atomicity, not survival of a Windows crash

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3; plan-brief Success Criteria
- **Detail**: `TerminateProcess` leaves the OS page cache intact, so the test would pass even with `synchronous=OFF`. The brief claimed "restart … proven by tests".
- **Fix**: End state and brief now say the kill test covers an app crash; OS-crash durability relies on `synchronous=FULL`, checked by reading the pragma back.
- **Decision**: FIXED

### F7 — The "Damaged" message names `backups` but not where it is

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 4 §3
- **Detail**: The user had to locate `%APPDATA%\com.sebas.day-navigator\backups` on their own. Vault panel messages already carry the user's own paths.
- **Fix**: `StoreError` stays path-free; `probe::store_status` adds the data dir to the panel message only.
- **Decision**: FIXED

### F8 — "Rename without replacing" isn't what `std::fs::rename` does on Windows

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §1
- **Detail**: `std::fs::rename` replaces an existing target on Windows (`MOVEFILE_REPLACE_EXISTING`).
- **Fix**: `std::fs::hard_link` the temp file to the final name (fails if it exists), then remove the temp file.
- **Decision**: FIXED
