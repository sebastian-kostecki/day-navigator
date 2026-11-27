---
date: 2026-11-26T14:14:41+01:00
researcher: Claude (agent, non-interactive run)
git_commit: d34904a4dc9f9400c8b386cc935a845b7ed6762b
branch: slice/configurable-daily-norm
repository: day-navigator
topic: "Configurable daily norm and half day off (S-02, MS-26, issue #26): every place the norm rule is used"
tags: [research, work-time, norm, days-off, settings, store, migration]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (agent, non-interactive run)
---

# Research: Configurable daily norm and half day off

**Date**: 2026-11-26T14:14:41+01:00
**Researcher**: Claude (agent, non-interactive run)
**Git Commit**: d34904a4dc9f9400c8b386cc935a845b7ed6762b
**Branch**: slice/configurable-daily-norm
**Repository**: day-navigator

## Research Question

The change was opened without intent text, and this run could not reach the user. The question comes from roadmap slice S-02 (`context/foundation/roadmap.md:106-118`), GitHub issue #26 and the caller's brief.

Map every place that uses the work-time norm rule or the fixed 8 h constant. The planned change makes:

- the daily norm configurable in `settings.json` (default 8 h);
- a half day off (half the norm) a third state of the day toggle in the days table;
- the report count both: month norm, deviation, average per remaining day, header deviation, reminders and dashboard tiles.

### Fixed user decisions (given to this run, not re-asked)

1. One daily-norm value in settings, with no norm history. A change applies to every month, past ones included. Plan and README must record this consequence.
2. The half day off is needed. It is a third state of the day toggle in the days table (workday → half day off → day off, or an equally clear control), and it halves the norm on that day.
3. Extend the existing `WorkTimeSettings` (`src-tauri/src/settings/work_time.rs`) and its `workTime` block.
4. Anchor the change as MS-26. Add no new FR. Update only the PRD's existing "8 h" wording.
5. The agent may choose defaults, recorded as "default chosen by agent, user may revise":
   - the key name, unit, range and step;
   - a half day blocked on weekends and holidays, like the day-off switch;
   - storage in the app data store through a new migration after v11.

## Summary

**Norm rule in Rust.** The rule lives in one pure module, `src-tauri/src/work_time/norm.rs`. The constant `DAILY_NORM_MS = 8 * 3_600_000` (`norm.rs:14`) is read in exactly three places:

- `day_norm_ms` (`norm.rs:57`);
- `figures`, for the month norm (`norm.rs:120`);
- `figures`, for the norm to date (`norm.rs:139`).

`figures` multiplies a count of workdays by the constant (`norm.rs:119-120`, `:137-139`). A half day therefore cannot be expressed today: the month norm has to become a sum of per-day norms.

**Report.** The only consumer of `norm.rs` is `report::compute` (`report.rs:122-183`). It feeds:

- the `work_month_report` command (`commands.rs:214-223`), which drives the "Czas pracy" tiles, the days table and the dashboard tiles;
- `status` (`mod.rs:180-215`), which carries `month_deviation_ms`, the old header deviation.

A grep of `src/` found no consumer of `monthDeviationMs` besides its type at `types.ts:52`. The header (`WorkTimerHeader.tsx`) shows only today's total. Outside `src-tauri/src/work_time/`, no Rust file references `norm::`, `DAILY_NORM`, `day_norm` or `deviation`.

**Day classification.** Days off are a flag table `days_off(day TEXT PRIMARY KEY, created_at_ms)`, created in migration v5 (`store/schema.rs:90-104`). The table has no "kind" column.

- The toggle is `set_day_off(day, off: bool)` (`report.rs:211-238`, command `commands.rs:225-237`).
- It refuses a weekend (`NotAWorkday`) and a weekday holiday (`Holiday`) before writing (`report.rs:218-223`).
- Classification is one function, `classify` (`report.rs:101-111`), with precedence weekend > holiday > day off > workday.
- The last migration is **v11** (`store/schema.rs:247-251`, the work-entry note).

**Settings.** `WorkTimeSettings { holidays }` is `Copy + Eq` (`settings/work_time.rs:13-16`). It is parsed from a `deny_unknown_fields` `RawWorkTime` (`:31-36`). The code anticipates S-02: the doc comment says "S-02 adds its daily-norm field here" (`:12`).

Settings reach the norm through one per-call read, `work_time_settings(path, Policy)` (`commands.rs:36-53`):

- Strict for the report and `set_day_off` (`:219`, `:231`);
- Lenient (falls back to defaults) for status, the entry list (`:118`, `:130`) and reminders (`work_reminders.rs:63`).

**TypeScript side.** The TS side computes no norm figure. It contains:

- one tagged `DayKind` union (`src/worktime/types.ts:79`);
- two boolean day-off paths: the table switch in `useDayOffToggle.ts:21-53` / `DaysTable.tsx:106-119`, and the "Wybrany dzień" checkbox in `useWorkEntries.ts:124-162` / `WorkTimeView.tsx:150-169`;
- three hard-coded "8 h" help strings (`WorkTimeView.tsx:65`, `:71`, `:75`);
- a `SettingsInvalid` message that lists only `holidays` (`api.ts:149`).

**Docs and tests.** "8 h" wording sits in `README.md:55`, `:123` (the `workTime` row) and in PRD `prd.md:68`, `:117`, `:155` (FR-014), `:176` (FR-020), `:205` and `:211`. MS-26 is not in the PRD; it is anchored only in the roadmap (`roadmap.md:36`).

Tests hard-code the 8 h norm in:

- `norm_tests.rs`: many assertions, including the proptest invariant "day rows sum to the month figures" at `:566-617`;
- `tests/work_time.rs:286`: `-8 * 3_600_000 * 7`.

Six `WorkTimeSettings { .. }` struct literals break when a field is added:

- `settings/work_time.rs:20`, `:108` and `:114`;
- `norm_tests.rs:23`;
- `tests/common/mod.rs:20`;
- `tests/work_time.rs:302`.

## Detailed Findings

### 1. Norm rule (`src-tauri/src/work_time/norm.rs`)

- `DAILY_NORM_MS` const, doc "a fixed 8 h per workday" (`norm.rs:13-14`). Public; imported by `norm_tests.rs:10`.
- `workdays(first, next_first, days_off, holidays) -> Vec<NaiveDate>`: Mon–Fri minus `days_off` minus holidays (`norm.rs:33-44`). Its only production caller is `report.rs:137`.
- `day_norm_ms(day, days_off, holidays)`: 0 on a weekend, a day off or a holiday; otherwise `DAILY_NORM_MS` (`norm.rs:49-59`). Its only production caller is `report.rs:156`.
- `day_deviation_ms(day, today, norm_ms, worked_ms)` already takes the norm as an argument, so it is norm-agnostic (`norm.rs:63-74`).
- `figures(workdays, today, phase, worked_ms)` (`norm.rs:113-152`):
  - `norm_ms = DAILY_NORM_MS * count` (`:120`);
  - `norm_to_date = DAILY_NORM_MS * to_date_count` (`:139`);
  - `remaining = max(norm - worked, 0)` (`:140`);
  - `needed_per_day = ceil_minute(ceil(remaining / workdays_after_today))` (`:145-148`);
  - `workdays` and `workdays_after_today` are plain day counts (`:105-106`, `:138`).
  - With half days, the norm sums must come from per-day norms. How `needed_per_day_ms` divides the remainder over a mix of full and half days needs a rule; see Open Questions Q1.
- `NormFigures` is flattened into `MonthReport` (`report.rs:40-41`), so its fields are TS-visible: `MonthReport` in `types.ts:97-142`.
- The archived holiday plan required that pure norm fns take `HolidayCalendar` and store-backed fns take `&WorkTimeSettings`. With that split, S-02 adds its field without new store-fn signatures (`context/archive/2026-11-26-public-holidays-norm/plan.md:84`, `:684` D2). The pure fns (`workdays`, `day_norm_ms`, `figures`) still need a norm parameter (or the settings) added.

### 2. Report and day-off storage (`src-tauri/src/work_time/report.rs`)

- `DayKind { Workday, DayOff, Holiday, Weekend }`, serialized camelCase (`report.rs:23-30`). This is the TS `DayKind` union at `types.ts:79`. A half day needs a new variant (e.g. `HalfDayOff` → `"halfDayOff"`), mirrored in TS in the same commit (AGENTS.md "Rust ↔ TypeScript Contracts").
- `DayReport { day, kind, worked_ms, norm_ms, deviation_ms, entry_count, holiday, day_off_owed }` (`report.rs:49-65`). S-05 (month-report copy) builds on these Rust report types (`roadmap.md:159`). The half-day state reaches S-05 through `kind` with no extra field.
- `MonthReport.days_off: Vec<String>` lists marked days, minus holiday-shadowed rows (`report.rs:42-44`, `:176-180`). The "Dni wolne" tile counts its length (`MonthReport.tsx:14-19`). Whether half days go in this list, a separate list, or count as 0.5 is Open Question Q2.
- Storage reads:
  - `days_off_between` reads the month's rows (`report.rs:67-89`);
  - `is_day_off` reads one day (`report.rs:91-96`);
  - `classify` (`report.rs:101-111`) and `day_kind` (`:113-119`) are the one classification. The archived holiday plan and its review chose this precedence (`context/archive/2026-11-26-public-holidays-norm/plan.md:103`, `:277-281`; `reviews/impl-review.md:27`).
- `compute` (`report.rs:122-183`):
  - reads days off and entries in one `store.read` snapshot (doc `:3-5`);
  - builds the per-day rows with `norm::day_norm_ms` (`:156`);
  - sums the month with `norm::figures(&workdays, ...)` (`:175`).
- `set_day_off(store, day, off: bool, now, settings)` (`report.rs:211-238`):
  - weekend → `NotAWorkday` (`:218-220`);
  - holiday → `Holiday` (`:221-223`);
  - otherwise `INSERT ... ON CONFLICT(day) DO NOTHING` (`:228-232`) or `DELETE` (`:234`), in one `store.write`.
- `is_workday` (`report.rs:240-248`): a grep of `src-tauri/src` (test files excluded) found no production caller.
- Migration v5 creates `days_off(day TEXT PRIMARY KEY CHECK (GLOB date), created_at_ms INTEGER NOT NULL) WITHOUT ROWID` (`store/schema.rs:90-104`). The last shipped migration is v11 (`schema.rs:247-251`). `validate` requires versions 1..N rising by 1 (`schema.rs:255-266`). Integration tests assert `user_version == MIGRATIONS.len()` (`tests/work_time.rs:383-385`), so they adjust to a v12 by themselves.
- Storage option for the half day. Recommendation (agent default): v12 `ALTER TABLE days_off ADD COLUMN kind TEXT NOT NULL DEFAULT 'full' CHECK (kind IN ('full','half'))`.
  - The primary key on `day` then makes "full" and "half" mutually exclusive by construction.
  - Existing rows stay full days.
  - Un-marking still deletes the row.
  - A separate `half_days_off` table would need a cross-table exclusivity rule.
- Old rows on a holiday are shadowed, not deleted (`report.rs:42-43`, `:178`; archived plan-brief `:41`). The same shadowing would apply to a half-day row.

### 3. Commands and settings read (`src-tauri/src/work_time/commands.rs`, `lib.rs`)

- `work_month_report(day)` uses Strict (`commands.rs:213-223`). `set_day_off(day, off: bool)` uses Strict and notifies `work-time-changed` (`:225-237`). Both are registered at `lib.rs:220-221`.
- Policy (`commands.rs:23-53`): Strict → `WorkTimeError::SettingsInvalid`; Lenient → `WorkTimeSettings::default()`.
- Consequence of Lenient with a configured norm: when the `workTime` block is broken, these paths silently use 8 h:
  - `status` → `month_deviation_ms` (`commands.rs:118`, `mod.rs:201`, `:211`);
  - `entries_for_day` → `day_kind` (`commands.rs:130`, `mod.rs:254`);
  - reminders (`work_reminders.rs:63`, `:68`).
  The report itself refuses. No current UI shows `monthDeviationMs` (grep of `src/`: only `types.ts:52`), so the visible effect is nil today. S-07 (end-of-day summary, "the month's deviation", `roadmap.md:32`) may surface it.
- Settings file → `WorkTimeSettings` (`settings/work_time.rs`):
  - the struct is `Copy, PartialEq, Eq` (`:13-16`), with default `holidays: Pl` (`:18-24`);
  - `RawWorkTime` uses `deny_unknown_fields`, so a new key must be added there or the block becomes invalid (`:31-36`);
  - the parser (`:64-88`) treats an absent or `null` field as the default and anything else unexpected as `WorkTimeSettingsInvalid`;
  - tests cover defaults, refusal of other values, unknown keys, file-level problems, BOM and block isolation (`:119-243`).
- Keeping `Eq` rules out an `f64` field. The norm is best held as integer ms (or minutes) in the struct and converted from the JSON number at parse time.
- Precedent for an integer-hours key with a range: `reminders.longRunningHours`, an integer 1..=24 or `false` (`src-tauri/src/settings.rs:255-257`, `:307`, `:370-379`).
- No settings UI exists. `settings.json` is edited by hand and opened through `open_settings_file` (`src/gitlab/api.ts:71-88`, used by `RemindersPanel.tsx:88-96`). `RemindersPanel` shows configured values with a "domyślnie" mark (`:64-79`), a pattern a norm display could copy.

### 4. Reminders (`src-tauri/src/work_time/reminders.rs`, `src-tauri/src/work_reminders.rs`)

- The "not running after HH:MM" reminder fires only when `facts.day_kind == DayKind::Workday` (`reminders.rs:144`). `day_kind` comes from `report::day_kind` (`reminders.rs:70`).
- A new `HalfDayOff` variant would, with the current `!= Workday` test, suppress the reminder on a half day. Whether that is wanted is Open Question Q3.
- Tests assert the reminder is off for `DayOff` and `Holiday` (`reminders_tests.rs:186-187`).
- The reminders read no norm hours. The long-running threshold is its own setting.

### 5. Frontend

- `src/worktime/types.ts`:
  - `DayKind` (`:79`);
  - `DayReport` (`:83-95`): `normMs`, `deviationMs`, `holiday`, `dayOffOwed`;
  - `MonthReportBase` / `MonthReport` (`:97-142`): `normMs`, `workdays`, `daysOff`, `workdaysAfterToday`, `neededPerDayMs`, `remainingMs`, `deviationMs`;
  - `WorkTimerStatus.monthDeviationMs` (`:52`);
  - there is no TS mirror of `workTime` settings.
- `src/worktime/api.ts`: `setDayOff(day, off: boolean)` → `invoke("set_day_off", {day, off})` (`:56-58`). The `SettingsInvalid` message lists only `"version": 1` and `holidays` (`:148-149`).
- `src/worktime/useDayOffToggle.ts`: boolean pending map (`:21`); stored state is `d.kind === "dayOff"` (`:26`); `toggle(day, off: boolean)` (`:37-53`).
- `src/worktime/DaysTable.tsx`:
  - the switch is a `checkbox role="switch"`, shown for `workday` and `dayOff` only (`:106-119`), under column header "Wolne" (`:146-148`);
  - rows are hatched for `dayOff` and `holiday` (`:58`);
  - `HoursBar norm={row.normMs / HOUR_MS}` (`:75-80`);
  - the live-today deviation is `row.deviationMs + (liveTodayMs - row.workedMs)`, which is norm-independent (`:14-26`).
- Grid columns: `--dn-table-cols: 84px 76px minmax(0,1fr) 52px 60px 44px` (`WorkTime.css:20-22`). The 44 px last column holds the switch. A three-state control may need more width.
- The second day-off path is the "Wybrany dzień" checkbox:
  - `useWorkEntries.ts:29`, `:124-132`, `:138` (`dayKind === "dayOff"`), `:148-162`;
  - `WorkTimeView.tsx:150-169`, shown for `workday` / `dayOff` (`:152`).
  - It must either learn the third state or stay a full-day toggle (Open Question Q4).
- `WorkTimeView.tsx` `HowCounted` hard-codes "8 h" in three places (`:65`, `:71`, `:75`).
- `MonthReport.tsx` (work-time tiles):
  - "Miesiąc … z {normMs} normy" (`:9-13`);
  - "Dni wolne" = `daysOff.length` with "oznaczone ręcznie" (`:14-19`);
  - "Odchylenie" (`:22-27`);
  - "Średnio / dzień" (`:30-48`).
- Dashboard tiles: `src/todo/summary.ts:23-71` ("Miesiąc", "Odchylenie", "Średnio / dzień"), fed by `useMonthReport` in `src/todo/TodoView.tsx:149-152`. That is the same `work_month_report` command, so the dashboard follows the Rust result with no TS change for the norm itself.
- `useMonthReport.ts` refetches on `work-time-changed`, on window focus or visibility, and every 60 s except for a past month (`:51-112`). A hand-edited `settings.json` therefore shows up on the next focus. This matches how the holiday switch works today.
- `components/HoursBar/HoursBar.tsx:17`: default `max = 10` h. A daily norm ≥ 10 h would put the norm tick at or past the bar's end, because `DaysTable` passes no `max`. This bounds the settings range (see the default below).
- No hard-coded `480`, `28_800_000` or `"8h"` in `src/`. A grep found only the three "8 h" strings above and a doc comment at `HoursBar.tsx:12`.

### 6. Docs mentioning the fixed 8 h

- `README.md`:
  - `:55` "against an 8-hour workday norm";
  - `:111` the JSON example `"workTime": { "holidays": "pl" }`;
  - `:123` the `workTime` table row (only `holidays`);
  - `:125` where an invalid `workTime` block is reported.
- `CHANGELOG.md`: the holidays entry sits under `[Unreleased]` (`:16-19`), so the S-02 entry goes there too. The historical lines at `:77-79` and `:132` are not edited.
- `context/foundation/prd.md`:
  - `:68` and `:117` "odchylenie miesiąca od (normy) 8 h na dzień";
  - `:155` FR-014 "normy 8 h na dzień roboczy";
  - `:176` FR-020 "pasek na tle 8 h";
  - `:205` "do normy 8 h na dzień roboczy";
  - `:211` "Norma = 8 h × dni robocze".
  - MS-26 has no entry in the PRD (grep: only MS-25 at `:156`). Under the user's decision, only the "8 h" wording changes, with no new FR.
- `context/foundation/shape-notes.md` holds older copies of the 8 h wording (`:44`, `:46`, `:125`, `:134`, `:189-190`, `:230`, `:236`). They are pre-PRD history and are not expected to change.
- `AGENTS.md:30`: domain rules, including the work-time norm, live in Rust and are unit-tested there.

### 7. Tests that encode the 8 h norm or the day-off contract

- `src-tauri/src/work_time/norm_tests.rs`:
  - `manual_only()` struct literal (`:22-26`);
  - `DAILY_NORM_MS * 21/20` (`:111`, `:115`);
  - worked examples with norms of 48 h, 160 h, 176 h, 184 h, 168 h, needed 8:48 and deviations (`:162-339`, `:652-669`, `:707-724`);
  - `set_day_off` idempotence and weekend refusal (`:341-369`);
  - three day kinds (`:399-410`);
  - `day_norm_ms == DAILY_NORM_MS` (`:418-438`, `:625-650`);
  - `day_deviation_ms` (`:441-452`);
  - days sum to the month (`:455-480`);
  - future-month rows (`:531-564`);
  - the **proptest** "day rows sum to month figures over random days off" (`:566-617`). Half days and a non-8 h norm must keep this invariant (archived `worktime-days-table/plan.md:91`).
- `src-tauri/src/work_time/tests.rs`: uses `manual_only()` (e.g. `:7`, `:113`) and asserts no norm numbers.
- `src-tauri/tests/work_time.rs`:
  - days off survive reopen (`:65-99`);
  - reminder facts (`:225-237`);
  - holiday test with the hard-coded `-8 * 3_600_000 * 7` (`:286`) and a struct literal (`:302-304`);
  - `user_version == MIGRATIONS.len()` (`:383-385`).
- `src-tauri/tests/common/mod.rs:19-23`: the `manual_only()` struct literal.
- `src-tauri/src/work_time/commands.rs` tests (`:240-314`) compare against `WorkTimeSettings::default()`.

### 8. Crash harness for the data slice

- `src-tauri/tests/common/crash.rs`: `WRITER_DIR_ENV` (`:14`), `KillAt { AfterCommits, AfterDelay }` (`:18-23`), `MAX_WRITER_COMMITS = 20_000` (`:29`), and `run_writer_until(test_name, dir, kill) -> u64` (`:45-116`). The last one re-runs the test binary as an `--ignored` writer, parses `committed <n>` lines and kills the child.
- Template: `src-tauri/tests/days_off_crash.rs`.
  - The writer (`:46-80`) resumes after the last stored day and marks distinct invented weekdays with `report::set_day_off(..., true, ..., &common::manual_only())`, one commit each.
  - The parent (`:82-121`) runs 10 kill rounds. Each round checks `PRAGMA quick_check == "ok"`, `rows >= last reported commit`, and that rows equal a contiguous weekday run (`first_weekdays`, `:123-133`).
  - A half-day crash test can follow this shape and also assert each row's `kind`: for example, odd commits mark half, even mark full, and there is never a torn or missing kind.

## Code References

- `src-tauri/src/work_time/norm.rs:14` — `DAILY_NORM_MS` (the 8 h constant).
- `src-tauri/src/work_time/norm.rs:49-59` — `day_norm_ms`.
- `src-tauri/src/work_time/norm.rs:113-152` — `figures` (count × constant; needed per day).
- `src-tauri/src/work_time/report.rs:23-30` — `DayKind`.
- `src-tauri/src/work_time/report.rs:67-119` — days-off reads, `classify`, `day_kind`.
- `src-tauri/src/work_time/report.rs:122-183` — `compute` (the only production caller of `norm.rs`).
- `src-tauri/src/work_time/report.rs:211-238` — `set_day_off(off: bool)`.
- `src-tauri/src/work_time/mod.rs:180-215` — `status` → `month_deviation_ms`.
- `src-tauri/src/work_time/reminders.rs:144` — reminder gated on `DayKind::Workday`.
- `src-tauri/src/work_time/commands.rs:23-53`, `:213-237` — settings policy, report and day-off commands.
- `src-tauri/src/settings/work_time.rs:12-88` — `WorkTimeSettings`, `RawWorkTime`, the parser.
- `src-tauri/src/store/schema.rs:90-104` — v5 `days_off`.
- `src-tauri/src/store/schema.rs:247-251` — v11, the last migration.
- `src/worktime/types.ts:52`, `:79`, `:83-142` — TS mirrors.
- `src/worktime/api.ts:56-58`, `:148-149` — `setDayOff`, the `SettingsInvalid` text.
- `src/worktime/useDayOffToggle.ts:21-53`, `src/worktime/DaysTable.tsx:58`, `:75-80`, `:106-119` — the table switch.
- `src/worktime/useWorkEntries.ts:124-162`, `src/worktime/WorkTimeView.tsx:150-169` — the selected-day checkbox.
- `src/worktime/WorkTimeView.tsx:65`, `:71`, `:75` — "8 h" help text.
- `src/worktime/MonthReport.tsx:9-48`, `src/todo/summary.ts:23-71` — month and dashboard tiles.
- `src/components/HoursBar/HoursBar.tsx:17` — bar scale `max = 10` h.
- `README.md:55`, `:111`, `:123`; `context/foundation/prd.md:68`, `:117`, `:155`, `:176`, `:205`, `:211` — 8 h wording.
- `src-tauri/tests/days_off_crash.rs:46-133`, `src-tauri/tests/common/crash.rs:14-116` — crash harness.

## Architecture Insights

- **One rule, one module.** All norm numbers come from `norm.rs` through `report::compute`. The UI and the dashboard only display them. S-08 (week summary) must reuse the same functions (`roadmap.md:198`). Changing `figures` to sum per-day norms, instead of count × constant, keeps one source for month, week and day. A helper such as `day_norm_ms(day, kind, daily_norm)` shared by month and week would serve S-08 directly.
- **The contract changes in lockstep.** These change together with their TS mirrors in one commit:
  - `DayKind` gets a new variant;
  - `set_day_off(off: bool)` becomes a three-state input, for example `state: "workday" | "halfDayOff" | "dayOff"` or a new command;
  - optionally `MonthReport` gets a half-day list or count.
- **Settings shape.** `deny_unknown_fields` plus `Copy + Eq` steer the new field to an integer representation. A missing or `null` key gives the default 8 h. Any other wrong value invalidates the whole `workTime` block. The report then refuses (Strict), and START/STOP keeps working (Lenient).
- **Retroactive norm.** With one setting and no history, `compute` recomputes every month with the current norm. Past months' norm and deviation change after an edit. This is the user-accepted consequence to record in plan and README.
- **Store.** v12 is appended. The `days_off` primary key keeps one state per day. The change spans one table, so one `write` holds it.

### Agent defaults proposed for the plan (default chosen by agent, user may revise)

- **D1: key and range.** The key is `workTime.dailyNormHours`: a JSON number of hours, default 8, range 1..=10, in whole minutes.
  - Whole minutes means `hours × 60` must be an integer, so 7.5 and 6.4 are valid and 7.33 is invalid.
  - The upper bound 10 matches `HoursBar` `max = 10`.
  - Internally it is integer ms, which keeps `Copy + Eq`.
  - `null` or absent gives 8.
- **D2: half-day norm.** A half day's norm is `daily_norm_ms / 2`, an exact integer in ms for any whole-minute norm.
- **D3: blocked days.** A half day is refused on weekends (`NotAWorkday`) and on weekday holidays (`Holiday`), like the day-off switch (`report.rs:218-223`). A holiday shadows a stored half-day row as it does a full one.
- **D4: storage.** v12 adds `kind TEXT NOT NULL DEFAULT 'full' CHECK (kind IN ('full','half'))` to `days_off`.
- **D5: table control.** The days-table control cycles workday → half day off → day off → workday. It is hatched for a full day off and gets a distinct mark for a half day.

## Historical Context (from prior changes)

- `context/archive/2026-11-23-work-norm-report/plan-brief.md:34`, `:47` and `plan.md:44`: the fixed 8 h, whole days off only, and "configurable norm, half days off" were explicitly out of scope at the time. These are still accurate as history; S-02 lifts them.
- `context/archive/2026-11-23-work-norm-report/plan.md:80-81`, `:116`: the `days_off` table, and an idempotent `set_day_off` that refuses weekends. Still true (`report.rs:211-238`). The migration was renumbered v3 → v5 (`schema.rs:90-92`).
- `context/archive/2026-11-24-worktime-days-table/plan-brief.md:24`, `plan.md:91`: "the UI does not know the number 8 h" is partly true today. The numbers come from Rust, but the `HowCounted` help text hard-codes "8 h" (`WorkTimeView.tsx:65`, `:71`, `:75`). The invariant Σ day norms == month norm is still guarded by the proptest (`norm_tests.rs:566-617`).
- `context/archive/2026-11-26-public-holidays-norm/plan.md:75`, `:190`, `:205`, `:217`, `:681` (Q6) and `:684` (D2): one shared `WorkTimeSettings` that S-02 extends, with its key added to `RawWorkTime`. The code matches this (`settings/work_time.rs:12`, `:31-36`).
- `context/archive/2026-11-26-public-holidays-norm/plan.md:446-451`: the Strict/Lenient policy split. The code matches this (`commands.rs:23-53`).
- `CHANGELOG.md:132`, "deviation in the header", is history. The current header shows only today's total (`WorkTimerHeader.tsx:67-70`), and `monthDeviationMs` has no UI consumer.

## Related Research

- `context/archive/2026-11-26-public-holidays-norm/research.md` — holiday rule and the shared settings block (`:74`, `:121`, `:262`).
- `context/archive/2026-11-23-work-norm-report/` and `context/archive/2026-11-24-worktime-days-table/` — the original norm and days-table decisions.

## Open Questions

Each question has an agent default the plan may adopt. None blocks planning.

- **Q1. Average per remaining day with half days.** Today it is `remaining / workdays_after_today` (`norm.rs:145-148`), where every day weighs the same.
  - Agent default: divide by full-day equivalents (a half day counts 0.5) and show "per full day".
  - Alternative: keep a plain day count.
  - Also needed: what `workdays` and `workdaysAfterToday` mean in the tiles (`MonthReport.tsx:30-48`, `summary.ts:51-69`).
- **Q2. "Dni wolne" tile and the `daysOff` list.**
  - Agent default: keep `daysOff` as full days only and add a separate `halfDaysOff` list, with the tile showing e.g. "2 + 1 × ½".
  - Alternative: count 0.5.
- **Q3. Start reminder on a half day off** (`reminders.rs:144`).
  - Agent default: suppress it, like a day off, because the working half may be the afternoon.
  - Alternative: treat a half day as a workday for the reminder.
- **Q4. "Wybrany dzień" checkbox** (`WorkTimeView.tsx:150-169`).
  - Agent default: give it the same three-state control as the table, so both paths share one contract.
- **Q5. Migration number collision.** Parallel slice worktrees exist at the same base `d34904a` (`git worktree list`): `end-of-day-summary`, `month-report-copy`, `week-summary` and `windows-autostart`. If another slice also appends a v12, the later merge renumbers its migration. This happened before (`schema.rs:90-92`). This run did not inspect the other worktrees.
- **Q6. Lenient fallback.** A broken `workTime` block makes `status`, `entries_for_day` and reminders use the default 8 h silently (`commands.rs:47-50`). No UI shows the status deviation today, but S-07 may. The plan should decide whether S-07's consumer needs Strict.
