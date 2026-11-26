# Polish public holidays in the work-time norm — Plan Brief

> Full plan: `context/changes/public-holidays-norm/plan.md`
> Research: `context/changes/public-holidays-norm/research.md`

## What & Why

Today every public holiday has to be marked as a day off by hand, or the month's norm and deviation are wrong. Roadmap S-01 (issue #25, MS-25) computes the Polish public holidays locally:

- the fixed dates, with 24.12 from 2025;
- Easter Monday and Corpus Christi, from Easter (Meeus/Jones/Butcher).

The holidays are counted out of the norm and shown in the days table like a day off, with their names. A `settings.json` key turns the calendar off.

## Starting Point

The workday rule is Monday–Friday minus manual `days_off` rows, written twice in Rust:

- the pure `norm.rs`;
- the store-backed `report.rs`. Its `set_day_off` refuses weekends.

Five consumers depend on it: the report, the header deviation, the day panel, the reminder and the dashboard. No work-time command reads `settings.json` yet, and there is no `workTime` block.

## Desired End State

- November 2026 has a 160 h norm with nothing marked. 11.11 is hatched as "Święto Niepodległości" with no switch.
- December 2026 has 168 h:
  - 24.12 and 25.12 are hatched holidays;
  - 26.12 (a Saturday) shows its name and the hint "należy się inny dzień wolny".
- `"workTime": { "holidays": false }` restores today's behaviour.
- An invalid value shows an error in "Czas pracy" while START/STOP keeps working.
- The PRD and the README describe the calendar.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Saturday holiday | Exchange day stays manual; the row shows a hint (`dayOffOwed`) | The employer sets the date (Labour Code art. 130 § 2), so the app cannot guess it | User (U1) |
| PRD Non-Goal | Drop only "automatycznego kalendarza świąt"; align FR-014, Socrates, "Norma", FR-020 | Otherwise the PRD contradicts itself | User (U2) |
| Default | On (`"pl"`) when the block or key is absent | The calendar is the point of the slice | Agent default (A1) |
| Holiday + marked day off | Holiday wins; the switch is hidden and `set_day_off` refuses with `Holiday` | Same model as weekends; old rows are shadowed, not deleted | Agent default (A2, Q2) |
| Anchor | MS-25, no new FR | Roadmap already anchors it | Agent default (A3) |
| Invalid block | Report and set-day-off refuse with `SettingsInvalid`; status, entries and reminders fall back to defaults; no health source | A settings typo must never hide START/STOP; S-03 owns the tray and source list | Agent default (Q1) |
| Drawing | Same hatch, no switch, name in the "Na tle normy" cell; Sunday holidays show the name only | Issue asks "jak dzień wolny, z nazwą"; no new token | Agent default (Q3) |
| "Dni wolne" stat | Counts only manual rows not shadowed; no "Święta" tile | The label "oznaczone ręcznie" stays true | Agent default (Q4) |
| Settings shape | `"pl"` or `false` only; one shared `WorkTimeSettings` that S-02 extends | Reminders precedent; avoids two `deny_unknown_fields` parsers | Agent default (Q6, D2) |
| Code placement | New `holidays.rs` and `settings/work_time.rs`; `settings.rs` +2 lines; no `lib.rs` change | S-03 and S-06 edit the same files in parallel | Agent default (D3) |

## Scope

**In scope:**

- the pure holiday calendar and its tests;
- the `workTime` loader;
- the rule in norm, kinds, guard and reminders;
- the Rust↔TS contract (`DayKind.holiday`, `DayReport.holiday` and `dayOffOwed`, `Holiday` and `SettingsInvalid` errors);
- the days-table and day-panel UI;
- the "Jak liczę" text;
- PRD and README.

**Out of scope:**

- an automatic day owed for a Saturday holiday;
- other calendars;
- the holiday name in "Wybrany dzień";
- an "Święta" stat;
- a health source for the block;
- any migration;
- the configurable norm (S-02);
- roadmap edits.

## Architecture / Approach

`work_time::holidays::HolidayCalendar { Off, Pl }` answers `holiday(day) -> Option<&'static str>`. The pure norm functions take it. The store-backed functions (`compute`, `set_day_off`, `status`, `entries_for_day`, `reminder_facts`, `is_workday`) take `&WorkTimeSettings`, so S-02 adds its norm field without new signatures.

The precedence is weekend > holiday > dayOff > workday, in one function. Commands read `settings.json` per call, inside the blocking task and before the store lock:

- strict for the report and set-day-off;
- lenient for status and entries.

The reminder round reads it leniently every minute.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Holiday calendar | Pure `holidays.rs` + Easter tests on known years | Algorithm or year gates wrong |
| 2. `workTime` block | Loader in `settings/work_time.rs`, tests, isolation | Diverging from the reminders conventions |
| 3. Rule + contract | Norm, kinds, guard, reminders gate, TS mirror; tests on 160 h / 168 h | ~50 test call sites; fixtures on 11.11 and 24.12 must use "holidays off" |
| 4. Settings at the edges | Strict/lenient policy, `SettingsInvalid`, reminder round | A typo hiding START/STOP |
| 5. UI | Hatch, name, Saturday hint, switch gates, "Jak liczę" | Long names in a fixed grid |
| 6. Docs | PRD lines 155/156/176/211/226, README key | Overlap with parallel README edits |

**Prerequisites:** none. S-01 has no prerequisites, and every layer it needs exists.
**Estimated effort:** ~6 short sessions, one phase per fresh sub-agent. Phase 3 is the largest, mostly mechanical test updates.

## Open Risks & Assumptions

- S-03 and S-06 code that calls `status`, `entries_for_day` or `reminder_facts` gets a new `&WorkTimeSettings` parameter. Whichever slice merges second fixes that call site (a compile error, not a silent bug).
- Real data may already hold manual rows on holiday dates. They are shadowed under `"pl"` and come back with `false`. This is assumed acceptable, with no cleanup.
- Without a health source, an invalid `workTime` block is visible only in "Czas pracy". The README notes this exception.
- The legal basis (the 1951 Act with the 2025 amendment, and the Labour Code art. 130 § 2) is domain knowledge, not repository evidence.

## Success Criteria (Summary)

- Months with holidays show the right norm (Nov 2026: 160 h, Dec 2026: 168 h) with nothing marked, and the holidays are named in the table.
- The switch and the backend both refuse marking a holiday. A Saturday holiday shows that a day off is owed.
- `false` restores the old behaviour, and an invalid value never hides START/STOP.
