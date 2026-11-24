# Wspólny stan licznika czasu pracy — plan implementacji

## Overview

Nagłówek, dashboard „Do zrobienia” i widok „Czas pracy” wołają dziś `useWorkTimer()` każdy osobno, więc każdy ma własny listener `work-time-changed`, własne odczyty przy focusie i widoczności, własny timeout końca dnia i własny tick co sekundę. Zastępujemy to jednym `WorkTimerProvider` (kontekst React) wokół nagłówka i widoków; `useWorkTimer()` czyta ten kontekst. Przy okazji trzy kopie wzoru „Dziś” i czwarta kopia na liście wpisów stają się jedną funkcją, a lista wpisów bierze wspólny `nowMs` zamiast własnego `useNow`. To F-02 z roadmapy (issue #9, MS-09); odblokowuje S-05 (tabela dni), którego wiersz „dziś” przeczyta wspólny stan zamiast czwartej subskrypcji. Komendy Rust i zachowanie START/STOP z potwierdzeniem się nie zmieniają.

## Current State Analysis

- `src/worktime/useWorkTimer.ts:22-163` — cały mechanizm licznika w jednym hooku: stan `TimerState` (`loading | error | loaded`), `nowMs`, `busy`, `actionError`; ochrona „latest request wins” (`issued`/`applied`, `:33-43`); subskrypcja `work-time-changed` przed pierwszym odczytem plus odświeżanie przy `visibilitychange` i `focus` (`:63-100`); timeout na koniec dnia (`:106-111`); tick co 1 s tylko gdy licznik działa, z zapasowym odczytem po końcu dnia (`:116-132`); `start`/`stop` z obsługą `AlreadyRunning`/`NotRunning` (`:134-157`); `retry` przez licznik `attempt` (`:160`).
- Trzy wywołania: `src/worktime/WorkTimerHeader.tsx:15` (wszystkie pola), `src/todo/TodoView.tsx:151` (`timer`, `nowMs` dla kafla „Dziś”), `src/worktime/WorkTimeView.tsx:87` (`timer`, `nowMs` dla statystyki „Dziś”). Nagłówek jest zamontowany zawsze (`src/App.tsx:67`), widoki naprzemiennie (`src/App.tsx:69-74`), więc naraz działają dwie kopie mechanizmu.
- Wzór „Dziś” (zamknięty czas + bieżący odcinek ucięty do końca dnia, nie mniej niż 0) jest skopiowany trzy razy: `src/worktime/WorkTimerHeader.tsx:69-73`, `src/todo/summary.ts:16-20`, `src/worktime/WorkTimeView.tsx:45-49`. Ten sam rdzeń dla `DayEntries` siedzi w `src/worktime/WorkTimeEntries.tsx:193-197` (`liveRunningMs`).
- „Czas pracy” ma drugi tick: `useNow(ticking ? 1000 : null)` (`src/worktime/WorkTimeView.tsx:97-103`), włączany przez `entries.data.runningFromMs`, używany tylko przez listę wpisów (`EntriesPanel`, `:199`).
- W Rust `running_from_ms` jest `Some` dokładnie wtedy, gdy licznik działa (`src-tauri/src/work_time/mod.rs:189`), więc warunek `running !== null && runningFromMs !== null` w trzech kopiach jest równoważny samemu `runningFromMs !== null`.
- Aplikacja działa w `React.StrictMode` (`src/main.tsx:8`); dzisiejszy hook jest na to odporny (flaga `cancelled`, odpięcie listenera zarejestrowanego po odmontowaniu, `:64-99`).
- `useMonthReport` i `useWorkEntries` też słuchają `work-time-changed`, ale to inne dane (raport miesiąca, wpisy dnia).
- UI nie ma runnera testów (AGENTS.md → Testing); weryfikacja to bramki lint/format/build, wyszukiwania w kodzie i ręczna kontrola w aplikacji.

## Desired End State

W całym UI działa jeden mechanizm licznika: `WorkTimerProvider` w `src/worktime/` owija w `App.tsx` nagłówek i widoki, a `useWorkTimer()` zwraca z kontekstu ten sam kształt co dziś (`timer`, `nowMs`, `busy`, `actionError`, `start`, `stop`, `retry`). W źródłach jest jedno miejsce, które subskrybuje `work-time-changed` dla statusu licznika, i jeden tick co sekundę dla „Dziś” i listy wpisów. Wzór „Dziś” i bieżący odcinek liczy jedna funkcja w `src/worktime/`. Nagłówek z potwierdzeniem START/STOP, kafel „Dziś” na dashboardzie, statystyka „Dziś” i lista wpisów w „Czasie pracy” pokazują te same wartości co w 0.2.x. Dwie zamierzone, widoczne różnice: (1) przełączenie widoku nie pokazuje już na chwilę „…” w „Dziś”, bo wartość jest gotowa; (2) błąd licznika jest wspólny — gdy nagłówek pokazuje „Czas pracy niedostępny”, kafel „Dziś” na dashboardzie i statystyka w „Czasie pracy” też są niedostępne, a wejście na widok nie robi własnej próby; stan wraca po „Spróbuj ponownie”, focusie okna albo zdarzeniu.

Weryfikacja: pełna bramka z AGENTS.md przechodzi, `src-tauri/` bez zmian, wyszukiwania z kryteriów sukcesu znajdują jeden listener i jeden wzór, a ręczna kontrola START/STOP i wartości w trzech miejscach przechodzi.

### Key Discoveries:

- Kontekst z `children` przerysowuje co sekundę tylko konsumentów (nagłówek i aktywny widok) — dokładnie tyle, co dziś; hook wołany w `App` przerysowywałby całe drzewo (GitLab, GitHub, „Moje zadania”) co sekundę.
- Konsumenci już destrukturyzują wynik `useWorkTimer()`, a `TimerState` jest importowany z `./useWorkTimer` (`src/todo/summary.ts:5`, `src/worktime/WorkTimeView.tsx:9`) — zachowanie nazwy pliku, eksportów i kształtu zwrotu oznacza zero zmian w konsumentach w fazie 1.
- Lista wpisów pokazuje bieżący odcinek tylko gdy `DayEntries.runningFromMs !== null`, a to w Rust oznacza działający licznik — tick licznika pokrywa wszystkie przypadki, w których lista dziś tyka.
- React 19: kontekst renderowany jako `<Context value={…}>`, odczyt przez `use(Context)`.

## What We're NOT Doing

- Żadnych zmian w Rust: komendy, zdarzenie `work-time-changed`, typy i testy w `src-tauri/` zostają.
- Nie łączymy listenerów `useMonthReport` i `useWorkEntries` (inne dane; poza MS-09).
- Nie zmieniamy `useNow(RELATIVE_TICK_MS)` w innych widokach (etykiety „N min temu”, osobny cel).
- Nie zmieniamy wyglądu, tekstów ani zachowania START/STOP, potwierdzenia (pytanie, „TAK, START/STOP”, „Anuluj” z fokusem, 400 ms ochrony, 6 s, Escape), błędu akcji i „Spróbuj ponownie”.
- Nie dodajemy odczytu statusu przy wejściu na widok — wspólny stan odświeża się jak dziś (zdarzenie, focus/widoczność, koniec dnia, akcje).
- Nie wprowadzamy biblioteki stanu ani routera; kontekst służy tylko licznikowi.
- Nie budujemy tabeli dni (S-05).

## Implementation Approach

Faza 1 to czysty refaktor: logika hooka przechodzi bez zmian do jednego źródła stanu wołanego w Providerze, a `useWorkTimer()` staje się czytnikiem kontekstu o tym samym kształcie. Konsumenci zostają nietknięci, więc każda różnica w zachowaniu po fazie 1 wskazuje na Provider. Faza 2 sprząta to, co wspólny stan umożliwia: jedna funkcja „Dziś”/bieżący odcinek oraz jeden tick dla listy wpisów.

## Critical Implementation Details

**Timing & lifecycle** — Treść dzisiejszego hooka przenosimy 1:1 (kolejność subskrypcja → odczyt, flaga `cancelled`, `attempt` w zależnościach efektu, timeout końca dnia, `refetchedFor`); nie upraszczamy jej przy przenosinach, bo chroni przed utratą zdarzenia, podwójnym montażem w StrictMode i późnym timerem po północy.

**Performance constraints** — Provider musi dostawać nagłówek i widoki jako `children`, a `App` nie może sam wołać `useWorkTimer()`; inaczej tick co sekundę przerysuje całe drzewo widoków.

## Phase 1: Wspólny stan licznika

### Overview

Jeden mechanizm licznika w Providerze, `useWorkTimer()` czyta kontekst, `App.tsx` owija nagłówek i widoki. Konsumenci bez zmian w kodzie.

### Changes Required:

#### 1. Źródło stanu i kontekst

**File**: `src/worktime/useWorkTimer.ts`

**Intent**: Dzisiejsza treść hooka staje się źródłem stanu wołanym tylko przez Provider, a `useWorkTimer()` zwraca wartość z kontekstu, żeby trzy miejsca korzystały z jednego listenera i jednego ticka.

**Contract**: Eksporty: `TimerState` (bez zmian), `WorkTimer` (typ zwrotu: `{ timer, nowMs, busy, actionError, start, stop, retry }` — dokładnie dzisiejszy kształt), `useWorkTimerSource(): WorkTimer` (dzisiejsza treść `useWorkTimer`, bez zmian w logice), `WorkTimerContext` (`createContext<WorkTimer | null>(null)`), `useWorkTimer(): WorkTimer` — `use(WorkTimerContext)`, a bez Providera rzuca `Error` z jasnym komunikatem (błąd programisty, nie stan UI). Jedyne `listen(WORK_TIME_CHANGED, …)` dla statusu licznika zostaje w `useWorkTimerSource`.

#### 2. Provider

**File**: `src/worktime/WorkTimerProvider.tsx` (nowy)

**Intent**: Komponent, który raz woła źródło stanu i udostępnia jego wynik przez kontekst wszystkim potomkom.

**Contract**: `export function WorkTimerProvider({ children }: { children: ReactNode })`; wartość kontekstu to wynik `useWorkTimerSource()` opakowany w `useMemo` po swoich siedmiu polach (nowy obiekt tylko gdy któreś pole się zmieni); render `<WorkTimerContext value={value}>{children}</WorkTimerContext>`.

#### 3. Podpięcie w App

**File**: `src/App.tsx`

**Intent**: Provider owija `AppHeader` (z `WorkTimerHeader`) i wszystkie widoki, żeby nagłówek, dashboard i „Czas pracy” czytały ten sam stan.

**Contract**: `WorkTimerProvider` jest najbliższym rodzicem obecnego fragmentu (`src/App.tsx:57-75`); `App` nie woła `useWorkTimer()`. Nawigacja i skróty klawiszowe bez zmian.

#### 4. Komentarz dashboardu

**File**: `src/todo/TodoView.tsx`

**Intent**: Komentarz przy `useWorkTimer()` (`:149-151`) przestaje sugerować własne wywołania statusu i listener; mówi, że licznik pochodzi ze wspólnego stanu, a START/STOP i „Sprawdź teraz” zostają w nagłówku i „Źródłach”.

**Contract**: Tylko komentarz; kod bez zmian.

#### 5. Konwencja w AGENTS.md

**File**: `AGENTS.md`

**Intent**: Plan wprowadza pierwszy kontekst React w aplikacji; AGENTS.md ma to nazwać, żeby kolejne zmiany (S-05) czytały wspólny stan zamiast subskrybować licznik po raz kolejny.

**Contract**: W `## Frontend Conventions`, przy punkcie „Navigation”, jedno zdanie: stan licznika to jeden `WorkTimerProvider` (kontekst React, `src/worktime/`) wokół nagłówka i widoków; czytaj go przez `useWorkTimer()` i nie subskrybuj statusu licznika drugi raz.

### Success Criteria:

#### Automated Verification:

- Lint, format i build przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- Bramka Rust przechodzi bez zmian w `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` w `src-tauri/` oraz `git diff --stat $(git merge-base HEAD develop) -- src-tauri` puste
- Jeden listener statusu licznika: `rg -c "listen\(WORK_TIME_CHANGED" src/worktime/useWorkTimer.ts` daje `1`, a `rg -n "useWorkTimerSource\(" src` poza definicją ma jedno wywołanie, w `src/worktime/WorkTimerProvider.tsx`
- Provider podpięty raz: `rg -n "<WorkTimerProvider" src` ma jedno trafienie, w `src/App.tsx`

#### Manual Verification:

- START i STOP w nagłówku (`npm run tauri dev`) działają jak w 0.2.x: klik START pokazuje „Rozpocząć pracę?” z „TAK, START” i „Anuluj” z fokusem; „Anuluj”, Escape i 6 s bez reakcji zamykają pytanie bez zmiany licznika; drugi klik podwójnego kliknięcia ląduje na „Anuluj” i nic nie startuje; „TAK, START” włącza REC i zegar tyka; STOP analogicznie z „Zakończyć pracę?” i „TAK, STOP”
- Przy działającym liczniku kafel „Dziś” na dashboardzie i statystyka „Dziś” w „Czasie pracy” rosną razem z nagłówkiem; przełączanie widoków (klawisze 1–6) nie zeruje ani nie cofa wartości
- Zmiana spoza nagłówka dociera wszędzie: dodanie lub edycja wpisu dzisiejszego dnia w „Czasie pracy” zmienia nagłówek i kafel „Dziś” bez odświeżania okna; po zminimalizowaniu i przywróceniu okna nagłówek pokazuje aktualną wartość

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Jedna reguła „Dziś” i jeden tick

### Overview

Jedna funkcja liczy bieżący odcinek i sumę „Dziś”; nagłówek, dashboard, „Czas pracy” i lista wpisów jej używają, a lista wpisów bierze wspólny `nowMs` zamiast własnego `useNow`.

### Changes Required:

#### 1. Funkcje sumy

**File**: `src/worktime/total.ts` (nowy)

**Intent**: Jedno miejsce na regułę „bieżący odcinek ucięty do końca dnia, nie mniej niż 0” i sumę „Dziś”, żeby cztery kopie (i przyszły wiersz „dziś” w S-05) liczyły tak samo.

**Contract**: `liveSpanMs(runningFromMs: number | null, dayEndMs: number, nowMs: number): number` — `0` dla `null`, w przeciwnym razie `Math.max(0, Math.min(nowMs, dayEndMs) - runningFromMs)`. `todayTotalMs(status: WorkTimerStatus, nowMs: number): number` — `status.todayClosedMs + liveSpanMs(status.running === null ? null : status.runningFromMs, status.dayEndMs, nowMs)` (warunek na `running` zachowuje dzisiejszą semantykę 1:1). Czyste funkcje, bez Reacta.

#### 2. Przepięcie sum

**File**: `src/worktime/WorkTimerHeader.tsx`, `src/todo/summary.ts`, `src/worktime/WorkTimeView.tsx`, `src/worktime/WorkTimeEntries.tsx`

**Intent**: Lokalne kopie wzoru zastępujemy funkcjami z `total.ts`; formatowanie i teksty zostają.

**Contract**: Nagłówek (`:67-73`) liczy `total` przez `todayTotalMs`; `todayTile` w `summary.ts` (`:13-20`) i `todayStat` w `WorkTimeView.tsx` (`:44-49`) też; `liveRunningMs` w `WorkTimeEntries.tsx` (`:193-197`) znika, a oba jego wywołania (`:213`, `:296`) wołają `liveSpanMs(data.runningFromMs, data.dayEndMs, nowMs)`. Komentarze „as in WorkTimerHeader” zastępuje odwołanie do `total.ts`.

#### 3. Jeden tick w „Czasie pracy”

**File**: `src/worktime/WorkTimeView.tsx`

**Intent**: Lista wpisów dostaje `nowMs` ze wspólnego stanu, więc w „Czasie pracy” nie ma drugiego ticka, a nagłówek i lista pokazują tę samą sekundę.

**Contract**: `const { timer, nowMs } = useWorkTimer()`; `nowMs` idzie do `todayStat` i do `EntriesPanel`; `ticking`, `useNow` i komentarz `:97-103` znikają (import `useNow` usunięty). Uzgodniony kompromis: gdy licznik jest w stanie błędu, a lista ma trwający wpis, czas na liście stoi do „Spróbuj ponownie” w nagłówku.

### Success Criteria:

#### Automated Verification:

- Lint, format i build przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- Bramka Rust przechodzi bez zmian w `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` w `src-tauri/` oraz `git diff --stat $(git merge-base HEAD develop) -- src-tauri` puste
- Jeden wzór: `rg -n "Math.min\(nowMs" src` ma jedno trafienie, w `src/worktime/total.ts`
- Jeden tick w „Czasie pracy”: `rg -n "useNow" src/worktime` nic nie znajduje

#### Manual Verification:

- START i STOP z potwierdzeniem działają jak po fazie 1 (pytanie, „TAK, START/STOP”, „Anuluj” z fokusem, Escape, 6 s, podwójny klik)
- Przy działającym liczniku nagłówek (HH:MM:SS), kafel „Dziś” na dashboardzie i statystyka „Dziś” w „Czasie pracy” (H:MM) pokazują tę samą sumę; po STOP wszystkie trzy stoją na tej samej wartości
- W „Czasie pracy” na dzisiejszym dniu czas trwającego wpisu i suma dnia na liście tykają co sekundę razem z nagłówkiem; po STOP przestają; na poprzednim dniu lista nie tyka, a sumy są takie jak przed zmianą

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Brak: UI nie ma runnera testów (AGENTS.md → Testing), a reguły domeny (norma, odchylenie) zostają w Rust i ich testy się nie zmieniają. `total.ts` to arytmetyka wyświetlania przeniesiona bez zmian.

### Integration Tests:

- `cargo test` w `src-tauri/` jako dowód, że kontrakt komend i zdarzenia jest nietknięty.

### Manual Testing Steps:

1. `npm run tauri dev`; w nagłówku START → sprawdź pytanie, fokus na „Anuluj”, Escape, 6 s, podwójny klik; potem „TAK, START”.
2. Przy działającym liczniku przejdź klawiszami 1 i 5 między dashboardem a „Czasem pracy”; porównaj „Dziś” w trzech miejscach.
3. W „Czasie pracy” dodaj lub popraw wpis z dzisiaj; sprawdź nagłówek i kafel „Dziś”.
4. Zminimalizuj okno, przywróć; sprawdź nagłówek.
5. STOP z potwierdzeniem; sprawdź, że trzy wartości stoją, a lista wpisów przestała tykać; przejdź na poprzedni dzień.

## Performance Considerations

Dziś przy działającym liczniku tykają dwa interwały (nagłówek i aktywny widok z licznikiem), a w „Czasie pracy” trzy (z `useNow`). Po zmianie jest jeden. Liczba przerysowań co sekundę się nie zwiększa: tick przerysowuje tylko konsumentów kontekstu (nagłówek i aktywny widok), bo `App` nie czyta stanu licznika.

## Migration Notes

Brak danych i ustawień do migracji. Wycofanie to revert commitów fazy 2, potem fazy 1; `src-tauri/` nie jest dotykane.

## References

- Roadmap: `context/foundation/roadmap.md` (F-02, MS-09; S-05 jako konsument)
- Issue: GitHub #9 „Wspólny stan licznika czasu pracy w UI”
- Plan licznika, który ustalił kształt hooka: `context/archive/2026-11-23-work-timer/plan.md:234-239`
- Wspólne narzędzia UI (`useNow`): `context/archive/2026-11-24-shared-ui-tools/plan.md`
- Kod: `src/worktime/useWorkTimer.ts:22-163`, `src/worktime/WorkTimerHeader.tsx:14-123`, `src/todo/TodoView.tsx:149-151`, `src/todo/summary.ts:8-26`, `src/worktime/WorkTimeView.tsx:39-58,87-103`, `src/worktime/WorkTimeEntries.tsx:193-197`, `src/App.tsx:56-76`, `src-tauri/src/work_time/mod.rs:189`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Wspólny stan licznika

#### Automated

- [x] 1.1 Lint, format i build przechodzą — b90031c
- [x] 1.2 Bramka Rust przechodzi bez zmian w `src-tauri/` — b90031c
- [x] 1.3 Jeden listener statusu licznika — b90031c
- [x] 1.4 Provider podpięty raz — b90031c

#### Manual

- [x] 1.5 START i STOP z potwierdzeniem działają jak w 0.2.x
- [x] 1.6 „Dziś” na dashboardzie i w „Czasie pracy” rośnie z nagłówkiem, przełączanie widoków nie zeruje
- [x] 1.7 Zmiana spoza nagłówka i powrót okna docierają wszędzie

### Phase 2: Jedna reguła „Dziś” i jeden tick

#### Automated

- [x] 2.1 Lint, format i build przechodzą — 004aa9f
- [x] 2.2 Bramka Rust przechodzi bez zmian w `src-tauri/` — 004aa9f
- [x] 2.3 Jeden wzór sumy w `total.ts` — 004aa9f
- [x] 2.4 Jeden tick w „Czasie pracy” — 004aa9f

#### Manual

- [x] 2.5 START i STOP z potwierdzeniem działają jak po fazie 1
- [x] 2.6 Nagłówek, dashboard i „Czas pracy” pokazują tę samą sumę „Dziś”
- [x] 2.7 Lista wpisów tyka z nagłówkiem dziś, nie tyka na poprzednim dniu
