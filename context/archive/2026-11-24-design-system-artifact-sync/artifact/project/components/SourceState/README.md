# SourceState

Blok zastępujący listę, gdy źródło nie ma danych do pokazania. Trzy rodzaje, nigdy pusty panel:

- `kind="empty"` — dane są, ale nic w nich nie ma. Ikona `check-circle` w `info`, jedno zdanie: „Brak otwartych zgłoszeń przypisanych do Ciebie w 3 repozytoriach z ustawień.”
- `kind="calm"` — GitLab bez VPN. Ikona `wifi-off` w `ink-faint`, wyjaśnienie, że to normalne, `lastOk` i akcja „Sprawdź teraz”. `role="status"`, bez czerwieni i bez `role="alert"`.
- `kind="error"` — prawdziwa awaria (wylogowane CLI, problem z bazą). Układ poziomy na `danger-bg`, ikona `alert-triangle` w `danger`, `role="alert"`, w treści jak to naprawić (`gh auth login`), `lastOk`. W kodzie widoków błąd renderuj przez `ErrorState`, który opakowuje ten wariant.

- `lastOk` to gotowe zdanie, wyświetlane tak, jak je podasz: „ostatnio OK 08:52”. Komponent nic nie dopisuje. Podawaj je zawsze dla `calm` i `error` — czas ostatniego udanego odświeżenia jest wymagany na każdym widoku źródła.
- `action` + `onAction` to przycisk `secondary` `lg`. `actionDisabled` blokuje go, gdy kliknięcie nic by nie zmieniło (odświeżanie w toku): przycisk dostaje `aria-disabled` + `aria-busy`, zostaje fokusowalny, a etykieta zostaje tekstem akcji.
- Baner w widoku, nie zamiast listy: `Notice`.
