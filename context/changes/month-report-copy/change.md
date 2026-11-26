---
change_id: month-report-copy
title: Copy the month work-time report to the clipboard as TSV
status: planned
created: 2026-11-26
updated: 2026-11-26
archived_at: null
---

## Notes

Roadmap slice S-05 (milestone M-3), GitHub issue #24, anchored as MS-24 (no new FRs in the PRD).

Outcome: in the "Czas pracy" view a "Kopiuj miesiąc" button copies the month report (days, entries with their notes, sums, deviation, days off and holidays with names) to the clipboard as text that pastes into a spreadsheet.

Decisions already made by the user: TSV (tab-separated); clipboard only, no file; the text is built in Rust from the same month report the view shows and unit-tested on invented data; the UI only triggers the copy and shows a short confirmation or the error; include the per-entry note and the holiday name.

Open for agent defaults (recorded in the plan as "default chosen by agent, user may revise"): exact rows/columns, time format, escaping of tabs/newlines in notes, how the clipboard is written (webview vs `tauri-plugin-clipboard-manager`, least surface per AGENTS.md).
