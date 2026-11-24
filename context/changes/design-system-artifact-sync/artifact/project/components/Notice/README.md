# Notice

Baner w widoku — kompaktowy kuzyn `SourceState`: `surface-raised`, obrys `line`, `radius-lg`, ikona tonu z lewej, tekst `ink-muted`.

- Ton (wymagany): `calm` (ikona `wifi-off`, niedostępne, nigdy czerwone), `info` (`check-circle`, zrobione lub informacja), `warn` (`alert-triangle`, osłabione działanie lub zła konfiguracja), `error` (`alert-triangle`, nieudana akcja albo część widoku).
- `title` to opcjonalny pogrubiony wiersz nad tekstem; treść jako `children` (tekst, `<code>`).
- `action` + `onAction` dają mały przycisk `secondary` po prawej („Spróbuj ponownie”); `actionDisabled` blokuje go na czas akcji.
- `compact` — mniejszy krój i odstępy: pod polem, w wierszu, w nagłówku (licznik czasu pracy). `inset` — wewnątrz treści `Panel`, z jego marginesem bocznym.
- `role`: domyślnie `alert` dla `error`, `status` dla pozostałych. Podaj `null`, gdy ogłasza go zewnętrzny obszar `aria-live`.
- Całą listę źródła zastępuje `SourceState` (puste, niedostępne) albo `ErrorState` (błąd) — `Notice` nie zastępuje listy i nie zostawia pustego panelu.
