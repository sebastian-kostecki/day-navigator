# RemindersPanel

Wzorzec widoku „Przypomnienia” — panel z progami przypomnień o liczniku czasu pracy; to złożenie komponentów, nie eksport paczki.

- W aplikacji: `src/worktime/RemindersPanel.tsx`, widok „Czas pracy”.
- Zbudowany z `Panel` (tytuł „Przypomnienia”), `StatusPill`, `Button` i `Notice`; układ to klasy `wt-*` z sekcji „Wzorce widoków” w `bundle.css`.
- Wiersz to `div.wt-reminder` w `dl.wt-reminders`: `dt` z nazwą, `dd.wt-reminder-value` (wartość w `code` albo „—”, `StatusPill glyph={false}` z tekstem WŁ./WYŁ., „domyślnie” w `meta`, gdy próg nie pochodzi z pliku ustawień) i `dd.meta.wt-reminder-hint` z jednym zdaniem wyjaśnienia.
- Stopka: cichy przycisk `sm` z ikoną `external` „Ustawienia” (otwiera plik ustawień). Błąd otwarcia to kompaktowy `Notice tone="error"` obok przycisku, nie osobny blok.
- Błąd wczytania progów idzie przez `ErrorState` w `wt-panel-body`, z „ostatnio OK GG:MM” i „Spróbuj ponownie”; wczytywanie to „Ładowanie…” (`dn-loading`). Nigdy pusty panel w miejscu błędu.
- Progi edytuje się w pliku ustawień, nie w panelu — panel nie ma pól.
