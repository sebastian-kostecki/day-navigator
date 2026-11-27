---
date: 2026-11-26T14:09:05+01:00
researcher: Claude (10x-research, non-interactive)
git_commit: d34904a4dc9f9400c8b386cc935a845b7ed6762b
branch: slice/week-summary
repository: day-navigator
topic: "Week summary (S-08): the current Monday–Sunday week's worked time against the week's norm, day by day, in 'Czas pracy'"
tags: [research, work-time, norm, report, week-summary, worktime-ui]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (10x-research, non-interactive)
---

# Research: Week summary in "Czas pracy"

**Date**: 2026-11-26T14:09:05+01:00
**Researcher**: Claude (10x-research, non-interactive)
**Git Commit**: d34904a4dc9f9400c8b386cc935a845b7ed6762b (develop with M-3 wave 1: S-01, S-03, S-06)
**Branch**: slice/week-summary
**Repository**: day-navigator

## Research Question

Roadmap S-08 / issue #18. How does the app compute the month norm, deviation, day classification and worked time per day today, and where and how can a week summary hook in? The summary is a Mon–Sun week, always shown whole, also across a month boundary. It is a section in "Czas pracy", computed in Rust with the month's exact rule, and the UI only displays it. It must reuse the existing norm and classification functions so that S-02 (configurable daily norm, half day off) can change them in parallel.

The user decisions are fixed and recorded in `change.md` Notes. The skill asked no questions. This run was non-interactive and the slice brief answered them.

## Summary

- **Hook point (recommended): a new file `src-tauri/src/work_time/week.rs`.** It holds `pub fn week_report(store, day, now, tz, settings) -> Result<WeekReport, WorkTimeError>`, which does one `store.read`. It loads the week's entries with `in_day` (`mod.rs:131`) and its days off with `days_off_between` (`report.rs:67`) over `[monday, next_monday)`. Both take any half-open range. It then builds one `DayReport` per day with the month's own per-day code.
- **Commands and registration.** A new command `work_week_report(day)` goes in `commands.rs` next to `work_month_report` (`commands.rs:213-223`), with `Policy::Strict` like the month. It needs one line in `lib.rs`'s `generate_handler!` (`lib.rs:213-222`) and one line `pub mod week;` in `mod.rs:11-19`.
- **A week is never a slice of one `MonthReport`.** A week crossing a month needs two months, and `compute` reads exactly one month (`report.rs:130-138`). Two month reads would break the one-snapshot rule.
- **Reuse, not re-implementation.** The per-day row today is built inline in `report::compute` (`report.rs:144-168`):
  - worked time from `rules::portion` over `day_bounds`;
  - norm from `norm::day_norm_ms`;
  - deviation from `norm::day_deviation_ms`;
  - kind from the private `classify`;
  - holiday name and `day_off_owed`.

  The safest reuse against S-02 is to extract that loop body into one `pub(super)` helper in `report.rs` that both `compute` and `week.rs` call. If S-02 then changes the day norm (daily-norm setting, half day), the week follows it automatically. The minimum alternative is to make `classify` and `days_off_between` `pub(super)` and repeat the loop in `week.rs`. That duplicates the rule that S-01's plan warned "a third copy would drift".
- **Week totals come from the day rows, not from `norm::figures`.**
  - `figures` computes the norm as `DAILY_NORM_MS × workday count` (`norm.rs:120, 139`). That assumes a full 8 h per workday, which S-02's half day off breaks.
  - The week norm is `Σ day.norm_ms`. "Norm so far" is `Σ day.norm_ms` for days `≤ today`. Worked is `Σ day.worked_ms`. The deviation is worked minus norm so far.
  - This is the same invariant the month proptest already pins: day norms add up to the month norm, and day worked adds up to the month worked (`norm_tests.rs:609-615`).
  - New code never names `DAILY_NORM_MS` or 8 h.
- **UI: a new panel in `WorkTimeView.tsx`, which is the only edit to that shared file.**
  - New files: `WeekSummary.tsx` (the panel body), `useWeekReport.ts` (copies `useMonthReport`'s loading, error, `lastOkAt`, `work-time-changed`, focus and 60 s poll pattern), a `workWeekReport` function in `api.ts`, and a `WeekReport` mirror in `types.ts`.
  - Reused components: `Panel`, `DataTable`, `HoursBar`, `ErrorState`, `StatStrip`, and the formatters `formatHm`, `formatSigned`, `formatDayLabel`.
  - For a running timer, today's row is extrapolated in the UI exactly as the days table already does (`DaysTable.tsx:14-26`). This is the one accepted "live" adjustment, not a re-sum of the norm.
- **Main risks:**
  1. S-02 edits `norm.rs`, `report.rs`, the "Jak liczę" text in `WorkTimeView.tsx` and `types.ts`. A shared day-row helper minimises semantic drift but creates a textual conflict in `report.rs`. Keeping `week.rs` separate keeps the rest conflict-free.
  2. Tests that build `WorkTimeSettings { holidays: … }` literally will break when S-02 adds a field. New tests should use `manual_only()` or `WorkTimeSettings::default()`.
  3. The month switch moves `day`. The plan must say whether the week follows the selected day or always the real current week.

## Detailed Findings

### Month norm and deviation (pure, `norm.rs`)

- `DAILY_NORM_MS = 8 * 3_600_000` (`norm.rs:14`). It is the only place the 8 h lives in Rust. It is used by `day_norm_ms` (`norm.rs:57`) and by `figures` (`norm.rs:120, 139`).
- `workdays(first, next_first, days_off, holidays)` (`norm.rs:33-44`): Mon–Fri days in `[first, next_first)` that are neither in `days_off` nor a holiday. It works for any half-open range, not only a month.
- `day_norm_ms(day, days_off, holidays)` (`norm.rs:49-59`): 8 h on a workday, 0 on a weekend, a day off or a holiday. A listed weekend is still 0.
- `day_deviation_ms(day, today, norm_ms, worked_ms)` (`norm.rs:63-74`): `None` after today, `None` when both norm and worked are 0, otherwise worked minus norm. Today counts in full: on a workday, today's 8 h is in the norm from the morning (also `WorkTimeView.tsx:70-73`).
- `MonthPhase` / `phase(first, today)` (`norm.rs:77-92`) compares year and month only. A week needs its own phase comparison, by the Monday of `day` vs the Monday of `today`. The enum shape (`past` / `current` / `future`) can be reused or mirrored.
- `figures(workdays, today, phase, worked_ms)` (`norm.rs:113-152`) fills `NormFigures`:
  - `norm_to_date_ms`, `deviation_ms`;
  - for the current phase, `workdays_after_today`, `remaining_ms` and `needed_per_day_ms`, rounded up to a minute.

  It multiplies `DAILY_NORM_MS` by counts. Inference: it will change under S-02's half day off.

### Day classification (`report.rs`)

- `DayKind { Workday, DayOff, Holiday, Weekend }` (`report.rs:23-30`), serialised camelCase.
- The private `classify(day, marked_off, holidays)` (`report.rs:101-111`) is the one order: weekend before holiday before day off before workday. A Saturday or Sunday holiday stays `Weekend`, and a weekday holiday wins over a marked day off.
- `day_kind(conn, day, settings)` (`report.rs:113-119`) is `pub(super)` and reads one day. It is used by `entries_for_day` (`mod.rs:254`) and `is_workday` (`report.rs:242-248`).
- `days_off_between(conn, first, next_first)` (`report.rs:67-89`) is private and returns a `BTreeSet<NaiveDate>` for any range.
- `HolidayCalendar::holiday(day)` (`holidays.rs:17-26`) is computed per date and per year. A week across the year end (28.12.2026–3.01.2027) needs nothing special.
- The settings are `WorkTimeSettings { holidays }` (`settings/work_time.rs:14-24`), with `Pl` as the default. S-01's plan says S-02 adds its daily-norm field to this struct, and that store-backed functions take `&WorkTimeSettings` so their signatures survive S-02. So `week_report(…, settings: &WorkTimeSettings)` is S-02-proof at the store-backed level.

### Worked time per day (running timer, midnight, DST)

- `in_day(conn, start, end, now)` (`mod.rs:131-153`) returns counted entries overlapping `[start, end)`. A running entry is skipped only when `now <= start`. It is private to `work_time` and visible to a child module such as `week.rs`.
- `rules::portion(start, end, now, day_start, day_end)` (`rules.rs:59-63`) clips an entry to a day. A running entry counts up to `now`.
- `rules::day_bounds(day, tz)` (`rules.rs:43-55`) runs from local midnight to the next local midnight, so DST days are 23 h or 25 h long.
- `compute` loops over each day (`report.rs:145-168`). It sums the portions per day and counts the entries with a non-zero portion, so an entry across midnight is split between the days. Existing tests pin this behaviour:
  - `an_entry_across_midnight_counts_in_both_days` (`norm_tests.rs:483`);
  - `an_entry_across_the_month_boundary_counts_only_its_part_per_month` (`norm_tests.rs:496`);
  - `dst_days_sum_their_real_lengths` (`norm_tests.rs:509`);
  - `a_running_entry_counts_up_to_now_today_and_its_part_yesterday` (`norm_tests.rs:520`).

  A week function over `[monday, next_monday)` inherits all of it unchanged.
- `today.rs` (`today.rs:23-40`, S-03) is the precedent for a small new file in `work_time` with its own single `store.read`, and so for `week.rs`.

### Commands and settings policy

- `work_month_report(day)` (`commands.rs:213-223`) calls `report::parse_day` and then `run_with_settings(…, Policy::Strict, …)`.
  - Strict means an invalid `workTime` block gives `SettingsInvalid`, which the view shows through `ErrorState` (`commands.rs:25-53`). The week report should match it, so the month and the week never disagree on the calendar.
- `notify` emits `work-time-changed` after every write: start/stop, add/update/remove, `set_day_off` (`commands.rs:108-114`). A read-only week command emits nothing.
- `lib.rs:213-222` lists the work-time handlers. One new line is needed.

### Frontend (`src/worktime/`, `src/components/`)

**`WorkTimeView.tsx`**
- One date state `day` (`:89`). The month is `day.slice(0, 7)` (`:111`). The month chevrons call `setDay(shiftMonth(day, ±1))` (`:182, :193`).
- Hooks: `useWorkEntries(day)`, `useMonthReport(day)`, `useDayOffToggle(report)` and `useWorkTimer()` (`:90-97`). `useWorkTimer` gives the one shared timer and `nowMs`. AGENTS.md says never to subscribe to it a second time.
- Layout (`:171-248`):
  - `MonthReport` StatStrip;
  - `.dn-columns` with a main column (Panel "Wybrany dzień", Panel "Dni miesiąca", `HowCounted`) and a side column (`EntriesPanel`, `RemindersPanel`).
- `HowCounted` (`:60-86`) explains the norm in text that names "8 h" three times. S-02 will edit it. The week panel should not add another "8 h" text.

**`api.ts` and `types.ts`**
- `api.ts:51-52`: `workMonthReport(day)` invokes `work_month_report`.
- `types.ts`:
  - `DayKind` (`:79`), `MonthPhase` (`:81`);
  - `DayReport` (`:83-95`): `day`, `kind`, `workedMs`, `normMs`, `deviationMs | null`, `entryCount`, `holiday`, `dayOffOwed`. A `WeekReport` can reuse `DayReport[]` as is;
  - `MonthReport`, a phase-discriminated union (`:97-142`);
  - `WorkTimeError` union (`:8-23`), with `SettingsInvalid` handled by `workTimeErrorMessage` (`api.ts:120`).

**`useMonthReport.ts`**
- States: loading / error `{message, lastOkAt}` / loaded.
- Requests are latest-wins.
- It subscribes to `work-time-changed` before the first load and refetches on focus or visibility. It polls every 60 s except for a loaded past month (`:16, :51-112`).
- `useWeekReport` should copy the pattern, keyed by the Monday instead of the month.

**`DaysTable.tsx`**
- `liveFigures` (`:14-26`) swaps today's `workedMs` for the live `todayTotalMs` (`total.ts:13-18`) and shifts the deviation by the same delta, only while the phase is `current` (`:156`).
- Rows use `DataTableRow current / hatched`, `HoursBar` with `norm` from `normMs` (the UI never knows 8 h), and the `formatSigned` tone: info above 0, warn below 0.
- `.wt-days` sets the column template (`WorkTime.css:20-22`).
- The week panel can reuse the row anatomy, or extract `DayRow` and `liveFigures` for sharing. Extraction touches a file S-02 will also edit (half-day switch), so copying is the lower-conflict option.

**Components and styles**
- `Panel {title, meta, footer, tone}`.
- `DataTable / DataTableRow / DataTableCell` (tones muted / faint / info / warn).
- `StatStrip {items: {label, value, sub?, tone?}[]}`, keyed by label.
- `HoursBar {hours, norm, max?, today?, label}`.
- `ErrorState {title, message, onRetry, lastSuccess}`.
- `Notice`.
- Tokens: `--info` / `--warn` are the deviation colours, plus the spacing scale (`tokens.css:72-75, 100-109`). No new token is needed.

### Month-boundary and calendar cases (invented-data test candidates)

These are verified against the existing test anchors. 2026-11-23 is a Monday (`norm_tests.rs:36-38`), 24.12 counts as a holiday from 2025, and 6.04.2026 is Easter Monday (S-01).

| Week (Mon–Sun) | Feature | Expected week norm |
| --- | --- | --- |
| 23–29.11.2026 | inside one month; today Thu 26.11 | 40 h, 32 h so far on Thu |
| 30.11–6.12.2026 | crosses Nov→Dec | 40 h |
| 21–27.12.2026 | 24 and 25.12 weekday holidays; 26.12 Saturday holiday (`dayOffOwed`) | 24 h |
| 28.12.2026–3.01.2027 | crosses the year; 1.01.2027 is a Friday holiday | 32 h |
| 6–12.04.2026 | Easter Monday | 32 h |
| 23–29.03.2026 / 19–25.10.2026 | DST Sunday (23 h / 25 h day) | 40 h; day sums use real lengths |

Add one week with a manual day off under `manual_only()`. Add a proptest like `norm_tests.rs:566-616`: the day rows add up to the week totals, and the week's days equal the corresponding days of the month reports.

## Code References

- `src-tauri/src/work_time/norm.rs:14` - `DAILY_NORM_MS`; never use it in new code
- `src-tauri/src/work_time/norm.rs:33-74` - `workdays`, `day_norm_ms`, `day_deviation_ms` (range-agnostic)
- `src-tauri/src/work_time/norm.rs:113-152` - `figures` (count × 8 h; not half-day safe)
- `src-tauri/src/work_time/report.rs:67-89` - `days_off_between` (private, any range)
- `src-tauri/src/work_time/report.rs:101-119` - `classify` (private), `day_kind` (`pub(super)`)
- `src-tauri/src/work_time/report.rs:122-183` - `compute`; per-day loop at `:144-168`
- `src-tauri/src/work_time/report.rs:186-194` - `month_report` (store-backed wrapper)
- `src-tauri/src/work_time/mod.rs:11-19` - module list (add `pub mod week;`)
- `src-tauri/src/work_time/mod.rs:131-153` - `in_day`
- `src-tauri/src/work_time/rules.rs:43-63` - `day_bounds`, `portion`
- `src-tauri/src/work_time/today.rs:1-40` - precedent for a small single-read file
- `src-tauri/src/work_time/commands.rs:213-223` - `work_month_report` (Strict)
- `src-tauri/src/lib.rs:213-222` - work-time handlers in `generate_handler!`
- `src-tauri/src/work_time/norm_tests.rs:22-27, 566-616` - `manual_only()`, month proptest
- `src-tauri/tests/common/mod.rs:19-21` - integration `manual_only()` (struct literal)
- `src/worktime/WorkTimeView.tsx:60-86, 89-111, 171-248` - `HowCounted`, state, layout
- `src/worktime/DaysTable.tsx:14-26, 150-163` - `liveFigures`, rows
- `src/worktime/useMonthReport.ts:6-119` - hook pattern to copy
- `src/worktime/types.ts:79-142` - `DayKind`, `MonthPhase`, `DayReport`, `MonthReport`
- `src/worktime/api.ts:51-52, 120` - `workMonthReport`, `workTimeErrorMessage`
- `src/worktime/total.ts:8-18` - `liveSpanMs`, `todayTotalMs`

## Architecture Insights

- **The norm rule lives at two levels.**
  - The pure level (`norm.rs`, `holidays.rs`) takes dates and a `HolidayCalendar`.
  - The store-backed level (`report.rs`, `mod.rs`) takes `&WorkTimeSettings`.

  S-02 is expected to change the pure signatures (daily norm, half day) but not the store-backed ones (S-01 plan decision D2). A week function at the store-backed level, whose norm comes only from day rows, is the S-02-proof seam.
- **One `store.read` per report.** Entries and days off come from one snapshot. This is why the week needs its own read over `[monday, next_monday)` and must not be assembled from two month reports.
- **New behaviour goes in new files.** S-03's `today.rs` and its "A12" decision set the precedent: shared files get only registration lines.
- **The UI live extrapolation is accepted.** The header, `MonthReport`'s "Dziś" tile and `DaysTable` already shift today's figures by the running span between refetches. The week panel can do the same for today's row and the week total, and recomputes no norm.

## Historical Context (from prior changes)

- `context/archive/2026-11-26-public-holidays-norm/plan.md`:
  - `:103`: the single `classify` order. A third copy would drift.
  - `:264`: the pure norm functions take `HolidayCalendar`.
  - `:446-453`: the Strict / Lenient policy; the report is Strict.
  - `:684` (D2): store-backed functions take `&WorkTimeSettings` so that S-02 does not change them again.
  - `:205, :217`: S-02 adds its field to `WorkTimeSettings` / `RawWorkTime`.
  - All supported in the current code.
- `context/archive/2026-11-26-public-holidays-norm/reviews/impl-review.md:25`: December 2026 = 168 h in the app vs 160 h statutory, by design (a Saturday holiday's exchange day stays manual). The same applies to the week of 21–27.12.2026 (24 h).
- `context/archive/2026-11-24-worktime-days-table/plan.md:35, 47, 132`: day deviations up to today add up to the month deviation, and day worked to month worked. This is pinned by the proptest and is supported. `:54`: "no new command" for the days table. That held for a same-month addition. It does not fit a week across months, which needs one new command.
- `context/archive/2026-11-24-worktime-days-table/plan.md:71`: the live today extrapolation `day.deviationMs + (todayTotal − day.workedMs)`. Supported by `DaysTable.tsx:14-26`.
- `context/archive/2026-11-24-worktime-days-table/plan.md:265`: a grep gate against 8 h in `src/` (`28800000`, `norm = 8`…). Reuse it for the week UI.
- `context/archive/2026-11-26-tray-timer-control/plan.md:80-87, 406`: `today.rs` was a new file, to keep edits to shared files small.
- `context/foundation/archive/shape-notes-2026-11-22-2031.md:139-140` and `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md:367`: the week view (old FR-008) was parked as nice-to-have. The current PRD lists "widok tygodnia" as considered but not excluded (`prd.md:234`), and no FR covers it. S-08 anchors it as MS-18 without a new FR, per the user decision. The plan should say that it reuses the FR-014 rule (`prd.md:155`).
- `context/foundation/roadmap.md:79`: the Baseline says the 8 h norm is at `norm.rs:12`. It is now at `norm.rs:14`. Stale line number only; the roadmap is not edited here.

## Related Research

- `context/archive/2026-11-26-public-holidays-norm/research.md`
- `context/archive/2026-11-26-tray-timer-control/research.md`
- `context/archive/2026-11-26-work-entry-note/research.md`

## Shared-file hotspots with the parallel slices

| File | S-08 touch | Also touched by (inference from roadmap outcomes) |
| --- | --- | --- |
| `work_time/norm.rs` | none, if week totals come from day rows | S-02 (daily norm, half day) |
| `work_time/report.rs` | `pub(super)` on `classify` / `days_off_between`, or the extracted day-row helper | S-02 (day norm), S-05 (report text) |
| `work_time/mod.rs` | `pub mod week;` (+ `#[cfg(test)] mod week_tests;`) | possibly S-07 |
| `work_time/commands.rs` | one new command | S-05 (copy command?), S-07 |
| `lib.rs` | one handler line | S-04 (autostart plugin), S-05, S-07 |
| `src/worktime/types.ts`, `api.ts` | `WeekReport`, `workWeekReport` appended | S-02 (half day), S-05 |
| `src/worktime/WorkTimeView.tsx` | one `<Panel>` + hook call | S-02 (`HowCounted` text, half-day control), S-05 (copy button) |
| `README.md`, `CHANGELOG.md` | one bullet each | every slice |

At the research commit, the parallel branches `slice/configurable-daily-norm`, `slice/month-report-copy`, `slice/end-of-day-summary` and `slice/windows-autostart` have no commits beyond `develop` (`git log develop..<branch>` empty). No actual conflicts can be checked yet.

## Open Questions

These are for `/10x-plan`. The agent defaults are suggestions; the user decides.

1. **Which week.** The options:
   - always the real current week;
   - the week of the selected `day` (follows the day picker and month chevrons; moving the month moves `day` to the 1st, so the week would jump);
   - the current week with its own prev/next chevrons.

   Suggested default: the current week only, matching the roadmap outcome "current week". The command takes a `day` anyway, so navigation can be added later without a contract change.
2. **Columns.** Suggested default per day: day label, `HoursBar` against `normMs`, worked `H:MM`, deviation `±H:MM`; plus a week row or StatStrip with worked, norm, and deviation so far. Entries count and the day-off switch stay in "Dni miesiąca".
3. **"So far" vs whole week.** Suggested default: mirror the month's current phase. The deviation is against the norm up to and including today (`normToDateMs`), and the full week norm is shown as "z X normy". Whether to add "remaining this week" (like `remainingMs`) is a plan choice. `needed_per_day_ms` would require `figures`-style rounding and is not needed.
4. **Reuse shape.** Choose between extracting the per-day row builder from `report::compute` into a shared `pub(super)` helper (no rule duplication; a small conflict with S-02 in `report.rs`) and widening `classify` / `days_off_between` to `pub(super)` and building rows in `week.rs` (smaller diff; duplicated assembly). Recommended: the extraction, because the user's rule "exactly the same rule as the month" is better guaranteed by one code path.
5. **Panel placement.** Suggested: above "Dni miesiąca" in the main column, or in the side column above "Przypomnienia". This is a UI choice for the plan.
