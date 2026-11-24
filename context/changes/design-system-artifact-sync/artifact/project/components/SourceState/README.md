# SourceState

Blok zastępujący listę, gdy źródło nie ma danych do pokazania. Trzy rodzaje, nigdy pusty panel:

- `kind="empty"` — dane są, ale nic w nich nie ma. Ikona `check-circle` w `info`, jedno zdanie: „Brak otwartych zgłoszeń przypisanych do Ciebie w 3 repozytoriach z ustawień.”
- `kind="calm"` — GitLab bez VPN. Ikona `wifi-off` w `ink-faint`, wyjaśnienie, że to normalne, `lastOk` i akcja „Sprawdź teraz”. Bez czerwieni, bez `role="alert"`.
- `kind="error"` — prawdziwa awaria (wylogowane CLI, problem z bazą). Układ poziomy na `danger-bg`, ikona `alert` w `danger`, `role="alert"`, w treści jak to naprawić (`gh auth login`), `lastOk`.

Zawsze podaj `lastOk` dla `calm` i `error` — czas ostatniego udanego odświeżenia jest wymagany na każdym widoku źródła.
