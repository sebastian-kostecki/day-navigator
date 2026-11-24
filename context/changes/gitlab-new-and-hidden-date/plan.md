# GitLab — flaga NOWE i data ukrycia — plan implementacji

## Overview

W „Praca (GitLab)” MR-y i widoczne zgłoszenia, które aplikacja pierwszy raz zobaczyła po ostatniej wizycie użytkownika w tym widoku, dostają flagę NOWE. Ukryte wiersze (po „Pokaż ukryte”) pokazują datę ukrycia, np. „ukryte 18.11”. Zamyka to FR-023 / issue #5 (slice S-02, M-2). Reguła NOWE żyje w Rust i ma testy jednostkowe. UI tylko wyświetla flagi i zgłasza koniec wizyty.

## Current State Analysis

- Data ukrycia już jest w bazie: `hidden_issues.hidden_at_ms INTEGER NOT NULL` (migracja v6, `src-tauri/src/store/schema.rs:106-123`). Ustawia ją `hide()` (`src-tauri/src/hidden_issues/mod.rs:94-130`). To upsert, więc ponowne ukrycie nadpisuje datę, a `Resurface` jej nie czyści. `partition()` (`hidden_issues/mod.rs:245-265`) ma rekord w ręku przy każdym ukrytym zgłoszeniu, ale zwraca zwykły klon `GitlabItem`, więc data nie trafia do UI.
- Moment pierwszego zobaczenia też już jest: `seen_assignments.first_seen_ms` (migracja v7, `schema.rs:130-144`). Klucz to nieprzezroczyste konto (SHA-256 z `gitlab::account_key`, `src-tauri/src/gitlab/mod.rs:25-33`), rodzaj i id. `record_round()` (`src-tauri/src/new_assignments/mod.rs:95-172`) wstawia `first_seen_ms` raz i potem podbija tylko `last_seen_ms`. Pierwsza runda konta zasiewa listę po cichu. `first_seen_ms` nie jest nigdzie odczytywane.
- `record_round` biegnie w `notify_new_assignments` dopiero po `*state.write() = next` i `publish` (`src-tauri/src/gitlab_refresh.rs:545-554`, `595-629`). Wynik „nowe” służy tylko toastom.
- Nie istnieje znacznik „widok oglądany do”. `App.tsx:69-74` odmontowuje widok przy przełączeniu, a każde zamontowanie `GitlabView` odpala `refreshGitlab()` (`src/gitlab/useGitlab.ts:61-65`). Aplikacja działa w `React.StrictMode` (`src/main.tsx`), więc w dev efekty montują się dwa razy.
- Zamknięcie okna to `api.prevent_close()` + `window.hide()` w `src-tauri/src/lib.rs:173-180`. Widok zostaje wtedy zamontowany.
- Kształtowanie listy ma trzy ścieżki, które muszą nieść nowe pola: udaną rundę (`commit_round` → `filter_shown`, `gitlab_refresh.rs:452-469`), nieudaną rundę z zachowaną listą (`reshape_kept` → `shape_items`, `:186-232`) i komendy ukryj/pokaż/odrzuć (`act_on_issue` → `shape_items`, `:109-152`).
- UI: `GitlabRow` (`src/gitlab/GitlabView.tsx:140-220`) pokazuje flagi w komórce tytułu (`Zmienione`, `Draft`, `W Do zrobienia`, wszystkie `tone="change"`). Ukryty wiersz ma tylko tekst dla czytnika „ (ukryte)” (`:176`). `Flag` (`src/components/Flag/Flag.tsx`) zna tony `change | info`. Tokeny `--info` / `--info-line` są w `src/styles/tokens.css`. W design systemie Konsola ton `new` to właśnie `info`/`info-line` z napisem NOWE.

## Desired End State

- MR albo widoczne zgłoszenie, którego `first_seen_ms` jest późniejsze niż zapisany znacznik wizyty konta, ma w komórce tytułu flagę NOWE (niebieską, ton `new`) przed pozostałymi flagami. Ukryte wiersze nigdy nie mają NOWE.
- Znacznik wizyty konta przesuwa się na czas ostatniej rundy, której snapshot był pokazany (nie na zegar „teraz”), gdy użytkownik opuszcza widok „Praca (GitLab)” albo chowa okno, mając ten widok aktywny. Przy następnej wizycie obejrzane pozycje nie mają NOWE. Pozycje, które przyszły po wyjściu, mają NOWE przez całą następną wizytę, także jeśli dojdą w jej trakcie.
- Bez znacznika (pierwsze uruchomienie po wydaniu, nowe konto `glab`) nic nie jest NOWE. Pierwsze wyjście z widoku zapisuje znacznik.
- Po zabitym procesie albo wyjściu z aplikacji z zasobnika znacznik się nie przesuwa i te same flagi pokażą się jeszcze raz. Błąd zawsze idzie w stronę „pokaż jeszcze raz”.
- Flagi przetrwają rundę bez VPN (zachowana lista) i komendy ukryj/pokaż/odrzuć.
- Ukryty wiersz pokazuje za tytułem szary tekst `ukryte DD.MM`, albo `ukryte DD.MM.RRRR`, gdy rok jest inny niż bieżący (czas lokalny). Zastępuje to dzisiejszy tekst „(ukryte)” dla czytnika.
- Błąd zapisu lub odczytu znacznika czy zbioru widzianych daje brak flag NOWE i wpis w logu z samym rodzajem błędu. Lista dalej się pokazuje, a tray nie zmienia stanu.
- W bazie przybywa tylko tabela `gitlab_view_marks` (nieprzezroczysty klucz konta i jeden czas), dodana nową migracją v9.

Weryfikacja: kontrole sprzed commita z `AGENTS.md` oraz ręczny przebieg w `npm run tauri dev` (z VPN i bez).

### Key Discoveries:

- `src-tauri/src/hidden_issues/mod.rs:257`: gałąź `Some(r) if r.is_hidden()` w `partition` to jedyne miejsce, gdzie trzeba wpiąć `hidden_at_ms`. Wszystkie trzy ścieżki przechodzą przez `partition`.
- `src-tauri/src/new_assignments/mod.rs:111`: `SELECT` czyta dziś tylko `kind, item_id, last_seen_ms`. Trzeba dodać `first_seen_ms` i znacznik w tej samej transakcji.
- `src-tauri/src/gitlab/mod.rs:488`: `next_snapshot` ustawia `account = account_key(config)`, więc runda może policzyć ten sam klucz z `cfg` przed bramką.
- `src-tauri/tests/new_assignments.rs:47-75` szuka migracji przez `position(|m| m.sql.contains(...))`, a `tests/hidden_issues.rs:84-127` używa `MIGRATIONS[..5]`. Dopisanie v9 ich nie psuje.
- `GitlabItem` jest budowany w 13 plikach (Rust src i tests). Nowe pola trzeba dopisać mechanicznie w każdym konstruktorze.

## What We're NOT Doing

- Brak flagi NOWE w „Do zrobienia” (picker), w GitHubie i w bloku „Zamknięte”. FR-023 dotyczy tylko „Praca (GitLab)”.
- Brak przycisku „oznacz jako przejrzane” i ręcznego gaszenia pojedynczej flagi.
- Brak zmiany toastów S-04 (treść, próg 3, `SessionSeen`). Zmienia się tylko miejsce wywołania `record_round` w rundzie.
- Brak zmiany semantyki `first_seen_ms`, zapominania po 30 dniach ani zasiewania konta.
- Brak migracji dla daty ukrycia: `hidden_at_ms` już istnieje.
- NOWE nie omija filtra widoczności: pozycja usunięta filtrem liczy się jako widziana przy wyjściu z widoku i po poszerzeniu filtra nie ma NOWE (impl review F2, świadomy kompromis z przeglądu planu).
- Brak synchronizacji artefaktu design systemu (F-05). Aplikacja dalej może pokazać kilka flag w jednym wierszu, jak dziś.
- Brak nowych wywołań `glab` i zmian w `src-tauri/src/cli/`.

## Implementation Approach

Rust liczy NOWE w tej samej transakcji, w której runda zapisuje zbiór widzianych. `record_round` czyta `first_seen_ms` i znacznik konta i zwraca obok kluczy do toastów zbiór „świeżych” kluczy, czyli `first_seen_ms > seen_until_ms`. Wywołanie przenosimy przed bramkę `hidden_gate`, a flagi wkładamy w listę, zanim snapshot zostanie zapisany i opublikowany. Zbiór świeżych kluczy trzymamy w stanie (`kept_new`) obok `kept_full`, więc `shape_items` nakłada go także w nieudanej rundzie i w komendach ukrywania. Nowe polecenie `mark_gitlab_seen` zapisuje znacznik `max(stary, shown_until)`, gdzie `shown_until` to `now_ms` ostatniej udanej rundy zapisanej w snapshocie. Potem czyści flagi w snapshocie i publikuje. UI wywołuje je przy wyjściu z widoku i przy zdarzeniu `main-hidden`, które Rust emituje przy chowaniu okna. Data ukrycia to pole wypełniane w `partition`, a formatowanie (DD.MM) robi UI w czasie lokalnym.

## Critical Implementation Details

- **Znacznik to czas pokazanej rundy, nie zegar.** Runda wpisuje `first_seen_ms = now_ms` w `record_round` przed bramką, a snapshot publikuje dopiero po niej. Znacznik „teraz” zapisany w tym oknie zgasiłby NOWE pozycji, których użytkownik nigdy nie widział (plan review F1). Dlatego stan trzyma `shown_until` (konto i `now_ms` rundy, której snapshot został zapisany), a `mark_seen` zapisuje właśnie tę wartość. Dzięki temu „NOWE gaśnie tylko po pokazaniu” wynika z danych, a nie z wyścigu.
- **Spóźniona runda.** Runda czyta znacznik z bazy w `record_round`, czyli przed bramką. Jeśli użytkownik wyjdzie z widoku, zanim runda weźmie bramkę, runda zapaliłaby flagi już pokazanych pozycji wobec starego znacznika. Dlatego `mark_seen` zapamiętuje zapisany znacznik w `last_mark`, a runda pod bramką odrzuca świeże klucze z `first_seen_ms <= last_mark`. Pozycje rundy w toku mają `first_seen_ms` większe od `shown_until`, więc zostają NOWE. Stąd świeże klucze wracają z `record_round` razem ze swoim `first_seen_ms`.
- **StrictMode.** Znacznika nie wolno zapisywać w cleanupie efektu `GitlabView`, bo w dev StrictMode od razu odmontuje i zamontuje widok i zgasi flagi przy wejściu. Wyjście z widoku wykrywamy po zmianie aktywnego widoku w `App.tsx`: poprzedni widok trzymamy w refie, a porównanie jest idempotentne przy podwójnym efekcie.

## Phase 1: Dane i reguły (Rust)

### Overview

Baza dostaje tabelę znacznika wizyty, `new_assignments` umie policzyć świeże klucze i zapisać znacznik, a `GitlabItem` niesie `isNew` i `hiddenAtMs`. Data ukrycia od razu trafia do snapshotu przez `partition`. Flaga NOWE jest jeszcze zawsze `false`.

### Changes Required:

#### 1. Migracja v9

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Zapisać per konto, do kiedy użytkownik oglądał widok GitLab, bez treści z pracy. Dopisujemy nową migrację na końcu listy, nie edytujemy wydanych.

**Contract**: `Migration { version: 9, sql: "CREATE TABLE gitlab_view_marks (account TEXT PRIMARY KEY, seen_until_ms INTEGER NOT NULL) WITHOUT ROWID;" }` z komentarzem w stylu pozostałych: S-02, `account` to klucz z `gitlab::account_key` (hash, nigdy host ani login), jeden czas UTC w ms, brak tytułów, referencji i URL-i. Klucz konta bez `REFERENCES assignment_accounts`, bo znacznik może powstać przed pierwszą rundą konta.

#### 2. Czysta reguła świeżości

**File**: `src-tauri/src/new_assignments/rules.rs`

**Intent**: Jedno miejsce, które mówi, co jest NOWE: klucz jest świeży, gdy jego `first_seen_ms` jest ściśle większe od znacznika. Bez znacznika nic nie jest świeże.

**Contract**: `pub fn fresh_since(seen: &[(ItemKey, i64)], mark: Option<i64>) -> Vec<(ItemKey, i64)>`. Zwraca pary `(klucz, first_seen_ms)` z `first_seen_ms > mark`, dla `None` zwraca pusty wektor. Kolejność jak na wejściu, bez duplikatów.

#### 3. Runda zwraca świeże klucze; zapis znacznika

**File**: `src-tauri/src/new_assignments/mod.rs` (oraz `rules.rs` dla typu wyniku)

**Intent**: W tym samym zapisie, który rejestruje rundę, odczytać `first_seen_ms` bieżących kluczy i znacznik konta, a potem zwrócić świeże klucze obok nowych do toastów. Dodać operację zapisu znacznika, która nigdy go nie cofa.

**Contract**:
- `record_round(store, account, current, now_ms) -> Result<RoundSeen, NewAssignmentError>`, gdzie `pub struct RoundSeen { pub new: Vec<ItemKey>, pub fresh: Vec<(ItemKey, i64)> }`. `new` ma dzisiejsze znaczenie. `fresh` = `fresh_since` liczone dla kluczy z `current` po upsercie (stary `first_seen_ms` albo `now_ms` dla wstawionych) i dla znacznika z `gitlab_view_marks` tego konta. Runda zasiewająca daje `fresh` puste, bo znacznika jeszcze nie ma albo wszystko ma `first_seen_ms = now_ms`.
- Wywołujący `record_round` dostosowujemy już w tej fazie, żeby 1.1 przeszło (plan review F2): `gitlab_refresh.rs:604` tymczasowo bierze `.map(|r| r.new)` (pełne przestawienie rundy w fazie 2), a asercje w `tests/new_assignments.rs` (`:77-228`) i `tests/new_assignments_crash.rs:104` czytają `.new`.
- `pub fn mark_view_seen(store: &mut Store, account: &str, seen_until_ms: i64) -> Result<(), NewAssignmentError>`: upsert `seen_until_ms = max(seen_until_ms, excluded.seen_until_ms)`, jeden `write`. Wywołujący podaje czas pokazanej rundy, nie zegar.
- `SessionSeen::resolve` i `items_for` bez zmian. Wywołujący przekazuje `stored.map(|r| r.new)`.
- `pub fn mark_fresh(items: &mut AssignedItems, fresh: &HashSet<ItemKey>)` ustawia `is_new` na `merge_requests` i `issues` według zbioru, a na `hidden_issues` zawsze `false`.
- Błędy nie niosą wartości wierszy (niezmiennik `NewAssignmentError`).

#### 4. Pola w `GitlabItem`

**File**: `src-tauri/src/gitlab/mod.rs` (oraz każdy konstruktor `GitlabItem` w `src-tauri/src/**` i `src-tauri/tests/**`)

**Intent**: Snapshot niesie flagę NOWE i datę ukrycia per pozycja. Parser odpowiedzi `glab` daje `false` / `None`.

**Contract**: `pub is_new: bool` (serde `isNew`) i `pub hidden_at_ms: Option<i64>` (serde `hiddenAtMs`, UTC ms) z komentarzami: `is_new` ustawia runda (S-02), tylko na MR-ach i widocznych zgłoszeniach; `hidden_at_ms` tylko na pozycjach `hidden_issues`. Test kształtu JSON (`snapshot_serializes_for_the_webview`, ok. `:1722`) albo nowy test pozycji sprawdza oba klucze. `ClosedItem` niesie je jako `false` / `null`.

#### 5a. Lustro TS w tym samym commicie

**File**: `src/gitlab/types.ts`

**Intent**: AGENTS.md wymaga zmiany typu Rust i jego lustra TS w jednym commicie, więc pola trafiają do TS już w tej fazie, choć UI ich jeszcze nie czyta.

**Contract**: `GitlabItem` dostaje `isNew: boolean` i `hiddenAtMs: number | null` z krótkimi komentarzami jak w Rust.

#### 5. Data ukrycia w `partition`

**File**: `src-tauri/src/hidden_issues/mod.rs`

**Intent**: Ukryte zgłoszenie wychodzi z `partition` z datą swojego ukrycia, a widoczne z `None`. Dzięki temu wszystkie trzy ścieżki kształtowania listy niosą datę bez dodatkowego kodu.

**Contract**: W gałęzi `Some(r) if r.is_hidden()` → `GitlabItem { hidden_at_ms: Some(r.hidden_at_ms), ..issue.clone() }`; w pozostałych gałęziach `hidden_at_ms: None`.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy: `fresh_since` (brak znacznika → nic; `first_seen == mark` → nie; `first_seen > mark` → tak), `partition` ustawia `hidden_at_ms` tylko na ukrytych, `mark_fresh` nie flaguje ukrytych
- Testy integracyjne w `src-tauri/tests/new_assignments.rs`: upgrade bazy z v8 do v9 zachowuje wiersze `seen_assignments`; `record_round` zwraca `fresh` dla kluczy pierwszy raz widzianych po znaczniku; runda zasiewająca daje puste `fresh`; `mark_view_seen` nie cofa znacznika; znacznik jednego konta nie wpływa na drugie
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag
- `npm run build` przechodzi (TS z nowymi polami w `types.ts`)

**Implementation Note**: Po tej fazie i zielonej weryfikacji automatycznej zatrzymaj się przed fazą 2.

---

## Phase 2: Runda i polecenie (Rust)

### Overview

Runda zapisuje zbiór widzianych przed zapisem snapshotu i nakłada flagi NOWE. Flagi przetrwają nieudaną rundę i komendy ukrywania. Polecenie `mark_gitlab_seen` przesuwa znacznik i gasi flagi. Chowanie okna emituje `main-hidden`.

### Changes Required:

#### 1. Stan i kształtowanie listy

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Świeże klucze żyją obok zachowanej listy, więc każda ścieżka, która kształtuje listę, nakłada te same flagi. Stan pamięta też ostatni znacznik zapisany w tej sesji, żeby spóźniona runda nie zapaliła zgaszonych flag.

**Contract**:
- Nowe pola `GitlabRefreshState`: `kept_new: Mutex<HashSet<ItemKey>>`, `shown_until: Mutex<Option<(String, i64)>>` (konto i `now_ms` ostatniej udanej rundy zapisanej w snapshocie; ustawiane pod bramką razem ze snapshotem, czyszczone razem z `kept_full`) i `last_mark: Mutex<Option<(String, i64)>>` (konto, ostatni zapisany `seen_until_ms`). Kolejność blokad w komentarzu modułu: brama, `kept_config`, `kept_full`, `kept_filter`, `kept_new`, `shown_until`, `last_mark`, snapshot. Lock store'u nigdy nie jest trzymany razem z `kept_*`.
- `shape_items(full, records, fresh: &HashSet<ItemKey>, filter)` woła `mark_fresh` po `partition`, przed filtrem. `reshape_kept` i `act_on_issue` przekazują `kept_new`.

#### 2. Kolejność rundy

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Zbiór widzianych zapisuje się przed bramką, a flagi trafiają do snapshotu tej samej rundy. Toasty dalej idą po publikacji, bez trzymanego locka, z tym samym `SessionSeen`.

**Contract**:
- `notify_new_assignments` dzielimy na `record_seen(app, account, full, session_seen) -> SeenOutcome { toast_keys: Vec<ItemKey>, fresh: Vec<(ItemKey, i64)> }` (store, bez locków, przed `hidden_gate`) i krok toastów po `publish` (`items_for` → `notices_for` → `toast`, jak dziś).
- `record_seen` wołamy tylko dla `(Some(cfg), Ok(full))` z `account = gitlab::account_key(cfg)`. Przy błędzie store'u `fresh` jest puste, `toast_keys` pochodzą z `SessionSeen::resolve`, a logowanie zostaje bez zmian (rodzaj błędu, `SEEN_SET_WARNED`).
- `record_seen` dostaje `now_ms` rundy i przekazuje je do `record_round`. To ta sama wartość, która trafia do `first_seen_ms` i do `shown_until`.
- Pod bramką w gałęzi sukcesu: `fresh` minus klucze z `first_seen_ms <= last_mark` dla tego konta, następnie `mark_fresh` na liście przed `filter_shown`, `kept_new` = ten zbiór i `shown_until = (account, now_ms rundy)`. Gdy zapis zbioru widzianych się nie udał, `shown_until` też się ustawia, bo lista była pokazana.
- Nieudana runda: `kept_new` zostaje, gdy `next_snapshot` zachowuje listę, i jest czyszczony razem z `kept_full`, gdy jej nie zachowuje.

#### 3. Polecenie `mark_gitlab_seen`

**File**: `src-tauri/src/gitlab_refresh.rs`, rejestracja w `src-tauri/src/lib.rs`, `From<BackgroundFailed>` w `src-tauri/src/new_assignments/error.rs`

**Intent**: Zakończyć wizytę: zapisać znacznik konta zachowanej listy, zgasić flagi w snapshocie i opublikować go. Wywołanie tylko pisze do store'u aplikacji i nie woła `glab`.

**Contract**:
- `GitlabRefreshState::mark_seen(&self, save: impl FnOnce(&str, i64) -> Result<(), NewAssignmentError>) -> Result<bool, NewAssignmentError>` pod `hidden_gate`. Bez `shown_until` (żadna udana runda nie została pokazana albo lista przepadła) zwraca `Ok(false)` bez zapisu. W przeciwnym razie `save(account, shown_until_ms)`, potem `last_mark = (account, shown_until_ms)`, czyści `kept_new`, ustawia `is_new = false` na pozycjach snapshotu, `seq += 1` i zwraca `Ok(true)`. Błąd `save` zostawia stan i snapshot bez zmian. Zegar nie bierze udziału.
- `#[tauri::command] pub async fn mark_gitlab_seen(app: AppHandle) -> Result<(), NewAssignmentError>` przez `crate::blocking`, `with_store` + `mark_view_seen`, `publish` po `Ok(true)`. Błąd logowany po rodzaju (`warn` dla store'u, oprócz `Busy` → `debug`).
- `NewAssignmentError` dostaje wariant dla `BackgroundFailed`, jak `HiddenIssueError` (`hidden_issues/error.rs:37`), bez treści.

#### 4. Zdarzenie przy chowaniu okna

**File**: `src-tauri/src/lib.rs`

**Intent**: UI musi wiedzieć, że użytkownik schował okno, bo widok zostaje wtedy zamontowany i nic się nie odmontowuje.

**Contract**: W gałęzi `CloseRequested` dla `main` po `window.hide()` emitujemy zdarzenie `main-hidden` (stała z nazwą, ładunek `()`). Błąd emisji logujemy jako `warn`, bez wartości.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy jednostkowe w `gitlab_refresh.rs`: `shape_items` flaguje tylko MR-y i widoczne zgłoszenia ze zbioru; `act_on_issue` zachowuje `is_new` i `hidden_at_ms` po ukryciu innego zgłoszenia; `reshape_kept` po nieudanej rundzie zachowuje flagi; `mark_seen` bez `shown_until` nic nie zapisuje i nie zmienia `seq`; `mark_seen` zapisuje `shown_until` (nie zegar), gasi flagi, podbija `seq` i ustawia `last_mark`; pozycja z rundy zapisanej po `shown_until` zostaje NOWE; błąd `save` zostawia snapshot bez zmian; świeże klucze z `first_seen_ms <= last_mark` są odrzucane
- Istniejące testy `tests/new_assignments.rs`, `tests/new_assignments_crash.rs` i `tests/hidden_issues_round.rs` przechodzą po zmianie typu zwracanego przez `record_round`
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag
- `npm run build` przechodzi

**Implementation Note**: Po tej fazie i zielonej weryfikacji automatycznej zatrzymaj się przed fazą 3.

---

## Phase 3: UI i dokumentacja

### Overview

Wiersz GitLaba pokazuje flagę NOWE i „ukryte DD.MM”. `App.tsx` zgłasza koniec wizyty przy wyjściu z widoku i przy schowaniu okna. AGENTS.md wymienia nowy rekord w danych aplikacji.

### Changes Required:

#### 1. API polecenia

**File**: `src/gitlab/api.ts`

**Intent**: Typowana funkcja nowego polecenia. Pola `GitlabItem` w `types.ts` doszły już w fazie 1.

**Contract**: `markGitlabSeen(): Promise<void>` (`invoke("mark_gitlab_seen")`). UI nie czyta błędu, bo wywołujący go ignoruje, więc lustro `NewAssignmentError` w TS nie jest potrzebne.

#### 2. Ton `new` we `Flag`

**File**: `src/components/Flag/Flag.tsx`, `src/components/Flag/Flag.css`

**Intent**: Ton z design systemu Konsola dla nowego przypisania.

**Contract**: `tone?: "change" | "info" | "new"`. `.dn-flag-new` używa `--info` i `--info-line` z `tokens.css`, bez literalnych kolorów. Komentarz przy propie wymienia NOWE.

#### 3. Formatter dnia

**File**: `src/components/time.ts` (eksport w `src/components/index.ts`)

**Intent**: Wspólny, czysty formatter krótkiej daty w czasie lokalnym.

**Contract**: `formatDay(ms: number, nowMs: number): string` zwraca `DD.MM`, gdy rok `ms` jest równy rokowi `nowMs`, a w przeciwnym razie `DD.MM.RRRR`. Używa istniejącego `pad2`.

#### 4. Wiersz GitLaba

**File**: `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`

**Intent**: Pokazać NOWE i datę ukrycia zgodnie z decyzjami. Reszta wiersza zostaje bez zmian.

**Contract**:
- W komórce tytułu, przed `Zmienione`, `Draft` i `W Do zrobienia`: `{item.isNew && <Flag tone="new">Nowe</Flag>}`.
- Dla `hidden` zastępujemy `<span className="dn-sr-only"> (ukryte)</span>` widocznym tekstem `ukryte {formatDay(item.hiddenAtMs, now)}` w spanie (np. `gitlab-hidden-at`, mono, kolor `var(--ink-faint)`) za tytułem. Gdy `hiddenAtMs` jest `null`, zostaje samo „ukryte”.
- `now` to istniejący `useNow` widoku.

#### 5. Koniec wizyty w `App.tsx`

**File**: `src/App.tsx` (logika w nowym haku `src/gitlab/useGitlabSeenMark.ts`)

**Intent**: Zgłosić koniec wizyty, gdy aktywny widok zmienia się z `gitlab` na inny i gdy przy aktywnym `gitlab` przyjdzie `main-hidden`, odpornie na podwójne efekty StrictMode.

**Contract**: `useGitlabSeenMark(view: string)`. Ref z poprzednim widokiem; efekt na `[view]` woła `markGitlabSeen()`, gdy poprzedni to `"gitlab"`, a bieżący nie, po czym zapisuje bieżący w refie. Drugi efekt, aktywny tylko przy `view === "gitlab"`, robi `listen("main-hidden", () => markGitlabSeen())` i w cleanupie odsłuchuje. Błędy `markGitlabSeen` łapiemy i ignorujemy (`.catch(() => {})`), bo Rust je loguje, a skutkiem jest tylko ponowne pokazanie flag. `App.tsx` woła hak z bieżącym `view`.

#### 6. AGENTS.md

**File**: `AGENTS.md`

**Intent**: Twarda reguła o danych w app data dir wymienia nowy rekord.

**Contract**: W punkcie „Outside folders configured in settings…” lista app data dir dostaje po „seen-assignment keys (opaque account key, kind, id)” wpis „the GitLab view's seen-until mark (opaque account key, one instant)”.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą
- `rg -n "#[0-9a-fA-F]{3,6}" src --glob "*.css" --glob "!src/styles/tokens.css"` nie znajduje nowych literalnych kolorów

#### Manual Verification:

- Po pierwszym uruchomieniu nowej wersji żadna pozycja w „Praca (GitLab)” nie ma NOWE; po przejściu do innego widoku i z powrotem nadal nic
- Z VPN: po przypisaniu sobie testowego zgłoszenia, gdy aktywny jest inny widok, następna wizyta w „Praca (GitLab)” pokazuje przy nim niebieską flagę NOWE; po wyjściu z widoku i powrocie flagi już nie ma
- Zgłoszenie przypisane w trakcie wizyty dostaje NOWE w tej wizycie; po zamknięciu okna (do zasobnika) i ponownym otwarciu z zasobnika flagi już nie ma
- W `npm run tauri dev` (StrictMode) samo wejście w widok nie gasi flag
- Bez VPN (karta „GitLab niedostępny” z zachowaną listą) flagi NOWE zostają; ukrycie innego zgłoszenia ich nie gasi
- „Pokaż ukryte” pokazuje przy każdym ukrytym wierszu „ukryte DD.MM” z dniem ukrycia; ponowne ukrycie zmienia datę na dzisiejszą
- W wąskim oknie (800 px) komórka tytułu z NOWE / „ukryte DD.MM” nie rozsypuje tabeli

**Implementation Note**: Po tej fazie i zielonej weryfikacji automatycznej poproś użytkownika o ręczne sprawdzenie.

---

## Testing Strategy

### Unit Tests:

- `new_assignments::rules::fresh_since`: brak znacznika, równość (`first_seen == mark` → nie NOWE), większy, wiele kluczy w kolejności.
- `new_assignments::mark_fresh`: MR i widoczne zgłoszenie ze zbioru dostają `is_new`, ukryte nigdy, reszta `false`.
- `hidden_issues::partition`: `hidden_at_ms` tylko na ukrytych, `None` na widocznych i zmienionych.
- `gitlab_refresh`: `shape_items` / `reshape_kept` / `act_on_issue` zachowują flagi; `mark_seen` (bez `shown_until`, sukces z zapisem `shown_until` zamiast zegara, błąd zapisu, `last_mark` odcina spóźnione świeże klucze, pozycja rundy w toku zostaje NOWE).
- Kształt JSON `GitlabItem` z `isNew` i `hiddenAtMs`.

### Integration Tests:

- `tests/new_assignments.rs`: upgrade v8 → v9 z zachowaniem wierszy; zasiew → puste `fresh`; klucz dodany po `mark_view_seen` jest w `fresh`, klucz sprzed niego nie; `mark_view_seen` z wcześniejszym czasem nie cofa znacznika; izolacja kont `ACCOUNT_A` / `ACCOUNT_B`; zapomniany po 30 dniach klucz po powrocie jest znowu świeży.
- Tylko wymyślone dane (`ACCOUNT_A`, `gitlab.example.invalid`, `acme/invented#N`).

### Manual Testing Steps:

1. Zainstaluj albo uruchom nową wersję i otwórz „Praca (GitLab)”: brak NOWE.
2. Przejdź do „Do zrobienia”, przypisz sobie testowe zgłoszenie w GitLabie i wróć do GitLaba (wejście w widok odpala odświeżenie): NOWE przy tym zgłoszeniu.
3. Przejdź do innego widoku i wróć: brak NOWE.
4. Zostań w GitLabie, przypisz drugie zgłoszenie, „Odśwież”: NOWE. Zamknij okno krzyżykiem, otwórz z zasobnika: brak NOWE.
5. Odłącz VPN przy widocznej fladze NOWE: po rundzie flaga zostaje.
6. Ukryj zgłoszenie, włącz „Pokaż ukryte”: „ukryte <dzisiaj>”.

## Performance Considerations

`record_round` czyta już wszystkie wiersze konta. Dodatkowa kolumna i jeden `SELECT` znacznika w tej samej transakcji są pomijalne przy dziesiątkach pozycji. `mark_gitlab_seen` to jeden upsert przy wyjściu z widoku.

## Migration Notes

Migracja v9 tworzy pustą tabelę `gitlab_view_marks`. Istniejące `seen_assignments` zostają, a ich `first_seen_ms` od razu działa. Brak znacznika po aktualizacji oznacza „nic nie jest NOWE” aż do pierwszego wyjścia z widoku. S-08 (`event-log`) doda swoją migrację jako v10, więc po tym planie numer v9 jest zajęty. Przywracanie z kopii sprzed v9 działa jak dotąd, bo starsza baza migruje się przy otwarciu.

## References

- Roadmap: `context/foundation/roadmap.md` (S-02), PRD FR-023, GitHub issue #5
- Hide i resurface (S-03): `context/archive/2026-11-23-hide-and-resurface/plan.md`
- Powiadomienia o nowych przypisaniach (S-04): `context/archive/2026-11-23-new-assignment-notify/plan.md`
- Wzorzec komend z bramką: `src-tauri/src/hidden_issues/commands.rs:21-53`
- Design system Konsola, `Flag` (`tone="new"` = `info`, NOWE): https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Dane i reguły (Rust)

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy: `fresh_since` (brak znacznika → nic; `first_seen == mark` → nie; `first_seen > mark` → tak), `partition` ustawia `hidden_at_ms` tylko na ukrytych, `mark_fresh` nie flaguje ukrytych — 371442d
- [x] 1.2 Testy integracyjne w `src-tauri/tests/new_assignments.rs`: upgrade bazy z v8 do v9 zachowuje wiersze `seen_assignments`; `record_round` zwraca `fresh` dla kluczy pierwszy raz widzianych po znaczniku; runda zasiewająca daje puste `fresh`; `mark_view_seen` nie cofa znacznika; znacznik jednego konta nie wpływa na drugie — 371442d
- [x] 1.3 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag — 371442d
- [x] 1.4 `npm run build` przechodzi (TS z nowymi polami w `types.ts`) — 371442d

### Phase 2: Runda i polecenie (Rust)

#### Automated

- [x] 2.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy jednostkowe w `gitlab_refresh.rs`: `shape_items` flaguje tylko MR-y i widoczne zgłoszenia ze zbioru; `act_on_issue` zachowuje `is_new` i `hidden_at_ms` po ukryciu innego zgłoszenia; `reshape_kept` po nieudanej rundzie zachowuje flagi; `mark_seen` bez `shown_until` nic nie zapisuje i nie zmienia `seq`; `mark_seen` zapisuje `shown_until` (nie zegar), gasi flagi, podbija `seq` i ustawia `last_mark`; pozycja z rundy zapisanej po `shown_until` zostaje NOWE; błąd `save` zostawia snapshot bez zmian; świeże klucze z `first_seen_ms <= last_mark` są odrzucane — 0dbb074
- [x] 2.2 Istniejące testy `tests/new_assignments.rs`, `tests/new_assignments_crash.rs` i `tests/hidden_issues_round.rs` przechodzą po zmianie typu zwracanego przez `record_round` — 0dbb074
- [x] 2.3 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` bez uwag — 0dbb074
- [x] 2.4 `npm run build` przechodzi — 0dbb074

### Phase 3: UI i dokumentacja

#### Automated

- [x] 3.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 15096c2
- [x] 3.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą — 15096c2
- [x] 3.3 `rg -n "#[0-9a-fA-F]{3,6}" src --glob "*.css" --glob "!src/styles/tokens.css"` nie znajduje nowych literalnych kolorów — 15096c2

#### Manual

- [x] 3.4 Po pierwszym uruchomieniu nowej wersji żadna pozycja w „Praca (GitLab)” nie ma NOWE; po przejściu do innego widoku i z powrotem nadal nic
- [x] 3.5 Z VPN: po przypisaniu sobie testowego zgłoszenia, gdy aktywny jest inny widok, następna wizyta w „Praca (GitLab)” pokazuje przy nim niebieską flagę NOWE; po wyjściu z widoku i powrocie flagi już nie ma
- [x] 3.6 Zgłoszenie przypisane w trakcie wizyty dostaje NOWE w tej wizycie; po zamknięciu okna (do zasobnika) i ponownym otwarciu z zasobnika flagi już nie ma
- [x] 3.7 W `npm run tauri dev` (StrictMode) samo wejście w widok nie gasi flag
- [x] 3.8 Bez VPN (karta „GitLab niedostępny” z zachowaną listą) flagi NOWE zostają; ukrycie innego zgłoszenia ich nie gasi
- [x] 3.9 „Pokaż ukryte” pokazuje przy każdym ukrytym wierszu „ukryte DD.MM” z dniem ukrycia; ponowne ukrycie zmienia datę na dzisiejszą
- [x] 3.10 W wąskim oknie (800 px) komórka tytułu z NOWE / „ukryte DD.MM” nie rozsypuje tabeli
