# GitLab — flaga NOWE i data ukrycia — Plan Brief

> Full plan: `context/changes/gitlab-new-and-hidden-date/plan.md`

## What & Why

W „Praca (GitLab)” chcemy widzieć, co przyszło od ostatniego sprawdzenia (flaga NOWE), i od kiedy wiersz jest ukryty („ukryte 18.11”), tak jak na makiecie Konsoli. Zamyka to FR-023 / issue #5 (S-02, M-2).

## Starting Point

Baza ma już oba czasy: `hidden_issues.hidden_at_ms` (S-03) i `seen_assignments.first_seen_ms` (S-04). Żaden z nich nie trafia do UI. `record_round` liczy „nowe” tylko dla toastów i dopiero po publikacji snapshotu. Nie istnieje znacznik „widok oglądany do”.

## Desired End State

MR-y i widoczne zgłoszenia pierwszy raz widziane po ostatnim wyjściu z widoku mają niebieską flagę NOWE przez całą wizytę. Po wyjściu z widoku albo schowaniu okna flagi gasną. Ukryte wiersze pokazują „ukryte DD.MM” (z rokiem, gdy inny niż bieżący). Flagi przetrwają rundę bez VPN i komendy ukrywania.

## Key Decisions Made

| Decision                    | Choice                                                         | Why (1 sentence)                                                                                         |
| --------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| „Ostatnie sprawdzenie”      | Ostatnia wizyta w widoku, znacznik per konto w bazie (v9)      | „Ostatni refresh” gasi flagę, zanim ją zobaczysz, bo wejście w widok samo odświeża listę.                 |
| Koniec wizyty               | Wyjście z widoku albo schowanie okna przy aktywnym widoku      | NOWE gaśnie dopiero po obejrzeniu; po awarii flagi wracają (bezpieczna strona).                           |
| Pierwsza wizyta bez znacznika | Nic nie jest NOWE                                            | Brak fali flag na starych pozycjach po aktualizacji, jak ciche zasianie S-04.                            |
| Data ukrycia                | Szary tekst „ukryte DD.MM” za tytułem                          | Widać datę ukrycia i datę zmiany obok siebie, bez kolejnej flagi.                                        |
| Reguła NOWE                 | `first_seen_ms > seen_until_ms`, liczona w Rust w `record_round` | Reguły domenowe żyją w Rust z testami; jedna transakcja, spójna z zapisem rundy.                       |
| Wartość znacznika           | Czas ostatniej pokazanej rundy (`shown_until`), nie zegar      | Pozycja z rundy w toku nie zgaśnie, zanim ją zobaczysz (plan review F1).                                  |
| Gdzie wykryć wyjście        | Zmiana aktywnego widoku w `App.tsx` + zdarzenie `main-hidden`  | Cleanup efektu nie działa w StrictMode, a chowanie okna nie odmontowuje widoku.                          |
| Ton flagi                   | Nowy `tone="new"` na `--info` / `--info-line`                  | Tak definiuje go design system Konsola.                                                                  |

## Scope

**In scope:**
- Migracja v9 `gitlab_view_marks` (nieprzezroczyste konto i jeden czas)
- `isNew` i `hiddenAtMs` w `GitlabItem` (Rust + TS)
- Przestawienie `record_round` przed zapis snapshotu, `kept_new` w stanie odświeżania
- Polecenie `mark_gitlab_seen`, zdarzenie `main-hidden`
- Flaga NOWE, „ukryte DD.MM”, `formatDay`, dopisek w AGENTS.md

**Out of scope:**
- NOWE w „Do zrobienia”, GitHubie i bloku „Zamknięte”
- Ręczne „oznacz jako przejrzane”
- Zmiany toastów S-04 i semantyki `first_seen_ms`
- Synchronizacja artefaktu design systemu (F-05)

## Architecture / Approach

Runda: `fetch` → kontrole komentarzy → **`record_round` (zbiór widzianych + świeże klucze wobec znacznika)** → bramka → `partition` (data ukrycia) → `mark_fresh` (NOWE) → filtr → snapshot → publikacja → toasty. Świeże klucze leżą w `kept_new`, więc nieudana runda i komendy ukrywania nakładają te same flagi. `mark_gitlab_seen` pod bramką zapisuje `max(znacznik, shown_until)`, czyli czas ostatniej pokazanej rundy, nie zegar, więc nie gasi pozycji, których nie widziałeś. Potem czyści flagi, zapamiętuje `last_mark` (odcina spóźnioną rundę) i publikuje. UI woła je przy zmianie widoku z `gitlab` i przy `main-hidden`.

## Phases at a Glance

| Phase                          | What it delivers                                              | Key risk                                                       |
| ------------------------------ | ------------------------------------------------------------- | -------------------------------------------------------------- |
| 1. Dane i reguły (Rust)        | v9, `fresh_since`, `RoundSeen`, `mark_view_seen`, pola itemu, data w `partition` | Mechaniczne dopisanie pól w 13 plikach z konstruktorami `GitlabItem` |
| 2. Runda i polecenie (Rust)    | Kolejność rundy, `kept_new`, `mark_gitlab_seen`, `main-hidden` | Wyścig rundy z wyjściem z widoku (rozwiązany przez `shown_until` + `last_mark`) |
| 3. UI i dokumentacja           | Flaga NOWE, „ukryte DD.MM”, koniec wizyty, AGENTS.md          | Wykrycie wyjścia z widoku odporne na StrictMode                |

**Prerequisites:** brak; S-02 nie zależy od innych slice'ów. Migracja v9 ląduje przed migracją S-08 (`event-log`), która będzie v10.
**Estimated effort:** ~2 sesje w 3 fazach.

## Open Risks & Assumptions

- Zakładamy, że `window.emit` w `CloseRequested` dociera do webview przed jego uśpieniem. Jeśli nie, flagi pokażą się jeszcze raz przy następnej wizycie, co jest bezpieczną stroną błędu.
- Wyjście z aplikacji z zasobnika przy aktywnym widoku GitLab nie zapisuje znacznika. To świadome: flagi pokażą się jeszcze raz.
- Ponowne ukrycie zgłoszenia nadpisuje `hidden_at_ms`, więc data to ostatnie ukrycie (zachowanie S-03 bez zmian).

## Success Criteria (Summary)

- Pozycja przypisana, gdy nie patrzysz na GitLaba, ma NOWE przy następnej wizycie i traci je po wyjściu z widoku.
- Ukryte wiersze pokazują dzień ukrycia.
- Bez VPN i po ukryciu innego zgłoszenia flagi zostają, a w bazie przybywa tylko nieprzezroczysty znacznik konta.
