# StatusPill

Pigułka stanu źródła z glifem i słowem: `ok` (● OK, `info` na `info-bg`), `calm` (○ NIEDOSTĘPNY, `ink-faint` na `surface-raised`), `error` (▲ BŁĄD, `danger` na `danger-bg-strong`).

- `calm` wyłącznie dla GitLaba bez VPN — stan normalny, bez alarmu i powiadomienia.
- `error` dla wylogowanego `glab`/`gh`, uszkodzonej lub nowszej bazy; to samo pokazuje zasobnik.
- Glif i słowo zawsze razem — stan nie zależy od koloru. Własny tekst przez `children` tylko w tym samym duchu („OK · 4 min temu” wyjątkowo w karcie).
