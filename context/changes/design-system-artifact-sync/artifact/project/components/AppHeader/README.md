# AppHeader

Stały nagłówek każdego ekranu: znak `DAY/NAV`, sześć zakładek widoków ze skrótami 1–6 i licznik czasu pracy z przyciskiem START/STOP.

- Podaj `items` (zawsze sześć, w kolejności: Do zrobienia, GitLab, GitHub, Moje zadania, Czas pracy, Źródła), `clock` jako `GG:MM:SS`, `running` i `onToggle`.
- Dokładnie jedna zakładka `active` — dostaje podkreślenie `accent` i `aria-current="page"`.
- Licznik zawsze widoczny; nie chowaj go na żadnym widoku.
- `● REC` w `danger` tylko gdy licznik biegnie. Zatrzymany: `○ STOP` i przycisk START w wariancie primary.
- Nagłówek zawija się na wąskim oknie: zakładki schodzą pod znak, licznik zostaje w całości.
