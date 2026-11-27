<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Konfigurowalna norma dzienna i pół dnia wolnego

- **Plan**: context/changes/configurable-daily-norm/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 5 observations

Commits reviewed: `3f24630` (p1), `7e02106` (p2), `6968a74` (p3), `9413214` (p4) on top of develop `d34904a`. Non-interactive run: no fixes applied; the observations below are listed for a fixer agent.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

## Evidence summary

- **Plan adherence**: every "Changes Required" item is in the diff with the planned contract: `WorkTimeSettings.daily_norm_ms` (`Copy + Eq`, 60..=600 whole minutes, `null`/absent = 8 h, non-number refused, `deny_unknown_fields` kept); `DEFAULT_DAILY_NORM_MS`, `DayOff`/`DaysOff`, `day_norm_ms`, public `day_norms` (any range, for S-08), `figures` from day norms with `half_days_after_today`; v12 `ALTER TABLE days_off ADD COLUMN kind … CHECK (kind IN ('full','half'))` appended, v1–v11 untouched (`git diff 3f24630.. -- schema.rs` has no removed lines); `DayKind::HalfDayOff`, `MonthReport.daily_norm_ms`/`half_days_off`; `DayMark` (Deserialize, camelCase) as the `set_day_off` input; Segmented `disabled`; three-state controls in the table and in "Wybrany dzień"; `formatDaysAfterToday` shared by the view and the dashboard; `formatDaysOff` with `pl-PL` decimal comma; README/CHANGELOG/PRD wording. Nothing planned is missing.
- **Scope**: no PRD FR added, `roadmap.md` and `lib.rs` untouched, header unchanged, no settings editor. Only unplanned edit: removal of the now-unused `.wt-check` CSS (no remaining users in `src/`), harmless.
- **Rule correctness**: half norm `daily_norm_ms / 2` is exact (whole minutes → multiple of 30 s; test 7:01 → 12_630_000 ms). "Średnio / dzień" = `ceil_to_minute(ceil(remaining × daily / norm_after))` in `i128`; equals the old `ceil(remaining / after)` without half days (unit test + proptest bounds). Classification order weekend > holiday > full > half > workday; holiday shadowing keeps the row (test). `ON CONFLICT(day) DO UPDATE SET kind = excluded.kind` keeps `created_at_ms` (test); weekend/holiday refused for all three marks before the write (tests in `norm_tests.rs` and `tests/work_time.rs`). `halfDaysAfterToday` is `Some` only for the current month (TS union: `null` for past/future). Reminder start is silent on `HalfDayOff` (existing `!= Workday`, new test).
- **Store rules**: one `store.write`; values always as `?N` parameters; CHECK refusal test shows no row value in the error; v11→v12 upgrade test (`MIGRATIONS[..11]`) keeps old rows as whole days; half day survives reopen; new crash test `tests/half_day_off_crash.rs` uses `tests/common/crash.rs` and checks `quick_check`, no lost commit, contiguous rows and kind per row.
- **Contracts**: each Rust type change and its TS mirror land in the same commit (p1 `dailyNormMs`, p2 `halfDayOff`/`halfDaysOff`/`halfDaysAfterToday`, p3 `DayMark` + `setDayOff(day, mark)`); serde camelCase; `invoke` only in `src/worktime/api.ts`; no literal colours in `WorkTime.css`/`Segmented.css`.
- **Test quality**: the proptest oracle is computed in the test from the random flags and the calendar (`daily × unmarked + daily / 2 × halves`, `norm_to_date`, `after`, `half_after`, needed-per-day bounds), independent of `day_norm_ms`; with random norm 60–600 min it would catch a wrong half norm, a wrong count or a wrong per-day formula. Invented data only.
- **Strict/Lenient**: report and `set_day_off` Strict, status/entries/reminders Lenient; test covers both for a valid and an invalid `dailyNormHours`.
- **Docs**: README (range 1–10 h in whole minutes, `null`/missing = 8, one value without history, labels "nie"/"½"/"tak" and "roboczy"/"pół dnia wolnego"/"wolny", 0.5 in "Dni wolne", per-full-day "Średnio / dzień", reminder silent) and CHANGELOG match the code; PRD "8 h" only as "domyślnie 8 h".
- **Merge risks** (as in the plan): migration number v12 (S-02 owns it; rename + `BEFORE_KIND` on clash); S-08 must call `day_norms` / the new `figures` signature; S-05 gets a new `DayKind` variant and new `MonthReport`/`NormFigures` fields (exhaustive matches/literals break on purpose); `WorkTimeSettings` literals in other branches need `..WorkTimeSettings::default()`; `set_day_off` callers in other branches need `DayMark`; text conflicts in `WorkTimeView.tsx`, `DaysTable.tsx`, `WorkTime.css` (`--dn-table-cols`), `types.ts`, `api.ts`, README, CHANGELOG.

## Success criteria (run in this review)

| Command | Result |
|---------|--------|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, no warnings |
| `cargo test` | pass (lib 566 passed; all integration binaries green, incl. `half_day_off_crash`, `days_off_crash`, `work_time`) |
| `npm run lint` | pass |
| `npm run format:check` | pass |
| `npm run build` | pass |
| `git grep -nw DAILY_NORM_MS -- src-tauri`; `git grep -n "8 h" -- src/worktime` | nothing found |
| `git grep -n "8 h" -- context/foundation/prd.md` | only "domyślnie 8 h" wording |
| hex/rgba grep on `WorkTime.css`, `Segmented.css`; `invoke` outside `api.ts` | nothing found |
| `git diff d34904a..HEAD --stat -- context/foundation/roadmap.md src-tauri/src/lib.rs` | empty |

Automated Progress rows `[x]` are truthful. Manual rows 1.5, 1.6, 2.5, 3.5–3.9 are unchecked (pending user; agents must not run `npm run tauri dev`, and the real database must be copied aside first per the plan).

## Findings

### F1 — Busy Segmented drops keyboard focus

- **Severity**: OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src/components/Segmented/Segmented.tsx:35 (used at src/worktime/DaysTable.tsx:127 and src/worktime/WorkTimeView.tsx:174)
- **Detail**: While a request is pending every button gets the `disabled` attribute. A button activated with Enter/Space is focused when it becomes disabled; Chromium/WebView2 then moves focus to `body`, so a keyboard user loses the place in the 31-row table and must Tab from the top again. The old checkbox had the same `disabled={busy}` pattern, so this is not a regression, and the plan prescribed `disabled`. Manual row 3.8 ("przełącznik działa z klawiatury") is where it shows.
- **Fix**: Only if 3.8 shows the focus loss: in `Segmented`, replace `disabled={disabled}` with `aria-disabled={disabled || undefined}` and make the click handler a no-op when `disabled` (`onClick={() => { if (!disabled) onChange?.(o.value); }}`); restyle `.dn-seg-btn:disabled` as `.dn-seg-btn[aria-disabled="true"]` in `Segmented.css:32`.
  - Strength: keeps focus on the pressed button; no change to callers.
  - Tradeoff: buttons stay in the tab order while busy (they do nothing).
  - Confidence: MED — focus fixup on disabled elements is current Chromium behaviour, not verified in this WebView2.
  - Blind spot: not run in the app.
- **Decision**: DEFERRED to manual row 3.8

### F2 — SQL literal uses a `\n` escape instead of a line continuation

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/work_time/report.rs:276
- **Detail**: The upsert string is one ~140-character line with an embedded `\n` followed by spaces (`"… VALUES (?1, ?2, ?3) \n             ON CONFLICT(day) …"`). Valid SQL, but the replaced code (and the rest of the module) splits SQL with a trailing `\` continuation; rustfmt cannot wrap it.
- **Fix**: Write it as `"INSERT INTO days_off(day, created_at_ms, kind) VALUES (?1, ?2, ?3) \` + newline + `ON CONFLICT(day) DO UPDATE SET kind = excluded.kind"`.
- **Decision**: FIXED

### F3 — Copied ignore reason names the other crash test

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/tests/half_day_off_crash.rs:59
- **Detail**: `#[ignore = "writer process for crash_keeps_days_off_consistent"]` was copied from `days_off_crash.rs`; the parent test in this file is `crash_keeps_half_and_whole_days_off_consistent`.
- **Fix**: Change the reason to `"writer process for crash_keeps_half_and_whole_days_off_consistent"`.
- **Decision**: FIXED

### F4 — Phase 4 Progress rows have no commit sha

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/configurable-daily-norm/plan.md:524-526
- **Detail**: Rows 4.1–4.3 are `[x]` (truthful: the gates pass, the greps match, `roadmap.md` is untouched) but lack the ` — <sha>` suffix the Progress convention asks for; phases 1–3 have it. `change.md` was still `plan_reviewed` before this review (now `impl_reviewed`).
- **Fix**: Append ` — 9413214` to rows 4.1, 4.2 and 4.3.
- **Decision**: FIXED

### F5 — "Wybrany dzień" control may be wider than the side panel

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/WorkTimeView.tsx:18-22
- **Detail**: "roboczy" / "pół dnia wolnego" / "wolny" with `padding: 0 var(--space-3)` per button is about 270–280 px; `.dn-col-side` can shrink to 280 px minus panel padding, so at some widths the middle label wraps inside its button or the group overflows. Not verifiable without running the app; belongs to manual row 3.6.
- **Fix**: If it wraps in the app, shorten the middle label to "pół dnia" (the group label "Dzień" already gives the context) and update the README sentence that quotes the labels.
- **Decision**: DEFERRED to manual row 3.6
