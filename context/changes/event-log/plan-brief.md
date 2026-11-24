# Dziennik zdarzeń na dashboardzie — Plan Brief

> Full plan: `context/changes/event-log/plan.md`

## What & Why

Makieta Konsoli ma na dashboardzie panel „Dziennik zdarzeń”, którego 0.2.0 nie ma, bo aplikacja nie zapisuje żadnych zdarzeń (FR-019, issue #1, slice S-08). Użytkownik chce w jednym miejscu zobaczyć, co dziś się wydarzyło: nowe przypisania, odhaczone i zdjęte pozycje „Do zrobienia”, START/STOP i awarie źródeł z powrotem.

## Starting Point

Każde z tych zdarzeń już dzieje się w Rust w jednym, znanym miejscu: `todo::set_ticked`/`remove`/`reconcile_*`, `work_time::start`/`stop`, `record_round` w rundzie GitLab i `health::transitions` w rundzie zdrowia. Nic ich nie zapisuje. Store ma 9 migracji, a prawa kolumna dashboardu zawiera tylko „Stan źródeł”.

## Desired End State

Pod „Stanem źródeł” panel pokazuje dzisiejsze zdarzenia od najnowszego, np. `09:14 Nowe przypisanie !421 — tytuł`, `10:02 Odhaczono #12 — tytuł`, `10:30 START`, `11:05 GitHub: błąd — wylogowano`. Aktualizuje się na żywo i o północy zaczyna pusty dzień. Baza trzyma tylko dzisiejsze zdarzenia i tylko to, co panel pokazuje.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Retencja | Tylko dziś; każdy zapis usuwa wiersze z innych dni, odczyt filtruje dziś | Nie trzymamy niczego, czego UI nie pokazuje. |
| Treść zdarzenia | Referencja + tytuł (własne: tytuł); w UI krótka referencja, pełna w podpowiedzi | Czytelne bez pamiętania numerów; `todo_items` już trzyma te same dane. |
| Zakres „Do zrobienia” | Odhaczenie i zdjęcie przez użytkownika + zamknięcie w źródle; bez odznaczenia i dodania | Wyjaśnia każde odhaczenie, bez szumu korekt. |
| Źródła | Tylko błąd, zmiana rodzaju błędu i powrót (`health::transitions`); „niedostępny” cichy | Ta sama reguła co toasty; brak VPN to spokojny stan. |
| Nowe przypisania | Tylko GitLab (`round.new`, runda zasiewająca nic nie zapisuje) | GitHub nie ma wykrywania nowych; dodanie go to osobny slice. |
| Atomowość | Akcje „Do zrobienia” i START/STOP zapisują zdarzenie w tym samym `write` | Akcja i jej wpis są razem albo wcale. |
| Czas i kolejność | Lokalny tekst `YYYY-MM-DDTHH:MM:SS` jak w `todo`, kolejność po `id` | „Dziś” to prefiks daty jak w `roll_over_in`; zmiana zegara nie miesza listy. |
| Odświeżanie UI | Nowe zdarzenie `events-changed` + focus + północ | Wzór `useTodo`, bez drugiej subskrypcji `todo-changed`. |
| Limit | 200 najnowszych, flaga `truncated` i stopka | Panel nigdy nie rośnie bez końca, a ucięcie jest widoczne. |

## Scope

**In scope:** migracja v10 `events`, moduł `src-tauri/src/events/` (zapis, czyszczenie, odczyt, błąd bez wartości), polecenie `list_events_today`, zapis w sześciu miejscach (odhaczenie, zdjęcie, zamknięcie w źródle GitLab/GitHub, START, STOP, nowe przypisanie GitLab, przejścia źródeł), `events-changed`, panel `EventLogPanel` + `useEventLog`, wpis w AGENTS.md, testy integracyjne i crash.

**Out of scope:** nowe przypisania GitHub, odznaczenie, dodanie do listy, przeniesienie dnia, „zrobione” z widoku zadań własnych, edycja wpisów czasu, stany „niedostępny”/„nieskonfigurowany”, historia poprzednich dni, zmiany toastów i artefaktu design systemu.

## Architecture / Approach

`events::record_in(tx, today, now, &NewEvent)` jest wołane wewnątrz istniejących transakcji `todo` i `work_time` (atomowo) albo przez `events::record(store, …)` jako osobny zapis w rundzie GitLab (po `record_round`, w tym samym `with_store`) i w rundzie zdrowia (gdy są przejścia). Każdy zapis czyści inne dni. Po commicie miejsce zapisu emituje `events-changed`. `list_events_today` (async, `spawn_blocking`) zwraca do 200 dzisiejszych zdarzeń; błąd źródła dostaje stałą frazę z `health::short_reason`, nigdy komunikat. UI: lustro typów w `src/events/`, panel w `src/todo/` pod `SourcesPanel`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Rekord zdarzeń (Rust) | Migracja v10, moduł `events`, polecenie odczytu, lustro TS, testy upgrade/crash | Reguła „tylko dziś” przy zmianie zegara i dnia |
| 2. Zapis w miejscach zmian (Rust) | Sześć miejsc zapisu + `events-changed` | Zmiana sygnatur `work_time::start`/`stop` (~30 wywołań w testach); zdarzenie tylko przy faktycznej zmianie stanu |
| 3. Panel i dokumentacja (UI) | `EventLogPanel` z czterema stanami, AGENTS.md | Długie tytuły w wąskiej kolumnie |

**Prerequisites:** F-03 (warstwa vault usunięta) jest w `develop`; v9 zajęte przez S-02, więc ten slice bierze v10 i scala się jako pierwszy z trzech równoległych.
**Estimated effort:** ~1 sesja, 3 fazy z commitem po każdej.

## Open Risks & Assumptions

- Błąd „Dane aplikacji” nie zapisze się, gdy baza jest niedostępna; zapisze się tylko powrót.
- Pozycja z wcześniej ukrytym zgłoszeniem, które wraca jako nowe po 30 dniach zapomnienia, też dostanie „Nowe przypisanie” — zgodnie z toastami.
- Przy każdym starcie aplikacji źródło, które jest w błędzie, da nowy wiersz błędu (pierwsza runda nie ma bazy porównania).

## Success Criteria (Summary)

- Każda z akcji z zakresu daje od razu jeden wiersz w panelu, a odmowy i akcje bez zmiany stanu nie dają żadnego.
- Po północy panel jest pusty, a w bazie nie ma wierszy z innych dni po pierwszym zapisie nowego dnia.
- Log aplikacji i błędy store nie zawierają referencji, tytułów ani komunikatów źródeł.
