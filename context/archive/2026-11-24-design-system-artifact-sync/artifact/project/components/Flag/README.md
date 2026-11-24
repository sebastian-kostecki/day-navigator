# Flag

Obrysowana etykieta przy tytule pozycji, która mówi, dlaczego pozycja wymaga uwagi. Wersaliki ustawia CSS (`text-transform: uppercase`), więc tekst możesz zapisać „Nowe”.

- `tone="change"` (domyślny, `flag`, bursztyn): `ZMIENIONE` (ukryte wróciło po zmianie statusu lub cudzym komentarzu), `Z PIĄTKU` / `Z WCZORAJ` (przeniesione), `DZIŚ` (termin), `DRAFT`, `W DO ZROBIENIA`.
- `tone="new"` (`info`, niebieski): `NOWE` — nowe przypisanie.
- `tone="info"` (`info`, niebieski): zamknięte w źródle.
- Wiersz może mieć kilka flag, zawsze w kolejności: `NOWE`, `ZMIENIONE`, `DRAFT`, `W DO ZROBIENIA`. W `ItemRow` pierwsza idzie do `flag` (z `flagTone`), pozostałe do `meta`.
- Bez wypełnienia — flaga nie konkuruje z przyciskiem primary. Znacznik priorytetu to `Chip`, nie `Flag`.
