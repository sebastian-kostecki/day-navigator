<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Week summary in "Czas pracy"

- **Plan**: context/changes/week-summary/plan.md
- **Mode**: Deep (verification inline; no user reachable, fixes applied by the reviewer)
- **Date**: 2026-11-26
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 5 warnings, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING (F1, F4) |
| Lean Execution | PASS |
| Architectural Fitness | WARNING (F3) |
| Blind Spots | PASS |
| Plan Completeness | WARNING (F2, F5, F6) |

## Grounding

Paths 10/10 exist (`report.rs`, `mod.rs`, `commands.rs`, `norm.rs`, `lib.rs`, `types.ts`, `api.ts`, `DaysTable.tsx`, `WorkTimeView.tsx`, `WorkTime.css`; README `:55-59`, `:125`; CHANGELOG `[Unreleased]`). Symbols 12/12: `days_off_between` / `classify` private, loop `report.rs:144-168`, `in_day` / `Entry` private in `mod.rs` (visible to children), `run_with_settings` / `Policy::Strict`, `parse_day`, `manual_only()` `pub(super)`, proptest `norm_tests.rs:566-616`, `liveFigures` private, `todayTotalMs`, `Panel` `meta` / `footer`, `StatStrip`, `--dn-table-cols`. Dates 21/21 checked (weekdays, Easter 5.04.2026, DST 29.03 / 25.10.2026, 24.12 holiday since 2025, 26.12.2026 a Saturday → `Weekend` + `day_off_owed`). `day_report` has 7 arguments (clippy `too_many_arguments` fires above 7); `work_time` is `pub mod`, so the Phase 2 `pub fn week_report` without a caller is not dead code. Baseline `rg` checks 1.4, 4.3-4.6 pass today. Brief ↔ plan consistent. No `lessons.md`, no `docs/reference/contract-surfaces.md`.

## Findings

### F1 — Saturday holiday loses its name and owed-day hint

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Phase 4 — Panel component (kind label)
- **Detail**: The label was an exhaustive switch on `DayKind` with `weekend: "weekend"` and the owed hint under `holiday`. `day_off_owed` is true only for a Saturday holiday, whose `kind` is `Weekend` (`report.rs:98-111,166`), so 26.12.2026 would show "weekend" and the promised hint was dead code; a Sunday holiday also lost its name, unlike `DaysTable.tsx:82-86`.
- **Fix**: Holiday name (plus hint when `dayOffOwed`) whenever `row.holiday !== null`; otherwise an exhaustive switch ending in a `never` check (A15).
- **Decision**: FIXED

### F2 — "today" in the panel and the hook's local date undefined

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 — Week hook, Panel component
- **Detail**: Rows compare `row.day === today` and the hook sends "the local date at request time", but `localToday` is private in `WorkTimeView.tsx:26-28` and the plan named no source for either.
- **Fix**: `useWeekReport.ts` exports `localDate()` (copy of the formula), called inside `refresh` and at render in `WeekSummary`.
- **Decision**: FIXED

### F3 — Week code not insulated from S-02's type changes

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Critical Implementation Details (Merge with S-02); Phase 2
- **Detail**: S-02's plan changes `days_off_between` to return a `DaysOff` map, `classify`, `day_norm_ms` (adds `daily_norm_ms`), `set_day_off` (`DayMark` instead of `bool`) and adds `DayKind::HalfDayOff`. The plan only anticipated the loop conflict and a new `DayKind`; the 8 h guard did not cover `week_tests.rs`.
- **Fix**: Merge note lists S-02's changes; `week.rs` binds the days-off result untyped and only passes it on; tests mark days off through one `mark_off` helper; new criteria 2.10 / 2.11; `halfDayOff` label at merge.
- **Decision**: FIXED

### F4 — PRD left saying the week view is not built

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: End-State Alignment
- **Location**: What We're NOT Doing; Decisions D4; Phase 4 Docs
- **Detail**: `prd.md:234` lists "widok tygodnia" among things considered but not built. The S-04 and S-07 plans each remove their own word from that sentence and expect S-08 to do the same (S-07 plan `:326`). The plan's "no PRD edit" went beyond the user decision, which forbids only new FRs (`change.md`).
- **Fix**: Phase 4 deletes "widok tygodnia, " and adds one "built as" line, no FR; criterion 4.15; D4 reworded to the user's words; A14 (agent default, user may revise).
  - Strength: The PRD stays true and matches the sibling slices.
  - Tradeoff: One more shared-file hunk (expected conflict with S-04 / S-07, resolved by keeping each removal).
  - Confidence: HIGH — the same pattern is already planned twice.
  - Blind spot: The user may prefer to sync the PRD at milestone close instead.
- **Decision**: FIXED (A14; user may revise)

### F5 — No test pins the serialized `WeekReport` shape

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 — Tests
- **Detail**: The Rust type lands in Phase 2 and its TS union in Phase 3, which relies on camelCase keys, lowercase `phase` and `null` per phase; nothing checked that shape, and the derives were not listed.
- **Fix**: Serde case per phase (like `norm_tests.rs:803-814`); derives listed for `WeekPhase` / `WeekReport`.
- **Decision**: FIXED

### F6 — Small ambiguities for a fresh implementer

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phases 1-3
- **Detail**: `week_days` returned a tuple but said "map an overflow to an error if a Result is needed"; helpers said "today_tests.rs style" but `today_tests.rs` has no `d` / `add_entry`; how to start a running entry in a test was unstated; `rg` is not on the PowerShell PATH; Phase 1 had no manual check though it refactors the month table.
- **Fix**: Exact `week_days` arithmetic (saturating like `month_days`); helpers from `norm_tests.rs:28-56`; `start(...)` for the running entry; note that `rg` runs in Git Bash; manual row 1.8; Phases 2-3 say why they have no manual row.
- **Decision**: FIXED

## Triage summary

- Fixed: F1, F2, F3, F4 (A14, user may revise), F5, F6 (6)
- Verdict after fixes: SOUND
