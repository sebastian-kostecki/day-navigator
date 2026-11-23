<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Work Timer Implementation Plan

- **Plan**: context/changes/work-timer/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE (SOUND after fixes)
- **Findings**: 0 critical, 3 warnings, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding
15/15 paths ✓, 6/6 symbols ✓ (MIGRATIONS, Store::write, with_store, blocking, StoreError, health-changed), brief↔plan ✓, Progress↔Phase ✓

## Findings

### F1 — Minute-truncated forms make START/STOP entries uneditable

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 1 §3 (update contract) + Phase 4 §2 (edit form)
- **Detail**: START/STOP store ms instants, but `startLocal`/`endLocal` are minute-precision and `update` always re-sends the full start. Editing only the end of an entry started right after a STOP in the same minute (STOP 12:00:30, START 12:00:40) resolves its start to 12:00:00 and is refused as `Overlap`; any edit silently moves the start up to 59 s earlier.
- **Fix**: Rust rule in `update`: a submitted local minute equal to the stored instant floored to the minute keeps the stored ms. Unit tests for "edit end only keeps start ms" and "START right after STOP in the same minute stays editable".
  - Strength: The rule stays in Rust and is unit-testable; UI and command contract unchanged.
  - Tradeoff: A subtle rule that the plan must spell out.
  - Confidence: HIGH — follows directly from the planned contract.
  - Blind spot: None significant.
- **Decision**: FIXED — Phase 1 §3 rule, new criterion + Progress 1.7, Testing Strategy, non-goal note

### F2 — "Dziś" can stay on yesterday's total after midnight

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Phase 3 §2 (useWorkTimer)
- **Detail**: The day-end check (`Date.now() >= dayEndMs`) had no timer while stopped (the 1 s interval runs only while running), and `visibilitychange` on Tauri `window.hide()`/`show()` in WebView2 is unverified.
- **Fix**: Always arm a `setTimeout` to `dayEndMs` (re-armed after each load) and also refetch on window `focus`.
- **Decision**: FIXED — Critical Implementation Details, Phase 3 §2 contract, new manual check + Progress 3.10

### F3 — How commands reach StoreHandle from spawn_blocking is unstated

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §2 (work-time commands)
- **Detail**: `StoreHandle` is neither Clone nor Arc (`store/mod.rs:182-185`), so `State<'_, StoreHandle>` cannot move into the `'static` `blocking` closure. The health round uses `app.state::<StoreHandle>()` (`runtime.rs:277`).
- **Fix**: Move an `AppHandle` clone into `blocking` and call `app.state::<StoreHandle>().with_store(today, …)` inside.
- **Decision**: FIXED — Phase 2 §2 Intent

### F4 — Header hook has no stale-response guard

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §2 (useWorkTimer)
- **Detail**: Several overlapping status fetches (event, visibility, focus, day end, START/STOP result) can resolve out of order; a read started before START could land after it and show "stopped". Phase 4 has the latest-response-wins guard, Phase 3 did not.
- **Fix**: Latest-request-wins sequence guard in useWorkTimer.
- **Decision**: FIXED — Phase 3 §2 contract
