# StatusPill

Pigułka stanu z glifem i słowem: `ok` (● OK, `info` na `info-bg`), `calm` (○ NIEDOSTĘPNY, `ink-faint` na `surface-raised`), `error` (▲ BŁĄD, `danger` na `danger-bg-strong`).

- `calm` wyłącznie dla GitLaba bez VPN albo źródła bez wpisu w ustawieniach — stan normalny, bez alarmu i powiadomienia.
- `error` dla wylogowanego `glab`/`gh`, uszkodzonej lub nowszej bazy; to samo pokazuje zasobnik.
- Domyślnie glif i słowo idą razem — stan nie zależy od koloru.
- `glyph={false}` pokazuje sam tekst: w karcie źródła glif stoi osobno w głowie karty, a w „Przypomnieniach” pigułka ma własny tekst WŁ. (`ok`) / WYŁ. (`calm`). Tekst podaj w `children` (WŁ., WYŁ., NIESKONFIGUROWANE) — też wersalikami.
- Własny tekst tylko w tym samym duchu co stan; `className` służy do dopasowania układu, nie koloru.
