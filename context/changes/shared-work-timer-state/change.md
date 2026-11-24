---
change_id: shared-work-timer-state
title: Wspólny stan licznika czasu pracy w UI
status: implemented
created: 2026-11-24
updated: 2026-11-24
---

## Notes

Roadmap F-02 (M-2), GitHub issue #9, kotwica MS-09. Nagłówek, dashboard „Do zrobienia” i widok „Czas pracy” mają dziś każdy własną subskrypcję `useWorkTimer` (osobny listener `work-time-changed`, odczyty przy focusie, własny tick co sekundę). Cel: jeden wspólny stan licznika w UI (jeden listener, jeden tick), bez zmiany zachowania i bez zmian w komendach Rust. START/STOP i potwierdzenie działają dokładnie jak dziś. Odblokowuje S-05 (`worktime-days-table`), którego wiersz „dziś” czyta wspólny stan.
