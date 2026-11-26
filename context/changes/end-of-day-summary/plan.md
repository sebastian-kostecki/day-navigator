# End-of-day summary after STOP Implementation Plan

## Overview

After STOP the user sees a summary of the day: time worked today, the month's deviation, what was ticked off in "Do zrobienia" and what carries over to tomorrow, with no decision forced (roadmap S-07, MS-19, GitHub issue #19). A STOP in the window header opens a dismissible summary panel in the window; a STOP from the tray menu shows a short Windows notification with today's time and the month's deviation only. The figures and the lists are computed in Rust from the existing sources (`work_time::status`, `todo::list`) and unit-tested there; nothing new is stored and there is no migration.

## Current State Analysis

- Both STOPs run one function, `work_time::commands::stop_timer` (`src-tauri/src/work_time/commands.rs:148-156`). The header calls the `stop_work` command (`commands.rs:166-170`, STOP then a fresh `WorkTimerStatus`); the tray calls `stop_timer` from `run_tray_action` (`src-tauri/src/runtime.rs:204-216`), the only tray caller, whose success branch is empty today (tray-timer-control A11: "S-07 adds the end-of-day notice later").
- `work_time::status(store, now, tz, settings)` (`src-tauri/src/work_time/mod.rs:179-214`) already gives `today_closed_ms`, `running_from_ms`, `as_of_ms` and `month_deviation_ms` (from `report::compute` → `norm::figures`). The daily norm lives only in `norm.rs` (`DAILY_NORM_MS`, `norm.rs:14`) and S-02 (configurable-daily-norm) is changing it in parallel, so the summary must pass the deviation through from `status`, never compute a norm.
- `todo::list(store, today, now, limit)` (`src-tauri/src/todo/mod.rs:267-290`) is one write: roll-over, then today's visible list. `ticked == true` rows are the ones done today (own task done, user tick or closed in source); `ticked == false` rows carry over to tomorrow automatically (`roll_over_in`, `mod.rs:119-137`). The `limit` argument does not change `items`.
- Today's event log (`events::list_today`) does not log unticks or "done" in "Moje zadania" and caps at 200 rows, so it is not a reliable source for ticks (research §3, §4).
- Toasts go through `runtime::toast(app, area, title, body)` (`runtime.rs:353-361`, gated by `health::notices_allowed()`), with fixed Polish texts from pure `Notice`-returning builders titled `"{APP_NAME} — …"` (`health/mod.rs:21,145-149`; precedent `tray_view::failure_notice`, `tray_view.rs:124-155`). Rust has only an unsigned `tray_view::format_hm` (`tray_view.rs:88-92`); TS `formatHm(ms, true)` (`src/components/time.ts:79-89`) is the signed rule (rounded toward zero, U+2212 minus, plain `0:00` under a minute).
- UI: one timer subscription in `useWorkTimerSource` shared through `WorkTimerProvider`. `stop()` resolves `void` in every case (`src/worktime/useWorkTimer.ts:156-179`), so the header cannot tell whether its STOP succeeded. The header's confirmation lives in `src/worktime/WorkTimerHeader.tsx:41-46,81-99`. `App.tsx:61-81` renders `WorkTimerProvider` → `AppHeader` (with `WorkTimerHeader`) → the active view. No dialog/overlay component exists and tokens.css has no z-index, shadow or scrim tokens; `Panel`, `StatStrip`, `ItemRow`, `Button`, `ErrorState` are the shared building blocks.
- PRD `context/foundation/prd.md:233-234` lists "zamknięcie dnia" among the things considered but not built; the same sentence names autostart (S-04) and the week view (S-08).

## Desired End State

- Header STOP (after "TAK, STOP") succeeds → a "Podsumowanie dnia" panel appears between the header and the active view, showing "Dziś" (H:MM), "Odchylenie miesiąca" (±H:MM, "do dziś"), the counts and two lists: "Zrobione dziś" and "Przechodzi na jutro", each with an explicit empty message. One "Zamknij" button dismisses it; nothing else is asked. It survives view switches, follows ticks made in "Do zrobienia" while it is open, disappears on the next START (header or tray), and a later STOP shows it again with fresh data. A failed read shows `ErrorState` with "Spróbuj ponownie" inside the panel, never an empty summary.
- Tray STOP succeeds → one toast "Nawigator dnia — koniec pracy" / "Dziś: 7:45. Odchylenie miesiąca: −0:15." (the figures follow the header); if the figures cannot be read, a fixed text without figures. No item titles or counts in the toast; no in-window panel on a tray STOP.
- `cargo test` covers the summary rules (done vs carry-over split, totals, pass-through deviation, midnight, errors) and the toast texts, including a signed formatter that matches TS `formatHm(ms, true)`.
- CHANGELOG, README and the PRD sentence describe the feature; `norm.rs`, `report.rs` and `store/schema.rs` are unchanged.

Verify with the Progress rows below: the automated gates per phase, then the manual checks in a running app and a built app.

### Key Discoveries:

- `stop_timer` is the single STOP path and its doc comment names this slice's hook points (`work_time/commands.rs:148-150`).
- `work_time::commands::work_time_settings(path, Policy::Lenient)` and `Policy` are `pub` (`commands.rs:23-53`), so a new module can read the holiday settings before taking the store lock, like `run_with_settings` does (`commands.rs:80-106`).
- `work_time::rules::local_day(ms, tz)` is `pub` (`work_time/rules.rs:33`); deriving one local day from `now` and passing it to `todo::list` keeps both halves of the summary on the same day.
- The live total is `today_closed_ms + (now − running_from_ms)` (`work_time/mod.rs:50`); after a STOP `running_from_ms` is `None`, but a START between STOP and the read must still give a correct figure.
- `act` in `useWorkTimer.ts` hands the status returned by `stop_work` to `apply` before it resolves (`useWorkTimer.ts:160-162`). `apply` may drop it as stale: `stop_timer` emits `work-time-changed` before `stop_work` reads the status, and the listener's `refresh` takes a higher request id, so its answer can land first (`useWorkTimer.ts:59-65,96-98`). Either way, once `call()` has resolved, the shared timer already shows the state after the STOP (a refresh issued after the commit or the STOP's own status). So "STOP committed" is "`call()` resolved", not "its status was applied".
- The dashboard's "Odchylenie" tile reads `work_month_report` (`Policy::Strict`, `src/todo/summary.ts:39-46`); `WorkTimerStatus.monthDeviationMs` comes from `report::compute` with `Policy::Lenient` and no view displays it today (`src/worktime/types.ts:52`). They agree unless the `workTime` block in `settings.json` is invalid (then the tile shows "—" and the summary uses the default calendar, like the header's START/STOP).
- The worktree has no `node_modules`; run `npm install` once before the first frontend gate.
- Unit-test helpers to copy: `open()` with `Store::open(..., schema::MIGRATIONS, day)` and `Europe/Warsaw` instants (`work_time/tests.rs:1-30`), own-task and GitLab item builders (`todo/tests.rs:1-60`).
- Value-free error mapping precedent: `own_task_default_space` maps a foreign error to `StoreError::Sql { reason }` (`lib.rs:110-124`).

## What We're NOT Doing

- No fixed-hour or scheduled summary; it appears only after STOP (user decision).
- No forced decision, no "carry over?" question, no change to the roll-over rule: unticked items keep carrying over automatically.
- No new store, table, migration, setting or file; no persistence of "summary seen/dismissed" (a reload simply clears it).
- No change to `stop_work`, `WorkTimerStatus`, `norm.rs`, `report.rs`, `today.rs` or the todo rules; no hard-coded 8 h anywhere.
- No use of the event log for ticks (unreliable, research §3); the event log is not read by this slice.
- No modal, overlay, scrim or new z-index/shadow token; no new shared component in `src/components/`.
- No reopen button in "Do zrobienia" or elsewhere; no click handling on the toast.
- No item titles, references or counts in the toast; no titles in the app log (logs carry error kinds only).
- No new FR in the PRD; no edit of `context/foundation/roadmap.md`.
- No AGENTS.md change: nothing new is stored and no new React context is added.

## Implementation Approach

New logic sits in new files so the shared files (`lib.rs`, `runtime.rs`, `useWorkTimer.ts`, `WorkTimerHeader.tsx`, `App.tsx`, docs, PRD) get a few lines each; this matters because S-02, S-04, S-05 and S-08 run in parallel.

- **Rust `src-tauri/src/day_summary/`**: `mod.rs` (types, pure `compose`, `today_ms`, `read` over a `Store`), `error.rs` (`DaySummaryError`), `notice.rs` (pure toast texts and the signed formatter), `commands.rs` (the only file importing `tauri`: the `day_summary` command and the tray helper), `tests.rs`.
- **Header path**: `stop()` resolves `true` on success → `WorkTimerHeader` calls a new `onStopped` prop → `App` mounts `DaySummaryPanel` (new folder `src/day-summary/`), which calls the `day_summary` command on mount and again on each `todo-changed` while open. That is a read, not a second timer subscription; the panel reads the shared timer only through `useWorkTimer()` to close itself on START.
- **Tray path**: `run_tray_action`'s success branch for STOP awaits `day_summary::commands::stop_notice(&app)` (a read of `work_time::status` on the blocking pool, lock released when it returns) and passes the returned `Notice` to `toast`.

Phases are ordered so each one is green on its own: Rust domain and command (with the TS mirror) → tray toast → window panel → docs and PRD.

Run every `git` check in the Success Criteria from the worktree root (pathspecs are relative to the current directory). New files are untracked until committed, so the grep checks use `git grep --untracked` and the "no change" checks use `git status --porcelain -- <paths>` (empty output = pass), which also sees new files; `git diff --quiet` alone does not.

## Critical Implementation Details

- **Timing & lifecycle**: the tray toast is sent only after the blocking read has returned, so the store lock is never held across `toast` (timer-reminders rule). The panel opens only after `stop()` resolved `true`, and by then the shared timer shows the post-STOP state (its own status or a newer refresh, Key Discoveries); so the panel's "close when running" rule cannot close it at mount.
- **State sequencing**: `day_summary::read` takes `status` first and then `todo::list` inside one `with_store` call, with the same local day for both; `todo::list` is a write (roll-over), so it must stay inside `with_store` on the blocking pool, never in a sync command.

## Phase 1: Day summary in the domain and the command

### Overview

Add the `day_summary` Rust module with its types, the pure rules, the store read, the error enum and the `day_summary` command, plus the TS mirror and typed `invoke` wrapper. No UI and no tray change yet.

### Changes Required:

#### 1. Summary types and rules

**File**: `src-tauri/src/day_summary/mod.rs` (new)

**Intent**: Turn the existing header status and today's "Do zrobienia" list into one summary of the day, with no rule of its own beyond the split and the live total, so S-02's norm change flows through unchanged.

**Contract**:

- Module doc: pure, no `tauri` import, `now` and the time zone are parameters; data comes from `work_time::status` and `todo::list` only.
- `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] #[serde(rename_all = "camelCase")] pub struct DaySummary { pub day: String /* YYYY-MM-DD, the list's day */, pub today_ms: i64, pub month_deviation_ms: i64, pub done: Vec<TodoItem>, pub carry_over: Vec<TodoItem> }`. `TodoItem` needs `PartialEq`/`Eq` (it has them).
- `pub fn today_ms(status: &WorkTimerStatus) -> i64` = `today_closed_ms + running_from_ms.map_or(0, |from| (as_of_ms - from).max(0))` (the header's live total at `as_of_ms`).
- `pub fn compose(status: &WorkTimerStatus, list: TodoList) -> DaySummary`: `day = list.day`, `today_ms = today_ms(status)`, `month_deviation_ms = status.month_deviation_ms` (pass-through), `done` = items with `ticked`, `carry_over` = items without, each keeping the order `todo::list` returned.
- `pub fn read<Tz: TimeZone>(store: &mut Store, now_ms: i64, tz: &Tz, settings: &WorkTimeSettings) -> Result<DaySummary, DaySummaryError>`: `let local = tz.timestamp_millis_opt(now_ms).earliest().ok_or(DaySummaryError::InvalidLocalTime)?.naive_local();` (the same resolution as `work_time::rules::local_day`, `rules.rs:33-39`); `status = work_time::status(store, now_ms, tz, settings)?` (`status` takes `&Store`; `&mut Store` coerces); `list = todo::list(store, local.date(), local, Ok(todo::DEFAULT_TODO_LIMIT))?` (the limit only fills `TodoList.limit`, which the summary drops); `Ok(compose(&status, list))`.
- Declares `pub mod commands; pub mod error;` and `#[cfg(test)] mod tests;` (Phase 2 adds `pub mod notice;`); re-exports `DaySummaryError`.

#### 2. Error enum

**File**: `src-tauri/src/day_summary/error.rs` (new)

**Intent**: A value-free tagged error for the command, per the AGENTS.md contract, that both source errors map into.

**Contract**:

- `#[derive(Debug, Clone, thiserror::Error, Serialize)] #[serde(tag = "kind", content = "detail")] pub enum DaySummaryError { Store(StoreError), InvalidLocalTime, Background }` (no struct variants, so no `rename_all_fields`), each variant with an `#[error("…")]` message in the style of `work_time/error.rs` (e.g. `"app data store error: {0}"`, `"local time is invalid or does not exist"`, `"background task failed"`), and `kind()` for logs (the store's own kind for `Store`, as `WorkTimeError::kind` does).
- `From<StoreError>`, `From<crate::BackgroundFailed>` (→ `Background`).
- `From<WorkTimeError>`: `Store(e)` → `Store(e)`, `InvalidLocalTime` → `InvalidLocalTime`, `Background` → `Background`, anything else → `Store(StoreError::Sql { reason: "work-time state unreadable".into() })`.
- `From<TodoError>`: `Store(e)` → `Store(e)`, `Background` → `Background`, anything else → `Store(StoreError::Sql { reason: "todo state unreadable".into() })`.
- No variant carries a title, reference, time or path.

#### 3. Command

**File**: `src-tauri/src/day_summary/commands.rs` (new)

**Intent**: The one-shot read the window calls after its own STOP; the only file in the module that imports `tauri`.

**Contract**:

- `#[tauri::command] pub async fn day_summary(app: AppHandle) -> Result<DaySummary, DaySummaryError>`: inside `crate::blocking`, read settings with `work_time::commands::work_time_settings(settings_file::settings_path(&app).as_deref(), Policy::Lenient)` before the lock, then `app.state::<StoreHandle>().with_store(today, |store| super::read(store, Utc::now().timestamp_millis(), &Local, &settings))`, where `today = Local::now().date_naive()` as in `work_time::commands::execute`.
- No emit (a read; like `list_todo`, the roll-over inside `todo::list` emits nothing). On error: `log::warn!("day_summary: read failed: {}", e.kind())`; never a title or value in the log.

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the module and the command.

**Contract**: `pub mod day_summary;` in the module list between `pub mod cli;` and `pub mod events;`; `day_summary::commands::day_summary,` in `generate_handler!` right after `own_task_default_space,` (not at the end: the last line `work_reminders::reminder_settings` has no trailing comma, so every slice that appends there conflicts). One line each, nothing else.

#### 4. Unit tests

**File**: `src-tauri/src/day_summary/tests.rs` (new)

**Intent**: Pin the split, the totals and the pass-through on invented data, without asserting any norm number (so S-02 cannot break them).

**Contract** (store helpers copied from `work_time/tests.rs` / `todo/tests.rs`, `Europe/Warsaw`, invented titles such as "Pozycja testowa 1"):

- Own task done today and a GitLab entry ticked today land in `done`; open entries (including one picked yesterday, `carried_over == true`) land in `carry_over`; the order inside each list equals `todo::list`'s order.
- An entry ticked yesterday is in neither list (rolled over).
- After START 08:00 / STOP 15:30 on a workday, `today_ms` is 7 h 30 min and equals `status.today_closed_ms`.
- With a running entry, `today_ms == today_closed_ms + (as_of_ms − running_from_ms)`.
- `month_deviation_ms` equals `work_time::status(...)` on the same store, now and settings (compare the two calls, no literal norm).
- An empty day gives `today_ms == 0` and two empty lists, not an error.
- STOP after midnight: an entry 22:00 → 00:30 read at 00:31 gives `day` = the new date and `today_ms` = 30 min.
- `compose` on hand-built values; `DaySummaryError` serialization (`{"kind":"Background"}`, `{"kind":"Store","detail":{"kind":"Busy"}}`) and the `From` mappings.

#### 5. TS mirror and invoke wrapper

**File**: `src/day-summary/types.ts` (new), `src/day-summary/api.ts` (new)

**Intent**: The TS contract in the same commit as the Rust type (AGENTS.md); `api.ts` is the folder's only `invoke` importer.

**Contract**:

- `types.ts`: `export interface DaySummary { day: string; todayMs: number; monthDeviationMs: number; done: TodoItem[]; carryOver: TodoItem[] }` (import `TodoItem` from `../todo/types`); `export type DaySummaryError = { kind: "Store"; detail: StoreError } | { kind: "InvalidLocalTime" } | { kind: "Background" }` (`StoreError` from `../components/storeError`).
- `api.ts`: `export function daySummary(): Promise<DaySummary>` → `invoke("day_summary")`; `export function daySummaryErrorMessage(e: unknown): string`: `Store` → `storeErrorMessage(detail)`, `InvalidLocalTime` → "Zegar systemowy wskazuje czas, którego nie ma w strefie lokalnej — spróbuj za chwilę.", `Background` → "Nie udało się wczytać podsumowania — spróbuj ponownie.", anything else → "Nieoczekiwany błąd: …" as in `todoErrorMessage`.

### Success Criteria:

#### Automated Verification:

- Rust tests pass, including the new `day_summary::tests`: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Only `commands.rs` in the module imports `tauri`: `git grep --untracked -n -E "use tauri|tauri::" -- src-tauri/src/day_summary` lists lines in `src-tauri/src/day_summary/commands.rs` and nowhere else (and does list that file, so the new files were searched)
- No norm in the module: `git grep --untracked -n -E "DAILY_NORM_MS|3_600_000" -- src-tauri/src/day_summary ':!src-tauri/src/day_summary/tests.rs'` returns nothing
- Norm, report and schema untouched: `git diff --quiet HEAD -- src-tauri/src/work_time/norm.rs src-tauri/src/work_time/report.rs src-tauri/src/store/schema.rs`

**Implementation Note**: After all automated verification passes, commit (`feat(end-of-day-summary): day summary in the domain and command`). No manual check in this phase.

---

## Phase 2: Tray STOP notification

### Overview

Add the pure toast texts with a signed H:MM formatter and send the toast from the tray's STOP success branch.

### Changes Required:

#### 1. Toast texts

**File**: `src-tauri/src/day_summary/notice.rs` (new; declare `pub mod notice;` in `day_summary/mod.rs`)

**Intent**: Fixed Polish wording built in Rust and unit-tested, with the same signed rule as the window, and no item content.

**Contract**:

- `pub fn format_signed_hm(ms: i64) -> String`: whole minutes rounded toward zero (`ms / 60_000`); `0:00` when that is 0; otherwise `+H:MM` or `−H:MM` with U+2212. Mirrors TS `formatHm(ms, true)` (`src/components/time.ts:79-89`).
- `pub fn stop_notice(today_ms: i64, month_deviation_ms: i64) -> Notice`: title `format!("{APP_NAME} — koniec pracy")`; body `format!("Dziś: {}. Odchylenie miesiąca: {}.", tray_view::format_hm(today_ms), format_signed_hm(month_deviation_ms))`.
- `pub fn stop_notice_unavailable() -> Notice`: same title; body "Licznik zatrzymany. Czas pracy chwilowo niedostępny — otwórz „Czas pracy”."
- No `tauri` import. Tests (in this file or `tests.rs`): `format_signed_hm` for 0, ±59 999 ms (`0:00`), 60 000 (`+0:01`), −90 min (`−1:30`), 125 h (`+125:00`); exact title and both bodies; a proptest over any `today_ms` and `month_deviation_ms` (`i64` ranges) that the body is exactly `"Dziś: " + A + ". Odchylenie miesiąca: " + B + "."`, where `A` is digits, `:`, two digits and `B` is `0:00` or `+`/`−` followed by digits, `:`, two digits (so no title, reference or count can appear). Check the shape by hand (`strip_prefix`/`strip_suffix`, `split_once(':')`, `chars().all(|c| c.is_ascii_digit())`): `regex` is not a dependency of the crate and this slice adds no dependency.

#### 2. Tray helper

**File**: `src-tauri/src/day_summary/commands.rs`

**Intent**: Read only today's figures for the tray (no todo write) and choose the notice, so `runtime.rs` gets two lines.

**Contract**: `pub(crate) async fn stop_notice(app: &AppHandle) -> Notice`: in `crate::blocking`, settings via `work_time_settings(..., Policy::Lenient)` then `with_store(today, |s| work_time::status(s, Utc::now().timestamp_millis(), &Local, &settings))`; `Ok(status)` → `notice::stop_notice(super::today_ms(&status), status.month_deviation_ms)`; `Err(e)` → `log::warn!("day_summary: stop figures failed: {}", e.kind())` and `notice::stop_notice_unavailable()`. Never a wrong figure, never a silent skip.

#### 3. Hook in the tray caller

**File**: `src-tauri/src/runtime.rs`

**Intent**: A successful tray STOP shows the end-of-day toast; a tray START and every failure behave as today.

**Contract**: in `run_tray_action`, on `Ok(())` with `action == TimerAction::Stop`: `let n = crate::day_summary::commands::stop_notice(&app).await; toast(&app, "day_summary", &n.title, &n.body);`. The `Err` branch is unchanged. Update the function's doc comment (one line). No other change in `runtime.rs`.

### Success Criteria:

#### Automated Verification:

- Rust tests pass, including the notice tests: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- `notice.rs` imports nothing from `tauri`: `git grep --untracked -n -E "use tauri|tauri::" -- src-tauri/src/day_summary/notice.rs` returns nothing
- No frontend, capability or config change: `git status --porcelain -- src src-tauri/capabilities src-tauri/tauri.conf.json src-tauri/Cargo.toml` prints nothing

#### Manual Verification:

- Tray STOP while the counter runs shows one toast titled "Nawigator dnia — koniec pracy" whose two figures match the header's "Dziś" and the dashboard's "Odchylenie" tile
- Tray START shows no toast; header STOP shows no toast

**Implementation Note**: After all automated verification passes, commit (`feat(end-of-day-summary): tray STOP notification`) and leave the manual rows for the human.

---

## Phase 3: Summary panel after header STOP

### Overview

Let the header learn that its STOP succeeded, and show a dismissible summary panel in the window built from shared components.

### Changes Required:

#### 1. STOP result in the shared timer

**File**: `src/worktime/useWorkTimer.ts`

**Intent**: The header needs to know that its own STOP committed, without a second subscription.

**Contract**: `act` resolves `true` when `call()` resolved (the action committed and its status went to `apply`), `false` otherwise (the `AlreadyRunning`/`NotRunning` refresh path and every error). Do not make the result depend on whether `apply` kept the status: the `work-time-changed` refresh can win the race and drop it as stale (Key Discoveries), and the STOP still committed. `WorkTimer.start` and `WorkTimer.stop` become `() => Promise<boolean>`. Nothing else changes; the only caller is `WorkTimerHeader.confirm` (`TodoView` and `WorkTimeView` read only `timer` and `nowMs`).

#### 2. Header callback

**File**: `src/worktime/WorkTimerHeader.tsx`

**Intent**: Tell the app that the header's STOP succeeded; a tray STOP never calls it (it does not go through `stop()`).

**Contract**: optional prop `onStopped?: () => void`; in `confirm`, the stop branch becomes `void stop().then((ok) => { if (ok) onStopped?.(); })`; the start branch is unchanged.

#### 3. Summary state hook

**File**: `src/day-summary/useDaySummary.ts` (new)

**Intent**: Loading/error/data state for one summary read, per the feature-folder convention.

**Contract**: `useDaySummary(): { state: { state: "loading" } | { state: "error"; message: string; lastOkAt: number | null } | { state: "loaded"; summary: DaySummary }; retry: () => void }`; messages from `daySummaryErrorMessage`. Loads on mount, again on `retry`, and again on every `todo-changed` event, so a tick or untick in "Do zrobienia" (right below the panel) after STOP updates both lists. Follow `useTodo` (`src/todo/useTodo.ts:65-117`): subscribe to `todo-changed` with `listen` from `@tauri-apps/api/event` first and load only once the listener is registered; latest request wins (request counter); unlisten on unmount. A failed reload replaces the data with the error state carrying `lastOkAt` (time of the last successful read), never stale lists without a warning. This is not a timer subscription; `work-time-changed` is not listened to here. Set `loading` in `retry` (event handler), not synchronously in the effect body (`react-hooks/set-state-in-effect` from eslint-plugin-react-hooks 7). Known residue: "done" in "Moje zadania" emits no `todo-changed` (`lib.rs:92-107`), so the panel catches it only on the next tick, retry or STOP.

#### 4. Summary panel

**File**: `src/day-summary/DaySummaryPanel.tsx` (new), `src/day-summary/DaySummaryPanel.css` (new)

**Intent**: A calm, dismissible in-window summary that asks for nothing, built only from shared components and tokens.

**Contract**:

- Props `{ onClose: () => void }`. Wrapper `<div className="container dn-day-summary">` holding a `Panel` titled "Podsumowanie dnia", `meta` = the summary day as `DD.MM` (sliced from `YYYY-MM-DD`, no `Date` parsing), footer = hint "Nieodhaczone pozycje przejdą na jutro same." (only when `carryOver` is non-empty) and a `Button` "Zamknij" calling `onClose`.
- Loading: "Wczytywanie podsumowania…". Error: `ErrorState` title "Podsumowanie dnia niedostępne", the message, `onRetry = retry`, `lastSuccess` = `formatClock(lastOkAt)` when a read succeeded earlier (a failed reload after `todo-changed`), omitted otherwise. The close button stays available in every state.
- Data: a `<div role="status">` around a `StatStrip` with "Dziś" (`formatHm(todayMs)`), "Odchylenie miesiąca" (`formatHm(monthDeviationMs, true)`, sub "do dziś", tone `warn` when negative else `info`, as `src/todo/summary.ts`), "Zrobione" (count of `done`), "Na jutro" (count of `carryOver`); then two lists of read-only `ItemRow`s (no checkbox) headed "Zrobione dziś" and "Przechodzi na jutro", with the same source chip and reference as TodoView's rows (`source` `my`/`gl`/`gh`; `shortRef` from `src/todo/shortRef.ts`; own entries show "praca"/"prywatne" from a local `Record<Space, string>` in the panel, because `SPACE_REF` is private to `TodoView.tsx:38` and this slice does not edit TodoView), plain-text titles, the "zamknięte w GitLabie/GitHubie" flag on done rows closed in source; empty messages "Dziś nic nie odhaczono w „Do zrobienia”." and "Nic nie przechodzi na jutro."
- Reads `useWorkTimer()` (no new subscription): when the shared timer is loaded and running, an effect calls `onClose`, so a later START (header or tray) removes a stale summary. (Calling the `onClose` prop from the effect is fine; the panel must not instead just render `null` while running, or a tray STOP would bring the old summary back.)
- CSS: layout only, `dn-` prefix, tokens only (no literal colours, no z-index, no shadow); it must not double the gap to the view below.

#### 5. Mount in the app

**File**: `src/App.tsx`

**Intent**: One piece of app state that opens a fresh summary on each header STOP and closes it on dismiss.

**Contract**: `const [summaryRun, setSummaryRun] = useState(0)`; `<WorkTimerHeader onStopped={() => setSummaryRun((n) => n + 1)} />`; between `</AppHeader>` and `<GitlabListViewProvider>`: `{summaryRun > 0 && <DaySummaryPanel key={summaryRun} onClose={() => setSummaryRun(0)} />}`. No router, no new context.

### Success Criteria:

#### Automated Verification:

- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Rust gates still pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- One timer subscription: `git grep --untracked -n -E "workTimerStatus|WORK_TIME_CHANGED|work-time-changed|stopWork|startWork" -- src/day-summary src/App.tsx` returns nothing, and `git grep --untracked -n "workTimerStatus()" -- src` lists only `src/worktime/useWorkTimer.ts` (today `listen(WORK_TIME_CHANGED` also sits in `useMonthReport.ts` and `useWorkEntries.ts`, which reload their own data, so a repo-wide `listen` grep is not the check)
- `invoke` stays in `api.ts` files: `git grep --untracked -n "@tauri-apps/api/core" -- src/day-summary` lists only `src/day-summary/api.ts`
- No literal colours in the new CSS: `git grep --untracked -n -E "#[0-9a-fA-F]{3,8}\b|rgba?\(" -- src/day-summary` returns nothing
- No Rust, capability or config change: `git status --porcelain -- src-tauri` prints nothing

#### Manual Verification:

- Header STOP after "TAK, STOP" shows "Podsumowanie dnia" between the header and the view; "Dziś" equals the header clock (H:MM) and "Odchylenie miesiąca" equals the dashboard tile
- "Zrobione dziś" lists exactly the ticked rows of "Do zrobienia" and "Przechodzi na jutro" exactly the unticked ones, with explicit empty messages when a list is empty
- "Zamknij" removes the panel with no other prompt; switching views keeps an open panel; a header or tray START removes it; a later STOP shows it again
- A tray STOP shows no panel in the window (only the toast)
- With the panel open on "Do zrobienia", ticking an unticked row moves it from "Przechodzi na jutro" to "Zrobione dziś" and the counts follow
- In a built app (`npm run tauri build -- --debug --no-bundle`) the panel works and the DevTools console shows no CSP violation and no "IPC custom protocol failed"

**Implementation Note**: After all automated verification passes, commit (`feat(end-of-day-summary): summary panel after header STOP`) and leave the manual rows for the human.

---

## Phase 4: Docs and PRD wording

### Overview

Record the feature for users and the next release, and correct the PRD sentence that lists "zamknięcie dnia" as not built.

### Changes Required:

#### 1. Changelog

**File**: `CHANGELOG.md`

**Intent**: Release notes for the next version.

**Contract**: under `## [Unreleased]` → `### Added`, one entry: after STOP in the header, a dismissible "Podsumowanie dnia" panel shows today's work time, the month's deviation, what was ticked off in "Do zrobienia" and what carries over to tomorrow (no question asked); after STOP from the tray, a Windows notification shows today's work time and the month's deviation.

#### 2. README

**File**: `README.md`

**Intent**: Mention the summary under "Czas pracy (Work time)".

**Contract**: one sub-bullet next to the tray START/STOP bullet (`README.md:58`), e.g. "After STOP: a day summary in the window (header STOP) or a short notification (tray STOP)."

#### 3. PRD

**File**: `context/foundation/prd.md`

**Intent**: The PRD must no longer claim "zamknięcie dnia" is unbuilt; anchor it as MS-19 without a new FR (precedent: S-01 and MS-25).

**Contract**: in the paragraph under "### Rozważone i świadomie NIE wykluczone" (`prd.md:234`) delete only the words "zamknięcie dnia, "; add one new line after that paragraph: "Zamknięcie dnia jest zbudowane jako podsumowanie dnia po STOP (MS-19), bez nowego FR." Touch no other words (S-04 and S-08 edit the same sentence; a merge conflict there is expected and resolved by keeping every slice's own removal).

### Success Criteria:

#### Automated Verification:

- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Rust gates still pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- Only docs change: `git status --porcelain` lists only `CHANGELOG.md`, `README.md`, `context/foundation/prd.md` (and this plan's Progress)
- The PRD no longer lists it as unbuilt: `git grep -n "skrzynka, zamknięcie dnia" -- context/foundation/prd.md` returns nothing and `git grep -n "MS-19" -- context/foundation/prd.md` finds the new line
- Across the slice, norm, report and schema are untouched: `git diff --quiet $(git merge-base HEAD develop) -- src-tauri/src/work_time/norm.rs src-tauri/src/work_time/report.rs src-tauri/src/store/schema.rs`

**Implementation Note**: After all automated verification passes, commit (`docs(end-of-day-summary): changelog, readme and PRD wording`).

---

## Testing Strategy

### Unit Tests:

- `day_summary::tests`: done vs carry-over split and order, roll-over of yesterday's ticks, live total with and without a running entry, deviation equal to `work_time::status`, empty day, STOP after midnight, error serialization and mappings.
- `day_summary::notice` tests: signed formatter cases matching TS `formatHm(ms, true)`, exact titles and bodies, a proptest that the body holds only the two figures.
- Existing `work_time`, `todo`, `tray_view` and `health` tests stay green; the tray STOP path itself is unchanged up to the success branch.

### Integration Tests:

- None new: the toast and the window panel need a running Tauri app and the repo has no Tauri or UI test harness. The grep checks guard the single timer subscription, the `invoke` boundary, the `tauri`-free pure files and the untouched norm/report/schema.

### Manual Testing Steps:

1. Start the counter in the header, tick one item in "Do zrobienia", leave another unticked; STOP in the header and confirm: the panel shows the times and both lists.
2. With the panel open, tick the open item in "Do zrobienia": it moves to "Zrobione dziś" in the panel; untick it again. Click "Zamknij": the panel disappears with nothing asked; the unticked item is still on the list.
3. START from the tray: no toast; STOP from the tray: the toast shows "Dziś" and "Odchylenie miesiąca" matching the header and the dashboard; no panel appears.
4. STOP in the header again, switch views: the panel stays; START from the header: the panel disappears.
5. Build with `npm run tauri build -- --debug --no-bundle`, repeat step 1 and check the DevTools console for CSP or IPC errors.

## Performance Considerations

One extra read per STOP: the header's panel runs `status` (month report) and `todo::list` (one small write) once, and once more per `todo-changed` while it is open (no loop: `day_summary` emits nothing); the tray runs one `status`. Both are the same queries the header and the dashboard already run on every refresh, so the cost is negligible.

## Migration Notes

None: no schema change, no new setting, no new stored data. Rollback is reverting the four commits.

## Decisions

User decisions (binding, given with the task):

- **D1.** The summary appears after STOP only; no fixed-hour trigger.
- **D2.** Header STOP: a dismissible summary in the window; closing it needs nothing else; no forced decision; unticked items keep carrying over automatically.
- **D3.** Tray STOP: a short Windows notification with today's work time and the month's deviation, fixed wording, no item titles, through `runtime::toast`, `health::notices_allowed()` and a pure `Notice` builder titled "Nawigator dnia — …".
- **D4.** Data from existing sources, computed in Rust and unit-tested; no second store of the day's state, no new migration.
- **D5.** Anchored as MS-19 with no new FR; the PRD's "zamknięcie dnia" fragment is reworded minimally.

Agent defaults (no user was reachable; default chosen by agent, user may revise):

- **A1.** In-window form: a `Panel` "Podsumowanie dnia" mounted in `App.tsx` between the header and the active view, figures in a `role="status"` region, "Zamknij" in the footer; no modal, overlay or new tokens.
- **A2.** No persistence: the panel lives in app state until "Zamknij", a reload, or a START (header or tray); a later STOP shows it again with fresh data; no reopen button. While open it re-reads on `todo-changed`, so a tick in "Do zrobienia" below it after STOP is reflected (plan review F3).
- **A3.** The header learns of its own successful STOP through `stop()` resolving `true` (= `stop_work` resolved, whether or not `apply` kept its status); a STOP that ends in `AlreadyRunning`/`NotRunning` or an error opens nothing. Pre-existing edge kept as is: when the STOP commits but `stop_work`'s follow-up status read fails, the header shows its action error and no panel opens.
- **A4.** A new one-shot command `day_summary` in a new module; `stop_work` and `WorkTimerStatus` stay unchanged (smaller diff against S-02/S-08).
- **A5.** Item titles appear in the window panel (read-only rows, same chips and references as "Do zrobienia"); never in the toast or the app log (logs carry error kinds only).
- **A6.** Tray notice texts: title "Nawigator dnia — koniec pracy"; body "Dziś: H:MM. Odchylenie miesiąca: ±H:MM."; if the figures cannot be read after a committed STOP, the body is "Licznik zatrzymany. Czas pracy chwilowo niedostępny — otwórz „Czas pracy”." (never a wrong figure, never a silent skip).
- **A7.** The tray notice reads only `work_time::status` (no todo write); the window summary reads `status` and `todo::list` in one `with_store` call on the same local day.
- **A8.** Deviation = `WorkTimerStatus.monthDeviationMs` (the header's status; no view displays it today), equal to the dashboard's "Odchylenie" tile: the current month's deviation including today's full norm, labelled "do dziś"; passed through from `status`, never computed. With an invalid `workTime` block the tile shows "—" while the summary uses the default calendar (`Policy::Lenient`, as START/STOP do).
- **A9.** STOP after midnight: the summary is about the new day (only the part after midnight; yesterday's ticks have rolled over). Accepted and documented, like the header.
- **A10.** The event log is not read: the todo store is the reliable source for ticks (research §3).
- **A11.** Signed H:MM in Rust lives in `day_summary/notice.rs` and mirrors TS `formatHm(ms, true)`; today's time reuses `tray_view::format_hm`.
- **A12.** No new shared component, so no design-system card is required; adding a "Podsumowanie dnia" pattern to the Konsola design-system artifact is an optional follow-up for the user.
- **A13.** The roadmap item is not moved to `planning` by this run (the task forbids editing `context/foundation/roadmap.md`).
- **A14.** README and CHANGELOG change in Phase 4; the PRD edit deletes only "zamknięcie dnia, " and adds one MS-19 line.

## References

- Research: `context/changes/end-of-day-summary/research.md`
- STOP path and commands: `src-tauri/src/work_time/commands.rs:23-106`, `:148-170`
- Tray caller and toast: `src-tauri/src/runtime.rs:202-216`, `:353-361`
- Status and totals: `src-tauri/src/work_time/mod.rs:43-64`, `:179-214`; `src-tauri/src/work_time/rules.rs:33`
- Todo list and roll-over: `src-tauri/src/todo/mod.rs:39-89`, `:119-137`, `:246-290`; `src-tauri/src/todo/commands.rs:68-120`
- Errors: `src-tauri/src/work_time/error.rs`, `src-tauri/src/todo/error.rs`, `src-tauri/src/lib.rs:110-124`
- Notices: `src-tauri/src/health/mod.rs:21,145-149,381-386`; `src-tauri/src/tray_view.rs:88-92,124-155`; `src-tauri/src/new_assignments/notice.rs`
- UI: `src/worktime/useWorkTimer.ts:156-184`, `src/worktime/WorkTimerHeader.tsx:15-46`, `src/App.tsx:60-82`, `src/components/time.ts:79-89`, `src/todo/summary.ts:23-50`, `src/todo/TodoView.tsx:276-310`
- PRD: `context/foundation/prd.md:233-234`; roadmap S-07 `context/foundation/roadmap.md:175-186`
- Prior changes: `context/archive/2026-11-26-tray-timer-control/plan.md` (A11, A12), `context/archive/2026-11-26-public-holidays-norm/plan.md` (PRD precedent), `context/archive/2026-11-23-timer-reminders/plan.md` (toast rules)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Day summary in the domain and the command

#### Automated

- [ ] 1.1 Rust tests pass, including the new `day_summary::tests`: `cargo test` (in `src-tauri/`)
- [ ] 1.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 1.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 1.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 1.5 Only `commands.rs` in the module imports `tauri`
- [ ] 1.6 No norm in the module (grep for `DAILY_NORM_MS|3_600_000` outside tests returns nothing)
- [ ] 1.7 Norm, report and schema untouched: `git diff --quiet HEAD -- src-tauri/src/work_time/norm.rs src-tauri/src/work_time/report.rs src-tauri/src/store/schema.rs`

### Phase 2: Tray STOP notification

#### Automated

- [ ] 2.1 Rust tests pass, including the notice tests: `cargo test` (in `src-tauri/`)
- [ ] 2.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 2.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 2.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 2.5 `notice.rs` imports nothing from `tauri`
- [ ] 2.6 No frontend, capability, config or dependency change (`git status --porcelain` on those paths prints nothing)

#### Manual

- [ ] 2.7 Tray STOP shows one "Nawigator dnia — koniec pracy" toast whose figures match the header and the dashboard
- [ ] 2.8 Tray START and header STOP show no toast

### Phase 3: Summary panel after header STOP

#### Automated

- [ ] 3.1 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 3.2 Rust gates still pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- [ ] 3.3 One timer subscription (no timer-status read or `work-time-changed` listener in `src/day-summary` or `App.tsx`; `workTimerStatus()` called only in `useWorkTimer.ts`)
- [ ] 3.4 `invoke` stays in `src/day-summary/api.ts`
- [ ] 3.5 No literal colours in `src/day-summary`
- [ ] 3.6 No Rust, capability or config change: `git diff --quiet HEAD -- src-tauri`

#### Manual

- [ ] 3.7 Header STOP shows "Podsumowanie dnia" with "Dziś" and "Odchylenie miesiąca" matching the header and the dashboard
- [ ] 3.8 "Zrobione dziś" and "Przechodzi na jutro" match the ticked and unticked rows of "Do zrobienia", with explicit empty messages
- [ ] 3.9 "Zamknij" dismisses with no prompt; view switches keep it; START removes it; a later STOP shows it again
- [ ] 3.10 A tray STOP shows no panel in the window
- [ ] 3.11 Built app: panel works, no CSP violation and no "IPC custom protocol failed" in DevTools
- [ ] 3.12 A tick in "Do zrobienia" while the panel is open moves the row between the panel's lists

### Phase 4: Docs and PRD wording

#### Automated

- [ ] 4.1 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 4.2 Rust gates still pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- [ ] 4.3 Only docs change (`CHANGELOG.md`, `README.md`, `context/foundation/prd.md`, plan Progress)
- [ ] 4.4 The PRD no longer lists "zamknięcie dnia" as unbuilt and names MS-19
- [ ] 4.5 Across the slice, norm, report and schema are untouched (diff against the merge base with `develop`)
