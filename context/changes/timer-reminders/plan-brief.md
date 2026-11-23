# Timer Reminders — Plan Brief

> Full plan: `context/changes/timer-reminders/plan.md`

## What & Why

Roadmap slice S-10 (PRD FR-018) adds two reminders. The first fires when the work counter has been running too long ("czy nadal pracujesz?"). The second fires when, on a workday after a set hour, no work has been recorded yet. A forgotten STOP or START is exactly what broke the old time record. Without reminders, the norm report (S-09) trusts wrong entries until someone corrects them by hand.

## Starting Point

S-08 and S-09 are done:
- Entries live in the SQLite store as UTC instants.
- The tauri-free `work_time` module takes `now` and the time zone as parameters.
- `report::is_workday` handles Monday–Friday minus days off.

The app already sends Polish Windows toasts from Rust, through one private helper gated by `notices_allowed()`. Background threads re-read their own `settings.json` block every round. The window hides to the tray, so the app runs for days.

## Desired End State

- **Long run:** when the running entry reaches 10 h, a toast "czy nadal pracujesz?" appears, then one more every full hour while the same entry keeps running.
- **Not running:** on a workday from 09:00, if nothing has been recorded today, a toast "licznik nie biegnie" appears, then one every hour until START.
- **Settings:** both values can be changed or turned off in `settings.json`.
- **Invalid settings:** an invalid block shows up as "Przypomnienia" in error in the tray. It never silently stops the reminders.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| What "biegnie długo" measures | Time since the running entry's start | Targets the forgotten STOP exactly, including entries left running overnight or across a restart. |
| Repeating the long-run reminder | Every full hour while the same entry runs | A missed toast, or one hidden by Focus Assist, gets another chance. |
| What "nie biegnie" means | Stopped, and today has no counted work yet | Targets the forgotten morning START; normal breaks never trigger it. |
| Repeating the not-running reminder | Every full hour until START or the end of the day | One missed toast should not leave a whole day unrecorded. |
| Channel | Windows toast only | Smallest change; when the window is open, the counter is already visible. |
| Invalid `reminders` block | "Przypomnienia" health source in error, reminders paused | Follows the rule to surface failures, never degrade silently. |
| Switching off | Each field accepts `false` | You can silence one reminder without losing the other. |
| Defaults | 10 h and 09:00 | Already decided (Open Roadmap Question #3). |
| Duplicate tracking | Hourly slots keyed by their start instant, remembered in memory | A pure comparison: no repeat within a slot, no backlog after sleep, at most one repeat after a restart, and correct after editing the running entry's start or changing settings. |

## Scope

**In scope:**
- The `reminders` settings block, with defaults, `false` to turn off, and value-free `SettingsInvalid`.
- `work_time::reminders`: the facts query, the slot rules and the Polish text.
- A 60 s `work-reminders` thread.
- The toast helper, shared with health.
- The "Przypomnienia" health source.
- Unit, integration and health tests.

**Out of scope:**
- A settings screen.
- Toast clicks or actions.
- Tray START/STOP items, a tray tooltip line, a header hint.
- An overwork reminder on today's total.
- A reminder after a break.
- Persisting what was sent, or catching up missed slots.
- Autostart, pause modes, TS changes.

## Architecture / Approach

The rules layer is pure and tauri-free: `ReminderFacts` (running entry, today, day kind, whether today has work) plus settings, `now`, the time zone and `ReminderMemory` go into `due_reminders`, which returns the toasts and the updated memory. A new thread runs every 60 s. Each round it reads settings and facts (the facts with the store lock held only for the read), releases the lock, then sends through `runtime::toast`. Health probes the same settings block and adds the "Przypomnienia" source to the existing tray and panel flow.

**Worked example (invented):** Monday 2026-11-23, entries 08:00–12:00 and 13:00–(running).
- At 19:00 the entry has run 6 h: no toast.
- At 23:00 the first toast fires: "od 13:00, 10 h".
- At 00:00 the second fires: "od wczoraj 13:00, 11 h".
- On Tuesday nothing about "nie biegnie" fires once work from 00:00 to 00:30 is recorded.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Reminder rules and settings (tauri-free) | Settings loader, facts query, slot rules, Polish text; unit and integration tests | Off-by-one slots, midnight, a DST gap at the anchor time |
| 2. Reminder thread, shared toast and health source | 60 s thread, shared gated toast, "Przypomnienia" source, wiring in `lib.rs`; manual checks | Holding the store lock across a toast; the store not open yet on an early round |

**Prerequisites:** S-08 work-timer and S-09 work-norm-report (done).
**Estimated effort:** ~1–2 sessions across 2 phases.

## Open Risks & Assumptions

- Reminders fire only while the app runs, since there is no autostart. A Windows restart without opening the app means no reminders.
- Toasts can be swallowed by Focus Assist, and `show()` reports success before Windows displays anything. The hourly repeat is the mitigation; the tray shows only configuration errors.
- On a sick day not marked as off, the not-running toast repeats hourly until the day is marked "Dzień wolny". This is accepted by design.
- A failed reminder thread (spawn failure or panic) is only logged, not shown in the tray. This is accepted as rare.
- A whole-file `settings.json` error shows both "Ustawienia" and "Przypomnienia" in error. They are folded into one toast.

## Success Criteria (Summary)

- A forgotten STOP gets a "czy nadal pracujesz?" toast at 10 h and every hour after, even with the window hidden in the tray.
- A forgotten morning START on a workday gets a toast from 09:00 every hour until START, and stays silent on weekends, days off and once work is recorded.
- A broken `reminders` setting is visible in the tray and the panel, and is never silently ignored.
