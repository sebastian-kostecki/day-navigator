---
change_id: todo-add-from-sources
title: "+" do „Do zrobienia” w widokach GitLab i GitHub
status: implementing
created: 2026-11-24
updated: 2026-11-24
archived_at: null
---

## Notes

Slice S-01 z milestone'u M-2 (`konsola-gaps-and-cleanup`), GitHub issue #8, PRD FR-026, US-01. Każdy wiersz „Praca (GitLab)” i „Projekty (GitHub)” dostaje przycisk „+” dodający pozycję do „Do zrobienia”, z blokadą limitu i podpowiedzią jak w pickerze; pozycja już dodana ma flagę W DO ZROBIENIA i wypełniony przycisk. Widoki źródeł poznają stan listy bez nowego pollera; reguły limitu i przenoszenia zostają w Rust.
