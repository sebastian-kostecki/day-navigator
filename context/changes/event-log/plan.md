# Dziennik zdarzeń na dashboardzie — plan implementacji

## Overview

Na dashboardzie („Do zrobienia”), w prawej kolumnie pod „Stanem źródeł”, pojawia się panel „Dziennik zdarzeń” z dzisiejszymi zdarzeniami, od najnowszego: nowe przypisanie w GitLabie, odhaczenie pozycji „Do zrobienia”, zdjęcie pozycji, zamknięcie pozycji w źródle, START i STOP licznika, przejście źródła w błąd i powrót. Zamyka to FR-019 / issue #1 (slice S-08, M-2). Zdarzenia zapisuje Rust w nowej tabeli `events` (migracja v10) w tym samym miejscu, w którym dzieje się sama zmiana. UI tylko je czyta i wyświetla.

## Current State Analysis

- Store: ostatnia migracja to v9 `gitlab_view_marks` (`src-tauri/src/store/schema.rs:209-220`); `validate()` wymaga numerów rosnących o 1 (`schema.rs:224-235`). Wzorzec modułu rekordu: `src-tauri/src/new_assignments/` (funkcje `fn(&mut Store, …)` bez `tauri` i bez zegara, `error.rs` z `Store(StoreError)` + `Background`, serde `tag="kind", content="detail"`). Czyszczenie starych wierszy w tym samym `write`: `hidden_issues/mod.rs:234-237`, `todo::roll_over_in` (`todo/mod.rs:122`, „inny dzień niż dziś” przez `substr(ts, 1, 10) <> today`, więc obejmuje też zegar cofnięty).
- „Do zrobienia”: `todo::set_ticked` (`todo/mod.rs:428`) i `todo::remove` (`:468`) to jeden `store.write` każdy, dostają `today: NaiveDate` i lokalne `now: NaiveDateTime`. Znacznik czasu to lokalny tekst `YYYY-MM-DDTHH:MM:SS` (`stamp`, `todo/mod.rs:90`). `set_ticked` czyta dziś tylko `own_task_id, ticked_at`; referencja i tytuł są w `todo_items` (`gitlab_ref`/`gitlab_title`, `github_ref`/`github_title`) albo w `own_tasks.title`. Polecenia idą przez `run` w `todo/commands.rs:79`, które po udanej mutacji emituje `todo-changed` i loguje tylko rodzaj błędu.
- Zamknięcie w źródle: `reconcile_gitlab` (`todo/mod.rs:497`) i `reconcile_github` (`:575`) w pętli mają w ręku `id`, referencję i tytuł wiersza, który odhaczają z `ticked_by = 'source'`. Wołane z `gitlab_refresh.rs:810` i `github_refresh.rs:234`, które emitują `todo-changed`, gdy coś się zmieniło.
- Licznik: `work_time::start` / `stop` (`work_time/mod.rs:249`, `:266`) to jeden `store.write`, dostają `now: i64` (UTC ms). Konwencja modułu dla czasu lokalnego to parametr `tz: &Tz` (`add`, `status`). Polecenia `start_work` / `stop_work` w `work_time/commands.rs:71-85` emitują `work-time-changed`. Nic nie zatrzymuje licznika automatycznie.
- Nowe przypisania: tylko GitLab. `record_seen` (`gitlab_refresh.rs:761`) woła `new_assignments::record_round` w `with_store`; `round.new` to klucze naprawdę nowe (runda zasiewająca daje pusto). `items_for(full, keys)` (`new_assignments/mod.rs:73`) zwraca `&GitlabItem` z `kind`, `reference` (pełna, `grupa/projekt!421`) i `title`. GitHub nie ma wykrywania nowych przypisań.
- Źródła: `check_round` (`runtime.rs:293`) liczy `health::transitions(baseline, &sources)` (`health/mod.rs:140`): wejście w błąd, zmiana rodzaju błędu, powrót Error→Ok. `Unavailable` (GitLab bez VPN) i `NotConfigured` są ciche. `Transition { id, label, from_kind, to_kind }` nie niesie treści komunikatu. Stałe polskie frazy rodzajów błędów: `short_reason` (`health/mod.rs:180`, dziś prywatna). Klucze źródeł: `settings`, `app-data`, `gitlab`, `github`, `reminders` (`SourceId::key`, `health/mod.rs:36`).
- UI: `TodoView.tsx:484-492` — kolumna `todo-col-side` (flex column z `gap`) zawiera tylko `<SourcesPanel/>` (`src/todo/SourcesPanel.tsx`, `Panel title="Stan źródeł"`, wiersze `todo-source` w `TodoView.css:52-95`). Krótka referencja wiersza: `shortRef` (`TodoView.tsx:94`, lokalna). Hooki subskrybują przed ładowaniem i przeładowują na focus, `visibilitychange` i o północy (`src/todo/useTodo.ts:33-94`). `ErrorState`, `Notice`, `Panel`, `useNow`, `plural` w `src/components/`.
- Test `tests/new_assignments.rs:229` ma na sztywno `assert_eq!(user_version(&store), 9)` i po v10 by padł.

## Desired End State

- Pod „Stanem źródeł” jest panel „Dziennik zdarzeń”. Każdy wiersz: `HH:MM`, etykieta i przedmiot, od najnowszego:
  - `Nowe przypisanie` + `!421 — tytuł` (MR albo zgłoszenie z GitLaba, pierwszy raz widziane w rundzie po zasianiu),
  - `Odhaczono` + `!421 — tytuł` / `#12 — tytuł` / tytuł zadania własnego (użytkownik odhaczył pozycję „Do zrobienia”),
  - `Zdjęto` + przedmiot (użytkownik zdjął pozycję z listy),
  - `Zamknięte w źródle` + przedmiot (runda GitLab/GitHub odhaczyła pozycję, bo zniknęła ze źródła),
  - `START` / `STOP` (licznik czasu pracy),
  - `<źródło>: błąd — <stała fraza rodzaju>` (np. „GitHub: błąd — wylogowano”) i `<źródło>: znowu działa`.
- Pełna referencja jest w podpowiedzi (`title`) przy krótkiej. Tytuł ucięty wielokropkiem w jednej linii.
- Panel ma stany: ładowanie, błąd (`ErrorState` z czasem ostatniego udanego odczytu i „Spróbuj ponownie”), pusto („Brak zdarzeń dzisiaj.”) i lista. Nieudany ponowny odczyt zostawia listę i pokazuje `Notice` z błędem. Przy więcej niż 200 zdarzeniach dnia panel pokazuje 200 najnowszych i stopkę „Pokazano 200 najnowszych”.
- Panel aktualizuje się na żywo (`events-changed`), na focus okna i o północy pokazuje pusty nowy dzień.
- W bazie są tylko zdarzenia dzisiejszego dnia (czas lokalny): każdy zapis zdarzenia usuwa wiersze z innych dni (także „przyszłych” po cofnięciu zegara), a odczyt zwraca tylko dzisiejsze.
- Akcja „Do zrobienia” i START/STOP zapisują się razem ze swoim zdarzeniem albo wcale (jeden `write`). Zdarzenia przypisań i źródeł są osobnym zapisem; jego błąd nie psuje rundy i trafia do logu tylko jako rodzaj błędu.
- Log aplikacji nie dostaje żadnej referencji, tytułu, etykiety źródła ani komunikatu ze zdarzeń.

Weryfikacja: kontrole sprzed commita z `AGENTS.md` oraz ręczny przebieg w `npm run tauri dev`.

### Key Discoveries:

- `src-tauri/src/todo/mod.rs:535-543` i `:612-625`: gałąź `None =>` w pętlach `reconcile_*` to jedyne miejsce zamknięcia w źródle; referencja i tytuł są tam już w zmiennych pętli.
- `src-tauri/src/todo/mod.rs:452-458`: `set_ticked` dla GitLab/GitHub nic nie robi, gdy `ticked_at` już jest; zdarzenie ma powstać tylko przy faktycznej zmianie na odhaczone. Dla zadania własnego to samo przez `own_tasks.done_at`.
- `src-tauri/src/gitlab_refresh.rs:770-772`: zamknięcie przekazane do `with_store` w `record_seen` może po `record_round` zapisać zdarzenia w tym samym zablokowanym store, bez dodatkowego `with_store`.
- `src-tauri/src/runtime.rs:307`: `transitions` są policzone przed `publish`; `check_round` ma `app`, więc `StoreHandle` jest osiągalny. Wątek zdrowia nie jest poleceniem Tauri, więc `with_store` wprost jest tam zgodny z AGENTS.md (tak samo robi `record_seen`).
- `src-tauri/tests/new_assignments.rs:229`: zamienić `9` na `MIGRATIONS.len()` (jak w pozostałych testach).

## What We're NOT Doing

- Brak wykrywania nowych przypisań w GitHubie (decyzja: tylko GitLab). Odhaczenia, zdjęcia, zamknięcia w źródle i błędy źródła GitHub są w dzienniku.
- Brak zdarzeń dla odznaczenia pozycji, dodania do „Do zrobienia”, przeniesienia na następny dzień (`roll_over_in`), oznaczenia zadania jako zrobionego w widoku „Zadania własne” (`set_own_task_done`), odświeżenia tytułu w `reconcile_*`, usunięcia lub edycji wpisu czasu pracy.
- Brak zdarzeń dla `Unavailable` (GitLab bez VPN) i `NotConfigured`; przejście Error→Unavailable jest ciche, jak w toastach.
- Brak historii poprzednich dni, przewijania do starszych, filtrów i stronicowania. Limit 200 najnowszych.
- Brak zmian w toastach, w treści logu (poza nowymi wpisami z samym rodzajem błędu) i w wywołaniach `glab`/`gh`.
- Brak synchronizacji artefaktu design systemu (F-05).

## Implementation Approach

Nowy moduł `src-tauri/src/events/` jest jedynym miejscem, które pisze i czyta tabelę `events`. Zapis to `record_in(tx, today, now, &NewEvent)` wołane wewnątrz istniejących transakcji (odhaczenie, zdjęcie, zamknięcie w źródle, START/STOP), więc akcja i jej zdarzenie są atomowe. Czas zdarzenia to lokalny tekst w formacie `todo` (`YYYY-MM-DDTHH:MM:SS`), a „dziś” to prefiks daty, dokładnie jak w `roll_over_in`; kolejność to `id` (kolejność wstawienia), więc zmiana zegara nie miesza listy. Zdarzenia przypisań i źródeł to osobny `write` przez `record(store, …)`. Każde miejsce zapisu po commicie emituje `events-changed` (bez ładunku). Polecenie `list_events_today` (async, `spawn_blocking`) zwraca dzisiejsze zdarzenia. UI ma lustro typów w `src/events/` i panel w `src/todo/`.

## Critical Implementation Details

- **Czas lokalny w `work_time`.** `start`/`stop` dostają dziś tylko UTC ms. Dostają parametr `tz: &Tz` (konwencja modułu, jak `add`) i z niego liczą `today` i lokalne `now` zdarzenia z tego samego `now` ms, żeby START/STOP w dzienniku miały ten sam czas co wpis. To mechaniczna zmiana ok. 30 wywołań w testach (`&Local` / strefa testowa).
- **Store w błędzie.** Zdarzenie „Dane aplikacji: błąd” nie zapisze się, gdy baza jest niedostępna; zapis zdarzeń źródeł próbuje raz na rundę z przejściami i przy błędzie loguje sam rodzaj (`warn`, `Busy` → `debug`). Powrót „Dane aplikacji: znowu działa” zapisze się normalnie.

## Phase 1: Rekord zdarzeń (Rust)

### Overview

Baza dostaje tabelę `events`, moduł `events` umie zapisać zdarzenie z czyszczeniem innych dni i odczytać dzisiejsze, polecenie `list_events_today` jest zarejestrowane, a TS ma lustro typów. Nic jeszcze nie zapisuje zdarzeń.

### Changes Required:

#### 1. Migracja v10

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Tabela na dzisiejsze zdarzenia, dopisana na końcu listy (nie edytujemy wydanych migracji).

**Contract**: `Migration { version: 10, sql: "CREATE TABLE events (...)" }` z komentarzem w stylu pozostałych (S-08; tylko dzisiejsze zdarzenia, czyszczone przy każdym zapisie; trzyma tylko to, co UI pokazuje: referencję i tytuł pozycji, klucz i etykietę źródła, rodzaj błędu; bez URL-i, hostów i komunikatów). Kolumny:
- `id INTEGER PRIMARY KEY`
- `at TEXT NOT NULL` (lokalny `YYYY-MM-DDTHH:MM:SS`)
- `kind TEXT NOT NULL CHECK (kind IN ('assigned','ticked','removed','closedInSource','workStarted','workStopped','sourceError','sourceRecovered'))`
- `item_source TEXT CHECK (item_source IN ('own','gitlab','github'))`, `item_kind TEXT CHECK (item_kind IN ('issue','mergeRequest'))`, `reference TEXT`, `title TEXT CHECK (title IS NULL OR length(title) <= 300)`
- `source_id TEXT CHECK (source_id IN ('settings','app-data','gitlab','github','reminders'))`, `source_label TEXT`, `error_kind TEXT`
- indeks na `at` nie jest potrzebny (kilkaset wierszy maksymalnie); czyszczenie i odczyt po `substr(at, 1, 10)`.

#### 2. Moduł `events`

**File**: `src-tauri/src/events/mod.rs`, `src-tauri/src/events/error.rs`, `src-tauri/src/events/tests.rs`; `pub mod events;` w `src-tauri/src/lib.rs`

**Intent**: Jedno miejsce zapisu i odczytu zdarzeń, bez `tauri` i bez zegara (czas przychodzi parametrem), na wzór `new_assignments`.

**Contract**:
- `pub enum NewEvent { Assigned { item_kind: ItemKind, reference: String, title: String }, Ticked(Subject), Removed(Subject), ClosedInSource(Subject), WorkStarted, WorkStopped, SourceError { source: SourceId, label: String, error_kind: String }, SourceRecovered { source: SourceId, label: String } }`, gdzie `pub struct Subject { pub source: ItemSource /* Own | Gitlab | Github */, pub reference: Option<String> /* None dla zadania własnego */, pub title: String }`.
- `pub fn record_in(tx: &Transaction<'_>, today: NaiveDate, now: NaiveDateTime, event: &NewEvent) -> Result<(), StoreError>`: najpierw `DELETE FROM events WHERE substr(at, 1, 10) <> ?today`, potem `INSERT`. Tytuł ucięty do 300 znaków (granica znaku, nie bajtu).
- `pub fn record(store: &mut Store, today, now, events: &[NewEvent]) -> Result<usize, EventError>`: jeden `write`, `record_in` dla każdego; pusta lista nie otwiera transakcji i zwraca 0.
- `pub fn from_transitions(ts: &[health::Transition]) -> Vec<NewEvent>`: `to_kind = Some(k)` → `SourceError { source: t.id, label: t.label, error_kind: k }`, `None` → `SourceRecovered`.
- `pub fn list_today(store: &Store, today: NaiveDate) -> Result<EventLog, EventError>`: wiersze z `substr(at, 1, 10) = today`, `ORDER BY id DESC LIMIT 201`; `EventLog { events: Vec<EventItem> /* max 200 */, truncated: bool }`. Wiersz, którego kolumny nie pasują do rodzaju, daje `StoreError::Damaged` bez wartości (jak `new_assignments/mod.rs:156-160`).
- DTO (serde `camelCase`): `EventItem { id: i64, at: String, #[serde(flatten)] event: EventView }`, gdzie `EventView` to `#[serde(tag = "kind", rename_all = "camelCase")]` z wariantami jak `NewEvent`, ale `SourceError { source_id, label, reason }`, gdzie `reason = health::short_reason(error_kind)`. `short_reason` w `health/mod.rs` staje się `pub(crate)`.
- `EventError { Store(StoreError), Background }` z `From<StoreError>`, `From<rusqlite::Error>`, `From<BackgroundFailed>`, `kind()`; serde `tag = "kind", content = "detail"`. Błędy nie niosą wartości wierszy.
- `pub const CHANGED_EVENT: &str = "events-changed";`

#### 3. Polecenie odczytu

**File**: `src-tauri/src/events/commands.rs`, rejestracja w `tauri::generate_handler!` w `src-tauri/src/lib.rs`

**Intent**: Jedyne polecenie modułu, tylko do odczytu, plus pomocnik emisji dla miejsc zapisu (faza 2).

**Contract**: `#[tauri::command] pub async fn list_events_today(app: AppHandle) -> Result<EventLog, EventError>` przez `crate::blocking` + `with_store(today, …)`; błąd logowany samym rodzajem (`warn`, `Busy` → `debug`). `pub fn notify(app: &AppHandle)` emituje `CHANGED_EVENT` z `()`, błąd emisji `warn` bez wartości.

#### 4. Lustro TS i API

**File**: `src/events/types.ts`, `src/events/api.ts`

**Intent**: AGENTS.md wymaga lustra typów w tym samym commicie co typ Rust.

**Contract**: `EventEntry = { id: number; at: string } & (…)` jako unia na `kind` (`assigned` z `itemKind`, `reference`, `title`; `ticked` | `removed` | `closedInSource` z `source`, `reference: string | null`, `title`; `workStarted`; `workStopped`; `sourceError` z `sourceId`, `label`, `reason`; `sourceRecovered` z `sourceId`, `label`), `EventLog = { events: EventEntry[]; truncated: boolean }`, `EventError` (unia na `kind`, `Store` z `StoreError` z `src/components/storeError`), `EVENTS_CHANGED = "events-changed"`. `api.ts`: `listEventsToday(): Promise<EventLog>` i `eventErrorMessage(e: unknown): string` (na wzór `todoErrorMessage`, przez `storeErrorMessage`).

#### 5. Test wersji schematu

**File**: `src-tauri/tests/new_assignments.rs`

**Intent**: Usunąć na sztywno wpisaną wersję 9.

**Contract**: `:229` porównuje z `MIGRATIONS.len()` (albo `MIGRATIONS.last().unwrap().version`).

#### 6. Testy integracyjne i crash

**File**: `src-tauri/tests/events.rs`, `src-tauri/tests/events_crash.rs`

**Intent**: Sprawdzić upgrade, regułę „tylko dziś” i trwałość zapisu po zabiciu procesu, z istniejącym harnessem.

**Contract**: `events.rs` (wymyślone dane: `acme/web!421`, `octo/demo#12`, „Wymyślony tytuł”): upgrade bazy z v9 do v10 zachowuje wiersze `todo_items` i `seen_assignments` i daje `user_version == MIGRATIONS.len()`; `record` + `list_today` zwraca najnowsze pierwsze z polami jak zapisane; zapis z nowym dniem usuwa wczorajsze; zapis po cofnięciu zegara usuwa wiersze z „jutra”; `list_today` nie zwraca wierszy innego dnia, nawet bez zapisu; 201 zdarzeń → 200 i `truncated = true`, 200 → `false`; tytuł 400 znaków (z wielobajtowymi) zapisany jako 300 znaków; `SourceError` zwraca `reason` ze stałej frazy. `events_crash.rs` na wzór `tests/new_assignments_crash.rs`: proces-pisarz w pętli `record` z czasem rosnącym o stały krok, tak że co K zapisów zmienia się dzień; po każdym zabiciu `quick_check == "ok"`, liczba zapisanych zdarzeń == zgłoszona albo +1, wiersze == czysty model „zdarzenia dnia ostatniego zapisu”; test kończy się asercją, że pisarz przeszedł przez zmianę dnia.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy jednostkowe w `src/events/tests.rs` (`from_transitions`: wejście w błąd, zmiana rodzaju, powrót; ucięcie tytułu na granicy znaku; kształt JSON `EventItem` z `kind` i polami camelCase) i `schema::validate(MIGRATIONS)`
- Testy integracyjne `tests/events.rs` (upgrade v9→v10, najnowsze pierwsze, czyszczenie wczoraj i „jutro”, filtr odczytu, limit 200 i `truncated`, ucięcie tytułu, `reason`) przechodzą
- Test `tests/events_crash.rs` przechodzi i dochodzi do zmiany dnia
- `tests/new_assignments.rs` przechodzi po v10
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag
- `npm run build` przechodzi (nowe `src/events/types.ts` i `api.ts`)

**Implementation Note**: Po tej fazie i zielonej weryfikacji automatycznej zatrzymaj się przed fazą 2.

---

## Phase 2: Zapis zdarzeń w miejscach zmian (Rust)

### Overview

Odhaczenie, zdjęcie, zamknięcie w źródle, START/STOP, nowe przypisania GitLab i przejścia źródeł zapisują zdarzenia, a każde miejsce zapisu emituje `events-changed`.

### Changes Required:

#### 1. „Do zrobienia”: odhaczenie i zdjęcie

**File**: `src-tauri/src/todo/mod.rs`

**Intent**: Zdarzenie w tej samej transakcji co akcja, tylko gdy stan faktycznie się zmienił.

**Contract**:
- `set_ticked`: przy `ticked = true` i zmianie stanu (GitLab/GitHub: `ticked_at` było puste; własne: zadanie nie było zrobione) → `events::record_in(tx, today, now, &NewEvent::Ticked(subject))`. Odznaczenie i ponowne odhaczenie już odhaczonego nie zapisują nic. `subject` z wiersza: własne → `(Own, None, own_tasks.title)`, GitLab → `(Gitlab, gitlab_ref, gitlab_title)`, GitHub → `(Github, github_ref, github_title)` (poszerzony `SELECT` albo mały pomocnik `subject_of(tx, id)`; sprawdzanie stanu zadania własnego bez zmiany `own_tasks::set_done_in`, jeśli to możliwe).
- `remove`: przed `UPDATE` czyta `subject`; po udanym `UPDATE` → `NewEvent::Removed(subject)`.
- Błąd `record_in` (`StoreError`) przechodzi jako `TodoError::Store` i cofa akcję.

#### 2. Zamknięcie w źródle

**File**: `src-tauri/src/todo/mod.rs`

**Intent**: Wyjaśnić w dzienniku, dlaczego pozycja odhaczyła się sama.

**Contract**: W `reconcile_gitlab` i `reconcile_github`, w gałęzi `None =>` po `UPDATE … ticked_by = 'source'` → `record_in(tx, today, now, &NewEvent::ClosedInSource(Subject { source, reference: Some(reference), title }))`. Gałąź odświeżenia tytułu/URL-a nie zapisuje zdarzenia. Sygnatury bez zmian.

#### 3. START/STOP

**File**: `src-tauri/src/work_time/mod.rs`, `src-tauri/src/work_time/commands.rs`, testy z wywołaniami `start`/`stop` (`src-tauri/src/work_time/tests.rs`, `src-tauri/tests/work_time.rs`, `src-tauri/tests/work_time_crash.rs` i inne, które je wołają)

**Intent**: START i STOP z czasem lokalnym wpisu, atomowo z wpisem.

**Contract**: `pub fn start<Tz: TimeZone>(store: &mut Store, now: i64, tz: &Tz) -> Result<i64, WorkTimeError>` i `pub fn stop<Tz: TimeZone>(store, now, tz) -> Result<(), WorkTimeError>`: lokalne `now` i `today` z `tz.timestamp_millis_opt(now).earliest()` (błąd → istniejący `WorkTimeError::InvalidLocalTime`), w tym samym `write` po `INSERT`/`UPDATE` → `record_in(…, &NewEvent::WorkStarted | WorkStopped)`. Polecenia przekazują `&Local`. Testy dostają `&Local` albo swoją stałą strefę.

#### 4. Nowe przypisania GitLab

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Każde naprawdę nowe przypisanie (to samo „nowe”, które dostaje toast z zapisanego zbioru) raz w dzienniku.

**Contract**: W `record_seen`, w zamknięciu `with_store`, po udanym `record_round`: jeśli `round.new` niepuste → `events::record(s, today, now_local, &items)`, gdzie `items` to `items_for(full, &round.new)` zmapowane na `NewEvent::Assigned { item_kind, reference, title }`, a `now_local` to lokalna forma `now_ms`. Błąd zapisu zdarzeń nie zmienia wyniku `record_round` (toasty i flagi NOWE bez zmian) i jest logowany samym rodzajem. Fallback z pamięci sesji (błąd store) nie zapisuje zdarzeń. Po zapisaniu ≥1 zdarzenia → `events::commands::notify(app)`.

#### 5. Przejścia źródeł

**File**: `src-tauri/src/runtime.rs`, `src-tauri/src/health/mod.rs` (`short_reason` → `pub(crate)`, jeśli nie zrobione w fazie 1)

**Intent**: Wejście źródła w błąd i powrót w dzienniku, według tej samej reguły co toasty.

**Contract**: W `check_round`, po `publish` i toaście: jeśli `transitions` niepuste → `app.state::<StoreHandle>().with_store(today, |s| events::record(s, today, now, &events::from_transitions(&transitions)))` z `today`/`now` z `Local::now()`. Sukces z ≥1 → `notify`. Błąd → `log::warn!("events: record failed: {}", e.kind())` (`Busy` → `debug`), bez etykiety i rodzaju źródła ponad to, co już loguje linia `health: …`.

#### 6. Emisja `events-changed`

**File**: `src-tauri/src/todo/commands.rs`, `src-tauri/src/work_time/commands.rs`, `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/github_refresh.rs`

**Intent**: Panel odświeża się po każdym zapisie zdarzenia, bez drugiej subskrypcji `todo-changed` w UI.

**Contract**: `events::commands::notify(app)` wszędzie tam, gdzie po udanej zmianie emitowane jest `todo-changed` (`run` w `todo/commands.rs` dla `Call::Mutation`, `reconcile_todo` w obu rundach przy `changed > 0`), w `start_work`/`stop_work` obok `notify`, oraz w punktach 4 i 5. Nadmiarowa emisja (np. po dodaniu pozycji) tylko przeładowuje panel.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy w `tests/todo.rs` (lub `src/todo/tests.rs`): odhaczenie GitLab/GitHub/własnej zapisuje jedno `Ticked` z referencją i tytułem (własna: bez referencji, z tytułem zadania); ponowne odhaczenie już odhaczonej i odznaczenie nie zapisują nic; `remove` zapisuje `Removed`; `reconcile_gitlab` i `reconcile_github` zapisują `ClosedInSource` tylko dla odhaczonych przez źródło, nie dla odświeżenia tytułu; odmowa (`NotFound`) nie zapisuje zdarzenia
- Nowe testy w `tests/work_time.rs`: `start` zapisuje `WorkStarted`, `stop` zapisuje `WorkStopped`, odmowa (`AlreadyRunning`, `NotRunning`) nie zapisuje nic; czas zdarzenia to lokalna forma `now` w strefie testowej
- Istniejące testy `todo*`, `work_time*`, `new_assignments*`, `hidden_issues_round`, `health_probe` przechodzą
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag
- `rg -n "log::(info|warn|debug|error)!" src-tauri/src/events` pokazuje tylko wpisy z `kind()` (bez referencji, tytułów i etykiet)

**Implementation Note**: Po tej fazie i zielonej weryfikacji automatycznej zatrzymaj się przed fazą 3.

---

## Phase 3: Panel i dokumentacja (UI)

### Overview

Dashboard pokazuje panel „Dziennik zdarzeń” pod „Stanem źródeł”, a AGENTS.md wymienia nowy rekord w danych aplikacji.

### Changes Required:

#### 1. Hak

**File**: `src/events/useEventLog.ts`

**Intent**: Stan ładowanie/błąd/dane dziennika, odświeżany na żywo, na focus i o północy, odporny na spóźnione odpowiedzi.

**Contract**: Wzór `src/todo/useTodo.ts` / `src/own-tasks/useOwnTasks.ts`: najpierw `listen(EVENTS_CHANGED)`, potem `listEventsToday()`; przeładowanie na `focus`, `visibilitychange` i o lokalnej północy (`setTimeout`, jak `useTodo.ts:33-38`, 86-92; wspólny pomocnik, jeśli już jest wyeksportowany); znacznik sekwencji odrzuca spóźnione odpowiedzi. Zwraca `{ load: {state:"loading"} | {state:"error"; message} | {state:"loaded"; log: EventLog}, lastSuccessAt: Date | null, reloadError: string | null, retry: () => void }`.

#### 2. Panel

**File**: `src/todo/EventLogPanel.tsx`, `src/todo/TodoView.tsx`, `src/todo/TodoView.css`, `src/todo/shortRef.ts`

**Intent**: Panel pod „Stanem źródeł”, w stylu jego wierszy, z czterema stanami.

**Contract**:
- `TodoView.tsx`: w `todo-col-side` po `<SourcesPanel/>` → `<EventLogPanel/>`. `shortRef` przeniesiony do `src/todo/shortRef.ts` i importowany w obu plikach (bez zmiany zachowania).
- `EventLogPanel`: `<Panel title="Dziennik zdarzeń" meta={liczba dzisiejszych zdarzeń}>`. Ładowanie: `<p className="dn-loading …" role="status">`. Błąd: `ErrorState` z tytułem „Nie udało się wczytać dziennika”, `message` z `eventErrorMessage`, `onRetry` i `lastSuccess` w formacie `lastRefreshLabel` z `TodoView` (wyeksportować albo powtórzyć lokalnie). Pusto: „Brak zdarzeń dzisiaj.” (`dn-empty`). `reloadError` przy danych → `<Notice tone="error" inset>`. `truncated` → stopka „Pokazano 200 najnowszych”.
- Wiersz `<li className="todo-event">`: czas `at.slice(11, 16)` (mono, tabular), etykieta (tabela w Desired End State), przedmiot `shortRef(reference) — title` z `title={reference}` na referencji, tytuł ucięty `text-overflow: ellipsis`. Linia `sourceError` z tonem błędu jak `is-error` w `SourcesPanel`, `sourceRecovered` z tonem ok.
- CSS w `TodoView.css` obok `.todo-source*`, tylko tokeny z `src/styles/tokens.css`; lista przewija się w panelu po przekroczeniu wysokości zbudowanej z tokenów rozmiaru wiersza.

#### 3. AGENTS.md

**File**: `AGENTS.md`

**Intent**: Twarda reguła o danych w app data dir wymienia nowy rekord.

**Contract**: W punkcie „Outside folders configured in settings…” lista app data dir dostaje po „work-time entries” wpis „today's event log (kind, local time, and the item reference and title or source name the dashboard shows; other days are pruned)”.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą
- `rg -n "#[0-9a-fA-F]{3,6}" src --glob "*.css" --glob "!src/styles/tokens.css"` nie znajduje nowych literalnych kolorów
- `rg -n "invoke" src/events src/todo/EventLogPanel.tsx` znajduje `invoke` tylko w `src/events/api.ts`

#### Manual Verification:

- Po pierwszym uruchomieniu nowej wersji panel „Dziennik zdarzeń” jest pod „Stanem źródeł” i pokazuje „Brak zdarzeń dzisiaj.” (albo błędy źródeł z pierwszej rundy)
- Odhaczenie pozycji GitLab, GitHub i własnej daje od razu wiersze „Odhaczono” z krótką referencją i tytułem (własna: sam tytuł); najechanie na referencję pokazuje pełną; odznaczenie nie dodaje wiersza; „Zdjęto” po zdjęciu pozycji
- START i STOP w nagłówku dają wiersze „START” / „STOP” z bieżącą godziną
- Z VPN: przypisanie sobie testowego zgłoszenia daje po rundzie „Nowe przypisanie”; zamknięcie zgłoszenia z „Do zrobienia” w GitLabie daje „Zamknięte w źródle”
- Wylogowanie `gh` (`gh auth logout`) daje „GitHub: błąd — wylogowano”, ponowne zalogowanie „GitHub: znowu działa”; odłączenie VPN nie dodaje wiersza
- Po restarcie aplikacji dzisiejsze zdarzenia są nadal widoczne; po północy (albo po zmianie daty systemowej na jutro) panel jest pusty
- Log aplikacji z tej sesji nie zawiera referencji ani tytułów z dziennika
- W wąskim oknie (800 px) długie tytuły ucinają się wielokropkiem i nie rozsypują kolumny

**Implementation Note**: Po tej fazie i zielonej weryfikacji automatycznej poproś użytkownika o ręczne sprawdzenie.

---

## Testing Strategy

### Unit Tests:

- `events::from_transitions`: wejście w błąd → `SourceError` z rodzajem, zmiana rodzaju → `SourceError` z nowym, powrót → `SourceRecovered`, pusto → pusto.
- Ucięcie tytułu do 300 znaków na granicy znaku (polskie znaki, emoji).
- Kształt JSON `EventItem` (`id`, `at`, `kind`, pola camelCase, `reference: null` dla własnej).

### Integration Tests:

- `tests/events.rs`: upgrade v9→v10, kolejność, czyszczenie innych dni (wczoraj, „jutro”), filtr odczytu bez zapisu, limit 200 / `truncated`, `reason`.
- `tests/todo.rs`, `tests/work_time.rs`: zdarzenia z akcji, brak zdarzeń przy odmowie i przy braku zmiany stanu, atomowość (zdarzenie i akcja razem w jednym `write`).
- `tests/events_crash.rs`: harness z `tests/common/crash.rs`, model „dzień ostatniego zapisu”.
- Tylko wymyślone dane (`acme/web!421`, `octo/demo#12`, „Wymyślony tytuł”).

### Manual Testing Steps:

1. Uruchom `npm run tauri dev`, otwórz „Do zrobienia”: panel pod „Stanem źródeł”.
2. Odhacz pozycję GitLab, GitHub i własną; odznacz jedną; zdejmij jedną: „Odhaczono” ×3 i „Zdjęto”.
3. START, po minucie STOP.
4. Z VPN przypisz sobie testowe zgłoszenie i zamknij inne z listy: „Nowe przypisanie”, „Zamknięte w źródle”.
5. `gh auth logout`, poczekaj na rundę, `gh auth login`: błąd i powrót GitHub. Odłącz VPN: brak nowego wiersza.
6. Zrestartuj aplikację: wiersze zostają. Sprawdź log aplikacji: bez referencji i tytułów.

## Performance Considerations

Najwyżej kilkaset wierszy dziennie; `DELETE … WHERE substr(at, 1, 10) <> ?` przy każdym zapisie skanuje tylko dzisiejszą tabelę. Odczyt to jeden `SELECT … LIMIT 201`. Runda zdrowia pisze tylko, gdy są przejścia.

## Migration Notes

Migracja v10 tworzy pustą tabelę `events`; istniejące dane bez zmian. S-07 (`app-data-details`) pokaże wersję schematu, więc po tym slice będzie to 10. Równoległe S-03 i S-05 nie dodają migracji; gdyby jakaś gałąź dodała swoją, przy merge'u bierze kolejny numer (wydanych nie przenumerowujemy). Przywracanie z kopii sprzed v10 działa jak dotąd, bo starsza baza migruje się przy otwarciu. Zainstalowane 0.2.x odmówi bazy v10 (`NewerSchema`), tak jak już odmawia v9.

## References

- Roadmap: `context/foundation/roadmap.md` (S-08), PRD FR-019, GitHub issue #1
- Wzorzec modułu rekordu i crash testu: `src-tauri/src/new_assignments/`, `src-tauri/tests/new_assignments_crash.rs`
- Przejścia źródeł: `src-tauri/src/health/mod.rs:140`, `src-tauri/src/runtime.rs:293-335`
- Panel źródeł: `src/todo/SourcesPanel.tsx`, `src/todo/TodoView.css:52-95`
- Poprzedni plan z migracją (v9): `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan.md`
- Design system Konsola: https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Rekord zdarzeń (Rust)

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy jednostkowe w `src/events/tests.rs` (`from_transitions`: wejście w błąd, zmiana rodzaju, powrót; ucięcie tytułu na granicy znaku; kształt JSON `EventItem` z `kind` i polami camelCase) i `schema::validate(MIGRATIONS)` — 1b77273
- [x] 1.2 Testy integracyjne `tests/events.rs` (upgrade v9→v10, najnowsze pierwsze, czyszczenie wczoraj i „jutro”, filtr odczytu, limit 200 i `truncated`, ucięcie tytułu, `reason`) przechodzą — 1b77273
- [x] 1.3 Test `tests/events_crash.rs` przechodzi i dochodzi do zmiany dnia — 1b77273
- [x] 1.4 `tests/new_assignments.rs` przechodzi po v10 — 1b77273
- [x] 1.5 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag — 1b77273
- [x] 1.6 `npm run build` przechodzi (nowe `src/events/types.ts` i `api.ts`) — 1b77273

### Phase 2: Zapis zdarzeń w miejscach zmian (Rust)

#### Automated

- [x] 2.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy w `tests/todo.rs` (lub `src/todo/tests.rs`): odhaczenie GitLab/GitHub/własnej zapisuje jedno `Ticked` z referencją i tytułem (własna: bez referencji, z tytułem zadania); ponowne odhaczenie już odhaczonej i odznaczenie nie zapisują nic; `remove` zapisuje `Removed`; `reconcile_gitlab` i `reconcile_github` zapisują `ClosedInSource` tylko dla odhaczonych przez źródło, nie dla odświeżenia tytułu; odmowa (`NotFound`) nie zapisuje zdarzenia — bd0d006
- [x] 2.2 Nowe testy w `tests/work_time.rs`: `start` zapisuje `WorkStarted`, `stop` zapisuje `WorkStopped`, odmowa (`AlreadyRunning`, `NotRunning`) nie zapisuje nic; czas zdarzenia to lokalna forma `now` w strefie testowej — bd0d006
- [x] 2.3 Istniejące testy `todo*`, `work_time*`, `new_assignments*`, `hidden_issues_round`, `health_probe` przechodzą — bd0d006
- [x] 2.4 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag — bd0d006
- [x] 2.5 `rg -n "log::(info|warn|debug|error)!" src-tauri/src/events` pokazuje tylko wpisy z `kind()` (bez referencji, tytułów i etykiet) — bd0d006

### Phase 3: Panel i dokumentacja (UI)

#### Automated

- [x] 3.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — e2f123c
- [x] 3.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą — e2f123c
- [x] 3.3 `rg -n "#[0-9a-fA-F]{3,6}" src --glob "*.css" --glob "!src/styles/tokens.css"` nie znajduje nowych literalnych kolorów — e2f123c
- [x] 3.4 `rg -n "invoke" src/events src/todo/EventLogPanel.tsx` znajduje `invoke` tylko w `src/events/api.ts` — e2f123c

#### Manual

- [x] 3.5 Po pierwszym uruchomieniu nowej wersji panel „Dziennik zdarzeń” jest pod „Stanem źródeł” i pokazuje „Brak zdarzeń dzisiaj.” (albo błędy źródeł z pierwszej rundy)
- [x] 3.6 Odhaczenie pozycji GitLab, GitHub i własnej daje od razu wiersze „Odhaczono” z krótką referencją i tytułem (własna: sam tytuł); najechanie na referencję pokazuje pełną; odznaczenie nie dodaje wiersza; „Zdjęto” po zdjęciu pozycji
- [x] 3.7 START i STOP w nagłówku dają wiersze „START” / „STOP” z bieżącą godziną
- [x] 3.8 Z VPN: przypisanie sobie testowego zgłoszenia daje po rundzie „Nowe przypisanie”; zamknięcie zgłoszenia z „Do zrobienia” w GitLabie daje „Zamknięte w źródle”
- [x] 3.9 Wylogowanie `gh` (`gh auth logout`) daje „GitHub: błąd — wylogowano”, ponowne zalogowanie „GitHub: znowu działa”; odłączenie VPN nie dodaje wiersza
- [x] 3.10 Po restarcie aplikacji dzisiejsze zdarzenia są nadal widoczne; po północy (albo po zmianie daty systemowej na jutro) panel jest pusty
- [x] 3.11 Log aplikacji z tej sesji nie zawiera referencji ani tytułów z dziennika
- [x] 3.12 W wąskim oknie (800 px) długie tytuły ucinają się wielokropkiem i nie rozsypują kolumny
