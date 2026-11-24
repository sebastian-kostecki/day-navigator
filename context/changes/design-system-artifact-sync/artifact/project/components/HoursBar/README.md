# HoursBar

Poziomy pasek godzin jednego dnia na tle normy: tor `line-subtle`, wypełnienie `ink-soft` (dziś: `accent`), pionowa kreska normy `ink-faint` na 8 h, skala domyślnie 10 h.

- W tabeli miesiąca w „Czasie pracy”, jeden pasek na dzień roboczy, obok sumy `G:MM` i odchylenia ze znakiem (`info` nad normą, `warn` pod).
- Dzień wolny: `norm={0}` (bez kreski) i wiersz w ukośne paski.
- Pasek jest `role="img"` z nazwą („7,5 h na tle normy 8 h”); liczby obok i tak są w tekście.
