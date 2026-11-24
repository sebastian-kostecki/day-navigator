<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Wspólne narzędzia UI (shared-ui-tools)

- **Plan**: `context/changes/shared-ui-tools/plan.md`
- **Mode**: Deep
- **Date**: 2026-11-24
- **Verdict**: SOUND
- **Findings**: 0 critical, 1 warning, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | PASS |
| Plan Completeness | WARNING |

## Grounding

10/10 paths ✓, 6/6 tokens ✓, brief↔plan ✓, Progress↔Phases 18/18 ✓. Verified in code: CSS order (components before views, `src/App.tsx:2`), `react-hooks/set-state-in-effect` active (`eslint.config.js`), `refreshing` always returns to `false` after a round (`src-tauri/src/gitlab/mod.rs:468`, `src-tauri/src/github/mod.rs:285`), `ErrorState.message` never used as a string, `WorkTimeView` `nowMs` only set by the 1 s tick.

## Findings

### F1 — Podział CSS tabeli GitLaba między DataTable a widok jest niedookreślony

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Completeness
- **Location**: Phase 2, change 4 „Szkielet tabeli” + Desired End State
- **Detail**: The plan moved „odstępy wiersza i typografia komórek” to `DataTable.css` and left only the column template and hiding in the view, but (a) the 600 px row padding override sits on `.gitlab-grid` (`GitlabView.css:124-128`) with no stated target after the move, (b) 11 px labels (`:65-67`) and the 14/20 title (`:84-90`) are GitLab-only and beat the global `.code` 12 px only by CSS order, (c) the plan did not record that component CSS loads before view CSS.
- **Fix**: Define the boundary: `DataTable.css` owns grid via `--dn-table-cols`, gap, side padding via `--dn-table-pad-x` (default `--space-4`), heights, lines, `min-width: 0`, `tone`/`truncate`; `GitlabView.css` keeps `--dn-table-cols` and `--dn-table-pad-x` per breakpoint, column hiding, 11 px labels, title layout, `.is-hidden`. Update Desired End State and add a Critical Implementation Details note on CSS order.
  - Strength: 600 px look and labels stay unchanged; S-05 reuses the same variables.
  - Tradeoff: GitLab's view keeps some typography, less than the issue's literal wording suggests.
  - Confidence: HIGH — import order and values checked in code.
  - Blind spot: S-05's padding needs not checked.
- **Decision**: FIXED (Fix in plan — Desired End State, new Critical Implementation Details, Phase 2 change 4; brief updated)

### F2 — Czwarta kopia odmiany w worktime/format.ts

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Phase 1, change 3
- **Detail**: `formatWorkdays` (`src/worktime/format.ts:32-38`) inlines the same 2–4 / 12–14 rule; the criterion grep for `function plural` would not catch it.
- **Fix**: `formatWorkdays` calls `plural` from `../components`; extend the criterion grep with `tens < 12`.
- **Decision**: FIXED (Fix in plan — Phase 1 change 3 and criteria 1.2 / Progress 1.2)
