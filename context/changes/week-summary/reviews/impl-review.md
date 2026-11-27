<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Week summary in "Czas pracy"

- **Plan**: context/changes/week-summary/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 3 observations

Diff reviewed: `git diff d34904a..HEAD` (commits 8e8d694, 99b039d, 0871bc6, ae1ae97 and their SHA docs follow-ups).

**Must fix before merge: none.** Every finding below is optional.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## What was checked

- **Plan adherence.** Phase 1: `day_report` extracted into `report.rs` with the planned signature (takes `&WorkTimeSettings`), `days_off_between` is `pub(super)`, one `DayReport` literal, the month loop calls it; month tests not edited. Phase 2: `week.rs` matches the contract (`week_days` with checked arithmetic, `WeekPhase` lowercase, `WeekReport` camelCase, totals summed from the rows, `norm_to_date` filtered on the `NaiveDate` rather than the row string, one `store.read`, no `tauri`, no `DAILY_NORM_MS` / `figures(` / `BTreeSet` / literal 8 h). `mod.rs` got exactly the two module lines. Phase 3: `work_week_report` is `async`, `Policy::Strict`, no `notify`, registered once in `lib.rs`; the TS union matches the serde shape per phase (`past`: `remainingMs` null; `future`: all three null); `invoke` only in `api.ts`. Phase 4: hook, panel, CSS, `export liveFigures`, the two-line `WorkTimeView.tsx` edit, README, CHANGELOG and the approved PRD line all as planned; `HowCounted` untouched.
- **AGENTS.md.** Domain rule in Rust with unit tests and a proptest; the UI only displays and applies the shared `liveFigures` to today's row; timer read only via `useWorkTimer()`; loading / `ErrorState` with "ostatnio OK HH:MM" and retry / data states; CSS is layout only (the px column template follows the existing `.wt-days` precedent at `WorkTime.css:21`), no colours; store read via an async command through `run_with_settings` (`spawn_blocking`); no migration, no capability or config change; invented test data only; no secrets.
- **Correctness.** Monday computation (Sunday maps back, year crossing) tested; DST weeks count real lengths; a running entry across Sun→Mon and entries across midnight / the week boundary split correctly; future / past / current figures; serialized shape per phase. A Saturday holiday shows the name plus " · należy się inny dzień wolny". The `holiday` branch of `kindLabel` returning `null` is safe: `classify` (`report.rs:101-111`) returns `Holiday` only when `holidays.holiday(day).is_some()`, and `day_report` fills `holiday` from the same call, so a `Holiday` row always has a name and is handled by the first `if` in `kindLabel`. Day / Monday rollover: the hook sends `localDate()` at request time and `today` is read at render, so the highlight and the week move on the next poll, focus or change (within 60 s). Effects clean up the listeners, the interval and a late `listen` resolution; a late `setReport` after unmount is harmless in React 19 and matches `useMonthReport`. Day labels are quiet `Button`s with an `aria-label`, as in `DaysTable`.
- **S-02 readiness.** No hard-coded 8 h / `DAILY_NORM_MS` / `norm::figures` in new code; `week.rs` binds `days_off` untyped and only passes it on; tests use only `default()` / `manual_only()`, expected norms are `n × workday_norm`, and the only `set_day_off` call is inside `mark_off`; the exhaustive `never` switch in `kindLabel` will flag `"halfDayOff"` at build time. Shared-file edits are small and local.
- **Progress rows.** All Automated rows are checked and reproduce (below); Manual rows 1.8 and 4.8–4.14 are unchecked, as expected because they need the real app.

## Gates (re-run in this review)

| Gate | Result |
|---|---|
| `npm run lint` | pass |
| `npm run format:check` | pass |
| `npm run build` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (0 warnings) |
| `cargo test` | pass (727 passed, 0 failed; 13 `week_tests` incl. the proptest) |
| `rg` checks 2.2, 2.10, 2.11, 4.4, 4.5 | as planned |

## Findings

### F1 — Week row copies the days-table row rules

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/WeekSummary.tsx:86-160 (copy of src/worktime/DaysTable.tsx:28-122; also `HOUR_MS` and `dm` at WeekSummary.tsx:19-24)
- **Detail**: `WeekRow` re-implements the display rules of `DayRow`: `showSum`, `showBar`, the long Saturday-holiday `title` text, `barLabel` and the deviation tone chain. The plan only shared `liveFigures`, so this is within plan, but the two rows can drift: S-02 edits `DayRow` (half day off) and only the `kindLabel` switch forces the week row to follow, not `showBar`, hatching or the title. Not a bug today: both rows render identically for every current `DayKind`.
- **Fix**: Optional, after S-02 merges (to avoid a conflict in `DaysTable.tsx`): export one helper from `DaysTable.tsx`, e.g. `dayRowDisplay(row, today, live, liveTodayMs)` returning `{ workedMs, deviationMs, showSum, showBar, holidayTitle, barLabel, deviationTone }`, and use it in both `DayRow` and `WeekRow`; export `dm` and `HOUR_MS` with it.
- **Decision**: PENDING

### F2 — Holiday rows' name not asserted in the week tests

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/work_time/week_tests.rs:83-101
- **Detail**: The UI's `kindLabel` relies on "a `Holiday` row always has a `holiday` name". This holds by construction (`classify` and `day_report` use the same `holidays.holiday(d)`), and the week rows are cross-checked against the month rows, but no week test asserts it directly (the Christmas, New Year and Easter cases assert `kind` only).
- **Fix**: Optional: in `check_totals_and_months` add `assert!(w.days.iter().filter(|r| r.kind == DayKind::Holiday).all(|r| r.holiday.is_some()));` so the invariant is pinned for every week, including the proptest.
- **Decision**: PENDING

### F3 — Planned short staleness windows

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/worktime/WeekSummary.tsx:56-84, src/worktime/useWeekReport.ts:96-105
- **Detail**: As planned (A8, A9): while the counter runs, today's row is live but the "Tydzień", "Odchylenie" and "Zostało" tiles catch up only on the next report (up to 60 s); right after midnight Sunday→Monday the panel keeps last week's rows (no row highlighted) until the next poll, focus or change. Matches the month tiles' behaviour.
- **Fix**: No action; revisit only if manual check 4.9 finds the lag confusing.
- **Decision**: PENDING

### F4 — change.md status skipped "implemented"

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: context/changes/week-summary/change.md:4
- **Detail**: The implementation left `status: plan_reviewed`; this review sets it to `impl_reviewed`. Commit subjects differ slightly from the plan's suggested wording (e.g. p1 "share per-day row between month and week"); harmless.
- **Fix**: No action.
- **Decision**: PENDING
