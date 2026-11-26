# Polish public holidays in the work-time norm — Implementation Plan

## Overview

Roadmap S-01 (M-3), GitHub issue #25, anchored as MS-25 (no new FR). Polish public holidays are computed locally in Rust and counted out of the month's norm. In the days table they look like a day off: hatched, no switch, and the holiday's name. A Saturday holiday gets a hint that a day off is owed for it. The calendar is controlled by `"workTime": { "holidays": "pl" }` in `settings.json`, where `false` turns it off. The PRD Non-Goal and the rule text change with it, and the README documents the key.

## Current State Analysis

- The workday rule is Monday–Friday minus the rows in `days_off`. It lives in two places in Rust:
  - pure functions in `src-tauri/src/work_time/norm.rs` (`is_weekend` :24, `workdays` :30, `day_norm_ms` :44, `figures` :104);
  - store-backed classification in `src-tauri/src/work_time/report.rs`: `kind_of` :90 and `day_kind` :100 duplicate the precedence (weekend, then day off); `compute` :111 builds the report from one `store.read`; `set_day_off` :191 refuses a weekend with `NotAWorkday` before any write; `is_workday` :217 is used by tests only.
- Five consumers depend on that rule:
  - the month report (`commands.rs:129`);
  - the header's month deviation (`mod.rs:188`, `status` calls `report::compute`);
  - the "Wybrany dzień" day kind (`mod.rs:239`);
  - the "not running after HH:MM" reminder (`reminders.rs:69`, gated on `DayKind::Workday` at `reminders.rs:143`);
  - the dashboard's "Średnio / dzień", through `workdaysAfterToday`. No TS change is needed for it.
- No work-time command reads settings today. `run` (`commands.rs:20-43`) passes only the store and the clock. The precedent for reading settings on every call is `todo/commands.rs:25-30` (`settings_path` + loader). The reminder round re-reads its block every minute (`work_reminders.rs:48-56`).
- There is no `workTime` block. The template is the `reminders` block (`settings.rs:253-395`):
  - `deny_unknown_fields`;
  - an absent or `null` block or field means the defaults;
  - `false` means off;
  - anything else is a fixed-text `SettingsInvalid` with no detail.
- `settings.rs` is 1692 lines with its tests inline. Parallel slices S-03 and S-06 also edit it.
- UI (`src/worktime/`):
  - `DaysTable.tsx:40,52,94` keys the muted text and the hidden switch on `kind === "weekend"`, and the hatch on `kind === "dayOff"`;
  - `WorkTimeView.tsx:150` shows the "Dzień wolny" checkbox when `dayKind !== "weekend"`;
  - `HowCounted` (`WorkTimeView.tsx:60-85`) says holidays are marked manually;
  - `types.ts:73,77-85` holds `DayKind` and `DayReport`, `types.ts:8-19` `WorkTimeError`;
  - `api.ts:50,141` holds the set-day-off doc comment and the `NotAWorkday` text.
- Existing tests use 2026-11-11 and 2026-12-24 as *manual* days off. Both are holidays from 2025, and a proptest marks random November weekdays off:
  - `norm_tests.rs:75,200,242,388,573`;
  - `tests/work_time.rs:63-89`;
  - `tests/days_off_crash.rs:68`, which walks weekdays from a first day.

## Desired End State

- With the default settings (no `workTime` block), November 2026 has 20 workdays and a norm of 160 h with no day marked:
  - 11.11 shows hatched as "Święto Niepodległości" with no day-off switch;
  - 1.11 (a Sunday) shows its name, muted, with no hint.
- December 2026:
  - 24.12 and 25.12 are hatched holidays, giving 21 workdays and 168 h;
  - 26.12 (a Saturday) is muted, with its name and the hint that a day off is owed.
- The same rule reaches the header deviation, the "Wybrany dzień" switch (hidden on a holiday), the reminder (silent on a holiday) and the dashboard average.
- `set_day_off` on a weekday holiday is refused with `Holiday` and writes nothing.
- `"workTime": { "holidays": false }` gives today's behaviour. Holiday dates are ordinary weekdays again, and earlier manual rows on them show as days off again.
- Any other value is invalid:
  - the month report and the days panel show `ErrorState` with a fixed Polish text, the dashboard's month tiles show "raport niedostępny", and setting a day off is refused;
  - the header (START/STOP), the entries panel and the reminders keep working on the default calendar.
- The PRD (rule text and Non-Goal) and the README (feature bullet, JSON example, table row, the invalid-block note) describe the calendar.

### Key Discoveries:

- `chrono` is already a dependency (`src-tauri/Cargo.toml:52`). No new crate is needed.
- `figures` (`norm.rs:104`) counts only the `workdays` slice, so it needs no change. Holidays enter through `workdays` and `day_norm_ms`.
- `compute` reads everything in one snapshot (`report.rs:123`). Holidays are computed, not stored, so there is no extra read and no migration.
- `work_time::reminders` already takes a settings struct from `crate::settings` (`reminders.rs:20`, `ReminderSettings`). That is the precedent for domain functions taking `&WorkTimeSettings`.
- A child module can read the parent's private items. `settings/work_time.rs`, as a submodule of `settings.rs`, can use `super::SUPPORTED_VERSION` and keeps the `settings.rs` diff to two lines.
- No two Polish holidays share a date:
  - Easter Monday falls 23.03–26.04;
  - Zielone Świątki falls 10.05–13.06;
  - Boże Ciało falls 21.05–24.06.
- Meeus/Jones/Butcher checked by hand: 2026 gives 5 April, which matches the issue.

## What We're NOT Doing

- No automatic "day off owed" for a Saturday holiday. It stays a manual day off, with a hint only (U1).
- No calendars other than `"pl"`, no regional or company holidays, no user-defined holiday list.
- No model of pre-2011 law (6.01 counts from 2011) or of the pre-1951 calendar. 24.12 counts from 2025.
- No holiday name in the "Wybrany dzień" panel. `DayEntries` only carries the new kind (Q7).
- No "Święta" stat tile. The "Dni wolne · oznaczone ręcznie" stat keeps its label and counts only manual rows not shadowed by a holiday (Q4).
- No new health source for an invalid `workTime` block. It touches `health/` and the tray source list, which S-03 also changes. The error shows in "Czas pracy" instead (Q1).
- No deletion or migration of existing `days_off` rows on holiday dates. They are shadowed, not removed.
- No new Tauri command, no `lib.rs` change, no capability or CSP change.
- No configurable daily norm (S-02), but its key goes into the same `workTime` block later.
- No edit of `context/foundation/roadmap.md`.

## Implementation Approach

Build bottom-up so that each phase compiles, tests green and commits on its own:

1. A pure holiday calendar with no store, clock or settings.
2. The `workTime` settings block as a standalone loader that nothing calls yet.
3. The calendar threaded through the norm, the day classification and the day-off guard. Store-backed domain functions take `&WorkTimeSettings`, so S-02 can add the daily norm without changing signatures again. The commands pass the defaults for now. `DayKind`, `WorkTimeError::Holiday` and their TS mirrors change in this phase, and every existing test moves to the "holidays off" helper.
4. The report rows carry the holiday's name and the Saturday hint, and `MonthReport.daysOff` drops shadowed rows. `DayReport` and its TS mirror change in this phase.
5. The settings read at the edges, with an error policy per command, plus the `SettingsInvalid` contract.
6. The UI: drawing a holiday, gating the switches and the "Jak liczę" text.
7. PRD and README.

**Parallel slices.** S-03 (tray-timer-control) and S-06 (work-entry-note) edit `lib.rs`, `settings.rs`, `WorkTimeView.tsx` and the README at the same time. This plan keeps those edits small and local:

- `lib.rs`: no change;
- `settings.rs`: one `mod` line and one `pub use` line, with all new code in `settings/work_time.rs`;
- `WorkTimeView.tsx`: the switch gate and the first and third `HowCounted` paragraphs;
- README: one bullet line, one JSON line, one table row and one sentence.

S-03 or S-06 code that calls `work_time::status`, `entries_for_day` or `reminder_facts` gets a new parameter. Whichever slice merges second fixes that call site, which is a mechanical compile error. A production caller outside the commands (e.g. an S-03 tray tooltip that reads `status`) gets its settings from the Phase 5 helper `work_time::commands::work_time_settings(path, Policy::Lenient)`, never from `WorkTimeSettings::default()`, so it follows the user's calendar.

## Critical Implementation Details

- **Test fixtures (Phase 3).** 2026-11-11 and 2026-12-24 are holidays under the default calendar, so every existing test that marks them as *manual* days off must pass the "holidays off" settings. Otherwise `set_day_off` is refused and `.unwrap()` panics. This also covers the proptest at `norm_tests.rs:541-584` and `tests/days_off_crash.rs`. Add one shared test helper (e.g. `manual_only()` returning `WorkTimeSettings { holidays: HolidayCalendar::Off }`) rather than inlining the struct at each of about 50 call sites. The worked-example figures do not change.
- **Settings read outside the store lock (Phase 5).** Read `settings.json` inside the blocking task but *before* `with_store`. The health round holds the store lock during reopen and backup (AGENTS.md), and a file read should not lengthen that.
- **One classification function (Phase 3).** Replace the duplicated precedence in `kind_of` and `day_kind` with one function that both use. The order is weekend > holiday > dayOff > workday. A third copy would drift.

## Phase 1: Holiday calendar (pure Rust)

### Overview

A pure module that knows the Polish public holidays of any year and the Easter date. It is not wired to anything yet.

### Changes Required:

#### 1. Holiday module

**File**: `src-tauri/src/work_time/holidays.rs` (new). Register it as `pub mod holidays;` in `src-tauri/src/work_time/mod.rs`, with `#[cfg(test)] mod holidays_tests;` next to the existing test modules.

**Intent**: One place that answers "is this day a Polish public holiday, and what is its name". No store, clock or settings, like `norm.rs`.

**Contract**:

- `pub enum HolidayCalendar { Off, Pl }`: `Debug, Clone, Copy, PartialEq, Eq`. No `Default` here; the default lives in `WorkTimeSettings` (Phase 2).
- `impl HolidayCalendar { pub fn holiday(self, day: NaiveDate) -> Option<&'static str> }`:
  - `Off` always gives `None`;
  - `Pl` gives the name when `day` is a holiday.
- `pub fn easter_sunday(year: i32) -> Option<NaiveDate>`: the anonymous Gregorian algorithm (Meeus/Jones/Butcher). It is valid from 1583, so it returns `None` for `year < 1583`. It builds the date with `NaiveDate::from_ymd_opt` and never panics. When it is `None`, `Pl` has the fixed dates only. `parse_day` accepts years 1–9999, so the report can ask for any of them.
- `pub fn polish_holidays(year: i32) -> Vec<(NaiveDate, &'static str)>`: sorted by date. `holiday` agrees with it for every day of the year.
- The calendar `Pl`, current law with two year gates. The fixed names are part of the UI text:

| Date | Name | Note |
|---|---|---|
| 01-01 | "Nowy Rok" | |
| 01-06 | "Święto Trzech Króli" | only `year >= 2011` |
| Easter | "Wielkanoc" | always a Sunday |
| Easter + 1 | "Poniedziałek Wielkanocny" | |
| 05-01 | "Święto Pracy" | |
| 05-03 | "Święto Konstytucji 3 Maja" | |
| Easter + 49 | "Zielone Świątki" | always a Sunday |
| Easter + 60 | "Boże Ciało" | |
| 08-15 | "Wniebowzięcie NMP" | |
| 11-01 | "Wszystkich Świętych" | |
| 11-11 | "Święto Niepodległości" | |
| 12-24 | "Wigilia" | only `year >= 2025` |
| 12-25 | "Boże Narodzenie" | |
| 12-26 | "Drugi dzień Bożego Narodzenia" | |

The Easter algorithm, using integer division, as the reference the tests check:

```
a = Y % 19; b = Y / 100; c = Y % 100; d = b / 4; e = b % 4
f = (b + 8) / 25; g = (b - f + 1) / 3
h = (19a + b - d - g + 15) % 30; i = c / 4; k = c % 4
l = (32 + 2e + 2i - h - k) % 7; m = (a + 11h + 22l) / 451
month = (h + l - 7m + 114) / 31; day = (h + l - 7m + 114) % 31 + 1
```

#### 2. Unit tests

**File**: `src-tauri/src/work_time/holidays_tests.rs` (new)

**Intent**: Pin the algorithm and the calendar on known dates.

**Contract**:

- Easter on at least 2000-04-23, 2008-03-23, 2011-04-24, 2019-04-21, 2024-03-31, 2025-04-20, 2026-04-05, 2027-03-28, 2028-04-16 and 2038-04-25.
- Easter Monday on 2025-04-21, 2026-04-06, 2027-03-29 and 2028-04-17.
- Boże Ciało on 2025-06-19, 2026-06-04, 2027-05-27 and 2028-06-15.
- 2026 has 14 holidays, 8 of them Monday–Friday.
- 2024-12-24 is not a holiday and 2025-12-24 is. 2010-01-06 is not and 2011-01-06 is.
- `Off` returns `None` for 2026-11-11.
- Easter and Zielone Świątki are Sundays and Boże Ciało a Thursday for every year 1990–2100.
- No two holidays share a date for any year 1990–2100.
- `holiday(d)` equals the lookup in `polish_holidays(d.year())` for every day of 2025–2028.
- `easter_sunday(1582)` is `None`. `polish_holidays` for years 1, 1582 and 9999 does not panic, and for year 1 it holds only the fixed dates.

### Success Criteria:

#### Automated Verification:

- Holiday unit tests pass: `cargo test holidays` (in `src-tauri/`)
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)

**Implementation Note**: commit as `feat(public-holidays-norm): holiday calendar` after the automated checks pass.

---

## Phase 2: `workTime` settings block

### Overview

A standalone loader for `"workTime": { "holidays": "pl" | false }`. It is built as the one shared struct that S-02 extends with the daily norm. Nothing calls it yet.

### Changes Required:

#### 1. Settings submodule

**File**: `src-tauri/src/settings/work_time.rs` (new). `src-tauri/src/settings.rs` gets exactly `mod work_time;` and `pub use work_time::{WorkTimeSettings, WorkTimeSettingsInvalid, load_work_time_settings};` near the top, with no other edit (parallel slices).

**Intent**: Parse the block the way `reminders` is parsed (`settings.rs:298-395`), in a separate file so `settings.rs` barely changes.

**Contract**:

- `pub struct WorkTimeSettings { pub holidays: HolidayCalendar }`:
  - `Debug, Clone, Copy, PartialEq, Eq`;
  - a manual `Default` with `holidays: HolidayCalendar::Pl` (A1);
  - doc comment: S-02 adds its daily-norm field here.
- `pub struct WorkTimeSettingsInvalid;`, a unit struct with no detail, like `RefreshSettingInvalid` (`settings.rs:405-406`).
- `pub fn load_work_time_settings(path: &Path) -> Result<WorkTimeSettings, WorkTimeSettingsInvalid>`. The defaults come from:
  - a missing file;
  - an absent or `null` `workTime` block;
  - an absent or `null` `holidays` key.
- Value domain:
  - `"pl"` gives `Pl`;
  - `false` gives `Off`;
  - anything else is invalid: `true`, `"PL"`, `""`, other strings, numbers, arrays and objects.
- These are also invalid:
  - a non-object block;
  - an unknown key in the block (`#[derive(Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")] struct RawWorkTime { #[serde(default)] holidays: Option<serde_json::Value> }`; S-02 adds its key to this struct);
  - unreadable file, non-UTF-8, malformed JSON, or `version` other than 1.
- A BOM is stripped, as in the other loaders.

#### 2. Tests

**File**: `src-tauri/src/settings/work_time.rs` (`#[cfg(test)] mod tests` in the same file)

**Intent**: Mirror the reminders tests (`settings.rs:1111-1229`) for this block.

**Contract**:

- Defaults: missing file; `{"version":1}`; `"workTime": null`; `"workTime": {}`; `"holidays": null`.
- `"pl"` and `false` map correctly.
- Every invalid value listed above is refused.
- An unknown key is refused.
- A BOM is accepted.
- Isolation: a broken `workTime` block leaves `load_reminder_settings` and `load_todo_limit` green, and a broken `reminders` block leaves `load_work_time_settings` green.

### Success Criteria:

#### Automated Verification:

- Settings tests pass: `cargo test settings::work_time::` (in `src-tauri/`; the tests live in `settings::work_time::tests`)
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- `settings.rs` diff is only the `mod` and `pub use` lines: `git diff --stat src-tauri/src/settings.rs` shows two added lines

**Implementation Note**: commit as `feat(public-holidays-norm): workTime settings block`.

---

## Phase 3: Holidays in the norm, day kinds and day-off guard

### Overview

The calendar enters the rule everywhere it is used: the norm, the report, the header deviation, the day panel's kind and the reminder gate. `DayKind`, the new error and their TS mirrors change together. The commands and the reminder round pass `WorkTimeSettings::default()` until Phase 5. The holiday's name, the Saturday hint and the `daysOff` filter come in Phase 4, so this phase is the rule, the signatures and the test migration only.

### Changes Required:

#### 1. Pure norm

**File**: `src-tauri/src/work_time/norm.rs`

**Intent**: A holiday is not a workday and has a norm of 0.

**Contract**:

- `workdays(first, next_first, days_off, holidays: HolidayCalendar)` and `day_norm_ms(day, days_off, holidays: HolidayCalendar)` also exclude a day for which `holidays.holiday(day).is_some()`.
- `figures` is unchanged.
- Update the doc comments.

#### 2. Day kind, report rows and the guard

**File**: `src-tauri/src/work_time/report.rs`

**Intent**: Classify holidays and refuse marking a holiday.

**Contract**:

- `DayKind` gains `Holiday` (serialized `"holiday"`).
- One private classification function (e.g. `fn classify(day, marked_off: bool, holidays: HolidayCalendar) -> DayKind`) replaces the logic in `kind_of` and `day_kind`, both of which call it. Precedence:
  - a weekend is `Weekend` (a Saturday or Sunday holiday too);
  - a weekday holiday is `Holiday` (even with a `days_off` row: the holiday wins, A2);
  - a row in `days_off` is `DayOff`;
  - otherwise `Workday`.
- `compute`, `month_report`, `set_day_off`, `is_workday` and `pub(super) day_kind` take `settings: &WorkTimeSettings` and use `settings.holidays`.
- `DayReport` and `MonthReport.days_off` keep their current shape in this phase (Phase 4 changes them).
- `set_day_off` refuses, before any write and in both directions (`off` true or false):
  - a weekend with `NotAWorkday`, as today;
  - then a weekday holiday with the new `WorkTimeError::Holiday`.
- `is_workday` is false on a holiday.

#### 3. Callers

**Files**: `src-tauri/src/work_time/mod.rs`, `src-tauri/src/work_time/reminders.rs`, `src-tauri/src/work_time/commands.rs`, `src-tauri/src/work_reminders.rs`

**Intent**: Thread the settings through. There is no behaviour change in these files beyond the new rule.

**Contract**:

- `status(store, now, tz, settings: &WorkTimeSettings)`.
- `entries_for_day(store, day, now, tz, settings: &WorkTimeSettings)`.
- `reminder_facts(store, now, tz, settings: &WorkTimeSettings)`.
- `due_reminders` is unchanged: `not_running_due` already stays silent when the kind is not `Workday`.
- `commands.rs` and `work_reminders.rs` pass `&WorkTimeSettings::default()` (replaced in Phase 5). No change to `lib.rs`.

#### 4. Error variant

**File**: `src-tauri/src/work_time/error.rs`

**Intent**: Name the refusal.

**Contract**: `Holiday` with `#[error("a public holiday is not a workday")]`, `kind()` returning `"Holiday"`, and no detail.

#### 5. TS mirror (same commit)

**Files**: `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: Keep the contract in step (AGENTS.md).

**Contract**:

- `DayKind = "workday" | "dayOff" | "holiday" | "weekend"`.
- `WorkTimeError` gains `| { kind: "Holiday" }`.
- In `api.ts`:
  - `workTimeErrorMessage` maps `Holiday` to "Święto nie jest dniem roboczym";
  - the `setDayOff` comment says a weekend is refused with `NotAWorkday` and a holiday with `Holiday`.
- No UI behaviour change in this phase. Holiday rows still show a switch until Phase 6, and the backend refuses it.

#### 6. Tests

**Files**:

- `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/src/work_time/tests.rs` and `src-tauri/src/work_time/reminders_tests.rs`;
- `src-tauri/tests/work_time.rs` and `src-tauri/tests/days_off_crash.rs`;
- the shared helper in `src-tauri/tests/common/` if the integration tests need one.

**Intent**: Existing tests keep meaning "a manual day off". New tests pin the holiday rule.

**Contract**:

- Every existing call passes the "holidays off" helper where it marks or expects manual days off, and the default settings elsewhere. The worked example stays at 20 workdays and 160 h. Unit tests share one `#[cfg(test)]` helper in `work_time/` (e.g. in `norm_tests.rs`, used by `tests.rs` too). Integration tests cannot see it, so they get their own copy in `tests/common/mod.rs` (`WorkTimeSettings` and `HolidayCalendar` are reachable as `day_navigator_lib::settings::WorkTimeSettings` and `day_navigator_lib::work_time::holidays::HolidayCalendar`). Fixtures to move: the `worked_example` and `report_at` helpers (`norm_tests.rs:72-82`), the direct `set_day_off` calls (`norm_tests.rs:200,220,242,332-353,380,388,573`), `tests/work_time.rs:51,70-89,131-133,145-207` and `tests/days_off_crash.rs:68` (its first day, 2026-01-01, is Nowy Rok).
- New tests with `Pl` (invented data only):
  - November 2026 with no marked day: 20 workdays, norm 160 h. 11.11 is `Holiday` with norm 0. 1.11 is `Weekend`.
  - December 2026: 21 workdays, 168 h. 24.12 and 25.12 are `Holiday`. 26.12 is `Weekend`.
  - April 2026: 5.04 is `Weekend` and 6.04 is `Holiday`.
  - `set_day_off(2026-11-11, true)` and `(…, false)` give `Holiday`, and no row is written.
  - A shadowed row: mark 2026-11-11 under `Off`, then report under `Pl`. The day is `Holiday` and the norm is 160 h. Under `Off` it is `DayOff` again.
  - `status` month deviation and `entries_for_day` `day_kind` (`Holiday`) on a holiday.
  - `reminder_facts` gives `day_kind == Holiday` on 2026-11-11.
  - The reminders test `not_running_is_silent_when_it_should_be` gains a `DayKind::Holiday` case.
  - The proptest `day_rows_add_up_to_the_month_figures` (`norm_tests.rs:541-584`) runs under both calendars: under `Pl` it skips marking holidays, and the day norms still add up to the month norm.
  - `DayKind::Holiday` serializes as `"holiday"`, and `WorkTimeError::Holiday` as `{"kind":"Holiday"}`.

### Success Criteria:

#### Automated Verification:

- Work-time tests pass, old and new: `cargo test work_time` and `cargo test --test work_time` (in `src-tauri/`)
- Crash harness test still passes: `cargo test --test days_off_crash` (in `src-tauri/`)
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- No `lib.rs` change: `git diff --quiet HEAD -- src-tauri/src/lib.rs`

**Implementation Note**: commit as `feat(public-holidays-norm): holidays in the norm and day kinds`. The Rust types and the TS mirror go in this one commit.

---

## Phase 4: Holiday name, Saturday hint and shadowed rows in the report

### Overview

The report rows carry what the days table needs to draw a holiday, and the "Dni wolne · oznaczone ręcznie" stat stops counting rows a holiday shadows. `DayReport` and its TS mirror change together. No UI behaviour change yet.

### Changes Required:

#### 1. Report rows

**File**: `src-tauri/src/work_time/report.rs` (`compute`, `DayReport`, `MonthReport`)

**Intent**: Carry the name and the Saturday hint to the UI (U1), and keep the manual-days-off list true to its label (Q4).

**Contract**:

- `DayReport` gains `holiday: Option<String>` and `day_off_owed: bool`, serialized `holiday` and `dayOffOwed`:
  - `holiday` is the name under the active calendar on any kind (weekday, Saturday or Sunday), `None` otherwise;
  - `day_off_owed` is true exactly when the holiday falls on a Saturday (U1).
- `MonthReport.days_off` lists only rows whose day is not a holiday under the active calendar (Q4). Update its doc comment.

#### 2. TS mirror (same commit)

**File**: `src/worktime/types.ts`

**Contract**: `DayReport` gains `holiday: string | null` and `dayOffOwed: boolean`, with doc comments. `MonthReport.daysOff` gets a doc comment that shadowed rows are left out.

#### 3. Tests

**Files**: `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/tests/work_time.rs`

**Contract** (with `Pl`, invented data only):

- November 2026: 11.11 has "Święto Niepodległości" and `dayOffOwed` false. 1.11 (`Weekend`) has "Wszystkich Świętych" and `dayOffOwed` false. 2.11 has `holiday` `None`.
- December 2026: 24.12 "Wigilia", 25.12 "Boże Narodzenie"; 26.12 has "Drugi dzień Bożego Narodzenia" and `dayOffOwed` true.
- August 2026: 15.08 (a Saturday) has `dayOffOwed` true.
- April 2026: 5.04 has "Wielkanoc" and `dayOffOwed` false.
- A shadowed row: mark 2026-11-11 under `Off`, report under `Pl`: `days_off` excludes it. Under `Off` it is listed again.
- Under `Off`, no row has a `holiday` name or `dayOffOwed`.
- JSON serialization of a `DayReport`: `"kind":"holiday"`, `"holiday":"Święto Niepodległości"`, `"dayOffOwed":false`, and `"holiday":null` on a plain workday.

### Success Criteria:

#### Automated Verification:

- Report tests pass, old and new: `cargo test work_time` and `cargo test --test work_time` (in `src-tauri/`)
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

**Implementation Note**: commit as `feat(public-holidays-norm): holiday names and the Saturday hint in the report`. The Rust type and the TS mirror go in this one commit.

---

## Phase 5: Settings at the edges and the invalid-block error

### Overview

Commands and the reminder round read the `workTime` block on every call or round. Each one has an explicit policy for an invalid block, so a typo never hides START/STOP.

### Changes Required:

#### 1. Error variant and mirror

**Files**: `src-tauri/src/work_time/error.rs`, `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: A fixed, value-free error for the strict commands.

**Contract**:

- Rust `SettingsInvalid` with `#[error("the workTime settings block is invalid")]`, `kind()` returning `"SettingsInvalid"`, and no detail.
- TS adds `| { kind: "SettingsInvalid" }`.
- `workTimeErrorMessage` maps it to the text "Nie można odczytać bloku „workTime” z settings.json: plik musi być poprawnym JSON-em z „version”: 1, a holidays to „pl” albo false." The loader gives the same error for a file-level problem (unreadable, bad UTF-8 or JSON, wrong `version`) as for a bad block, so the text names both (the "Ustawienia" source in **Źródła** also reports a file-level problem).

#### 2. Command policy

**File**: `src-tauri/src/work_time/commands.rs`

**Intent**: Read settings per call, off the main thread and before the store lock, with one policy per command.

**Contract**:

- A tauri-free helper resolves the settings: `pub enum Policy { Strict, Lenient }` and `pub fn work_time_settings(path: Option<&Path>, policy: Policy) -> Result<WorkTimeSettings, WorkTimeError>`. It is `pub`, so other Rust callers (the reminder round here, an S-03 tray caller later) reuse it. Unit-test it in a `#[cfg(test)]` block of this file, with `tempfile`:
  - the path comes from `settings_file::settings_path(&app)`, resolved before the blocking task as in `work_reminders::reminder_settings`;
  - `Ok(defaults)` when the path is `None` (config dir unresolved, like the reminders);
  - `Strict` gives `Err(WorkTimeError::SettingsInvalid)` on an invalid block;
  - `Lenient` gives the defaults plus `log::debug!` (kind only) on an invalid block.
- A sibling of `run`, `run_with_settings(app, name, policy, f: impl FnOnce(&mut Store, i64, &WorkTimeSettings) -> …)`, calls the helper inside the blocking task before `with_store` and passes `&WorkTimeSettings` to the closure. A `Strict` refusal is logged at debug like the other refusals. `run` keeps its shape for the commands that do not use the rule.
- Strict: `work_month_report`, `set_day_off`.
- Lenient: `work_timer_status` (and so `start_work` / `stop_work`), `list_work_entries`.
- Commands that do not use the rule (`add_work_entry`, `update_work_entry`, `remove_work_entry`) keep their current shape.

#### 3. Reminder round

**File**: `src-tauri/src/work_reminders.rs`

**Intent**: The workday gate follows the user's calendar.

**Contract**:

- Each round calls `work_time_settings(settings_path.as_deref(), Policy::Lenient)` before `with_store`. An invalid block or an unresolved path uses the defaults, logged at debug and not every minute at warn.
- The `reminders` block handling is unchanged.
- Update the module doc comment.

### Success Criteria:

#### Automated Verification:

- Policy helper tests pass (strict refuses, lenient falls back, no path gives defaults): `cargo test commands` (in `src-tauri/`)
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

#### Manual Verification:

- With `"workTime": { "holidays": 1 }`:
  - the month report and the "Dni miesiąca" panel show `ErrorState` with the workTime text;
  - the dashboard's three month tiles show "raport niedostępny" (it reads the same report);
  - the header counter and START/STOP still work;
  - the entries list still loads.
- After fixing the file, "Spróbuj ponownie" (or window focus) brings the report back with no restart.

**Implementation Note**: commit as `feat(public-holidays-norm): read the workTime block in commands and reminders`. The manual rows stay for the user.

---

## Phase 6: Days table and day panel UI

### Overview

Draw a holiday like a day off: hatched, no switch, with its name. Show the Saturday hint, hide the "Dzień wolny" checkbox on holidays and update "Jak liczę".

### Changes Required:

#### 1. Days table

**File**: `src/worktime/DaysTable.tsx` (layout in `src/worktime/WorkTime.css` only)

**Intent**: A holiday row reads as a day off the user did not have to mark (issue #25: "zakreskowane jak dzień wolny, z nazwą święta").

**Contract**:

- `hatched` when `kind` is `"dayOff"` or `"holiday"`.
- The day-off switch renders only for `"workday"` and `"dayOff"`.
- Text tone: muted only for weekends, as today.
- When `row.holiday` is set, the "Na tle normy" cell shows the name as text:
  - truncated, with the full text in `title`;
  - the bar renders only when `workedMs > 0` (as on a weekend; today `showBar = !weekend || workedMs > 0` would draw an empty bar on a weekday holiday), and then stays before the name;
  - on a Saturday holiday (`dayOffOwed`) the text adds "· należy się inny dzień wolny", and the `title` reads "Święto w sobotę: pracodawca wyznacza za nie inny dzień wolny — oznacz go w tabeli".
- Use existing components and tones (`DataTableCell` `tone`/`truncate`). Any new CSS is layout under a `wt-` class in `WorkTime.css`, with no literal values that belong in `tokens.css`. A new reusable component, if one is needed at all, gets a `dn-` prefix and tokens only.
- The table's column template (`WorkTime.css:20`) is unchanged.

#### 2. Day panel gate and "Jak liczę"

**File**: `src/worktime/WorkTimeView.tsx` (small, local edits; S-06 also edits this file)

**Intent**: The second switch follows the same gate, and the explanation states the new rule.

**Contract**:

- `dayOffControl` renders only when `dayKind` is `"workday"` or `"dayOff"`.
- The first `HowCounted` paragraph changes to: "Norma = 8 h × dni robocze: poniedziałek–piątek minus święta państwowe i dni oznaczone ręcznie jako wolne (urlop). Kalendarz świąt można wyłączyć w settings.json („workTime”). Za święto w sobotę pracodawca wyznacza inny dzień wolny — oznacz go ręcznie."
- The "0 h w dzień wolny i w weekend" wording in the third paragraph also covers holidays ("0 h w dzień wolny, w święto i w weekend").
- No other edits in this file.

### Success Criteria:

#### Automated Verification:

- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- No literal colours outside tokens: `git diff -U0 HEAD -- ":(glob)src/**/*.css" ":(exclude)src/styles/tokens.css" | grep -E "^\+.*(#[0-9a-fA-F]{3,8}|rgba?\(|hsla?\()"` prints nothing (Git Bash)
- Rust gates still pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)

#### Manual Verification:

- November 2026 in "Czas pracy":
  - 11.11 is hatched with "Święto Niepodległości" and no switch;
  - 1.11 is muted with "Wszystkich Świętych" and no hint;
  - the month norm is 160 h with no marked day.
- December 2026:
  - 24.12 and 25.12 are hatched with names;
  - 26.12 is muted, with its name and the "należy się inny dzień wolny" hint and tooltip;
  - the norm is 168 h.
- Picking 11.11 in "Wybrany dzień" shows no "Dzień wolny" checkbox. A normal weekday still shows it and toggling works.
- With `"workTime": { "holidays": false }` (after a refocus): 11.11 is a normal weekday with a switch, and a day off marked earlier on it shows hatched as a manual day off.
- The dashboard's "Średnio / dzień" in a month with a holiday after today matches the month report.
- The long holiday names do not break the row layout at the narrowest window width the app supports.

**Implementation Note**: commit as `feat(public-holidays-norm): show holidays in the days table`. The manual rows stay for the user.

---

## Phase 7: PRD and README

### Overview

The documents describe the calendar, and the PRD stops contradicting itself.

### Changes Required:

#### 1. PRD

**File**: `context/foundation/prd.md`

**Intent**: U2. Remove only the holiday-calendar clause of the Non-Goal, and align the rule text with it. No new FR, and the slice stays MS-25.

**Contract**:

- Line 226 Non-Goal: drop "automatycznego kalendarza świąt" and keep the rest. It reads: "**Brak automatyki czasu pracy.** Bez wykrywania bezczynności i mierzenia aktywności; czas liczy się wyłącznie z przycisków START/STOP i ręcznej korekty (FR-017)."
- FR-014 (line 155): workdays are "poniedziałek–piątek minus polskie święta państwowe (kalendarz liczony lokalnie, wyłączany w ustawieniach) i dni oznaczone przez użytkownika jako wolne".
- Its Socrates line (156): replace "bez automatycznego kalendarza świąt" with a resolution that holidays come from a local calendar (MS-25), while a day owed for a Saturday holiday and leave stay manual.
- The "Norma" paragraph (211): add holidays to the definition of workdays.
- FR-020 (176): add "święta zakreskowane z nazwą, bez przełącznika".
- Frontmatter `updated:` is set to the commit date.

#### 2. README

**File**: `README.md` (small, local edits; S-03 and S-06 also edit it)

**Intent**: Document the feature and the key.

**Contract**:

- Feature bullet (line 55): "day and month sums against an 8-hour workday norm, with Polish public holidays counted out".
- JSON example: add `"workTime": { "holidays": "pl" }`.
- Table row: `workTime` with "`holidays`: `"pl"` (default) counts Polish public holidays out of the norm; `false` turns the calendar off."
- After the table, the sentence on invalid blocks gets an exception: an invalid `workTime` block is reported in the **Czas pracy** view (month report and days table) and in the dashboard's month tiles, not in **Źródła**; START/STOP keeps working.

### Success Criteria:

#### Automated Verification:

- The Non-Goal no longer mentions the holiday calendar: `grep -n "Brak automatyki czasu pracy" context/foundation/prd.md` shows a line without "świąt", and `grep -n "automatycznego kalendarza świąt" context/foundation/prd.md` returns nothing
- README documents the key: `grep -n '"workTime"' README.md` finds the JSON line, and the table has a `workTime` row
- No new FR id: `grep -oE "FR-[0-9]+" context/foundation/prd.md | sort -u | wc -l` is unchanged from before the phase
- Frontend gates pass (README is outside `src/`, so this is a regression check): `npm run lint`, `npm run format:check`, `npm run build`

#### Manual Verification:

- The user reads the changed PRD lines (155, 156, 176, 211, 226) and the README section and accepts the wording.

**Implementation Note**: commit as `docs(public-holidays-norm): PRD and README for the holiday calendar`.

---

## Testing Strategy

### Unit Tests:

- `holidays_tests.rs`:
  - Easter on 10 known years;
  - Easter Monday and Boże Ciało on 2025–2028;
  - the 2011 and 2025 year gates;
  - 14 holidays and 8 weekday ones in 2026;
  - weekday invariants and no collisions over 1990–2100;
  - `holiday` agreeing with `polish_holidays`;
  - no panic and fixed dates only before 1583.
- `settings/work_time.rs`: defaults, both valid values, every invalid shape, an unknown key, a BOM, and block isolation.
- `norm_tests.rs`:
  - November 2026 at 160 h and December 2026 at 168 h with `Pl`;
  - kinds, names and `dayOffOwed`;
  - the guard in both directions;
  - shadowed rows under `Pl` and `Off`;
  - the day-rows proptest under both calendars;
  - serialization.
- `reminders_tests.rs`: the `Holiday` kind is silent.
- The `commands.rs` policy helper: strict, lenient, no path.

### Integration Tests:

- `tests/work_time.rs`: existing manual-day-off tests run with "holidays off". New: `reminder_facts` and `status` on 2026-11-11 with the defaults.
- `tests/days_off_crash.rs`: unchanged behaviour with "holidays off".

### Manual Testing Steps:

1. Open "Czas pracy" on November 2026 and check 11.11, 1.11 and the 160 h norm.
2. Go to December 2026 and check 24.12, 25.12, the 26.12 hint and tooltip, and the 168 h norm.
3. Pick 11.11 in "Wybrany dzień" and confirm there is no "Dzień wolny" checkbox.
4. Set `"workTime": { "holidays": false }`, refocus, and confirm 11.11 is a workday again with a switch.
5. Set `"workTime": { "holidays": 1 }`, refocus, and confirm the report error while START/STOP works. Fix the file and press "Spróbuj ponownie".

## Performance Considerations

The calendar is computed per day: one Easter computation (a dozen integer operations) per lookup, about 31 lookups per report. The settings file is read once per command call and once per reminder round, as the `todo` and `reminders` blocks already are. Both costs are negligible.

## Migration Notes

There is no schema change and no data migration. Existing `days_off` rows on holiday dates stay in the database:

- under `Pl` they are shadowed (the kind is `Holiday`, they are not listed in `daysOff`, and the norm is 0 either way);
- with `false` they show again as manual days off.

Rollback is reverting the commits, or `"holidays": false` as a runtime switch.

## Decisions

User decisions (fixed):

- **U1.** A holiday on a Saturday leaves the exchange day off manual. The days table shows a hint on that row that a day off is owed (`dayOffOwed`).
- **U2.** The PRD Non-Goal "bez automatycznego kalendarza świąt" changes in this slice. The rest of that Non-Goal stays, and the contradicting rule text (FR-014, its Socrates line, "Norma", FR-020) is aligned.

The defaults below were chosen by the agent; the user may revise them.

- **A1.** Holidays are on by default: an absent or `null` `workTime` block or `holidays` key means `"pl"`.
- **A2.** On a day that is both a holiday and a marked day off, the holiday wins. The day-off switch is blocked on holidays like on weekends: hidden in both UI places, and refused by `set_day_off` with `Holiday` in both directions.
- **A3.** The slice stays anchored as MS-25, with no new FR in the PRD.
- **Q1.** Invalid `workTime` block:
  - the month report and `set_day_off` refuse with `SettingsInvalid` (`ErrorState` in the view, "raport niedostępny" in the dashboard's month tiles, R3);
  - the header status, the day entries and the reminder round fall back to the default calendar;
  - no new health source (the S-03 parallel work owns the tray and source list).
- **Q2.** A new `WorkTimeError::Holiday` variant ("Święto nie jest dniem roboczym") rather than reusing `NotAWorkday`.
- **Q3.**
  - A weekday holiday is drawn with the same hatch as a day off, no switch, and its name in the "Na tle normy" cell.
  - Sunday holidays (Wielkanoc, Zielone Świątki, and fixed dates falling on a Sunday) show their name with no hint.
  - The hint appears only on Saturdays.
  - No new colour token.
- **Q4.** `MonthReport.daysOff` excludes rows shadowed by a holiday, so "Dni wolne · oznaczone ręcznie" means what it says. No "Święta" stat.
- **Q5.** Existing tests about manual days off pass "holidays off". New tests use the default `Pl` on known dates.
- **Q6.** Only `"pl"` and `false` are accepted, and `true` is refused, as in the reminders precedent. The block is one shared `RawWorkTime` / `WorkTimeSettings` that S-02 extends.
- **Q7.** The "Wybrany dzień" panel gets only the new kind (switch gate), not the holiday name.
- **D1.** All 14 statutory days are in the calendar, the two Sunday-only ones (Wielkanoc, Zielone Świątki) included for their names. 6.01 counts from 2011 and 24.12 from 2025.
- **D2.** Store-backed domain functions take `&WorkTimeSettings` and the pure norm functions take `HolidayCalendar`, so S-02 adds its norm without changing the store-backed signatures again.
- **D3.** The new settings code lives in `src-tauri/src/settings/work_time.rs`, a submodule of `settings.rs`, to keep the parallel-edited `settings.rs` diff to two lines.
- **D4.** The roadmap item is not set to `planning` by this plan run, because the orchestrator forbids editing `roadmap.md` here.

Plan review (`reviews/plan-review.md`) defaults, chosen by agent, user may revise:

- **R1.** The rule phase is split in two: Phase 3 (rule, signatures, `Holiday` kind and error, test migration) and Phase 4 (holiday name, `dayOffOwed`, `daysOff` filter). Each fits one fresh sub-agent and leaves every gate green.
- **R2.** A file-level problem in `settings.json` (unreadable, bad JSON, wrong `version`) is treated like an invalid `workTime` block: strict commands refuse, lenient ones use the defaults. The `SettingsInvalid` text names both causes. Falling back silently would show numbers the user may have switched off.
- **R3.** The dashboard's month tiles read the same strict report, so they show "raport niedostępny" on an invalid block. This is accepted (a surfaced failure, not an empty list) and is in the end state, the manual check and the README.
- **R4.** The settings policy helper is `pub` (`work_time::commands::work_time_settings` with `Policy`), so the reminder round and a parallel slice's tray code read the calendar the same way instead of passing the defaults.
- **R5.** `easter_sunday` is `None` before 1583, where `Pl` keeps only the fixed dates. Nothing panics for any year `parse_day` accepts.
- **R6.** A holiday row draws the hours bar only when hours were worked, like a weekend.

## References

- Related research: `context/changes/public-holidays-norm/research.md`
- Issue: GitHub #25. Roadmap: `context/foundation/roadmap.md:91-105` (S-01) and `:106-119` (S-02, same settings block).
- Rule today: `src-tauri/src/work_time/norm.rs:24-50`, `src-tauri/src/work_time/report.rs:90-222`
- Settings template: `src-tauri/src/settings.rs:253-395` (reminders), `src-tauri/src/todo/commands.rs:25-30` (loading in a command)
- Reminder round: `src-tauri/src/work_reminders.rs:48-82`
- UI: `src/worktime/DaysTable.tsx:38-106`, `src/worktime/WorkTimeView.tsx:60-85,149-167`, `src/worktime/types.ts:8-19,73-85`, `src/worktime/api.ts:50-53,141-142`
- Prior plans: `context/archive/2026-11-23-work-norm-report/plan.md`, `context/archive/2026-11-24-worktime-days-table/plan-brief.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Holiday calendar (pure Rust)

#### Automated

- [x] 1.1 Holiday unit tests pass: `cargo test holidays` — 0466d77
- [x] 1.2 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 0466d77

### Phase 2: `workTime` settings block

#### Automated

- [x] 2.1 Settings tests pass: `cargo test settings::work_time::` — 12439c1
- [x] 2.2 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 12439c1
- [x] 2.3 `settings.rs` diff is only the `mod` and `pub use` lines — 12439c1

### Phase 3: Holidays in the norm, day kinds and day-off guard

#### Automated

- [x] 3.1 Work-time tests pass, old and new: `cargo test work_time` and `cargo test --test work_time` — 794268f
- [x] 3.2 Crash harness test still passes: `cargo test --test days_off_crash` — 794268f
- [x] 3.3 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 794268f
- [x] 3.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 794268f
- [x] 3.5 No `lib.rs` change — 794268f

### Phase 4: Holiday name, Saturday hint and shadowed rows in the report

#### Automated

- [x] 4.1 Report tests pass, old and new: `cargo test work_time` and `cargo test --test work_time` — 364bbae
- [x] 4.2 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 364bbae
- [x] 4.3 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 364bbae

### Phase 5: Settings at the edges and the invalid-block error

#### Automated

- [x] 5.1 Policy helper tests pass (strict refuses, lenient falls back, no path gives defaults) — 967cc2d
- [x] 5.2 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 967cc2d
- [x] 5.3 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 967cc2d

#### Manual

- [ ] 5.4 Invalid `workTime` block shows ErrorState in the report and days panel and "raport niedostępny" on the dashboard while START/STOP and entries keep working
- [ ] 5.5 Fixing the file brings the report back without a restart

### Phase 6: Days table and day panel UI

#### Automated

- [x] 6.1 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 1a0e672
- [x] 6.2 No literal colours outside tokens — 1a0e672
- [x] 6.3 Rust gates still pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 1a0e672

#### Manual

- [ ] 6.4 November 2026 shows 11.11 and 1.11 correctly with a 160 h norm
- [ ] 6.5 December 2026 shows 24.12, 25.12 and the 26.12 Saturday hint with a 168 h norm
- [ ] 6.6 "Wybrany dzień" hides the day-off checkbox on a holiday and keeps it on a weekday
- [ ] 6.7 `"holidays": false` restores the plain weekday and shows earlier manual rows
- [ ] 6.8 Dashboard "Średnio / dzień" matches the month report with a holiday ahead
- [ ] 6.9 Long holiday names do not break the row layout

### Phase 7: PRD and README

#### Automated

- [x] 7.1 The Non-Goal no longer mentions the holiday calendar — 2a9a1dc
- [x] 7.2 README documents the key — 2a9a1dc
- [x] 7.3 No new FR id — 2a9a1dc
- [x] 7.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 2a9a1dc

#### Manual

- [ ] 7.5 The user accepts the PRD and README wording
