<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Timer Reminders Implementation Plan

- **Plan**: context/changes/timer-reminders/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: SOUND
- **Findings**: 0 critical, 2 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | PASS |

## Grounding
13/13 paths ✓, 9/9 symbols ✓, brief↔plan ✓, Progress↔Phase ✓

## Findings

### F1 — Slot-index memory silences reminders after an edit

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Implementation Approach (slot rule); Phase 1 §3 `ReminderMemory`
- **Detail**: The memory kept slot indexes `(entry_id, slot)` / `(today, slot)` and fired only when the index grew. The running entry's start can be edited while it keeps running (`work_time/mod.rs:320-351`; the UI shows "Edytuj" on the running row). If slot 2 was sent and the start is then moved later, slot 0 ≤ 2 when the new threshold is reached, so the reminder is silent for three more hours. The same applies to `notRunningAfter` moved later mid-day, or a higher `longRunningHours`.
- **Fix**: Key on the slot's start instant (`anchor + slot·1h`, epoch ms) instead of its index, and add tests for an edited start and a mid-day settings change.
  - Strength: One comparison; keeps the no-repeat, no-backlog and clock-moved-back guarantees.
  - Tradeoff: The memory type changes from slot to instant.
  - Confidence: HIGH. The new threshold lies after now, which is after every instant already sent.
  - Blind spot: Moving the reference earlier waits for the next hour (stated in the plan).
- **Decision**: FIXED (Fix in plan)

### F2 — Store lifecycle and logging do not match the code

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 2 §2 (Reminder thread, Contract)
- **Detail**: The plan said "the store is opened by the first health round". In fact `with_store` opens it lazily for any first caller, including backup and migrations (`store/mod.rs:205-230`). The "one warning when failures start" state diverges from the `reconcile_todo` precedent, which logs at debug level (`gitlab_refresh.rs:357-372`). Notification `show()` does not block the main thread. `settings_path` is moved into `gitlab_refresh::start` at `lib.rs:186`, so it needs a clone.
- **Fix**: Correct the rationale, log store errors at debug level, and note the `settings_path` clone.
- **Decision**: FIXED (Fix in plan)

### F3 — Third copy of the settings-block loader

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Lean Execution
- **Location**: Phase 1 §1
- **Detail**: The read + BOM + UTF-8 + version + block-extraction code already exists twice (`settings.rs:98-135`, `150-187`). The reminders loader adds a third copy.
- **Fix**: Keep the copy and state in the plan that extracting a shared helper is out of scope (it would refactor the shipped loaders).
- **Decision**: FIXED (stated as out of scope in Phase 1 §1)

### F4 — A dead reminder thread is silent, against the brief's promise

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2
- **Detail**: A spawn failure or a panic in the loop is only logged, and "Przypomnienia" stays Ok. GitLab reports `ThreadDown` in the same case (`gitlab_refresh.rs:174-184`).
- **Fix**: Record it as an accepted risk in "What We're NOT Doing" and in the brief's Open Risks.
- **Decision**: ACCEPTED (recorded in plan and brief)
