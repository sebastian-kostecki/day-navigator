---
date: 2026-11-26T12:18:54+01:00
researcher: Claude (research agent, non-interactive run)
git_commit: 455cbeda5879b666ed52021fe1b2bc2a02748807
branch: worktree-agent-ab918e4ae3c0dbddc
repository: day-navigator
topic: "Polish public holidays counted out of the work-time norm (S-01, MS-25, issue #25)"
tags: [research, work-time, norm, days-off, settings, holidays]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (research agent, non-interactive run)
---

# Research: Polish public holidays in the work-time norm

**Date**: 2026-11-26T12:18:54+01:00
**Researcher**: Claude (research agent, non-interactive run; no user reachable, so the defaults below are marked as agent defaults)
**Git Commit**: 455cbeda5879b666ed52021fe1b2bc2a02748807
**Branch**: worktree-agent-ab918e4ae3c0dbddc
**Repository**: day-navigator

## Research Question

How do the work-time norm, the days table, day-off marking and weekend blocking work today, and what has to change so that Polish public holidays are counted out of the month's norm? The holidays are the fixed dates (24.12 included from 2025) plus Easter Monday and Corpus Christi (Meeus/Jones/Butcher). They must show in the days table like a day off, with the holiday's name, and be switchable by a `settings.json` key with README docs. The research covers the Rust work-time modules, settings parsing and validation, the TS view, the existing tests, the Rust↔TS contracts that change, and the PRD/roadmap text to touch.

Inputs: roadmap S-01 (`context/foundation/roadmap.md:91-105`) and GitHub issue #25. Slice anchored to PRD as MS-25, with no new FR.

Decisions given by the user:

- **U1.** A holiday on a Saturday stays a manual day off. The days table shows a hint on the Saturday holiday that a day off is owed for it.
- **U2.** The PRD Non-Goal "bez automatycznego kalendarza świąt" changes in this slice. The rest of that Non-Goal stays.

Agent defaults (the user may revise them):

- **A1.** Holidays are on by default.
- **A2.** On a day that is both a holiday and a marked day off, the holiday wins. The day-off switch is blocked on holidays, as it is on weekends.
- **A3.** The slice stays anchored as MS-25.

## Summary

- **The workday rule exists in two places, both in Rust.**
  - Pure functions in `src-tauri/src/work_time/norm.rs`: `is_weekend` :24, `workdays` :30, `day_norm_ms` :44, `figures` :104.
  - Store-backed classification in `src-tauri/src/work_time/report.rs`: `kind_of` :90, `day_kind` :100, `compute` :111, `set_day_off` :191 (weekend refused with `NotAWorkday` at :197-198), `is_workday` :217.
  - A holiday has to enter both. The cleanest seam is a new pure module (e.g. `work_time/holidays.rs`) whose result is passed into these functions next to `days_off`.
  - No new dependency is needed: `chrono` is already a dependency (`src-tauri/Cargo.toml:52`).
- **Five consumers depend on that rule, so a holiday reaches them all once the rule changes:**
  - the month report (`work_month_report`, `commands.rs:129`);
  - the header's month deviation (`status` calls `report::compute`, `mod.rs:188`);
  - the "Wybrany dzień" panel's day kind (`entries_for_day` calls `report::day_kind`, `mod.rs:239`);
  - the "not running after HH:MM" reminder (`reminder_facts` calls `report::day_kind`, `reminders.rs:69`, gated on `DayKind::Workday` at `reminders.rs:143`);
  - the dashboard's "Średnio / dzień" through `workdaysAfterToday` (`src/todo/summary.ts:67`).
- **Settings reach no work-time command today.** `run(app, name, f(store, now))` gets only the store and the clock (`commands.rs:20-43`).
  - The precedent for loading settings in a command is `todo/commands.rs:21-30`: `settings_path(&app)` then `load_todo_limit`, re-read on every call.
  - The background reminder round re-reads settings every minute (`work_reminders.rs:48-56`).
  - There is no `workTime` block yet. The closest template is the `reminders` block (`settings.rs:253-401`). It uses hand-written `Option<serde_json::Value>` matching where `false` switches a field off (:369-372, :383-396), `deny_unknown_fields`, and a fixed-text `SettingsInvalid` error.
- **Rust↔TS contracts that change** (Rust type and TS mirror in the same commit, per AGENTS.md):
  - `DayKind` (`report.rs:21-27` ↔ `src/worktime/types.ts:73`);
  - `DayReport` (`report.rs:45-57` ↔ `types.ts:77-85`), to carry the holiday's name and the "day off owed" hint;
  - possibly `WorkTimeError` (`error.rs:10-35` ↔ `types.ts:8-19`), for a holiday refusal and/or an invalid settings block.
- **Main risk.** Existing tests use 2026-11-11 (Narodowe Święto Niepodległości, a Wednesday) as the example manual day off. They also use 2026-12-24 (Wigilia), and a proptest marks random November weekdays off. These include:
  - the PRD-style worked example `norm_tests.rs:73-78`;
  - `norm_tests.rs:329-356`, `:386-397`, `:430-455`, `:570-575`;
  - `tests/work_time.rs:63-89`.
  - With A1 + A2 (on by default, switch blocked on holidays), `set_day_off(2026-11-11, true)` would be refused. These tests then need either an explicit "holidays off" parameter or new fixture dates. The figures of the worked example stay the same (20 workdays, 160 h) when 11.11 is a holiday instead of a marked day off.
- **Second risk.** An invalid `workTime` block must not break the header.
  - `WorkTimerHeader.tsx:52-58` replaces the counter and START/STOP with an error notice when `work_timer_status` fails.
  - If `status` returned a settings error, a typo in `settings.json` would hide START/STOP.
  - See Open Questions Q1.

## Detailed Findings

### Norm rule (pure, Rust)

- `DAILY_NORM_MS` is a fixed 8 h (`norm.rs:12`). S-02 (configurable norm, MS-26) changes it later (`roadmap.md:106-119`).
- `is_weekend` is Sat/Sun (`norm.rs:24-26`).
- `workdays(first, next_first, days_off)` keeps Monday–Friday days not in `days_off` (`norm.rs:30-40`).
- `day_norm_ms(day, days_off)` is 8 h on such a day, else 0 (`norm.rs:44-50`).
- `figures` derives these from the `workdays` slice (`norm.rs:104-143`): norm, norm-to-date, deviation, workdays after today, needed per day, remaining.
- A holiday can be added by passing a second set (or one merged "non-working" set) into `workdays` and `day_norm_ms`. `figures` needs no change because it only counts the slice.
- Worked numbers (computed with the Meeus/Jones/Butcher algorithm, checked against the issue's Easter dates 2026-04-05, 2027-03-28, 2028-04-16):
  - Easter 2025 is 2025-04-20. Easter Monday dates: 2025-04-21, 2026-04-06, 2027-03-29, 2028-04-17. Corpus Christi (Easter + 60): 2025-06-19, 2026-06-04, 2027-05-27, 2028-06-15.
  - Of the 12 norm-relevant dates (10 fixed + 2 movable), the number falling Monday–Friday is 10 in 2025, 8 in 2026, 8 in 2027 and 9 in 2028.
  - Saturday holidays (U1 hint cases): 2025-05-03 and 2025-11-01; 2026-08-15 and 2026-12-26; 2027-05-01 and 2027-12-25; 2028-01-01 and 2028-11-11.
  - Example: November 2026 has 21 Monday–Friday days (`norm_tests.rs:95-101`). With 11.11 as a holiday it has 20, a norm of 160 h, the same as today's worked example with 11.11 marked off (`norm_tests.rs:172-173`).
- Legal basis (domain knowledge, not repository evidence):
  - the Act of 18 January 1951 on days off work, with 24.12 added by the amendment in force from 1 Feb 2025;
  - art. 130 § 2 of the Labour Code (Kodeks pracy) for the day owed for a Saturday holiday (quoted in issue #25).
- 6.01 has been a holiday since 2011. The rule only needs to be right for years the app holds data in. The plan should still state the year range (e.g. 24.12 only for `year >= 2025`).

### Day classification, day-off toggle and weekend blocking (store-backed, Rust)

- `DayKind = Workday | DayOff | Weekend`, serialized camelCase (`report.rs:21-27`).
  - `kind_of` (in-memory set, used by the month report, `report.rs:90-98`) and `day_kind` (one SQL lookup, used by entries and reminders, `report.rs:100-108`) duplicate the same precedence: weekend first, then day off.
  - A holiday has to be inserted into both, at the same precedence for A2: weekend > holiday > dayOff > workday. A Saturday holiday stays `Weekend` and gets a hint field, per U1.
- `compute` (`report.rs:111-166`) reads `days_off` for the month in one `store.read` snapshot (:123). It then builds `workdays`, the entries, one `DayReport` per day (:132-152) and `MonthReport.days_off` (:160-163). It gets no settings.
- `set_day_off` refuses a weekend with `WorkTimeError::NotAWorkday` before any write (`report.rs:191-199`). Insert/delete is idempotent (:201-213).
  - For A2, the same guard extends to holidays. That needs the calendar in `set_day_off`, i.e. `commands.rs:138-146` has to load settings too.
  - The error text is "a weekend is not a workday" (`error.rs:31`). The TS text is "Weekend nie jest dniem roboczym" (`src/worktime/api.ts:141-142`). This needs either a new variant (e.g. `Holiday`) or a more general message.
- `is_workday` (`report.rs:217-222`) has no production caller in the inspected tree. Its callers found by grep over `src-tauri` are `norm_tests.rs:342-353` and `tests/work_time.rs:88-89`.
- Existing user data: before this slice, holidays could only be handled by marking them off, so the real `days_off` table may already hold rows on holiday dates.
  - Under A2 these rows are shadowed, not deleted: the holiday wins, and nothing is double-counted because the norm is 0 either way. If the calendar is switched off later, the rows show again as days off.
  - No migration is needed. `MonthReport.days_off` (`report.rs:39-40`) feeds the "Dni wolne · oznaczone ręcznie" stat (`src/worktime/MonthReport.tsx:14-19`). Whether shadowed rows still count there is Q4.

### Callers of the rule

These paths reach the rule. They were found by grepping `src-tauri` for `report::compute`, `report::day_kind`, `reminder_facts(` and `super::status(`.

| Path | Function | Settings available today? |
|---|---|---|
| Month report command | `commands.rs:129-135` → `report::month_report` → `compute` | no |
| Header status command | `commands.rs:51-57` → `mod.rs:167-202` (`compute` at :188) | no |
| Day entries command | `commands.rs:59-66` → `mod.rs:205-242` (`day_kind` at :239) | no |
| Set day off command | `commands.rs:137-146` → `report::set_day_off` | no |
| Reminder round (background thread) | `work_reminders.rs:48-77` → `reminders.rs:51-73` (`day_kind` at :69) | yes, `settings_path` is a parameter (:48) |

Integration tests call `work_time::status`, `entries_for_day`, `reminder_facts` and `report::*` directly (`tests/work_time.rs:51,131-207`), so their signatures are public API for the tests.

### Settings (`settings.json`)

- No single settings struct. Each block has its own loader that re-reads the file. `check_settings` validates only the file as a whole: exists, UTF-8 with BOM allowed, JSON, `version == 1` (`settings.rs:13,52-87`). Unknown top-level keys are ignored. Unknown keys inside a block are refused via `#[serde(deny_unknown_fields)]` (e.g. `RawReminders`, `settings.rs:289-295`).
- No `workTime` block exists (grep for `workTime` in `settings*.rs` found none). S-02 will add a daily-norm key to work-time settings as well (`roadmap.md:108`; issue #26), so the block should be one shared `RawWorkTime` struct that S-02 extends. Otherwise `deny_unknown_fields` on two separate parsers would refuse each other's keys.
- Template, `reminders` (`settings.rs:253-401`):
  - a domain struct with a manual `Default`;
  - `parse_*` that treats a missing or `null` block as defaults (:356-361) and refuses a non-object (:362-364);
  - per-field `None` as default, `Some(Bool(false))` as off, a valid value as set, anything else `SettingsInvalid` with no detail (serde errors can quote the value);
  - a loader where a missing file gives defaults and IO, UTF-8 or JSON errors give `SettingsInvalid` (:312-332).
- Proposed shape (agent default, fits the template):
  - `"workTime": { "holidays": "pl" }`;
  - absent or `null` means `"pl"` (A1); `false` means off;
  - any other value is invalid. That includes `true`, other strings and numbers, mirroring the reminders tests that refuse `true` (`settings.rs:1166-1205`).
- Test pattern to copy, in `settings.rs` `mod tests` from :611:
  - helpers `write`, `reminders_text`;
  - `reminders_defaults` :1111, `reminders_false_switches_each_off_separately` :1136, `reminders_invalid_values_are_refused` :1166-1205;
  - isolation tests: a broken block leaves the other loaders green (:1219, :1229);
  - a serialized-kind test at :1378.
- How an invalid block is surfaced today:
  - Reminders have their own health source "Przypomnienia" (`SourceId::Reminders`, `health/mod.rs:35,45`; `health/probe.rs:136-156`). Its fixed Polish text `REMINDERS_INVALID_PL` is kept equal to the TS copy in `src/worktime/api.ts:61-63` by a test.
  - The reminder round itself stops silently on an invalid block (`work_reminders.rs:50-53`, "Health surfaces an invalid block").
  - README states "An invalid block is reported in the **Źródła** view and does not affect the other blocks" (`README.md:120`).

### Frontend (`src/worktime/`)

- `DaysTable.tsx`:
  - `weekend = row.kind === "weekend"` (:40) drives muted text (:42), the bar (:45) and the switch, which is rendered only `!weekend` (:94-105).
  - Hatching is `row.kind === "dayOff"` (:52, `DataTableRow hatched`; CSS `src/components/DataTable/DataTable.css:25-27`).
  - There is no column for a name or hint. The day label cell (:53-62) or a new cell is where the holiday name and the U1 hint would go.
  - An archived decision says hatching means "marked by you" (`context/archive/2026-11-24-worktime-days-table/plan-brief.md:25`). A holiday "shown like a day off" therefore either reuses the hatch or needs a variant. This is a UI choice for the plan, using tokens only (`src/styles/tokens.css`).
- `useDayOffToggle.ts:26`: the stored state is `d.kind === "dayOff"`. With a `holiday` kind, a holiday row reads as "not off", which is fine because its switch is hidden. Stale pending requests settle on reload (:29-35).
- "Wybrany dzień" panel: `WorkTimeView.tsx:149-167` shows the "Dzień wolny" checkbox when `dayKind !== "weekend"`. `useWorkEntries.ts:137` reads `dayKind === "dayOff"`. Both gates need the holiday kind as well, or a holiday shows a switch that the backend refuses.
- `types.ts`: `DayKind` :73, `DayReport` :77-85, `MonthReport` :87-129 (discriminated on `phase`), `WorkTimeError` :8-19. `api.ts:50-53` has the `setDayOff` doc comment ("a weekend is refused with `NotAWorkday`") and `api.ts:141-142` the message.
- `MonthReport.tsx:14-19`: the "Dni wolne" stat counts `report.daysOff.length` with the sub-label "oznaczone ręcznie". Holidays are not manual, so either a separate "Święta" stat or no change (Q4).
- `src/todo/summary.ts:60-70` (dashboard) only displays `workdaysAfterToday` and `neededPerDayMs`, which follow the Rust figures with no TS change.
- The UI has no test runner (AGENTS.md), so TS changes are checked by `npm run build`, lint and a manual pass.

### Existing tests that touch the rule

- `src-tauri/src/work_time/norm_tests.rs`:
  - pure-rule tests :94-159;
  - the worked example with day off 2026-11-11 (:55-78, asserted at :163-195);
  - `set_day_off` idempotence and weekend refusal on 2026-11-11 (:328-356);
  - three day kinds with 11.11 as `DayOff` (:385-397);
  - day norm with 11.11 in the set (:405-413);
  - day rows add up, with 11.11 as `DayOff` (:429-455);
  - a future-December test asserting `norm_ms == 8 h` iff `kind == Workday` (:527-538). This stays true if holidays get their own kind.
  - a proptest that marks random weekdays off with `.unwrap()` (:541-584). It would panic on 11.11 if holidays are refused and on by default in that call.
- `src-tauri/tests/work_time.rs:63-89` marks 2026-11-11 and 2026-12-24 as days off across a store reopen. `:195-207` checks `reminder_facts` day kinds.
- `src-tauri/src/work_time/reminders_tests.rs` builds `ReminderFacts` with explicit `DayKind`s (:38-44, :185-187). A new variant needs one more "no reminder on a holiday" case there.
- `src-tauri/tests/days_off_crash.rs` uses the crash harness on `days_off` writes and is unaffected if no migration is added.
- All dates above are invented test data, not work data.

### PRD, roadmap and README text to touch

- PRD (`context/foundation/prd.md`):
  - FR-014 :155 ("dni robocze to poniedziałek–piątek minus dni oznaczone przez użytkownika jako wolne");
  - its Socrates line :156 ("…bez automatycznego kalendarza świąt");
  - the "Norma" paragraph :211;
  - the Non-Goal :226 ("**Brak automatyki czasu pracy.** Bez automatycznego kalendarza świąt, wykrywania bezczynności i mierzenia aktywności…").
  - Per U2, remove only the holiday-calendar clause at :226 and keep idle detection and activity measuring. The rule text at :155, :156 and :211 then has to mention holidays, otherwise the PRD contradicts itself.
  - FR-020 :176 describes the days table (switch, hatching). The PRD has no MS-25 entry: MS-NN anchors live in the roadmap (`roadmap.md:35`) and no FR is added (A3).
- Roadmap: S-01 is at `roadmap.md:91-105` and MS-25 at :35. It is not edited in this slice (orchestrator rule). Its three unknowns (:99-102) are answered by U1, U2 and A2.
- README:
  - the `## Configuration` section at `README.md:90-121`: JSON example :95-107, a table with one row per block :109-118. A `workTime` line and a row go here.
  - the work-time feature bullets at :52-58 ("8-hour workday norm; a table of the month's days and days off") need one line on holidays.

## Code References

- `src-tauri/src/work_time/norm.rs:12,24-50,104-143` — fixed norm, weekend, workdays, day norm, figures
- `src-tauri/src/work_time/report.rs:21-57` — `DayKind`, `MonthReport`, `DayReport` (contract types)
- `src-tauri/src/work_time/report.rs:90-108` — the two copies of the day classification
- `src-tauri/src/work_time/report.rs:111-166` — `compute` (one snapshot)
- `src-tauri/src/work_time/report.rs:190-222` — `set_day_off` weekend guard, `is_workday`
- `src-tauri/src/work_time/mod.rs:167-202,205-242` — header `status` (uses `compute`), `entries_for_day` (uses `day_kind`)
- `src-tauri/src/work_time/reminders.rs:51-73,143` — reminder facts and the workday gate
- `src-tauri/src/work_time/commands.rs:20-43,129-146` — `run` helper with no settings, report and set-day-off commands
- `src-tauri/src/work_time/error.rs:10-54` — `WorkTimeError` and `kind()`
- `src-tauri/src/settings.rs:253-401` — `reminders` block: the template for a `workTime` block
- `src-tauri/src/todo/commands.rs:21-30` — loading settings inside a command
- `src-tauri/src/work_reminders.rs:48-77` — reminder round, which already has `settings_path`
- `src-tauri/src/health/probe.rs:136-156`, `src-tauri/src/health/mod.rs:35,45` — per-block health source precedent
- `src/worktime/DaysTable.tsx:38-106` — row rendering, weekend gate, hatching, switch
- `src/worktime/WorkTimeView.tsx:149-167`, `src/worktime/useWorkEntries.ts:137` — the second day-off switch
- `src/worktime/useDayOffToggle.ts:24-27` — stored state from `kind === "dayOff"`
- `src/worktime/types.ts:8-19,73-85` — TS mirrors to change
- `src/worktime/api.ts:50-53,141-142` — set-day-off doc comment and refusal text
- `src/worktime/MonthReport.tsx:14-19` — "Dni wolne" stat
- `src/components/DataTable/DataTable.tsx:30-44`, `DataTable.css:25-27` — the `hatched` row
- `src/worktime/WorkTimerHeader.tsx:48-58` — header error state replaces START/STOP

## Architecture Insights

- **Domain rule in Rust, UI displays.** AGENTS.md puts the work-time norm in Rust with unit tests. The U1 hint ("a day off is owed for this Saturday holiday") is a domain fact too. It belongs in `DayReport` (e.g. `holiday: Option<String>` for the name plus `dayOffOwed: bool`), not computed in TS from the weekday.
- **Pure core, no clock, no settings inside.** `norm.rs` and `report.rs` take dates and connections only. `commands.rs` is the single `tauri` importer (`commands.rs:1-2`). Settings are loaded at the edge (commands, reminder round) and passed in as a small value, e.g. `enum HolidayCalendar { Off, Pl }`. That keeps tests explicit and lets existing tests pass `Off` where they mean "a plain marked day off".
- **One snapshot per report.** `compute` reads everything in one `store.read`. Holidays are computed, not stored, so they add no read and no migration.
- **Per-block settings isolation.** A broken block never breaks other blocks (`settings.rs` tests :1219, :1229; README :120). A broken `workTime` block must not break START/STOP either (Q1).
- **Parallel slices.** S-03 (tray-timer-control) and S-06 (work-entry-note) run in other worktrees and touch `lib.rs`, `settings.rs`, `WorkTimeView.tsx` and README. Edits there should stay small and local: a new loader function and a test block appended in `settings.rs`, one README row and JSON line, and the switch-gate change in `WorkTimeView.tsx`. Most of the work can live in new files (`work_time/holidays.rs` plus its tests) and in `norm.rs`, `report.rs` and `DaysTable.tsx`. Registering a new command in `lib.rs` is not needed if holidays ride on the existing `MonthReport`/`DayEntries`. A new health source would touch `health/` (and the tray's source list, which S-03 also touches).

## Historical Context (from prior changes)

- `context/archive/2026-11-23-work-norm-report/plan.md:45` (also `plan-brief.md:48`): "No holiday calendar…" was out of scope because of the PRD Non-Goal. **Superseded** by U2 for the holiday part only.
- Same plan, still valid:
  - `plan.md:44,98-99,131`: workdays are Mon–Fri minus `days_off`, and weekends are never workdays, even when listed.
  - `plan.md:116,197`: `set_day_off` refuses a weekend with `NotAWorkday` and writes no row.
  - `plan.md:19,334`: reminders must reuse the same workday rule, not a second one. This applies to holidays too.
- `context/archive/2026-11-24-worktime-days-table/plan-brief.md:24`: the day norm comes from Rust (`norm::day_norm_ms`) and the UI does not know the 8 h. Still valid.
- Same archive, `plan-brief.md:25`: days off are hatched and weekends muted with no switch, because "Kreskowanie znaczy 'oznaczone przez ciebie'". Still valid, but it is a constraint on how a holiday is drawn (Q3). Also `plan.md:37,224` and the `--hatch-bg` token at `plan.md:171`.
- Same archive, `plan-brief.md:39` (out-of-scope list): "konfigurowalna norma, kalendarz świąt". The holiday calendar is now in scope here, the configurable norm in S-02.
- Same archive, `plan-brief.md:7`: the user "oznacza urlop albo święto prosto z tabeli", which confirms that real data may already hold manual days off on holiday dates (see the shadowed-rows note above).
- `context/archive/2026-11-23-work-timer/plan.md:43`: holiday calendars out of scope (PRD Non-Goals). Superseded for holidays by U2.

## Related Research

- `context/archive/2026-11-23-work-norm-report/` and `context/archive/2026-11-24-worktime-days-table/`: the plans that built the norm and the days table.
- No other research on holidays exists. A grep for "święt" or "holiday" over `context/archive` matched only the files listed above, plus one unrelated hit in `2026-11-23-new-assignment-notify/plan.md:59` that was not checked.

## Open Questions

Agent defaults are proposed where the run had no user to ask. The plan should confirm them.

- **Q1. What happens with an invalid `workTime` block?**
  - Options:
    - (a) The month report fails with a fixed-text settings error and the days panel shows `ErrorState`. The header `status` must not fail; it computes its month deviation with the default calendar while the error is shown elsewhere.
    - (b) Everything falls back to the default calendar and only a health source shows the error.
    - (c) Everything fails, header included.
  - (c) hides START/STOP (`WorkTimerHeader.tsx:52-58`). (b) silently shows numbers the user may have meant to change.
  - **Agent default:** (a). Add the error to Źródła the same way reminders do (`health/probe.rs:136-156`) if a new health source fits the S-03 parallel work. Otherwise only the report error.
- **Q2. Holiday refusal error.**
  - Options: a new `WorkTimeError::Holiday` variant ("Święto nie jest dniem roboczym"), or reuse `NotAWorkday` with a general message.
  - **Agent default:** a new variant, since the TS message names the reason, with its TS mirror in the same commit.
- **Q3. How to draw a holiday.**
  - It must be "like a day off" (issue #25), but hatching means "marked by you" (archived decision).
  - **Agent default:** the same hatch, no switch, and the holiday name in the row. Do not invent a new colour token unless the design system needs one.
  - The Saturday hint (U1) is a short note on that row, e.g. "za święto przysługuje dzień wolny".
  - Should Sunday holidays (Wielkanoc, Zielone Świątki, or 1.11 or 3.05 when they fall on a Sunday) show their name? They change nothing. **Agent default:** show the name on any holiday row, with the hint only on Saturdays.
- **Q4. "Dni wolne · oznaczone ręcznie" stat.**
  - Count only manual days off that are not shadowed by a holiday (agent default), and optionally add an "Święta" count.
  - Should `MonthReport.days_off` keep listing shadowed rows? **Agent default:** no, filter them out, so the stat means what its label says.
- **Q5. Fixtures.**
  - Options: switch the existing 2026-11-11 and 2026-12-24 day-off fixtures to non-holiday weekdays, or pass `HolidayCalendar::Off` in those tests.
  - **Agent default:** pass `Off` in the tests that are about manual days off (minimal diff). Add new tests with `Pl` on known dates: Easter 2026-04-05, 2027-03-28, 2028-04-16, and the November 2026 norm of 160 h with no marked day.
- **Q6. Settings value domain.**
  - Is `"pl"` the only accepted string, and is `true` refused? **Agent default:** yes to both, as in the reminders precedent.
  - The block must be shared with S-02's daily-norm key (Q-coordination with S-02, `roadmap.md:106-119`).
- **Q7. Show the holiday name in the "Wybrany dzień" panel too?** `DayEntries` carries only `day_kind` (`mod.rs:82`). **Agent default:** gate the switch on the new kind and leave the name to the days table, to keep the contract change small.
