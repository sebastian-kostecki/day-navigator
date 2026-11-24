# DataTable

Gęsta tabela na siatce CSS (`role="table"`): wiersz `size-row`, dzielnik `line-subtle`, nagłówek `label-caps` na `size-control` z kreską `line`.

- Widok ustawia szablon kolumn w klasie podanej w `className` (wymagane): `--dn-table-cols` (np. `84px 76px minmax(0, 1fr) 52px 60px`), opcjonalnie `--dn-table-pad-x`. Bez tego wiersze się zapadają. Podaj też `label` — nazwę tabeli dla czytnika.
- Składaj z `DataTableRow` i `DataTableCell`. Nagłówek to wiersz z `head` i komórkami `header`.
- `current` — wiersz „teraz”: tło `surface-raised`, kreska 2px `accent` z lewej i `aria-current`. `hatched` — dzień wolny: ukośne paski (`--hatch-bg` na `hatch-stripe`). Oba łączą się; paski zostają widoczne pod `current`.
- Ton komórki: `muted` (drugi plan, np. weekend), `faint` (meta, brak danych), `info` (wartość nad normą), `warn` (pod normą, ze znakiem `−`). `truncate` ucina tekst wielokropkiem w jednej linii; pełny tekst daj w `title`.
- Liczby w komórkach jako `G:MM` w `mono` (klasa `code`), wyrównane do prawej; pasek godzin to `HoursBar` z jawnymi `norm` i `label`.
- Tabelę trzymaj w `Panel`; nie dodawaj własnych kolorów tła wierszy.
