# Field

Etykieta `label-caps` nad polem tekstowym, datą, listą wyboru (gdy podasz `options`) albo wieloliniowym polem (`multiline`). Pole: `bg` z obrysem `line-control`, wysokość `size-field` (40px), `color-scheme: dark` dla natywnych kontrolek daty.

- Każde pole ma widoczną etykietę, powiązaną przez `id`/`htmlFor`; placeholder tylko podpowiada („Tytuł — wystarczy sam tytuł”), nie zastępuje etykiety. W aplikacji `id` dobiera sam komponent; podaj własne, gdy sterujesz nim z zewnątrz.
- Pola opcjonalne (termin, priorytet) nie są oznaczane gwiazdką; wymagane jest tylko to, co naprawdę wymagane — w zadaniu sam tytuł.
- `grow` w rzędzie formularza zabiera wolną szerokość (tytuł), pozostałe pola mają stałą szerokość.
- `multiline` renderuje `textarea` (notatka zadania): zwykły tekst, `rows` ustawiasz sam, rozciąga się tylko w pionie, bez formatowania.
- `disabled` ustawia tekst w `ink-disabled` i nie zmienia układu; pozostałe atrybuty (`type`, `value`, `placeholder`, `style`) przechodzą do kontrolki.
- Obok pola o wysokości 40px ustaw `Segmented size="md"` — wtedy wysokości się zgadzają.
