<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Windows autostart into the tray

- **Plan**: context/changes/windows-autostart/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 2 warnings, 4 observations

Diff reviewed: `d34904a..HEAD` on `slice/windows-autostart` (p1 `b27f063`, p2 `f6b4cbd`, p3 `8eb30de`, plus the chore commits that record SHAs). Run non-interactively: no triage happened, every decision is PENDING. `change.md` was left as it is because the task allowed committing only this report. The orchestrator sets `status` when it closes the slice.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING (F2) |
| Scope Discipline | WARNING (F2: unplanned migration, needed and harmless) |
| Safety & Quality | WARNING (F1) |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS (automated green; manual rows 2.10–2.18 pending, as planned) |

## What was checked and holds

- **No test path to the real registry.** `HkcuRun` is named only in `autostart/hkcu.rs` (in a private `mod hkcu`, so `tests/` cannot name it) and `autostart/startup.rs`. Neither file has a `cfg(test)` module. Only `startup::start` reaches `HkcuRun`, it needs a live `AppHandle`, and no unit or integration test calls it. `apply = !cfg!(debug_assertions)` keeps debug builds and `cargo test` away from the registry, and the release path is still compiled and linted. `run` returns before any `entry` call when the block is invalid or `apply` is false, and tests cover both cases with a call counter.
- **Panic safety.** No `unwrap`/`expect`/indexing in `hkcu.rs` or `startup.rs`. Both mutex locks use `unwrap_or_else(|p| p.into_inner())`. `store`, `status` and `runtime::refresh_tray` all use `try_state`. `Manager::manage` returns a bool and does not panic. The `unwrap_or_default()` in `run` (`autostart/mod.rs:142`) cannot panic. A failed thread spawn stores `Failed { Thread, .. }` before `runtime::start`, so the first round reports it.
- **Value and comparison.** `expected_value` gives `"<exe>" --autostart`, quoted, and a test uses an invented path that contains a space. A non-UTF-8 path gives `Failed { ExePath, InvalidData }` (tested on Windows). The comparison is an exact string compare. A different value is overwritten and an equal one is left alone. `NotFound` on the key or the value maps to `Ok(None)` / `Ok(())` in both `read` and `remove` (`hkcu.rs:18-27, 38-47`). A read error writes and removes nothing (tested for on and off).
- **Start-up flow.** `startup::start` runs before `runtime::start` (`lib.rs:171-172`). The wake from `refresh_tray` goes into the channel `build_tray` already created, so a sync that finishes before the health thread starts is still picked up. If the first round misses the source, the later round compares it as Unknown, so Unknown → `Error` still toasts. `show_main` is skipped when `--autostart` is an exact argument after the program name (`lib.rs:179-182`). The single-instance callback skips the program path and does not show the window for an autostart relaunch (`lib.rs:137-142`). The window-state flags are unchanged.
- **Contracts and AGENTS.md.** `SourceId::Autostart` is wired in all three places (`events::source_from_key`, `tests/events.rs` match and list, TS `EventSourceId`), in the same commit as the Rust type. No new command or error enum crosses IPC, so the `kind`/`detail` rule does not apply. Settings stay read-only. Messages and logs carry only a fixed phrase, the `FailedStep` name and the `io::ErrorKind` name, and a test asserts that no invented path fragment appears in the serialized status. No plugin or npm package is added, and capabilities, `tauri.conf.json` and the CSP are unchanged. The AGENTS.md exception is narrow and matches the code: release build only, one value, write while on, remove while off, no `StartupApproved`. The new Project Structure bullet names `autostart/` as the only registry code.
- **Migration v12** (`store/schema.rs:252-276`). Columns, types, every CHECK and the `INTEGER PRIMARY KEY` match v10 exactly. The only change is that the `source_id` list gains `'autostart'`. v10 has no index or trigger on `events` and no foreign key points at it, so nothing is lost. The copy names every column explicitly and keeps `id`. The runner applies each migration inside one `IMMEDIATE` transaction together with `PRAGMA user_version` (`store/mod.rs:124-139`). Without the migration the first Autostart transition would fail the CHECK, so the deviation was necessary. `every_source_id_round_trips` records an Autostart event on a fully migrated store and would catch a later events rebuild that drops the key.
- **Ease of renumbering.** `12` appears only in `version: 12` at `schema.rs:256`. Existing upgrade tests locate their migrations by SQL content, not by number. `upgrade_from_v9_keeps_rows_and_reaches_latest` searches for `"CREATE TABLE events"`, which also matches `"CREATE TABLE events_new"`, but `position` returns the first match (v10), so it stays correct. Renumbering to v13 means changing that one literal and the comment.
- **Merge risk.** The edits to shared files are small and local: one `mod` and one `pub use` line in `settings.rs`; one variant, key, toast arm and test in `health/mod.rs`; one parameter, one `extend` and one argument in `runtime.rs` `probe_round`/`check_round` (S-07 edits `run_tray_action`); README lines placed between `todo` and `reminders`; a separate PRD line. The only real merge conflict is the migration number (S-02), and the integrator knows about it.
- **Docs.** README, CHANGELOG and PRD match the behaviour: off by default, read at start, one Run value, Task Manager respected and the row still OK, how to clean up before uninstalling. The PRD drops only the word "autostart" and adds the MS-17 line.

## Success criteria evidence (run 2026-11-27 in the worktree)

- `cargo fmt --check`: OK. `cargo clippy --all-targets -- -D warnings`: OK. `cargo test`: all suites green (lib 574 passed; 28 tests match `autostart`).
- `npm run lint`, `npm run format:check`, `npm run build`: OK.
- `git grep` checks: no `tauri` in `autostart/mod.rs` / `settings/autostart.rs`; `winreg|HKEY_` only in `hkcu.rs`; `HkcuRun` only in `hkcu.rs` / `startup.rs`; no `cfg(test)` in either; the only file changed under `src/`, `capabilities/` and `tauri.conf.json` is `src/events/types.ts`.
- `cargo tree -i winreg -e normal`: `day-navigator` is the only dependent. `Cargo.lock` adds only the `winreg` edge.
- Not re-run: `cargo audit` (needs network; the lock gains no package).
- Manual rows 2.10–2.18 are unchecked, as the plan expects (they need an installed release build and a Windows sign-in). None is falsely ticked.

## Findings

### F1 — Migration v12 has no upgrade test from v11 with existing rows

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/schema.rs:252 (test belongs in src-tauri/tests/events.rs)
- **Detail**: The table rebuild (create `events_new`, copy, drop, rename) is the only migration in this slice, and it rewrites user data. In this repo every migration that changes data has an upgrade test that opens the store at the previous version, writes rows and reopens at latest: `upgrade_from_v9_keeps_rows_and_reaches_latest` (events v10), `note_migration` in `tests/work_time.rs` (v11), and the tests in `tests/new_assignments.rs` and `tests/todo.rs`. v12 is covered only on a fresh store (`every_source_id_round_trips`). Nothing checks that today's rows survive the copy with their `id`, `at` and fields, that the old CHECKs still reject bad values, or that the copy matches column for column after the integrator renumbers it to v13.
- **Fix**: Add a test to `tests/events.rs`. Locate the migration by content (`MIGRATIONS.iter().position(|m| m.sql.contains("CREATE TABLE events_new"))`, never by number, so renumbering to v13 needs no test change). Open with `MIGRATIONS[..before]` and insert one item event (for example `ticked`, own, invented title) and one `sourceError` for `'reminders'`. Show that `'autostart'` is refused at that version. Reopen with the full list and assert: `user_version == MIGRATIONS.len()`, both rows are present with the same `id` and fields (also through `events::list_today`), an Autostart `sourceError` now records, and a bogus `source_id` / `kind` is still refused.
- **Decision**: FIXED (commit: see git log, `fix(windows-autostart): impl-review fixes`)

### F2 — The plan still says "no store migration"

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: context/changes/windows-autostart/plan.md:64 and :328
- **Detail**: "What We're NOT Doing" says "No new FR, no store migration", and "Migration Notes" says "No store change." The implementation added migration v12 in p1 (`b27f063`). It was needed, because the v10 `source_id` CHECK refuses `'autostart'`, which research and the plan review both missed. The deviation is recorded only in the p1 commit message. The plan is the source of truth for the integrator (who must renumber it) and for archive, so it should say so.
- **Fix**: Add an addendum to the plan. Under "Migration Notes", add: "Migration v12 (to be renumbered on merge; S-02 also claims v12) rebuilds `events` so its `source_id` CHECK accepts `'autostart'`; same columns and CHECKs otherwise; found in p1." Change the NOT-doing bullet to "No new FR, no roadmap edit". Add the `store/schema.rs` file to Phase 1 §2 "Changes Required".
- **Decision**: FIXED (same commit)

### F3 — Before merge, check that no local database was migrated by this branch

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/schema.rs:256
- **Detail**: A debug or dev build of this branch uses the same bundle identifier, so it uses the same app data dir as the installed app. If any build of this branch ever opened the real `day-navigator.db`, that database is at `user_version = 12` with the events rebuild. After the integrator moves this migration to v13 and S-02 takes v12, such a database would skip S-02's v12 forever and then run v13. The schema would drift silently. This is not a code defect, and the risk applies to every slice that adds a migration in parallel.
- **Fix**: At merge, confirm that the real database's `PRAGMA user_version` is ≤ 11 (wave 1 level). If it is not, restore a backup as AGENTS.md describes before running the merged build.
- **Decision**: NOTED, deferred to the integrator (check `PRAGMA user_version` <= 11 at merge)

### F4 — README "Privacy and safety" does not mention the registry write

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: README.md:71-73
- **Detail**: The "Local data" bullet lists where the app keeps data (config folder, app data folder). The new Run value is the app's only write outside its folders. It is documented in Configuration (README.md:129) but not where a reader checks what the app touches. The plan did not ask for this, so this is optional.
- **Fix**: Append to the "Local data" bullet: "With `autostart` on, the app also keeps one entry in the current user's Windows Run key (see Configuration)."
- **Decision**: FIXED (same commit)

### F5 — A Run value of a non-string type is reported on every start and never repaired

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/autostart/hkcu.rs:23-27
- **Detail**: `get_value::<String, _>` fails with `InvalidData` when a value named `com.sebas.day-navigator` exists with a non-string type (for example REG_DWORD or REG_BINARY). That gives `Failed { Read, InvalidData }`, and A7 means nothing is written. The message promises "Aplikacja spróbuje ponownie przy następnym uruchomieniu", but every later start fails the same way until the user deletes the value by hand. This is very unlikely, because only this app writes that name. It is never silent, since the error is surfaced. It is consistent with A7.
- **Fix**: None needed. If it matters, `FailedStep::Read` with `InvalidData` could use a phrase that tells the user to remove the entry in Task Manager instead of promising a retry.
- **Decision**: NOTED, no change needed (consistent with A7)

### F6 — Progress rows 2.8 and 2.9 have no commit SHA

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/windows-autostart/plan.md:402-403
- **Detail**: Every other ticked Automated row ends with ` — <sha>`. 2.8 (`winreg` normal dependency) and 2.9 (no frontend/config change) do not. Both checks pass today (see the evidence above), so this is bookkeeping only.
- **Fix**: Append ` — f6b4cbd` to rows 2.8 and 2.9.
- **Decision**: FIXED (same commit)
