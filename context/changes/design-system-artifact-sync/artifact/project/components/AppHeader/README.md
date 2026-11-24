# AppHeader

Stały nagłówek każdego ekranu: znak `DAY/NAV`, sześć numerowanych zakładek widoków i slot po prawej na licznik czasu pracy.

- Props: `items` (zawsze sześć, w kolejności: Do zrobienia, GitLab, GitHub, Moje zadania, Czas pracy, Źródła), `onSelect(id)` i `children` — licznik. Komponent sam nie rysuje zegara, `running` ani START/STOP.
- Element `items`: `{ id, label, active, key }`. `key` („1”…„6”) pokazuje numer przed etykietą i ustawia `aria-keyshortcuts`; `id` trafia do `data-view`. Dokładnie jedna zakładka `active` — podkreślenie `accent` i `aria-current="page"`.
- Skróty: cyfry 1–6 przełączają widok. Obsługuje je aplikacja (`App.tsx`), nie komponent: bez modyfikatora, bez powtórzeń przy przytrzymaniu, nie podczas pisania w polu, a fokus przechodzi na wybraną zakładkę (`[data-view]`).
- Licznik zawsze widoczny; nie chowaj go na żadnym widoku. `● REC` w `danger` tylko gdy biegnie; zatrzymany: `○ STOP` i START w wariancie `primary`, biegnący: STOP `secondary`. Przyciski `size="lg"` z `dn-btn-mono`.
- START/STOP najpierw pyta („Rozpocząć pracę?” / „Zakończyć pracę?”) i podmienia przycisk na `[TAK, START|STOP]` (START `primary`, STOP `secondary`), a po nim `[Anuluj]` — „Anuluj” stoi w miejscu START/STOP i dostaje fokus. Pytanie wygasa po 6 s lub po Escape; kliknięcia w pierwszych 400 ms ignoruje (ogon dwukliku).
- Wczytywanie: „Czas pracy: wczytywanie…”. Błąd licznika to kompaktowy `Notice tone="error"` z „Spróbuj ponownie”, nie zniknięcie licznika.
- Nagłówek zawija się na wąskim oknie: zakładki schodzą pod znak, licznik zostaje w całości.
