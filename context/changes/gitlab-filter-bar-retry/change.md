---
change_id: gitlab-filter-bar-retry
title: GitLab — pasek wartości filtra i czas następnej próby
status: implementing
created: 2026-11-24
updated: 2026-11-24
---

## Notes

Slice S-03 z milestone'u M-2 (`konsola-gaps-and-cleanup`), GitHub issue #6, PRD FR-024. Nad listą „Praca (GitLab)” pasek FILTR pokazuje wartości aktywnego filtra z linkiem otwierającym `settings.json`, a karta „GitLab niedostępny” podaje interwał odświeżania i czas następnej próby (odliczanie i godzina). Wartości filtra to dane pracy: trafiają tylko do UI, nigdy do logów, testów ani fixture'ów.
