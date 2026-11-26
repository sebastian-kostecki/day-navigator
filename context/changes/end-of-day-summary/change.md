---
change_id: end-of-day-summary
title: End-of-day summary after STOP
status: plan_reviewed
created: 2026-11-26
updated: 2026-11-26
archived_at: null
---

## Notes

Roadmap S-07 (M-3), GitHub issue #19. After STOP the user sees a summary of the day: time worked today, the month's deviation, what was ticked off in "Do zrobienia" and what carries over to tomorrow, with no decision forced.

User decisions (fixed, 2026-11-26):

- Shown after STOP only (no fixed-hour trigger).
- STOP in the window (header): a dismissible summary in the window; closing it needs nothing else; unticked items still carry over automatically as today.
- STOP from the tray: a short Windows notification with today's work time and the month's deviation (fixed wording, no item titles; follow existing notification conventions).
- Summary data comes from existing sources (today's entries, the month report, the "Do zrobienia" list, today's event log), computed in Rust and unit-tested; no second store of the day's state, no new migration.
- Anchor as MS-19; no new FRs in the PRD (if the PRD lists end-of-day close as considered-but-excluded, update that wording in the plan).
- Open for agent defaults (decided in plan): where/how the in-window summary appears, whether it can be reopened later today, tray vs header STOP behaviour, item titles in the UI (allowed in UI, never in app log).
