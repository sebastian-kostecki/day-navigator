# Own Tasks Implementation Plan

## Overview

Roadmap slice S-06 (FR-006, FR-007, FR-008, FR-010, PRD `[removed] S-05`). The user can add their own tasks in the Work or Private space in a "Moje zadania" view, edit the title and a short plain note, set an optional due date and priority, mark a task done and reopen it. The tasks are app-owned records in the SQLite store (F-01), the first real data slice on top of it. The old vault-based task screen (`TasksView` + `tasks.rs`) is removed in the same change; the generic vault layer stays dormant.

## Current State Analysis

- **Store is ready, empty.** `src-tauri/src/store/schema.rs:10` holds `MIGRATIONS = &[]`; versions must start at 1 and rise by 1 (`schema.rs:13`). `StoreHandle::with_store(today, |store| …)` (`store/mod.rs:233`) opens lazily; `Store::write` (`store/mod.rs:152`) is one IMMEDIATE transaction that commits only on `Ok`. `StoreError` (`store/error.rs:10-29`) is `#[serde(tag = "kind", content = "detail")]` and never carries row values or paths; `From<rusqlite::Error>` maps constraint failures to `Sql`.
- **No store command exists yet.** The only async helper, `blocking` (`src-tauri/src/lib.rs:76-87`), is hard-wired to `TaskError`. The store is managed as `StoreHandle` (`lib.rs:185-189`) and is not opened at startup on purpose; the health round opens it and holds its mutex during reopen/backup, so store commands must run inside `spawn_blocking`.
- **Space enum exists only in the vault layer.** `Space { Work, Private }` with `serde(rename_all = "lowercase")` and `Display` "work"/"private" lives in `src-tauri/src/vault/mod.rs:26-40`; used by the vault registry, `health/probe.rs:42,105-108` and `lib.rs:71`. No TS `Space` type exists.
- **Dates.** `chrono` 0.4 (`clock`, `serde`) is available; `rusqlite` 0.43 has no `chrono` feature, so dates go in as text. The old screen validated due dates as strict `YYYY-MM-DD` (`src-tauri/src/tasks.rs:371-387`).
- **Old vault task screen (to remove).** `src/tasks/{TasksView.tsx,api.ts}`, mounted at `src/App.tsx:2,9`; `src-tauri/src/tasks.rs` (756 lines); in `lib.rs`: `pub mod tasks` (:6), imports (:15-17), `TasksState` (:43), `TaskSettingsView` (:45-52), `task_cfg`/`work_vault`/`blocking` (:54-87), four commands (:89-145), `app.manage(TasksState…)` (:184), handler entries (:205-208); task CSS block in `src/App.css` (`/* Tasks view */`, ~:119-205, plus dark-mode rules near the end); `src-tauri/tests/tasks.rs`; `test_tasks_settings()` and the `tasks: None` field in `src-tauri/tests/common/mod.rs:54-68`.
- **Settings trap.** `SpaceSettings.tasks` / `TaskSettings` / `RawTasks` / `convert_tasks` in `src-tauri/src/settings.rs:23-69,144-186` (+ tests :260-333). `RawSpace` has `#[serde(deny_unknown_fields)]` (`settings.rs:51`): dropping the `tasks` field naively would make an existing `settings.json` with a `tasks` block fail as `SettingsInvalid`, turning the Settings source red and skipping the vault probes.
- **Navigation and counter are not designed yet.** S-01 (navigation + header) and S-08 (work-time counter) each have only a `change.md`; no plan, no "counter running" API. `App.tsx` stacks views with the comment "No navigation yet (S-01)".
- **Error rendering.** `src/components/ErrorState.tsx` (`{title, message, onRetry?}`) is the documented error surface; the old `TasksView` bypassed it.

## Desired End State

- `own_tasks` table in the store (migration v1). Each task has: space (work/private), title (1–200 chars, single line), note (0–2000 chars, plain text, line breaks kept), optional due date (`YYYY-MM-DD`), optional priority (high/medium/low), created/updated timestamps and an optional `done_at`.
- Rust commands `list_own_tasks`, `add_own_task`, `edit_own_task`, `set_own_task_done`, `own_task_default_space`, all async and store-backed through `spawn_blocking`.
- A "Moje zadania" view replaces the old "Zadania" view in `App.tsx`:
  - a Wszystkie / Praca / Prywatne filter (default Wszystkie) and a space badge per row;
  - open tasks ordered by due date (none last), then priority (high → low, none last), then creation time;
  - done tasks hidden behind "Pokaż zrobione";
  - a checkbox toggles done/reopen;
  - add and edit share one form; the add form's space is prefilled from `own_task_default_space` (Private until S-08 lands).
- Loading, error (via `ErrorState`, with the time of the last successful refresh) and empty states are all explicit.
- No code path for the vault task screen remains. A `settings.json` that still has a `spaces.work.tasks` block loads without error.
- Verification: `cargo test` (unit + integration + crash), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `npm run lint`, `npm run format:check`, `npm run build`, and a manual pass in `npm run tauri dev`.

### Key Discoveries:

- `Store::write` rolls back on `Err` or panic (`store/mod.rs:152`), so a validation error inside the closure never half-applies.
- `StoreHandle::with_store` needs `today: NaiveDate` (`store/mod.rs:233`); the store never reads the clock, so commands pass time in (pattern: `lib.rs:120,142`).
- `tauri::State` is not `'static`; a `spawn_blocking` closure needs `AppHandle` + `app.state::<StoreHandle>()` inside it.
- The crash harness `tests/common/crash.rs` (`run_writer_until`, `KillAt`) is the durability proof every data slice reuses (`tests/store_crash.rs:30,62` is the example).
- SQLite `length()` on TEXT counts characters, which matches Rust's `chars().count()`, so `CHECK` constraints can mirror the Rust limits.
- The fixture `tests/fixtures/vault/Templates/` holds only task templates, but `tests/vault_guard.rs:144-155` asserts the root listing includes `Templates`. The vault fixture therefore stays untouched.

## What We're NOT Doing

- No deleting tasks (FR-009 was removed); a mistaken task is edited or marked done.
- No "Do zrobienia" list, carry-over or ticking from it (S-07). This slice only makes `done_at` the single done state S-07 will reuse.
- No reading of the work-time counter: `own_task_default_space` passes `counter_running = false` until S-08 wires it.
- No navigation or fixed header (S-01); the new view is swapped in place of `TasksView` in the stacked `App.tsx`.
- No overdue highlighting, reminders, search, manual reordering, tags, Markdown rendering or rich text.
- No configurable priority list: the scale is fixed at three levels.
- No changes to the generic vault layer (`src-tauri/src/vault/`) or its fixtures, apart from moving `Space` behind a re-export and narrowing visibility of helpers the removal orphans. Roadmap Open Question #4 (keep or remove the vault layer) stays open.
- No import of tasks from the old vault `Tasks/` folder.

## Implementation Approach

The work runs bottom-up and follows the established patterns:
- a plain Rust domain module (no `tauri` import, time passed in, typed serde errors) with unit tests;
- one appended migration;
- integration and crash tests against a temp store;
- thin async commands;
- a feature folder (`api.ts` / `types.ts` / `use<Name>.ts` / `<Name>View.tsx`).

Ordering is a domain rule, so it lives in Rust and the UI only filters what it receives. The default-space rule is a pure Rust function. Its command is the single seam S-08 changes. The old screen is unmounted in Phase 2 and deleted in Phase 3, so every phase leaves a building, testable app.

## Critical Implementation Details

- **Migration numbering with S-08.** This slice appends migration **version 1**. S-08 (work-timer, in a parallel worktree) will also add a migration. Whichever merges second renumbers its own migration to the next version before merging. That is safe because nothing has shipped. Once a version is released, its migration is frozen (`schema.rs:3`).
- **No user content in errors or logs.** Validation errors name the rule and the limit, never the title, note or due value. `StoreError` already strips row values. Never `log::` a task field.
- **Orphaned `pub(crate)` helpers.** Removing `tasks.rs` and `convert_tasks` leaves some vault helpers without a caller: `check_plain`, `BOM`, `frontmatter_span`, `SpanHash::from_hex`, `replace_fields`, `create_note_in_existing_folder`. `pub` items in the lib crate don't warn, but `pub(crate)` ones do, and clippy runs with `-D warnings`. Narrow such items back to private when they are still used inside their module. Otherwise keep them with their existing tests (vault contract). Do not delete vault tests.

## Phase 1: Own-task domain and store records

### Overview

Add the own-task records and rules in Rust with full test coverage, no UI yet.

### Changes Required:

#### 1. Neutral Space enum

**File**: `src-tauri/src/space.rs` (new), `src-tauri/src/vault/mod.rs`, `src-tauri/src/lib.rs`

**Intent**: Move `Space { Work, Private }` (with its serde, `Display` and derives unchanged) out of the dormant vault module so own tasks don't depend on vault code. `vault` re-exports it, so every existing `crate::vault::Space` path keeps compiling.

**Contract**: `pub mod space;` in `lib.rs`; `vault/mod.rs` replaces the definition with `pub use crate::space::Space;`. Serialized form stays `"work"` / `"private"`.

#### 2. Own-task domain module

**File**: `src-tauri/src/own_tasks.rs` (new), registered as `pub mod own_tasks;` in `lib.rs`

**Intent**: Holds the own-task records, validation, ordering and default-space rule, plus the store operations that read and write them. It is plain Rust: no `tauri` import, and `now: NaiveDateTime` is passed in.

**Contract**:
- `Priority { High, Medium, Low }`: `serde(rename_all = "lowercase")`, stored as `"high"`/`"medium"`/`"low"`.
- `OwnTaskInput { space: Space, title: String, note: String, due: Option<String>, priority: Option<Priority> }`: `serde(rename_all = "camelCase")`. The same shape serves add and edit, and edit replaces every field.
- `OwnTask { id: i64, space, title, note, due: Option<String>, priority: Option<Priority>, created_at: String, updated_at: String, done_at: Option<String> }`: camelCase. Timestamps are local `YYYY-MM-DDTHH:MM:SS`.
- `OwnTaskError` (`thiserror` + `Serialize`, `#[serde(tag = "kind", content = "detail")]`), with variants:
  - `Store(StoreError)`
  - `TitleEmpty`
  - `TitleTooLong { max: usize }`
  - `TitleMultiline`
  - `NoteTooLong { max: usize }`
  - `InvalidDue`
  - `NotFound`

  It implements `From<StoreError>` (and `From<rusqlite::Error>` through `StoreError`). No variant carries a task field value.
- Validation rules (`TITLE_MAX = 200`, `NOTE_MAX = 2000`, both counted in `char`s):
  - The title is trimmed. Empty → `TitleEmpty`. Containing `\r` or `\n` → `TitleMultiline`. Over 200 → `TitleTooLong`.
  - The note's `\r\n` becomes `\n`. A whitespace-only note becomes `""`. Over 2000 after that → `NoteTooLong`.
  - `due` must parse as `%Y-%m-%d` and format back to exactly the same string (rejects `2026-2-1`, `2026-02-30`) → `InvalidDue`.
- `pub fn default_space(counter_running: bool) -> Space`: Work if running, else Private.
- `pub fn sort_for_display(tasks: &mut [OwnTask])`:
  - Open tasks come before done ones.
  - Open tasks sort by `due` ascending (none last), then priority (High, Medium, Low, none), then `created_at`, then `id`.
  - Done tasks sort by `done_at` descending, then `id`.
- Store operations, each a single `Store::write` / `Store::read` (validation runs inside the closure, so an error rolls back):
  - `list(store) -> Vec<OwnTask>` (sorted)
  - `add(store, input, now) -> OwnTask`
  - `edit(store, id, input, now) -> OwnTask` (`NotFound` if no row changed)
  - `set_done(store, id, done: bool, now) -> OwnTask`:
    - `done = true` sets `done_at = now` only if it is currently null (idempotent);
    - `done = false` clears it;
    - both update `updated_at`;
    - `NotFound` for an unknown id.

#### 3. Migration v1

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append the first domain migration creating the `own_tasks` table. Its `CHECK` constraints mirror the Rust rules as a second line of defence.

**Contract**: `Migration { version: 1, sql }`:

```sql
CREATE TABLE own_tasks (
  id         INTEGER PRIMARY KEY,
  space      TEXT NOT NULL CHECK (space IN ('work','private')),
  title      TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
  note       TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 2000),
  due        TEXT,
  priority   TEXT CHECK (priority IN ('high','medium','low')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  done_at    TEXT
);
```

S-07 will reference `own_tasks(id)`; `done_at` is the single "done" state (FR-008).

#### 4. Tests

**Files**: unit tests in `src-tauri/src/own_tasks.rs`; `src-tauri/tests/own_tasks.rs` (new); `src-tauri/tests/own_tasks_crash.rs` (new)

**Intent**: Prove the rules, the round trip through a real store, and durability under a crash. All data is invented.

**Contract**:
- Unit tests:
  - Validation boundaries: title of 200 vs 201 chars, multibyte chars counted as one, whitespace-only title, newline in title, note of 2000 vs 2001, CRLF note normalized, valid/invalid/non-canonical due.
  - `default_space` for both inputs.
  - `sort_for_display` cases: none-due last, same due ordered by priority, ties by creation, done after open, done newest first.
  - Error JSON shape (`{"kind":"TitleTooLong","detail":{"max":200}}`).
  - No error string contains the submitted title or note.
- Integration tests (`tests/common::test_store_dir`, migrations from `schema::MIGRATIONS`):
  - Add → list → edit → done → reopen round trip.
  - Data survives a close and reopen.
  - `edit` / `set_done` on an unknown id → `NotFound`.
  - A failed validation leaves the table unchanged.
  - Marking done twice keeps the first `done_at`.
  - Migration v1 applies on a fresh store, which reports `user_version = 1`.
- Crash test reusing `tests/common/crash.rs`:
  - A writer test adds numbered invented tasks and prints `committed <n>`.
  - The parent kills it at several points, reopens, and asserts `quick_check` is ok, every task `1..=n` is present with its exact title and fields, and no row is partial.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the new unit, integration and crash tests
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo fmt --check` passes

#### Manual Verification:

- Review: no own-task field value appears in any error variant, `Display` string or log call

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Commands and "Moje zadania" view

### Overview

Expose own tasks through async store commands and build the view that replaces the old "Zadania" screen in `App.tsx`.

### Changes Required:

#### 1. Store helper and commands

**File**: `src-tauri/src/lib.rs`

**Intent**: Add one generic helper that runs a store operation off the main thread with the current local time. Then add the five thin commands and register them. The old `blocking` helper stays until Phase 3 deletes it with the task commands.

**Contract**:
- The helper is `async fn on_store<T, E>(app: tauri::AppHandle, f: impl FnOnce(&mut Store, NaiveDateTime) -> Result<T, E> + Send + 'static) -> Result<T, E>`, where `T: Send + 'static` and `E: From<StoreError> + Send + 'static`.
  - It takes `now = Local::now().naive_local()` once before spawning.
  - Inside `spawn_blocking`, it calls `app.state::<StoreHandle>().with_store(now.date(), |s| f(s, now))`.
  - A join failure becomes a path-free `StoreError` (`Io`).
- Commands (`#[tauri::command] async`, `Result<_, OwnTaskError>`):
  - `list_own_tasks() -> Vec<OwnTask>`
  - `add_own_task(input: OwnTaskInput) -> OwnTask`
  - `edit_own_task(id: i64, input: OwnTaskInput) -> OwnTask`
  - `set_own_task_done(id: i64, done: bool) -> OwnTask`
  - `own_task_default_space() -> Space`, which returns `own_tasks::default_space(false)`. The S-08 seam: replace `false` with the counter's running state.

#### 2. Frontend feature folder

**Files**: `src/own-tasks/types.ts`, `src/own-tasks/api.ts`, `src/own-tasks/useOwnTasks.ts`, `src/own-tasks/OwnTaskForm.tsx`, `src/own-tasks/OwnTasksView.tsx` (all new)

**Intent**: Follow the feature-folder convention. `api.ts` is the only `invoke` importer; the hook owns loading, error and data; the view renders. Ordering comes from Rust; the UI only filters by space and done.

**Contract**:
- `types.ts` mirrors Rust:
  - `Space = "work" | "private"`, `Priority = "high" | "medium" | "low"`;
  - `OwnTask`, `OwnTaskInput`;
  - `OwnTaskError` as a union tagged on `kind`, with the `Store` variant's `detail` typed as a `StoreError` union (`DirUnavailable`, `Damaged`, `Missing`, `NewerSchema`, `MigrationFailed`, `Busy`, `Io`, `Sql`).
- `api.ts`:
  - one typed function per command;
  - `ownTaskErrorMessage(e)` returns a Polish message per kind, e.g. "Tytuł jest wymagany.", "Tytuł może mieć najwyżej 200 znaków.", "Notatka może mieć najwyżej 2000 znaków.", "Nieprawidłowa data terminu.", "Zadanie nie istnieje — odśwież listę.", and per store kind a message naming "Dane aplikacji".
- `useOwnTasks.ts`:
  - the list state is `loading | error | loaded`, plus `lastSuccessAt` and a stale-reload guard (pattern: `src/tasks/TasksView.tsx:31-46`);
  - it exposes `add`, `edit`, `setDone` and `reload`;
  - each mutation reloads the list on success;
  - a mutation error is returned to the caller, which shows it inline. It never replaces a loaded list with an error screen.
- `OwnTaskForm.tsx` (shared by add and edit):
  - title `<input>` with `maxLength=200`, note `<textarea>` with `maxLength=2000`;
  - space `<select>` Praca / Prywatne, due `<input type="date">` (empty → `null`), priority `<select>` "—" / Wysoki / Średni / Niski;
  - submit is disabled while pending;
  - on a validation error, the input is kept and the Polish message shown.

  The add form preloads the space from `ownTaskDefaultSpace()`. If that call fails, it shows the error inline and selects Prywatne.
- `OwnTasksView.tsx`:
  - heading "Moje zadania", a Wszystkie / Praca / Prywatne filter (local state, default Wszystkie), a "Dodaj zadanie" form;
  - rows show:
    - a done checkbox (toggle = `setDone`), the title, a space badge (Praca / Prywatne), the due date and the priority label;
    - the note in a collapsed preview that keeps its line breaks;
    - an "Edytuj" action opening the form inline;
  - a "Pokaż zrobione" toggle (default off).
  - States:
    - loading "Ładowanie…";
    - error via `ErrorState` with the message and "Ostatnie udane odświeżenie: <time>" (or "brak" before the first success) and a retry;
    - empty "Nie masz jeszcze żadnych zadań." vs a filtered empty "Brak otwartych zadań w tej przestrzeni.".

#### 3. Mount and styles

**Files**: `src/App.tsx`, `src/App.css`

**Intent**: Swap the old view for the new one without inventing navigation (S-01 owns that), and add the view's styles.

**Contract**: `App.tsx` renders `<OwnTasksView />` where `<TasksView />` was, above `<HealthPanel />`; the stacked layout and the "No navigation yet (S-01)" comment stay. `App.css` gets a `/* Own tasks view */` block with light and dark rules that reuses the shared `/* Controls */` styles.

### Success Criteria:

#### Automated Verification:

- `npm run build` passes (tsc strict + Vite)
- `npm run lint` and `npm run format:check` pass
- `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass in `src-tauri/`

#### Manual Verification:

- In `npm run tauri dev`, add a task with only a title: it appears under Prywatne (counter not wired yet). Add one in Praca with a due date and priority.
- Edit the title, note (multi-line), space, due date and priority; changes persist after quitting from the tray and restarting the app.
- Ordering: a task due earlier appears above one due later. Among tasks with the same due date, Wysoki comes before Niski. Tasks without a due date come last.
- Tick a task: it disappears from the list and shows under "Pokaż zrobione". Untick it there: it is open again in its old place.
- The filter Praca / Prywatne / Wszystkie narrows the list. An empty filter shows the filtered empty message, never a blank area.
- A 201-character title or a 2001-character note (pasted) is refused with the Polish message, and the typed text is kept.
- The "Zadania" vault screen is no longer shown. HealthPanel still works and "Dane aplikacji" stays green.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Remove the vault task screen

### Overview

Delete the now-unmounted vault task screen and its backend, keep old settings files loading, and update the docs that name it.

### Changes Required:

#### 1. Frontend removal

**Files**: `src/tasks/` (delete folder), `src/App.css`

**Intent**: Remove the old view and its CSS (the `/* Tasks view */` block and its dark-mode rules). Keep shared controls and anything the new view reuses.

**Contract**: No file imports from `src/tasks/`; `npm run build` passes.

#### 2. Backend removal

**Files**: `src-tauri/src/tasks.rs` (delete), `src-tauri/src/lib.rs`, `src-tauri/tests/tasks.rs` (delete), `src-tauri/tests/common/mod.rs`

**Intent**: Remove the task domain and its commands. Keep `VaultState` (still written by `runtime.rs:284`) and the generic vault layer.

**Contract**:
- In `lib.rs`, remove:
  - `pub mod tasks`, the task imports, `TasksState`, `TaskSettingsView`;
  - `task_cfg`, `work_vault` and the old `blocking`;
  - the four `*_work_task*` commands and their handler entries;
  - `app.manage(TasksState…)`.
- In `tests/common/mod.rs`, drop `test_tasks_settings()` and the `tasks` field.
- Narrow any `pub(crate)` vault helper this orphans (see Critical Implementation Details).
- The vault fixtures stay byte-identical.

#### 3. Settings compatibility

**File**: `src-tauri/src/settings.rs`

**Intent**: Drop the task configuration from the settings model while still accepting, and ignoring, a legacy `tasks` block, so an existing hand-written `settings.json` keeps loading.

**Contract**:
- Remove `SpaceSettings.tasks`, `TaskSettings`, `RawTasks`, `convert_tasks` and their tests.
- `RawSpace` keeps `deny_unknown_fields` and gains a field `#[serde(default, rename = "tasks")] _legacy_tasks: Option<serde::de::IgnoredAny>`.
- New unit tests:
  - a `spaces.work` with a full legacy `tasks` block loads;
  - a `tasks` value of any JSON shape loads;
  - an unrelated unknown key under a space is still rejected.

#### 4. Docs

**File**: `AGENTS.md`

**Intent**: Replace `src/tasks/` with `src/own-tasks/` as the feature-folder example. Historical documents (roadmap baseline, stack assessment, archive) stay as written.

**Contract**: `AGENTS.md` "Project Structure" line naming feature folders.

### Success Criteria:

#### Automated Verification:

- `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass in `src-tauri/`
- `npm run build`, `npm run lint` and `npm run format:check` pass
- No references remain: searching `src/` and `src-tauri/src/` for `TasksView`, `TaskSettings`, `list_work_tasks` and `mod tasks` returns nothing

#### Manual Verification:

- With a `settings.json` that still contains a `spaces.work.tasks` block, `npm run tauri dev` starts with the Settings source green, and the vault probes run as before
- The "Moje zadania" view works as in Phase 2 after the removal

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Validation boundaries: 200/201-char title, 2000/2001-char note, multibyte counting, trim, newline in title, CRLF note, canonical due date.
- `sort_for_display`: due, then priority, then creation; none-due last; done after open; done newest first.
- `default_space(true/false)`.
- Error serde shape, and errors carrying no title or note.
- Settings: a legacy `tasks` block is accepted and ignored; other unknown keys are still rejected.

### Integration Tests:

- Store round trip: add, list, edit, done, reopen. Data survives a reopen. `NotFound` for an unknown id. A rejected write leaves the table unchanged. Migration v1 on a fresh store.
- Crash durability via `tests/common/crash.rs`: committed tasks are never lost or partial.

### Manual Testing Steps:

1. Add a title-only task → it lands in Prywatne.
2. Add a Work task with a due date and priority. Edit all its fields. Restart the app → everything persists.
3. Check ordering by due date and priority. Tick and untick a task via "Pokaż zrobione".
4. Use the filter tabs and check the empty messages.
5. Paste an over-long title or note → refused with a Polish message, the input is kept.
6. Start with a legacy `tasks` block in `settings.json` → Settings is green.

## Performance Considerations

Single user, tens to hundreds of rows: list reads the whole table and sorts in Rust; no index needed. Each command opens no new connection (the handle is reused) and runs off the main thread.

## Migration Notes

- First domain migration (v1). After it ships, an older app build refuses the newer database (`NewerSchema`). That is the intended F-01 behaviour.
- If S-08 merges first with its own v1, renumber this migration to v2 before merging. The same applies in reverse.
- Users with a `tasks` block in `settings.json` need no action; it is ignored. Their vault `Tasks/` notes stay untouched and are not imported.

## References

- Roadmap slice: `context/foundation/roadmap.md` (S-06, :172-182)
- PRD: `context/foundation/prd.md` (FR-006–FR-010, :132-141; `[removed] S-05`, :166)
- Store contract: `context/archive/2026-11-23-app-data-store/plan.md`; `src-tauri/src/store/mod.rs:152,233`
- Old screen being replaced: `context/archive/2026-11-20-work-tasks-from-template/`; `src/tasks/TasksView.tsx`; `src-tauri/src/tasks.rs`
- Crash harness example: `src-tauri/tests/store_crash.rs:30,62`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Own-task domain and store records

#### Automated

- [ ] 1.1 `cargo test` passes in `src-tauri/`, including the new unit, integration and crash tests
- [ ] 1.2 `cargo clippy --all-targets -- -D warnings` passes
- [ ] 1.3 `cargo fmt --check` passes

#### Manual

- [ ] 1.4 Review: no own-task field value appears in any error variant, `Display` string or log call

### Phase 2: Commands and "Moje zadania" view

#### Automated

- [ ] 2.1 `npm run build` passes (tsc strict + Vite)
- [ ] 2.2 `npm run lint` and `npm run format:check` pass
- [ ] 2.3 `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass in `src-tauri/`

#### Manual

- [ ] 2.4 In `npm run tauri dev`, add a task with only a title: it appears under Prywatne (counter not wired yet). Add one in Praca with a due date and priority.
- [ ] 2.5 Edit the title, note (multi-line), space, due date and priority; changes persist after quitting from the tray and restarting the app.
- [ ] 2.6 Ordering: a task due earlier appears above one due later. Among tasks with the same due date, Wysoki comes before Niski. Tasks without a due date come last.
- [ ] 2.7 Tick a task: it disappears from the list and shows under "Pokaż zrobione". Untick it there: it is open again in its old place.
- [ ] 2.8 The filter Praca / Prywatne / Wszystkie narrows the list. An empty filter shows the filtered empty message, never a blank area.
- [ ] 2.9 A 201-character title or a 2001-character note (pasted) is refused with the Polish message, and the typed text is kept.
- [ ] 2.10 The "Zadania" vault screen is no longer shown. HealthPanel still works and "Dane aplikacji" stays green.

### Phase 3: Remove the vault task screen

#### Automated

- [ ] 3.1 `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass in `src-tauri/`
- [ ] 3.2 `npm run build`, `npm run lint` and `npm run format:check` pass
- [ ] 3.3 No references remain: searching `src/` and `src-tauri/src/` for `TasksView`, `TaskSettings`, `list_work_tasks` and `mod tasks` returns nothing

#### Manual

- [ ] 3.4 With a `settings.json` that still contains a `spaces.work.tasks` block, `npm run tauri dev` starts with the Settings source green, and the vault probes run as before
- [ ] 3.5 The "Moje zadania" view works as in Phase 2 after the removal
