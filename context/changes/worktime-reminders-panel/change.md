---
change_id: worktime-reminders-panel
title: Panel „Przypomnienia” w „Czas pracy” (progi i stan, tylko odczyt)
status: implemented
created: 2026-11-24
updated: 2026-11-24
---

## Notes

Roadmap S-06 (M-2), GitHub issue #3, PRD FR-021. W „Czas pracy” użytkownik widzi panel „Przypomnienia” z progami przypomnień (licznik biegnie dłużej niż N h; brak START-u w dzień roboczy po GG:MM), informacją, czy każde jest włączone, i dopiskiem „domyślnie”, gdy wartość nie pochodzi z `settings.json`. Bez edycji: ustawienia zostają w pliku, panel ma tylko przycisk „Ustawienia” otwierający plik. Wartości czyta Rust tym samym parserem co wątek przypomnień; UI tylko wyświetla. Nieprawidłowy blok `reminders` to stan błędu („przypomnienia wstrzymane”), nigdy „wyłączone”. Buduje na S-05 (`WorkTimeView.tsx` z tabelą dni).
