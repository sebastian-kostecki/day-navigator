# "To do" List Implementation Plan

## Overview

Roadmap slice S-07 (FR-011, FR-012, US-01 second half) adds the "Do zrobienia" view. The user picks today's items from their open own tasks and their assigned GitLab issues and MRs, up to a limit from `settings.json` (default 5). They can tick items off, untick them and remove them from the list. Unticked items carry over to the next day. A GitLab item that drops out of a successful assigned-and-open refresh ticks itself. Ticking an own task is the same state as marking it done in "Moje zadania". GitHub items join once S-05 is unblocked; nothing here waits for it.

## Current State Analysis

- **Store.** SQLite via `store::StoreHandle`, with migrations v1 (`work_entries`) and v2 (`own_tasks`) in `src-tauri/src/store/schema.rs:12,28`. Multi-record changes go through one `Store::write` transaction. Commands are `async` and reach the store inside `spawn_blocking` (`on_store`, `src-tauri/src/lib.rs:63`).
- **Own tasks.** `src-tauri/src/own_tasks.rs`: `done_at` is the single done state, reserved for S-07 (migration v2 comment; S-06 plan). `set_done` (`own_tasks.rs:303`) opens its own write and is idempotent (`coalesce(done_at, ?)`) and reversible.
- **GitLab list.** The list is held in memory only (`GitlabRefreshState` snapshot, `src-tauri/src/gitlab_refresh.rs`). The fetch asks for `state=opened` assigned items only (`src-tauri/src/cli/mod.rs:128-134`). Nothing reports "closed": a closed, merged or unassigned item simply drops out of the next successful list. `refresh_round` (`gitlab_refresh.rs:167`) computes the next snapshot (`:190`) and then publishes `gitlab-changed`. Items are keyed by `kind` + `reference` (`references.full`) (`src-tauri/src/gitlab/mod.rs:26`).
- **Settings.** `settings.json` is hand-written and versioned. Each top-level block is read on its own, so a typo in one block never breaks another (`load_gitlab_settings`, `src-tauri/src/settings.rs:97`). There is no settings UI and no "To do" setting yet.
- **UI.** `src/App.tsx:11,19` holds a 4-entry view list in one `useState`, with GitLab as the default. Feature folders follow the `View` / `api.ts` / `types.ts` / `use*.ts` layout. `ErrorState` renders failures with the time of the last success. `storeErrorMessage` and the full `StoreError` mirror are private to `src/own-tasks/`.
- **Tests.** Integration tests use `tests/common::test_store_dir`. Durability tests reuse the kill harness in `tests/common/crash.rs`, as `tests/own_tasks_crash.rs` does.

## Desired End State

- "Do zrobienia" is the first nav tab and opens at app start.
- It shows today's list: open items in pick order, each with a source badge (GitLab MR/issue reference, or Praca/Prywatne for own tasks). Items picked on an earlier day carry an "od DD.MM" badge. Below them, a "Zrobione dziś" part lists items ticked today, struck through and still untickable.
- An "Dodaj z list" picker offers the GitLab items from the current snapshot and open own tasks that are not on the list. Adding is refused once the open count reaches the limit, and the view says "otwarte N/limit".
- Ticking an own-task entry sets its `done_at`, and "Moje zadania" shows it done. Marking it done or reopening it in "Moje zadania" shows the same state in "Do zrobienia".
- After a successful GitLab refresh, open GitLab entries missing from the assigned-and-open list become ticked with a "zamknięte w GitLabie" marker. Entries still present get their stored title and link refreshed.
- No VPN, a GitLab error and "not configured" never tick anything. The view shows the stored titles plus a calm note that their GitLab state is unverified.
- The next local day, ticked entries are gone and open ones remain. A task reopened later does not come back on its own.
- Everything survives restart and crash. A store or settings failure is shown via `ErrorState`, never as an empty list.

Verify with the Phase 1–3 automated suites plus the manual pass in Testing Strategy.

### Key Discoveries:

- `done_at` already has the exact semantics FR-008 needs (`own_tasks.rs:303-325`). S-07 must not add a second "done" column for own tasks.
- The fetch filter `state=opened` (`cli/mod.rs:128`) is what makes "missing from a successful list" mean "closed, merged or unassigned". The future S-02 visibility filter must not narrow the set used for reconciliation (roadmap S-02 risk note).
- `refresh_round` already runs on its own thread with an `AppHandle`, so it can call `StoreHandle::with_store` directly. That is the AGENTS.md "never on the main thread" rule, satisfied by construction.
- The settings block-per-loader pattern (`settings.rs:97-135`) lets `todo.limit` fail without affecting vaults or GitLab.

## What We're NOT Doing

- GitHub items (S-05 is blocked). The source model leaves room for them, but no code reads GitHub.
- Writing to GitLab: ticking never closes anything in the source (PRD Constraints).
- Auto-unticking a GitLab entry that reappears in the list, and per-item GitLab state queries (no new `CliCall`).
- Manual reordering, priorities or notes on To-do entries. The order is pick order.
- A settings UI. The limit is edited in `settings.json`.
- Moving or trimming entries when the limit is lowered. The limit gates adding only.
- Picking done own tasks, and history or reporting of past days' lists.
- A frontend test runner (AGENTS.md: the UI has none).

## Implementation Approach

The work runs bottom-up and follows the S-06 and S-08 patterns:

1. A tauri-free `todo` domain module stores one row per pick. Own entries take their ticked state from `own_tasks.done_at`. GitLab entries store their own tick and a reference/title snapshot.
2. The GitLab refresh thread reconciles after successful rounds only.
3. Thin async commands and a `src/todo/` feature folder sit on top.

The day is the local calendar date. Visibility and roll-over are computed in Rust on every list call, so there is no scheduler and no state that only advances while the app runs.

**Visibility rule (day D, local date).** An entry is on D's list if it is not removed and not closed, and either open, or ticked on D. "Ticked on D" means `date(own_tasks.done_at) = D` for own entries and `date(ticked_at) = D` for GitLab entries. **Roll-over:** every `todo` write on D (list, add, tick, remove, reconcile) and the own-task done/reopen command first gives `closed_at` to every entry ticked before D (and not removed or closed), in the same transaction. A closed entry never comes back, even if its task is reopened later. Roll-over is idempotent, so running it on every write is safe and keeps the unique "live" index and the reopen rule correct even when the list was not read yet today. **Limit:** the number of open entries on D's list. It is checked only when adding.

## Critical Implementation Details

- **State sequencing (reconcile vs publish).** Reconciliation runs in `refresh_round` after `next` is computed and before `publish`. It runs only when `next.state` is `Ok`, using `next.items`. A store failure there is logged by kind and never changes the GitLab snapshot or the tray. The `todo-changed` event is emitted only when at least one row changed.
- **Day boundary.** Every "today" comes from `Local::now()` at the call (`on_store` already passes `now`). The view reloads when the window becomes visible or focused (the `useWorkTimer.ts` pattern), because WebView timers do not advance while the laptop sleeps. It also schedules one reload at the next local midnight while mounted, as a fallback, so a list left open overnight rolls over without user action.

## Phase 1: To-do domain and store records

### Overview

Add the `todo_items` table, the pure `todo` module (pick, tick, untick, remove, list with roll-over, limit, reconcile), an own-task transaction helper and the `todo.limit` settings loader. Test all of it on invented data.

### Changes Required:

#### 1. Migration v3

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append the S-07 table. Shipped migrations stay untouched.

**Contract**: `Migration { version: 3, … }` creating `todo_items` with columns:
- `id`
- `source` (`'own'|'gitlab'`, CHECK)
- `own_task_id` (INTEGER REFERENCES `own_tasks(id)`, set iff `source='own'`)
- `gitlab_kind` (`'issue'|'mergeRequest'`), `gitlab_ref`, `gitlab_title`, `gitlab_url` (nullable; the first three are set iff `source='gitlab'`)
- `picked_on` (TEXT `YYYY-MM-DD`), `picked_at` (TEXT local timestamp, same format as own tasks)
- `ticked_at`, `ticked_by` (`'user'|'source'`; GitLab only, both NULL or both set)
- `removed_at`, `closed_at`

A table CHECK enforces the per-source column rules. Two partial unique indexes allow at most one live entry per item, where live means `removed_at IS NULL AND closed_at IS NULL`: one on `own_task_id`, one on `(gitlab_kind, gitlab_ref)`.

#### 2. Own-task transaction helper

**File**: `src-tauri/src/own_tasks.rs`

**Intent**: Let the To-do tick change `done_at` inside the caller's transaction, so FR-008 stays one state and one write.

**Contract**: `pub fn set_done_in(tx: &Transaction<'_>, id: i64, done: bool, now: NaiveDateTime) -> Result<OwnTask, OwnTaskError>`. The existing `set_done` becomes `store.write(|tx| set_done_in(tx, …))`, with unchanged behaviour and tests.

#### 3. `todo` domain module

**File**: `src-tauri/src/todo/mod.rs`, `src-tauri/src/todo/error.rs`, `src-tauri/src/todo/tests.rs` (mirrors `work_time/`). No `tauri` import; `now` and `today` are passed in.

**Intent**: All "To do" rules live here and are unit-tested: visibility, roll-over, limit, duplicate refusal, own tick = `done_at`, and GitLab reconcile.

**Contract**:
- `TodoList { day: String, limit: Option<u32>, limit_error: Option<TodoError>, open_count: u32, items: Vec<TodoItem> }`. `limit: None` comes with `limit_error` set to `SettingsInvalid` or `ConfigDirUnresolved`, so the UI names the real cause.
- `TodoItem { id, picked_on, carried_over: bool, ticked: bool, picked_at, entry: TodoEntry }`.
- `TodoEntry` is tagged on `kind`:
  - `Own { task_id, space, title }`, with the title read live from `own_tasks`;
  - `Gitlab { item_kind, reference, title, web_url, closed_in_source: bool }`.
- All serde types use `camelCase`. Items are ordered with open entries first by `(picked_at, id)`, then ticked entries.
- Functions (each runs `roll_over_in` first inside its one write; `limit` is `Result<u32, TodoError>` from the settings loader):
  - `pub fn roll_over_in(tx: &Transaction<'_>, today: NaiveDate) -> Result<usize, StoreError>`: gives `closed_at` to every live entry ticked before `today`; idempotent.
  - `list(store, today, now, limit) -> TodoList`: one write that runs roll-over, then reads.
  - `add_own(store, task_id, today, now, limit)`
  - `add_gitlab(store, &GitlabItem, today, now, limit)`
  - `set_ticked(store, id, ticked, today, now)`: own entry → `own_tasks::set_done_in`; GitLab → `ticked_at`/`ticked_by='user'` or cleared. Only entries on today's list can change.
  - `remove(store, id, today, now)`: sets `removed_at`. An own task's done state is untouched.
  - `reconcile_gitlab(store, &AssignedItems, today, now) -> usize`: ticks with `ticked_by='source'` every open, live GitLab entry whose `(kind, reference)` is absent from the list, and updates `gitlab_title`/`gitlab_url` of present ones. Returns the count of changed rows.
- `TodoError` (`thiserror` + `Serialize`, `tag="kind", content="detail"`): `Store(StoreError)`, `LimitReached { limit }`, `AlreadyOnList`, `TaskDone`, `NotFound`, `NotInGitlabList`, `SettingsInvalid`, `ConfigDirUnresolved`, `Background`. No variant carries a title, reference, URL or host.
- Limit semantics: `add_*` with `Err(e)` returns that `e` (`SettingsInvalid` or `ConfigDirUnresolved`). With `Ok(n)`, it returns `LimitReached` when `open_count >= n`. Untick and remove are never limit-checked.

#### 4. `todo.limit` setting

**File**: `src-tauri/src/settings.rs`

**Intent**: Read the limit the way the GitLab block is read: independently and value-free on failure.

**Contract**: `pub fn load_todo_limit(path: &Path) -> Result<u32, TodoError>`.
- A missing file, absent block, `null` block or absent `limit` returns `5` (`DEFAULT_TODO_LIMIT`).
- `{"todo": {"limit": N}}` with integer `1 ≤ N ≤ 20` returns `N`.
- Anything else (wrong type, out of range, unknown key via `deny_unknown_fields`, wrong `version`, broken JSON) returns `SettingsInvalid` with no detail.

Vault and GitLab loaders ignore the `todo` block. A test proves that a broken `todo` block leaves both of them green.

#### 5. Integration and crash tests

**File**: `src-tauri/tests/todo.rs`, `src-tauri/tests/todo_crash.rs`

**Intent**: Prove the rules against a real store with production migrations, and prove that a killed writer never leaves a half-applied pick or tick.

**Contract**: Use `common::test_store_dir`, invented titles and `gitlab.example.invalid` references only. The crash writer alternates `add_own` + `set_ticked` (own) and `add_gitlab` + `reconcile_gitlab`. After each kill:
- every acknowledged add or tick is present, and no unacknowledged write is half-applied;
- at most one live entry exists per own task and per `(gitlab_kind, gitlab_ref)`;
- `ticked_at` and `ticked_by` are both set or both NULL on every GitLab row.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the new `todo` unit tests, `tests/todo.rs` and `tests/todo_crash.rs`
- Unit tests cover:
  - visibility per day (open carried, ticked today shown, ticked yesterday closed);
  - a task reopened after roll-over not coming back;
  - roll-over running inside add/tick/remove/reconcile without a prior `list` (a GitLab entry ticked yesterday can be re-picked today);
  - limit counting open entries only (limit 5, 5 picked, 2 ticked → 2 more can be added; the 3rd is refused);
  - a lowered limit refusing adds without dropping entries;
  - untick allowed above the limit;
  - duplicate live entry refused;
  - re-pick allowed after remove or close;
  - picking a done own task refused;
  - error JSON shape and no leaked titles or references.
- Settings tests cover the default, the valid range edges (1, 20), and invalid values 0, 21, `"5"`, `5.5` and unknown keys returning `SettingsInvalid`, with vault and GitLab loaders unaffected
- Existing `own_tasks` unit, integration and crash tests still pass after the `set_done_in` refactor
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass

**Implementation Note**: This phase has no manual gate; proceed once all automated verification passes. Phase blocks use plain bullets; the corresponding `- [ ]` checkboxes for these items live in the `## Progress` section at the bottom of the plan.

---

## Phase 2: GitLab reconciliation

### Overview

After every successful GitLab refresh round, auto-tick GitLab entries that are no longer assigned and open, refresh the stored title and link of the rest, and tell the webview.

### Changes Required:

#### 1. Reconcile hook in the refresh round

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: FR-012's "closed in the source ticks itself", driven by the existing 5-minute and on-demand rounds, and never on a failed round.

**Contract**:
- In `refresh_round`, after `next` is computed and before `publish`: if `next.state == GitlabState::Ok` and `next.items` is `Some`, call `todo::reconcile_gitlab` through `app.state::<StoreHandle>().with_store(today, …)` with the local `today`/`now`.
- If it changed ≥ 1 row, emit `todo-changed` (no payload) after `publish`.
- A store error is logged as `todo: reconcile failed: <kind>`. The snapshot, the GitLab health status and the round are unaffected.
- Log lines carry kinds and counts only, never a reference or title.

#### 2. Reconcile tests

**File**: `src-tauri/src/todo/tests.rs`, `src-tauri/tests/todo.rs`

**Intent**: Pin the rules the hook relies on.

**Contract**: Tests cover:
- a missing item is ticked with `closed_in_source`;
- a present item gets its title and URL updated and stays open;
- ticked, removed or closed entries are left alone;
- an empty successful list ticks all open GitLab entries;
- reconcile never touches own entries;
- the returned count matches the changed rows (0 when nothing changed, so no event).

The "only on `Ok`" guard lives in `refresh_round`. It is covered by a small pure helper (`fn reconcile_input(&GitlabSnapshot) -> Option<&AssignedItems>`) with unit tests for `Ok`, `Unavailable`, `Error`, `NotConfigured` and `Starting`.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes, including the reconcile unit and integration tests and the `reconcile_input` state tests
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass

#### Manual Verification:

- With VPN, an item closed or merged in GitLab that sits in "Do zrobienia" shows as ticked "zamknięte w GitLabie" after "Odśwież" in "Praca (GitLab)" (checked in Phase 3's view)
- Without VPN, a refresh leaves every GitLab entry in "Do zrobienia" unchanged

**Implementation Note**: After automated verification passes, proceed to Phase 3 without a manual gate. Manual checks 2.3–2.4 need the Phase 3 view; run them during the Phase 3 manual pass.

---

## Phase 3: Commands and the "Do zrobienia" view

### Overview

Expose the domain through five async commands and build the `src/todo/` feature: today's list, the picker, the limit counter, badges, and loading, error and empty states. Make the view the first nav tab and the default view.

### Changes Required:

#### 1. Commands

**File**: `src-tauri/src/todo/commands.rs` (the only `todo` file importing `tauri`), registered in `src-tauri/src/lib.rs`

**Intent**: Thin async wrappers. They read the limit from `settings.json` on each call, so an edit takes effect without a restart. GitLab picks come from the live snapshot, never from UI-supplied text.

**Contract**:
- `list_todo() -> TodoList`
- `add_todo_own_task(taskId) -> TodoList`
- `add_todo_gitlab_item(itemKind, reference) -> TodoList`: looks the item up in `GitlabRefreshState::snapshot().items`, and returns `NotInGitlabList` if it is absent.
- `set_todo_ticked(id, ticked) -> TodoList`
- `remove_todo(id) -> TodoList`

Each command:
- runs via `spawn_blocking` + `with_store` (the `on_store` pattern);
- resolves the settings path from `app.path().app_config_dir()`; an unresolved dir is `Err(ConfigDirUnresolved)` for the limit;
- returns the fresh list;
- logs failures by kind only.

The existing `set_own_task_done` command (`src-tauri/src/lib.rs:103`) becomes one `store.write` that calls `todo::roll_over_in(tx, today)` and then `own_tasks::set_done_in`. A task ticked yesterday and reopened in "Moje zadania" before the list was read today then stays closed in "Do zrobienia".

#### 2. Shared store-error helpers (frontend)

**File**: `src/components/storeError.ts` (new), `src/own-tasks/api.ts`, `src/own-tasks/types.ts`

**Intent**: One Polish message table and one `StoreError` mirror for both features, instead of a copy.

**Contract**: Move the `StoreError` type and `storeErrorMessage` there, then import them in `own-tasks` and `todo`. Own-tasks behaviour is unchanged.

#### 3. `src/todo/` feature folder

**File**: `src/todo/types.ts`, `src/todo/api.ts`, `src/todo/useTodo.ts`, `src/todo/TodoView.tsx`, `src/App.css`, `src/gitlab/useGitlab.ts` (the `refreshOnMount` option)

**Intent**: Display the Rust results and send user actions; no rule is recomputed in the UI.

**Contract**:
- `types.ts`: mirrors `TodoList`, `TodoItem`, `TodoEntry` (tagged on `kind`) and `TodoError`.
- `api.ts`: one function per command plus `todoErrorMessage`, covering:
  - `LimitReached` → "Limit {n} otwartych pozycji — odhacz lub zdejmij coś, żeby dodać";
  - `SettingsInvalid` → names the `"todo": { "limit": 1–20 }` block;
  - `ConfigDirUnresolved` → says the app config folder cannot be resolved (not the `todo` block);
  - `NotInGitlabList`, `TaskDone`, `AlreadyOnList` → refresh hint.
- `useTodo.ts`:
  - load/error/data states with `lastSuccessAt`;
  - reloads on mount, on the `todo-changed` event, on `visibilitychange` (visible) and window `focus` (as `useWorkTimer.ts:62-72`), and at the next local midnight as a fallback;
  - like `useOwnTasks`, a failed reload never replaces a loaded list and is shown next to it.
- `TodoView.tsx`:
  - header line "Otwarte N/limit" (or the settings error);
  - open list in pick order, then "Zrobione dziś". Each row has:
    - a checkbox;
    - a source badge (GitLab reference with MR/issue, or Praca/Prywatne);
    - the title, as a link via `openGitlabItem` when `webUrl` is set;
    - an "od DD.MM" badge when `carriedOver`;
    - a "zamknięte w GitLabie" badge when `closedInSource`;
    - a "Zdejmij" button;
  - the empty data state says "Nic na dziś — dodaj z list poniżej".
- Picker "Dodaj z list":
  - candidates are GitLab MRs and issues from `useGitlab({ refreshOnMount: false })`'s snapshot and open tasks from `listOwnTasks()`, minus those already on the list. `src/gitlab/useGitlab.ts` gets this option (default `true`, so "Praca (GitLab)" is unchanged); without it, every visit to the default tab would start a `glab` round;
  - add buttons are disabled at the limit;
  - when GitLab is unavailable or erroring, the GitLab section shows the calm state line, and stale snapshot items stay pickable;
  - an own-task load failure shows `ErrorState` in its section.
- When open GitLab entries exist and the GitLab state is not `ok`, a calm line reads "GitLab niedostępny — stan pozycji z GitLaba niezweryfikowany (ostatnie odświeżenie HH:MM)".

#### 4. Navigation

**File**: `src/App.tsx`

**Intent**: US-01's morning starts on the daily list.

**Contract**: Add view id `"todo"` labelled "Do zrobienia" as the first `VIEWS` entry and make it the `useState` default. The other entries keep their order.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` and `npm run build` pass (zero TS errors)
- `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/`
- A command-serialization unit test pins the `TodoList` JSON shape that the TS mirror relies on

#### Manual Verification:

- At app start "Do zrobienia" is the active tab, and the empty list shows "Nic na dziś — dodaj z list poniżej"
- Pick 5 items (mixed GitLab and own); the 6th add is refused with the limit message; tick 2 and 2 more can be added
- Ticking an own task shows it done in "Moje zadania"; reopening it there shows it unticked in "Do zrobienia"
- Remove an item: it leaves the list, an own task's done state is unchanged, and the item is pickable again
- Set `"todo": { "limit": 0 }` in settings.json: the view shows the settings error, the list stays visible and adding is refused; restoring a valid value works without a restart
- Without VPN, GitLab entries show their stored titles with the "niezweryfikowany" line and stay unticked
- Next day (or with the system date moved forward on a backed-up store): open items carry over with "od DD.MM", and items ticked the day before are gone
- After a restart all entries and ticks are intact

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful.

---

## Testing Strategy

### Unit Tests:

- Visibility and roll-over across invented days, including ticked-today vs ticked-yesterday, a reopen after close, and roll-over inside every write without a prior `list`
- The limit as open count: the 5/2-ticked case, a lowered limit, and untick above the limit
- Duplicate live entry refusal, re-pick after remove or close, and picking a done task
- Own tick ↔ `done_at` both ways, and the GitLab user tick vs source tick
- Reconcile: missing, present, already ticked or removed, an empty list, own entries untouched, and the change count
- `reconcile_input` per `GitlabState`
- `load_todo_limit` defaults, bounds and invalid shapes, with neighbour loaders unaffected
- Error JSON shapes and no leaked titles, references or hosts

### Integration Tests:

- `tests/todo.rs`: full flows against `Store::open` with production migrations
- `tests/todo_crash.rs`: kill harness for pick+tick and pick+reconcile writes

### Manual Testing Steps:

1. Back up and clear `day-navigator.db*` per AGENTS.md before `npm run tauri dev`. The dev build shares the installed app's data dir, so restore afterwards.
2. Follow the Phase 3 manual list in order, then the Phase 2 VPN on/off checks.
3. Check the app log: no titles, references or hosts from "To do" lines.

## Performance Considerations

The list holds a few rows per day; every query is a small indexed scan. Reconcile runs at most once per GitLab round (5 min or on demand) and writes only changed rows. No new background thread.

## Migration Notes

Migration v3 only adds a table, so existing data is untouched. A store opened by an older app build is refused as `NewerSchema` (existing behaviour). `settings.json` without a `todo` block keeps working with the default limit of 5.

## References

- Roadmap: `context/foundation/roadmap.md` S-07; PRD FR-011, FR-012, FR-008, US-01
- Own-task done state: `src-tauri/src/own_tasks.rs:303`, `src-tauri/src/store/schema.rs:28`
- GitLab round: `src-tauri/src/gitlab_refresh.rs:167-200`; fetch filter: `src-tauri/src/cli/mod.rs:128-134`
- Settings block loader pattern: `src-tauri/src/settings.rs:97-135`
- Command/store pattern: `src-tauri/src/lib.rs:63`, `src-tauri/src/work_time/commands.rs`
- Frontend pattern: `src/own-tasks/useOwnTasks.ts`, `src/gitlab/useGitlab.ts`, `src/components/ErrorState.tsx`
- Prior plans: `context/archive/2026-11-23-own-tasks/plan.md`, `context/archive/2026-11-23-gitlab-assigned-list/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: To-do domain and store records

#### Automated

- [x] 1.1 `cargo test` passes in `src-tauri/`, including the new `todo` unit tests, `tests/todo.rs` and `tests/todo_crash.rs` — 802eeba
- [x] 1.2 Unit tests cover visibility, roll-over (including inside every write), reopen-after-close, the open-count limit (5 picked, 2 ticked → 2 more; 3rd refused), lowered limit, untick above limit, duplicates, re-pick, done-task refusal, error JSON and no leaks — 802eeba
- [x] 1.3 Settings tests cover default, range edges 1 and 20, invalid 0/21/"5"/5.5/unknown keys → `SettingsInvalid`, vault and GitLab loaders unaffected — 802eeba
- [x] 1.4 Existing `own_tasks` unit, integration and crash tests still pass after the `set_done_in` refactor — 802eeba
- [x] 1.5 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass — 802eeba

### Phase 2: GitLab reconciliation

#### Automated

- [x] 2.1 `cargo test` passes, including the reconcile unit and integration tests and the `reconcile_input` state tests — d88dd4d
- [x] 2.2 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass — d88dd4d

#### Manual

- [x] 2.3 With VPN, an item closed or merged in GitLab that sits in "Do zrobienia" shows as ticked "zamknięte w GitLabie" after a refresh
- [x] 2.4 Without VPN, a refresh leaves every GitLab entry in "Do zrobienia" unchanged

### Phase 3: Commands and the "Do zrobienia" view

#### Automated

- [x] 3.1 `npm run lint`, `npm run format:check` and `npm run build` pass (zero TS errors) — 814a9ff
- [x] 3.2 `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/` — 814a9ff
- [x] 3.3 A command-serialization unit test pins the `TodoList` JSON shape that the TS mirror relies on — 814a9ff

#### Manual

- [x] 3.4 At app start "Do zrobienia" is the active tab and the empty list shows "Nic na dziś — dodaj z list poniżej"
- [x] 3.5 Pick 5 items (mixed); the 6th is refused with the limit message; tick 2 and 2 more can be added
- [x] 3.6 Ticking an own task shows it done in "Moje zadania"; reopening it there shows it unticked in "Do zrobienia"
- [x] 3.7 Remove an item: it leaves the list, an own task's done state is unchanged, and it is pickable again
- [x] 3.8 An invalid `todo.limit` shows the settings error with the list still visible; a valid value works without a restart
- [x] 3.9 Without VPN, GitLab entries show stored titles with the "niezweryfikowany" line and stay unticked
- [x] 3.10 Next day: open items carry over with "od DD.MM" and items ticked the day before are gone
- [x] 3.11 After a restart all entries and ticks are intact
