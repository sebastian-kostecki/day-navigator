---
change_id: event-log
title: Dziennik zdarzeń na dashboardzie
status: implemented
created: 2026-11-24
updated: 2026-11-24
---

## Notes

Slice S-08 z milestone'u M-2 (`konsola-gaps-and-cleanup`), GitHub issue #1, PRD FR-019. Na dashboardzie („Do zrobienia”) pod „Stanem źródeł” panel „Dziennik zdarzeń” z dzisiejszymi zdarzeniami: nowe przypisanie w GitLabie, odhaczenie lub zdjęcie pozycji „Do zrobienia” (także zamknięcie w źródle), START/STOP, przejście źródła w błąd i powrót. Nowa tabela `events` (migracja v10) trzyma tylko dzisiejsze zdarzenia i tylko to, co UI i tak pokazuje (referencja i tytuł pozycji, nazwa źródła, rodzaj błędu). Nic z niej nie trafia do logu aplikacji.
