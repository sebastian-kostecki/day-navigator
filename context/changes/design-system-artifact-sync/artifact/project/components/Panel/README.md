# Panel

Podstawowy pojemnik ekranu: `surface` z obrysem `line` i promieniem `radius-lg`, nagłówek z tytułem `panel-title` i meta w `mono` po prawej, opcjonalna stopka `meta`.

- Zawartość podaje konsument: wiersze `ItemRow`, listy, tabelę, `dl` z faktami.
- `meta` to liczba lub licznik: `[4/5]`, `15 pozycji`, `23.11 pn`. Nie wstawiaj tam przycisków.
- `tone="danger"` tylko dla karty źródła w stanie błędu.
- Panele układaj w rzędzie `flex-wrap` z odstępem `space-5`; bez cieni i bez zagnieżdżania paneli w panelach.
