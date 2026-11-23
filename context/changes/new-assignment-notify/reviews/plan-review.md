<!-- PLAN-REVIEW-REPORT -->
# Plan Review: New Assignment Notifications Implementation Plan

- **Plan**: context/changes/new-assignment-notify/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: SOUND
- **Findings**: 0 critical, 2 warnings, 3 observations (all fixed in the plan)

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | PASS |

## Grounding
11/11 paths ✓, 13/13 symbols ✓ (line refs match), brief↔plan ✓, Progress↔Phase ✓

## Findings

### F1 — Session fallback can permanently drop an overnight item

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2 — Session fallback (`SessionSeen::resolve`)
- **Detail**: Store `Ok(new)` was reduced by the last round's keys, and a first-round store error silently set those keys to `current`. An overnight item Z seen silently in a failed first round would be removed from the store's `new` on recovery and then stored, so it never notifies. No test covered "store error first, then recovery".
- **Fix**: Keep `last` (fallback diff) and `toasted_unstored` (keys the fallback toasted) per account; `Ok(new)` → `new − toasted_unstored`, then clear; `Err` → `current − last`, added to `toasted_unstored`; `Err` with no `last` → nothing. Add the recovery tests.
  - Strength: Matches the stated intent (no duplicate after recovery) without losing a notification.
  - Tradeoff: One more set per account; slightly more involved tests.
  - Confidence: HIGH — traced against the plan's resolve() rules.
  - Blind spot: None significant.
- **Decision**: FIXED — Phase 2 §2 contract, §4 tests, Implementation Approach, Testing Strategy, brief decision row

### F2 — Max interval 15 min can't meet the ≤15 min freshness NFR

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Phase 3 §1 (`REFRESH_MINUTES_MAX = 15`); brief decision table vs "Open Risks"
- **Detail**: `recv_timeout` starts after the round ends (`gitlab_refresh.rs:193-194`), so with 15 the gap between fetches always exceeds 15 min. The brief claimed both "can never break the NFR" and "can slightly exceed 15 min".
- **Fix**: Cap at 14 (constant, view text "dozwolone 1–14", tests refuse 15) and align the brief.
- **Decision**: FIXED — plan Overview, Desired End State, Phase 3 §1/§3/§4; brief

### F3 — Migration "v7" may collide with parallel slices

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 1 §1, §4; Migration Notes
- **Detail**: `schema.rs` records two renumberings from parallel merges; S-02, S-05 and S-10 are "ready" and may append a migration too.
- **Fix**: Migration Notes say to renumber at merge; the integration test runs "previous latest → new" from the migration list, not literal 6/7.
- **Decision**: FIXED

### F4 — Combined toast body will be cut by Windows

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §1 — `notices_for`
- **Detail**: Three lines of full reference + up to 100-char title wrap past the few lines a collapsed toast shows, hiding `i N−3 więcej` first. A second literal 3 duplicated `health::MAX_NOTICE_LINES`.
- **Fix**: Cut titles to 40 chars in combined-toast lines; reuse `health::MAX_NOTICE_LINES`.
- **Decision**: FIXED — Phase 2 §1 contract and §4 tests; brief row

### F5 — Invalid-interval notice blames gitlabRefresh for any file error

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §1/§3
- **Detail**: Mirroring `load_todo_limit`, invalid JSON or a wrong `version` anywhere would also return `Err`, so the view would point at `gitlabRefresh.minutes` while the GitLab block already reports `SettingsInvalid`.
- **Fix**: File-level problems give `Ok(10)` without the flag; only block-level problems give `Err`.
- **Decision**: FIXED — Phase 3 §1 contract and §4 loader cases
