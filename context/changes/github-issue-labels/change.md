---
change_id: github-issue-labels
title: Etykiety przy zgłoszeniach w „Projekty (GitHub)”
status: implementing
created: 2026-11-24
updated: 2026-11-24
archived_at: null
---

## Notes

Roadmap S-04 (M-2 `konsola-gaps-and-cleanup`), GitHub issue #7, PRD FR-025. Użytkownik widzi etykiety każdego zgłoszenia GitHub jako tekst w jego wierszu, tak jak w GitLabie. Zmienia stałe wywołanie `gh issue list` w `src-tauri/src/cli/` (dalej tylko do odczytu, stałe argumenty, objęte testem strażnika); testy przez `fake-cli` z wymyślonymi danymi.
