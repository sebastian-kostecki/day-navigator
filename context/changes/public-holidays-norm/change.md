---
change_id: public-holidays-norm
title: Polish public holidays counted out of the work-time norm
status: preparing
created: 2026-11-26
updated: 2026-11-26
archived_at: null
---

## Notes

Roadmap S-01 (milestone M-3), GitHub issue #25, anchored to PRD MS-25 (no new FRs).

Outcome: the user sees Polish public holidays (fixed dates incl. 24.12 Wigilia as a holiday from 2025, plus Easter Monday and Corpus Christi computed from Easter via the Meeus/Jones/Butcher algorithm) counted out of the month's work-time norm and shown in the days table like a day off, with the holiday's name. A `settings.json` key (e.g. `"workTime": { "holidays": "pl" }`, `false` turns it off) controls it; README documented.

User decisions: (1) a holiday on a Saturday: the day off in exchange stays a manual day off, but the days table shows a hint next to the Saturday holiday that a day off is owed for it. (2) The PRD Non-Goal "no automatic holiday calendar" changes in this slice (prd.md updated accordingly, the rest of that Non-Goal stays).

Agent defaults (user may revise): holidays on by default; a day that is both a holiday and a marked day off → the holiday wins and the day-off switch is blocked on holidays like on weekends; keep the slice anchored as MS-25.
