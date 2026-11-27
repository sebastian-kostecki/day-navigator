# Remembered GitLab View State Implementation Plan

## Overview

After the app is quit ("Zakończ", reboot, autostart launch) and started again, "Praca (GitLab)" shows the same type choice ("Wszystko" / "Zgłoszenia" / "MR-y") and the same state of the "Pokaż ukryte" toggle (the "Ukryte" expansion, `showHidden`). The view filter (project, label and milestone values) keeps resetting: those values are work data and stay in memory only. Anchor: roadmap M-4, S-05, MS-34 (GitHub issue #34). No new FR in the PRD.

The two values go into one fixed row of a new app-data table (migration v14), read once when the provider mounts and written on every user change. A failed read or write never blocks the view: it starts with the defaults and Rust logs the failure by error kind only.

This plan was written non-interactively by an agent. Every point where `/10x-plan` would ask the user is settled below and marked "default chosen by agent"; the user decisions given with the task are marked "user decision".

## Current State Analysis

- The state lives in `useGitlabListViewSource` (`src/gitlab/useGitlabListView.ts:28-31`) as plain `useState`, commented "In memory only": `kind: KindFilter = "all" | "issue" | "mergeRequest"` (default `"all"`) and `showHidden` (default `false`). `setShowHidden` takes an updater (`:10`); the view calls `setShowHidden((s) => !s)` (`src/gitlab/GitlabView.tsx:520`).
- `GitlabListViewProvider` (`src/gitlab/GitlabListViewProvider.tsx:1-8`) wraps every view in `src/App.tsx:77-84` and mounts at app start; the start view is "todo" (`App.tsx:36`), so a load normally finishes before the GitLab tab is opened.
- The state already survives tab switches and closing the window (close hides it, `src-tauri/src/lib.rs:186-197`; only "Zakończ" exits, `:239-248`). Only a process restart resets it.
- The store is at v13 (`src-tauri/src/store/schema.rs:264-285`). `validate` (`:289-300`) requires contiguous versions, so the next migration is v14. Every record is a typed table with CHECKs; there is no key/value table.
- `src-tauri/tests/events.rs:184` (`MIGRATIONS.len() == 13`) and `:200` (`user_version == 13`) break when v14 is appended. No other test hard-codes the full migration count (searched `src-tauri/tests` and `src-tauri/src`).
- The closest precedent is the seen-until mark: table `gitlab_view_marks` (v9, `schema.rs:209-220`), domain fn `new_assignments::mark_view_seen` (`src-tauri/src/new_assignments/mod.rs:102-118`), tagged error `NewAssignmentError { Store, Background }` (`src-tauri/src/new_assignments/error.rs:10-46`), command `mark_gitlab_seen` (`src-tauri/src/gitlab_refresh.rs:407-431`) that runs through `crate::blocking` and logs `Busy` at `debug`, everything else at `warn`, by kind only. The UI side is fire-and-forget (`src/gitlab/useGitlabSeenMark.ts:5-8`).
- `crate::blocking` (`lib.rs:38-49`) is `pub(crate)`; `on_store` (`lib.rs:54-73`) is private to `lib.rs`. Both run `with_store` inside `spawn_blocking`.
- `settings.json` is hand-written and read-only for the app (`src-tauri/src/settings.rs:1`, `deny_unknown_fields`), so it cannot hold this state.
- The Rust `gitlab::ItemKind` serializes as `"issue"` / `"mergeRequest"` (`src-tauri/src/gitlab/mod.rs:38-43`); the TS `StoreError` mirror is `src/components/storeError.ts:1-13`.
- `node_modules` is missing in this worktree: run `npm install` once before the npm gates and revert any `package-lock.json` change.

## Desired End State

- Migration v14 creates `gitlab_view_state` with at most one row: `kind` (one of the three values) and `show_hidden` (0/1). Nothing else, no work data.
- A new Rust module `gitlab_view_state` offers `load` (defaults when there is no row) and `save` (one upsert in one `write`), plus two async commands `gitlab_view_state` and `set_gitlab_view_state` with a tagged, value-free error type and its TS mirror.
- `useGitlabListViewSource` loads once on mount, applies each loaded field only if the user has not changed that field meanwhile, and saves both fields after every user change (latest value, one save in flight). The `GitlabListView` contract and `GitlabView.tsx` stay unchanged.
- AGENTS.md lists the new record in the app-data list and says that type and hidden expansion are remembered while the view filter stays in memory.
- Verification: the Rust tests below pass; in a built app, choices survive "Zakończ" + restart while filter values reset (Manual rows, checked later by the user in an integrated build).

### Key Discoveries:

- `src/gitlab/useGitlabListView.ts:6-14` — the `GitlabListView` contract every consumer depends on; keep it byte-compatible so `GitlabView.tsx` (edited by S-04) is not touched.
- `src/gitlab/GitlabView.tsx:33` imports `type KindFilter` from `./useGitlabListView`; the hook file must keep exporting that name.
- `src/gitlab/GitlabView.tsx:515-523` — the toggle is `disabled={hiddenCount === 0 && !showHidden}`, so a remembered `true` with no hidden issues still lets the user turn it off.
- `src-tauri/tests/new_assignments.rs:203-237` — upgrade-test template: find the migration by SQL content, open `MIGRATIONS[..pos]`, write old rows, reopen with the full list.
- `src-tauri/tests/half_day_off_crash.rs` and `src-tauri/tests/common/crash.rs:38-140` — crash-test template and harness (`run_writer_until`, `KillAt`, `MAX_WRITER_COMMITS`, `WRITER_DIR_ENV`).
- `src/main.tsx:8` — the app runs in `React.StrictMode`, so the mount effect runs twice in dev.

## What We're NOT Doing

- No persistence of view-filter values (project, label, milestone selection) — work data (user decision).
- No account-keyed row: one global row, even with several GitLab accounts over time (default chosen by agent; avoids reading the account key from `GitlabRefreshState`, which F-01 refactors).
- No generic key/value or "UI preferences" table (default chosen by agent).
- No change to `settings.json`, the tray, the event log or the health round.
- No UI notice for a failed load or save (default chosen by agent, see Implementation Approach).
- No change to `src/gitlab/GitlabView.tsx`, `src-tauri/src/gitlab_refresh.rs` or `src-tauri/src/github_refresh.rs`.
- No persistence of other view state (other tabs, scroll position, the active tab).
- No new FR in `context/foundation/prd.md`; no edit of `context/foundation/roadmap.md` in this change (the orchestrator owns roadmap status).
- No `npm run tauri dev` in the automated verification (port 1420 is shared with parallel slices).

## Implementation Approach

Decisions (each is used by the phases below):

1. **Storage** — app data store via `store::StoreHandle`, migration v14 (user decision).
2. **What is stored** — `kind` with all three values `"all" | "issue" | "mergeRequest"` and `show_hidden`; nothing else (user decision).
3. **Table shape** — one fixed row: `id INTEGER PRIMARY KEY CHECK (id = 1)`, `kind TEXT NOT NULL CHECK (kind IN ('all','issue','mergeRequest'))`, `show_hidden INTEGER NOT NULL CHECK (show_hidden IN (0, 1))` (default chosen by agent).
4. **Defaults** — live in Rust: `load` returns `{ kind: all, showHidden: false }` when there is no row (first start, right after the upgrade). The TS hook uses the same values only as its pre-load initial state (default chosen by agent).
5. **Write policy** — write on every user change, one command sending both fields; no debounce, so a hard kill loses nothing already shown (default chosen by agent).
6. **Save ordering** — the hook sends saves one at a time, each carrying the latest in-memory value, and only after the mount load has settled; a later click never loses to an earlier in-flight save (default chosen by agent).
7. **Load timing** — the provider loads once at mount; each loaded field is applied only if the user has not changed that field meanwhile; StrictMode-safe via a cancelled flag in the effect cleanup; no gating of the toolbar while loading (default chosen by agent).
8. **Failure behaviour** — a failed load or save never blocks the view and shows no UI hint: the view keeps the defaults (or the user's in-memory choice), the TS call swallows the error like `useGitlabSeenMark`, and Rust logs it by error kind only (`Busy` at `debug`, everything else at `warn`). A damaged or missing store is already reported by the tray as "Dane aplikacji". This is a convenience record, not a source, so the "never render an empty list in place of a failed source" rule does not apply (default chosen by agent).
9. **Unknown stored kind** — cannot happen (CHECK), but if `load` reads a value it does not know, it returns a value-free `Store(Sql { reason: "unknown view kind" })` error (no reset, no value in the message); the UI then keeps the defaults (default chosen by agent).
10. **Command placement** — a new module `src-tauri/src/gitlab_view_state/` (not `gitlab_refresh.rs`, not `new_assignments`), run through `crate::blocking` + `StoreHandle::with_store`, like `mark_gitlab_seen`. `on_store` would need a visibility change in `lib.rs`; `blocking` keeps the `lib.rs` diff to one `mod` line and two handler lines (default chosen by agent).
11. **Error type** — own `GitlabViewStateError { Store(StoreError), Background }`, `thiserror` + `Serialize`, `#[serde(tag = "kind", content = "detail")]`, with a TS mirror (default chosen by agent; matches `NewAssignmentError`).
12. **Tests** — Rust integration tests (defaults, round-trip, single row, DB-level refusal, v13→v14 upgrade found by SQL content), a serde-shape unit test, a crash test on the harness, and the `events.rs` assert fix. Invented data only (default chosen by agent; the crash test follows AGENTS.md "data slices reuse the crash harness" even though the comparable seen mark has none).
13. **AGENTS.md** — both lines updated in this change (user decision); the data-list line lands with the migration (Phase 1), the Frontend Conventions sentence with the hook change (Phase 2) (default chosen by agent).
14. **Phases** — two phases; Rust command types and their TS mirror land in the same phase and commit (AGENTS.md rule) (default chosen by agent).

## Critical Implementation Details

**State sequencing (hook).** Persist from an effect on the combined view state, guarded by "has the user changed a field yet": with no user change the effect does nothing, so the mount render (defaults) and a pure load-apply never write, and the StrictMode double mount is a no-op. Mark a field as user-changed in the setter, outside the `setState` updater (updaters run twice under StrictMode and must stay pure). Chain saves on one promise kept in a ref that starts as the load promise, and read the value to send from a "latest" ref at send time, not at click time; otherwise a click before the load finishes would write the default for the field the load then restores.

**Migration numbering.** Only this slice adds a migration in this wave. If another migration lands on `develop` first anyway, renumber to the next free version and update the one assert in the upgrade test that names `14` (the rest finds the migration by SQL content and uses `MIGRATIONS.len()`).

## Phase 1: Store record, Rust commands and TS mirror

### Overview

Add migration v14, the `gitlab_view_state` Rust module (types, `load`, `save`, error, commands), register the commands, add the TS mirror and typed `api.ts` functions (not yet called), fix the `events.rs` asserts, add the tests, and update the AGENTS.md app-data list. After this phase the commands work and are tested; the UI behaves as before.

### Changes Required:

#### 1. Migration v14

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append the table that holds the remembered view state, with a comment in the existing style (slice S-05 `gitlab-view-memory`; one row; no filter values, no work data; `kind` values mirror `gitlab_view_state::ViewKind`).

**Contract**: `Migration { version: 14, sql: "CREATE TABLE gitlab_view_state (id INTEGER PRIMARY KEY CHECK (id = 1), kind TEXT NOT NULL CHECK (kind IN ('all','issue','mergeRequest')), show_hidden INTEGER NOT NULL CHECK (show_hidden IN (0, 1)));" }` appended after v13; never edit v1-v13.

#### 2. Rust module `gitlab_view_state`

**Files**: `src-tauri/src/gitlab_view_state/mod.rs`, `src-tauri/src/gitlab_view_state/error.rs`, `src-tauri/src/gitlab_view_state/commands.rs`

**Intent**: Own the record: typed values, defaults, one read and one upsert, a value-free error type, and two Tauri commands. `commands.rs` is the only file in the module that imports `tauri` (like `hidden_issues`).

**Contract**:
- `mod.rs`:
  - `pub enum ViewKind { All, Issue, MergeRequest }` — `Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default` (`All` is the default), `#[serde(rename_all = "camelCase")]`; DB text via a small `as_db()` / parse pair (`"all"`, `"issue"`, `"mergeRequest"`).
  - `pub struct GitlabViewState { pub kind: ViewKind, pub show_hidden: bool }` — same derives, `Default` = `{ All, false }`, `#[serde(rename_all = "camelCase")]` (JSON `{ "kind": "mergeRequest", "showHidden": true }`).
  - `pub fn load(store: &Store) -> Result<GitlabViewState, GitlabViewStateError>` — reads row `id = 1` with `Store::read`; no row gives `GitlabViewState::default()`; an unknown `kind` text or a `show_hidden` other than 0/1 gives `Store(StoreError::Sql { reason: "unknown view kind" })` (fixed text, no value).
  - `pub fn save(store: &mut Store, view: GitlabViewState) -> Result<(), GitlabViewStateError>` — one `Store::write` with `INSERT INTO gitlab_view_state (id, kind, show_hidden) VALUES (1, ?1, ?2) ON CONFLICT (id) DO UPDATE SET kind = excluded.kind, show_hidden = excluded.show_hidden`.
  - `#[cfg(test)]` unit test: serde shape of `GitlabViewState` for each kind (`"all"`, `"issue"`, `"mergeRequest"`, `showHidden`), deserialization of the same JSON, refusal of an unknown kind string by serde, and the serialized shapes of `GitlabViewStateError` (`{"kind":"Store","detail":{"kind":"Busy"}}`, `{"kind":"Background"}`).
- `error.rs`: `GitlabViewStateError { Store(StoreError), Background }`, `thiserror::Error` + `Serialize`, `#[serde(tag = "kind", content = "detail")]`; `From<StoreError>`, `From<rusqlite::Error>`, `From<crate::BackgroundFailed>`; `kind() -> &'static str`. Same shape and invariant comment as `new_assignments/error.rs` (no variant carries row values).
- `commands.rs`:
  - `#[tauri::command] pub async fn gitlab_view_state(app: AppHandle) -> Result<GitlabViewState, GitlabViewStateError>`
  - `#[tauri::command] pub async fn set_gitlab_view_state(app: AppHandle, view: GitlabViewState) -> Result<(), GitlabViewStateError>`
  - Both run `crate::blocking` with `app.state::<StoreHandle>().with_store(Local::now().date_naive(), …)`; on `Err`, log `"gitlab_view_state: {load|save} failed: {kind}"` — `Busy` at `debug`, everything else at `warn`; never log the values. Never call `glab`.

#### 3. Registration

**File**: `src-tauri/src/lib.rs`

**Intent**: Make the module and its commands available; nothing else in `lib.rs` changes.

**Contract**: one line `pub mod gitlab_view_state;` (alphabetical, after `pub mod gitlab_refresh;`) and two lines in `generate_handler!` placed right after `hidden_issues::commands::dismiss_gitlab_change,` (away from the `gitlab_refresh::*` lines F-01 may touch): `gitlab_view_state::commands::gitlab_view_state,` and `gitlab_view_state::commands::set_gitlab_view_state,`. App commands need no capability entry (`src-tauri/capabilities/default.json` description).

#### 4. TS mirror and typed API

**Files**: `src/gitlab/types.ts`, `src/gitlab/api.ts`, `src/gitlab/useGitlabListView.ts`

**Intent**: Mirror the Rust types in the same commit and add one typed function per command. `KindFilter` becomes the mirror of `ViewKind` and moves to `types.ts`; the hook file re-exports it so `GitlabView.tsx:33` keeps compiling unchanged.

**Contract**:
- `types.ts`: `/** Mirrors gitlab_view_state::ViewKind. */ export type KindFilter = "all" | "issue" | "mergeRequest";`, `/** Mirrors gitlab_view_state::GitlabViewState. */ export type GitlabViewState = { kind: KindFilter; showHidden: boolean };`, `/** Mirrors gitlab_view_state::GitlabViewStateError. */ export type GitlabViewStateError = { kind: "Store"; detail: StoreError } | { kind: "Background" };`.
- `api.ts`: `getGitlabViewState(): Promise<GitlabViewState>` → `invoke("gitlab_view_state")`; `setGitlabViewState(view: GitlabViewState): Promise<void>` → `invoke("set_gitlab_view_state", { view })`. Doc comments say a failure rejects with `GitlabViewStateError` and Rust has already logged it.
- `useGitlabListView.ts`: replace the local `KindFilter` declaration with `import type { KindFilter } from "./types"` plus `export type { KindFilter }`; no behaviour change in this phase.

#### 5. Fix hard-coded schema version asserts

**File**: `src-tauri/tests/events.rs`

**Intent**: Stop the v11 upgrade test from pinning the total migration count, so v14 (and later ones) do not break it.

**Contract**: delete the `:184` assert `assert_eq!(schema::MIGRATIONS.len(), 13);` (the content-based asserts at `:182-183` already pin v12 and v13); change `:200` to `assert_eq!(scalar(&store, "PRAGMA user_version"), schema::MIGRATIONS.len() as i64);` (default chosen by agent).

#### 6. Integration tests

**File**: `src-tauri/tests/gitlab_view_state.rs` (new; uses `common::test_store_dir`)

**Intent**: Prove the record's behaviour against a real store with invented values.

**Contract** (test names are suggestions):
- `empty_store_loads_the_defaults` — fresh store: `load` gives `{ All, false }` and the table has 0 rows.
- `save_then_load_round_trips_every_value` — all 3 kinds × both `show_hidden` values; after reopening the store the last saved value loads.
- `save_keeps_exactly_one_row` — several saves; `SELECT count(*) FROM gitlab_view_state` is 1.
- `the_table_refuses_invalid_rows` — direct SQL inserts of `kind = 'epic'`, `show_hidden = 2` and `id = 2` each fail with a constraint error; the row count stays unchanged.
- `v13_database_upgrades_to_the_view_state_table` — find the migration by `"CREATE TABLE gitlab_view_state"`, assert its version is 14, open `MIGRATIONS[..pos]` (user_version 13), write an invented v13-era row (e.g. one `days_off` row), reopen with the full list: `user_version == MIGRATIONS.len()`, the old row is kept, `load` gives the defaults.

#### 7. Crash test

**File**: `src-tauri/tests/gitlab_view_state_crash.rs` (new; reuses `src-tauri/tests/common/crash.rs`)

**Intent**: Prove that killing a process mid-save leaves a consistent store with no lost commit and no torn row.

**Contract**: an `#[ignore]` `writer_process` opens the store from `WRITER_DIR_ENV` and, for `n` in `1..=MAX_WRITER_COMMITS`, calls `save(state_of(n))` then prints `committed n`; `state_of(n)` cycles through the 6 combinations so consecutive states differ. The parent runs ~10 rounds with mixed `KillAt::AfterCommits` / `AfterDelay` on the same dir, then reopens and asserts `quick_check == "ok"`, exactly one row, and: if `last >= 1`, the loaded value is `state_of(last)` or `state_of(last + 1)`; if `last == 0`, it is the previous round's value (or the defaults in round 0) or `state_of(1)`. Each writer run starts counting at 1.

#### 8. AGENTS.md app-data list

**File**: `AGENTS.md` (Hard Rules, the "Outside folders configured in settings…" bullet)

**Intent**: List the new record next to the seen-until mark.

**Contract**: after "the GitLab view's seen-until mark (opaque account key, one instant)," insert "the GitLab view's remembered type choice and hidden-section expansion (one row: `all`/`issue`/`mergeRequest` and on/off; no filter values or other work data),". Nothing else in the bullet changes.

### Success Criteria:

#### Automated Verification:

- `cargo fmt --check` passes in `src-tauri/`
- `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/`
- `cargo test` passes in `src-tauri/`, including `tests/gitlab_view_state.rs`, `tests/gitlab_view_state_crash.rs`, the serde unit test in `gitlab_view_state` and the updated `tests/events.rs`
- `npm run lint` passes (after a one-time `npm install`; any `package-lock.json` change reverted)
- `npm run format:check` passes
- `npm run build` passes with zero errors
- `git diff --stat 2b6ac08 -- src/gitlab/GitlabView.tsx src-tauri/src/gitlab_refresh.rs src-tauri/src/github_refresh.rs` prints nothing
- `git diff 2b6ac08 -- src-tauri/src/lib.rs` shows only the `pub mod gitlab_view_state;` line and the two handler lines

**Implementation Note**: After completing this phase and all automated verification passes, commit (gates first, Conventional Commit, e.g. `feat(gitlab-view-memory): store the GitLab view state (v14)`) and continue with Phase 2. This phase has no manual checks of its own; the user checks the whole slice later in an integrated build.

---

## Phase 2: Load and save in the list-state hook

### Overview

Wire `useGitlabListViewSource` to load the remembered values once on mount and save both fields after every user change, keep the `GitlabListView` contract unchanged, and update the provider comment and the AGENTS.md Frontend Conventions sentence.

### Changes Required:

#### 1. List-state hook

**File**: `src/gitlab/useGitlabListView.ts`

**Intent**: Make `kind` and `showHidden` survive a restart without changing what consumers see; the view filter (`selection`, `toggle`, `clear`) stays exactly as it is, in memory only.

**Contract**:
- Hold `kind` and `showHidden` as one `GitlabViewState` value (initial `{ kind: "all", showHidden: false }`); `GitlabListView` (`:6-14`) is unchanged, including the updater signature of `setShowHidden`.
- Mount effect: call `getGitlabViewState()` once; on success apply each field only if the user has not changed that field yet; on failure keep the current values (no UI state, `.catch(() => {})`). A `cancelled` flag in the cleanup stops a StrictMode first run from applying.
- `setKind` / `setShowHidden` mark their field as user-changed (outside the updater) and update the value.
- Persist effect on the value: does nothing until the user has changed a field; then queues `setGitlabViewState(latest)` on a promise chain that starts with the load, so at most one save is in flight, saves run after the load settled, and each sends the latest value at send time; every failure is swallowed (Rust logs it).
- Update the doc comment of `useGitlabListViewSource`: type and hidden are remembered in the app data store, the view filter is in memory only.

#### 2. Provider comment

**File**: `src/gitlab/GitlabListViewProvider.tsx`

**Intent**: Keep the comment true.

**Contract**: the doc comment (`:4`) says the provider keeps kind, hidden and view filter across tab switches, and that kind and hidden are also remembered across restarts. No code change.

#### 3. AGENTS.md Frontend Conventions

**File**: `AGENTS.md` (Frontend Conventions, "Navigation" bullet)

**Intent**: Replace "in memory only" for the GitLab list state with the new split.

**Contract**: the sentence becomes: "The list state of "Praca (GitLab)" (type, hidden, view filter) is one `GitlabListViewProvider` (`src/gitlab/`) read through `useGitlabListView()`; the type and the hidden expansion are remembered in the app data store (`gitlab_view_state`), the view filter stays in memory only (work data)."

### Success Criteria:

#### Automated Verification:

- `npm run lint` passes
- `npm run format:check` passes
- `npm run build` passes with zero errors
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass in `src-tauri/`
- `git diff --stat 2b6ac08 -- src/gitlab/GitlabView.tsx` prints nothing
- `grep -c "the type and the hidden expansion are remembered in the app data store" AGENTS.md` prints `1`

#### Manual Verification:

- In a built app (`npm run tauri build -- --debug --no-bundle`, database backed up first), choose "MR-y" and turn on "Pokaż ukryte", quit via the tray "Zakończ", start again: "Praca (GitLab)" shows "MR-y" with the hidden rows
- With project/label/milestone filter values selected, quit via "Zakończ" and start again: the filter values are empty, while type and "Pokaż ukryte" are remembered
- Switch back to "Wszystko" and turn "Pokaż ukryte" off, quit and start again: the defaults are shown (changing back is saved too)
- With "Pokaż ukryte" remembered on and no hidden issues, the button stays enabled and turns it off
- First start on a copy of the 0.11.0 (v13) database: the app opens, the GitLab view starts with "Wszystko" and hidden off, the tray "Dane aplikacji" stays healthy
- On a backup copy with a refused store (e.g. `day-navigator.db` replaced by a zero-byte file): the GitLab view still opens with the defaults, the toggles work in memory, the app log shows a `gitlab_view_state` warning with the error kind only
- The app log contains no type or filter values from the view-state commands, and the DevTools console of the built app shows no CSP violation or "IPC custom protocol failed"

**Implementation Note**: After completing this phase and all automated verification passes, commit (gates first, e.g. `feat(gitlab-view-memory): remember the GitLab type and hidden toggle`). The Manual rows stay unchecked; the user checks them later in an integrated build.

---

## Testing Strategy

### Unit Tests:

- Serde shape of `GitlabViewState` and `ViewKind` (camelCase, three kind strings, unknown kind refused) and of `GitlabViewStateError` (tagged `kind`/`detail`) — the TS mirror depends on it.

### Integration Tests:

- `tests/gitlab_view_state.rs`: defaults on an empty store, round-trip of all six combinations across a reopen, exactly one row after many saves, DB-level refusal of an invalid `kind`, `show_hidden` and `id`, v13→v14 upgrade found by SQL content keeping old rows.
- `tests/gitlab_view_state_crash.rs`: kill-at-any-point consistency on the shared harness.
- `tests/events.rs`: still passes with v14 appended.

### Manual Testing Steps:

1. Build `npm run tauri build -- --debug --no-bundle` after backing up the database.
2. Run the Manual Verification rows of Phase 2 in order.
3. Restore the backed-up database if a refused-store check was done on the live data folder (prefer a copy).

## Performance Considerations

One single-row read at start and one single-row upsert per click (a few per session) on the blocking pool; negligible next to the health and refresh rounds.

## Migration Notes

- v14 only creates a table; existing rows are untouched. A v13 database upgrades on the next open; the first load after the upgrade returns the defaults.
- Rollback to 0.11.0 after v14 ran is refused as `NewerSchema` (existing store rule); restore a pre-upgrade backup as described in AGENTS.md ("Restoring").
- Back up the database before running a build with this migration on real data (user's standing practice).

## Merge Notes

Parallel slices in this wave: F-01 `source-refresh-skeleton` and S-04 `gitlab-offline-last-list`.

- `src-tauri/src/store/schema.rs` — only this slice adds a migration (v14). If another lands first, renumber (see Critical Implementation Details).
- `src-tauri/src/lib.rs` — this slice adds one `pub mod gitlab_view_state;` line after `pub mod gitlab_refresh;` and two handler lines after `hidden_issues::commands::dismiss_gitlab_change,`. F-01 may edit `setup` (`:167-179`) and the `gitlab_refresh::*` / `github_refresh::*` handler lines; on conflict keep both sides.
- `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/github_refresh.rs` — not touched (F-01 rewrites them).
- `src/gitlab/GitlabView.tsx` — not touched (S-04 edits the `!offline` list condition). `KindFilter` keeps its import path via the re-export.
- `src/gitlab/useGitlabListView.ts`, `GitlabListViewProvider.tsx`, `api.ts`, `types.ts` — only this slice in this wave.
- `src-tauri/tests/events.rs` — two asserts at `:184`, `:200`.
- `AGENTS.md` — two sentences (app-data list in Hard Rules, Navigation bullet in Frontend Conventions). F-01 may edit other lines; merge by line.

## References

- Related research: `context/changes/gitlab-view-memory/research.md`
- Roadmap item: `context/foundation/roadmap.md` (M-4, S-05, MS-34)
- Seen-mark precedent: `src-tauri/src/new_assignments/mod.rs:102-118`, `src-tauri/src/new_assignments/error.rs:10-46`, `src-tauri/src/gitlab_refresh.rs:407-431`, `src/gitlab/useGitlabSeenMark.ts:5-8`
- Upgrade-test template: `src-tauri/tests/new_assignments.rs:203-237`
- Crash-test template: `src-tauri/tests/half_day_off_crash.rs`, harness `src-tauri/tests/common/crash.rs`
- Provider origin: `context/archive/2026-11-25-gitlab-issue-labels/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Store record, Rust commands and TS mirror

#### Automated

- [ ] 1.1 `cargo fmt --check` passes in `src-tauri/`
- [ ] 1.2 `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/`
- [ ] 1.3 `cargo test` passes in `src-tauri/`, including `tests/gitlab_view_state.rs`, `tests/gitlab_view_state_crash.rs`, the serde unit test in `gitlab_view_state` and the updated `tests/events.rs`
- [ ] 1.4 `npm run lint` passes (after a one-time `npm install`; any `package-lock.json` change reverted)
- [ ] 1.5 `npm run format:check` passes
- [ ] 1.6 `npm run build` passes with zero errors
- [ ] 1.7 `git diff --stat 2b6ac08 -- src/gitlab/GitlabView.tsx src-tauri/src/gitlab_refresh.rs src-tauri/src/github_refresh.rs` prints nothing
- [ ] 1.8 `git diff 2b6ac08 -- src-tauri/src/lib.rs` shows only the `pub mod gitlab_view_state;` line and the two handler lines

### Phase 2: Load and save in the list-state hook

#### Automated

- [ ] 2.1 `npm run lint` passes
- [ ] 2.2 `npm run format:check` passes
- [ ] 2.3 `npm run build` passes with zero errors
- [ ] 2.4 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass in `src-tauri/`
- [ ] 2.5 `git diff --stat 2b6ac08 -- src/gitlab/GitlabView.tsx` prints nothing
- [ ] 2.6 `grep -c "the type and the hidden expansion are remembered in the app data store" AGENTS.md` prints `1`

#### Manual

- [ ] 2.7 In a built app (`npm run tauri build -- --debug --no-bundle`, database backed up first), choose "MR-y" and turn on "Pokaż ukryte", quit via the tray "Zakończ", start again: "Praca (GitLab)" shows "MR-y" with the hidden rows
- [ ] 2.8 With project/label/milestone filter values selected, quit via "Zakończ" and start again: the filter values are empty, while type and "Pokaż ukryte" are remembered
- [ ] 2.9 Switch back to "Wszystko" and turn "Pokaż ukryte" off, quit and start again: the defaults are shown (changing back is saved too)
- [ ] 2.10 With "Pokaż ukryte" remembered on and no hidden issues, the button stays enabled and turns it off
- [ ] 2.11 First start on a copy of the 0.11.0 (v13) database: the app opens, the GitLab view starts with "Wszystko" and hidden off, the tray "Dane aplikacji" stays healthy
- [ ] 2.12 On a backup copy with a refused store (e.g. `day-navigator.db` replaced by a zero-byte file): the GitLab view still opens with the defaults, the toggles work in memory, the app log shows a `gitlab_view_state` warning with the error kind only
- [ ] 2.13 The app log contains no type or filter values from the view-state commands, and the DevTools console of the built app shows no CSP violation or "IPC custom protocol failed"
