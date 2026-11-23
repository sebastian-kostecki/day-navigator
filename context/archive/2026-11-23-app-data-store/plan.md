# App Data Store Implementation Plan

## Overview

Roadmap foundation F-01: give the app one durable, app-owned data store in the app data dir. It is a SQLite database opened through `rusqlite`, with versioned migrations, atomic write transactions, a daily backup and refusal (never silent reset) when the file is damaged or newer than the app. The app opens it at startup and the tray reports it as the "Dane aplikacji" source. No records are defined here: own tasks (S-06), "To do" (S-07), hidden-issue snapshots (S-03/S-04) and work time (S-08) each add their own tables as migrations on top of this contract.

## Current State Analysis

- The only app-owned file is `settings.json` in the app config dir, read-only and versioned (`src-tauri/src/settings.rs:77`). No store exists for tasks, "To do", hidden issues or work time (roadmap Baseline "Data: partial").
- The only atomic-write code is `src-tauri/src/vault/fsops.rs` (temp file + fsync + `ReplaceFileW`), tied to `VaultError` and vault labels; it gives no atomicity across files, which S-07 needs (ticking a "To do" item that is an own task also marks the task done).
- Tauri wiring keeps per-source state in managed state that tolerates failure and is replaced by the health round (`VaultState`, `src-tauri/src/lib.rs:21`); blocking I/O runs through `spawn_blocking` (`lib.rs:74`).
- Health sources are a fixed list built by `health::probe::probe` (`src-tauri/src/health/probe.rs:13`), which returns early when settings fail; the round runs every 30 s (`src-tauri/src/runtime.rs:28`, `probe_round` at `runtime.rs:235`). Sources serialise as string keys (`health/mod.rs:29`), and the UI mirror takes `id: string` (`src/health/types.ts:11`), so a new source needs no TS change. Tray notices use fixed Polish phrases per error kind (`health/mod.rs:156`).
- Domain modules do not import `tauri` and take time as a parameter (`tasks.rs:1-3`); error enums are `thiserror` + `Serialize` with `tag = "kind", content = "detail"` and carry no content (`vault/error.rs:1-12`).
- `tauri-plugin-single-instance` guarantees one app process, so only one process ever opens the store. CI builds and tests on `windows-latest` and runs `cargo audit` (`.github/workflows/ci.yml:41,57,96`).
- PRD: "Trwałość czasu pracy" — the counter and history survive closing, a Windows restart and a crash, losing at most a few seconds (`context/foundation/prd.md:181`); no data migration, the work-time history starts fresh (`prd.md:173`).

## Desired End State

- `src-tauri/src/store/` provides `Store` (open, read, write transaction, liveness check, daily backup) with no `tauri` import. Opening creates the app's own data folder if needed, sets WAL + `synchronous=FULL` + foreign keys, runs `quick_check`, refuses a damaged file or a newer schema without touching it, and applies pending migrations atomically.
- The production migration list (`store/schema.rs`) is empty in F-01; later slices append to it.
- Once per calendar day a consistent copy lands in `<app data dir>/backups/day-navigator-YYYY-MM-DD.db`; the last 7 are kept. A failed backup is logged and never blocks the store.
- A child-process kill test proves that after an abrupt process death the store opens, passes `quick_check`, holds every commit the writer reported, and holds no half-applied transaction. The kill harness lives in `src-tauri/tests/common` for later slices. A kill leaves the OS page cache intact, so this covers an app crash; durability across a Windows crash or power loss relies on `synchronous=FULL`, verified only by reading the pragma back.
- The app opens the store at startup from `app_data_dir()` (`%APPDATA%\com.sebas.day-navigator\`). A failure is stored, never fatal. The health round retries opening, checks liveness, runs the daily backup and reports the "Dane aplikacji" source (`app-data`) in the tray, notices and the health panel.
- `AGENTS.md` names the store conventions so later slices use it instead of ad-hoc files.

Verify with `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo audit`, and a manual run: the health panel shows "Dane aplikacji" OK, today's backup exists, a corrupted file turns the source red without touching the file, and restoring a backup turns it green within one round.

### Key Discoveries:

- `probe()` returns early on a settings error (`health/probe.rs:14-20`), so the store status must be appended in `runtime::probe_round`, not inside `probe()`, or a broken `settings.json` would hide the store's state.
- The app keeps running in the tray for days (`lib.rs:187-214` hides on close, prevents exit), so "first open of the day" alone would skip backups; the health round must also trigger the daily backup.
- On Windows, Tauri's `app_data_dir()` and `app_config_dir()` both resolve to `%APPDATA%\<identifier>\`, so the database sits next to `settings.json`.
- `user_version` lives in the database header and is transactional, so a migration and its version bump commit or roll back together.
- `VACUUM INTO ?1` takes a bound file name and writes a consistent, compacted copy while the connection stays open; it fails if the target exists.

## What We're NOT Doing

- No tables or records for tasks, "To do", hidden issues or work time; each slice adds its own migration.
- No new Tauri commands and no UI view; the store is visible only through the health source.
- No automatic repair or reset of a damaged or newer store, and no automatic restore from backup (restoring is a manual step).
- No backups off this disk (cloud, another drive) and no export/import.
- No `tauri-plugin-sql` or `tauri-plugin-store`, and no store access from the frontend.
- No change to vault code, `settings.json` or its schema; no data migration from `Management/Timeline.md` or Helper App (PRD: none).
- No pre-migration backup beyond the daily one (the daily backup runs before migrations at open).

## Implementation Approach

Build bottom-up so every phase is testable on its own: first the `store` module and its contract with in-process tests, then backups, then the out-of-process crash proof, and only then the Tauri wiring and the health source. The module follows the `settings.rs`/`tasks.rs` pattern: pure Rust, time passed in, typed errors with no row content. All synchronisation lives in a small shared handle (mutex around an optional open store) that the commands of later slices and the health round both use.

## Critical Implementation Details

**Open sequence.** The order inside `Store::open` is load-bearing: create the own folder → refuse a lost file (below) → open the connection → `busy_timeout` → `quick_check` (a non-SQLite file fails here as "not a database") → compare `user_version` with the newest migration (newer → refuse) → set `journal_mode=WAL`, `synchronous=FULL`, `foreign_keys=ON` → daily backup → apply pending migrations. WAL mode is stored in the file header, so switching to it is a write: it comes only after every check. Every refusal happens before anything writes to the file, so a refused store stays byte-identical. The backup comes before migrations so a failing migration always has a same-day copy from before it.

**Lost file.** SQLite treats a 0-byte file as a valid empty database, and a missing file is simply created. Both would turn lost history into an empty store reported as OK. So before opening: a 0-byte `day-navigator.db` → `Damaged`; a missing `day-navigator.db` while `backups/` holds a dated backup → `Missing`. Neither case creates or changes any file. Only a data dir with no database and no dated backup is a first run.

**Error content.** `rusqlite` error messages can name tables and columns but must never carry bound values; build `StoreError` reasons from the SQLite error code and message only, and never format parameters or row data into errors or logs.

## Phase 1: Store core

### Overview

Add `rusqlite` and the `store` module: open with pragmas and checks, migrations, the read/write API and `StoreError`, with in-process tests for reopen, rollback and every refusal.

### Changes Required:

#### 1. Dependency

**File**: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`

**Intent**: Add SQLite with the library compiled in, so the installer needs no system SQLite.

**Contract**: `rusqlite = { version = "0.43", features = ["bundled"] }` (use the newest release that builds on rustc 1.90 and passes `cargo audit`); `Cargo.lock` refreshed by `cargo build`.

#### 2. Store module

**File**: `src-tauri/src/store/mod.rs` (new), `src-tauri/src/lib.rs` (`pub mod store;`)

**Intent**: One entry point for every app-owned record: open the database file `day-navigator.db` in a given folder, apply the open sequence above, and run reads and atomic writes. Opening creates the folder itself (it is the app's own data dir, not a configured path).

**Contract**: `busy_timeout` 5 s first; after the checks, pragmas `journal_mode=WAL` (assert the returned mode is `wal`), `synchronous=FULL`, `foreign_keys=ON` (order per "Open sequence"). Write transactions start `IMMEDIATE`. Later slices depend on this signature:

```rust
pub const DB_FILE: &str = "day-navigator.db";
pub struct Migration { pub version: u32, pub sql: &'static str }

impl Store {
    pub fn open(dir: &Path, migrations: &[Migration], today: NaiveDate) -> Result<Store, StoreError>;
    pub fn read<T, E: From<StoreError>>(&self, f: impl FnOnce(&Connection) -> Result<T, E>) -> Result<T, E>;
    /// Commits only if `f` returns Ok; any Err (or panic) rolls back the whole transaction.
    pub fn write<T, E: From<StoreError>>(&mut self, f: impl FnOnce(&Transaction<'_>) -> Result<T, E>) -> Result<T, E>;
    pub fn check(&self) -> Result<(), StoreError>; // cheap liveness query for the health round
}
```

#### 3. Errors

**File**: `src-tauri/src/store/error.rs` (new)

**Intent**: Typed refusals and failures in the project's error style, with `From<rusqlite::Error>` mapping SQLite codes to variants.

**Contract**: `#[derive(Debug, Clone, thiserror::Error, Serialize)] #[serde(tag = "kind", content = "detail")] pub enum StoreError { DirUnavailable { kind }, Damaged { reason }, Missing, NewerSchema { found: u32, supported: u32 }, MigrationFailed { version: u32, reason }, Busy, Io { kind }, Sql { reason } }`. Mapping: `NOTADB`/`CORRUPT` → `Damaged`; `BUSY`/`LOCKED` → `Busy`; `CANTOPEN`/`READONLY`/`IOERR`/`FULL` → `Io`; the rest → `Sql`. Invariant (module doc): no variant carries row values or absolute paths.

#### 4. Migrations

**File**: `src-tauri/src/store/schema.rs` (new)

**Intent**: The production migration list, empty in F-01, plus a validator so a malformed list is caught by tests and refused at open.

**Contract**: `pub const MIGRATIONS: &[Migration] = &[];` and `pub fn validate(migrations: &[Migration]) -> Result<(), StoreError>` (versions start at 1, contiguous, increasing). Each pending migration runs with `PRAGMA user_version = N` in one transaction; on failure → `MigrationFailed { version }` and the store stays at the previous version. Doc comment: a shipped migration is never edited, only followed by a new one.

#### 5. Tests

**File**: unit tests in `src-tauri/src/store/*.rs`, integration tests in `src-tauri/tests/store.rs` (new), helper `test_store_dir()` in `src-tauri/tests/common/mod.rs`

**Intent**: Prove the contract on invented data in temp dirs.

**Contract**: cases: fresh open creates folder and file and reaches the latest `user_version`; pragmas read back as WAL / FULL (2) / foreign keys on; data written, store dropped, reopened → present; a `write` whose closure fails after one insert leaves no rows; a panicking closure rolls back; `user_version` above the newest → `NewerSchema` and file bytes unchanged (hash before/after; the fixture is created on purpose in the default rollback-journal mode, so a WAL switch before the checks would fail the test); a non-SQLite file → `Damaged`, bytes unchanged; a 0-byte file → `Damaged`, still 0 bytes; no database but a dated file in `backups/` → `Missing`, no database created; an empty dir → first run succeeds; a failing v2 migration → `MigrationFailed { version: 2 }`, `user_version` stays 1, v1 rows intact; `validate` rejects gaps, duplicates and a list not starting at 1, and accepts `schema::MIGRATIONS`; a constraint error after inserting a sentinel string serialises as `{kind, detail}` without the sentinel.

### Success Criteria:

#### Automated Verification:

- Store unit and integration tests pass: `cargo test` in `src-tauri/`
- Clippy is clean: `cargo clippy --all-targets -- -D warnings`
- Formatting is clean: `cargo fmt --check`
- No advisories for the new dependency: `cargo audit`

---

## Phase 2: Daily backups

### Overview

Once per calendar day, write a consistent copy of the store into `backups/` and keep the last 7; a failure is logged and never blocks the store.

### Changes Required:

#### 1. Backup routine

**File**: `src-tauri/src/store/backup.rs` (new), call site in `Store::open`

**Intent**: Protect work-time history against a damaged file or a bad migration without a new dependency.

**Contract**: `impl Store { pub fn backup_if_due(&mut self, today: NaiveDate) }` (no `Result`: failures are logged with `log::warn!` showing the error kind and file names only). `Store` keeps `last_backup_attempt: Option<NaiveDate>`; if it equals `today` → nothing, so a failing backup is tried and logged at most once per date (and once more after a restart), not every health round. If `backups/day-navigator-<today>.db` exists → nothing. Otherwise `VACUUM INTO ?1` to `backups/.day-navigator-<today>.db.tmp`, then `std::fs::hard_link` the temp file to the final name (fails instead of replacing an existing one) and remove the temp file. Then delete dated backups beyond the newest 7 (by date in the name) and leftover `.tmp` files in `backups/`; files with other names are never touched. `open` calls it after the checks and before migrations.

#### 2. Tests

**File**: `src-tauri/tests/store_backup.rs` (new)

**Intent**: Prove the daily and retention rules with injected dates.

**Contract**: cases: open on 9 consecutive dates → exactly the 7 newest backups remain; a second call on the same date adds nothing; a backup opens as a valid database with the same rows; a stale `.tmp` is removed, an unrelated file stays; `backups` existing as a file (backup impossible) → `open` still succeeds, and a second `backup_if_due` on the same date makes no new attempt; a refused store (damaged or newer) writes no backup.

### Success Criteria:

#### Automated Verification:

- Backup tests pass: `cargo test --test store_backup` in `src-tauri/`
- Full suite, clippy and fmt pass: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`

---

## Phase 3: Crash-proof tests

### Overview

Prove durability across an abrupt process death by killing a real writer process at varying moments, and leave the harness reusable for later data slices.

### Changes Required:

#### 1. Kill harness

**File**: `src-tauri/tests/common/crash.rs` (new; `pub mod crash;` in `tests/common/mod.rs`)

**Intent**: Run a writer in a child process and kill it hard, without adding a `[[bin]]` (that would interfere with `tauri build`).

**Contract**: `pub fn run_writer_until(test_name: &str, dir: &Path, kill: KillAt) -> u64` re-launches `std::env::current_exe()` with `<test_name> --exact --ignored --nocapture --test-threads=1` and env `DN_CRASH_WRITER_DIR=<dir>`, reads stdout lines `committed <n>`, kills the child (`Child::kill`, which is `TerminateProcess` on Windows) after a given number of commits or a given delay, waits for it, and returns the last reported `n`. Lines from libtest itself are ignored.

#### 2. Crash test

**File**: `src-tauri/tests/store_crash.rs` (new)

**Intent**: The F-01 proof that a crash never leaves a half-applied write and never loses a reported commit.

**Contract**: an `#[ignore]` writer test that does nothing unless `DN_CRASH_WRITER_DIR` is set. Otherwise it opens a store with a test-only migration (table `ticks(n PRIMARY KEY, payload BLOB)` and a one-row `counter`). In a loop, each `write` inserts tick `n` with a ~64 KB invented payload and sets `counter = n`, then prints and flushes `committed n`. The parent runs about 10 rounds with a fixed list of kill points (after k commits and after short delays, so some kills land mid-transaction). After each kill it reopens the store and asserts: `open` succeeds, `quick_check` is ok, `count(ticks) == max(n) == counter`, and `max(n) >= last reported`. The whole test stays under ~20 s on CI.

### Success Criteria:

#### Automated Verification:

- Crash test passes locally and is not ignored in the default run: `cargo test --test store_crash` in `src-tauri/`
- The writer helper alone is skipped in a normal run (reported as ignored, not failed): `cargo test --test store_crash`
- Full suite, clippy and fmt pass: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`

#### Manual Verification:

- The crash test passes in CI on `windows-latest` (check the PR/push run)

---

## Phase 4: App wiring and tray health

### Overview

Open the store at startup from the app data dir, share it with later commands and the health round, report it as the "Dane aplikacji" source, and document the conventions.

### Changes Required:

#### 1. Shared handle

**File**: `src-tauri/src/store/mod.rs`

**Intent**: One place that holds the open store or the reason it is not open, reopens on demand, and is safe across threads; commands of later slices and the health round go through it.

**Contract**: `pub struct StoreHandle { dir: Option<PathBuf>, inner: Mutex<Option<Store>> }` with `pub fn with_store<T, E: From<StoreError>>(&self, today: NaiveDate, f: impl FnOnce(&mut Store) -> Result<T, E>) -> Result<T, E>` (opens lazily with `schema::MIGRATIONS` when not open; a `None` dir → `DirUnavailable`) and `pub fn status(&self, today: NaiveDate) -> Result<(), StoreError>` (open if needed, `check()`, `backup_if_due(today)`). On any `check()` error other than `Busy`, `inner` drops back to `None`, so the next call reopens and runs the full open sequence again. The handle stores no error: a failed open is logged and the status is recomputed on each call. A poisoned mutex is recovered like `VaultState::current` (`lib.rs:30`).

#### 2. Startup wiring

**File**: `src-tauri/src/lib.rs`

**Intent**: Resolve `app.path().app_data_dir()` in `setup`, manage a `StoreHandle`, and try one open at startup; a failure is logged (kind only), never fatal, and the health round retries it.

**Contract**: `app.manage(StoreHandle::new(data_dir_or_none))` before `runtime::start`; today from `Local::now().date_naive()`.

#### 3. Health source

**File**: `src-tauri/src/health/mod.rs`, `src-tauri/src/health/probe.rs`, `src-tauri/src/runtime.rs`

**Intent**: Report the store every round, independent of settings, so a broken store reaches the tray, notices and the health panel; a store fixed by hand recovers within one round.

**Contract**: `SourceId::AppData` with key `app-data`, label "Dane aplikacji". A `probe::store_status(handle, today) -> SourceStatus` maps `Ok` → `SourceState::Ok`, errors → `SourceState::Error { kind: <variant name>, message: <Polish text> }`. `StoreError` stays path-free; `store_status` adds the handle's data dir to the panel message only (like vault messages carry the user's own path), while the tray and notices use `short_reason` and never show it. For example, `Damaged`: "Plik danych aplikacji jest uszkodzony. Zamknij aplikację i przywróć kopię z folderu <data dir>\backups."; `Missing`: "Brak pliku danych aplikacji, choć istnieją kopie. Przywróć kopię z folderu <data dir>\backups." `runtime::probe_round` changes to `probe_round(settings_path, store: &StoreHandle, today)`; `check_round` passes `&app.state::<StoreHandle>()` and `Local::now().date_naive()`. It appends the store status after the existing sources, including when settings fail. `short_reason` gains: `Damaged` → "plik danych uszkodzony", `Missing` → "brak pliku danych", `NewerSchema` → "dane z nowszej wersji aplikacji", `MigrationFailed` → "aktualizacja danych nie powiodła się", `DirUnavailable` → "folder danych niedostępny", `Busy` → "plik danych zajęty". The existing `Io` phrase is reused. The TS mirror needs no change (`id: string`).

#### 4. Tests

**File**: `src-tauri/src/health/*` unit tests, `src-tauri/tests/health_probe.rs`

**Intent**: Cover the mapping and the retry without Tauri.

**Contract**: cases: an open store → `app-data` Ok; a damaged file → Error `Damaged` with the Polish message naming the data dir, and the serialised `StoreError` carries no path; an open store whose `check()` fails (non-`Busy`) is reopened on the next `status`; after replacing the file with a valid backup the next `status` → Ok; `StoreHandle` with no dir → `DirUnavailable`; `short_reason` returns the new phrases; `SourceId::AppData` serialises as `"app-data"`.

#### 5. Conventions

**File**: `AGENTS.md`

**Intent**: Make later slices use the store instead of ad-hoc files.

**Contract**: a short "App data store" block (under Project Structure or Rust ↔ TypeScript Contracts). App-owned records go only through `store::StoreHandle` (`src-tauri/src/store/`). A schema change = append a `Migration` to `store/schema.rs`; never edit a shipped one. Changes spanning records happen in one `write`. Store errors never carry row values. Store commands are `async` and call `with_store` inside `spawn_blocking` (the `blocking` helper in `lib.rs`), never from a sync command: the health round holds the store lock during reopen and backup. Starting over on purpose = quit the app and clear both `day-navigator.db*` and `backups/` (a missing database next to existing backups is refused as `Missing`). Data slices reuse the crash harness in `src-tauri/tests/common/crash.rs`. Restoring = quit the app, copy a file from `backups/` over `day-navigator.db`, delete `day-navigator.db-wal` and `-shm`.

### Success Criteria:

#### Automated Verification:

- Health and store tests pass: `cargo test` in `src-tauri/`
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo audit`
- UI gates still pass: `npm run lint`, `npm run format:check`, `npm run build`

#### Manual Verification:

- `npm run tauri dev`: the health panel lists "Dane aplikacji" as OK, and `%APPDATA%\com.sebas.day-navigator\` holds `day-navigator.db` and `backups\day-navigator-<today>.db`
- With the app quit, overwrite `day-navigator.db` with a text file and start the app: the tray shows the source error with "plik danych uszkodzony", the health panel shows the Polish message, other sources are unaffected and the text file is unchanged
- Quit the app, copy today's backup over `day-navigator.db` (delete `-wal`/`-shm`), start again: "Dane aplikacji" is OK

---

## Testing Strategy

### Unit Tests:

- Error mapping from SQLite codes; `schema::validate`; `short_reason` phrases; `SourceId::AppData` key.

### Integration Tests:

- `tests/store.rs`: open/reopen, pragmas, rollback on error and panic, refusals leave bytes unchanged, migration failure keeps the previous version, errors carry no values.
- `tests/store_backup.rs`: daily rule, retention of 7, valid copy, leftovers, non-fatal failure.
- `tests/store_crash.rs`: child-process kill rounds with the reusable harness.
- `tests/health_probe.rs`: store status mapping and recovery.

All data is invented and lives in temp dirs; no test touches the real app data dir.

### Manual Testing Steps:

1. Run the app and confirm the "Dane aplikacji" source and today's backup file.
2. Corrupt the database with the app quit; confirm the error state and the untouched file.
3. Restore from `backups/`; confirm recovery.

## Performance Considerations

`synchronous=FULL` in WAL mode fsyncs the WAL on every commit, which costs milliseconds and is fine for writes at human pace (starting a timer, ticking an item). The health-round `check()` is a trivial query; the full `quick_check` runs only at open. `VACUUM INTO` copies the whole file once a day; the store stays small (text records), so this is negligible.

## Migration Notes

No existing data to migrate. The first start after this change creates an empty `day-navigator.db` at `user_version = 0` next to `settings.json`. Rolling back the app version is safe while the migration list is empty; once slices add migrations, an older app refuses the newer file with `NewerSchema` instead of damaging it.

## References

- Roadmap item: `context/foundation/roadmap.md` (F-01: App-owned data store)
- PRD: `context/foundation/prd.md:73` (guardrail "Czas pracy nie ginie"), `:173` (no data migration), `:181` (NFR "Trwałość czasu pracy")
- Atomic-write precedent: `src-tauri/src/vault/fsops.rs:69`
- Managed-state pattern: `src-tauri/src/lib.rs:21`
- Health probe and round: `src-tauri/src/health/probe.rs:13`, `src-tauri/src/runtime.rs:235`
- Error style: `src-tauri/src/vault/error.rs:11`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Store core

#### Automated

- [x] 1.1 Store unit and integration tests pass: `cargo test` in `src-tauri/` — b1f773c
- [x] 1.2 Clippy is clean: `cargo clippy --all-targets -- -D warnings` — b1f773c
- [x] 1.3 Formatting is clean: `cargo fmt --check` — b1f773c
- [x] 1.4 No advisories for the new dependency: `cargo audit` — b1f773c

### Phase 2: Daily backups

#### Automated

- [x] 2.1 Backup tests pass: `cargo test --test store_backup` in `src-tauri/` — 987ad56
- [x] 2.2 Full suite, clippy and fmt pass: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` — 987ad56

### Phase 3: Crash-proof tests

#### Automated

- [x] 3.1 Crash test passes locally and is not ignored in the default run: `cargo test --test store_crash` in `src-tauri/` — 43ce13c
- [x] 3.2 The writer helper alone is skipped in a normal run (reported as ignored, not failed): `cargo test --test store_crash` — 43ce13c
- [x] 3.3 Full suite, clippy and fmt pass: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` — 43ce13c

#### Manual

- [ ] 3.4 The crash test passes in CI on `windows-latest` (check the PR/push run)

### Phase 4: App wiring and tray health

#### Automated

- [x] 4.1 Health and store tests pass: `cargo test` in `src-tauri/` — 2811847
- [x] 4.2 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo audit` — 2811847
- [x] 4.3 UI gates still pass: `npm run lint`, `npm run format:check`, `npm run build` — 2811847

#### Manual

- [ ] 4.4 `npm run tauri dev`: the health panel lists "Dane aplikacji" as OK, and `%APPDATA%\com.sebas.day-navigator\` holds `day-navigator.db` and `backups\day-navigator-<today>.db`
- [ ] 4.5 With the app quit, overwrite `day-navigator.db` with a text file and start the app: the tray shows the source error with "plik danych uszkodzony", the health panel shows the Polish message, other sources are unaffected and the text file is unchanged
- [ ] 4.6 Quit the app, copy today's backup over `day-navigator.db` (delete `-wal`/`-shm`), start again: "Dane aplikacji" is OK
