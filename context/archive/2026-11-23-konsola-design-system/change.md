---
change_id: konsola-design-system
title: Design system Konsola w aplikacji — tokeny, komponenty i przestylowanie widoków
status: archived
created: 2026-11-23
updated: 2026-11-24
archived_at: 2026-11-24T09:15:45Z
---

## Notes

Przenieść design system "Day Navigator — Konsola" (artefakt https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY) do aplikacji w src/: tokeny jako zmienne CSS (ciemny motyw, kolory, typografia IBM Plex Sans + JetBrains Mono, odstępy, promienie, rozmiary), komponenty współdzielone w src/components/ (AppHeader, Button, Icon, SourceChip, Flag, StatusPill, StatStrip, Panel, ItemRow, Segmented, Tabs, SourceState, Field, HoursBar) jako komponenty React 19 + TS, a następnie przestylowanie sześciu istniejących widoków (todo, gitlab, github, own-tasks, worktime, health) na te komponenty. Zasady: bursztynowy akcent tylko w trzech rolach, "dobrze" = niebieski, czerwony tylko dla błędu i REC, brak VPN to spokojny stan; ErrorState.tsx zostaje źródłem stanów loading/error/empty. Bez nowych zależności npm poza tym, co konieczne (fonty: rozważyć lokalne pliki zamiast Google Fonts, bo aplikacja desktopowa offline). Logika domenowa bez zmian.
