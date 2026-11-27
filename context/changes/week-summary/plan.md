# Week summary in "Czas pracy" — Implementation Plan

## Overview

Roadmap S-08 (M-3), GitHub issue #18, anchored as MS-18 (no new FR, no PRD edit; the week reuses the FR-014 norm rule, `context/foundation/prd.md:155`). The "Czas pracy" view gets a panel "Bieżący tydzień": the current Monday–Sunday week, always all seven days even across a month or year boundary, day by day (worked, day norm, deviation, kind of non-working day) with the week's totals (worked, whole-week norm, deviation against the norm up to and including today, remaining to the week's norm). Every figure is computed in Rust by the month's own per-day code over one store snapshot; the UI only displays it.

## Current State Analysis

- The per-day row (worked time and entry count from `rules::portion` over `day_bounds`, `norm::day_norm_ms`, `norm::day_deviation_ms`, the private `classify`, the holiday name and `day_off_owed`) is built inline in the month loop of `report::compute` (`src-tauri/src/work_time/report.rs:144-168`). `days_off_between` (`report.rs:67-89`) and `classify` (`report.rs:101-111`) are private; both work for any half-open date range.
- `compute` reads exactly one month (`report.rs:130-138`), so a week across a month cannot be a slice of one `MonthReport`, and two month reads would break the one-snapshot rule.
- `in_day(conn, start, end, now)` (`src-tauri/src/work_time/mod.rs:131-153`) and the private `Entry` (`mod.rs:95-100`) are visible to child modules of `work_time`; `today.rs` (`today.rs:23-40`) is the precedent for a small new file with its own single `store.read`.
- `norm::figures` (`norm.rs:113-152`) multiplies `DAILY_NORM_MS` (`norm.rs:14`) by a workday count; S-02 (configurable daily norm, half day off) will change it and `day_norm_ms`. `MonthPhase` / `phase` (`norm.rs:77-92`) compare year and month only.
- `work_month_report(day)` (`commands.rs:213-223`) parses with `report::parse_day` and runs under `Policy::Strict`; `lib.rs:213-222` lists the work-time handlers.
- UI: `WorkTimeView.tsx` holds one `day` state (`:89`) and the main column panels "Wybrany dzień" and "Dni miesiąca" (`:200-229`); `useMonthReport.ts` is the loading / error with `lastOkAt` / loaded hook with `work-time-changed`, focus/visibility refetch and a 60 s poll (`:16-119`); `DaysTable.tsx` has the row anatomy and the private `liveFigures` (`:14-26`) that shifts today's row by the running counter. `useWorkTimer()` ticks `nowMs` only while the counter runs (`useWorkTimer.ts:138-150`), so the view does not re-render at midnight on its own.
- Tests: `norm_tests::manual_only()` is `pub(super)` and already reused by `tests.rs:7`; the month proptest pins "day rows add up to the month figures" (`norm_tests.rs:566-616`).

## Desired End State

- In "Czas pracy", between "Wybrany dzień" and "Dni miesiąca", a panel "Bieżący tydzień" with the week's dates in its head (e.g. `23.11 – 29.11.2026`, `30.11 – 06.12.2026`, `28.12.2026 – 03.01.2027`).
- A stat strip: "Tydzień" (worked, "z H:MM normy"), "Odchylenie" (worked minus the norm up to and including today, info/warn tone), "Zostało" (remaining to the whole-week norm, "norma tygodnia wyrobiona" at 0).
- Seven rows Monday to Sunday: day label (click picks that day in "Wybrany dzień"), an `HoursBar` against the day norm with the kind label for a non-working day (weekend / dzień wolny / holiday name, with the owed-day hint for a Saturday holiday), worked `H:MM`, day norm `H:MM`, deviation `±H:MM`. Today's row is highlighted and follows the running counter like the days table.
- Loading, error (`ErrorState` with "ostatnio OK HH:MM" and retry) and data states; an invalid `workTime` block shows `SettingsInvalid` exactly like the month report.
- For any day, the week row equals the month report's row for that day; with default settings the weeks of the research table have the norms 40 h / 40 h / 24 h / 32 h / 32 h.
- README and CHANGELOG (`[Unreleased]` → Added) mention the panel; the PRD's "considered but not built" sentence (`prd.md:234`) no longer lists the week view (no new FR). All per-commit gates are green after every phase.

### Key Discoveries:

- One shared per-day function used by both the month and the week is the only way to guarantee "exactly the same rule": S-02's norm change then lands in one place and flows into the week (`report.rs:144-168`; S-01 plan warned a third copy would drift).
- Week totals come from the day rows (`Σ norm_ms`, `Σ worked_ms`), never from `norm::figures`, which is not half-day safe (`norm.rs:120, 139`).
- The week's Monday and Sunday strings can be taken from the first and last `DayReport.day`, so `DAY_FORMAT` stays private.
- The view does not re-render at midnight while the counter is stopped, so the week hook must determine "today" at each request, not take it from a render (`useWorkTimer.ts:138-150`).

## What We're NOT Doing

- No previous/next week navigation and no week of the selected day: always the week containing today (the command takes a day, so navigation can come later without a contract change).
- No separate view or navigation item (user decision).
- No change to `norm.rs`, to the month's figures or to `MonthReport` / `DayReport` shapes; no `DAILY_NORM_MS`, no literal 8 h and no `norm::figures` in new code.
- No "średnio / dzień" for the week, no entry counts and no day-off switch in the week rows (those stay in "Dni miesiąca").
- No live re-sum of the week totals in the UI: only today's row follows the running counter (as in the days table); the totals refresh with the report.
- No edit to the "Jak liczę" panel (`HowCounted`), which S-02 rewrites; the week panel explains itself in its footer without naming 8 h.
- No new FR and no PRD edit beyond removing "widok tygodnia" from the "considered but not built" sentence (`prd.md:234`, A14); no `context/foundation/roadmap.md` edit (status sync skipped per task), no migration, no new token.

## Implementation Approach

Four phases, each green on its own and committed separately:

1. **Shared day row** — a mechanical extraction in `report.rs`: the loop body of `compute` becomes `pub(super) fn day_report`, and `days_off_between` becomes `pub(super)`. Month behaviour and every existing test are unchanged.
2. **Week report domain** — new `work_time/week.rs` (pure week bounds and phase, store-backed `week_report` with one `store.read`) and `work_time/week_tests.rs` with the invented-data calendar cases and a proptest. Only `mod.rs` gets the two module lines.
3. **Command and contract** — `work_week_report(day)` (Strict) in `commands.rs`, one line in `lib.rs`, and in the same commit the TS mirror (`WeekReport`, `WeekPhase`) and `workWeekReport` in `api.ts`.
4. **Panel** — `useWeekReport.ts`, `WeekSummary.tsx`, new rules at the end of `WorkTime.css`, `export` on `liveFigures`, one import and one element in `WorkTimeView.tsx`, README and CHANGELOG bullets, the one-line PRD update.

New behaviour sits in new files; shared files get a few local lines each (`report.rs` extraction, `mod.rs` 2–4 lines, `commands.rs` one command, `lib.rs` one line, `types.ts`/`api.ts` appended blocks, `WorkTimeView.tsx` two lines, `DaysTable.tsx` one word, `prd.md` one sentence).

The `rg` checks in the success criteria run in the Bash tool (Git Bash); `rg` is not on the PowerShell PATH in this environment.

## Critical Implementation Details

- **Extraction must be byte-for-byte behaviour-preserving.** `compute` keeps its own month-span `worked_ms` and `norm::figures` call; only the per-day row moves. `norm_tests.rs` is not edited in Phase 1, so a green `cargo test` proves the month is unchanged.
- **`day_report` takes `&WorkTimeSettings`, not `HolidayCalendar`.** S-02 adds its daily-norm field to `WorkTimeSettings` (`settings/work_time.rs:12`); passing the whole settings keeps the helper's signature stable and lets S-02 change only its body.
- **Midnight rollover.** `useWeekReport` sends the local date computed at request time (not a prop) and keeps no per-key cache, so the Sunday→Monday rollover is picked up by the next poll, focus or `work-time-changed` (within 60 s) without a loading flash.
- **Merge with S-02.** S-02's plan (`slice/configurable-daily-norm`, `context/changes/configurable-daily-norm/plan.md`) changes, among others: `days_off_between` → returns `DaysOff = BTreeMap<NaiveDate, DayOff>`; `classify(day, Option<DayOff>, holidays)`; `norm::day_norm_ms(day, &DaysOff, holidays, daily_norm_ms)`; `report::set_day_off(store, day, mark: DayMark, …)` instead of `off: bool`; a new `DayKind::HalfDayOff` (TS `"halfDayOff"`, not hatched, labelled "pół dnia wolnego" in the days table). So:
  - The `report.rs` extraction textually conflicts with S-02's edit of the same loop; resolve by moving S-02's change into `day_report` (its `days_off` parameter takes S-02's `&DaysOff`).
  - `week.rs` never names the days-off type and never inspects it: it binds `let days_off = report::days_off_between(…)?;` and only passes `&days_off` to `day_report`, so S-02's type change needs no edit in `week.rs`.
  - `week_tests.rs` marks days off only through one local helper `mark_off(store, day, settings)` wrapping `report::set_day_off(…, true, …)`, so S-02's `DayMark` is a one-line fix there; no test builds a days-off set by hand.
  - `WeekSummary`'s kind label ends in an exhaustive `switch` on `DayKind` with a `never` check, so `npm run build` flags `"halfDayOff"` at merge time; label it "pół dnia wolnego", not hatched, as S-02 does in the days table.

## Phase 1: Shared day row

### Overview

Move the per-day row construction of the month report into one `pub(super)` function so the month and the week share one code path. No behaviour change.

### Changes Required:

#### 1. Extract the day row

**File**: `src-tauri/src/work_time/report.rs`

**Intent**: Turn the body of the `for d in first.iter_days()…` loop in `compute` (`:145-168`) into a function that builds one `DayReport`, and call it from the loop. Make `days_off_between` reachable from `week.rs`.

**Contract**: `days_off_between` becomes `pub(super)` (signature unchanged). New function, placed right after `day_kind`, with a doc comment saying it is the one place a day's worked time, norm, deviation and kind are computed and that `entries` / `days_off` must cover `d` (a wider range is fine):

```rust
pub(super) fn day_report<Tz: TimeZone>(
    d: NaiveDate,
    today: NaiveDate,
    now: i64,
    tz: &Tz,
    entries: &[Entry],
    days_off: &BTreeSet<NaiveDate>,
    settings: &WorkTimeSettings,
) -> Result<DayReport, WorkTimeError>
```

`compute`'s loop becomes `days.push(day_report(d, today, now, tz, &entries, &days_off, settings)?)`. Import `super::Entry` next to `super::in_day`. `MonthReport`, `DayReport`, `classify`, `day_kind`, `month_report` and the month figures are otherwise untouched.

### Success Criteria:

#### Automated Verification:

- Rust tests pass with the month tests untouched: `cargo test` (in `src-tauri/`)
- Month tests not edited: `git diff --quiet HEAD -- src-tauri/src/work_time/norm_tests.rs src-tauri/src/work_time/tests.rs src-tauri/tests`
- Only `report.rs` changes in Rust: `git diff --name-only HEAD -- src-tauri` lists only `src-tauri/src/work_time/report.rs`
- One `DayReport` construction site: `rg -n "DayReport \{" src-tauri/src/work_time/report.rs` lists the struct definition and one literal inside `day_report`
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

#### Manual Verification:

- In the running app, "Dni miesiąca" and the month tiles of the current month show the same figures as before the refactor.

**Implementation Note**: Commit as `refactor(week-summary): shared day row for month and week (p1)`.

---

## Phase 2: Week report domain

### Overview

A new store-backed week report over `[monday, next_monday)` from one snapshot, built only from `report::day_report`, with unit tests on invented data.

### Changes Required:

#### 1. Week module

**File**: `src-tauri/src/work_time/week.rs` (new)

**Intent**: Compute the Monday–Sunday week containing `day`: its bounds, its phase relative to today, the seven day rows and the totals derived from them. No `tauri` import, no clock: `now` and the time zone are parameters (like `today.rs`).

**Contract**:

- `pub fn week_days(any_day: NaiveDate) -> (NaiveDate, NaiveDate)`: the Monday on or before `any_day` and the next Monday (half-open). Days in 1..=9999 only reach here (`parse_day`), so no overflow is possible; still use checked arithmetic with a saturating fallback like `norm::month_days` (`norm.rs:17-23`): `monday = any_day.checked_sub_days(Days::new(u64::from(any_day.weekday().num_days_from_monday()))).unwrap_or(NaiveDate::MIN)`, `next_monday = monday.checked_add_days(Days::new(7)).unwrap_or(NaiveDate::MAX)`. No `Result`.
- `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] WeekPhase { Past, Current, Future }` with `rename_all = "lowercase"`; `pub fn week_phase(monday: NaiveDate, today: NaiveDate) -> WeekPhase` compares `monday` with the Monday of `today` (`week_days(today).0`).
- `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] WeekReport`, `rename_all = "camelCase"`:
  - `monday: String`, `sunday: String` (`YYYY-MM-DD`, from the first and last day row);
  - `phase: WeekPhase`, `as_of_ms: i64`;
  - `worked_ms: i64` = `Σ day.worked_ms`;
  - `norm_ms: i64` = `Σ day.norm_ms` (whole week);
  - `norm_to_date_ms: Option<i64>`: past = `norm_ms`, current = `Σ day.norm_ms` for days `≤ today` (today counts in full), future = `None`;
  - `deviation_ms: Option<i64>` = `worked_ms − norm_to_date_ms` (`None` for future);
  - `remaining_ms: Option<i64>`: current only, `max(norm_ms − worked_ms, 0)`; `None` otherwise;
  - `days: Vec<DayReport>`: exactly seven, Monday first.
- `pub fn week_report<Tz: TimeZone>(store: &Store, day: NaiveDate, now: i64, tz: &Tz, settings: &WorkTimeSettings) -> Result<WeekReport, WorkTimeError>`: one `store.read` that calls `report::days_off_between(conn, monday, next_monday)` and `in_day(conn, week_start, week_end, now)` (bounds from `day_bounds` of Monday and next Monday), then `report::day_report` for each of the seven days (iterate the `NaiveDate`s `monday..next_monday`, and use those dates, not the row strings, for the `≤ today` filter) with `today = local_day(now, tz)`. The days-off result is bound without a type annotation and only passed on (see "Merge with S-02"). Never names `DAILY_NORM_MS`, 8 h, `norm::figures` or `BTreeSet`.

#### 2. Module registration

**File**: `src-tauri/src/work_time/mod.rs`

**Intent**: Register the new module and its tests.

**Contract**: `pub mod week;` in the module list (`:11-19`, alphabetical, after `today`) and `#[cfg(test)] mod week_tests;` in the test list (after `today_tests`). Nothing else changes.

#### 3. Tests

**File**: `src-tauri/src/work_time/week_tests.rs` (new)

**Intent**: Pin the week rule on invented data in `Europe/Warsaw`, robust to S-02.

**Contract**: Local helpers copied from `norm_tests.rs:28-56` (`t`, `d`, `open`, `add_entry` with a far-future `now`) plus `mark_off(store, day, settings)` (the only call of `report::set_day_off` in the file); a running entry is started with `start(&mut store, t("…"), &Warsaw)`; settings only through `WorkTimeSettings::default()` and `super::norm_tests::manual_only()`, never a `WorkTimeSettings { … }` literal. Expected norms are written as `n × workday_norm`, where `workday_norm` is the `norm_ms` of an ordinary `Workday` row of the same report, and every week is cross-checked row by row against `report::month_report` of each day's month; hour values below are the default-settings illustration only. Cases:

- `week_days` for every weekday of 23–29.11.2026 (Sunday maps to the Monday before), and across the year (Fri 1.01.2027 → Mon 28.12.2026); `week_phase` past / current / future, including today = Sunday and today = Monday.
- 23–29.11.2026, today Thu 26.11 noon: 7 rows, phase current, norm = 5 workdays (40 h), norm to date = 4 workdays (32 h), deviation = worked − norm to date, remaining = max(norm − worked, 0); Friday's deviation `None`.
- 30.11–6.12.2026 (crosses Nov→Dec): 5 workdays; rows equal November's and December's month rows.
- 21–27.12.2026 (default calendar): 24.12 and 25.12 `Holiday` with norm 0, 26.12 `Weekend` with `day_off_owed`; 3 workdays (24 h).
- 28.12.2026–3.01.2027 (crosses the year): 1.01.2027 `Holiday`; 4 workdays (32 h).
- 6–12.04.2026: Easter Monday `Holiday`; 4 workdays (32 h).
- 23–29.03.2026 and 19–25.10.2026 (DST Sundays): 5 workdays; an entry over the DST night counts its real length (e.g. 25.10 00:00–06:00 = 7 h), and `Σ day.worked_ms = worked_ms`.
- A manual day off under `manual_only()` (e.g. Wed 25.11.2026): `DayOff`, norm 0, week norm 4 workdays.
- A running entry started Sun 22.11 22:00, report at Mon 23.11 01:00: 1 h on Monday of the week 23–29.11, 2 h on Sunday of the (past) week 16–22.11.
- An entry Sun 29.11 22:00 – Mon 30.11 02:00 counts 2 h in each week; an entry across midnight inside a week counts in both days.
- A future week (7–13.12.2026 seen from 26.11): `norm_to_date_ms`, `deviation_ms`, `remaining_ms` and every row's deviation are `None`, `worked_ms` 0; a past week (16–22.11.2026): `norm_to_date_ms = norm_ms`, `remaining_ms` `None`.
- Serialized shape (the TS mirror in Phase 3 relies on it, like `norm_tests.rs:803-814`): `serde_json::to_value` of the current week 23–29.11.2026 has the keys `monday`, `sunday`, `phase` (`"current"`), `asOfMs`, `workedMs`, `normMs`, `normToDateMs`, `deviationMs`, `remainingMs`, `days` (7 entries); the future week 7–13.12.2026 has `"phase": "future"` and `normToDateMs`, `deviationMs`, `remainingMs` all `null`; the past week 16–22.11.2026 has `"phase": "past"` and `remainingMs` `null`.
- Proptest (24 cases, like `norm_tests.rs:566-616`): random closed entries in [23.11 00:00, 5.12 00:00) 2026 (inserted by SQL as the month proptest does), random manual days off over the two weeks, default or `manual_only()` settings, report at Sat 5.12.2026 12:00 (first week past, second current, no entry after `now`); for both weeks, 7 rows, `Σ worked = worked_ms`, `Σ norm = norm_ms`, `Σ deviation.unwrap_or(0) = deviation_ms` (non-future), and each row equals the month report's row for that day.

### Success Criteria:

#### Automated Verification:

- Rust tests pass, including the new `work_time::week_tests`: `cargo test` (in `src-tauri/`)
- No fixed norm in the week code: `rg -n "DAILY_NORM_MS|figures\(|3_600_000|28_800_000|28800000" src-tauri/src/work_time/week.rs` returns nothing
- No settings struct literal in the week tests: `rg -n "WorkTimeSettings \{" src-tauri/src/work_time/week.rs src-tauri/src/work_time/week_tests.rs` returns nothing
- `week.rs` imports nothing from `tauri`: `rg -n "use tauri|tauri::" src-tauri/src/work_time/week.rs` returns nothing
- Norm rules untouched: `git diff --quiet HEAD -- src-tauri/src/work_time/norm.rs src-tauri/src/work_time/report.rs`
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- No frontend change: `git diff --quiet HEAD -- src`
- No fixed norm or hand-built days-off set in the week tests: `rg -n "DAILY_NORM_MS|norm::figures|BTreeSet" src-tauri/src/work_time/week.rs src-tauri/src/work_time/week_tests.rs` returns nothing
- One day-off write path in the week tests: `rg -n "set_day_off" src-tauri/src/work_time/week_tests.rs` finds one line (inside `mark_off`)

No manual verification in this phase: nothing user-visible changes (the real app is checked in Phase 4).

**Implementation Note**: Commit as `feat(week-summary): week report in the work-time domain (p2)`.

---

## Phase 3: Command and contract

### Overview

Expose the week report to the UI as a read-only command with the month's settings policy, and mirror its type in TypeScript in the same commit.

### Changes Required:

#### 1. Command

**File**: `src-tauri/src/work_time/commands.rs`

**Intent**: A read-only command next to `work_month_report`, so the month and the week never disagree on the calendar or on an invalid `workTime` block.

**Contract**: `#[tauri::command] pub async fn work_week_report(app: AppHandle, day: String) -> Result<WeekReport, WorkTimeError>`: `report::parse_day(&day)?`, then `run_with_settings(&app, "week_report", Policy::Strict, move |store, now, settings| week::week_report(store, day, now, &Local, settings))`. No `notify` (it writes nothing). Import `super::week::{self, WeekReport}`.

#### 2. Registration

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the command.

**Contract**: One line `work_time::commands::work_week_report,` right after `work_time::commands::work_month_report,` in `generate_handler!` (`:213-222`). No capability change (custom commands need none).

#### 3. TS mirror and API

**File**: `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: Mirror `WeekReport` / `WeekPhase` and add the one typed function, in the same commit as the Rust type.

**Contract**:

- `types.ts`, appended after `MonthReport`: `export type WeekPhase = "past" | "current" | "future";` and `WeekReport` as a union discriminated on `phase`, over a base `{ monday: string; sunday: string; asOfMs: number; workedMs: number; normMs: number; days: DayReport[] }`:
  - `past`: `normToDateMs: number; deviationMs: number; remainingMs: null`;
  - `current`: `normToDateMs: number; deviationMs: number; remainingMs: number`;
  - `future`: `normToDateMs: null; deviationMs: null; remainingMs: null`.
- `api.ts`, after `workMonthReport`: `/** \`day\` is any \`YYYY-MM-DD\`; the report is the Monday–Sunday week containing it. */ export function workWeekReport(day: string): Promise<WeekReport>` invoking `work_week_report` with `{ day }`. Add `WeekReport` to the type import. `workTimeErrorMessage` needs no new case (no new error kind).

### Success Criteria:

#### Automated Verification:

- Rust tests pass: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- The command is registered and called once: `rg -n "work_week_report" src-tauri/src/lib.rs src-tauri/src/work_time/commands.rs src/worktime/api.ts` finds one line in each file
- No capability or config change: `git diff --quiet HEAD -- src-tauri/capabilities src-tauri/tauri.conf.json`

No manual verification in this phase: the UI does not call the command yet (the real app is checked in Phase 4).

**Implementation Note**: Commit as `feat(week-summary): work_week_report command and TS mirror (p3)`.

---

## Phase 4: "Bieżący tydzień" panel

### Overview

Show the week report as a panel in "Czas pracy" with loading, error and data states, and document it.

### Changes Required:

#### 1. Week hook

**File**: `src/worktime/useWeekReport.ts` (new)

**Intent**: Load the current week's report and keep it fresh, copying `useMonthReport`'s pattern (latest request wins, subscribe to `WORK_TIME_CHANGED` before the first load, refetch on focus and visibility, 60 s poll while visible, an error re-subscribes on the next poll and on retry).

**Contract**: `export function useWeekReport(): { report: WeekReportState; retry: () => void }` with `WeekReportState = { state: "loading" } | { state: "error"; message: string; lastOkAt: number | null } | { state: "loaded"; report: WeekReport }`. The file also exports `localDate(): string`, the local calendar date `YYYY-MM-DD` (never UTC), a copy of the formula of the private `localToday` in `WorkTimeView.tsx:17-28` (copied, not imported, so `WorkTimeView.tsx` keeps its two-line edit). Each request sends `localDate()` computed at request time, inside `refresh`; no per-key cache, so a loaded report stays shown while the next request runs. The poll runs in every phase (the current week always moves). Errors through `workTimeErrorMessage`.

#### 2. Panel component

**File**: `src/worktime/WeekSummary.tsx` (new)

**Intent**: The panel body: stat strip and seven day rows, from the report only; the UI never sums or knows a norm value.

**Contract**: `export function WeekSummary(props: { onPickDay: (day: string) => void })`. It calls `useWeekReport()` and reads `useWorkTimer()` (the shared context, no second subscription) for `todayTotalMs(status, nowMs)` (`./total`). `today = localDate()` is taken at render, as `DaysTable` gets `localToday()` (`WorkTimeView.tsx:134`); the panel re-renders on every report load and timer tick.

- `Panel title="Bieżący tydzień"`, `meta` = the date range when loaded (`DD.MM – DD.MM.YYYY`; both dates with the year when the years differ), `footer` = one muted line: "Tydzień od poniedziałku do niedzieli. Norma jak w miesiącu: dni robocze minus święta i dni wolne; odchylenie liczone do dziś włącznie." (no "8 h").
- Loading: `<p className="dn-loading wt-panel-body">Ładowanie…</p>`. Error: `ErrorState` titled "Nie udało się wczytać tygodnia", `lastSuccess` "ostatnio OK HH:MM", `onRetry` = hook retry.
- `StatStrip` (in `.wt-panel-body`), by phase like `MonthReport.tsx`: "Tydzień" (`formatHm(workedMs)`, sub `z ${formatHm(normMs)} normy`); "Odchylenie" (`formatSigned(deviationMs)`, sub `wobec ${formatHm(normToDateMs)} do dziś` / "tydzień zamknięty" for past, tone warn below 0 else info; omitted for future); "Zostało" (current only, `formatHm(remainingMs)`, sub "do normy tygodnia" or "norma tygodnia wyrobiona" at 0).
- `DataTable label="Bieżący tydzień" className="wt-week"`, head row "Dzień", "Na tle normy", "Suma", "Norma", "±". Each row: `DataTableRow current={row.day === today} hatched={dayOff || holiday}`; day label as a quiet `Button` (`formatDayLabel`) calling `onPickDay(row.day)`; `HoursBar` against `normMs` (same show rule as `DaysTable`) plus a kind label from a private `kindLabel(row: DayReport): string | null`, in the `.wt-days-holiday` span with the holiday `title` as in `DaysTable.tsx:46-51,82-86`:
  - `row.holiday !== null` → the holiday name on any kind of day, plus " · należy się inny dzień wolny" when `row.dayOffOwed`. A Saturday holiday (e.g. 26.12.2026) is `kind === "weekend"` with `dayOffOwed`, and a Sunday holiday is `weekend` too, so the name must not depend on `kind`.
  - otherwise an exhaustive `switch (row.kind)`: `workday` → `null`; `weekend` → "weekend"; `dayOff` → "dzień wolny"; `holiday` → `null` (never without a name); `default: { const unknown: never = row.kind; return unknown; }`, so a new `DayKind` (S-02's `"halfDayOff"`) fails `npm run build`.

  Then worked `H:MM` (or "—"), norm `H:MM` (or "—" at 0), deviation `formatSigned` with info/warn tone (or "—" when `null`). Today's row uses `liveFigures` while `phase === "current"` and the timer is loaded; weekend rows are muted.

#### 3. Shared live helper

**File**: `src/worktime/DaysTable.tsx`

**Intent**: Reuse the one live-today rule instead of a copy.

**Contract**: Add `export` to `function liveFigures` (`:14`). Nothing else changes.

#### 4. Layout

**File**: `src/worktime/WorkTime.css`

**Intent**: Column template for the week table, layout only.

**Contract**: New rules appended at the end of the file (existing rules untouched): `.wt-week { --dn-table-cols: 84px minmax(0, 1fr) 52px 52px 60px; }` and `.wt-week .dn-hours { min-width: 0; }`; cells may reuse `.wt-days-note`, `.wt-days-holiday` and `.wt-days-num`. Spacing only through tokens, no literal colours.

#### 5. View

**File**: `src/worktime/WorkTimeView.tsx`

**Intent**: Place the panel in the main column.

**Contract**: `import { WeekSummary } from "./WeekSummary";` and `<WeekSummary onPickDay={setDay} />` between the "Wybrany dzień" and "Dni miesiąca" panels. No other edit (in particular not `HowCounted`).

#### 6. Docs

**File**: `README.md`, `CHANGELOG.md`, `context/foundation/prd.md`

**Intent**: One user-facing line each, like S-03 and S-06; the PRD stops listing the week view as not built, the way the S-04 and S-07 plans treat their own items in the same sentence.

**Contract**: README "Czas pracy" sub-list (`README.md:55-59`): a bullet "a summary of the current week (Monday to Sunday, also across months) against the week's norm, day by day;"; and in the `workTime` paragraph (`README.md:125`) "month report and days table" becomes "month report, days table and week summary". CHANGELOG `[Unreleased]` → `### Added`: "\"Czas pracy\": a \"Bieżący tydzień\" panel shows the current Monday–Sunday week, also across a month or year boundary, day by day against the week's norm (the month's rule: holidays and days off count out), with the deviation up to today and what remains." PRD: in the paragraph under "### Rozważone i świadomie NIE wykluczone" (`prd.md:233-234`) delete only the words "widok tygodnia, " and add one new line after that paragraph: "Widok tygodnia jest zbudowany jako panel „Bieżący tydzień” w „Czas pracy” (MS-18), bez nowego FR; norma tygodnia to reguła FR-014." Touch no other words and add no FR (S-04 and S-07 edit the same sentence; on a merge conflict keep every slice's own removal and line).

### Success Criteria:

#### Automated Verification:

- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- No 8 h rule in the UI: `rg -n "28800000|28_800_000|8 \* 3600|8 \* 60 \* 60|norm = 8|norm \?\? 8" src/` returns nothing
- The timer is read only through the context: `rg -n "useWorkTimerSource" src/` finds only `useWorkTimer.ts` and `WorkTimerProvider.tsx`
- `invoke` stays in `api.ts`: `rg -n "@tauri-apps/api/core" src/worktime` finds only `api.ts`
- No literal colours in view CSS: `rg -n "#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(" src/worktime/WorkTime.css` returns nothing
- No Rust change in this phase: `git diff --quiet HEAD -- src-tauri`
- The PRD no longer lists the week view as not built and gains no FR: `rg -n "widok tygodnia," context/foundation/prd.md` returns nothing and `git diff HEAD -- context/foundation/prd.md | rg "^\+.*FR-0[0-9]{2}:"` returns nothing

#### Manual Verification:

- In the running app, "Bieżący tydzień" shows seven rows Monday to Sunday of the current week, today highlighted, and the head shows the week's dates.
- With the counter running, today's row (worked, bar, ±) grows live and matches the header's "Dziś"; the totals catch up within about 60 s or after STOP.
- Marking a weekday of this week as a day off in "Dni miesiąca" hatches it in the week panel, drops its norm to "—" and lowers "z H:MM normy" by one workday; unmarking restores it.
- A week with a holiday or a month boundary (check with the month chevrons that the week panel does not move, and on a real such week when available) shows all seven days and the holiday name.
- Clicking a day label in the week panel switches "Wybrany dzień" and the entries panel to that day.
- An invalid `workTime` block in `settings.json` shows the error state with retry in the week panel as in the month report; fixing it and retrying recovers.
- At 800 px window width the week table fits the column without clipped numbers.

**Implementation Note**: Commit as `feat(week-summary): current-week panel in Czas pracy (p4)`. The Konsola design-system artifact is not updated by this slice (manual follow-up for the user if wanted).

---

## Testing Strategy

### Unit Tests:

- `week_tests.rs`: week bounds and phase; the calendar cases (inside a month, across a month, Christmas, across the year, Easter Monday, both DST weeks, a manual day off); running timer and entries across midnight and across the week boundary; past / current / future figures; the serialized shape per phase; the proptest on day rows vs totals and vs month rows.
- Existing month tests stay unedited and green after the Phase 1 extraction.

### Integration Tests:

- None new: the command is a thin `run_with_settings` wrapper like `work_month_report`, and the store-backed function is tested on a real temp store.

### Manual Testing Steps:

1. Start the app on a work day, open "Czas pracy", check the week panel against the days table for the same dates.
2. START the counter, watch today's row in both tables move together; STOP and check the totals.
3. Toggle a day off on a weekday of this week and back.
4. Break the `workTime` block in `settings.json`, check both error states, restore.

## Performance Considerations

One extra read per refresh: seven days of `days_off` and the week's entries, the same order of work as a month report on a quarter of the range. The 60 s poll matches the month hook.

## Migration Notes

No schema change and no settings change. Rollback is a plain revert of the four commits.

## Decisions

User decisions (binding, given with the task):

- **D1.** A section (panel) in the "Czas pracy" view, not a separate view or navigation item.
- **D2.** The week is Monday–Sunday and always complete, also across a month or year boundary (e.g. Mon 30.11 – Sun 6.12).
- **D3.** The week norm uses exactly the month's rule (holidays from S-01, manual days off), computed in Rust and unit-tested; the UI only displays.
- **D4.** Anchored as MS-18; no new FR in the PRD; the week reuses the FR-014 rule.

Agent defaults (no user was reachable; default chosen by agent, user may revise):

- **A1.** Only the current week (containing today), independent of the month and day the view shows; no prev/next week navigation in this slice. The command takes a day, so navigation can be added later without a contract change.
- **A2.** One shared `report::day_report` used by the month and the week (research option 4, extraction), kept as a mechanical, separately committed refactor; week logic in a new `week.rs`.
- **A3.** Week totals only from the day rows (`Σ norm`, `Σ worked`); never `norm::figures`, `DAILY_NORM_MS` or a literal 8 h in new code.
- **A4.** Figures mirror the month: worked, whole-week norm, norm to date (today in full), deviation against the norm to date, remaining to the whole-week norm (current week only); no "średnio / dzień".
- **A5.** Own `WeekPhase` (past / current / future, by Monday) in `week.rs` instead of reusing `MonthPhase`, so `norm.rs` is not touched.
- **A6.** Command `work_week_report(day)` under `Policy::Strict`, like the month report.
- **A7.** Panel "Bieżący tydzień" in the main column between "Wybrany dzień" and "Dni miesiąca"; stat strip plus a seven-row table (Dzień, Na tle normy, Suma, Norma, ±); kind label for non-working days; a day label click picks the day.
- **A8.** Only today's row is live (shared `liveFigures`, exported from `DaysTable.tsx`); the week totals refresh with the report, as the month tiles do.
- **A9.** The hook computes the local date at each request (handles midnight without a re-render) and polls every 60 s.
- **A10.** The panel explains itself in a footer line without "8 h"; `HowCounted` is left to S-02.
- **A11.** Tests: settings only through `WorkTimeSettings::default()` / `manual_only()`; expected norms as multiples of a workday row's norm plus a row-by-row cross-check against the month report, so S-02's configurable norm does not break them.
- **A12.** README and CHANGELOG get one line each in Phase 4 (S-03 / S-06 precedent); the design-system artifact is a manual follow-up.
- **A13.** The roadmap item is not moved to `planning` by this run (the task forbids editing `context/foundation/roadmap.md`).
- **A14.** (added in plan review) Phase 4 removes "widok tygodnia" from the PRD's "considered but not built" sentence (`prd.md:234`) and adds one line saying it is built as the "Bieżący tydzień" panel (MS-18, no new FR), as the S-04 and S-07 plans do for their items. No FR is added or changed.
- **A15.** (added in plan review) The kind label shows the holiday name whenever the row has one (a Saturday or Sunday holiday is `weekend`), and falls back to an exhaustive `DayKind` switch otherwise.

## References

- Research: `context/changes/week-summary/research.md`
- Month report and day row: `src-tauri/src/work_time/report.rs:67-89`, `:101-119`, `:122-194`
- Norm rules: `src-tauri/src/work_time/norm.rs:14`, `:33-92`, `:113-152`
- Reads and bounds: `src-tauri/src/work_time/mod.rs:95-100`, `:131-153`; `src-tauri/src/work_time/rules.rs:33-63`; precedent `src-tauri/src/work_time/today.rs:23-40`
- Commands: `src-tauri/src/work_time/commands.rs:71-78`, `:213-223`; `src-tauri/src/lib.rs:213-222`
- Tests: `src-tauri/src/work_time/norm_tests.rs:22-27`, `:483-532`, `:566-616`; `src-tauri/src/work_time/tests.rs:7`
- UI: `src/worktime/WorkTimeView.tsx:89-111`, `:198-231`; `src/worktime/DaysTable.tsx:14-26`, `:28-122`; `src/worktime/MonthReport.tsx:8-49`; `src/worktime/useMonthReport.ts:6-119`; `src/worktime/types.ts:79-142`; `src/worktime/api.ts:51-52`; `src/worktime/useWorkTimer.ts:138-150`
- Parallel plans (read-only, on their branches): S-02 `slice/configurable-daily-norm:context/changes/configurable-daily-norm/plan.md`; S-07 `slice/end-of-day-summary:context/changes/end-of-day-summary/plan.md` (PRD sentence, its `:326`)
- Prior changes: `context/archive/2026-11-26-public-holidays-norm/plan.md`, `context/archive/2026-11-24-worktime-days-table/plan.md`, `context/archive/2026-11-26-tray-timer-control/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Shared day row

#### Automated

- [x] 1.1 Rust tests pass with the month tests untouched: `cargo test` (in `src-tauri/`) — 8e8d694
- [x] 1.2 Month tests not edited: `git diff --quiet HEAD -- src-tauri/src/work_time/norm_tests.rs src-tauri/src/work_time/tests.rs src-tauri/tests` — 8e8d694
- [x] 1.3 Only `report.rs` changes in Rust: `git diff --name-only HEAD -- src-tauri` lists only `src-tauri/src/work_time/report.rs` — 8e8d694
- [x] 1.4 One `DayReport` construction site: `rg -n "DayReport \{" src-tauri/src/work_time/report.rs` lists the struct definition and one literal inside `day_report` — 8e8d694
- [x] 1.5 Formatting passes: `cargo fmt --check` (in `src-tauri/`) — 8e8d694
- [x] 1.6 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`) — 8e8d694
- [x] 1.7 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 8e8d694

#### Manual

- [ ] 1.8 In the running app, "Dni miesiąca" and the month tiles of the current month show the same figures as before the refactor.

### Phase 2: Week report domain

#### Automated

- [x] 2.1 Rust tests pass, including the new `work_time::week_tests`: `cargo test` (in `src-tauri/`)
- [x] 2.2 No fixed norm in the week code: `rg -n "DAILY_NORM_MS|figures\(|3_600_000|28_800_000|28800000" src-tauri/src/work_time/week.rs` returns nothing
- [x] 2.3 No settings struct literal in the week tests: `rg -n "WorkTimeSettings \{" src-tauri/src/work_time/week.rs src-tauri/src/work_time/week_tests.rs` returns nothing
- [x] 2.4 `week.rs` imports nothing from `tauri`: `rg -n "use tauri|tauri::" src-tauri/src/work_time/week.rs` returns nothing
- [x] 2.5 Norm rules untouched: `git diff --quiet HEAD -- src-tauri/src/work_time/norm.rs src-tauri/src/work_time/report.rs`
- [x] 2.6 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [x] 2.7 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [x] 2.8 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [x] 2.9 No frontend change: `git diff --quiet HEAD -- src`
- [x] 2.10 No fixed norm or hand-built days-off set in the week tests: `rg -n "DAILY_NORM_MS|norm::figures|BTreeSet" src-tauri/src/work_time/week.rs src-tauri/src/work_time/week_tests.rs` returns nothing
- [x] 2.11 One day-off write path in the week tests: `rg -n "set_day_off" src-tauri/src/work_time/week_tests.rs` finds one line (inside `mark_off`)

### Phase 3: Command and contract

#### Automated

- [ ] 3.1 Rust tests pass: `cargo test` (in `src-tauri/`)
- [ ] 3.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- [ ] 3.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- [ ] 3.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 3.5 The command is registered and called once: `rg -n "work_week_report" src-tauri/src/lib.rs src-tauri/src/work_time/commands.rs src/worktime/api.ts` finds one line in each file
- [ ] 3.6 No capability or config change: `git diff --quiet HEAD -- src-tauri/capabilities src-tauri/tauri.conf.json`

### Phase 4: "Bieżący tydzień" panel

#### Automated

- [ ] 4.1 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 4.2 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- [ ] 4.3 No 8 h rule in the UI: `rg -n "28800000|28_800_000|8 \* 3600|8 \* 60 \* 60|norm = 8|norm \?\? 8" src/` returns nothing
- [ ] 4.4 The timer is read only through the context: `rg -n "useWorkTimerSource" src/` finds only `useWorkTimer.ts` and `WorkTimerProvider.tsx`
- [ ] 4.5 `invoke` stays in `api.ts`: `rg -n "@tauri-apps/api/core" src/worktime` finds only `api.ts`
- [ ] 4.6 No literal colours in view CSS: `rg -n "#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(" src/worktime/WorkTime.css` returns nothing
- [ ] 4.7 No Rust change in this phase: `git diff --quiet HEAD -- src-tauri`
- [ ] 4.15 The PRD no longer lists the week view as not built and gains no FR: `rg -n "widok tygodnia," context/foundation/prd.md` returns nothing and `git diff HEAD -- context/foundation/prd.md | rg "^\+.*FR-0[0-9]{2}:"` returns nothing

#### Manual

- [ ] 4.8 In the running app, "Bieżący tydzień" shows seven rows Monday to Sunday of the current week, today highlighted, and the head shows the week's dates.
- [ ] 4.9 With the counter running, today's row (worked, bar, ±) grows live and matches the header's "Dziś"; the totals catch up within about 60 s or after STOP.
- [ ] 4.10 Marking a weekday of this week as a day off in "Dni miesiąca" hatches it in the week panel, drops its norm to "—" and lowers "z H:MM normy" by one workday; unmarking restores it.
- [ ] 4.11 A week with a holiday or a month boundary (check with the month chevrons that the week panel does not move, and on a real such week when available) shows all seven days and the holiday name.
- [ ] 4.12 Clicking a day label in the week panel switches "Wybrany dzień" and the entries panel to that day.
- [ ] 4.13 An invalid `workTime` block in `settings.json` shows the error state with retry in the week panel as in the month report; fixing it and retrying recovers.
- [ ] 4.14 At 800 px window width the week table fits the column without clipped numbers.
