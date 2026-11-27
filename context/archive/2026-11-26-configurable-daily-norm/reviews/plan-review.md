<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Konfigurowalna norma dzienna i pół dnia wolnego

- **Plan**: context/changes/configurable-daily-norm/plan.md
- **Mode**: Deep (claims checked inline against the code, no sub-agent)
- **Date**: 2026-11-26
- **Verdict**: REVISE → SOUND after fixes (all findings fixed in the plan; non-interactive run, fixes chosen by the agent)
- **Findings**: 0 critical, 4 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

16/16 paths ✓ (norm.rs, report.rs, settings/work_time.rs, commands.rs, mod.rs, schema.rs, types.ts, api.ts, WorkTimeView.tsx, useDayOffToggle.ts, useWorkEntries.ts, MonthReport.tsx, summary.ts, Segmented, tests/days_off_crash.rs, tests/common/crash.rs); symbols ✓ (`DAILY_NORM_MS` 14 uses, `DayKind::` matches: none exhaustive outside report.rs, `set_day_off` callers = the plan's list, six `WorkTimeSettings {` literals, `MIGRATIONS[..n]` upgrade pattern, `constraint_error_carries_no_row_values` pattern, reminders `!= DayKind::Workday`); all `work_time` submodules and `settings` are `pub`, so pub items used only by tests between phases (`DayMark::HalfDayOff`, `norm::workdays`, `is_workday`) raise no dead-code warning; brief↔plan ✓; Progress↔Phase ✓ (1.1–1.6, 2.1–2.5, 3.1–3.9, 4.1–4.3 one for one).

Per-phase gates: phase 2 keeps `set_day_off(off: bool)` and only adds TS fields/variant (no exhaustive `switch` on `DayKind` in `src/`), so `npm run build` stays green; phase 3 changes `DayMark` Deserialize + command + `setDayOff` together. Store rules (v12 appended, one `write`, upgrade test, crash test, CHECK error without values) and Rust↔TS lockstep hold. Average-per-day formula `ceil_to_minute(ceil(remaining × daily_norm / norm_after_today))` reduces exactly to today's result without half days; half norm is exact in ms.

## Findings

### F1 — "8 h" neutral text contradicts criterion 1.4

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §7 and Success Criteria 1.4
- **Detail**: §7 had `HowCounted` show "norma dzienna z settings.json, domyślnie 8 h" while loading, but 1.4 requires `git grep -n "8 h" -- src/worktime` to find nothing; also the UI should not know the 8 h value (archived days-table invariant). 1.4's "`git grep -n DAILY_NORM_MS` finds only `DEFAULT_DAILY_NORM_MS`" was not a mechanical check.
- **Fix**: Neutral text without a number; 1.4 rewritten as `git grep -nw "DAILY_NORM_MS" -- src-tauri` and `git grep -n "8 h" -- src/worktime` find nothing (block and Progress row, same wording).
- **Decision**: FIXED

### F2 — Manual 1.6 expects a message the dashboard does not show

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 Manual Verification 1.6
- **Detail**: `src/todo/summary.ts:27-34` renders only "—" / "raport niedostępny" on a report error; the message about `dailyNormHours` appears only in "Czas pracy". The manual row would fail against correct code.
- **Fix**: 1.6 now says "Czas pracy" shows the error with the `dailyNormHours` message and the dashboard tiles show "raport niedostępny" (block and Progress row).
- **Decision**: FIXED

### F3 — Proptest invariant becomes true by construction

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 1 §5, Phase 2 §7, Testing Strategy
- **Detail**: After `figures` sums `day_norm_ms` over `day_norms`, "Σ row norms = month norm" holds whatever `day_norm_ms` returns, so a wrong half norm (e.g. full norm for `Half`) or a wrong average-per-day rule would still pass the extended proptest.
- **Fix**: Proptest gets an independent oracle computed in the test from the flags and the calendar (`norm_ms = daily × unmarked weekdays + daily/2 × half days`) and an exact control condition for "Średnio / dzień" (`needed × norm_after ≥ remaining × daily` and `(needed − 1 min) × norm_after < remaining × daily`, in `i128`), recorded in Critical Implementation Details.
  - Strength: catches the arithmetic the slice is about, not only internal consistency.
  - Tradeoff: a few more lines in the proptest.
  - Confidence: HIGH — the condition follows from the formula (ceil then ceil to minute = smallest whole minute ≥ the real quotient).
  - Blind spot: None significant.
- **Decision**: FIXED

### F4 — "Header deviation" in the outcome handled only implicitly

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Desired End State, What We're NOT Doing, Phase 2 §7
- **Detail**: The outcome lists header deviation; the header (`WorkTimerHeader.tsx`) shows only today's total and `monthDeviationMs` (`mod.rs:211`) has no UI consumer. The plan mentioned this only inside the Lenient bullet, and no test covered the status field with a half day.
- **Fix**: Desired End State and NOT Doing now say explicitly that header deviation = the status field `monthDeviationMs`, which follows the new rule via `report::compute`, and that no deviation is added to the header; Phase 2 §7 adds a status test with a half day (extends `status_month_deviation_equals_the_report_deviation`); brief updated.
- **Decision**: FIXED

### F5 — Odd-minute norm halves to 30 s fractions

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details, Phase 2 §7
- **Detail**: A norm of 7:01 gives a half day of 3:30:30; Rust is exact in ms but the UI floors to minutes (`formatHm`), and there was no explicit test or note.
- **Fix**: Note added (Rust exact, UI shows 3:30, sub-minute deviation shows "0:00", no rounding in Rust) and a unit test for 421 min → 12_630_000 ms and the month norm.
- **Decision**: FIXED

### F6 — 104 px is too narrow for a three-button Segmented

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 §4
- **Detail**: `.dn-seg-btn` has `padding: 0 var(--space-3)` (24 px per button) plus the label, so "nie" / "½" / "tak" needs about 120 px.
- **Fix**: Width now sized to three `sm` buttons (about 120 px), with the bar column `minmax(0, 1fr)` absorbing it; still checked at 800 px (manual 3.8).
- **Decision**: FIXED

## Triage summary

- Fixed: F1, F2, F3, F4, F5, F6 (6)
- Verdict after fixes: SOUND
