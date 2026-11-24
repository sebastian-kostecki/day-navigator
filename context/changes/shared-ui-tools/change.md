---
change_id: shared-ui-tools
title: Wspólne narzędzia UI wyciągnięte z widoków
status: implemented
created: 2026-11-24
updated: 2026-11-24
archived_at: null
---

## Notes

Roadmap F-01 (M-2), GitHub issue #10. Przenieść do `src/components/` narzędzia, które powstały lokalnie w widokach przy dopasowaniu do makiet Konsoli: `useNow(intervalMs)`, `plural()`, `Tabs`, komponent tabeli/siatki, rozmiar `Segmented` dla pól 40 px, chip priorytetu, `ErrorState.message` jako `ReactNode`, `disabled` dla akcji `SourceState`. Widoki przepięte na wspólne elementy bez zmiany wyglądu; odblokowuje S-03 (`gitlab-filter-bar-retry`) i S-05 (`worktime-days-table`).
