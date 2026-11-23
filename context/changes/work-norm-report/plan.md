# Work-Norm Report Implementation Plan

## Overview

Roadmap slice S-09 (FR-014): the user sees how far the month is from the work norm. The norm is 8 h × workdays, where workdays are Monday–Friday minus days the user marks as off. A month report at the top of the "Czas pracy" view shows:
- the month's worked total;
- the deviation to date, with today counted in full;
- the average needed per workday after today.

Past months stay viewable. Days off are marked with a "Dzień wolny" checkbox on the day chosen in that view. The always-visible header shows a compact month deviation next to "Dziś".

All rules live in the Rust `work_time` module and are unit-tested on invented months. The UI only displays results.

## Current State Analysis

- **Work-time domain is ready to extend.** `src-tauri/src/work_time/mod.rs` keeps entries as UTC epoch-ms instants and computes day totals from `rules::portion` and `rules::day_bounds` (`rules.rs:43-63`). It takes `now` and the time zone as parameters (`mod.rs:5-9`). `in_day(conn, start, end, now)` (`mod.rs:105-127`) already works for any `[start, end)` interval, so it also works for a month.
- **The header status is today-only.** `WorkTimerStatus` (`mod.rs:34-43`) has `today_closed_ms` and `running_from_ms`. The UI's live total is `todayClosedMs + (now − runningFromMs)`, capped at `dayEndMs` (`src/worktime/WorkTimerHeader.tsx:26-31`).
- **The day view exists.** `src/worktime/WorkTimeView.tsx` has the ◀ ▶ day picker (`:210-225`), the entry list and the day "Razem" total (`:269-271`). It refetches on `work-time-changed` through `useWorkEntries` (`src/worktime/useWorkEntries.ts:50-80`).
- **No days off anywhere.** No table, type or setting for them exists. The store has migrations 1 (`work_entries`) and 2 (`own_tasks`) (`src-tauri/src/store/schema.rs:9-42`). S-10 (timer reminders) will reuse the workday rule and the days-off list (`context/foundation/roadmap.md:231`).
- **Commands pattern.** `work_time/commands.rs` wraps each domain call in `run()` (blocking pool, `with_store`, error kind logged) and emits `work-time-changed` after a successful write (`commands.rs:33-63`). `WorkTimeError` (`work_time/error.rs:10-33`) carries no times or row values. Its TS mirror and Polish messages live in `src/worktime/types.ts:8-18` and `src/worktime/api.ts:71-102`.
- **Tests.** `work_time/tests.rs` uses `chrono_tz::Europe::Warsaw` and an invented "now" of Monday 2026-11-23 12:00 (`tests.rs:14-28`). `tests/work_time.rs` and `tests/work_time_crash.rs` cover reopening and hard kills via `tests/common/crash.rs` (`run_writer_until`, `KillAt`).

## Desired End State

- "Czas pracy" starts with a month report for the month of the chosen day.
  - **Current month:** the month's worked total, the norm (workdays and hours), the deviation to date, the month's days off, and how much is needed per remaining workday. The deviation is signed, e.g. "−4:00", with today's 8 h included when today is a workday. The needed figure is the average over the workdays after today. On the last workday it shows the remaining shortfall instead, and once the norm is met it says so.
  - **Past month:** the final worked total and final deviation, with no needed figure.
  - **Future month:** only the norm and the days off.
- The chosen weekday shows a "Dzień wolny" checkbox. Ticking it removes that day from the norm in both the report and the header. Weekends never show it. Hours worked on a day off or a weekend still count as worked.
- The header shows "Dziś H:MM:SS" plus a compact month deviation (e.g. "Miesiąc −4:00"). While the counter runs, the deviation moves with it.
- A store failure shows `ErrorState` (view) or the header's existing error, never a zero report.
- **Worked example (invented):** November 2026, day off 2026-11-11, now Monday 2026-11-23 12:00. The month has 20 workdays, so the norm is 160:00. Norm to date: 15 workdays → 120:00. Workdays after today: 5. Worked 116:00, so the deviation is **−4:00** and the needed figure is (160 − 116) / 5 = **8:48 per day**.
- Verify: `cargo test` covers the rules (worked example, month edges, DST months, days off, weekends, rounding) plus reopen and crash tests. Then run the manual steps per phase.

### Key Discoveries:

- `src-tauri/src/work_time/rules.rs:59-63`: `portion(start, end, now, lo, hi)` clips any entry to any interval. Month totals use `[local midnight of the 1st, local midnight of the next 1st)`, so DST months (2026-10, 2027-03) need no special case.
- `src-tauri/src/work_time/mod.rs:159`: `status` runs one `store.read`. The month figures must be computed inside the same closure, so "Dziś" and the deviation come from one snapshot.
- `src-tauri/src/work_time/commands.rs:83-100`: START/STOP emit first and read status second. A day-off write returns `()` and emits after a successful write, like add/update/remove.
- `src/worktime/useWorkTimer.ts:37-58`: latest-request-wins guard plus subscribe-before-load. Reuse it for the month report hook.

## What We're NOT Doing

- No configurable norm hours, half days off, or extra working Saturdays. The norm is a fixed 8 h per workday (PRD FR-014). Days off are whole days, and workdays are only Mon–Fri minus days off.
- No holiday calendar, imported holidays, or vacation ranges (PRD Non-Goals). Days off are toggled one day at a time.
- No timer reminders (S-10). It will reuse `is_workday` and the days-off table from this slice.
- No export, e-mail or `Timeline.md` output. No reading of old records (Helper App, `Management/Timeline.md`).
- No month calendar grid and no separate "Raport" view.
- No per-second ticking of the "needed per day" figure. The report refreshes once a minute while it shows the current month, because it shows minutes.
- No date library in the UI and no `settings.json` changes. Days off are app data, kept in the store.

## Implementation Approach

A pure `work_time/norm.rs` module holds the calendar and norm rules: workdays of a month, month phase relative to today, and the figures (norm, norm to date, deviation, workdays after today, needed per day rounded up to the minute, remaining shortfall). A `work_time/report.rs` module reads entries and days off from the store and calls those rules. It also owns the day-off toggle and `is_workday`. `status()` gains the current month's deviation "as of" its `now`. The header extrapolates that value by elapsed running time, the same way it already extrapolates "Dziś".

Days off are one new table (migration 3). Toggling is idempotent and emits the existing `work-time-changed` event, so the header, the day list and the report refetch together. No new event is needed.

## Critical Implementation Details

- **Today counts in full (decided).** Norm to date = 8 h × workdays in `[1st, today]`, including today when it is a workday. "Workdays after today" = workdays in `(today, last day]`. Needed per day = `ceil_to_minute(max(0, norm − worked) / after)` when `after > 0`. When `after == 0`, the report shows `remaining = max(0, norm − worked)` instead. "Today" is the local day of `now` in the given time zone, never the chosen day.
- **Header extrapolation.** `WorkTimerStatus.monthDeviationMs` is computed at `asOfMs` (= the status's `now`). The header shows `monthDeviationMs + (running ? max(0, min(nowMs, dayEndMs) − asOfMs) : 0)`. Capping at `dayEndMs` matters: after midnight the day-end refetch adds the new day's 8 h, and the extrapolation must not keep counting into it.

## Phase 1: Norm domain and days-off store

### Overview

Add the `days_off` table, the pure norm rules, the month report, the day kind for the day view, the header deviation, and the day-off toggle, all in the tauri-free `work_time` module. Unit tests cover the rules. Integration tests cover reopening and a hard kill.

### Changes Required:

#### 1. Migration 3

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append the days-off table. One row per day off, keyed by local date, so a day cannot be marked twice.

**Contract**: `Migration { version: 3, sql }` with a `// S-09` comment, like the others:

```sql
CREATE TABLE days_off (
  day           TEXT PRIMARY KEY
                CHECK (day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  created_at_ms INTEGER NOT NULL
) WITHOUT ROWID;
```

Un-marking deletes the row. A day off is a flag, not a user record (entries keep their soft removal).

#### 2. Pure norm rules

**File**: `src-tauri/src/work_time/norm.rs` (new)

**Intent**: Every calendar and norm rule as a pure function on `NaiveDate`s and milliseconds: no store, no clock, no time zone.

**Contract**:
- `pub const DAILY_NORM_MS: i64 = 8 * 3_600_000;`
- `month_days(any_day) -> (first: NaiveDate, next_first: NaiveDate)`
- `is_weekend(day) -> bool` (Saturday, Sunday)
- `workdays(first, next_first, days_off: &BTreeSet<NaiveDate>) -> Vec<NaiveDate>`: Mon–Fri not in `days_off`
- `MonthPhase { Past, Current, Future }`, serde `rename_all = "lowercase"`, from comparing the month with `today`'s month
- `NormFigures { norm_ms, norm_to_date_ms: Option, deviation_ms: Option, workdays: u32, workdays_after_today: Option<u32>, needed_per_day_ms: Option, remaining_ms: Option }`, from `figures(workdays, today, phase, worked_ms)`:
  - Past: norm to date = norm, deviation = worked − norm. The after-today, needed and remaining fields are `None`.
  - Current: everything per "Critical Implementation Details". `needed_per_day_ms` is `Some` only when `after > 0`. `remaining_ms` is always `Some`.
  - Future: only `norm_ms` and `workdays`.
- `ceil_to_minute(ms) -> i64`

#### 3. Store-backed report and day-off toggle

**File**: `src-tauri/src/work_time/report.rs` (new), `src-tauri/src/work_time/mod.rs`, `src-tauri/src/work_time/error.rs`

**Intent**: Read the days off and the month's entries in one `store.read` and turn them into a `MonthReport`. Toggle a day off in one `store.write`. Extend the existing status and day listing with what the UI needs.

**Contract**:
- `MonthReport` (serde camelCase): `month: "YYYY-MM"`, `phase: MonthPhase`, `asOfMs`, `workedMs` (counted entries clipped to the month bounds, the running one up to `now`), plus the flattened `NormFigures` fields, and `daysOff: Vec<String>` (`YYYY-MM-DD`, sorted, this month only).
- `month_report<Tz>(store, day: NaiveDate, now, tz) -> Result<MonthReport, WorkTimeError>`. Month bounds come from `day_bounds(first).0` and `day_bounds(next_first).0`, and entries come from `in_day` with those bounds. "Today" is `local_day(now, tz)`.
- `set_day_off(store, day: NaiveDate, off: bool, now) -> Result<(), WorkTimeError>`. It is idempotent (`INSERT OR IGNORE` / `DELETE`). A weekend day is refused as the new `WorkTimeError::NotAWorkday`, which carries no detail and so keeps the no-values invariant.
- `is_workday(store, day) -> Result<bool, WorkTimeError>`, public for S-10.
- `WorkTimerStatus` gains `month_deviation_ms: i64` (current month, as of `now`, computed inside the existing `status` read) and `as_of_ms: i64`.
- `DayEntries` gains `day_kind: DayKind` (`Workday | DayOff | Weekend`, serde `rename_all = "camelCase"`).
- `mod.rs` declares `pub mod norm; pub mod report;`. `commands.rs::kind()` maps `NotAWorkday`.

#### 4. Tests

**File**: `src-tauri/src/work_time/tests.rs` (extend, or a sibling `norm_tests.rs`), `src-tauri/tests/work_time.rs`, `src-tauri/tests/days_off_crash.rs` (new)

**Intent**: Prove the decided semantics on invented months, and prove that days off survive reopening and a hard kill like the entries do.

**Contract**:
- Unit, pure (`norm.rs`):
  - November 2026 has 21 workdays (168:00), or 20 (160:00) with 2026-11-11 off.
  - Weekends are never workdays, even if passed in `days_off`.
  - Phase by month: October 2026 is past, November current, December future, relative to today 2026-11-23.
  - Rounding: 43:59 / 5 → 8:48.
- Unit, against the store, in `Europe/Warsaw` with "now" 2026-11-23 12:00:
  - The worked example from "Desired End State" gives deviation −4:00, needed 8:48 and 5 days after today.
  - Last workday (now 2026-11-30 12:00): `workdays_after_today = 0`, needed `None`, remaining = norm − worked.
  - Norm met: remaining 0 and needed 0.
  - Today marked off: today drops out of norm to date and the deviation rises by 8:00.
  - Weekend "today" (2026-11-28): norm to date counts only the days before it.
  - Hours on a day off or a Saturday count in `workedMs`.
  - An entry 2026-10-31 22:00 → 2026-11-01 02:00 counts 2:00 in each month.
  - DST: October 2026 and March 2027 month totals from entries across 2026-10-25 and 2027-03-28 match their real lengths.
  - A running entry started the previous month counts only its part in this month.
  - A past month (October) has its final deviation and no needed figure. A future month (December, 23 workdays) has only the norm.
  - The status `monthDeviationMs` equals the report's deviation for the same `now`.
  - `set_day_off` is idempotent both ways, a weekend is refused with `NotAWorkday` and no row, and `DayEntries.dayKind` covers all three kinds.
- Integration `tests/work_time.rs`: days off survive dropping and reopening the store.
- Crash `tests/days_off_crash.rs`: a child process marks distinct invented weekdays off, printing `committed <n>`, and is killed via `KillAt` like `tests/work_time_crash.rs`. After reopening: `quick_check` is ok, rows ≥ the last reported commit, and no weekend rows.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the new unit tests, `tests/work_time.rs` and `tests/days_off_crash.rs`
- Unit tests cover the worked example (−4:00, 8:48, 5 days), last workday, norm met, today off, weekend today, work on a day off and Saturday, the month-boundary entry, both DST months, a running entry from the previous month, past and future phases, and rounding up to the minute
- Unit tests prove `set_day_off` is idempotent, refuses a weekend with `NotAWorkday` and leaves no row
- A test proves the status `monthDeviationMs` equals the report deviation for the same `now`
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass

**Implementation Note**: This phase has no manual verification. Once all automated verification passes, continue to Phase 2. Phase blocks use plain bullets — the corresponding `- [ ]` checkboxes for these items live in the `## Progress` section at the bottom of the plan.

---

## Phase 2: Commands and TS contract

### Overview

Expose the report and the toggle as async commands. Mirror the extended Rust types in TypeScript and add the typed API functions and the Polish message for the new error.

### Changes Required:

#### 1. Commands

**File**: `src-tauri/src/work_time/commands.rs`, `src-tauri/src/lib.rs` (`generate_handler!`)

**Intent**: Thin wrappers over `report.rs` through the existing `run()` helper. The toggle notifies listeners after a successful write.

**Contract**:
- `work_month_report(day: String /* YYYY-MM-DD, any day of the month */) -> MonthReport`. A bad date is `InvalidLocalTime` (as in `list_work_entries`).
- `set_day_off(day: String, off: bool) -> ()`. It emits `work-time-changed` on success.
- Both return `Result<_, WorkTimeError>`.

#### 2. TypeScript mirrors and API

**File**: `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: Change the Rust types and their TS mirrors in the same commit (AGENTS.md). `api.ts` stays the only file in the folder that imports `invoke`.

**Contract**:
- Types:
  - `MonthPhase = "past" | "current" | "future"`
  - `MonthReport`, with the optional figures as `number | null`
  - `DayKind = "workday" | "dayOff" | "weekend"`
  - `WorkTimerStatus` gains `monthDeviationMs` and `asOfMs`
  - `DayEntries` gains `dayKind`
  - The `WorkTimeError` union gains `{ kind: "NotAWorkday" }`
- API: `workMonthReport(day)` and `setDayOff(day, off)`. `workTimeErrorMessage` gains `NotAWorkday` → "Weekend nie jest dniem roboczym".

### Success Criteria:

#### Automated Verification:

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass
- `npm run build`, `npm run lint` and `npm run format:check` pass

#### Manual Verification:

- In `npm run tauri dev`, calling `work_month_report` and `set_day_off` from the devtools console (`window.__TAURI_INTERNALS__.invoke`) returns the report, toggles a weekday, refuses a Saturday, and fires `work-time-changed`

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Report UI and header deviation

### Overview

Show the month report at the top of "Czas pracy" for the chosen day's month, add the "Dzień wolny" checkbox for the chosen weekday, and add the compact month deviation to the header.

### Changes Required:

#### 1. Formatting helpers

**File**: `src/worktime/format.ts`

**Intent**: Display helpers only, no rules.

**Contract**:
- `formatHm(ms)` → `H:MM`, floored to whole minutes.
- `formatSigned(ms)` → `+H:MM` / `−H:MM` (U+2212), rounded toward zero to whole minutes, and plain `0:00` under one minute.
- `formatMonth("YYYY-MM")` → Polish month name and year, via `toLocaleDateString("pl-PL", { month: "long", year: "numeric" })`.

#### 2. Month report hook

**File**: `src/worktime/useMonthReport.ts` (new)

**Intent**: Load the report for a month and keep it fresh like the header does, so a failure is visible and never reads as zeros.

**Contract**:
- `useMonthReport(day: string)` keys on the month (`day.slice(0, 7)`), so stepping days within a month does not refetch.
- It returns `loading | error {message, lastOkAt} | loaded {report}` plus `retry`.
- It subscribes to `work-time-changed` before the first load, with the latest-request-wins guard (`useWorkTimer.ts:30-58`).
- It refetches on `visibilitychange` → visible, on window `focus`, and every 60 s while `report.phase === "current"`, whether or not the counter runs. That covers the running counter and also midnight: today's 8 h join the norm to date, and on the 1st the shown month turns `past`. Both happen within a minute, so the report never disagrees with the header, which refetches at the day end on its own.

#### 3. Report section and day-off checkbox

**File**: `src/worktime/MonthReport.tsx` (new), `src/worktime/WorkTimeView.tsx`, `src/worktime/useWorkEntries.ts` (expose a `setDayOff` action with busy state), `src/App.css`

**Intent**: Render the decided report and the toggle. Loading, error (`ErrorState` with the last OK time) and data are separate states, as AGENTS.md requires.

**Contract**:
- Section heading: "Miesiąc: listopad 2026".
- Current month shows these lines:
  - "Przepracowano 116:00 z 160:00 (20 dni roboczych)"
  - "Odchylenie do dziś −4:00"
  - exactly one of these, checked in this order:
    1. `remainingMs === 0` → "Norma miesiąca wyrobiona" (even when `neededPerDayMs` is `0`)
    2. `workdaysAfterToday === 0` → "Do wyrobienia normy brakuje 6:00"
    3. otherwise → "Średnio 8:48 dziennie przez 5 dni roboczych"
- Past month: "Przepracowano …", "Odchylenie −2:30 (miesiąc zamknięty)".
- Future month: "Norma 184:00 (23 dni robocze)".
- Days off: "Dni wolne: 11.11" as buttons that move the day picker to that day, or "Brak dni wolnych".
- The "Dzień wolny" checkbox sits under the day picker. It is shown when `dayKind !== "weekend"`, checked when `dayKind === "dayOff"`, and disabled while saving. A refusal shows inline via `workTimeErrorMessage`.
- The report sits above the day list and keeps its own error state, separate from the entries.

#### 4. Header deviation

**File**: `src/worktime/WorkTimerHeader.tsx`, `src/App.css`

**Intent**: Answer "work longer or shorter?" without a view switch.

**Contract**: Next to "Dziś", show `Miesiąc {formatSigned(dev)}` with `dev = status.monthDeviationMs + (running ? max(0, min(nowMs, dayEndMs) − asOfMs) : 0)`. It uses `title="Odchylenie miesiąca od normy (8 h × dni robocze do dziś)"`. Header error behaviour stays as it is: the error message, never a zero deviation.

### Success Criteria:

#### Automated Verification:

- `npm run build`, `npm run lint`, `npm run format:check` pass
- `cargo test` still passes

#### Manual Verification:

- With invented entries this month, the report's worked total equals the sum of the days' "Razem" totals, and the deviation matches a hand count (8 h × workdays up to and including today)
- Ticking "Dzień wolny" on a past weekday raises the deviation by 8:00 in the report and the header at once; unticking restores it; a Saturday shows no checkbox
- With the counter running, the header deviation moves with "Dziś", and the report's figures update within a minute
- Stepping the day picker into the previous month shows its final deviation with "(miesiąc zamknięty)" and no needed figure; the next month shows only the norm
- Marking a future weekday off (vacation) lowers the "Średnio … dziennie" figure's day count by one
- After midnight (or with the clock set forward), with the counter stopped, the header deviation drops by 8:00 on a workday without interaction, and the report follows within a minute
- With the store failing, the report shows `ErrorState` and the header shows its error, never zeros

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Pure rules in `norm.rs`:
  - workday counts with and without days off
  - weekends ignored even when listed as days off
  - month phase
  - `ceil_to_minute`
- Store-backed rules in `Europe/Warsaw` with invented instants:
  - the worked example
  - last workday, norm met
  - today off, weekend today
  - work on days off and weekends
  - the month-boundary entry, DST months
  - a running entry from the previous month
  - past and future phases
  - status and report agreement
  - idempotent toggle, weekend refusal

### Integration Tests:

- `tests/work_time.rs`: days off survive dropping and reopening the store.
- `tests/days_off_crash.rs`: hard kills during day-off writes leave a consistent store with no lost reported commit.

### Manual Testing Steps:

1. Run the Phase 2 console calls.
2. Run the Phase 3 steps: hand-count check, day-off toggle, running counter, month browsing, vacation day, midnight, failing store.
3. Check that the Health view still reports "Dane aplikacji" as OK after migration 3 ran.

## Performance Considerations

A month is at most a few hundred entries and about 23 days-off rows. The report is one range scan on `work_entries_start` plus a primary-key range on `days_off`. Status now reads the month instead of the day on each call, which is still trivial. The extra IPC is one report call per minute, only while the "Czas pracy" view shows the current month. The "Lekkość w tle" NFR is unaffected.

## Migration Notes

- Migration 3 sits on top of migrations 1–2. An existing `day-navigator.db` is upgraded on the next open, after the daily backup (F-01 open order).
- An older build refuses the upgraded database as `NewerSchema`, by design. Rollback is the documented restore from `backups/` (AGENTS.md "App Data Store").
- S-10 should call `report::is_workday` and read `days_off` rather than define its own workday rule.

## References

- Roadmap item: `context/foundation/roadmap.md` (S-09, lines 209–219; S-10 coupling, line 231)
- PRD: `context/foundation/prd.md` FR-014 (line 152), Business Logic "Norma" (line 195), Success Criteria (line 65)
- Work-timer plan this extends: `context/archive/2026-11-23-work-timer/plan.md`
- Domain: `src-tauri/src/work_time/mod.rs:153-223`, `src-tauri/src/work_time/rules.rs:43-63`
- Commands: `src-tauri/src/work_time/commands.rs:33-100`
- UI: `src/worktime/WorkTimeView.tsx`, `src/worktime/WorkTimerHeader.tsx`, `src/worktime/useWorkTimer.ts:30-111`
- Crash harness: `src-tauri/tests/common/crash.rs`, example `src-tauri/tests/work_time_crash.rs`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Norm domain and days-off store

#### Automated

- [x] 1.1 `cargo test` passes in `src-tauri/`, including the new unit tests, `tests/work_time.rs` and `tests/days_off_crash.rs` — 1146ef5
- [x] 1.2 Unit tests cover the worked example, last workday, norm met, today off, weekend today, work on day off/Saturday, month-boundary entry, both DST months, running entry from previous month, past/future phases and rounding — 1146ef5
- [x] 1.3 Unit tests prove `set_day_off` is idempotent, refuses a weekend with `NotAWorkday` and leaves no row — 1146ef5
- [x] 1.4 A test proves status `monthDeviationMs` equals the report deviation for the same `now` — 1146ef5
- [x] 1.5 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass — 1146ef5

### Phase 2: Commands and TS contract

#### Automated

- [x] 2.1 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` pass — 74aabae
- [x] 2.2 `npm run build`, `npm run lint` and `npm run format:check` pass — 74aabae

#### Manual

- [ ] 2.3 `work_month_report` and `set_day_off` from the devtools console return the report, toggle a weekday, refuse a Saturday and fire the event

### Phase 3: Report UI and header deviation

#### Automated

- [x] 3.1 `npm run build`, `npm run lint`, `npm run format:check` pass
- [x] 3.2 `cargo test` still passes

#### Manual

- [ ] 3.3 Report worked total equals the sum of day totals and the deviation matches a hand count
- [ ] 3.4 Ticking "Dzień wolny" on a past weekday raises the deviation by 8:00 in report and header; unticking restores it; Saturday has no checkbox
- [ ] 3.5 With the counter running, the header deviation moves with "Dziś" and the report updates within a minute
- [ ] 3.6 Previous month shows its final deviation as closed with no needed figure; next month shows only the norm
- [ ] 3.7 Marking a future weekday off lowers the remaining-day count of the needed figure by one
- [ ] 3.8 After midnight the header deviation drops by 8:00 on a workday without interaction, and the report follows within a minute
- [ ] 3.9 A failing store shows ErrorState in the report and the header error, never zeros
