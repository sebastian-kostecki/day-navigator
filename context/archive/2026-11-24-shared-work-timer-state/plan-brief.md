# Wspólny stan licznika czasu pracy — Plan Brief

> Full plan: `context/changes/shared-work-timer-state/plan.md`

## What & Why

Nagłówek, dashboard „Do zrobienia” i „Czas pracy” mają dziś każdy własną kopię licznika (`useWorkTimer`): osobny listener `work-time-changed`, osobne odczyty przy focusie, osobny tick co sekundę. Działa, ale to trzy kopie tego samego stanu, a tabela dni (S-05) dołożyłaby czwartą. Robimy jeden wspólny stan licznika w UI, bez zmiany zachowania i bez zmian w Rust. To F-02 z roadmapy (issue #9, MS-09).

## Starting Point

`src/worktime/useWorkTimer.ts` zawiera cały mechanizm (latest-request-wins, subskrypcja przed odczytem, odczyt przy focusie i widoczności, timeout końca dnia, tick, START/STOP, retry) i jest wołany w trzech miejscach. Wzór „Dziś” jest skopiowany trzy razy (nagłówek, `todo/summary.ts`, `WorkTimeView`), a lista wpisów ma czwartą kopię i własny `useNow` co sekundę.

## Desired End State

Jeden `WorkTimerProvider` (kontekst React) w `src/worktime/` owija w `App.tsx` nagłówek i widoki; `useWorkTimer()` zwraca z kontekstu ten sam kształt co dziś. Jeden listener statusu, jeden tick, jedna funkcja „Dziś” w `src/worktime/total.ts`. START/STOP z potwierdzeniem i wszystkie wartości wyglądają jak w 0.2.x. Dwie zamierzone różnice: brak krótkiego „…” w „Dziś” przy przełączaniu widoku oraz wspólny błąd licznika (nagłówek niedostępny = „Dziś” niedostępne wszędzie, do „Spróbuj ponownie”, focusu albo zdarzenia).

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Mechanizm współdzielenia | Kontekst + `WorkTimerProvider` z `children` | Konsumenci prawie bez zmian, tick przerysowuje tylko konsumentów jak dziś, S-05 czyta stan bez propsów; zgodne z issue #9. |
| Hook w `App` + propsy | Odrzucone | `App` przerysowywałby całe drzewo widoków co sekundę. |
| Tick listy wpisów | Lista bierze wspólny `nowMs`, `useNow` znika z „Czasu pracy” | Spełnia „one tick”; kompromis: przy błędzie licznika czas na liście stoi do „Spróbuj ponownie”. |
| Wzór „Dziś” | Jedna funkcja `todayTotalMs` / `liveSpanMs` w `src/worktime/total.ts` | Jedna reguła zamiast czterech kopii; S-05 użyje jej dla wiersza „dziś”. |
| Odczyt przy wejściu na widok | Nie — wystarczą zdarzenie, focus/widoczność, koniec dnia, akcje | Te same źródła prawdy co nagłówek dziś; brak mignięcia „…”. |
| AGENTS.md | Jedno zdanie o `WorkTimerProvider` przy „Navigation” | Pierwszy kontekst w aplikacji; kolejne zmiany nie subskrybują licznika drugi raz. |

## Scope

**In scope:**

- `WorkTimerProvider` i `useWorkTimer()` jako czytnik kontekstu; podpięcie w `App.tsx`
- `total.ts` z `todayTotalMs` i `liveSpanMs`; przepięcie nagłówka, `summary.ts`, `WorkTimeView`, `WorkTimeEntries`
- Wspólny `nowMs` dla listy wpisów; komentarz w `TodoView`; zdanie w AGENTS.md

**Out of scope:**

- Rust (komendy, zdarzenie, typy, testy)
- Listenery `useMonthReport` i `useWorkEntries`; `useNow(RELATIVE_TICK_MS)` w innych widokach
- Wygląd i zachowanie START/STOP oraz potwierdzenia; tabela dni (S-05)

## Architecture / Approach

`App` → `WorkTimerProvider` (woła raz `useWorkTimerSource()`: listener, focus, koniec dnia, tick, akcje; wartość w `useMemo`) → `children`: `AppHeader` z `WorkTimerHeader` oraz aktywny widok. `WorkTimerHeader`, `TodoView` i `WorkTimeView` wołają `useWorkTimer()` = `use(WorkTimerContext)`. Sumy liczą czyste funkcje z `total.ts`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Wspólny stan licznika | Provider + kontekst, jeden listener i tick; konsumenci bez zmian; AGENTS.md | Przeniesienie logiki hooka bez utraty ochrony przed StrictMode i utraconym zdarzeniem |
| 2. Jedna reguła „Dziś” i jeden tick | `total.ts` w czterech miejscach; lista wpisów na wspólnym `nowMs` | Rozjazd sum lub lista, która przestaje tykać |

**Prerequisites:** F-01 (`useNow` w `src/components/`) — done. Brak zmian w Rust.
**Estimated effort:** ~1 sesja, 2 małe fazy, ręczna kontrola po każdej.

## Open Risks & Assumptions

- Zakładamy, że `runningFromMs` w statusie i w `DayEntries` jest ustawione tylko przy działającym liczniku (`src-tauri/src/work_time/mod.rs:189`); na tym opiera się wspólny tick listy wpisów.
- Wspólny stan sprzęga błąd: gdy licznik jest niedostępny, „Dziś” jest niedostępne w trzech miejscach naraz, a jedyny retry jest w nagłówku (dashboard i „Czas pracy” już dziś nie mają własnego retry licznika).

## Success Criteria (Summary)

- START/STOP z potwierdzeniem działa dokładnie jak w 0.2.x (sprawdzone ręcznie po każdej fazie).
- Nagłówek, dashboard i „Czas pracy” pokazują tę samą sumę „Dziś”, a lista wpisów tyka razem z nagłówkiem.
- W kodzie jeden listener statusu licznika, jeden tick i jeden wzór sumy; `src-tauri/` bez zmian.
