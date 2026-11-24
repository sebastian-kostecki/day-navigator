# Panel

Podstawowy pojemnik ekranu: `surface` z obrysem `line` i promieniem `radius-lg`, nagłówek z tytułem `panel-title` i meta w `mono` po prawej, opcjonalna stopka.

- Zawartość podaje konsument: wiersze `ItemRow`, listy, `DataTable`, `dl` z faktami, `Notice` z `inset`.
- `meta` to liczba lub licznik: `[4/5]`, `15 pozycji`, `23.11 pn`. Nie wstawiaj tam przycisków.
- `footer` to drobny tekst `meta` albo narzędzia panelu (cichy przycisk `sm`, kompaktowy `Notice`), np. „Ustawienia” w „Przypomnieniach”.
- `tone="danger"` tylko dla karty źródła w stanie błędu.
- `headingLevel`: `h2` domyślnie; `1` tylko gdy tytuł panelu jest tytułem widoku.
- `className` dodaje klasę widoku (np. `flex: 1 1 280px`); komponent nie przyjmuje `style`.
- Panele układaj w rzędzie `flex-wrap` z odstępem `space-5`; bez cieni i bez zagnieżdżania paneli w panelach.
