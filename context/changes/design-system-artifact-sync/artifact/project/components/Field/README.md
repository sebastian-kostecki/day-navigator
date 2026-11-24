# Field

Etykieta `label-caps` nad polem tekstowym, datą albo listą wyboru (gdy podasz `options`). Pole: `bg` z obrysem `line-control`, wysokość `size-field`, `color-scheme: dark` dla natywnych kontrolek daty.

- Każde pole ma widoczną etykietę; placeholder tylko podpowiada („Tytuł — wystarczy sam tytuł”), nie zastępuje etykiety.
- Pola opcjonalne (termin, priorytet) nie są oznaczane gwiazdką; wymagane jest tylko to, co naprawdę wymagane — w zadaniu sam tytuł.
- `grow` w rzędzie formularza zabiera wolną szerokość (tytuł), pozostałe pola mają stałą szerokość.
- Notatka zadania to zwykłe `textarea` z klasą `dn-input`, bez formatowania.
