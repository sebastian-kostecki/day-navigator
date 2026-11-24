# ItemRow

Wiersz pozycji o wysokości `size-row` (52px): opcjonalny checkbox, `SourceChip`, numer w `mono`, tytuł `item`, opcjonalna `Flag`, etykiety, meta po prawej i narzędzia wiersza.

- Checkbox pojawia się tylko z `onToggle`; wtedy `id` jest wymagane (łączy etykietę z checkboxem — komponent go nie losuje). Podaj `checkboxLabel` („Zrobione: tytuł”) jako nazwę dla czytnika; `checkboxDisabled` blokuje go na czas zapisu. Tytuł jest etykietą checkboxa (klik w tytuł odhacza).
- Bez `onToggle` tytuł jest zwykłym `span`, bez checkboxa i bez `id`.
- Odhaczona pozycja (`done` albo `checked`, to samo) zostaje w miejscu: tytuł `ink-done` z przekreśleniem.
- `selected` (edytowany wiersz): `surface-raised` i kreska 2px `accent` po lewej.
- `flag` + `flagTone` to pierwsza flaga wiersza; kolejne flagi (kolejność: NOWE, ZMIENIONE, DRAFT, W DO ZROBIENIA) idą do `meta` jako `Flag`.
- `labels` (lista nazw) wyświetla się po przecinku w `mono`, z ukrytym prefiksem „Etykiety:” dla czytnika; pusta lub brakująca lista nic nie renderuje.
- Narzędzia jako `children`: przyciski `Button variant="quiet" iconOnly` z `aria-label`, najwyżej trzy.
- Tytuł obcina się wielokropkiem w jednej linii; pełny tekst jest w źródle.
