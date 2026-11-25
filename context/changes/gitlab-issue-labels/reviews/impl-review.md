<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Etykiety GitLaba jako kolorowe chipy i filtr widoku „Praca (GitLab)”

- **Plan**: context/changes/gitlab-issue-labels/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-25
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 2 warnings, 6 observations

Coverage note: the Automated rows of all three phases are ticked. All 14 Manual rows (1.7, 2.3–2.6, 3.3–3.11) are still pending, and none is falsely ticked. This review looks at the code from commits f1aadd4, 78996ff, 584f64d and 38c5b7c (`b2fbf2c..HEAD`).

Automated criteria re-run at HEAD:
- `npm run lint`, `npm run format:check` and `npm run build`: pass.
- The literal-colour `rg` finds no matches.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test`: 636 passed, 0 failed.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | WARNING |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Findings

### F1 — MultiSelect handles Escape globally and stays open after Tab-out

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/components/MultiSelect/MultiSelect.tsx:25-44
- **Detail**: While the panel is open, a `document` keydown listener handles Escape wherever focus is.
  - It calls `event.stopPropagation()`, which blocks the `window` Escape listener in `WorkTimerHeader.tsx:31-34`. So Escape leaves an armed START/STOP confirmation pending and moves focus to the MultiSelect button.
  - The panel closes only on an outside `pointerdown`. Tabbing out leaves it open, and a later Escape anywhere steals focus back.
  - Two panels opened by keyboard can both be open at once.
- **Fix**: Handle Escape only when `rootRef.current.contains(document.activeElement)`, drop `stopPropagation()`, and close on `focusout` when `relatedTarget` is outside the root.
- **Decision**: FIXED (62d250c) — Escape handled in the root onKeyDown without stopPropagation; close on focus-out

### F2 — „Dokumentacja ustawień” is invisible exactly when there is no settings filter

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src/gitlab/GitlabFilterBar.tsx:44
- **Detail**: The plan (Phase 3, item 7) put the docs button "next to the settings.json button" in `GitlabFilterBar`. But the bar returns `null` when `gitlabFilter` has no values. The link is meant for someone who wants to set up a filter, and that person usually has none yet, so they never see it. Manual row 3.11 can only be checked with a filter already configured. The implementation follows the plan literally; the plan's placement is the problem.
- **Fix A ⭐ Recommended**: When there is no settings filter, render a minimal bar row with only "Dokumentacja ustawień" (and "Ustawienia").
  - Strength: Keeps the link where the plan put it, next to the settings button. It becomes reachable in the state where it is most needed.
  - Tradeoff: One more row above the list for users without a settings filter.
  - Confidence: HIGH — a small conditional in one component.
  - Blind spot: Whether an always-present row adds noise in the toolbar area; not checked visually.
- **Fix B**: Move the docs link into `.gitlab-toolbar` as a quiet icon button, always visible.
  - Strength: Always reachable without adding a row.
  - Tradeoff: Drifts from the plan's placement, and the toolbar is already busy with three pickers and the type switch.
  - Confidence: MED — the toolbar layout at narrow widths is unverified.
  - Blind spot: How it wraps below 900 px.
- **Decision**: DISMISSED — the user chose to remove the settings docs link from this change: 38c5b7c reverted, item 7 and row 3.11 dropped from plan.md, F2 fix discarded

### F3 — Picker counts include hidden issues and ignore the type switch

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture
- **Location**: src/gitlab/GitlabView.tsx:448-454
- **Detail**: `viewOptions` counts MRs, issues, hidden issues (even when „Pokaż ukryte” is off) and closed items, whatever the type switch says. The plan asked for exactly this ("ze wszystkich sekcji"). In use, though, a label that appears only on hidden issues shows a count of N, and choosing it gives the empty state. Separately, the first colour seen wins when two labels share a name with different colours; that is cosmetic only.
- **Fix A ⭐ Recommended**: Keep counting all sections, as planned. Treat the count as "how many items carry this value", with the empty state as the explanation.
  - Strength: The counts stay stable when the type switch or the hidden toggle changes. Matches the approved plan.
  - Tradeoff: A count can be larger than the number of rows visible after selecting.
  - Confidence: MED — depends on how often values live only in hidden or other-type items.
  - Blind spot: Real-data distribution not checked.
- **Fix B**: Count only the items the current type and hidden toggle can show; closed items stay counted when that section is enabled.
  - Strength: Each count matches the rows you get.
  - Tradeoff: The counts move when the type switch or the hidden toggle changes, and this departs from the plan's contract.
  - Confidence: HIGH — a small change to the `useMemo` input.
  - Blind spot: Selected values may show count 0 more often.
- **Decision**: ACCEPTED (Fix A) — counts over all sections, as the plan specifies

### F4 — MultiSelect panel shares z-index 10 with the sticky header

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/components/MultiSelect/MultiSelect.css:32
- **Detail**: `z-index: 10` equals `.dn-header` (`AppHeader.css:4`). The panel comes later in the DOM, so when the page scrolls with a panel open it paints over the sticky header.
- **Fix**: Set the panel's z-index below the header's (e.g. 5).
- **Decision**: FIXED (62d250c) — z-index 5, below the header

### F5 — `aria-haspopup="true"` announces a menu that is a checkbox group

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/components/MultiSelect/MultiSelect.tsx:52
- **Detail**: `aria-haspopup="true"` means `menu`, but the panel is a `role="group"` of checkboxes with no `menuitem`s.
- **Fix**: Use `aria-haspopup="dialog"`, or drop it and keep `aria-expanded` plus `aria-controls`.
- **Decision**: FIXED (62d250c) — aria-haspopup removed

### F6 — A panel closed by losing its options reopens on its own

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/components/MultiSelect/MultiSelect.tsx:18-23
- **Detail**: `isOpen = open && !disabled`, but `open` is never reset. If a refresh empties the options while a panel is open, the next refresh that brings options back reopens the panel without user action.
- **Fix**: Reset `open` to false when the panel becomes disabled, deriving it during render or in an effect keyed on `disabled`.
- **Decision**: FIXED (62d250c) — open resets when the options run out

### F7 — Unplanned: the „Zamknięte” panel count follows the view filter, plus a new hook file

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Scope Discipline
- **Location**: src/gitlab/GitlabView.tsx (ClosedBlock), src/gitlab/useGitlabListView.ts
- **Detail**:
  - The plan pinned only the view-header counter as unchanged. The „Zamknięte” panel's count now shows the view-filtered number, which is consistent with its rows.
  - The context and hook live in a new `useGitlabListView.ts` that the plan does not list. This mirrors `WorkTimerProvider` / `useWorkTimer` and keeps the react-refresh lint rule clean.
  - Both are harmless.
- **Fix**: Accept as is; the epilogue/report already records the hook split as an adaptation.
- **Decision**: ACCEPTED — harmless, consistent with the rows and the WorkTimer pattern

### F8 — MultiSelect button re-implements control styling

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/components/MultiSelect/MultiSelect.css:8-28
- **Detail**: `.dn-multiselect-btn` copies the control look of `Button` / `.dn-seg-btn` (min-height, padding, border, colours) instead of reusing `Button`. This makes a third copy of the control styling.
- **Fix**: Leave for the design-system sync change (out of scope here); note it there.
- **Decision**: SKIPPED — left for the design-system sync change
