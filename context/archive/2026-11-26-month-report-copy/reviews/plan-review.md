<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Kopiowanie raportu miesiąca do schowka — plan implementacji

- **Plan**: context/changes/month-report-copy/plan.md (reviewed at bdd5ef5)
- **Mode**: Deep (claims verified inline against the code, no sub-agent)
- **Date**: 2026-11-26
- **Verdict**: REVISE → SOUND after the fixes below (applied non-interactively, no user reachable)
- **Findings**: 0 critical, 4 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

Grounding: 14/14 paths ✓ (`report.rs`, `norm.rs`, `mod.rs`, `rules.rs`, `commands.rs:213-237`, `lib.rs:220`, `store/mod.rs:142-147`, `today_tests.rs`, `api.ts`, `WorkTimeView.tsx:173-196`, `WorkTime.css`, `OwnTaskEditPanel.tsx:134-141`, `GithubView.tsx:182-195`, `Notice.tsx`), 14/14 symbols ✓ (`compute` is `pub(super)`, `in_day` and the `Entry` fields are private to `work_time` and so visible from a child module, `month_days`, `day_bounds`, `portion`, `local_string`, `parse_day`, `run_with_settings`, `Policy::Strict`, `WEEKDAYS`, `formatHm`, `formatSigned`, `workTimeErrorMessage`, `WorkTimeSettings::default()` = `HolidayCalendar::Pl`), exactly 4 `log::` lines in `work_time` ✓, the baseline greps of 2.3 pass ✓, brief↔plan ✓, Progress↔criteria 17/17 ✓ (script check, no checkboxes outside Progress).

### Verified claims

- **Same numbers as the screen.** The "empty Suma" rule `worked_ms == 0 && deviation_ms.is_none()` is equivalent to `DaysTable.tsx:44` (`norm.rs:63-74`: with `worked == 0`, `None` ⇔ `day > today` or `norm == 0`). The rounding mirrors `formatHm` (floor, clamp at 0) and `formatSigned` (truncate toward zero, `0:00` under a minute). Decimal hours `(m*100+30)/60` never tie (10·m ≡ 3 mod 6 has no solution). Every non-zero |m| gives at least `0,02`, so `-0,00` never appears. Phases: Future leaves `norm_to_date`, `deviation`, `remaining`, `needed`, `after` all `None`; Past sets only `norm_to_date = norm` and `deviation`; Current sets all of them, and `needed` only when `after > 0` (`norm.rs:113-151`). The block-1 table with empty cells fits all three phases. The live today row is equal by construction: the same `now` and the same `portion` as `status`, as proved in `today_tests.rs:31-43`.
- **Test dates.** 2026-11-11 is a Wednesday, 2026-11-13 a Friday (so a day off can be set), 2026-11-23 a Monday, 2026-08-15 a Saturday. 2026-10-25 is the DST end, so 01:00–04:00 local is 4 h. November has 30 days, so block 2 has 31 rows. The literal example rows check out against the column list.
- **Activation after `await invoke`.** Chromium requires a focused document for `writeText`. Transient activation lasts about 5 s and `writeText` does not consume it, so a millisecond command keeps it valid. The real failure mode is a loss of focus (see F3).

## Findings

### F1 — "Same as the screen" overstated for the month summary

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Desired End State, bullet 2
- **Detail**: The plan promises that every copied cell equals the screen. The month tiles (`MonthReport.tsx`) are a snapshot. They refresh every 60 s, on `work-time-changed` and on focus (`useMonthReport.ts:49-112`). The copy is recomputed at click time. While the counter runs, or right after a change from the tray, "Przepracowano" and "Odchylenie" in the copy can be up to a minute fresher than the tiles. Research noted this; the plan dropped it. A manual tester comparing tiles with the copy would report a false bug.
- **Fix**: State the difference in the Desired End State. Only the today row is promised live, which 2.11 already checks.
- **Decision**: FIXED (applied to plan.md)

### F2 — Exhaustive `match` does not catch new report fields at merge

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 1 §1 Contract, Merge notes (S-02)
- **Detail**: The merge safety net is the `match DayKind` without `_`. It fires only if S-02 models a half day off as a new variant. If S-02 adds a field instead (for example `half_day_off: bool` on `DayReport`, a daily norm in `NormFigures`, or a different `days_off` shape), `month_text.rs` still compiles. The copy then silently leaves the value out, and the tests stay green because they compare only the fields they know. S-08 can add fields in the same way.
- **Fix**: Read `MonthReport`, `NormFigures` and `DayReport` through destructuring without `..`. Any new field is then a compile error at merge, and the branch merged second chooses a column, a row at the end of the summary, or an explicit `field: _` with a comment.
- **Decision**: FIXED (Critical Implementation Details, Phase 1 contract, Merge notes, brief)

### F3 — A clipboard failure cannot be diagnosed from the UI

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details ("Aktywacja schowka"), Phase 2 §2 hook, Implementation Note
- **Detail**: The fallback decision depends on whether the manual check fails with `NotAllowedError`. The planned message "Nie udało się skopiować do schowka" hides the error name, so the user cannot tell a focus or permission refusal from a missing API (`TypeError`) without DevTools. The plan also gave the wrong hard condition, "focus or fresh activation". Chromium always needs a focused document: switching windows during the call, or a focused DevTools window, gives "Document is not focused". Activation lasts about 5 s and is not consumed, so it is not the risk after a millisecond `invoke`. The longest wait is the store lock during the health round's backup.
- **Fix**: Show `e.name` in the message (`Nie udało się skopiować do schowka (NotAllowedError)`). The name carries no content. Catch the `writeText` call in its own `try`. Document the focus requirement and the 5 s activation window (a Chromium constant, not verified in WebView2). The manual check 2.6 clicks in the focused app window, not from the DevTools console.
- **Decision**: FIXED

### F4 — Decimal comma outside pl-PL fails silently, not visibly

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §5 Documentation, Decision 10
- **Detail**: The README line only says the numbers are for pl-PL. In an en-US sheet, `7,50` can be read as a grouped number (750) rather than as text, which corrupts the hours ×100 without any warning. The `H:MM` columns parse correctly in any locale, so they are the safe fallback, and the README can say so. The pl-PL default stays (user's locale, agent default).
- **Fix**: Extend the README sentence and Decision 10. In another locale the decimal hours may paste as text or as a wrong number, and the `H:MM` columns stay correct. Criterion 2.5 is unchanged.
- **Decision**: FIXED

### F5 — Manual check 2.10 is racy as written

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 Manual 2.10 (and Progress 2.10)
- **Detail**: To break `settings.json` the user leaves the app. Coming back fires `focus`, which refetches the report (`useMonthReport.ts:58`). The report then becomes `SettingsInvalid` and disables the button, so "kliknięcie pokazuje komunikat błędu" may be impossible to perform. The real invariant is "never Skopiowano. with a broken block".
- **Fix**: Reword 2.10 in both places, keeping the index. Either the first click after returning shows the error or the button is already disabled, and never "Skopiowano.".
- **Decision**: FIXED

### F6 — Small precision gaps for a fresh implementer

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Format TSV, Phase 1 §1 and §5, Phase 2 §2, Implementation Note
- **Detail**:
  - (a) `signed_hours`: applying `(m*100+30)/60` to a negative `m` with Rust's truncating `/` gives `-1,49` for −90 min. The rounding must run on `|m|`, with the sign added afterwards. A test catches it, but the plan should say it.
  - (b) "chwila wypada w dniu wiersza" leaves the midnight tie open. The screen's `localLabel` compares the local date string, so an end at exactly 00:00 shows the next day's date.
  - (c) The future-month test did not list `Pozostało`, `Średnio / dzień` or `Dni robocze po dziś` as empty.
  - (d) Example row 4 ends in an invisible `\t`.
  - (e) The hook should derive per-month state in render, as `useMonthReport.ts:95,116` does, without a `useEffect` reset or ref reads in render, because `eslint-plugin-react-hooks` 7 `recommended` is in `npm run lint`.
  - (f) `npm install` can add a lone `"license": "MIT"` to `package-lock.json` and break 2.4.
  - (g) Text cells that look like a date or number (a note `12.11`, `Miesiąc` = `2026-11`) get converted by the sheet. This is a known limitation, accepted as an agent default.
- **Fix**: One clause each in the plan, all applied.
- **Decision**: FIXED

## Checked and left as is

- AGENTS.md: there is no new error variant, so `WorkTimeError` with its TS mirror is untouched. The command returns `String`, so no type mirror is needed and Phase 1 / Phase 2 do not split a contract. The command is `async` through `run_with_settings` → `execute` → `blocking` (spawn_blocking) and never calls the store from a sync command. Logging is limited to the existing kind-only lines, and no `notify` or event is added. CSS uses tokens only. An action failure is shown with `Notice tone="error"`, which fits the `Notice` doc ("error = an action or part failed"), not with `ErrorState`.
- Phase sizing: Phase 1 is one module, one test file and four one-line edits. Phase 2 is one function, one hook, one component, one CSS rule, two lines in the view, and the docs. Each phase fits one sonnet sub-agent with the gate commands listed.
- Merge risk: S-08 will likely add its lines right after `work_month_report` in `lib.rs` and `api.ts`, a context-only conflict that the Merge notes cover ("keep both"). S-07 and S-04 do not overlap.
- Fallback (Rust-only `tauri-plugin-clipboard-manager`): correctly scoped as a separate change.

## Open for the user (no blocker; agent defaults kept)

- pl-PL decimal comma only (no en-US variant). Revisit if the sheet is ever shared with another locale.
- Whether a note that looks like a date should be forced to text (a `'` prefix on every note) at the cost of a visible apostrophe in some apps.
