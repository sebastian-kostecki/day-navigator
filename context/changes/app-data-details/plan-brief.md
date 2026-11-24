# Szczegóły „Dane aplikacji” w „Stanie źródeł” — Plan Brief

> Full plan: `context/changes/app-data-details/plan.md`

## What & Why

Karta „Dane aplikacji” w „Stanie źródeł” ma pokazywać czas ostatniej kopii, liczbę zachowanych kopii („7 z 7”), wersję schematu i przycisk „Otwórz folder kopii” (S-07, PRD FR-022, issue #4). Dziś karta mówi tylko, że baza jest otwarta, a nieudana kopia dzienna zostawia ślad wyłącznie w logu.

## Starting Point

Kopia dzienna (`store/backup.rs`) tworzy `backups/day-navigator-YYYY-MM-DD.db` i trzyma 7 najnowszych z datą ≤ dziś. Runda health co 30 s sprawdza bazę pod blokadą store i publikuje `HealthSnapshot`; karta renderuje listę `facts`. Otwieranie pliku z Rust ma gotowy wzorzec w `settings_file.rs`, a plugin opener jest już podłączony.

## Desired End State

Karta pokazuje „Sprawdzono”, „Ostatnia kopia” (np. „2 godz. temu”, z ostrzeżeniem, gdy dzisiejszej kopii brak), „Kopie 7 z 7”, „Schemat v10” i przycisk „Otwórz folder kopii”, który otwiera tylko `<app data dir>/backups/`. Gdy baza jest uszkodzona lub jej brak, karta dalej pokazuje kopie — dokładnie wtedy, gdy komunikat każe jedną przywrócić.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Skąd dane | Runda health → `HealthSnapshot.appData` | Blokada store i tak jest trzymana w `status()`; brak nowej komendy i osobnych stanów ładowania/błędu w karcie. |
| Karta przy błędzie bazy | Kopie zawsze, schemat tylko przy otwartej bazie | Folder kopii czyta się bez bazy, a przy awarii to najważniejsza informacja. |
| Liczenie „N z 7” | Jak `prune`: tylko kopie z datą ≤ dziś | Liczba zawsze ≤ 7 i zgadza się z regułą przycinania; kopia z przyszłości nie jest „ostatnią”. |
| Brak dzisiejszej kopii | Ton ostrzeżenia na „Ostatnia kopia” | Pierwszy widoczny ślad nieudanej kopii, bez alarmu w zasobniku i bez toastu. |
| Otwieranie folderu | Komenda Rust `open_backups_folder` wzorem `settings_file.rs` | Stała ścieżka, brak folderu = błąd (nic nie tworzy), bez zmian w capabilities i pluginach. |
| Nieczytelny folder | Osobny stan `unreadable` | Nigdy pusta lista w miejscu błędu (`AGENTS.md`). |

## Scope

**In scope:**
- Podsumowanie `backups/` (`kept`, `keep`, `last { date, atMs }`, `stale`) i wersja schematu w `HealthSnapshot.appData`
- Komenda `open_backups_folder` z błędem `DataDirUnresolved | Missing | OpenFailed`
- Lustro TS, nowy `src/health/api.ts` (także dla istniejących `invoke` health)
- Fakty karty z tonem ostrzeżenia, przycisk z błędem inline

**Out of scope:**
- Migracje, nowe pluginy, zmiany `capabilities/default.json`
- Zmiana stanu źródła, zasobnika, toastów lub dziennika z powodu brakującej kopii
- Zmiany reguł kopii, „zrób kopię teraz”, przywracanie z UI
- Ścieżki w danych karty

## Architecture / Approach

`check_round` → `store_status`: najpierw `StoreHandle::status(today)` (otwarcie, sprawdzenie, kopia; zwraca `user_version`), potem `StoreHandle::backup_summary(today)` (czysty `read_dir` folderu `backups/`). Obie wartości lądują w `HealthSnapshot.appData` i dochodzą do UI zdarzeniem `health-changed`. Przycisk woła `open_backups_folder`, który bierze ścieżkę z `StoreHandle::backups_dir()` bez blokady i otwiera ją openerem z Rust.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Kontrakt backendu | `appData` w migawce, komenda folderu, lustro TS + `api.ts`, testy Rust | Odczyt folderu przed `status` dałby fałszywe `stale` po północy/starcie |
| 2. Karta „Dane aplikacji” | Fakty (ostatnia kopia z ostrzeżeniem, „N z 7”, schemat), przycisk z błędem inline | Ton ostrzeżenia tylko z tokenów; inne karty bez zmian |

**Prerequisites:** F-03 (done); gałąź od `develop` z migracją v10.
**Estimated effort:** ~1 sesja, 2 fazy.

## Open Risks & Assumptions

- Czas ostatniej kopii pochodzi z `modified()` pliku; bez niego karta pokazuje samą datę.
- Równoległy S-06 może dodać migrację — wersja schematu jest czytana z bazy, nie wpisana na sztywno.

## Success Criteria (Summary)

- Karta „Dane aplikacji” pokazuje ostatnią kopię, „N z 7” i wersję schematu zgodne z plikami w `backups/`.
- „Otwórz folder kopii” otwiera `backups/`, a brak folderu daje czytelny błąd bez tworzenia folderu.
- Wszystkie bramki (`cargo test`, clippy, fmt, lint, format, build) przechodzą.
