# START/STOP from the tray and today's work time in the tray tooltip Implementation Plan

## Overview

Roadmap S-03 (M-3), issue #23, PRD MS-23 / FR-013. The user can start and stop the work-time counter from a "START"/"STOP" item in the tray menu, and the tray tooltip shows today's work time under the source state, within the Windows limit of 127 UTF-16 units. The tray uses the same counter state as the header (the running row in `work_entries`), writes the same `workStarted` / `workStopped` event-log rows as the header, and the header follows a tray change through its one existing `work-time-changed` subscription. The tray icon gets a "counter running" variant; a source error keeps priority.

## Current State Analysis

- The counter state lives only in Rust: a running entry is a `work_entries` row with `end_ms IS NULL` (at most one, unique partial index). `work_time::start` / `stop` (`src-tauri/src/work_time/mod.rs:257-294`) write the entry and the event-log row in one `store.write`.
- The commands `start_work` / `stop_work` (`src-tauri/src/work_time/commands.rs:68-87`) run the domain write on the blocking pool (`run`, `:19-43`), then emit `work-time-changed` (`notify`, `:45-49`) and `events-changed`, then read the status. This logic is inlined in the two `#[tauri::command]` functions; there is no shared function a tray handler could call.
- The UI already follows changes made elsewhere: `useWorkTimerSource` (`src/worktime/useWorkTimer.ts:85-122`) is the single `listen(WORK_TIME_CHANGED)` behind `WorkTimerProvider`; `useEventLog` listens to `events-changed`. The header drops a stale "Zakończyć pracę?" question when the counter changes elsewhere (`src/worktime/WorkTimerHeader.tsx:21-24`). So no frontend change is needed if the tray path emits the same two events.
- The tray is static except for the health snapshot. The menu (`open`, `check`, `quit`) is built once in `runtime::build_tray` (`src-tauri/src/runtime.rs:149-185`); no item handle is kept. Menu callbacks run on the main thread.
- The tray has one writer: `update_tray` (`runtime.rs:206-216`), called only from `publish` (`:228-237`) under the `published` mutex by the health check thread (every 30 s or on `HealthState.wake`) and the watchdog. Every check round bumps `seq`, so a round always publishes.
- The icon is chosen from `Overall` alone (`tray_image`, `runtime.rs:139-145`; `ok`, `error`, `idle` PNGs compiled in with `tauri::include_image!`). Sources are SVGs in `src-tauri/icons/source/`; the recipe is in `src-tauri/icons/source/app-icon.svg:5-7`. Verified in this planning run: `npx tauri icon src-tauri/icons/source/tray-ok.svg -o <tmp> --png 32` (from the repo root) produces `<tmp>/32x32.png`, byte-identical to `src-tauri/icons/tray/ok.png` (32x32, RGBA).
- The tooltip is the pure `health::tooltip` (`src-tauri/src/health/mod.rs:257-289`), capped by the private `cap_utf16` (`:291-307`) to `TOOLTIP_MAX_UTF16 = 127` (`:17-19`), with exact-string tests and a proptest (`:688-738`).
- Nothing in Rust returns today's live total. `status()` (`work_time/mod.rs:166-202`) returns closed time plus `running_from_ms` and also runs the month report; the UI adds the running span capped at the day end and clamped at zero (`src/worktime/total.ts:8-18`). `rules::portion` (`src-tauri/src/work_time/rules.rs:57-63`) already does that per entry. `reminder_facts` (`work_time/reminders.rs:51-73`) is the pattern for a small background read.
- Threads: store access must run off the main thread (AGENTS.md; the health round holds the store lock during reopen and backup). Tray and menu setters block until the main thread runs them, so no lock may be held across them except `published`, which the main thread never takes (`runtime.rs:5-9`).
- No lessons file exists (`context/foundation/lessons.md` absent).

## Desired End State

- The tray menu starts with one item, "START" while the counter is stopped and "STOP" while it runs, then a separator, then "Otwórz", "Sprawdź teraz", "Zakończ". Clicking it starts or stops the counter at once (no confirmation), from the same Rust function the header command uses, so the same entry and event-log rows are written and `work-time-changed` / `events-changed` are emitted. The header, "Czas pracy" and the dashboard's event log update through their existing listeners.
- Before the first work-time read the item is disabled and reads "Czas pracy: wczytywanie…"; when the read fails or the check loop is stalled it is disabled and reads "Czas pracy niedostępny".
- The tooltip has two lines: the existing health text, then the work line, for example `Nawigator dnia — OK` / `Czas pracy dziś: 3:42 (trwa)`. It never exceeds 127 UTF-16 units; when it is too long the health part is cut with "…" and the work line stays whole.
- The icon is `running` (the ok glyph with an amber dot at the bottom right) when the sources are OK and the counter runs; `error` whenever a source fails, running or not; `idle` while starting or stalled; `ok` otherwise.
- Every work-time write (START, STOP, add, edit, remove, day off), from the header or the tray, refreshes the tray within one health round; the minute display is otherwise at most about 30 s late.
- A refused or failed tray action shows a value-free toast; a refusal that only means "already changed elsewhere" (`AlreadyRunning` / `NotRunning`) just refreshes the tray.

Verify with `cargo test` (pure rules) and in the running app (menu, tooltip line break, icon, header follow-up).

### Key Discoveries:

- `start_work` / `stop_work` notify right after the commit and only then read the status (`work_time/commands.rs:68-69`); the shared functions keep that order.
- `check_round` is already the tray's writer, runs on its own thread, already reads the store (`runtime.rs:296-326`), and stamps the heartbeat right after the probe (`:305-308`); the work-time read belongs inside the probe part, so a hanging store read is caught by the watchdog.
- `HealthSnapshot` is serialised to the webview (`health-changed`); keeping the work line outside it means no TS mirror change.
- The capability file grants only `core:event:allow-listen` / `allow-unlisten` and no `core:tray:*` on purpose (`src-tauri/capabilities/default.json:4-8`); tray control stays Rust-only, so nothing changes there.
- `in_day` and `running_entry` are private in `work_time/mod.rs` but visible to child modules (as `reminders.rs` uses them).

## What We're NOT Doing

- No confirmation for a tray STOP (user decision D1); the header keeps its confirmation unchanged.
- No new FR and no PRD edit (D3); no roadmap edit in this change.
- No "origin" (header vs tray) in the event log, no migration, no new event kinds.
- No frontend, TypeScript, `tokens.css`, capability, CSP or `tauri.conf.json` change; no new Tauri command or event.
- No toast on a successful tray START/STOP (S-07 adds the end-of-day notice later).
- No `origin` parameter on the shared START/STOP functions (S-07 adds what it needs).
- No change to autostart or start-up order (S-04).
- No update of the Konsola design-system artifact by an implementer: it is outside the repo and listed as a Manual follow-up for the user.

## Implementation Approach

Pure rules first, side effects last, following the existing split (`health/` and `work_time/` are `tauri`-free; `runtime.rs` and `commands.rs` own side effects):

1. A domain read of today's live total in a new `work_time/today.rs`.
2. A new `tauri`-free `tray_view.rs` that decides, from the health state and the work line, the icon, the tooltip and the START/STOP item, plus the text of a failure toast.
3. The running icon asset and a four-state `tray_image`.
4. The shared START/STOP functions in `work_time/commands.rs`; every work-time write wakes the health round; the health round reads the work line next to its store probe and stays the only tray writer (research option A).
5. The menu item, its main-thread handler that hands the work to the async runtime, and the docs.

New files keep edits in files shared with parallel slices (`lib.rs`, `runtime.rs`, `work_time/mod.rs`, `commands.rs`, README, CHANGELOG) to a few local lines each.

## Critical Implementation Details

**Threads and locks.** The tray menu callback runs on the main thread: it must only read the shown action and call `tauri::async_runtime::spawn`; it must never touch the store, take `published` or the snapshot lock. Tray and menu setters (`set_icon`, `set_tooltip`, `MenuItem::set_text`, `set_enabled`) run only inside `update_tray`, under `published`, on the check or watchdog thread. The work-line lock is never held across a tray, menu, emit or toast call.

**Act on the shown intent.** The handler starts or stops according to the action the item showed when it was last set, never by reading the store and toggling, so a stale label can never stop a counter the user meant to start. While `update_tray` changes the label, the shown action is `None` (cleared before the setters, set after them), so a click in that window does nothing. `AlreadyRunning` / `NotRunning` mean "changed elsewhere": refresh only, no toast.

**Ordering in the health round.** Read the work line right after `probe_round` and before `stamp_heartbeat`, and store it before the snapshot's `seq` is bumped, so any publish that sees the new `seq` also sees the new work line.

## Phase 1: Today's work total in the domain

### Overview

Add a cheap, pure read of today's live work time and running flag for the tray, equal to what the header shows.

### Changes Required:

#### 1. Today's total

**File**: `src-tauri/src/work_time/today.rs` (new)

**Intent**: Return whether the counter runs and today's total including the running span, without the month report that `status()` computes. It is the value the tray tooltip shows and must match the header's `todayTotalMs`.

**Contract**:
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct TodayWork { pub running: bool, pub today_ms: i64 }`.
- `pub fn today_work<Tz: TimeZone>(store: &Store, now: i64, tz: &Tz) -> Result<TodayWork, WorkTimeError>`: one `store.read`; `today_ms` = sum of `rules::portion(e.start, e.end, now, day_start, day_end)` over `in_day(today)` (closed and running entries alike, so a running span is capped at the day end and clamped at zero, a running entry from yesterday counts only from midnight, removed entries are ignored); `running` = `running_entry(..).is_some()` (any day).
- Module doc: no `tauri` import and no clock, like `reminders.rs`.

#### 2. Module registration

**File**: `src-tauri/src/work_time/mod.rs`

**Intent**: Register the new module and its test file; nothing else in this shared file changes.

**Contract**: add `pub mod today;` and `#[cfg(test)] mod today_tests;` next to the existing module lines.

#### 3. Tests

**File**: `src-tauri/src/work_time/today_tests.rs` (new)

**Intent**: Pin the total and the running flag on invented data in `Europe/Warsaw`, with its own small helpers copied from `src-tauri/src/work_time/tests.rs:14-28` (`t()` via `rules::resolve_local`, `open()` with `Store::open(…, schema::MIGRATIONS, today)` in a `tempfile` dir), using `start`, `stop`, `add` and `remove` from the domain to create entries. Import them with `use super::*;` and call them unqualified (`start(&mut store, …)`), never as `crate::work_time::start(`, which the Phase 4 grep (4.5) would flag.

**Contract**: tests at a fixed "now" of 2026-11-23 12:00 local:
- no entries → `{ running: false, today_ms: 0 }`;
- closed entries only → their sum, `running: false`;
- closed entries plus a running entry started today → sum plus `now - start`, `running: true`;
- a running entry started yesterday 22:00 → counts from local midnight (12 h at noon), `running: true`;
- a closed entry crossing midnight from yesterday → only today's part;
- a removed entry → not counted;
- a running entry whose start is after `now` (clock moved back) → contributes 0;
- for each fixture above, `today_ms` equals `status().today_closed_ms + max(0, min(now, day_end_ms) - running_from_ms)` (the header formula from `src/worktime/total.ts`), and `running` equals `status().running.is_some()`.

### Success Criteria:

#### Automated Verification:

- Rust tests pass, including the new `work_time::today_tests`: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

**Implementation Note**: After completing this phase and all automated verification passes, commit it (`feat(tray-timer-control): today's work total`) and continue; this phase has no manual checks.

---

## Phase 2: Tray view rules

### Overview

Add a pure, `tauri`-free module that decides what the tray shows from the health state and the work line: the icon, the two-line tooltip within 127 UTF-16 units, the START/STOP item, and the text of a failed tray action's toast.

### Changes Required:

#### 1. Expose the tooltip cap

**File**: `src-tauri/src/health/mod.rs`

**Intent**: Let the tray composition reuse the existing UTF-16 cap and limit; behaviour and existing tests unchanged.

**Contract**: `TOOLTIP_MAX_UTF16` becomes `pub const`; `cap_utf16` becomes `pub fn cap_utf16(text: String, max: usize) -> String` (doc: at most `max` UTF-16 units including the "…", never splits a character). `health::tooltip` is unchanged.

#### 2. Tray view module

**File**: `src-tauri/src/tray_view.rs` (new), registered with `pub mod tray_view;` in `src-tauri/src/lib.rs` (one line, alphabetical)

**Intent**: One place for every rule about the tray's content, testable without `tauri`. All items are `pub` so the module compiles warning-free before it is wired in Phase 4.

**Contract**:
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum WorkLine { Loading, Unavailable, Known(TodayWork) }` (`TodayWork` from `work_time::today`).
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TrayIconKind { Idle, Ok, Running, Error }` (not `TrayIcon`, which is the `tauri::tray::TrayIcon` handle `runtime.rs` works with) and `pub fn tray_icon(overall: Overall, work: &WorkLine) -> TrayIconKind`: `Error` → `Error` (whatever the counter); `Starting` / `NotRunning` → `Idle`; `Ok` with `Known { running: true, .. }` → `Running`; `Ok` otherwise → `Ok`.
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TimerAction { Start, Stop }` and `pub struct WorkItem { pub text: &'static str, pub action: Option<TimerAction> }` (enabled exactly when `action` is `Some`); `pub fn work_item(overall: Overall, work: &WorkLine) -> WorkItem`: `NotRunning` (check loop stalled, the tray can no longer refresh) → `"Czas pracy niedostępny"`, `None`; otherwise `Known` running → `"STOP"`, `Some(Stop)`; `Known` stopped → `"START"`, `Some(Start)`; `Loading` → `"Czas pracy: wczytywanie…"`, `None`; `Unavailable` → `"Czas pracy niedostępny"`, `None`.
- `pub fn format_hm(ms: i64) -> String`: `H:MM` floored to whole minutes, negative as `0:00` (same as `formatHm` in `src/worktime/format.ts`).
- `pub fn work_line_text(overall: Overall, work: &WorkLine) -> String`: `NotRunning` → `"Czas pracy niedostępny"`; `Known` running → `"Czas pracy dziś: H:MM (trwa)"`; `Known` stopped → `"Czas pracy dziś: H:MM"`; `Loading` → `"Czas pracy: wczytywanie…"`; `Unavailable` → `"Czas pracy niedostępny"`.
- `pub const LINE_SEPARATOR: &str = "\n";` and `pub fn tooltip(overall: Overall, sources: &[SourceStatus], work: &WorkLine) -> String`: `cap_utf16(health::tooltip(overall, sources), TOOLTIP_MAX_UTF16 - units(LINE_SEPARATOR) - units(work line))` + `LINE_SEPARATOR` + work line, so the result is at most 127 units and always ends with the whole work line. Only the app name, source labels and the user's own counter value appear; no titles, paths or messages.
- `pub fn failure_notice(action: TimerAction, err: &WorkTimeError) -> Option<health::Notice>`: `AlreadyRunning` / `NotRunning` → `None`. Otherwise title `"Nawigator dnia — START nie zadziałał"` / `"Nawigator dnia — STOP nie zadziałał"` (built from `APP_NAME`) and a fixed body by kind: Start + `Overlap` → `"Zegar systemowy wskazuje czas sprzed końca innego wpisu — sprawdź zegar lub popraw wpis w „Czas pracy”."`; Stop + `EndNotAfterStart` → `"Zegar systemowy wskazuje czas sprzed początku wpisu — STOP zadziała, gdy zegar go dogoni."`; `Store(_)` other than `Store(StoreError::Busy)` → `"Dane aplikacji są niedostępne — szczegóły w panelu „Źródła”."`; anything else (including the transient `Store(Busy)`, which the "Źródła" panel would not show) → `"Otwórz aplikację i spróbuj ponownie."`. No ids, times or error detail in the text.

#### 3. Tests

**File**: `src-tauri/src/tray_view.rs` (`#[cfg(test)] mod tests`)

**Intent**: Pin the exact texts, the icon priority, the item states and the length limit, on invented labels.

**Contract**:
- `format_hm`: `0` → `0:00`; `59_999` → `0:00`; `60_000` → `0:01`; 3 h 42 min 59 s → `3:42`; 25 h → `25:00`; `-1` → `0:00`.
- Exact tooltips: `(Ok, [], Known{running: true, 3 h 42 min 30 s})` → `"Nawigator dnia — OK\nCzas pracy dziś: 3:42 (trwa)"`; `(Ok, [], Known{running: false, 0})` → `"Nawigator dnia — OK\nCzas pracy dziś: 0:00"`; `(Starting, [], Loading)` → `"Nawigator dnia — uruchamianie…\nCzas pracy: wczytywanie…"`; `(Error, [Dane aplikacji Damaged], Unavailable)` → `"Nawigator dnia — błąd: Dane aplikacji\nCzas pracy niedostępny"`; `(NotRunning, [], Known{running: true, ..})` → `"Nawigator dnia — sprawdzanie nie działa\nCzas pracy niedostępny"`.
- A very long source label (`"Żółć".repeat(60)`, `Error`): length ≤ 127, the first line ends with "…", the tooltip ends with the whole work line.
- Proptest (same label generator as `health::tests::tooltip_is_always_within_limit`, every `Overall`, every `WorkLine` with `today_ms` in `0..=25 h` and both `running` values): length ≤ 127 UTF-16 units, starts with `"Nawigator dnia"`, ends with `work_line_text(overall, &work)`.
- Icon table over every `Overall` × {`Loading`, `Unavailable`, `Known` running, `Known` stopped}: `Error` is always `Error`, `Running` appears only for `Ok` + running.
- Item table over the same product: `NotRunning` always disabled; `Known` running → `STOP`, stopped → `START`; `Loading` / `Unavailable` disabled with their texts.
- `failure_notice`: `None` for `AlreadyRunning` and `NotRunning` with both actions; the four body texts above; `Store(Busy)` gives the generic body, another `Store(_)` (for example `Store(Io { .. })`) the "Dane aplikacji" body; with `Overlap { entry_id: 4242 }` neither title nor body contains `4242`; every title starts with `"Nawigator dnia"`.

### Success Criteria:

#### Automated Verification:

- Rust tests pass, including the new `tray_view::tests` and the unchanged `health` tooltip tests: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- `tray_view.rs` imports nothing from `tauri`: `git grep -n -E "use tauri|tauri::" -- src-tauri/src/tray_view.rs` returns nothing
- No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

**Implementation Note**: After completing this phase and all automated verification passes, commit it (`feat(tray-timer-control): tray view rules`) and continue; this phase has no manual checks.

---

## Phase 3: Running tray icon

### Overview

Add the fourth tray icon state and let `tray_image` take a `tray_view::TrayIconKind` instead of `Overall`. Visible behaviour is unchanged until Phase 4 passes a real work line.

### Changes Required:

#### 1. Icon source

**File**: `src-tauri/icons/source/tray-running.svg` (new)

**Intent**: The "counter running" state: the ok glyph (amber ring and needle) with a filled amber dot at the bottom right, cut out of the ring the same way the error badge is cut at the top right. Not red and not top right, which mean "error". The header's REC marker is `--danger` red (`src/worktime/WorkTimerHeader.css`, `.dn-rec`); the tray deliberately does not copy it, because in the tray red means a source error (D2). Do not "align" the dot to red.

**Contract**: a copy of `tray-ok.svg` with the comment changed to state "running" (amber dot at the bottom right = the counter runs; red and the top-right badge are reserved for errors), one more mask circle `<circle cx="824" cy="824" r="248" fill="#000"/>` and, after the masked group, `<circle cx="824" cy="824" r="176" fill="#F5A524"/>`.

#### 2. Icon PNG

**File**: `src-tauri/icons/tray/running.png` (new, generated)

**Intent**: The compiled-in 32x32 RGBA tray image, produced by the documented recipe.

**Contract**: from the repository root, with `<tmp>` a fresh directory outside the repo (for example under `%TEMP%`):
- `npx tauri icon src-tauri/icons/source/tray-running.svg -o <tmp> --png 32`
- copy `<tmp>/32x32.png` to `src-tauri/icons/tray/running.png`.
Never run `npx tauri icon` without `-o`: its default output is `src-tauri/icons/` and would overwrite the bundle icons.

#### 3. Four-state `tray_image`

**File**: `src-tauri/src/runtime.rs`

**Intent**: Map the `tray_view::TrayIconKind` decision to the compiled-in images; keep today's behaviour at both call sites.

**Contract**: `fn tray_image(icon: TrayIconKind) -> Image<'static>` with four `include_image!` arms (`Idle` → `idle.png`, `Ok` → `ok.png`, `Running` → `running.png`, `Error` → `error.png`). `build_tray` passes `TrayIconKind::Idle`; `update_tray` passes `tray_view::tray_icon(snap.overall, &WorkLine::Loading)` (which yields the same icon as today) until Phase 4.

### Success Criteria:

#### Automated Verification:

- The recipe still reproduces the shipped icon: `npx tauri icon src-tauri/icons/source/tray-ok.svg -o <tmp2> --png 32` gives a `32x32.png` whose `Get-FileHash` equals that of `src-tauri/icons/tray/ok.png`
- `running.png` is 32x32 RGBA: PowerShell `$b=[IO.File]::ReadAllBytes("src-tauri/icons/tray/running.png"); "{0}x{1} type {2}" -f [BitConverter]::ToInt32($b[19..16],0), [BitConverter]::ToInt32($b[23..20],0), $b[25]` prints `32x32 type 6`
- Bundle icons untouched: `git status --short src-tauri/icons` lists only `tray-running.svg` and `running.png`
- Rust tests pass: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

**Implementation Note**: After completing this phase and all automated verification passes, commit it (`feat(tray-timer-control): running tray icon`) and continue; the icon's look is checked in Phase 4.

---

## Phase 4: Shared START/STOP path and live tray state

### Overview

Give the header command and the tray one START and one STOP function, make every work-time write refresh the tray, and let the health round read today's work and compose it into the icon and the tooltip. After this phase the tray follows the header; the menu item comes in Phase 5.

### Changes Required:

#### 1. Shared START/STOP functions

**File**: `src-tauri/src/work_time/commands.rs`

**Intent**: One clear Rust function per action (user decision D4), used by `start_work` / `stop_work` now and by the tray in Phase 5, so S-07 (a notice after STOP) and S-04 have one place to hook into. The body is the one inlined in the commands today, unchanged in order: domain write, then `work-time-changed`, then `events-changed`.

**Contract**:
- `pub(crate) async fn start_timer(app: &AppHandle) -> Result<(), WorkTimeError>` and `pub(crate) async fn stop_timer(app: &AppHandle) -> Result<(), WorkTimeError>`: `run(app, "start"|"stop", …super::start|stop…)` then, on success only, `notify(app)` and `crate::events::commands::notify(app)`. Doc comment on `stop_timer`: the one STOP path for the header and the tray; a later notice after STOP hooks in here (for every STOP) or in the tray caller (tray STOP only).
- `start_work` / `stop_work` become `start_timer(&app).await?; work_timer_status(app).await` (and the same for stop); the existing ordering comment moves to the shared functions. Commands keep their names, signatures and return types (no TS change).
- `notify(app)` additionally calls `crate::runtime::refresh_tray(app)`, so every work-time write (START, STOP, add, edit, remove, day off) refreshes the tray.

#### 2. Tray refresh and the work line in the health round

**File**: `src-tauri/src/runtime.rs`

**Intent**: Keep the health round the only tray writer (research option A): a work-time write wakes it; it reads today's work next to its store probe and publishes the icon and the tooltip from both the health snapshot and the work line.

**Contract**:
- `pub(crate) fn refresh_tray(app: &AppHandle)`: sends on `HealthState.wake` if `HealthState` is managed (`try_state`); a closed channel is ignored (the stalled check is already shown).
- `HealthState` gets a private `work: RwLock<WorkLine>` (initially `WorkLine::Loading`), not part of the serialised `HealthSnapshot`; `health_status` and `health-changed` stay unchanged.
- `check_round`: hoist the probe's date into `let today = Local::now().date_naive();` (today it is computed inline in the `probe_round` call) and pass it to `probe_round`; right after `probe_round` and before `stamp_heartbeat`, read `work_time::today::today_work(store, chrono::Utc::now().timestamp_millis(), &Local)` through `app.state::<StoreHandle>().with_store(today, …)`. Use `Utc::now().timestamp_millis()` (an `i64`, as `commands::run` does), not `runtime::now_ms()`, which returns `u64`. `Ok` → `WorkLine::Known`; `Err(WorkTimeError::Store(StoreError::Busy))` → keep the previous line (logged at debug); any other error → `WorkLine::Unavailable` (logged at warn by `kind()` only). Store it before the snapshot write that bumps `seq`.
- `publish` reads the work line (a copy) after the snapshot, under `published`, and passes it to `update_tray(app, &snap, &work)`.
- `update_tray` sets the icon from `tray_view::tray_icon(snap.overall, work)` and the tooltip from `tray_view::tooltip(snap.overall, &snap.sources, work)`.
- `build_tray` keeps its initial tooltip but builds it with `tray_view::tooltip(Overall::Starting, &[], &WorkLine::Loading)`.
- Update the module's locking comment to name the work-line lock: never held across a tray, emit or toast call.

### Success Criteria:

#### Automated Verification:

- Rust tests pass: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- One START and one STOP call into the domain: `git grep -n -E "super::(start|stop)\(" -- src-tauri/src/work_time/commands.rs` lists exactly two lines, inside `start_timer` and `stop_timer`, and `git grep -n -E "work_time::(start|stop)\(" -- src-tauri/src` lists nothing
- No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

#### Manual Verification:

- In the running app, the tray tooltip shows two lines: the health text, then "Czas pracy dziś: H:MM" (the line break renders; if Windows shows it on one line, set `LINE_SEPARATOR` to `" · "` and re-run the Phase 2 tests)
- Header START: within a few seconds the tray icon shows the running variant and the tooltip's work line ends with "(trwa)"; header STOP: the ok icon returns and "(trwa)" disappears
- The running icon is legible at 16 px on the taskbar and clearly different from the error and ok icons
- With a source in error while the counter runs (for example `settings.json` made invalid), the icon is the error icon and the tooltip still ends with the work line
- The tooltip's minutes follow the header's clock within about 30 s

**Implementation Note**: After completing this phase and all automated verification passes, commit it (`feat(tray-timer-control): shared START/STOP and live tray state`) and pause for manual confirmation from the human that the manual testing was successful before proceeding.

---

## Phase 5: START/STOP item in the tray menu

### Overview

Add the tray menu item, its main-thread handler that hands the action to the async runtime, the failure toast, and the user-facing docs.

### Changes Required:

#### 1. Menu item, handler and refresh

**File**: `src-tauri/src/runtime.rs`

**Intent**: A "START"/"STOP" item at the top of the tray menu that acts at once (D1) on the action it shows, through the shared functions from Phase 4, and is kept up to date by `update_tray`.

**Contract**:
- `build_tray`: create `MenuItem::with_id(app, "work", "Czas pracy: wczytywanie…", false, None::<&str>)`, then `PredefinedMenuItem::separator(app)`, then the existing `open`, `check`, `quit`. Manage a `TrayWorkItem { item: MenuItem<tauri::Wry>, shown: Mutex<Option<TimerAction>> }` (or an equivalent atomic), registered before the tray is built.
- `update_tray`: get the item with `app.try_state::<TrayWorkItem>()` (absent → skip the item); `let w = tray_view::work_item(snap.overall, work)`; store `None` in `shown`, then `set_text(w.text)`, `set_enabled(w.action.is_some())` (failures logged at warn), then store `w.action` in `shown`. Each `shown` write is a short lock released before the next setter. So a click while the label is changing does nothing, and `shown` only ever holds an action the label shows. Only here are the menu setters called.
- `on_menu_event` `"work"`: read `shown` (a short lock, never held across another call); `None` → do nothing; `Some(action)` → `tauri::async_runtime::spawn(run_tray_action(app.clone(), action))`. No store access, no `published`, no snapshot lock on the main thread.
- `async fn run_tray_action(app: AppHandle, action: TimerAction)`: call `work_time::commands::start_timer` or `stop_timer`; on `Err(e)`: if `tray_view::failure_notice(action, &e)` is `Some(n)`, `toast(&app, "work_time", &n.title, &n.body)`; then `refresh_tray(&app)` (a success already refreshed through `notify`). Logging is left to `run`, by kind only.
- Extend the module's locking comment: menu setters block like tray setters and run only in `update_tray`; the menu callback only spawns.

#### 2. User docs

**File**: `README.md`

**Intent**: Mention the tray control under "Czas pracy (Work time)".

**Contract**: one sub-bullet, e.g. "START/STOP from the tray menu; the tray tooltip shows today's work time and the tray icon shows when the counter runs."

**File**: `CHANGELOG.md`

**Intent**: Record the feature for the next release.

**Contract**: under `## [Unreleased]`, an `### Added` entry: the tray menu has START/STOP for the work-time counter (no confirmation, logged in the event log like the header), the tooltip shows today's work time under the source state, and the icon has a "counter running" variant (an error still shows the error icon).

### Success Criteria:

#### Automated Verification:

- Rust tests pass: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Menu setters are called only in `update_tray`: `git grep -n -E "set_text|set_enabled" -- src-tauri/src` lists only lines inside `update_tray` in `runtime.rs`
- No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

#### Manual Verification:

- The tray menu shows "START" at the top while stopped and "STOP" while running, above a separator and "Otwórz", "Sprawdź teraz", "Zakończ"
- With the main window hidden, tray START starts the counter; on showing the window the header shows REC with the running time, and the dashboard's event log has a new START row
- Tray STOP stops the counter at once with no question; the visible header switches to stopped without focusing the window, and the event log has a STOP row
- With the header's "Zakończyć pracę?" question open, tray STOP makes the question disappear and the header shows stopped
- In a built app (`npm run tauri build -- --debug --no-bundle`), the tray item works and the DevTools console shows no CSP violation and no "IPC custom protocol failed"
- Follow-up outside the repo: the Konsola design-system artifact (https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY) documents the fourth tray icon state "running" and the START/STOP menu item

**Implementation Note**: After completing this phase and all automated verification passes, commit it (`feat(tray-timer-control): START/STOP in the tray menu`) and pause for manual confirmation from the human that the manual testing was successful.

---

## Testing Strategy

### Unit Tests:

- `work_time::today_tests`: today's total and running flag, midnight split, removed entries, clock behind a running start, and agreement with the header formula over `status()`.
- `tray_view::tests`: exact tooltips, truncation that keeps the work line, a proptest for the 127-unit limit over every state, the icon priority table, the item table, and value-free failure toasts.
- Existing `health` tooltip tests and the `work_time` integration test `start_and_stop_store_events_in_local_time_and_refusals_store_none` (`src-tauri/tests/work_time.rs`) stay green; the tray reuses exactly that domain path.

### Integration Tests:

- None new: the side effects (menu callback, spawn, emits, tray setters) need a running Tauri app and the repo has no Tauri test harness. The grep checks in Phases 4 and 5 guard the single START/STOP path and the single place for menu setters.

### Manual Testing Steps:

1. Start the app with the counter stopped; hover the tray icon: two lines, "Czas pracy dziś: H:MM".
2. Click START in the tray menu (no question): the icon gets the amber dot, the tooltip adds "(trwa)", the header shows REC, the event log shows START.
3. Wait a few minutes: the tooltip's minutes follow the header within about 30 s.
4. Open the header's STOP question, then click STOP in the tray: the question disappears, the counter stops, the event log shows STOP.
5. Make `settings.json` invalid with the counter running: the icon turns to the error icon, the tooltip still shows the work line; restore the file.
6. Build with `npm run tauri build -- --debug --no-bundle` and repeat step 2; check the DevTools console for CSP or IPC errors.

## Performance Considerations

Each work-time write now wakes one extra health round (settings read, store status, one `today_work` query) and one extra `health-changed` emit; the round is the same work the 30 s timer already does, so the cost is negligible for a personal app. `today_work` is a single query over today's entries, cheaper than `status()`, which also computes the month report.

## Migration Notes

None: no schema change, no new setting, no new stored data.

## Decisions

User decisions (binding, given with the task):

- **D1.** A tray STOP needs no confirmation; it works at once and is logged in the event log.
- **D2.** The tray icon gets a "counter running" variant; a source error keeps priority in the icon.
- **D3.** The slice stays anchored as MS-23; no new FR, no PRD edit.
- **D4.** START and STOP are each one Rust function shared by the header command and the tray (`start_timer` / `stop_timer`), so S-07 and S-04 can hook into them.

Agent defaults (no user was reachable; each can be revisited):

- **A1.** The health round stays the only tray writer: every work-time write wakes it, and it reads today's work next to its store probe (research option A), rather than a second tray writer with a changed `seq` guard.
- **A2.** Every work-time write (not only START/STOP) refreshes the tray, through `notify` in `commands.rs`.
- **A3.** One menu item (`work`) whose text is "START" or "STOP", at the top of the menu with a separator below; the handler acts on the shown action, and `AlreadyRunning` / `NotRunning` only refresh the tray.
- **A4.** Disabled item texts: "Czas pracy: wczytywanie…" before the first read, "Czas pracy niedostępny" when the read fails or the check loop is stalled.
- **A5.** Two-line tooltip: health text first, then "Czas pracy dziś: H:MM" with " (trwa)" while running; "Czas pracy: wczytywanie…" / "Czas pracy niedostępny" otherwise; minutes floored like the UI's `formatHm`. Line break `"\n"`, with `" · "` as the fallback if Windows does not render it (manual check 4.7).
- **A6.** Icon priority: `Error` → error; `Starting` / `NotRunning` → idle; `Ok` + running → running; `Ok` otherwise → ok.
- **A7.** Running icon: the ok glyph with a filled amber dot at the bottom right, cut out of the ring like the error badge; never red, never top right.
- **A8.** Failure toast texts for a tray action, fixed by kind and value-free (Phase 2 contract).
- **A9.** A `Busy` store during the tray read keeps the previous work line; any other failure shows "niedostępny".
- **A10.** While the check loop is stalled (`NotRunning`), the work line reads "Czas pracy niedostępny" and the item is disabled, because the tray can no longer refresh.
- **A11.** No toast on a successful tray action; no origin column in the event log; no origin parameter on the shared functions.
- **A12.** Pure rules in new files (`work_time/today.rs`, `tray_view.rs`) to keep edits in files shared with S-01, S-04 and S-06 to a few lines.
- **A13.** README and CHANGELOG are updated in Phase 5; the Konsola design-system artifact is a manual follow-up for the user.
- **A14.** The roadmap item is not moved to `planning` by this run (the task forbids editing `context/foundation/roadmap.md`).

Agent defaults added by the plan review (`reviews/plan-review.md`; no user was reachable):

- **A15.** `update_tray` clears the shown action before the menu setters and sets the new one after them, so a click during a label change does nothing (F2).
- **A16.** A transient `Store(Busy)` on a tray action gets the generic toast body, not the "Dane aplikacji są niedostępne" one (F6).
- **A17.** The icon decision type is `TrayIconKind`, so it is not confused with `tauri::tray::TrayIcon` (F3).
- **A18.** The tray's running dot stays amber although the header's REC is red, because red in the tray means a source error (D2, F5).

## References

- Research: `context/changes/tray-timer-control/research.md`
- Tray and health writer: `src-tauri/src/runtime.rs:5-9`, `:139-185`, `:206-237`, `:296-355`
- Tooltip and cap: `src-tauri/src/health/mod.rs:17-21`, `:257-307`, `:688-738`
- Domain START/STOP and reads: `src-tauri/src/work_time/mod.rs:109-141`, `:166-202`, `:257-294`; `src-tauri/src/work_time/rules.rs:57-63`; `src-tauri/src/work_time/reminders.rs:51-73`
- Commands: `src-tauri/src/work_time/commands.rs:17-87`
- UI subscription and header: `src/worktime/useWorkTimer.ts:82-176`, `src/worktime/WorkTimerHeader.tsx:15-46`, `src/worktime/total.ts:8-18`, `src/worktime/format.ts:21-24`
- Icon recipe: `src-tauri/icons/source/app-icon.svg:5-7`, `src-tauri/icons/source/tray-ok.svg`, `tray-error.svg`
- Prior changes: `context/archive/2026-11-20-source-health-status/`, `context/archive/2026-11-23-work-timer/`, `context/archive/2026-11-24-shared-work-timer-state/`, `context/archive/2026-11-24-event-log/`, `context/archive/2026-11-24-design-system-artifact-sync/`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Today's work total in the domain

#### Automated

- [x] 1.1 Rust tests pass, including the new `work_time::today_tests`: `cargo test` (in `src-tauri/`)
- [x] 1.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [x] 1.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [x] 1.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [x] 1.5 No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

### Phase 2: Tray view rules

#### Automated

- [ ] 2.1 Rust tests pass, including the new `tray_view::tests` and the unchanged `health` tooltip tests: `cargo test` (in `src-tauri/`)
- [ ] 2.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 2.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 2.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 2.5 `tray_view.rs` imports nothing from `tauri`: `git grep -n -E "use tauri|tauri::" -- src-tauri/src/tray_view.rs` returns nothing
- [ ] 2.6 No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

### Phase 3: Running tray icon

#### Automated

- [ ] 3.1 The recipe still reproduces the shipped icon: regenerated `tray-ok.svg` `32x32.png` hash equals `src-tauri/icons/tray/ok.png`
- [ ] 3.2 `running.png` is 32x32 RGBA (IHDR check prints `32x32 type 6`)
- [ ] 3.3 Bundle icons untouched: `git status --short src-tauri/icons` lists only `tray-running.svg` and `running.png`
- [ ] 3.4 Rust tests pass: `cargo test` (in `src-tauri/`)
- [ ] 3.5 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 3.6 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 3.7 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

### Phase 4: Shared START/STOP path and live tray state

#### Automated

- [ ] 4.1 Rust tests pass: `cargo test` (in `src-tauri/`)
- [ ] 4.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 4.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 4.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 4.5 One START and one STOP call into the domain (grep lists exactly two lines in `commands.rs`, none elsewhere)
- [ ] 4.6 No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

#### Manual

- [ ] 4.7 Tray tooltip shows two lines: health text, then "Czas pracy dziś: H:MM" (line break renders)
- [ ] 4.8 Header START/STOP switches the tray icon to running and back, and "(trwa)" appears and disappears
- [ ] 4.9 Running icon is legible at 16 px and clearly different from the error and ok icons
- [ ] 4.10 A source error while running shows the error icon and the tooltip still ends with the work line
- [ ] 4.11 Tooltip minutes follow the header's clock within about 30 s

### Phase 5: START/STOP item in the tray menu

#### Automated

- [ ] 5.1 Rust tests pass: `cargo test` (in `src-tauri/`)
- [ ] 5.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 5.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 5.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 5.5 Menu setters are called only in `update_tray` (grep for `set_text|set_enabled`)
- [ ] 5.6 No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

#### Manual

- [ ] 5.7 Tray menu shows START/STOP at the top, above a separator and the existing items
- [ ] 5.8 Tray START with the window hidden starts the counter; header and event log show it
- [ ] 5.9 Tray STOP stops at once without a question; header and event log follow without focusing the window
- [ ] 5.10 An open header "Zakończyć pracę?" question disappears after a tray STOP
- [ ] 5.11 Built app (`npm run tauri build -- --debug --no-bundle`): tray item works, no CSP violation, no "IPC custom protocol failed"
- [ ] 5.12 Konsola design-system artifact documents the running tray state and the START/STOP item (user, outside the repo)
