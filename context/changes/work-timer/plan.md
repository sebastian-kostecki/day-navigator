# Work Timer Implementation Plan

## Overview

Roadmap slice S-08 (FR-013, FR-017, NFR "Trwałość czasu pracy"): the user starts and stops work time with START/STOP in a header that is always visible at the top of the window. The header shows today's running total. A running counter survives closing the window, "Zakończ", a Windows restart and a crash. In a new "Czas pracy" view the user can correct an entry's start or end, add a missing entry and remove a mistaken one. Removed entries are kept in the store and stop counting.

This slice adds the first app-owned table in the F-01 store (migration 1), the first store-backed commands, and the app shell that AGENTS.md describes: a fixed header plus one active view.

## Current State Analysis

- **Store ready, but empty.** `schema::MIGRATIONS` is `&[]` (`src-tauri/src/store/schema.rs:10`). `Store::write` runs one IMMEDIATE transaction and commits only on `Ok` (`src-tauri/src/store/mod.rs:152-163`). `StoreHandle::with_store(today, f)` locks and opens lazily (`mod.rs:233-241`). Store commands must be `async` and call `with_store` inside `spawn_blocking`, because the health round holds the lock during reopen and backup (`mod.rs:180-181`).
- **No command uses the store yet.** The `blocking` helper (`src-tauri/src/lib.rs:76-87`) is hard-typed to `TaskError`, and `TaskError` has no `From<StoreError>` (`src-tauri/src/tasks.rs:20-51`).
- **`StoreError` maps constraint violations to `Sql`** (`src-tauri/src/store/error.rs:61-86`). It carries no row values and no paths (`error.rs:3-5`). Domain refusals (overlap, end before start) therefore need their own error type, checked in Rust before writing.
- **Time handling.** `chrono 0.4` with `clock` and `serde` (`src-tauri/Cargo.toml:52`). rusqlite 0.43 has no `chrono` feature, so instants are stored as integers. Only `chrono::Local` is used in the app (`lib.rs:120`, `lib.rs:142`), and the store takes `today: NaiveDate` from the caller.
- **Lifecycle.** Closing the window hides it (`lib.rs:194-201`). Only the tray's "Zakończ" really quits (`lib.rs:212-221`). Nothing starts the app after a Windows restart, and nothing flushes on exit. So durability has to come from committing at the moment of START/STOP, not from shutdown hooks.
- **Crash harness.** `tests/common/crash.rs` re-runs an `#[ignore]` writer test as a child process. The child prints `committed <n>` and is killed with `KillAt::AfterCommits | AfterDelay` (`crash.rs:18-74`). `tests/store_crash.rs` is the worked example, and `tests/common/mod.rs:91-96` provides `test_store_dir()`.
- **Events.** One event exists: `health-changed`, emitted via `tauri::Emitter` (`src-tauri/src/runtime.rs:21,35,205-224`). The frontend listens first and loads second (`src/health/useHealth.ts:27-54`). The capability already allows listen/unlisten.
- **UI.** `src/App.tsx:5-12` stacks `TasksView` and `HealthPanel` with no header, navigation or view state. Styling is one plain global `src/App.css` with dark mode via `prefers-color-scheme` (`App.css:206-258`). The create form in `src/tasks/TasksView.tsx:82-170` is the form pattern to mirror. `src/components/ErrorState.tsx` takes `{ title, message, onRetry? }`. There is no date library (`package.json`).
- **Roadmap coupling.** S-01 owns "navigation between views and the fixed header area" (`context/foundation/roadmap.md:112`) and runs in parallel. Its change folder has no plan yet. S-06 will read "is the counter running" for its space default (`roadmap.md:181`).

## Desired End State

- The header is always visible above the active view. It shows "Dziś H:MM:SS", the sum of today's entry portions including the running one, ticking every second while running, and one START or STOP button. When the store fails, the header shows an error with retry and the time of the last successful refresh, never 0:00:00.
- Clicking START commits a running entry immediately. Closing the window, "Zakończ", killing the process or restarting Windows leaves that entry running. On the next launch the header shows the same start, with elapsed time counted through the downtime.
- The "Czas pracy" view shows a chosen day's entries, plus the day total. An entry across midnight appears under both days with the full start/end and only that day's portion counted. The user can add a closed entry, edit start/end (start only for the running entry), and remove an entry after an inline confirmation. A refusal (overlap, end not after start, time in the future, a time that doesn't exist in local time) shows a Polish message and saves nothing.
- Verify: `cargo test` covers the rules (including midnight split and DST days in `Europe/Warsaw`), reopen durability and the crash test. The manual steps in each phase pass.

### Key Discoveries:

- `src-tauri/src/store/mod.rs:152-163`: `write` = IMMEDIATE transaction. A read-validate-write sequence inside one `write` closure is atomic against other writers.
- `src-tauri/src/store/mod.rs:3`: the store module never reads the clock. The work-time domain follows suit and takes `now` and the time zone as parameters.
- `src-tauri/src/lib.rs:189`: `StoreHandle` is managed state and is not opened in `setup`. The first command or health round opens it.
- `src-tauri/tests/store.rs:291`: a constraint error is proven to carry no row values. New errors must hold that line, so an overlap is reported by entry id, never by times.
- `src/health/useHealth.ts:21-54`: subscribe before the first load, guard with `cancelled`, unlisten on cleanup. Reuse this for `work-time-changed`.

## What We're NOT Doing

- Reminders for a long-running or not-started counter (S-10, FR-018) and any heartbeat or "last seen" tracking. After a shutdown, a forgotten STOP stays running until the user stops and corrects it (decided).
- Month sums, the norm, the deviation, days off (S-09, FR-014). Only today's total in the header and the per-day total in the view.
- A start-date gate for 2026-12-01: entries can be recorded from whenever this ships (decided). November test entries can be removed.
- A START/STOP item in the tray menu. A restore button for removed entries (they stay in the database only). Hard deletion.
- Merging or auto-trimming overlapping entries: overlaps are refused (decided).
- Autostart, idle detection, holiday calendars (PRD Non-Goals).
- Second-level precision in corrections: the edit and add forms use minutes. START/STOP keep millisecond instants, and an edit keeps the stored instant of a field whose minute was not changed (Phase 1 §3).
- Reading or writing vaults, `settings.json` changes, a date library in the UI, a router or a global state library.
- Building S-01's GitLab view. This slice only leaves a view switch that S-01 extends with one more entry.

## Implementation Approach

The data is one table of entries stored as absolute instants (UTC epoch milliseconds): start, optional end (none = running), and an optional removal instant (soft removal). All rules live in a tauri-free `work_time` module that takes `now` and a `chrono::TimeZone`. Production passes `chrono::Local`, and tests pass `chrono_tz::Europe::Warsaw`, so midnight and DST cases are deterministic on any CI runner.

Every mutation is one `Store::write` closure that reads the neighbours, validates and writes, so a refusal changes nothing. Local day boundaries and local "YYYY-MM-DDTHH:MM" strings from the form are resolved in Rust. The UI formats only what Rust returns and adds `Date.now() - runningFromMs` for the live tick.

Commands emit `work-time-changed` after each successful write. The header and the view both listen, so they never disagree. The app shell (view state and fixed header) is built here once, and S-01 adds its view to it.

## Critical Implementation Details

- **Timing & lifecycle.** The webview keeps running while hidden in the tray, possibly for days, and WebView2 may throttle hidden timers. The header must refetch status when the page becomes visible again (`visibilitychange`) or the window gains focus (the tray's `set_focus()`, `runtime.rs:115-117`; `visibilitychange` on `window.hide()`/`show()` in WebView2 is unverified), and when the local day ends, via a `setTimeout` armed to `dayEndMs` from the last status that runs whether or not the counter is running. Otherwise "Dziś" keeps yesterday's total after midnight.
- **State sequencing.** Overlap and "at most one running entry" are checked inside the same IMMEDIATE transaction as the insert/update. A partial unique index enforces the single running entry as a second line of defence. A double-clicked START must yield `AlreadyRunning` for the second call, never two running rows.
- **Local time resolution.** A form time that falls in the spring DST gap (e.g. 2027-03-28 02:30 in Poland) is refused as `InvalidLocalTime`. An ambiguous autumn time (e.g. 2026-10-25 02:30) resolves to the earlier instant. Day bounds use local midnight via `earliest()`, so DST days are 23 h or 25 h long.

## Phase 1: Work-time domain and store

### Overview

Add the `work_entries` table as migration 1 and a tauri-free `work_time` module holding every rule: start, stop, add, edit, remove, day totals split at local midnight, and local-time resolution. Unit tests cover the rules, and integration tests cover reopening and a hard kill.

### Changes Required:

#### 1. Migration 1

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append the first production migration with the entries table. The schema itself enforces end-after-start and at most one running (not removed) entry, as a backstop for the Rust checks.

**Contract**: `Migration { version: 1, sql }` with the snippet below. The production list stays validated by the existing `accepts_contiguous_and_production_list` test.

```sql
CREATE TABLE work_entries (
  id            INTEGER PRIMARY KEY,
  start_ms      INTEGER NOT NULL,
  end_ms        INTEGER,            -- NULL = running
  removed_at_ms INTEGER,            -- NULL = counted; soft removal otherwise
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  CHECK (end_ms IS NULL OR end_ms > start_ms)
);
CREATE UNIQUE INDEX work_entries_one_running
  ON work_entries ((end_ms IS NULL)) WHERE end_ms IS NULL AND removed_at_ms IS NULL;
CREATE INDEX work_entries_start ON work_entries (start_ms);
```

#### 2. Work-time error

**File**: `src-tauri/src/work_time/error.rs` (new)

**Intent**: One error enum for every work-time refusal and failure, serialized for the UI, with no times or row values inside, the same rule as `StoreError`.

**Contract**: `WorkTimeError` derives `thiserror::Error` + `Serialize`, `#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]`. Variants: `Store(StoreError)`, `AlreadyRunning`, `NotRunning`, `NotFound`, `EndNotAfterStart`, `InFuture`, `Overlap { entry_id: i64 }`, `InvalidLocalTime`, `RunningEndEdit` (an end was sent for the running entry: use STOP), `Background` (the blocking task failed). `impl From<StoreError>` and `From<rusqlite::Error>` (via `StoreError`).

#### 3. Rules and operations

**File**: `src-tauri/src/work_time/mod.rs` (new), optionally split into `rules.rs` for the pure functions

**Intent**: Pure rule functions plus store operations. Each mutation is one `Store::write` closure that loads the relevant rows, validates and writes, so a refusal writes nothing. Removed rows are ignored by every rule and total.

**Contract**:
- Types (serde, `rename_all = "camelCase"`):
  - `WorkTimerStatus { running: Option<RunningEntry>, todayClosedMs: i64, runningFromMs: Option<i64>, dayStartMs: i64, dayEndMs: i64 }`. `RunningEntry { id, startMs }`. `runningFromMs = max(start, dayStart)` so the UI's live total is `todayClosedMs + (now - runningFromMs)`.
  - `DayEntries { day: "YYYY-MM-DD", entries: Vec<EntryRow>, closedMs: i64, runningFromMs: Option<i64>, dayEndMs: i64 }`.
  - `EntryRow { id, startMs, endMs: Option<i64>, startLocal: "YYYY-MM-DDTHH:MM", endLocal: Option<String>, dayPortionMs: i64 }`. The portion of a running entry is computed up to `now`, capped at the day end.
- Functions, generic over `Tz: TimeZone`, taking `now_ms: i64`:
  - `status(&Store, now, tz) -> WorkTimerStatus`
  - `entries_for_day(&Store, day: NaiveDate, now, tz) -> DayEntries`. Entries overlapping `[dayStart, dayEnd)`, sorted by start, the running one included.
  - `start(&mut Store, now)`: `AlreadyRunning` if one runs, `Overlap` if a closed entry ends after `now`.
  - `stop(&mut Store, now)`: `NotRunning`. Sets `end = now`. A stop within the same millisecond as start (`end <= start`) is `EndNotAfterStart`.
  - `add(&mut Store, start_local, end_local, now, tz)`: both ends required.
  - `update(&mut Store, id, start_local, end_local: Option, now, tz)`: `end_local` is required for a closed entry and must be `None` for the running one (`RunningEndEdit`).
  - `remove(&mut Store, id, now)`: sets `removed_at_ms`. Removing the running entry stops counting.
  - `is_running(&Store) -> bool`, for S-06.
- Validation order for add/update: resolve local times (`InvalidLocalTime`) → `EndNotAfterStart` → `InFuture` (start or end > now) → `Overlap` against every other counted entry. Intervals are half-open `[start, end)`. The running entry counts as `[start, +∞)`. Adjacent entries (one ends 12:00, the next starts 12:00) do not overlap. `NotFound` covers unknown or removed ids.
- Local resolution: `"YYYY-MM-DDTHH:MM"` → `tz.from_local_datetime`. `None` → `InvalidLocalTime`, `Ambiguous` → earlier. Day bounds: local midnight of `day` and `day + 1` via `earliest()`.
- Unchanged minute keeps the stored instant: in `update`, when a submitted local start (or end) equals the stored instant floored to the minute in `tz`, the stored millisecond instant is kept, not the resolved `HH:MM:00`. The form always re-sends both fields, so without this rule editing only the end would move the start up to 59 s earlier. An entry started by START right after a STOP in the same minute (STOP 12:00:30, START 12:00:40) would then be refused as `Overlap` with its neighbour, although the user never touched its start.

#### 4. Module registration and test dependency

**File**: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`

**Intent**: Declare `pub mod work_time;` and add `chrono-tz` as a **dev-dependency only**, so the DST and midnight tests use the real `Europe/Warsaw` rules regardless of the runner's time zone.

**Contract**: `[dev-dependencies] chrono-tz = "0.10"` (or the newest that passes `cargo audit`). No production dependency changes.

#### 5. Integration tests

**File**: `src-tauri/tests/work_time.rs`, `src-tauri/tests/work_time_crash.rs` (new)

**Intent**: Prove durability against the real store. A running entry and history survive closing and reopening the store. A hard kill of a process that loops START/STOP commits leaves the store consistent. Removed entries remain as rows.

**Contract**: Use `Store::open(dir, schema::MIGRATIONS, today)` with `test_store_dir()`. The crash test reuses `tests/common/crash.rs` (`run_writer_until`, `KillAt`) like `tests/store_crash.rs`. After each kill and reopen: `quick_check` ok, at most one running entry, every closed entry has `end > start`, no two counted entries overlap, and the number of start/stop actions ≥ the last reported commit. Invented timestamps only.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the new unit tests and `tests/work_time.rs` / `tests/work_time_crash.rs`
- Unit tests cover: midnight split (22:00–02:00 → 2 h + 2 h), the DST days 2026-10-25 (25 h) and 2027-03-28 (23 h) in `Europe/Warsaw`, ambiguous → earlier, a gap time refused, adjacent entries allowed, overlap with a closed entry and with the running entry refused, `InFuture`, `EndNotAfterStart`, `RunningEndEdit`, a second START → `AlreadyRunning`, STOP without running → `NotRunning`, removed entries ignored in totals and overlap checks, and a refusal leaves the table unchanged
- A test proves the partial unique index rejects a second running row inserted directly
- A test asserts that a serialized `WorkTimeError::Overlap` contains only the entry id
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- `cargo audit` passes with the `chrono-tz` dev-dependency
- Unit tests prove an unchanged minute keeps the stored instant: editing only the end keeps the start's milliseconds, and an entry started right after a STOP in the same minute stays editable without `Overlap`

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase. Phase blocks use plain bullets — the corresponding `- [ ]` checkboxes for these items live in the `## Progress` section at the bottom of the plan.

---

## Phase 2: Commands and event

### Overview

Expose the domain to the UI through async commands that run on the blocking pool, emit `work-time-changed` after every successful write, and add the TypeScript mirrors and typed API functions.

### Changes Required:

#### 1. Generic blocking helper

**File**: `src-tauri/src/lib.rs`

**Intent**: Make `blocking` serve both the vault commands and the store commands without changing the existing `TaskError` behaviour.

**Contract**: `async fn blocking<T, E>(work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Result<T, E>` where `E: From<BackgroundFailed> + Send + 'static`. A small `BackgroundFailed` type carries the join-error text. `TaskError` keeps mapping it to today's `VaultError::Io { path: "", kind: "background task failed: …" }`, and `WorkTimeError` maps it to `Background`.

#### 2. Work-time commands

**File**: `src-tauri/src/work_time/commands.rs` (new, the only file in `work_time` that imports `tauri`), registered in `src-tauri/src/lib.rs` `generate_handler!`

**Intent**: Thin async wrappers. Each takes `AppHandle`, reads `now` (`Utc::now().timestamp_millis()`) and `today` (`Local::now().date_naive()`), moves an `AppHandle` clone into `blocking` and calls `app.state::<StoreHandle>().with_store(today, …)` inside it, and on a successful write emits `work-time-changed` (no payload). `StoreHandle` is neither `Clone` nor `Arc` (`store/mod.rs:182-185`), so a `State<'_, StoreHandle>` cannot cross into the `'static` closure; the health round reaches it the same way (`runtime.rs:277`). Errors are logged by kind only.

**Contract**:
- `work_timer_status() -> WorkTimerStatus`
- `list_work_entries(day: String /* YYYY-MM-DD */) -> DayEntries` (a bad date → `InvalidLocalTime`)
- `start_work() -> WorkTimerStatus`, `stop_work() -> WorkTimerStatus`
- `add_work_entry(start: String, end: String) -> ()`
- `update_work_entry(id: i64, start: String, end: Option<String>) -> ()`
- `remove_work_entry(id: i64) -> ()`

All return `Result<_, WorkTimeError>`. Event name constant `"work-time-changed"`.

#### 3. TypeScript mirrors and API

**File**: `src/worktime/types.ts`, `src/worktime/api.ts` (new)

**Intent**: Mirror the Rust types and error union, with one typed function per command. `api.ts` is the only file in the folder that imports `invoke`, and it holds the Polish message mapper for `WorkTimeError`.

**Contract**:
- `WorkTimeError` is a tagged union on `kind`. `Store` nests `StoreError`'s `{kind, detail}`.
- `workTimeErrorMessage(e, entriesById?)` returns Polish text, e.g. `Overlap` → "Wpis nachodzi na inny wpis (08:00–12:00)", using times looked up from the loaded list by `entryId`, or a generic phrase if the entry is not loaded.
- A `WORK_TIME_CHANGED` event name constant.

### Success Criteria:

#### Automated Verification:

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass
- Existing vault task tests still pass with the generic `blocking`
- `npm run build`, `npm run lint` and `npm run format:check` pass

#### Manual Verification:

- In `npm run tauri dev`, calling `start_work` / `stop_work` from the devtools console (`window.__TAURI_INTERNALS__.invoke`) creates and closes an entry, and the event fires

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: App shell and header counter

### Overview

Give `App.tsx` one piece of view state and a fixed header. The header holds the work-time counter with START/STOP and a view switch, and the existing Tasks and Health screens become views.

### Changes Required:

#### 1. App shell

**File**: `src/App.tsx`, `src/App.css`

**Intent**: Implement the AGENTS.md navigation: the active view in one `useState`, a sticky header that is always visible, and the active view below it. S-01 later adds its view as one more entry in the same list.

**Contract**: `type View = "tasks" | "work-time" | "health"`. The view switch is a row of buttons in the header ("Zadania", "Czas pracy", "Stan źródeł") with `aria-current` on the active one. The default view is "tasks". Header CSS: `position: sticky; top: 0`, light and dark variants alongside the existing `prefers-color-scheme` block. No router, no context provider.

#### 2. Header counter

**File**: `src/worktime/useWorkTimer.ts`, `src/worktime/WorkTimerHeader.tsx`, `src/worktime/format.ts` (new)

**Intent**: Load the status, keep it fresh, tick locally, and offer START/STOP. A failure must be visible and must never read as 0:00:00.

**Contract**:
- `useWorkTimer()` returns a discriminated state `loading | error {message, lastOkAt} | loaded {status}`, plus `start()`, `stop()`, `busy`, `actionError`, `retry()`.
- It subscribes to `work-time-changed` before the first load (the `useHealth` pattern) and refetches on the event, on `visibilitychange` → visible, on window `focus`, and at day end via a `setTimeout` armed to `status.dayEndMs` (re-armed after each status load, armed whether or not the counter is running).
- Latest-request-wins: each status request (load, refetch, START/STOP response) takes a sequence number, and a response older than the last applied one is dropped (the `TasksView` guard), so a slow read cannot show "stopped" after START.
- The 1 s interval runs only while running.
- The display is `Dziś H:MM:SS` with total = `todayClosedMs + (running ? Date.now() - runningFromMs : 0)`. `format.ts` has `formatDuration(ms)` and `formatClock(ms)` (HH:MM via `toLocaleTimeString("pl-PL")`).
- The header shows START when stopped and STOP when running, disabled while `busy`. After `AlreadyRunning`/`NotRunning` it just refetches with no message, because another action already changed the state.
- Error: compact "Czas pracy niedostępny: <message> (ostatnio OK HH:MM)" with "Spróbuj ponownie". START/STOP are hidden while in error.

### Success Criteria:

#### Automated Verification:

- `npm run build`, `npm run lint`, `npm run format:check` pass
- `cargo test` still passes

#### Manual Verification:

- The header stays visible while scrolling the Tasks view, and the view switch shows Zadania, Czas pracy (placeholder until Phase 4) and Stan źródeł
- START → the counter ticks every second. STOP → it freezes. START again later → today's total continues from the frozen value
- With the counter running: close the window, reopen it from the tray → still running with the same total. "Zakończ" and relaunch → still running, and the downtime is counted
- With the counter running, end the process in Task Manager and relaunch → still running from the same start
- Restart Windows with the counter running, relaunch → still running
- Leave the app hidden in the tray across midnight (or set the clock forward), show it → "Dziś" counts only the time since midnight
- Make the store fail (e.g. rename `day-navigator.db` while the app is quit and keep a backup so it is refused as `Missing`), launch → the header shows the error and retry, not 0:00:00
- With the counter stopped and the window left open across midnight (or the clock set forward), "Dziś" resets without any interaction

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 4: "Czas pracy" view

### Overview

The correction screen: pick a day, see its entries and total, add a missing entry, edit start/end, and remove an entry with inline confirmation. Refusals are shown in Polish.

### Changes Required:

#### 1. Entries hook

**File**: `src/worktime/useWorkEntries.ts` (new)

**Intent**: Load one day's entries, refetch on `work-time-changed`, and expose add/update/remove with per-row busy state and the latest-response-wins guard from `TasksView`.

**Contract**: `useWorkEntries(day: string)` returns `loading | error {message, lastOkAt} | loaded {data: DayEntries}`, plus `add`, `update`, `remove`, `busyIds`, `retry`. Mutations return the `WorkTimeError` for the form to show, and the list refreshes via the event.

#### 2. View

**File**: `src/worktime/WorkTimeView.tsx` (new), `src/App.tsx` (replace the placeholder), `src/App.css`

**Intent**: Render loading, error (`ErrorState` with the last OK time) and data. "Empty" is an explicit message ("Brak wpisów w tym dniu"). Mirror the `TasksView` form pattern (`fieldset disabled` while saving, inline `role="alert"` error).

**Contract**:
- Day picker: `<input type="date">` defaulting to today, with "◀ / ▶" buttons.
- Each row: start–end in HH:MM, with the date shown when the entry crosses into or out of the day, "trwa" for the running one, and the day portion duration.
- The footer shows the day total. While an entry is running, the total ticks every second on today's view, computed the same way as the header (`closedMs + now − runningFromMs`).
- Edit: inline row form with `datetime-local` (`step=60`) prefilled from `startLocal`/`endLocal`. No end field for the running entry.
- Add: form with start and end `datetime-local`, prefilled with the chosen day.
- Remove: "Usuń" → inline "Usunąć wpis? Tak / Anuluj". There is no restore button.
- Error text comes from `workTimeErrorMessage` with the overlapping entry's times.

### Success Criteria:

#### Automated Verification:

- `npm run build`, `npm run lint`, `npm run format:check` pass
- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass

#### Manual Verification:

- Add a missing entry 08:00–12:00 for yesterday: it appears in the list and the day total is 4:00
- Add 11:30–13:00 on the same day → refused with "nachodzi na 08:00–12:00", and nothing changes. Add 12:00–13:00 → accepted (adjacent)
- Edit an entry's end to before its start → refused. Set an end in the future → refused
- Edit the running entry's start to earlier: the header total grows accordingly, and no end field is offered
- Add 22:00–02:00 across midnight: it shows under both days with 2:00 counted on each
- Remove an entry after confirmation: it disappears, the total drops, and the header updates if it was today's
- A day with no entries shows "Brak wpisów w tym dniu". With the store failing, the view shows `ErrorState`, not an empty list

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Rules in `work_time` with `chrono_tz::Europe::Warsaw` and invented instants:
  - the midnight split
  - DST days (25 h / 23 h)
  - ambiguous and gap local times
  - half-open overlap, including adjacency and the running entry as `[start, +∞)`
  - future checks
  - removed entries ignored
  - `AlreadyRunning`/`NotRunning`
  - `RunningEndEdit`
  - `NotFound` for removed ids
  - an unchanged minute keeps the stored instant on edit
- Refusal atomicity: after each refused mutation, the table is byte-for-byte the same rows.
- Error serialization: the `kind`/`detail` shape, and no times in `Overlap`.

### Integration Tests:

- `tests/work_time.rs`:
  - START, drop the store, reopen → the running entry has the same start
  - add, stop, remove, reopen → the history and the removed row are intact
- `tests/work_time_crash.rs`: a child process loops START/STOP on the real store and is killed at commit counts and delays. After each reopen the invariants hold, and no reported commit is lost.

### Manual Testing Steps:

1. Run the Phase 3 durability steps: close window, Zakończ, Task Manager kill, Windows restart.
2. Run the Phase 4 correction steps, including the midnight entry and the overlap refusal.
3. Check that the Health view still reports "Dane aplikacji" as OK after the migration ran.

## Performance Considerations

At most a few thousand rows a year. The day query is a range scan on `work_entries_start`. The header's 1 s tick is a local re-render with no IPC, and IPC happens only on events, visibility changes and day change, so the "Lekkość w tle" NFR is unaffected.

## Migration Notes

- This is migration 1 on top of F-01. An existing `day-navigator.db` at `user_version 0` is upgraded on the next open, and the daily backup is taken before migrating (F-01 open order).
- Going back to an older app build after this ships is refused as `NewerSchema`, by design. The rollback path is the documented restore from `backups/` (AGENTS.md "App Data Store").
- No data is migrated from the old time records (`Management/Timeline.md`, Helper App), per the PRD.
- Coordination with S-01: S-01's plan should extend the `View` union and header from this slice rather than design its own shell. If S-01 lands first, this slice adopts its shell and keeps only the counter and the "Czas pracy" view.

## References

- Roadmap item: `context/foundation/roadmap.md` (S-08, lines 196–207)
- PRD: `context/foundation/prd.md` FR-013, FR-017, NFR "Trwałość czasu pracy", Guardrail "Czas pracy nie ginie"
- Store contract: `context/archive/2026-11-23-app-data-store/plan.md`, `src-tauri/src/store/mod.rs:141-259`
- Crash harness: `src-tauri/tests/common/crash.rs`, example `src-tauri/tests/store_crash.rs`
- Event pattern: `src-tauri/src/runtime.rs:205-224`, `src/health/useHealth.ts:21-54`
- Form pattern: `src/tasks/TasksView.tsx:82-170`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Work-time domain and store

#### Automated

- [x] 1.1 `cargo test` passes in `src-tauri/`, including the new unit tests and `tests/work_time.rs` / `tests/work_time_crash.rs` — 0094bd6
- [x] 1.2 Unit tests cover midnight split, DST days, ambiguous/gap times, adjacency, overlaps (closed and running), InFuture, EndNotAfterStart, RunningEndEdit, AlreadyRunning, NotRunning, removed entries ignored, refusal leaves table unchanged — 0094bd6
- [x] 1.3 A test proves the partial unique index rejects a second running row inserted directly — 0094bd6
- [x] 1.4 A test asserts that a serialized `WorkTimeError::Overlap` contains only the entry id — 0094bd6
- [x] 1.5 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass — 0094bd6
- [x] 1.6 `cargo audit` passes with the `chrono-tz` dev-dependency — 0094bd6
- [x] 1.7 Unit tests prove an unchanged minute keeps the stored instant on edit — 0094bd6

### Phase 2: Commands and event

#### Automated

- [x] 2.1 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass — 301c475
- [x] 2.2 Existing vault task tests still pass with the generic `blocking` — 301c475
- [x] 2.3 `npm run build`, `npm run lint` and `npm run format:check` pass — 301c475

#### Manual

- [ ] 2.4 `start_work` / `stop_work` from the devtools console create and close an entry, and the event fires

### Phase 3: App shell and header counter

#### Automated

- [x] 3.1 `npm run build`, `npm run lint`, `npm run format:check` pass
- [x] 3.2 `cargo test` still passes

#### Manual

- [ ] 3.3 Header stays visible while scrolling; view switch shows Zadania, Czas pracy, Stan źródeł
- [ ] 3.4 START ticks, STOP freezes, START again continues today's total
- [ ] 3.5 Running counter survives window close and "Zakończ" + relaunch, with downtime counted
- [ ] 3.6 Running counter survives a Task Manager kill
- [ ] 3.7 Running counter survives a Windows restart
- [ ] 3.8 After midnight in the tray, "Dziś" counts only time since midnight
- [ ] 3.9 A failing store shows the header error with retry, not 0:00:00
- [ ] 3.10 A stopped counter left open across midnight resets "Dziś" without interaction

### Phase 4: "Czas pracy" view

#### Automated

- [ ] 4.1 `npm run build`, `npm run lint`, `npm run format:check` pass
- [ ] 4.2 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass

#### Manual

- [ ] 4.3 Adding a missing 08:00–12:00 entry shows it with a 4:00 day total
- [ ] 4.4 An overlapping entry is refused with the other entry's times; an adjacent one is accepted
- [ ] 4.5 End before start and an end in the future are refused
- [ ] 4.6 Editing the running entry's start updates the header; no end field is offered
- [ ] 4.7 A 22:00–02:00 entry shows under both days with 2:00 counted on each
- [ ] 4.8 Removing an entry after confirmation drops it from the list and totals
- [ ] 4.9 An empty day shows "Brak wpisów w tym dniu"; a failing store shows ErrorState
