<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Work-Norm Report Implementation Plan

- **Plan**: context/changes/work-norm-report/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: SOUND
- **Findings**: 0 critical, 1 warning, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

11/11 paths ✓, 8/8 symbols ✓, brief↔plan ✓. Worked example recomputed ✓: November 2026 has 21 workdays (20 with 11.11 off), 15 of them up to and including 23.11, 5 after it, and 44:00 / 5 = 8:48. December 2026 has 23 workdays.

Blast radius (sub-agent):
- The new fields touch only one Rust literal of each struct (`work_time/mod.rs:173`, `:215`) and the TS mirrors (`types.ts:25-31`, `:44-50`).
- `kind()` (`commands.rs:18-31`) has no wildcard, so it won't compile until `NotAWorkday` gets an arm. The plan already covers this.
- No test depends on the number of migrations (`tests/own_tasks.rs:41-53` reads `MIGRATIONS.last()`).
- `days_off_crash.rs` needs no change in `Cargo.toml`: the harness re-launches the test binary (`common/crash.rs:44-58`).
- The app's own commands need no permissions in `capabilities/default.json`.

## Findings

### F1 — Month report goes stale at midnight while the counter is stopped

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 3 §2 useMonthReport; manual step 3.8
- **Detail**:
  - The header refetches at the day end whether or not the counter runs (`useWorkTimer.ts:104-110`).
  - The report refetched only on the event, on focus or visibility, and every 60 s while `report.running`.
  - Overnight, with the window visible and the counter stopped, the header's deviation drops by 8:00 but the report keeps yesterday's number.
  - On 30.11 → 1.12 the November report stays "current".
- **Fix A ⭐ Recommended**: Refetch every 60 s while `phase === "current"` and drop `MonthReport.running`.
  - Strength: Less code. One condition covers the counter, midnight and the month change.
  - Tradeoff: One IPC call per minute while the current month is shown.
  - Confidence: HIGH — the report is a cheap read.
  - Blind spot: None significant.
- **Fix B**: Add `todayEndMs` to MonthReport and set a timeout at it, like useWorkTimer.
  - Strength: Exact moment, and no polling.
  - Tradeoff: Duplicates the timeout logic; one more contract field.
  - Confidence: HIGH.
  - Blind spot: A throttled timer in a hidden webview.
- **Decision**: FIXED (Fix A). Edits:
  - `running` removed from MonthReport
  - 60 s refetch while `phase === "current"`
  - step 3.8 and Progress 3.8 extended to the report
  - "What We're NOT Doing", "Performance Considerations" and the brief's "Live values" updated

### F2 — Order of the three "needed" lines is not stated

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 §3 (report lines)
- **Detail**: When the norm is met before the last workday, the plan gives `neededPerDayMs = 0` and `remainingMs = 0` together. The contract said "one of" without an order.
- **Fix**: The order is remaining == 0 → "Norma miesiąca wyrobiona", then after == 0 → "brakuje …", otherwise "Średnio …".
- **Decision**: FIXED

### F3 — Two misleading lines in the plan text

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Key Discoveries (commands.rs:83-100); Phase 1 Implementation Note
- **Detail**: (a) "A day-off write follows the same order" (emit, then read the status), but set_day_off returns `()`. (b) Phase 1 has no manual criteria, yet its note asks for a manual pause.
- **Fix**: (a) "emits after a successful write, like add/update/remove". (b) Phase 1 continues to Phase 2 once automated checks pass.
- **Decision**: FIXED
