# GitLab — pasek wartości filtra i czas następnej próby: plan brief

> Full plan: `context/changes/gitlab-filter-bar-retry/plan.md`

## What & Why

Makieta „Praca (GitLab)” ma nad listą pasek FILTR z wartościami aktywnego filtra i linkiem do ustawień, a karta „GitLab niedostępny” podaje interwał odświeżania i czas następnej próby. Dziś UI wie tylko, że filtr jest aktywny, a karta offline mówi ogólnie „przy następnym automatycznym odświeżeniu”. To slice S-03 (issue #6, PRD FR-024).

## Starting Point

Wartości filtra (`GitlabFilter`) i interwał (`refresh_interval`) żyją tylko w Rust; migawka niesie `filterActive`, `filterInvalid` i liczniki. Aplikacja nie ma ekranu ustawień — ustawienia to `settings.json`, a webview nie ma uprawnień do otwierania plików.

## Desired End State

Przy aktywnym filtrze (albo `closedDays`) nad listą stoi pasek „FILTR · Projekty: … · bez: … · Etykiety: … · Kamienie milowe: … · Zamknięte: 7 dni · [Ustawienia]”; przycisk otwiera `settings.json` w domyślnym programie. Bez VPN karta mówi „Odświeżam co 10 min — następna próba za 4 min (14:32).”, z odliczaniem i „sprawdzam teraz…” w trakcie rundy.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Link do ustawień | Komenda Rust `open_settings_file` otwiera `settings.json` w domyślnym programie | Prawdziwy link jak w makiecie, bez nowych uprawnień webview; brak pliku to błąd, pliku nie tworzymy. |
| Kiedy pasek | Tylko przy aktywnym filtrze lub `closedDays` | Bez filtra pasek nie niesie informacji. |
| Tekst następnej próby | Odliczanie + godzina | Czytelne i po chwili nieobecności przy ekranie. |
| Kto liczy czas próby | Rust: `nextRefreshAtMs` = koniec rundy + interwał | Reguła oczekiwania żyje w wątku Rust; UI tylko formatuje. |
| Postać wartości | Jak działa filtr (przycięte, małe litery) | Uczciwie pokazuje dopasowanie bez rozróżniania wielkości liter, bez drugiej kopii wpisów. |
| Prywatność | `GitlabFilter` z nieprzezroczystym `Debug`, test na `Debug` migawki | Wartości to dane pracy: tylko UI, nigdy logi. |

## Scope

**In scope:**
- Pola migawki `filter`, `refreshMinutes`, `nextRefreshAtMs` + lustro TS
- Komenda `open_settings_file` z błędem `ConfigDirUnresolved | Missing | OpenFailed`
- Pasek FILTR nad listą, linia następnej próby w karcie offline, wspólny `formatClock`

**Out of scope:**
- Nowe wywołania `glab`, migracje, ekran lub edycja ustawień w aplikacji
- Oryginalna pisownia wpisów filtra, pasek bez aktywnego filtra
- Zmiana logiki oczekiwania wątku i widoku „Stan źródeł”

## Architecture / Approach

`refresh_round` ustawia w migawce filtr, który ukształtował listę (tym samym cyklem życia co `filter_active`: runda, `reshape_kept`, `act_on_issue`, przeniesienie z listą w `next_snapshot`), oraz flagi rundy `refresh_minutes` i `next_refresh_at_ms` (koniec rundy + oczekiwanie). Widok renderuje `GitlabFilterBar` w `Lists` i `retryText(...)` w karcie `SourceState kind="calm"`, odliczając istniejącym `useNow(RELATIVE_TICK_MS)`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Kontrakt migawki i komenda | Nowe pola migawki, `open_settings_file`, lustro TS, testy | Wartości filtra wyciekające przez `Debug`/logi |
| 2. Pasek FILTR i linia próby | Widoczny pasek i tekst w karcie offline | Długie listy wartości łamiące układ (zawijanie) |

**Prerequisites:** F-01 (`useNow`, `plural`, `Chip` w `src/components/`) — gotowe w `develop`.
**Estimated effort:** ~1 sesja, 2 fazy.

## Open Risks & Assumptions

- Po uśpieniu laptopa oczekiwanie wątku może się przesunąć — UI pokazuje wtedy „lada chwila” zamiast czasu w przeszłości.
- `.json` otwiera się w programie skojarzonym w Windows; to wybór użytkownika.
- Możliwe drobne konflikty z równoległymi slice'ami w `lib.rs` (rejestracja komendy) i w `src/components/index.ts` (osobna linia eksportu).

## Success Criteria (Summary)

- Użytkownik widzi nad listą, jaki filtr działa, i jednym kliknięciem otwiera `settings.json`.
- Bez VPN wie, co ile minut aplikacja próbuje i kiedy następna próba — spokojnie, bez alarmu.
- Wszystkie bramki z AGENTS.md są zielone, a wartości filtra nie trafiają do logów ani testów.
