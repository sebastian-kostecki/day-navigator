# Week summary in "Czas pracy" — Plan Brief

> Full plan: `context/changes/week-summary/plan.md`
> Research: `context/changes/week-summary/research.md`

## What & Why

Roadmap S-08 (issue #18, MS-18, reuses the FR-014 rule; no new FR): in "Czas pracy" the user sees the current week's work against the week's norm, day by day. The month view answers "where am I this month"; the week panel answers "where am I this week", including the weeks that straddle two months.

## Starting Point

The month report (`report::compute`) builds one row per day (worked, norm, deviation, kind, holiday) inline and reads exactly one month in one store snapshot. A week across a month therefore cannot be cut out of one month report, and there is no week command or UI today.

## Desired End State

Between "Wybrany dzień" and "Dni miesiąca" a panel "Bieżący tydzień" shows the dates of the current Monday–Sunday week, a stat strip (worked "z H:MM normy", deviation up to today, remaining), and seven rows with a bar against the day norm, the kind of non-working day, worked, norm and ±. Today's row runs live with the counter. Loading and error states (with last-success time and retry) are as in the month report.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Placement | A panel in "Czas pracy", no new view | User asked for a section | User (D1) |
| Week shape | Mon–Sun, always 7 days, across month/year | Matches how a work week is felt | User (D2) |
| Norm rule | The month's own per-day code, in Rust, unit-tested | "Exactly the same rule"; UI only displays | User (D3) |
| PRD | MS-18, no new FR; only "widok tygodnia" leaves the "considered but not built" sentence | Scope stays the roadmap slice; the PRD stays true, as S-04/S-07 do | User (D4) / Plan review (A14) |
| Which week | Current week only, no navigation | Roadmap says "current week"; command takes a day for later | Plan (agent default A1) |
| Reuse | Extract `report::day_report`, shared by month and week | One code path, so S-02's norm change reaches the week | Research / Plan (A2) |
| Totals | Sums of the day rows; never `norm::figures` / 8 h | `figures` is not half-day safe (S-02) | Research / Plan (A3) |
| Figures | Worked, whole-week norm, norm to date, deviation to date, remaining | Mirrors the month's current phase | Plan (A4) |
| Policy | `work_week_report(day)`, `Policy::Strict` | Month and week never disagree on the calendar | Research / Plan (A6) |
| Live today | Only today's row live (shared `liveFigures`) | Same as the days table; UI never re-sums | Plan (A8) |
| Midnight | Hook sends the local date at each request, 60 s poll | The view does not re-render at midnight while stopped | Plan (A9) |
| Tests vs S-02 | Settings via defaults / `manual_only()`; norms as multiples of a workday row; cross-check vs month rows; days off only via one `mark_off` helper; `week.rs` never names the days-off type | Survive S-02's new settings field, configurable norm, `DaysOff` map and `DayMark` | Plan (A11) / Plan review |
| Kind label | Holiday name whenever the row has one, else exhaustive `DayKind` switch | A Saturday holiday is `weekend` with `dayOffOwed`; S-02's `halfDayOff` fails the build | Plan review (A15) |

## Scope

**In scope:**
- `report.rs`: mechanical extraction of `day_report`, `days_off_between` made `pub(super)`
- New `work_time/week.rs` + `week_tests.rs` (calendar cases, DST, running timer, midnight, proptest)
- `work_week_report` command, `lib.rs` line, TS mirror and `workWeekReport`
- `useWeekReport.ts`, `WeekSummary.tsx`, CSS rules, two lines in `WorkTimeView.tsx`
- One README and one CHANGELOG line; one PRD sentence (drop "widok tygodnia" from "considered but not built", add a built-as line, no FR)

**Out of scope:**
- Week navigation, week of the selected day, separate view
- Changes to `norm.rs`, month figures, `HowCounted`, PRD FRs, roadmap
- "Średnio / dzień" for the week, day-off switch or entry counts in week rows
- Migration, new tokens

## Architecture / Approach

`week_report(store, day, now, tz, settings)` does one `store.read` over `[monday, next_monday)`: days off via `days_off_between`, entries via `in_day`, then `report::day_report` for each of the seven days; totals are sums of those rows, the phase compares Mondays. The command wraps it like the month report. The panel reads it through its own hook and the shared timer context, and displays only.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Shared day row | `day_report` used by the month; no behaviour change | Textual conflict with S-02 in the same loop |
| 2. Week report domain | `week.rs` + tests on invented weeks | Test expectations tied to 8 h (avoided by A11) |
| 3. Command and contract | `work_week_report`, TS mirror, API function | Rust/TS drift (same commit) |
| 4. "Bieżący tydzień" panel | Panel, hook, CSS, README/CHANGELOG | Visual fit at 800 px; live today vs totals lag |

**Prerequisites:** S-01 merged (it is on `develop`); worktree `slice/week-summary`.
**Estimated effort:** ~2 sessions across 4 phases (two small Rust phases, one wiring phase, one UI phase).

## Open Risks & Assumptions

- S-02 edits the same per-day loop in `report.rs` and changes `days_off_between` (→ `DaysOff` map), `classify`, `day_norm_ms` and `set_day_off` (→ `DayMark`): expect a merge conflict; resolve by moving S-02's change into `day_report`. S-02 adds `DayKind::HalfDayOff`, so the exhaustive switch in `WeekSummary` fails `npm run build` until labelled "pół dnia wolnego".
- The PRD edit (A14) is an agent default from the plan review; the user may revise it (drop it and leave the PRD sentence as is).
- The week totals lag today's live row by up to 60 s while the counter runs (accepted, as in the month tiles).
- After Sunday→Monday midnight the panel shows last week until the next poll, focus or write (≤ 60 s).
- Manual checks need the real app; they stay unchecked until the user runs them.

## Success Criteria (Summary)

- The user sees the current week's seven days and totals against the week's norm, with the same norms and deviations as the month table for the same days.
- Weeks across a month or year, with holidays, DST or a running counter show correct figures (pinned by Rust tests).
- A failed load shows an error with retry, never an empty week.
