# ErrorState

Jedyne miejsce, w którym aplikacja pokazuje błąd źródła albo listy: opakowuje `SourceState kind="error"` (poziomy układ na `danger-bg`, ikona `alert-triangle` w `danger`, `role="alert"`), więc awaria nigdy nie wygląda jak pusta lista.

- Podaj `title` (co się nie udało) i `message` — wyłącznie treść liniową (tekst, `<code>`, linki), bo trafia do `<p>`. Napisz, jak to naprawić („Zaloguj się w terminalu poleceniem `gh auth login`”).
- `onRetry` pokazuje przycisk „Spróbuj ponownie”; etykiety nie zmieniasz.
- `lastSuccess` to gotowe zdanie, np. „ostatnio OK 08:52” — komponent nic nie dopisuje. Podaj je, gdy wcześniej było udane odświeżenie.
- Błąd całego źródła: `ErrorState`. Brak VPN to nie błąd: użyj `SourceState kind="calm"`. Błąd części widoku lub akcji: `Notice tone="error"`.
