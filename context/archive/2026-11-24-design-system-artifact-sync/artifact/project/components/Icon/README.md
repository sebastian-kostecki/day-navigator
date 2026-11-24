# Icon

Kreskowa ikona 24×24 (obrys 2px, zaokrąglone końce) w `currentColor`, domyślnie 14px.

- Nazwy: `plus`, `close`, `refresh`, `edit`, `eye`, `eye-off`, `external`, `chevron-left`, `chevron-right`, `wifi-off`, `alert-triangle`, `check-circle`, `repo`, `clock`, `folder`.
- Aliasy starszych nazw: `hide` = `eye-off`, `show` = `eye`, `alert` = `alert-triangle`. W nowym kodzie używaj nazw docelowych.
- Bez `label` ikona jest dekoracyjna (`aria-hidden`); z `label` dostaje `role="img"`. W przycisku-ikonie nazwę daje `aria-label` przycisku.
- Kolor dziedziczy z rodzica: `ink` w przyciskach, `ink-faint` w nagłówku repozytorium, `danger` tylko w bloku błędu. `size` i `strokeWidth` zmieniaj rzadko (ikony stanów w `SourceState` mają 36px i 1.5).
- Nie zastępuj ikon emoji ani znakami — wyjątkiem są glify stanu `● ○ ▲`.
