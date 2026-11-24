# HoursBar

Poziomy pasek godzin jednego dnia na tle normy: tor `line-subtle`, wypełnienie `ink-soft` (dziś: `accent`), pionowa kreska normy `ink-faint`, skala `max` domyślnie 10 h.

- `norm` i `label` są wymagane, `norm` nie ma wartości domyślnej. Normę dnia (w godzinach) liczy Rust i przekazuje do widoku; UI jej nie zgaduje. Nie wpisuj na sztywno 8.
- `label` to nazwa paska dla czytnika, gotowe zdanie: „7:30 na tle normy 8:00”; bez normy: „2:00, bez normy”. Pasek jest `role="img"`; liczby obok i tak są w tekście.
- W tabeli miesiąca w „Czasie pracy” (`DataTable`): jeden pasek na dzień, obok sumy `G:MM` i odchylenia ze znakiem (`info` nad normą, `warn` pod).
- Dzień wolny lub bez normy: `norm={0}` ukrywa kreskę; wiersz dostaje ukośne paski (`hatched` w `DataTableRow`).
- `today` wypełnia pasek kolorem `accent` — tylko dla bieżącego dnia.
