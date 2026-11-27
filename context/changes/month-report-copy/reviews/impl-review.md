<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Kopiowanie raportu miesiąca do schowka — plan implementacji

- **Plan**: context/changes/month-report-copy/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 2 observations

Reviewed `git diff d34904a..HEAD` (`160cbf7` p1 Rust builder and command, `7d1f881` p2 UI button and docs) against the plan, `reviews/plan-review.md`, `research.md` and `AGENTS.md`. Run non-interactively (no user reachable), so every finding is left `PENDING` for a separate fix agent.

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

### Success criteria (re-run in this review)

| Criterion | Result |
|---|---|
| 1.1 / 2.2 `cargo test` | pass: 556 lib tests, including all 10 `work_time::month_text_tests`, plus every integration binary |
| 1.2 / 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | pass |
| 1.3 / 2.1 `npm run lint`, `npm run format:check`, `npm run build` | pass |
| 1.4 `git grep -n "log::" -- src-tauri/src/work_time` | exactly the 4 old lines, all in `commands.rs` (`:48`, `:100`, `:102`, `:110`) |
| 1.5 forbidden patterns in `month_text.rs` and `git diff --quiet d34904a -- report.rs norm.rs types.ts Cargo.* capabilities tauri.conf.json` | no hits, exit 0 |
| 2.3 `invoke` only in `api.ts`, no `console.`, no literal colours in `WorkTime.css` | pass |
| 2.4 `git diff --quiet d34904a -- package.json package-lock.json …` | exit 0 |
| 2.5 "Kopiuj miesiąc" and "pl-PL" in README and CHANGELOG | found in both |
| Manual 2.6–2.12 | unchecked on purpose: they need the built app. They are pending, not a finding. |

Deliberate-break check (the p1 agent did not re-run it after adding the weekday assertion): this review changed `Weekday::Mon => "pn"` to `"wt"` in `month_text.rs` and ran `cargo test --lib month_text`. `current_month_matches_the_report` failed at `month_text_tests.rs:224`, so the weekday assertion pins the behaviour. Then the file was restored with `git checkout` and the tree is clean.

### Plan adherence (planned file vs actual)

| File | Verdict | Note |
|---|---|---|
| `src-tauri/src/work_time/month_text.rs` | MATCH | One `store.read` with `report::compute` and `in_day` over `month_days` + `day_bounds`. Pure `build`, no `tauri`, clock or logging. `cell`/`hm`/`hours`/`signed_hours` are `pub(super)`. `signed_hours` rounds on `|m|`. `MonthReport`, `NormFigures` and `DayReport` are destructured without `..` (`days: _` is explicit). `MonthPhase`, `DayKind` and `Weekday` are matched without `_`. Module comment on clipboard-only. |
| `src-tauri/src/work_time/mod.rs` | MATCH | Two lines in the sorted lists. |
| `src-tauri/src/work_time/commands.rs` | MATCH | Appended after `set_day_off`; `parse_day` first, then `run_with_settings(…, "month_report_text", Policy::Strict, …)`. Same shape as `work_month_report`; no `notify`, no events, no new `log::`. |
| `src-tauri/src/lib.rs` | MATCH | One line right after `work_month_report`. |
| `src-tauri/src/work_time/month_text_tests.rs` | MATCH | Every planned case is present: cell, numbers, current month vs report, DST + Saturday holiday, future month, shape, sentinel. |
| `src/worktime/api.ts` | MATCH | `workMonthReportText` right after `workMonthReport`. |
| `src/worktime/useMonthCopy.ts` | MATCH | The outcome is keyed to the clicked month and derived in render. No `useEffect` reset and no ref reads. The command error and the clipboard error are caught in separate `try` blocks. `writeText` runs right after the `await` of the command. The message carries `e.name` only. The text is never kept in state. |
| `src/worktime/MonthCopyButton.tsx` | MATCH | An always-mounted `role="status"` holds `Notice tone="info" compact role={null}`. The error uses `Notice tone="error" compact` (default `alert`). The button is `quiet`/`sm` with `disabled={!enabled \|\| busy}` and shows "Kopiuję…" while busy. |
| `src/worktime/WorkTime.css` | MATCH | `.wt-month-copy` comes right after `.wt-month-name`. It is layout only and uses tokens only. |
| `src/worktime/WorkTimeView.tsx` | MATCH | One import, then `dn-spacer` and the button after `wt-month-switch`. `enabled={report.state === "loaded"}`, and `useMonthReport` already reports another month's result as `loading`. |
| `README.md`, `CHANGELOG.md` | MATCH | One appended point in each, with the pl-PL note. The README says the H:MM columns are the safe fallback. |

No EXTRA code files. The other files in the diff are the change's own docs (`change.md`, `plan-brief.md`, `plan.md`, `research.md`, `reviews/plan-review.md`).

### Checked and found sound

- **Same numbers as the screen.** `hm` matches `formatHm` (floor, clamp at 0). `signed_hours` truncates toward zero like `formatSigned` (`0,00` under a minute). The empty "Suma" rule `worked_ms == 0 && deviation_ms.is_none()` matches `DaysTable.tsx` `showSum = workedMs > 0 || (day <= today && normMs > 0)`, because `norm.rs` sets `deviation_ms` to `None` with zero work exactly for a future day or a zero norm. The weekday labels match `WEEKDAYS`. The owed-day suffix matches `DaysTable.tsx:84`. The today row uses the same `now` and `portion` as `compute`, so it is the live value at the click. Month phases: the summary has a fixed 13 rows with empty cells for `None`, and the tests cover the past, current and future phases. DST: `day_bounds` per day and the 4:00 entry on 2026-10-25 are tested. `Od`/`Do` uses the same local-date comparison as `localLabel`.
- **Merge safety for S-02/S-08.** A new field in `MonthReport`, `NormFigures` or `DayReport`, or a new variant of `DayKind` or `MonthPhase`, is a compile error in `month_text.rs`. Shared-file edits are one to three appended lines each (`mod.rs`, `commands.rs` at the end, `lib.rs`, `api.ts`, `WorkTimeView.tsx`, `WorkTime.css`, README, CHANGELOG), so a merge conflict can only be about context.
- **Cell cleaner.** Tabs, every `char::is_control()` (so `\r`, `\n`, C0, DEL, C1) and `U+2028`/`U+2029` become a space. The trim (Unicode whitespace, NBSP included) runs before the guard, so `"\t=1"` → `'=1`. Every formula start `= + - @` plus `"`, `'` and the full-width forms gets `'`. The holiday name goes through `cell` too. Numeric cells never start with `+`, and a negative deviation is a plain number (`-1,50`), not a formula.
- **Note privacy.** The text leaves Rust only as the command's `Ok` value. `execute` logs only the command name and `e.kind()`. There is no `notify`, no event and no `console.`, and the hook does not keep the text in state.
- **AGENTS.md conventions.** The command is `async` through `execute` → `blocking` (spawn_blocking) → `with_store`. There is no new error variant, so the TS mirror is unaffected. `invoke` is only in `api.ts`. There is no new dependency, permission, CSP change or migration, and `report.rs`, `norm.rs` and `types.ts` are unchanged.
- **React.** The derived per-month state and the stale-month race are handled: a late result for an old month is stored under that month and not shown. A double click is blocked because a click is a discrete event, so React commits `busy` before the next one. A `setState` after unmount is a no-op in React 19. ESLint (react-hooks 7) passes.

## Findings

### F1 — Test maps DayKind labels by declaration order

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/work_time/month_text_tests.rs:236-239
- **Detail**: The expected `Rodzaj` is `["roboczy", "wolny", "święto", "weekend"][*kind as usize]`, so it relies on the declaration order of `DayKind` in `report.rs:24-29`. It still catches swapped labels. But if S-02 adds a variant (for example a half day off), the test fails with an index-out-of-bounds panic or a shifted label. It does not point at the missing label, and the merge note asks the branch merged second to "add one case in the test", which a positional array does not show.
- **Fix**: Replace the array index with an explicit `match kind { DayKind::Workday => "roboczy", DayKind::DayOff => "wolny", DayKind::Holiday => "święto", DayKind::Weekend => "weekend" }` (no `_`), so a new variant is a compile error in the test as well.
- **Decision**: PENDING

### F2 — Entry ending exactly at midnight is not pinned by a test

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/work_time/month_text_tests.rs:85-112 (fixture `november`)
- **Detail**: The Format TSV section (and plan-review F6b) says that an entry ending exactly at local midnight (22:00–00:00) shows `Do` with the next day's date (`localLabel` rule), and that it has no row in the next day because its part there is 0. No test asserts this. The code is correct: `local_label` compares the local date prefix, and a part `<= 0` is skipped. A later rewrite of `local_label` that compares against the day bounds could change it without any test failing.
- **Fix**: In `current_month_matches_the_report` (or a new small test), add an entry `2026-11-17T22:00`–`2026-11-18T00:00` and assert the row `2026-11-17\t22:00\t2026-11-18 00:00\t2:00\t2,00\t` is in the text and that no row for `2026-11-18` comes from it (`entry_count` of 18.11 stays as the report says).
- **Decision**: PENDING

_F1 and F2 were applied in the commit "test(month-report-copy): pin day-kind labels and midnight end"._
