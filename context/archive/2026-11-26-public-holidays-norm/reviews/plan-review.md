<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Polish public holidays in the work-time norm

- **Plan**: context/changes/public-holidays-norm/plan.md
- **Mode**: Deep (risky claims checked inline against the code, no sub-agent: each was a direct file read or grep)
- **Date**: 2026-11-26
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 4 warnings, 4 observations
- **Run**: non-interactive. Every finding was fixed in the plan by the review agent; choices are recorded in the plan's Decisions as R1–R6 ("default chosen by agent, user may revise").

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

Grounding: 14/14 paths ✓ (2 new files, `settings/` dir absent as expected), 18/18 symbols and line refs ✓ (`norm.rs:24,30,44,104`, `report.rs:90,100,111,191,217`, `mod.rs:188,239`, `reminders.rs:69,143`, `commands.rs:20-43,129`, `settings.rs:405-406`, 1692 lines, `DaysTable.tsx:40,52,94`, `WorkTimeView.tsx:60-85,150`), brief↔plan ✓.

Checked and confirmed:

- Holiday numbers: November 2026 has 21 weekdays (1.11 is a Sunday) minus 11.11 = 20 workdays, 160 h. December 2026 has 23 weekdays minus 24.12 and 25.12 = 21, 168 h; 26.12 is a Saturday. 15.08.2026 is a Saturday, 5.04.2026 a Sunday. 2026 has 14 holidays, 8 of them Monday–Friday. The Easter dates in the test list and Corpus Christi 2026-06-04 are right; the written Meeus/Jones/Butcher formula is correct and has no negative remainder.
- No exhaustive `match` on `DayKind` in Rust and no `Record<DayKind, …>` in TS, so adding the variant only breaks the call sites the plan lists.
- A child module (`settings/work_time.rs`) can read `super::SUPPORTED_VERSION`; `settings`, `work_time` and `settings_file` are `pub` in `lib.rs`, so integration tests reach the new types.
- `serde` maps `"holidays": null` to `None` for `Option<Value>`, so the defaults rule holds without extra code.
- `tempfile` and `proptest` are dev-dependencies.
- `tests/days_off_crash.rs` starts at 2026-01-01 (Nowy Rok): the plan's "holidays off" migration covers it.

## Findings

### F1 — Phase 3 too large for one fresh sub-agent

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Completeness
- **Location**: Phase 3
- **Detail**: Phase 3 combined the norm change, the classification, the guard, a new error, four caller files, two contract types (`DayKind` and `DayReport`) with their TS mirrors, the `daysOff` filter, ~50 test call-site migrations in five files and about a dozen new tests. A clean-context Sonnet sub-agent has to hold all of it at once, and a red gate late in the phase is hard to localize.
- **Fix**: Split into Phase 3 (rule, signatures, `Holiday` kind and error with TS mirror, test migration, kind/norm tests) and a new Phase 4 (`DayReport.holiday`, `dayOffOwed`, `daysOff` filter, TS `DayReport` mirror, name/hint/serialization tests). Later phases renumbered 5–7, Progress renumbered to match, cross-references updated. Each phase changes a Rust contract together with its TS mirror, so all gates stay green per phase. Also added concrete fixture pointers and the integration-test helper location (`tests/common/mod.rs`).
  - Strength: Each phase has one reason to fail; Phase 4 is small and purely additive.
  - Tradeoff: One more commit and session.
  - Confidence: HIGH — Phase 4 touches only `compute`, two structs and one TS type.
  - Blind spot: None significant.
- **Decision**: FIXED (R1)

### F2 — `SettingsInvalid` text misleads on a file-level problem

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 (loader), old Phase 4 / new Phase 5 (error text)
- **Detail**: The loader returns the same invalid result for an unreadable file, bad UTF-8, malformed JSON or a wrong `version` as for a bad `workTime` value, and the strict report command maps it to `SettingsInvalid`. The planned text ("holidays to „pl” albo false") then points the user at the wrong place after a JSON typo in another block. Before this slice such a typo did not affect the work-time report at all.
- **Fix**: Kept the strict behaviour (safer than silently using the defaults, R2) and changed the text to name both causes: "Nie można odczytać bloku „workTime” z settings.json: plik musi być poprawnym JSON-em z „version”: 1, a holidays to „pl” albo false."
- **Decision**: FIXED (R2)

### F3 — Dashboard month tiles are an unlisted consumer of the strict report

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Desired End State, Q1, old Phase 4 manual checks, Phase 7 README
- **Detail**: `src/todo/TodoView.tsx:150` calls `useMonthReport`, and `src/todo/summary.ts:28-35` turns an error into "raport niedostępny" on the three month tiles. With an invalid block the dashboard therefore shows an error too, but the end state, Q1, the manual check and the README note named only "Czas pracy".
- **Fix**: Accepted as correct behaviour (a surfaced failure, R3) and added it to the end state, Q1, the Phase 5 manual check (Progress 5.4) and the README sentence in Phase 7.
- **Decision**: FIXED (R3)

### F4 — Three verification commands do not work as written

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2, old Phase 5 / new Phase 6, old Phase 6 / new Phase 7
- **Detail**:
  - `cargo test work_time_settings` filters by test path, but the tests live in `settings::work_time::tests`, so it runs zero tests and passes vacuously.
  - `git diff -U0 -- src/**/*.css` "adds no `#`…" is not a runnable pass/fail check.
  - `grep -n "kalendarza świąt" prd.md` returns nothing only if the new Socrates resolution avoids that phrase; "z lokalnego kalendarza świąt" is a natural wording and would fail the check. `grep -c "FR-0"` counts mentions, not ids.
- **Fix**: `cargo test settings::work_time::`; a Git Bash pipeline with `:(glob)` / `:(exclude)` pathspecs and a `grep -E` for hex, `rgb(a)(` and `hsl(a)(`; check the Non-Goal line itself plus the absence of "automatycznego kalendarza świąt"; count unique `FR-[0-9]+` ids.
- **Decision**: FIXED

### F5 — Settings helper for parallel-slice callers left unspecified

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Implementation Approach (parallel slices), old Phase 4 / new Phase 5 §2–3
- **Detail**: The policy helper was private and unnamed ("`run` (or a sibling helper)"). An S-03 tray caller of `status` that merges second would most likely pass `WorkTimeSettings::default()` and ignore the user's `false`. The parallel-slice list also said "one `HowCounted` paragraph" while Phase 6 changes two.
- **Fix**: Named the helper `pub fn work_time_settings(path: Option<&Path>, policy: Policy)` with `pub enum Policy { Strict, Lenient }` in `work_time/commands.rs`, plus a `run_with_settings` sibling of `run` (`run` unchanged). The reminder round uses it, and the parallel-slice note says other callers do too. Fixed the paragraph count.
- **Decision**: FIXED (R4)

### F6 — Easter for years before 1583 left open

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1
- **Detail**: "Valid from 1583; `None` outside the range chrono can build" does not say what happens for 1–1582. `parse_day` accepts years 1–9999, so the report can ask for them.
- **Fix**: `None` for `year < 1583`. `Pl` then has the fixed dates only, and nothing panics. Added tests for years 1, 1582 and 9999.
- **Decision**: FIXED (R5)

### F7 — Hours-bar rule on a holiday row ambiguous

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: old Phase 5 / new Phase 6 §1
- **Detail**: "The bar, if any hours were worked, stays before it" conflicts with today's `showBar = !weekend || workedMs > 0` (`DaysTable.tsx:45`), which would draw an empty bar on every weekday holiday.
- **Fix**: State that a holiday row draws the bar only when `workedMs > 0`, like a weekend.
- **Decision**: FIXED (R6)

### F8 — Sum invariant not tested with holidays

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 tests
- **Detail**: The proptest `day_rows_add_up_to_the_month_figures` (`norm_tests.rs:541-584`) was only moved to "holidays off". The invariant it guards (day norms add up to the month norm, day deviations to the month deviation) is exactly what a holiday could break, because `day_norm_ms` and `workdays` both change.
- **Fix**: Run the proptest under both calendars; under `Pl` it skips marking holidays.
- **Decision**: FIXED

## Triage summary

- Fixed: F1, F2, F3, F4, F5, F6, F7, F8 (8)
- Skipped / Accepted / Dismissed: none
- Verdict after fixes: REVISE → SOUND
