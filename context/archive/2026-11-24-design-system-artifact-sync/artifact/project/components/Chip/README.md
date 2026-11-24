# Chip

Mały obrysowany znacznik wartości, w `mono` i pogrubiony — w „Moich zadaniach” oznacza priorytet, w pasku filtrów GitLaba wartość filtra.

- Ton: `warn` (bursztyn, priorytet wysoki), `soft` (`ink-soft`, normalny), `faint` (`ink-faint`, niski albo wykluczona wartość filtra). Bez `tone` chip dziedziczy kolor tekstu.
- Zmianę pozycji sygnalizuje wyłącznie `Flag`; `Chip` nie mówi „nowe” ani „zmienione”.
- Źródło pozycji (`GL`, `GH`, `MY`, `MR`, `ISS`) to `SourceChip`, nie `Chip`.
- Dla czytnika dopisz przed wartością ukryty prefiks (`<span className="dn-sr-only">Priorytet: </span>`); sam kolor niczego nie mówi.
- `Chip` i `SourceChip` mają wspólną klasę `dn-chip`, więc wyglądają tak samo (mono, pogrubienie, wyśrodkowanie); różni je tylko kolor tekstu.
- Nie dodawaj własnych tonów i nie używaj `danger`.
