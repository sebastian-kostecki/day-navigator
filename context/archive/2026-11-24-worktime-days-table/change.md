---
change_id: worktime-days-table
title: Tabela dni miesiąca w „Czas pracy” (pasek na tle 8 h)
status: archived
created: 2026-11-24
updated: 2026-11-24
archived_at: 2026-11-24T12:10:42Z
---

## Notes

Roadmap S-05 (M-2), GitHub issue #2, PRD FR-020. W „Czas pracy” użytkownik widzi tabelę dni miesiąca: wpisy, pasek na tle normy dnia, sumę, odchylenie od normy dnia i przełącznik dnia wolnego; dziś jest wyróżniony, a dni wolne zakreskowane. Dzień wolny można oznaczyć prosto z tabeli. Raport dzienny i reguła normy dnia żyją w Rust i mają testy jednostkowe; UI tylko wyświetla. Wiersz „dziś” czyta wspólny stan licznika (`useWorkTimer()`, F-02); tabela korzysta z `DataTable` i `plural` (F-01). Panel przypomnień (S-06) zostaje nietknięty.
