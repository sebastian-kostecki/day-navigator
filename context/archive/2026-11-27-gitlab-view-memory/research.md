---
date: 2026-11-27T00:00:00+01:00
researcher: Claude (Opus 5.5, agent)
git_commit: 2b6ac0861b564e2e096afc50f39e343f1fa41d11
branch: slice/gitlab-view-memory
repository: day-navigator
topic: "Remembered GitLab view state: persist the type choice and the hidden toggle of Praca (GitLab) across restarts"
tags: [research, gitlab, store, migration, ui-state, frontend]
status: complete
last_updated: 2026-11-27
last_updated_by: Claude (Opus 5.5, agent)
---

# Research: Remembered GitLab view state

**Date**: 2026-11-27 (time of day not recorded; timezone Europe/Warsaw assumed)
**Researcher**: Claude (Opus 5.5, agent, non-interactive run)
**Git Commit**: 2b6ac0861b564e2e096afc50f39e343f1fa41d11
**Branch**: slice/gitlab-view-memory
**Repository**: day-navigator

## Research Question

Roadmap M-4, S-05 "Remembered GitLab view state" (change-id `gitlab-view-memory`, anchor MS-34, GitHub issue #34): after the app is quit and started again, "Praca (GitLab)" should show the same type choice and the same expansion of the hidden section. The view filter (project, label and milestone values) keeps resetting. What does the codebase offer and require for this?

Fixed user decisions (given to the agent, not open):

- Remember only the type and the hidden expansion. No filter values, because they are work data. No work data is stored.
- Store it in the app data store (`store::StoreHandle`). This slice owns the wave's only migration, v14.
- Update AGENTS.md in the same change: the app-data list and the "in memory only" sentence.
- No new FR in the PRD; MS-34 is the anchor.

The empty-intent prompt of `/10x-new-research` was not shown: the run was non-interactive, so the question came from the orchestrator's brief (default chosen by agent). `change.md` Notes keep the template comment, because `/10x-new` got no freeform intent.

## Summary

- **What is held today.** The state to remember is two fields of `useGitlabListViewSource` in `src/gitlab/useGitlabListView.ts:28-31`, both plain `useState` and commented "In memory only":
  - `kind` (`KindFilter = "all" | "issue" | "mergeRequest"`, default `"all"`, `:4`, `:29`);
  - `showHidden` (default `false`, `:30`).
- **What "Ukryte expansion" means.** It is the `showHidden` flag behind the toolbar button "Pokaż ukryte (N)" (`src/gitlab/GitlabView.tsx:515-523`). It adds the hidden rows to the table (`:461`). There is no separate `<details>` today.
- **Type has three values.** The roadmap names only "issues/MRs", but the default is `"all"` ("Wszystko", `GitlabView.tsx:84-86`).
- **When the state resets.** It already survives tab switches: the provider wraps every view in `src/App.tsx:77-84`. It also survives closing the window, because close hides the window and keeps the view mounted (`src-tauri/src/lib.rs:186-197`, `:239-248`). It resets only when the process ends ("Zakończ", reboot, autostart launch). So the slice only needs to load on start and save on change.
- **No key/value table exists.** All 13 migrations (`src-tauri/src/store/schema.rs:9-286`) create typed tables with CHECKs. The closest precedent is `gitlab_view_marks` (v9, `schema.rs:209-220`): one row per opaque account key, `WITHOUT ROWID`.
- **v14 and the tests.**
  - Appending v14 breaks two hard-coded asserts in `src-tauri/tests/events.rs`: `:184` (`MIGRATIONS.len() == 13`) and `:200` (`user_version == 13`).
  - The other version asserts in the inspected tests use prefixes (`MIGRATIONS[..k]`) or `MIGRATIONS.len()`/`.last()`, so they hold.
- **The seen-mark path is fire-and-forget.** Rust: the `mark_gitlab_seen` command (`src-tauri/src/gitlab_refresh.rs:407-431`) calls `new_assignments::mark_view_seen` (`src-tauri/src/new_assignments/mod.rs:102-118`) and logs errors by kind. TS: `useGitlabSeenMark.ts:5-8` swallows the error.
  - The new record differs in one way: it must also be **read** at start. No command reads the seen mark directly (`gitlab_snapshot`, `gitlab_refresh.rs:392-397`, never touches the store).
- **`mark_gitlab_seen` sits in the file F-01 rewrites.** The new commands should not go into `gitlab_refresh.rs` (F-01 refactors that file; roadmap `:88-99`). A small new module is the conflict-free choice.
- **What is shared with the parallel slices.** Two files are shared: `lib.rs` (S-05 adds handler lines in `generate_handler!`; F-01 may touch `setup` at `:167-179`) and `AGENTS.md`. S-05 can stay out of `GitlabView.tsx`, which S-04 edits, if the change stays inside `useGitlabListView.ts`.

## Detailed Findings

### Current UI state and its consumers

- `src/gitlab/useGitlabListView.ts`:
  - `:6-14`: the `GitlabListView` contract is `{ kind, setKind, showHidden, setShowHidden(updater), selection, toggle, clear }`.
  - `:27-48`: `useGitlabListViewSource` is the single source, called only by the provider. `setShowHidden` takes an updater function (`:10`), and the view calls it as `setShowHidden((s) => !s)` (`GitlabView.tsx:520`).
- `src/gitlab/GitlabListViewProvider.tsx:1-8` is a thin wrapper. Its comment (`:4`) says "kind, hidden, view filter" and needs an update.
- `src/gitlab/GitlabView.tsx`:
  - `:444-445`: `Lists` reads the state.
  - `:509-514`: the "Typ" `Segmented`.
  - `:515-523`: the "Pokaż ukryte" button. It is `disabled={hiddenCount === 0 && !showHidden}`. If `showHidden` is remembered as `true` and there are no hidden issues, the button stays enabled, so the user can turn it off.
  - `:461`, `:471-473`: `kind` and `showHidden` also drive the counts in the footer note.
  - `:360`: `ClosedBlock` filters by `kind`.
- `Lists` renders only when `items && state.state !== "notConfigured" && !offline` (`GitlabView.tsx:697`). S-04 changes exactly this condition.
- `src/App.tsx:77-84`: the provider sits inside `WorkTimerProvider` and around all six views. It mounts at app start, before the GitLab tab is first opened.

### Prior decisions (history)

- The provider was introduced in `context/archive/2026-11-25-gitlab-issue-labels/plan.md`:
  - `:14`: before it, type and "Pokaż ukryte" were `useState` in the view.
  - `:24`: they survive a tab switch and reset to defaults after a restart.
  - `:44`, What We're NOT Doing: no saving of view-filter choices to disk (settings, database), no migration.
  - `:202-206`: the contract.
  - `:297`: "Wybory filtra widoku są tylko w pamięci."
- Inspected archive documents give no explicit reason for keeping `kind`/`showHidden` in memory; it was a scope cut. The work-data reason covers only the filter values:
  - `context/archive/2026-11-23-gitlab-visibility-filter/plan.md:72`;
  - `context/archive/2026-11-24-gitlab-filter-bar-retry/change.md:12`;
  - roadmap Parked, `context/foundation/roadmap.md:216`.
- The precedent for adding a small view record to the AGENTS.md app-data list is `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan.md:254` (the seen-until mark, v9).
- Hidden issues once sat in a collapsed `<details>` "Ukryte (N)" (`context/archive/2026-11-23-hide-and-resurface/plan.md:10`, `:29`, `:383`). Its expansion was never persisted, and the current UI is the toolbar toggle above.

### Roadmap, issue and AGENTS.md text

- Roadmap (`context/foundation/roadmap.md`):
  - MS-34 (`:36`): type choice and hidden expansion are remembered; the view filter stays in memory, "so no work data is stored".
  - S-05 entry (`:154-164`): Parallel with F-01, S-01, S-02, S-03, S-04, S-06. Risk: "a migration and a new kind of record in the app data dir (no work data); the AGENTS.md list of app data and the 'in memory only' rule change with it".
  - `:72` and `:81`: one migration per wave; the store is at v13, so S-05 takes v14.
  - `:208`, open question 3: S-05 updates the AGENTS.md lines.
- Issue #34 (read with `gh issue view 34`): same problem statement. The roadmap chose the "type + Ukryte only" alternative.
- AGENTS.md lines that must change:
  - Line 11 (Hard Rules), the app-data list. Today it contains "… the GitLab view's seen-until mark (opaque account key, one instant), …" and needs a new entry for the remembered view choices (type, hidden toggle; no work data).
  - Line 28 (Frontend Conventions): "The list state of "Praca (GitLab)" (type, hidden, view filter) is one `GitlabListViewProvider` … in memory only." It must say that type and hidden are remembered in the app data store and the view filter stays in memory.
- PRD (`context/foundation/prd.md`): nothing about remembering view state. FR-001 (`:122`) and FR-003 (`:126`) are the referenced FRs. Guardrails `:77`, `:193`, `:199`: work data never goes into repo, tests or logs.

### App data store: API, migrations, errors

- `src-tauri/src/store/mod.rs`:
  - `Migration { version, sql }` (`:25-28`).
  - `Store::read` (`:142-147`) and `Store::write` (`:149-164`): `write` is one IMMEDIATE transaction that commits on `Ok` and rolls back on `Err` or panic.
  - `StoreHandle::with_store(today, f)` (`:241-250`) opens the store lazily under a mutex, always with `schema::MIGRATIONS` (`:223`).
- Open order (`mod.rs:47-122`):
  1. validate the migration list;
  2. refuse a zero-byte file (`Damaged`) or a missing file next to backups (`Missing`);
  3. `quick_check`, otherwise `Damaged`;
  4. refuse a newer `user_version` before any write (`NewerSchema`);
  5. WAL / `synchronous=FULL` / `foreign_keys=ON`;
  6. `backup_if_due`;
  7. migrate. Each pending migration runs in its own transaction with `PRAGMA user_version = N` (`:124-140`). A failed migration keeps the previous version (`tests/store.rs:228-258`).
- `validate` (`schema.rs:289-300`) requires versions 1..N, contiguous. v14 is the next valid number.
- `StoreError` (`src-tauri/src/store/error.rs:12-29`): `DirUnavailable`, `Damaged`, `Missing`, `NewerSchema`, `MigrationFailed`, `Busy`, `Io`, `Sql`. Variants carry no values (`:3-5`); serde `tag = "kind", content = "detail"`.
- Two helpers in `lib.rs`:
  - `on_store` (`lib.rs:54-73`, `spawn_blocking` + `with_store` + local `now`) is used by the own-tasks commands (`:75-126`).
  - `crate::blocking` (`lib.rs:38-49`) is used by work-time and by `mark_gitlab_seen`.
  - Either satisfies the AGENTS.md rule "async command, `with_store` inside `spawn_blocking`".

### Small-record precedents

- **`gitlab_view_marks` (v9).**
  - Schema: `account TEXT PRIMARY KEY, seen_until_ms INTEGER NOT NULL) WITHOUT ROWID` (`schema.rs:209-220`). `account` is the opaque hash from `gitlab::account_key`, with no REFERENCES.
  - Write: one upsert in `Store::write` (`new_assignments/mod.rs:102-118`).
  - Read: only inside `record_round` (`:166-172`).
  - Error type: `NewAssignmentError { Store, Background }` (`src-tauri/src/new_assignments/error.rs:10-46`, tagged serde). It has **no TS mirror** in `src/gitlab/types.ts`, which mirrors only `HiddenIssueError` and `OpenSettingsError` (`:122-131`).
- **`days_off` (v5 + v12).**
  - Schema: one row per day, `kind` CHECK `'full'|'half'` (`schema.rs:93-100`, `:255-258`).
  - Domain function: `set_day_off` (`src-tauri/src/work_time/report.rs:265-300`) validates first, then one `write` that deletes or upserts.
  - Command: `work_time/commands.rs:238-254`, through `execute` (`:81-107`). Store errors are logged at `warn`, refusals at `debug`.
- **`assignment_accounts` (v7)** (`schema.rs:130-135`): one row per account, no command of its own.
- **`settings.json` is not an option.** It is read-only for the app:
  - `src-tauri/src/settings.rs:1` describes it as a read-only loader for the hand-written file;
  - `src-tauri/src/settings_file.rs:1-3`, `:43-53`: the app only opens it in the default program;
  - production code never writes it (every `fs::write` found is in `#[cfg(test)]`);
  - settings structs use `deny_unknown_fields` (`settings.rs:94` and others), so an app-written key would make the file invalid.

### Frontend load/save patterns

- `src/gitlab/useGitlabSeenMark.ts:5-8`: fire-and-forget; `markGitlabSeen().catch(() => {})`, and Rust logs the failure. It never blocks rendering.
- `src/own-tasks/useOwnTasks.ts:12-64` (as reported by the sub-agent): a tagged `loading`/`error`/`loaded` state, latest-request-wins, and a failed reload never replaces loaded data.
- `src/worktime/useDayOffToggle.ts:22-56` (as reported by the sub-agent): an optimistic pending state kept until the reload agrees; a failed write drops it and shows an inline error.
- `src/gitlab/api.ts:15-18`: `markGitlabSeen` is the template for a typed `invoke` wrapper. AGENTS.md says only `api.ts` imports `invoke`.

### Tests a v14 slice must touch or add

- **Must update:**
  - `src-tauri/tests/events.rs:184`: `assert_eq!(schema::MIGRATIONS.len(), 13);`;
  - `src-tauri/tests/events.rs:200`: `assert_eq!(scalar(&store, "PRAGMA user_version"), 13);`.
  - Fix: use `MIGRATIONS.len()` (as `as i64`, matching the scalar type), or bump to 14. `:182-183` (versions 12 and 13 found by content) stay valid.
- **Unaffected:**
  - prefix-based hard-coded numbers: `events.rs:69`, `:187`; `new_assignments.rs:209`, `:213`; `work_time.rs:393`, `:620-633`; `hidden_issues.rs:100`, `:110-119`; `todo.rs:277`, `:311`;
  - generic checks: `store/mod.rs:334`, `tests/health_probe.rs:209`, `schema.rs:322`, `tests/store.rs:287`, and others.
  - Neither group is affected by an append, as reported by the sub-agent from code reading; not run.
- **Upgrade-test convention.** Find the migration by SQL content, never by number (`events.rs:100-104`). Open `&MIGRATIONS[..pos]`, write old rows, reopen with the full list, and assert `user_version == MIGRATIONS.len()` and that the rows were kept. Example: `new_assignments.rs:203-237` (v8→v9, the seen mark).
- **Crash harness** (`src-tauri/tests/common/crash.rs`):
  - API: `run_writer_until(test_name, dir, KillAt)` (`:69-140`), `KillAt::AfterCommits` / `AfterDelay` (`:42-47`), `MAX_WRITER_COMMITS` (`:53`), `WRITER_DIR_ENV` (`:38`).
  - Typical slice test (`work_entry_note_crash.rs:65-141`, `half_day_off_crash.rs:57-137`): an `#[ignore]` `writer_process` loops the real domain function and prints `committed n`.
  - The parent kills it 8-10 times at mixed points, reopens, and asserts `quick_check == "ok"`, no lost commit, and rows equal to a model.
  - The seen mark (`gitlab_view_marks`) has **no** crash test.
- `tests/common/mod.rs:11-15`: `test_store_dir()` gives a temp dir.

### Overlap with parallel slices (same wave)

| File | S-05 (this) | F-01 `source-refresh-skeleton` | S-04 `gitlab-offline-last-list` |
|---|---|---|---|
| `src-tauri/src/store/schema.rs` | appends v14 | no migration (roadmap `:88-99`) | no migration (`:142-152`) |
| `src-tauri/src/lib.rs` | `mod` line + handler lines in `generate_handler!` (`:198-236`) | possibly `setup` wiring `:167-179` (inferred, not stated) | — |
| `src-tauri/src/gitlab_refresh.rs` | avoid (holds `mark_gitlab_seen` `:407-431`) | refactors it and `github_refresh.rs` | — |
| `src/gitlab/useGitlabListView.ts`, `GitlabListViewProvider.tsx` | main UI change | — | — |
| `src/gitlab/api.ts`, `types.ts` | new wrappers / mirrors | — | — (not named) |
| `src/gitlab/GitlabView.tsx` | avoidable if the hook keeps its contract | — | `offline` `:602`, summary `:604-606`, card `:644`, `!offline` `:697` |
| `src-tauri/tests/events.rs` | two asserts `:184`, `:200` | — | — |
| `AGENTS.md` | lines 11 and 28 | maybe (not stated) | — |

## Code References

- `src/gitlab/useGitlabListView.ts:4,10,27-31` - `KindFilter`, updater setter, in-memory state
- `src/gitlab/GitlabListViewProvider.tsx:4` - comment listing kind/hidden/filter
- `src/gitlab/GitlabView.tsx:84-86,444-445,461,509-523,697` - type options, consumer, hidden rows, toolbar, list condition
- `src/App.tsx:77-84` - provider mount
- `src/gitlab/useGitlabSeenMark.ts:5-8` - fire-and-forget store call
- `src/gitlab/api.ts:15-18` - `markGitlabSeen` wrapper
- `src-tauri/src/lib.rs:38-73,186-197,198-236,239-248` - helpers, close-hides-window, handlers, exit rule
- `src-tauri/src/store/mod.rs:25-28,47-140,142-164,241-250` - API, open/migrate, read/write, `with_store`
- `src-tauri/src/store/schema.rs:209-220,264-285,289-300` - v9 seen mark, v13 last, validate
- `src-tauri/src/store/error.rs:3-29` - value-free errors
- `src-tauri/src/new_assignments/mod.rs:102-118` - `mark_view_seen` upsert
- `src-tauri/src/new_assignments/error.rs:10-46` - tagged error type
- `src-tauri/src/gitlab_refresh.rs:392-397,407-431` - snapshot (no store), seen-mark command
- `src-tauri/src/settings.rs:1`, `src-tauri/src/settings_file.rs:1-3` - settings are hand-written, read-only
- `src-tauri/tests/events.rs:184,200` - asserts broken by v14
- `src-tauri/tests/new_assignments.rs:203-237` - upgrade-test template
- `src-tauri/tests/common/crash.rs:38-140` - crash harness

## Architecture Insights

- Every app record is a typed SQLite table with CHECKs; single-value records are keyed rows in `WITHOUT ROWID` tables. A key/value table would be a new pattern in this schema.
- Fire-and-forget store writes from the UI already exist (seen mark). That precedent supports a non-blocking design in which a failed save or load falls back to defaults and Rust logs it by kind.
- Domain rules live in Rust (AGENTS.md); here the "rule" is only validation of the stored values (CHECK + Rust enum).
- Commands use tagged serde errors; a new error type, or reuse of an existing one, needs a TS mirror in the same commit if the UI reads it.

## Historical Context (from prior changes)

- `context/archive/2026-11-25-gitlab-issue-labels/plan.md:14,24,44,202-206,297` - provider created, in-memory by scope, no disk/migration then.
- `context/archive/2026-11-23-gitlab-visibility-filter/plan.md:72` - filter values are work data.
- `context/archive/2026-11-24-gitlab-filter-bar-retry/change.md:12` - same rule for the filter bar.
- `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan.md:254` - precedent for adding a view record to the AGENTS.md app-data list.
- `context/archive/2026-11-23-hide-and-resurface/plan.md:10,29,383` - earlier collapsed "Ukryte (N)" section, never persisted.

## Related Research

- `context/archive/2026-11-24-gitlab-new-and-hidden-date/` - seen-until mark (v9) design.
- `context/archive/2026-11-23-app-data-store/` - store foundations (not re-read here).

## Open Questions

These are options for `/10x-plan`; none is decided here.

1. **Table shape.**
   - (a) One fixed row, e.g. `gitlab_view_state (id INTEGER PRIMARY KEY CHECK (id = 1), kind TEXT NOT NULL CHECK (kind IN ('all','issue','mergeRequest')), show_hidden INTEGER NOT NULL CHECK (show_hidden IN (0,1)))`. Typed, matches the schema style.
   - (b) One row per opaque account key, like `gitlab_view_marks`. This needs the account key at load time; today it is known in `GitlabRefreshState` (`gitlab_refresh.rs`, which F-01 refactors) only after a round.
   - (c) A generic key/value table (`ui_prefs(key, value)`). This would be new in the schema and its values would be untyped.
   - Evidence favours (a) for simplicity, or (b) for multi-account symmetry.
2. **No row yet.** On first start, or right after the v14 upgrade, the defaults (`"all"`, `false`) come from Rust or from TS?
3. **Write policy.**
   - (a) Write on every change: two controls, low frequency, one small `write`.
   - (b) Debounce.
   - (c) Write on visit end or `main-hidden`, like `useGitlabSeenMark`. This risks losing the last change on a hard kill.
   - Whether to send both fields at once or one field per command.
4. **Failure behaviour.** A failed load must not block the view, which follows from the fixed decision.
   - Options: silently use defaults (seen-mark precedent) or show a quiet notice.
   - For a failed save: ignore and log by kind, or show an inline error (day-off precedent).
   - A `Damaged`/`Missing` store is already reported by the tray "Dane aplikacji".
5. **Load timing.** The provider mounts at app start (`App.tsx:77`), before the GitLab tab opens.
   - Options: load in the provider on mount and apply when it arrives (but never overwrite a choice the user made in the meantime), or gate the toolbar until loaded.
   - Watch StrictMode double effects.
6. **Command placement.**
   - (a) A new small module (e.g. `src-tauri/src/gitlab_view_state.rs` or `gitlab/view_state.rs`) with its own error type, or reuse of `StoreError` wrapped like `NewAssignmentError`.
   - (b) Inside `new_assignments` next to `mark_view_seen`.
   - Avoid `gitlab_refresh.rs` because of F-01.
   - Choose between `on_store` and `crate::blocking`.
7. **Test scope.**
   - Unit tests: round-trip, CHECK refusal of unknown kinds.
   - An upgrade test v13→v14 found by content.
   - Fix of `events.rs:184,200`.
   - Whether a crash test is worth it for a one-row upsert. No crash test exists for the comparable seen mark.
8. **AGENTS.md wording** for line 11 (e.g. "the GitLab view's remembered type and hidden toggle (no filter values)") and line 28.
