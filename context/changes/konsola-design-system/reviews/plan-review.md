<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Design system Konsola w aplikacji

- **Plan**: context/changes/konsola-design-system/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 1 critical, 4 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | WARNING |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | FAIL |

## Grounding
Grounding: 9/9 paths ✓, 2/2 symbols ✓ (`NotePreview`, `VIEWS`), brief↔plan ✓ (component count changed by F6)

## Findings

### F1 — Grep checks can never pass as written

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Progress 1.3, 3.2, 4.2, 5.2, 6.3
- **Detail**: `rg` is not installed in the environment. The 3.2/4.2/5.2 checks searched the whole `src`, where the old classes would stay in `App.css` until phase 6, so they could never pass.
- **Fix**: Switch to `git grep --untracked`, and have each view phase delete its own block from App.css.
- **Decision**: FIXED

### F2 — HoursBar hard-codes the 8 h norm in the UI

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architectural Fitness
- **Location**: Phase 6 §1, Progress 6.6
- **Detail**: Rust sends no daily norm. Writing 8 h in TSX would move a norm rule into the frontend, which AGENTS.md forbids.
- **Fix A ⭐ Recommended**: Drop HoursBar from phase 6 (out of scope)
  - Strength: Zero Rust and domain-logic changes, as agreed.
  - Tradeoff: A design-system component stays unused in the app.
  - Confidence: HIGH — verified that the daily norm is not in the data.
  - Blind spot: None significant.
- **Fix B**: Add `dayNormMs` to the Rust day DTO
  - Strength: The bar works and the rule stays in Rust.
  - Tradeoff: Rust change plus test, against "existing data only".
  - Confidence: MED — where the day type is computed is not yet checked.
  - Blind spot: The TS mirror has to change in the same commit.
- **Decision**: FIXED (Fix A)

### F3 — Old App.css rules override the new components between phases

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architectural Fitness
- **Location**: Critical Implementation Details, Phase 1 §5
- **Detail**: Several old selectors beat a single `.dn-*` class: `button:hover:not(:disabled)` (0,2,1), `:disabled` (0,1,1), and the nav, filter and row rules on checkboxes and buttons.
- **Fix**: Wrap the global element rules in `:where()` in phase 1, and delete each view's block in the phase that migrates it.
- **Decision**: FIXED

### F4 — Window flashes at its default size before the restore

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §4
- **Detail**: The window has no `visible: false`, and VISIBLE is left out of the saved state on purpose (closing hides the window to the tray), so nothing delays showing the window until the restore is done.
- **Fix**: Add `"visible": false` plus `runtime::show_main` at the end of `setup`, and a new manual check 2.9.
- **Decision**: FIXED

### F5 — No source gives the font files in a latin + latin-ext subset

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §2, Performance Considerations, brief
- **Detail**: The official releases ship full woff2 files; subsetting would need a new tool. The ~300 KB estimate is too low.
- **Fix**: Use full woff2 files, estimate under 1 MB, and add a manual check for Polish characters.
- **Decision**: FIXED

### F6 — Tabs is built but no phase uses it

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Lean Execution
- **Location**: Phase 1 §3
- **Detail**: The own-tasks filter uses Segmented, and nothing else uses Tabs.
- **Fix**: Drop Tabs and list it under "What We're NOT Doing". Component count: 12.
- **Decision**: FIXED

### F7 — Wrong facts about MonthReport and the space default

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 5 §1, Phase 6 §1
- **Detail**: `workdaysAfterToday` does not count today, `neededPerDayMs` can be null, and a future month has no deviation. The space default comes from `ownTaskDefaultSpace()` in OwnTasksView, not from a hook.
- **Fix**: Name the fields, the null cases and the real source of the default.
- **Decision**: FIXED

### F8 — The change notes say ErrorState covers loading and empty too

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Key Decisions (brief), change.md Notes
- **Detail**: The plan keeps ErrorState for errors only; the notes say otherwise.
- **Fix**: Add a sentence about dn-loading, dn-empty and SourceState in the plan and the brief.
- **Decision**: FIXED
