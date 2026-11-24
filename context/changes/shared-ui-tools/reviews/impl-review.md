<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Wspólne narzędzia UI (shared-ui-tools)

- **Plan**: `context/changes/shared-ui-tools/plan.md`
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

## Evidence

- Commits: `de5842c` (p1), `1bc7d3a` (p2), `9f7fd0d` (p3), `c8f5315` (epilogue). All 26 changed `src/` files are listed in the plan; no unplanned files.
- Plan drift: every planned item MATCH (useNow, plural, view rewiring, Tabs, Chip, Segmented size, DataTable, OwnTasks.css, ErrorState, SourceState, GitHub/GitLab changes). Moved CSS equals the old rules value for value; Tabs keyboard handling identical to the deleted `OwnTaskTabs`.
- Safety: no `dangerouslySetInnerHTML`; `ErrorState.message` receives backend strings (escaped by React) or a static fragment. Hooks clean up intervals; no new per-second renders; no import cycles (`src/components` imports no feature folder).
- Automated criteria rerun on the final state: lint, format:check, build, cargo fmt, clippy, cargo test (531 passed, 0 failed), phase greps — all PASS. Manual rows 1.4–1.6, 2.4–2.7, 3.3–3.5 confirmed by the user during implementation.
- Checked and not a finding: `src/worktime/format.ts` importing `plural` from the `../components` barrel follows the precedent in `src/todo/summary.ts:1`; `aria-busy` on a disabled action button follows `src/health/HealthPanel.tsx:130`.

## Findings

### F1 — Fokus znika po kliknięciu „Sprawdź teraz”

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality (a11y)
- **Location**: src/components/SourceState/SourceState.tsx:49-55
- **Detail**: When `actionDisabled` becomes true right after a keyboard click, the focused button gets `disabled` and focus falls to `<body>`. The same happens today with „Sprawdź wszystkie” in `HealthPanel.tsx:128-130`, so this follows the existing pattern rather than introducing a new one.
- **Fix**: Keep the button focusable while busy (`aria-disabled` + ignore clicks instead of `disabled`) — best done for SourceState and the HealthPanel button together in a later change.
- **Decision**: FIXED — SourceState action and HealthPanel „Sprawdź wszystkie” use aria-disabled + ignored click instead of disabled; Button.css styles [aria-disabled="true"] like :disabled

### F2 — DataTable: lokalny `cx()` i braki w dokumentacji propsów

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/components/DataTable/DataTable.tsx:4-6, props `head` / `header` / `tone`
- **Detail**: A local `cx()` helper repeats the inline `[...].filter(Boolean).join(" ")` idiom used by Flag, StatusPill, Button, Panel and Chip. `head`, `header` and `tone` lack the one-line JSDoc the sibling components have, and nothing on the component says `--dn-table-cols` is required (`DataTable.css:3` has no fallback, so a view that forgets it gets a collapsed grid).
- **Fix**: Inline the class joining like the siblings, add one-line JSDoc to `head`/`header`/`tone`, and state on `DataTable` that the view must set `--dn-table-cols`.
- **Decision**: FIXED — class joining inlined like sibling components, JSDoc on head/header/tone, DataTable.className required and documented as carrying --dn-table-cols

### F3 — `ErrorState.message` trafia do `<p>`

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/components/ErrorState.tsx:6, src/components/SourceState/SourceState.tsx:46
- **Detail**: `message` is now a `ReactNode` and is rendered inside `<p className="dn-state-text">`; a future caller passing a block element (`<ul>`, `<div>`) would produce invalid nesting.
- **Fix**: Add a one-line doc on `message`: inline content only (text, `<code>`, links).
- **Decision**: FIXED — doc on ErrorState.message: inline content only
