# StatStrip

Pas kafli wskaźników rozdzielonych 1px szczeliną w `line`: etykieta `label-caps`, wartość `stat` w `mono`, podpis `meta`.

- Na „Do zrobienia” i „Czas pracy”: Dziś, Miesiąc, Odchylenie, Średnio / dzień, plus Źródła albo Dni wolne. Pięć kafli, nie więcej.
- `tone="info"` dla odchylenia dodatniego, `tone="warn"` dla ujemnego (ze znakiem `−`), `tone="soft"` dla wartości pomocniczych (`ink-soft`; `muted` to jego alias). Bez tonu wartość jest `ink-strong`.
- Wartości czasu jako `G:MM`; podpis mówi, z czego wynika liczba („do wczoraj włącznie”, „przez 6 dni roboczych”).
- Kafle zawijają się od 180px szerokości.
