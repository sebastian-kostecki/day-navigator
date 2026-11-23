<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: App Data Store Implementation Plan

- **Plan**: context/changes/app-data-store/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 6 observations

Note: phases 3 and 4 have Manual rows still unticked (3.4, 4.4–4.6). Their automated work is done and is reviewed here; the manual rows are counted as pending (see F8).

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | WARNING |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

Automated gates re-run during this review: `cargo fmt --check` ok, `cargo clippy --all-targets -- -D warnings` ok, `cargo test` ok (store 15, store_backup 7, store_crash 1 passed + 1 ignored writer, health_probe 12, lib 107), `cargo audit` exit 0 (6 allowed warnings, none in rusqlite/libsqlite3-sys), `npm run lint` / `format:check` / `build` ok.

Plan drift: every planned file, contract and test case is present. Two harmless differences are not findings. Leftover cleanup removes only `.day-navigator-*.tmp` (narrower than "leftover `.tmp` files", and safer). The AGENTS.md block is its own `## App Data Store` section rather than a subsection.

## Findings

### F1 — A refused WAL-mode file can still be written on close

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/mod.rs:83
- **Detail**: The `quick_check` and `user_version` refusals run on a normal read-write connection. When `open` refuses the file (`Damaged`, `NewerSchema`), the connection is dropped, and closing the last connection to a WAL-mode database with frames in `-wal` checkpoints them into the main file. Every file this app (or a newer version) creates is in WAL mode, and Windows shutdown killing the tray app is the usual way a non-empty `-wal` is left behind. So the realistic "newer app wrote it, older app opens it" case changes the file's bytes. For a `Damaged` file, SQLite writes into a corrupt file. The "bytes unchanged" tests (tests/store.rs NewerSchema, store_backup.rs refused-store) use rollback-journal fixtures on purpose, so they never cover this case. Plan contract: "Every refusal happens before anything writes to the file, so a refused store stays byte-identical."
- **Fix A ⭐ Recommended**: Set `SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE` on the connection right after open and clear it once the checks pass. Add a test with a WAL-mode, newer-schema file that has a non-empty `-wal` and must stay byte-identical.
  - Strength: Small, local change (rusqlite `set_db_config`). It removes the checkpoint write, which is the one write path into the main file during a refusal.
  - Tradeoff: A hot rollback journal is still rolled back at first read. That is SQLite's own recovery, and it does not happen with WAL files.
  - Confidence: MED — documented SQLite flag; not tried in this repo yet.
  - Blind spot: Whether rusqlite 0.43 exposes this flag in `DbConfig` with the bundled feature set.
- **Fix B**: Run the checks on a connection opened with `SQLITE_OPEN_READ_ONLY` (no `CREATE`), then reopen read-write for the pragmas, backup and migrations.
  - Strength: Guarantees by construction that nothing writes the main file before a refusal.
  - Tradeoff: Two opens per start. A read-only connection that meets a hot journal fails with `READONLY_*`, which maps to `Io` and gives a misleading message.
  - Confidence: MED — needs care with WAL read-only rules (`-shm` handling).
  - Blind spot: Behaviour of read-only WAL opens when `-shm` is missing.
- **Decision**: FIXED (Fix A) — NO_CKPT_ON_CLOSE set right after open and cleared before the WAL switch (store/mod.rs); regression test newer_wal_store_with_pending_frames_is_refused_and_file_untouched in tests/store.rs failed before the fix and passes after.

### F2 — A backup failure can log an absolute path

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/backup.rs:45, :61
- **Detail**: `write_backup` maps a `VACUUM INTO` error through `error::reason_of`, which keeps SQLite's message. SQLite runs `VACUUM INTO` as `ATTACH %Q` (bundled sqlite3.c:162398). A failed attach reports `"unable to open database: %s"` with the file name (sqlite3.c:126145), here the full temp path under `%APPDATA%`, which contains the Windows user name. Line 45 logs it. This contradicts the plan ("error kind and file names only"), the module doc and the AGENTS rule that store errors carry no paths. No test reaches this path: the "backups is a file" test fails earlier, at `create_dir_all`.
- **Fix**: In `write_backup`, map the `VACUUM INTO` error to its code only (`SqliteFailure(err, _) → format!("{:?}", err.code)`), not `reason_of`.
- **Decision**: FIXED — VACUUM INTO failures map to the SQLite error code only (store/backup.rs).

### F3 — Startup store open blocks the main thread

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src-tauri/src/lib.rs:188
- **Detail**: `setup` calls `StoreHandle::status()` on the main thread. On the first start of a day that runs `quick_check` (reads the whole file), `VACUUM INTO` (copies it), any pending migrations, and up to 5 s of `busy_timeout`, all before the event loop and tray start. The plan asked for this ("try one open at startup"), so it is a plan flaw. It is also redundant, because `check_loop` runs a round straight away, and that round opens the store on the check thread. It also bends the AGENTS rule that store work runs off sync paths. Cheap today with an empty schema, but the cost grows with the data.
- **Fix**: Remove the startup `status()` call (and its comment); the first health round opens the store on the check thread.
- **Decision**: FIXED — startup status() call removed from setup (lib.rs); the first health round opens the store on the check thread. This deviates from the plan contract "try one open at startup" on purpose.

### F4 — A lasting store failure logs a warning every 30 s

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/store/mod.rs:219
- **Detail**: `ensure` logs `store: open failed: <kind>` on every failed open. A lasting `Damaged`/`Missing`/`NewerSchema` state therefore logs about 2.9k lines a day, which pushes other diagnostics out of the rotating log. The health runtime already logs only transitions (`health: app-data - -> Damaged`, runtime.rs:308-316), so this line duplicates it. A `Damaged` file also gets a full `quick_check` every round, which is acceptable while the store is small.
- **Fix**: Downgrade the `ensure` open-failure log to `log::debug!`; the transition log in runtime already records the state change.
- **Decision**: FIXED — open-failure log in StoreHandle::ensure downgraded to log::debug! (runtime keeps logging transitions).

### F5 — Backup file is not fsynced before it counts as today's copy

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/backup.rs:59-65, :41
- **Detail**: The `VACUUM INTO` output is hard-linked to its final name without `sync_all`. Later, `exists()` accepts any file with today's name as done. After a power loss a truncated backup can stand as the day's copy. The vault atomic write syncs before persisting (vault/fsops.rs:238-239).
- **Fix**: `fs::File::open(&tmp)?.sync_all()` before `hard_link`.
- **Decision**: FIXED — temp backup is flushed with sync_all (write handle, as Windows requires) before hard_link; a failed flush removes the temp file.

### F6 — Backup name parsing is duplicated; retention trusts future dates

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/mod.rs:35-46, src-tauri/src/store/backup.rs:25-29, :87-91
- **Detail**: `has_dated_backup` (the `Missing` refusal) re-implements `backup::dated`, so the two rules can drift apart. Retention sorts only by the date in the name. If the clock was ever set forward, future-dated backups hold the newest-7 slots for good, and with 7 of them today's new backup is pruned right after it is written. This is a theoretical edge case.
- **Fix**: Make `has_dated_backup` use `backup::dated`, and in `prune` exclude dates after `today` from the kept-7 count (never delete them, just don't let them take up slots).
- **Decision**: FIXED — has_dated_backup reuses backup::dated; prune(dir, today) keeps future-dated backups without counting them; test future_dated_backups_never_push_out_todays_copy in tests/store_backup.rs.

### F7 — "Panic rolls back" relies on unwinding; release builds abort

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/mod.rs:147-148, src-tauri/Cargo.toml:59
- **Detail**: The release profile sets `panic = "abort"`. The `write` doc's "any Err (or panic) rolls back" relies on `Transaction::drop`, which runs only when a panic unwinds. The panic-rollback test therefore proves it only in debug builds. In release, SQLite discards the uncommitted transaction at the next open, so the guarantee still holds by another route, and the crash test covers that route. The poisoned-mutex recovery is also dead code in release.
- **Fix**: Extend the `write` doc comment: in release builds a panic aborts and SQLite discards the open transaction on the next open (covered by tests/store_crash.rs).
- **Decision**: FIXED — write() doc explains the panic = "abort" release path.

### F8 — Manual checks pending; restore test does not use a real backup

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/app-data-store/plan.md:333, :345-347; src-tauri/tests/health_probe.rs:191-234
- **Detail**: 3.4 (crash test on CI `windows-latest`) cannot be ticked yet: `main` is 31 commits ahead of `origin/main`, so no CI run exists for these commits. 4.4–4.6 (manual app runs) are unticked; none were falsely ticked. Separately, the "restore from backup" recovery test copies a live WAL-mode database from another run, not a `VACUUM INTO` file from `backups/`, so the AGENTS.md restore path is not covered end to end. `runtime::probe_round` (store appended even when settings fail) has no test; the plan did not ask for one.
- **Fix**: Make the recovery test restore from the `backups/day-navigator-<date>.db` file the store itself wrote. Then push and tick 3.4, and run 4.4–4.6 by hand.
- **Decision**: FIXED (test part) — recovery test in tests/health_probe.rs now restores from the backups/ file written by a first run. Still open for the user: push to get a CI run for 3.4, and run manual checks 4.4–4.6.

### F9 — Crash harness can leave a runaway writer

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/tests/common/crash.rs, src-tauri/tests/store_crash.rs:41-59
- **Detail**: `run_writer_until` kills and waits for the child on its own panic paths, but has no kill-on-drop guard. If the parent test is aborted from outside (IDE stop, runner timeout), the writer keeps inserting 64 KB rows into the temp dir with no limit. Theoretical, but the harness is meant for reuse by later data slices.
- **Fix**: Wrap the `Child` in a guard that kills and waits on `Drop`, and cap the writer loop (e.g. 10 000 commits).
- **Decision**: FIXED — KillOnDrop guard in tests/common/crash.rs; the writer is capped at MAX_WRITER_COMMITS (2000) per run.

## Triage Summary

- Fixed: F1 (Fix A), F2, F3, F4, F5, F6, F7, F8 (test part), F9 (9)
- Open for the user: push so CI runs the crash test (3.4); manual checks 4.4–4.6
- Gates after fixes: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (store 16, store_backup 8, health_probe 12, store_crash 1 + 1 ignored) all green; no dependency or frontend change.
