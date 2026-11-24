<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Tabela dni miesiąca w „Czas pracy”

- **Plan**: context/changes/worktime-days-table/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

All 12 planned changes match their intent and contract. Nothing listed in "What We're NOT Doing" was done: no command, event or migration was added, and `lib.rs`, `api.ts` and `store/` are untouched. The automated criteria were rerun and are green:

- fmt, clippy and the 29 `norm_tests`
- `cargo test`: 566 passed, 0 failed
- build, lint and format:check
- the rg check for an 8 h rule finds nothing
- `useWorkTimerSource` is used only in the provider

The manual rows 2.7–2.13 were confirmed by the user in `npm run tauri dev`. Security and data safety are clean: the SQL is parameterised, the day is parsed by `parse_day`, and the tests use invented data.

## Findings

### F1 — The "Dzień wolny" checkbox stays checked and disabled after the table changes the same day back

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/worktime/useWorkEntries.ts:137-143
- **Detail**: The flaw already existed; the table makes it reachable. `dayOffRequest` is never cleared after a successful write. `pending` is derived as `dayOffRequest.off !== storedOff`. Steps:
  1. Tick "Dzień wolny" for day X in "Wybrany dzień" (request `{X, true}`).
  2. Clear X with the "Wolne" switch in the table.
  3. `storedOff` is now false, so `pending` is true. The checkbox shows ticked and disabled although X is a workday, and it stays that way until the view remounts.

  "Pokaż dzień" shows the same day in both places, so this path is a normal one.
- **Fix**: Drop `dayOffRequest` once the loaded day agrees with it, as `useDayOffToggle` does, or right after a successful `setDayOff`.
- **Decision**: FIXED

### F2 — The weekend muted tone does not reach the day button

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/DaysTable.tsx:53-62 (cause: src/components/Button/Button.css:10)
- **Detail**: The cell gets `tone="muted"`, but `.dn-btn` sets `color: var(--ink)` explicitly. The weekend label ("21.11 sb") therefore renders at full brightness, while the rest of the row is muted.
- **Fix**: In `DataTable.css`, add `.dn-table-cell-muted .dn-btn, .dn-table-cell-faint .dn-btn { color: inherit; }`.
- **Decision**: FIXED

### F3 — The hatching depends on rule order and is nearly invisible on the "today" row

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/components/DataTable/DataTable.css (`.is-current`, `.is-hatched`), src/styles/tokens.css:90
- **Detail**: `.is-current` uses the `background` shorthand, which resets `background-image`. The stripes survive only because `.is-hatched` comes later. The stripe colour `--line-subtle` (#1f242d) differs from `--surface-raised` (#1c212a) by only 3 per channel, so a day that is both today and a day off shows almost no hatching.
- **Fix**: Make `.is-current` use `background-color`, and add a separate stripe token in `tokens.css` that contrasts with both `--surface` and `--surface-raised`.
- **Decision**: FIXED

### F4 — A disabled switch drops keyboard focus

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/DaysTable.tsx:94-102
- **Detail**: `disabled={busy}` takes the switch out of the tab order the moment it is pressed, so focus drops to `<body>`. The project already has a pattern for busy actions that keeps focus: `aria-disabled` (`Button.css:66`, "looks disabled but keeps focus").
- **Fix**: Use `aria-disabled={busy}` and ignore `onChange` while busy.
- **Decision**: SKIPPED

### F5 — `useDayOffToggle` edge cases: errors are hidden instead of cleared, and settling waits only for the report

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/worktime/useDayOffToggle.ts:29-38, 55-57
- **Detail**: The plan says a month change *clears* the error. The hook only *hides* it, so after returning to that month the old refusal shows again. A request also settles only when the report agrees. If `notify` fails after a successful write (it is only logged) on a past month, which has no polling, the switch stays busy until focus or visibility changes. The render-phase `setRequests` itself is correct: it is guarded and idempotent under StrictMode.
- **Fix**: Reset `error` when the report's month changes, and after a successful `setDayOff` call `retry()`/refresh the report so settling does not depend on the event alone.
- **Decision**: SKIPPED

### F6 — Day-button names and `aria-current`

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/DaysTable.tsx:57, src/components/DataTable/DataTable.tsx:39
- **Detail**: The visible text is "19.11 cz", but the accessible name is "Pokaż dzień 19.11". The weekday is missing, and the name does not contain the visible label (WCAG 2.5.3). For a calendar row, `aria-current="date"` would be more precise than `"true"`.
- **Fix**: Use `aria-label={`Pokaż dzień ${formatDayLabel(day)}`}` and give `current` the type `boolean | "date"`.
- **Decision**: SKIPPED

### F7 — Duplicate formatters: `DD.MM` in three places, and two `formatHm` functions

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/DaysTable.tsx:8-11, src/worktime/WorkTimeView.tsx (error Notice), src/worktime/WorkTimeEntries.tsx:11
- **Detail**: `dm()`, an inline template in the Notice and `shortDay` all build `DD.MM`. `DaysTable` imports `formatHm` from `../components` while the rest of `worktime/` uses its own from `./format`.
- **Fix**: Move `shortDay` to `format.ts` and use it in all three places, and import `formatHm` from `./format` in `DaysTable`.
- **Decision**: SKIPPED

### F8 — One failing source shows two ErrorStates on screen

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/WorkTimeView.tsx (panel „Dni miesiąca”), src/worktime/MonthReport.tsx
- **Detail**: When the month report fails, an ErrorState with a retry button appears both under the tiles and in the "Dni miesiąca" panel. The plan accepted this (AGENTS.md wants loading/error/data in every view), but it is noisy.
- **Fix**: Leave it as it is (the plan accepted it), or in the table panel show a short reference to the error above.
- **Decision**: SKIPPED

### F9 — Rust: `day_bounds` runs for every day inside `status()`, and the day classification is duplicated

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src-tauri/src/work_time/report.rs:90-108, 131-152
- **Detail**:
  - `compute` now runs `day_bounds` for every day of the month, and `status()` calls `compute` for the header. In a zone whose midnight falls in a DST gap, the header would fail for the whole month. Europe/Warsaw is not affected (its change is at 02:00/03:00).
  - `kind_of` and `day_kind` repeat the weekend/day-off rule.
  - The extra cost (31 rows per status call) is negligible: microseconds, and status is not called every tick.
- **Fix**: Leave it for now, or split `compute` into a month-figures-only version for `status()` and a full one for the report.
- **Decision**: SKIPPED
