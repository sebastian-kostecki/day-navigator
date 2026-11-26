---
change_id: week-summary
title: Week summary in "Czas pracy" against the week's norm
status: planned
created: 2026-11-26
updated: 2026-11-26
archived_at: null
---

## Notes

Roadmap S-08 (milestone M-3), GitHub issue #18, anchored to PRD MS-18 (no new FRs).

Outcome: the user sees in "Czas pracy" the current week's sum against the week's norm, day by day.

User decisions: (1) a section (panel) in the "Czas pracy" view, not a separate view or navigation item. (2) The week is Monday–Sunday and always complete, also across a month boundary (e.g. Mon 30.11 – Sun 6.12). (3) The week's norm uses exactly the same rule as the month (holidays from S-01, manual days off), computed in Rust and unit-tested; the UI only displays, never sums. (4) Anchor as MS-18; do not add new FRs to the PRD.

Open (to be decided as agent defaults in `/10x-plan`): current week only vs prev/next navigation; exact columns (worked, norm, deviation per day + week total); how the "so far" vs "whole week" norm is shown.

Merge risk: S-02 configurable-daily-norm changes norm functions in `norm.rs` in parallel, so the week norm goes only through existing norm/day-classification functions, never a hard-coded 8 h or `DAILY_NORM_MS` in new code.
