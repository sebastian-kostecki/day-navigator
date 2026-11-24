# ItemRow

Wiersz pozycji o wysokości `size-row` (52px): checkbox, `SourceChip`, numer w `mono`, tytuł `item`, opcjonalna `Flag`, meta po prawej i narzędzia wiersza.

- Checkbox pojawia się, gdy podasz `done` lub `onToggle`; tytuł jest jego etykietą (klik w tytuł odhacza).
- Odhaczona pozycja zostaje w miejscu: tytuł `ink-done` z przekreśleniem.
- `selected` (edytowany wiersz): `surface-raised` i kreska 2px `accent` po lewej.
- Narzędzia jako `children`: przyciski `Button variant="quiet" iconOnly` z `aria-label`, najwyżej trzy.
- Tytuł obcina się wielokropkiem w jednej linii; pełny tekst jest w źródle.
