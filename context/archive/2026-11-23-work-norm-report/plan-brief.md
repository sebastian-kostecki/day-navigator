# Work-Norm Report — Plan Brief

> Full plan: `context/changes/work-norm-report/plan.md`

## What & Why

Roadmap slice S-09 (PRD FR-014): show how far the month is from the work norm of 8 h × workdays. Workdays are Monday–Friday minus days the user marks as off. The PRD's primary success criterion is that "at the end of every day the user knows how much is missing to the norm", so they can decide whether to work longer or shorter.

## Starting Point

S-08 (work-timer) is done. Entries live in the SQLite store as UTC instants. The tauri-free `work_time` module computes day totals split at local midnight. The header shows "Dziś H:MM:SS" with START/STOP. "Czas pracy" lists a chosen day's entries. There are no days off, no month sums and no norm anywhere yet.

## Desired End State

"Czas pracy" opens with a month report for the chosen day's month:
- worked total and norm;
- signed deviation to date, with today counted in full;
- the average needed per workday after today, the remaining shortfall on the last workday, or "Norma miesiąca wyrobiona";
- the month's days off.

Past months show their final deviation, and future months only the norm. A "Dzień wolny" checkbox on the chosen weekday marks days off. The header shows "Miesiąc −4:00" next to "Dziś", moving live while the counter runs.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Today in the deviation | Today's 8 h counts in full once today is a workday | The end-of-day reading is exact, which is what the success criterion needs. |
| "Needed per remaining day" | `(norm − worked) / workdays after today`, rounded up to the minute; shortfall on the last workday | Consistent with "today counts": at day end it is the plan for the rest of the month. |
| Placement | Report at the top of "Czas pracy", plus compact deviation in the header | Always visible without a click, with one screen for all time data. |
| Past months | The report follows the chosen day's month; past = final, future = norm only | Last month's settlement number is still there on the 1st. |
| Marking days off | "Dzień wolny" checkbox on the chosen weekday (hidden on weekends) | Reuses the ◀ ▶ day picker; no new calendar component. |
| Days-off storage | `days_off` table (migration 3) in the app store; un-marking deletes the row | App data belongs in the store, and S-10 reuses it. |
| Live values | Header extrapolates the deviation per second; report refetches every 60 s while it shows the current month, so midnight is caught too | The rules stay in Rust; the UI only adds elapsed running time, as it does for "Dziś". |
| Norm hours | Fixed 8 h, whole days off only | The PRD fixes it; configurability is unrequested scope. |

## Scope

**In scope:**
- Migration 3 `days_off`
- pure norm rules (`work_time/norm.rs`) and store-backed report and toggle (`work_time/report.rs`)
- `WorkTimerStatus` with the month deviation, and `DayEntries.dayKind`
- commands `work_month_report` and `set_day_off`, with TS mirrors
- report section, day-off checkbox and header deviation
- unit, reopen and crash tests

**Out of scope:**
- configurable norm, half days off, working Saturdays
- holiday calendar, vacation ranges, calendar grid, separate "Raport" view
- reminders (S-10), export or `Timeline.md`, old records

## Architecture / Approach

`norm.rs` holds pure calendar and norm math on `NaiveDate`s and milliseconds. `report.rs` reads entries clipped to the local month bounds, which handles DST through the existing `day_bounds`/`portion`, together with the days off in one `store.read`, then calls `norm.rs`. `status()` computes the current month's deviation in its existing read. Toggling a day off writes one row and emits the existing `work-time-changed`, so the header, the day list and the report refetch together.

**Worked example (invented):** November 2026, 2026-11-11 off, now Monday 2026-11-23 12:00, worked 116:00. That gives 20 workdays and a norm of 160:00. Norm to date is 120:00, so the deviation is −4:00. With 5 workdays after today, the needed figure is 8:48 per day.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Norm domain and days-off store | Migration 3, norm rules, month report, header deviation, toggle; unit, reopen and crash tests | Off-by-one on "today" or the month edges (DST, entries across months) |
| 2. Commands and TS contract | `work_month_report`, `set_day_off`, extended types and mirrors, Polish error text | Rust and TS mirrors drifting apart (same commit) |
| 3. Report UI and header deviation | Report section, "Dzień wolny" checkbox, "Miesiąc ±H:MM" in the header | Header extrapolation crossing midnight; stale figures while hidden |

**Prerequisites:** S-08 work-timer merged (done); `chrono-tz` dev-dependency already present.
**Estimated effort:** ~2–3 sessions across 3 phases.

## Open Risks & Assumptions

- The month starts and ends at local midnight in the system time zone, which is the same assumption as the day totals.
- A morning view of the deviation shows a large minus by design ("today counts in full"). The needed figure is the forward-looking number.
- S-10 is expected to reuse `report::is_workday` and `days_off`. Building a second workday rule there would drift.

## Success Criteria (Summary)

- At the end of any workday, the header and the report show the exact month deviation, including today.
- Marking a holiday or vacation day immediately changes the norm, the deviation and the needed per-day figure.
- Last month's final deviation is readable after the month ends. A store failure never reads as a zero report.
