<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Work-Norm Report

- **Plan**: context/changes/work-norm-report/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 7 observations

Notes: every automated check passes. Run in this review: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (all suites, including `days_off_crash`), `npm run build`, `npm run lint` and `npm run format:check`. Manual checks 2.3 and 3.3–3.9 are still unchecked in Progress and count as pending.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING (manual checks pending) |

## Findings

### F1 — Polish day-count plurals wrong for common counts

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence (plan defect copied into code)
- **Location**: src/worktime/MonthReport.tsx:11, 19, 28
- **Detail**: The plan gave fixed suffixes, and the code copies them as-is:
  - "(N dni roboczych)" in the current/past worked line
  - "przez N dni roboczych" in the needed line
  - "(N dni robocze)" in the future norm line

  Polish needs "dzień roboczy" for 1, "dni robocze" for 2–4 and 22–24 (but not 12–14), and "dni roboczych" otherwise. As a result, October 2026 shows "(22 dni roboczych)", December's norm shows "(23 dni robocze)" correctly but November's would show "(20 dni robocze)", and the last week of every month shows "przez 1 dni roboczych" / "przez 3 dni roboczych".
- **Fix**: Add a `workdaysLabel(n)` plural helper in `format.ts` and use it in all three lines.
- **Decision**: FIXED — `formatWorkdays(n)` plural helper in format.ts, used in all three lines

### F2 — Day-off insert silently swallows CHECK violations

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/report.rs:141, src-tauri/src/work_time/commands.rs:143-145
- **Detail**: `INSERT OR IGNORE INTO days_off …` ignores CHECK violations as well as the primary-key conflict. `parse_day` uses chrono's `%Y`, which accepts signed years with more than four digits (e.g. `+10000-01-01`). Such a date formats back to a key that fails the `GLOB` CHECK, so the row is dropped. The command still returns `Ok`, `work-time-changed` fires, and the UI shows nothing. That breaks the "surface the failure" rule. The picker can't send such a date today, so reachability is low. `list_work_entries` (`commands.rs:78`) also repeats the parse instead of calling `parse_day`.
- **Fix**: Use `INSERT INTO days_off … ON CONFLICT(day) DO NOTHING` so only the duplicate is ignored. Bound the year in `parse_day` (1..=9999 → otherwise `InvalidLocalTime`) and reuse `parse_day` in `list_work_entries`.
- **Decision**: FIXED — `ON CONFLICT(day) DO NOTHING`; `report::parse_day` requires a year in 1..=9999 in canonical form and is reused by `list_work_entries`; 2 new unit tests

### F3 — Month report stops refreshing after an error or when a future month becomes current

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/worktime/useMonthReport.ts:86-96
- **Detail**: The 60 s interval runs only while the last result is `loaded` with `phase === "current"`. That causes two gaps:
  - After one failed refresh the state is `error` and the interval stops. Recovery then waits for focus, visibility, `work-time-changed` or a manual retry.
  - If December is on screen on 30 Nov, its `future` phase never starts the interval. After midnight it keeps showing only the norm until something else refetches.

  The plan's rule ("every 60 s while phase is current") has the same gap, and it undercuts the plan's goal that "the report never disagrees with the header".
- **Fix**: Run the interval unless the last result is a loaded `past` month, so error and future states also re-poll. Skip the tick while `document.visibilityState !== "visible"`.
- **Decision**: FIXED — polls unless a loaded past month; an error re-subscribes and reloads; ticks skipped while hidden

### F4 — Crash test's weekend assertion cannot fail

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/tests/days_off_crash.rs:106-111
- **Detail**: The child writer skips weekends itself (lines 65-67), and `set_day_off` refuses them before writing, so "no weekend rows" is true by construction. The real proof in this test is `quick_check` plus `rows >= last commit`, which is valid and mirrors `work_time_crash.rs`.
- **Fix**: Replace the weekend assertion with a check that the rows are exactly the first N weekdays from `first_day()` (contiguous, no gaps).
- **Decision**: FIXED — asserts the rows are exactly the first N weekdays from `first_day()`

### F5 — Day-off checkbox shows a stale state between write and refetch

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/worktime/WorkTimeView.tsx:301-319, src/worktime/useWorkEntries.ts:121-131
- **Detail**: The checkbox is controlled by `entries.data.dayKind`. After a click it snaps back to the old state, and `dayOffBusy` clears when the command returns, which can happen before the event-driven entries refetch arrives. The result is a short window where the checkbox is enabled but stale.
- **Fix**: Keep `dayOffBusy` set until the next entries load for that day lands (or show the requested value optimistically while busy).
- **Decision**: FIXED — the request stays pending (checked as requested, disabled) until the reloaded `dayKind` agrees or the write fails

### F6 — Rust types and TS mirrors landed in different commits

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: commits 1146ef5 (p1) / 74aabae (p2)
- **Detail**: p1 extended the serialized `WorkTimerStatus` (`monthDeviationMs`, `asOfMs`) and `DayEntries` (`dayKind`). Their TS mirrors arrived only in p2. AGENTS.md says "Change the Rust type and its TS mirror in the same commit". The plan's phase split caused it. The fields are additive, so nothing broke at runtime, and the history can't be changed now.
- **Fix**: No code change. In future plans, keep a serialized-type change and its TS mirror in the same phase (candidate for `/10x-lesson`).
- **Decision**: ACCEPTED — history cannot change; lesson candidate for future plans (keep Rust type + TS mirror in one phase)

### F7 — Phase-dependent report fields fall back to 0 instead of failing

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/types.ts (MonthReport), src/worktime/MonthReport.tsx:11-24
- **Detail**: `MonthReport` mirrors the flattened Rust struct as flat `number | null` fields. The view covers the nulls with `?? 0` (`formatSigned(report.deviationMs ?? 0)`, `formatHm(report.remainingMs ?? 0)`, `report.workdaysAfterToday ?? 0`). This can't happen for the right phase today, but a future contract drift would render "0:00", which is the "zeros instead of a failure" pattern AGENTS.md forbids.
- **Fix**: Type `MonthReport` in TS as a union discriminated on `phase` (TS only, JSON unchanged) and drop the `?? 0` fallbacks.
- **Decision**: FIXED — `MonthReport` TS type is a union discriminated on `phase`; `?? 0` fallbacks removed

### F8 — Plan contradicts itself on report placement

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: context/changes/work-norm-report/plan.md:25 vs src/worktime/WorkTimeView.tsx:321-333
- **Detail**: "Desired End State" says the view "starts with a month report". The Phase 3 contract puts the checkbox under the day picker and the report above the list. The code follows the Phase 3 contract: h1, picker, checkbox, report, list.
- **Fix**: Amend the Desired End State sentence to "the report sits between the day picker and the entry list".
- **Decision**: FIXED — Desired End State amended in plan.md

### F9 — Header now fails whole if the month computation fails

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src-tauri/src/work_time/mod.rs:176
- **Detail**: `status` now calls `report::compute(...)?` inside its read. Any month-level failure (e.g. `day_bounds` on a local midnight that doesn't exist) would now take down "Dziś" and START/STOP as well. In the app's real zone (Local = Europe/Warsaw) the DST change is at 02:00/03:00, so this can't happen today. The plan chose this coupling on purpose (one snapshot).
- **Fix**: Accept as designed. Revisit only if month computation gains new failure modes.
- **Decision**: ACCEPTED — coupling chosen by the plan (one snapshot); unreachable in Europe/Warsaw

### F10 — Minor hygiene

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/useMonthReport.ts (lastOkAt), src-tauri/tests/work_time.rs:70,90-92
- **Detail**:
  - `lastOkAt` is one ref shared across months, so an error on month B can show month A's last-OK time.
  - `tests/work_time.rs` declares `unmarked` and also an `unmarked_day()` helper for the same date.
  - The worktime views fold `lastOkAt` into the message, while own-tasks passes `ErrorState`'s `lastSuccess` prop. This is consistent within worktime but not across features.
- **Fix**: Reset `lastOkAt` when `month` changes, and drop the duplicate test helper.
- **Decision**: FIXED — `lastOkAt` kept per month; duplicate test helper removed (ErrorState prop style left consistent with worktime)
