# Notatka przy wpisie czasu pracy — plan implementacji

## Overview

Użytkownik może dodać, poprawić i zobaczyć krótką, opcjonalną notatkę bez formatowania przy wpisie czasu pracy: w formularzu „Dopisz brakujący wpis” i w formularzu edycji wpisu, a lista wpisów dnia pokazuje ją pod godzinami. Slice S-06 (M-3), MS-27, issue #27. Notatka to nowa kolumna `work_entries.note` (migracja v11), część typu wpisu w Rust, z którego czyta też raport miesiąca (pod przyszły S-05). Walidacja (przycięcie, jedna linia, ≤ 200 znaków) jest w Rust; UI tylko wyświetla i wysyła. Notatka nigdy nie trafia do logu aplikacji ani do dziennika zdarzeń, a błędy nie niosą jej wartości.

## Current State Analysis

- Tabela `work_entries` (v1, `src-tauri/src/store/schema.rs:12-26`) ma tylko instanty. Ostatnia wydana migracja to v10 `events` (`schema.rs:225-242`); `validate` wymaga ciągłej numeracji (`schema.rs:246-257`). Wzór kolumny notatki: `own_tasks.note TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 2000)` (`schema.rs:34`); wzór `ALTER TABLE … ADD COLUMN`: v4 (`schema.rs:86-89`).
- `work_time/mod.rs`: prywatny `Entry { id, start, end }` (`:85-89`) wypełnia `row_to_entry` po indeksach 0–2 (`:91-97`); trzy SELECT-y go współdzielą: `counted_by_id` (`:100-107`), `running_entry` (`:109-116`), `in_day` (`:119-141`). Publiczny `EntryRow` (`:58-67`) trafia do UI przez `entries_for_day` (`:205-242`). `report::compute` czyta wpisy przez `in_day` (`report.rs:125`).
- `add` (`mod.rs:297-321`) i `update` (`mod.rs:339-370`) to po jednym `store.write`; `update` przez `resolve_or_keep` (`:325-335`) zachowuje zapisane instanty, gdy czas lokalny się nie zmienił. `start` (`:259-276`) wstawia wiersz bez podania nowych kolumn.
- Komendy `add_work_entry(app, start, end)` i `update_work_entry(app, id, start, end: Option<String>)` (`commands.rs:89-116`); `run` loguje tylko `e.kind()` (`commands.rs:33-41`). Rejestracja w `lib.rs` się nie zmienia.
- `WorkTimeError` (`error.rs:10-35`) nie ma pól tekstowych; niezmiennik w `error.rs:3-4`. `StoreError` usuwa wartości wierszy (`store/error.rs:3-5`, `:48-59`).
- Dziennik zdarzeń: `add`/`update`/`remove` nie zapisują zdarzeń; `WorkStarted`/`WorkStopped` nie niosą danych (`events/mod.rs:81-82`).
- Precedens walidacji notatki/tytułu: `own_tasks.rs:122-141` (`trim`, `TitleMultiline` przy `\r`/`\n`, limit przez `chars().count()`), test wycieku `own_tasks.rs:498-515`, test „błąd ograniczenia bez wartości” `tests/store.rs:290-307`.
- UI: `EntryForm` (`src/worktime/WorkTimeEntries.tsx:31-92`) obsługuje dopisywanie i edycję; wiersz listy `EntryItem` (`:94-189`); `SaveFn` (`:29`); typy propsów `add`/`update` w `LoadedEntries` (`:196-197`) i `EntriesPanel` (`:261-262`). `useWorkEntries.ts:113-121` (`add`, `update`), `api.ts:32-39` i komunikaty `api.ts:115-148`, `types.ts:8-19` (`WorkTimeError`) i `:53-61` (`EntryRow`). `WorkTimeView.tsx:237-238` przekazuje `add`/`update` z hooka bez własnych typów, więc zostaje bez zmian.
- Wzór pola z licznikiem: `OwnTaskEditPanel.tsx:76-106`, `charCount` i uzasadnienie braku `maxLength` w `src/own-tasks/labels.ts:31-37`. Wzór wyświetlania notatki: `OwnTasksView.tsx:147`, `.own-row-note` (`OwnTasks.css:66-75`).
- Testy wołające `add`/`update` (zmiana sygnatury): `src-tauri/src/work_time/tests.rs` (helper `add_entry` `:58-60` i bezpośrednie wywołania `:89`, `:136`, `:144`, `:186`, `:203`, `:224`, `:232`, `:245`, `:264`, `:271`, `:338`, `:350`, `:358`, `:385`, `:503`, `:528`, `:552`, `:556`), `src-tauri/src/work_time/norm_tests.rs:45`, `src-tauri/tests/work_time.rs:98`, `:152`, `:178`.
- Harness awarii: `src-tauri/tests/common/crash.rs` (`run_writer_until`, `KillAt`, `MAX_WRITER_COMMITS = 20_000`); wzory: `tests/work_time_crash.rs`, `tests/days_off_crash.rs`.

## Desired End State

- W formularzu dopisywania i edycji wpisu jest jednoliniowe pole „Notatka (opcjonalnie)” z licznikiem `N/200`. Zapis z notatką to jeden zapis w store. Pusta notatka (także same spacje) oznacza brak notatki.
- Lista wpisów dnia pokazuje niepustą notatkę w osobnej linii pod godzinami, przyciemnioną, uciętą wielokropkiem; pełna treść w podpowiedzi (`title`) i w formularzu edycji.
- Notatka dłuższa niż 200 znaków albo z łamaniem linii jest odrzucana przez Rust z polskim komunikatem w formularzu; nic się nie zapisuje.
- Edycja samej notatki nie przesuwa zapisanych instantów; notatkę ma też wpis trwający (pole w jego formularzu edycji). START tworzy wpis bez notatki.
- Istniejąca baza v10 migruje się do v11 bez utraty wierszy; stare wpisy mają pustą notatkę.
- Log aplikacji, dziennik zdarzeń i treść błędów nie zawierają notatki. AGENTS.md wymienia notatki wpisów w danych katalogu danych aplikacji; CHANGELOG ma wpis w `[Unreleased]`.
- Weryfikacja: testy z faz poniżej oraz bramki `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` zielone po każdej fazie.

### Key Discoveries:

- Notatka w `Entry` (wspólny mapper trzech SELECT-ów) daje ją `entries_for_day` i `report::compute` bez drugiego zapytania (`mod.rs:91-141`, `report.rs:125`) — spełnia decyzję o S-05.
- `resolve_or_keep` (`mod.rs:325-335`) już gwarantuje, że edycja samej notatki zachowuje instanty z sekundami.
- `run` loguje tylko `e.kind()` (`commands.rs:36-39`); nowe warianty błędów niosą wyłącznie limit, więc niezmiennik `error.rs:3-4` zostaje.
- SQLite `length()` na TEXT liczy znaki (punkty kodowe), tak jak `chars().count()` w Rust i `[...value].length` w TS.
- `WorkTimeView.tsx` i `lib.rs` nie wymagają zmian, co ogranicza konflikty z S-01 i S-03.

## What We're NOT Doing

- Brak nowej komendy `set_note`: notatka wchodzi w istniejące `add_work_entry`/`update_work_entry` (jedna mutacja = jeden `write`).
- Brak notatki w nagłówku (`WorkTimerStatus`, `RunningEntry`), w przypomnieniach (toast), w zasobniku (S-03) i w ładunku `work-time-changed`.
- Brak nowych zdarzeń w dzienniku zdarzeń i brak pól notatki w `NewEvent`.
- Brak notatki w `DayReport`/`MonthReport` i w widoku miesiąca — to zakres S-05 (kopiowanie raportu); tu tylko `Entry.note` gotowe do odczytu.
- Brak notatek wieloliniowych, formatowania i `maxLength` w polu (limit jest tylko w Rust).
- Brak zmian w `lib.rs`, `WorkTimeView.tsx`, `README.md`, `context/foundation/roadmap.md` (także bez synchronizacji statusu S-06 przez `/10x-plan`) i PRD (bez nowych FR; slice zostaje przy MS-27).
- Brak twardego usuwania notatki przy miękkim usunięciu wpisu (wiersz zostaje jak dziś).

## Implementation Approach

Trzy fazy, każda zielona sama w sobie i commitowana osobno:

1. **Ścieżka odczytu**: migracja v11, notatka w `Entry`/`EntryRow`, lustro TS `EntryRow.note`, wyświetlanie na liście. Komendy bez zmian; wszystkie notatki są jeszcze `''`, więc UI wygląda jak dotąd.
2. **Ścieżka zapisu**: czysta `normalize_note` w nowym pliku `work_time/note.rs`, dwa warianty błędu (z lustrem TS i komunikatami), parametr `note` w `add`/`update` i w komendach, a w tym samym commicie `api.ts`, hook i formularz. Kontrakt Rust↔TS zmienia się w jednym commicie.
3. **Dowód awaryjny i dokumentacja**: nowy test harnessu awarii, AGENTS.md, CHANGELOG.

Nowy kod trzyma się w nowych plikach (`note.rs`, `work_entry_note_crash.rs`) i małych hunkach w plikach współdzielonych z S-01/S-03 (`commands.rs` tylko dwie komendy wpisów, `types.ts`/`api.ts` tylko regiony wpisów i błędów).

## Critical Implementation Details

- **Kolejność walidacji**: `normalize_note` wywołuj na samym początku `add` i `update`, przed parsowaniem czasów i przed `store.write`. Błąd notatki ma więc pierwszeństwo przed błędem czasu (test to utrwala). Do `INSERT`/`UPDATE` trafia wyłącznie znormalizowana wartość, zawsze jako parametr `?N`, nigdy przez `format!` w SQL (inaczej komunikat SQLite mógłby nieść wartość).
- **Indeks kolumny**: `note` dochodzi do wszystkich trzech SELECT-ów na indeksie 3, a `row_to_entry` czyta `r.get(3)?`. Rozjazd jednego zapytania kończy się błędem w czasie działania, nie kompilacji.
- **Pole w formularzu**: reguła `.wt-form-fields .dn-field { flex: 1 1 180px }` (`WorkTime.css:195`) w kontenerze z `flex-direction: column` zamieniłaby 180px w wysokość. Opakowanie pola notatki ma mieć `display: grid` (dzieci gridu ignorują `flex`).
- **Miejsce nowych reguł CSS** (mniej konfliktów z S-01, które rusza obszar `.wt-days`): `.wt-note` zaraz po `.wt-row-error` (`WorkTime.css:150-152`), `.wt-note-field` i `.wt-note-count` zaraz po `.wt-form-actions` (`WorkTime.css:199-202`).
- **Ręczne przebiegi dzielą prawdziwą bazę.** Każdy worktree, `npm run tauri dev` i zainstalowane wydanie używają identyfikatora `com.sebas.day-navigator` (`tauri.conf.json:4`), więc build deweloperski migruje ten sam `%APPDATA%\com.sebas.day-navigator\day-navigator.db`, którego użytkownik używa na co dzień. Po przebiegu z v11 zainstalowane wydanie 0.10.1 (obsługuje v10) odmawia bazy jako `NewerSchema` („Dane aplikacji” na czerwono, licznik nie działa) do czasu wydania z v11; build deweloperski S-01/S-03 bez v11 też jej odmówi. Po przenumerowaniu migracji przy merge'u baza z kolumną `note` jako v11 nie przejdzie innej v11 (`MigrationFailed`) i zostanie odrzucona, nigdy zresetowana. Procedura (precedens `context/archive/2026-11-23-own-tasks/plan.md:78-86`): przed pierwszym przebiegiem ręcznym zamknąć zainstalowaną aplikację i skopiować cały katalog danych aplikacji na bok; po zakończeniu testów (a także po uruchomieniu builda innego worktree i po przenumerowaniu) zamknąć aplikację i przywrócić kopię (`day-navigator.db*` i `backups/`), chyba że zainstalowane jest już wydanie z tą migracją. Wpisy dodane podczas testów to wymyślone dane i znikną przy przywróceniu.
- **Merge z równoległymi slice'ami.** Faza 2 zmienia sygnatury publicznych `work_time::add` i `work_time::update`. Gałąź, która scala się druga, po rebase/merge uruchamia `cargo test` i dopisuje `""` do każdego nowego wywołania `add`/`update`, które przyniosła druga gałąź (najpewniej nowe testy normy S-01 w `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/src/work_time/tests.rs` albo `src-tauri/tests/work_time.rs`; S-03 woła tylko `start`/`stop`). Gdyby inna gałąź dodała migrację, v11 dostaje kolejny wolny numer, a asercja pozycji w teście upgrade'u (Faza 1 §5a) zmienia się razem z nim.

## Phase 1: Kolumna notatki i ścieżka odczytu

### Overview

Dodaje kolumnę w store, przenosi notatkę przez typy wpisu w Rust do UI i pokazuje ją na liście. Bez zmiany komend; po tej fazie każda notatka to `''`.

### Changes Required:

#### 1. Migracja v11

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Dopisać na końcu `MIGRATIONS` migrację v11 dodającą notatkę wpisu; wydanych migracji nie ruszać.

**Contract**: `Migration { version: 11, sql }` po v10, z komentarzem w stylu sąsiednich: „S-06 (work-entry-note): optional one-line note on a work-time entry, `''` = none. Typed by the user; never logged and never in the event log. The CHECK mirrors `work_time::note::NOTE_MAX` (second line of defence; Rust validates first).” SQL:

```sql
ALTER TABLE work_entries ADD COLUMN note TEXT NOT NULL DEFAULT ''
  CHECK (length(note) <= 200 AND instr(note, char(10)) = 0 AND instr(note, char(13)) = 0);
```

Gdyby przy merge'u inna gałąź zajęła v11, bierze się kolejny wolny numer (precedens: komentarze `schema.rs:91-92`, `:104-105`, `:149-150`).

#### 2. Notatka w typach wpisu

**File**: `src-tauri/src/work_time/mod.rs`

**Intent**: Notatka staje się częścią prywatnego `Entry` (czytanego też przez raport miesiąca) i publicznego `EntryRow` (UI).

**Contract**: `Entry { id, start, end, note: String }`; `row_to_entry` czyta `note` z indeksu 3; SELECT-y w `counted_by_id`, `running_entry` i `in_day` zaczynają się od `SELECT id, start_ms, end_ms, note FROM work_entries`. `EntryRow` dostaje `pub note: String` (po `day_portion_ms`; serializuje się jako `note`, zawsze string, `''` = brak). `entries_for_day` przepisuje `e.note`. `RunningEntry`, `WorkTimerStatus`, `start`, `stop`, `add`, `update` bez zmian w tej fazie.

#### 3. Lustro TS

**File**: `src/worktime/types.ts`

**Intent**: Lustro `EntryRow` w tym samym commicie co zmiana Rust.

**Contract**: `EntryRow` dostaje `note: string` z komentarzem „Trimmed one-line note; `""` = none.”

#### 4. Wyświetlanie na liście

**File**: `src/worktime/WorkTimeEntries.tsx`, `src/worktime/WorkTime.css`

**Intent**: Wiersz w trybie podglądu pokazuje niepustą notatkę w osobnej linii pod godzinami; pusta nie renderuje niczego.

**Contract**: W `EntryItem`, w gałęzi podglądu, po `.wt-row-actions`/potwierdzeniu i przed `removeError`: `{row.note && <span className="body wt-note" title={row.note}>{row.note}</span>}` (rozmiar z klasy tekstowej `.body` z `tokens.css`). W `WorkTime.css` nowa reguła `.wt-note`: `flex-basis: 100%`, `min-width: 0`, `overflow: hidden`, `text-overflow: ellipsis`, `white-space: nowrap`, `color: var(--ink-faint)` — tylko istniejące tokeny, bez literalnych kolorów, tak jak `.wt-duration` (`WorkTime.css:137-142`) i `.own-row-note`.

#### 5. Testy migracji i ograniczenia

**File**: `src-tauri/tests/work_time.rs`

**Intent**: Rozstrzygnąć w praktyce zachowanie `ADD COLUMN … CHECK` na istniejących wierszach i udowodnić, że błąd ograniczenia nie niesie wartości.

**Contract**: (a) `upgrade_from_v10_adds_an_empty_note_and_keeps_rows`: pozycję migracji znajdź przez `MIGRATIONS.iter().position(|m| m.sql.contains("ADD COLUMN note"))` i asertuj `== 10` (wzór `tests/events.rs:62-96`); otwórz `MIGRATIONS[..10]`, wstaw surowym SQL trzy wiersze: zamknięty, trwający, usunięty (wymyślone instanty); otwórz pełną listą; asertuj `user_version == MIGRATIONS.len()`, niezmienione `(id, start_ms, end_ms, removed_at_ms)`, `note = ''` we wszystkich trzech, `entries_for_day` dnia zamkniętego wpisu zwraca `note == ""` i `serde_json::to_value(&row)["note"] == ""`. (b) `note_check_refuses_raw_values_and_counts_characters`: na pełnym schemacie surowy `UPDATE … SET note = ?1` przyjmuje 200 × „ż”, odrzuca 201 × „ż”, `"a\nb"` i `"a\rb"`. (c) `note_check_error_carries_no_note`: surowy `UPDATE` z notatką `SENTINEL-invented-…` + 200 znaków zwraca błąd store'u, którego `to_string()` i JSON nie zawierają sentinela (wzór `tests/store.rs:290-307`).

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy migracji v10→v11, ograniczenia CHECK i błędu bez wartości w `tests/work_time.rs`, `schema::validate(MIGRATIONS)` oraz istniejące testy upgrade'u (`tests/events.rs`, `tests/hidden_issues.rs`, `tests/new_assignments.rs`, `tests/todo.rs`)
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css` nic nie znajduje

#### Manual Verification:

- Po pierwszym uruchomieniu wersji z v11 na prawdziwych danych użytkownika (z kopią katalogu danych wg „Ręczne przebiegi dzielą prawdziwą bazę”) widok „Czas pracy” pokazuje dotychczasowe wpisy bez zmian i bez dodatkowej linii pod godzinami, a „Stan źródeł” pokazuje „Dane aplikacji” jako sprawne

**Implementation Note**: Po zielonej weryfikacji automatycznej commit tylko tej fazy, np. `feat(work-entry-note): store the note column and show it in the entry list`. Wiersz Manual zostaje dla użytkownika; przed nim procedura z „Ręczne przebiegi dzielą prawdziwą bazę” (Critical Implementation Details).

---

## Phase 2: Zapis notatki — walidacja, komendy i formularz

### Overview

Dodaje regułę notatki w Rust, parametr `note` w domenie i w komendach, lustro błędów i API w TS oraz pole w formularzu. Kontrakt `invoke` i jego lustro zmieniają się w tym jednym commicie.

### Changes Required:

#### 1. Reguła notatki

**File**: `src-tauri/src/work_time/note.rs` (nowy), `src-tauri/src/work_time/mod.rs` (`pub mod note;`)

**Intent**: Jedna czysta funkcja normalizująca notatkę, z testami jednostkowymi w tym samym pliku; bez store'u, bez zegara, bez logowania.

**Contract**: `pub const NOTE_MAX: usize = 200;` i `pub fn normalize_note(raw: &str) -> Result<String, WorkTimeError>`: `trim()` z obu stron; wynik zawierający `\r` lub `\n` → `NoteMultiline`; `chars().count() > NOTE_MAX` → `NoteTooLong { max: NOTE_MAX }`; inaczej `Ok(trimmed.to_owned())` (puste i same białe znaki → `""`). Kolejność sprawdzeń jak w `own_tasks.rs:122-132`. Komentarz modułu: wartość nigdy nie trafia do błędu ani do logu.

#### 2. Warianty błędu

**File**: `src-tauri/src/work_time/error.rs`

**Intent**: Dwie nowe odmowy, które nazywają regułę i limit, nigdy wartość.

**Contract**: `#[error("note is longer than {max} characters")] NoteTooLong { max: usize }` i `#[error("note has more than one line")] NoteMultiline`; gałęzie w `kind()` (`"NoteTooLong"`, `"NoteMultiline"`). JSON: `{"kind":"NoteTooLong","detail":{"max":200}}`, `{"kind":"NoteMultiline"}`.

#### 3. Domena: `add` i `update`

**File**: `src-tauri/src/work_time/mod.rs`

**Intent**: Notatka zapisuje się w tym samym `write` co wpis; edycja zastępuje notatkę.

**Contract**: `add(store, start_local, end_local, note: &str, now, tz) -> Result<i64, _>` i `update(store, id, start_local, end_local: Option<&str>, note: &str, now, tz) -> Result<(), _>`. Obie zaczynają od `let note = note::normalize_note(note)?;`. `INSERT INTO work_entries(start_ms, end_ms, note, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?4)`; `UPDATE work_entries SET start_ms = ?1, end_ms = ?2, note = ?3, updated_at_ms = ?4 WHERE id = ?5`. `start`, `stop`, `remove` bez zmian (START dostaje `DEFAULT ''`).

#### 4. Komendy

**File**: `src-tauri/src/work_time/commands.rs`

**Intent**: Wymagany parametr `note` w dwóch komendach wpisów; reszta pliku bez zmian (mały hunk pod merge z S-03).

**Contract**: `add_work_entry(app, start: String, end: String, note: String)` i `update_work_entry(app, id: i64, start: String, end: Option<String>, note: String)` przekazują `&note` do domeny. Żadnego nowego `log::`.

#### 5. Istniejące wywołania w testach

**File**: `src-tauri/src/work_time/tests.rs`, `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/tests/work_time.rs`

**Intent**: Dostosować wywołania do nowych sygnatur bez zmiany ich znaczenia i rozszerzyć migawkę tabeli o notatkę, żeby odmowy dowodziły, że notatka też się nie zmieniła.

**Contract**: Wszystkie wywołania `add`/`update` z listy w Current State Analysis dostają `""` (helpery `add_entry`, `add` w `tests/work_time.rs:151-153` i `norm_tests.rs:45` też). `Snapshot` w `tests.rs:30-52` dostaje `note` jako siódmy, ostatni element (`SELECT …, updated_at_ms, note`); istniejące dostępy po indeksach 0–5 zostają, ewentualne pełne destrukturyzacje wskazane przez kompilator poprawić.

#### 6. Nowe testy domeny

**File**: `src-tauri/src/work_time/note.rs` (moduł `tests`), `src-tauri/src/work_time/tests.rs`, `src-tauri/tests/work_time.rs`

**Intent**: Udowodnić regułę, zapis w jednym `write`, zachowanie instantów i brak wycieku wartości.

**Contract**:
- `note.rs`: `"  Wymyślona notatka  "` → `"Wymyślona notatka"`; `""`, `"   "`, `"\t\n "` → `""`; `"\nabc\n"` → `"abc"` (łamania na brzegach znikają przy przycięciu); 200 × „ż” i 200 × „👍” → `Ok` (znaki, nie bajty ani jednostki UTF-16); 201 × „ż” → `NoteTooLong { max: 200 }`; `"a\nb"`, `"a\r\nb"`, `"a\rb"` → `NoteMultiline`; `errors_never_contain_the_note`: dla notatki-sentinela za długiej i wieloliniowej `format!("{e} {e:?} {}", serde_json::to_string(&e))` oraz `e.kind()` nie zawierają sentinela (wzór `own_tasks.rs:498-515`); kształt JSON obu wariantów.
- `tests.rs`: `add` z `"  Notatka A "` → `entries_for_day` zwraca `"Notatka A"`; `update` zmienia samą notatkę i zachowuje `start_ms`/`end_ms` z sekundami (wpis z `start` w `t(...) + 12_345`, potem `stop`, potem `update` z tymi samymi czasami lokalnymi i nową notatką); `update` z `""` czyści notatkę; notatka na trwającym wpisie (`update(id, start, None, "Trwa", …)`) i wpis nadal trwa; `start` po wpisach z notatką tworzy wpis z `""`; `NoteTooLong` i `NoteMultiline` na `add` i `update` przez helper `refused` (tabela, łącznie z notatką, bez zmian); `add` z błędnym czasem (koniec ≤ początek) i wieloliniową notatką zwraca `NoteMultiline` (kolejność walidacji).
- `tests/work_time.rs`: notatka przeżywa zamknięcie i ponowne otwarcie store'u; `note_never_reaches_the_event_log`: `add` i `update` z notatką-sentinelem nie dodają zdarzeń (`events::list_today` puste), potem `start`/`stop` dodają dwa zdarzenia, a `SELECT count(*) FROM events WHERE coalesce(reference,'') || coalesce(title,'') || coalesce(source_label,'') || coalesce(error_kind,'') LIKE '%' || ?1 || '%'` z sentinelem daje 0; odmowy z notatką-sentinelem (`Overlap` przy `add`, `NotFound` przy `update` usuniętego wpisu) mają `to_string()`, `{:?}`, JSON i `kind()` bez sentinela.

#### 7. Lustro TS błędów i API

**File**: `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: Lustro nowych wariantów i nowego kontraktu `invoke` w tym samym commicie co Rust.

**Contract**: `WorkTimeError` dostaje `| { kind: "NoteTooLong"; detail: { max: number } } | { kind: "NoteMultiline" }`. `addWorkEntry(start: string, end: string, note: string)` → `invoke("add_work_entry", { start, end, note })`; `updateWorkEntry(id: number, start: string, end: string | null, note: string)` → `invoke("update_work_entry", { id, start, end, note })`. `workTimeErrorMessage`: `NoteTooLong` → `` `Notatka może mieć najwyżej ${err.detail.max} znaków` ``, `NoteMultiline` → „Notatka musi mieścić się w jednej linii”.

#### 8. Hook i formularz

**File**: `src/worktime/useWorkEntries.ts`, `src/worktime/WorkTimeEntries.tsx`, `src/worktime/WorkTime.css`

**Intent**: Pole notatki w obu formularzach; dopisywanie zaczyna od pustej notatki, edycja od zapisanej. `WorkTimeView.tsx` zostaje bez zmian (typy płyną z hooka).

**Contract**:
- `useWorkEntries.ts`: `add(start, end, note)` i `update(id, start, end, note)` przekazują `note` do API.
- `WorkTimeEntries.tsx`: stała `ENTRY_NOTE_MAX = 200` z komentarzem „same as Rust `work_time::note::NOTE_MAX`; no `maxLength` on purpose, Rust is the only limit” i lokalny `charCount = (v: string) => [...v].length` (bez importu z `own-tasks`). `SaveFn = (start, end: string | null, note: string) => Promise<unknown>`. `EntryForm` dostaje prop `initialNote: string`, stan `note` i po polu „Koniec” (przed `.wt-form-actions`) blok `<div className="wt-note-field">` z `Field label="Notatka (opcjonalnie)"` (domyślny jednoliniowy input, `aria-describedby` na licznik z `useId`) i `<span className="meta wt-note-count">{charCount(note)}/{ENTRY_NOTE_MAX}</span>`; `submit` przekazuje `note` bez przycinania (przycina Rust). Edycja: `initialNote={row.note}`, `onUpdate(row.id, start, end, note)`; dopisywanie: `initialNote=""`, `add(start, end ?? "", note)`. Typy `onUpdate` w `EntryItem` i `add`/`update` w `LoadedEntries`/`EntriesPanel` dostają `note: string`.
- `WorkTime.css`: `.wt-note-field { flex: 1 1 100%; display: grid; gap: var(--space-1); }` i `.wt-note-count { justify-self: end; font-family: var(--font-mono); font-variant-numeric: tabular-nums; color: var(--ink-faint); }` — tylko tokeny (wzór `.own-counter`, `OwnTasks.css:129-134`).

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym testy jednostkowe `work_time::note` (przycięcie, pusta, 200/201 znaków wielobajtowych i emoji, `\n`/`\r\n`/`\r`, brak wycieku, kształt JSON), nowe testy w `src-tauri/src/work_time/tests.rs` (zapis, zachowanie instantów, czyszczenie, trwający wpis, START bez notatki, odmowy bez zmian w tabeli, kolejność walidacji) i w `src-tauri/tests/work_time.rs` (trwałość, brak notatki w dzienniku zdarzeń, odmowy bez sentinela)
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -n "log::" -- src-tauri/src/work_time` pokazuje dokładnie trzy linie, wszystkie w `commands.rs` (dotychczasowe: dwa logi rodzaju błędu i błąd emitu)
- `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css` nic nie znajduje, a `git grep -n "invoke" -- src/worktime` znajduje `invoke` tylko w `src/worktime/api.ts`

#### Manual Verification:

- „Dopisz brakujący wpis” z notatką „Wymyślona notatka” zapisuje wpis, a lista pokazuje notatkę w linii pod godzinami; wpis bez notatki nie ma dodatkowej linii
- Edycja wpisu pokazuje zapisaną notatkę; zmiana samej notatki nie zmienia godzin ani czasu trwania; wyczyszczenie pola usuwa linię notatki
- Notatka trwającego wpisu (po START) daje się ustawić w jego formularzu edycji, a licznik w nagłówku dalej biegnie
- Wklejenie ponad 200 znaków daje komunikat „Notatka może mieć najwyżej 200 znaków” i nic się nie zapisuje; licznik pokazuje np. `201/200`
- Długa notatka w wąskim oknie (800 px) ucina się wielokropkiem, pełna treść jest w podpowiedzi po najechaniu
- Notatka zostaje po restarcie aplikacji; plik logu aplikacji z tej sesji i panel „Dziennik zdarzeń” nie zawierają jej treści

**Implementation Note**: Po zielonej weryfikacji automatycznej commit tylko tej fazy, np. `feat(work-entry-note): add and edit the note on a work-time entry`. Wiersze Manual zostają dla użytkownika.

---

## Phase 3: Dowód awaryjny i dokumentacja

### Overview

Dowód z harnessu awarii, że zapis wpisu z notatką nigdy nie zostawia rozdartej ani utraconej notatki, oraz aktualizacja AGENTS.md i CHANGELOG.

### Changes Required:

#### 1. Test awarii zapisu notatki

**File**: `src-tauri/tests/work_entry_note_crash.rs` (nowy)

**Intent**: Reguła „Data slices reuse the crash harness” dla nowej kolumny. Nowy plik zamiast rozszerzania `work_time_crash.rs`, bo nie koliduje z równoległymi slice'ami i ma własny prosty model.

**Contract**: Wzór `tests/days_off_crash.rs`: ignorowany test `writer_process` (`WRITER_DIR_ENV`) i `crash_keeps_entry_notes_consistent` z tą samą listą `KillAt`. Pisarz: otwiera store, liczy istniejące wiersze `k`, w pętli do `MAX_WRITER_COMMITS` woła `work_time::add(&mut store, start_local, end_local, &note_for(k), NOW_MS, &chrono::Utc)` dla wpisu `k` o początku `BASE_MS + k × 2 min` i końcu `+1 min` (lokalne `YYYY-MM-DDTHH:MM` w UTC), raportuje `committed {k+1}`. `note_for(k)` to wymyślona, jednoliniowa notatka z polskimi znakami i numerem, np. `format!("Wymyślona notatka {k} — żółć")`. `NOW_MS` stały, późniejszy od każdego możliwego wpisu (np. `BASE_MS + 400 dni`). Asercje po każdym zabiciu: `PRAGMA quick_check == "ok"`; liczba wierszy ≥ zgłoszona; wiersze po `id` to dokładnie wpisy `0..n` modelu: właściwe `start_ms`/`end_ms` i `note == note_for(k)` (nic rozdartego), każda notatka ≤ `NOTE_MAX` znaków i bez `\r`/`\n`. Tylko wymyślone dane.

#### 2. AGENTS.md

**File**: `AGENTS.md` (linia 11, reguła „Outside folders configured in settings…”)

**Intent**: Decyzja użytkownika 1: wymienić notatki wpisów w danych katalogu danych aplikacji i usunąć sprzeczność zdania „Never user content or vault data there.” z notatkami, które użytkownik już tam trzyma. Edycja tylko tej linii, minimalna.

**Contract**: `work-time entries,` → `work-time entries (start, end and an optional one-line note the user typed),`. Zdanie `Never user content or vault data there.` → `Nothing else there: no vault data and no work-source content beyond what is listed. Notes the user types (own tasks, work-time entries) stay in the app data dir, never in the app log or the event log.` Reszta linii bez zmian.

#### 3. CHANGELOG

**File**: `CHANGELOG.md`

**Intent**: Wpis pod `## [Unreleased]` (bez podbijania wersji).

**Contract**: Pod `## [Unreleased]` sekcja `### Added` z linią: `- "Czas pracy": a work-time entry can have a short one-line note (up to 200 characters). Set it when adding or editing the entry; the day's entry list shows it under the times.` Jeśli przy merge'u inna gałąź już dodała `### Added` w `[Unreleased]`, dopisać linię pod nią.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym `tests/work_entry_note_crash.rs` (wszystkie rundy zabicia, `quick_check`, brak utraconego commita, notatki zgodne z modelem)
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -n "optional one-line note" -- AGENTS.md` i `git grep -n "one-line note" -- CHANGELOG.md` znajdują nowe brzmienie, a `git grep -n "Never user content or vault data there" -- AGENTS.md` nic nie znajduje

**Implementation Note**: Po zielonej weryfikacji automatycznej commit tylko tej fazy, np. `test(work-entry-note): crash proof and docs`. Nie podbijać wersji w `Cargo.toml`.

---

## Testing Strategy

### Unit Tests:

- `work_time::note::normalize_note`: przycięcie, pusta i same białe znaki → `""`, łamania na brzegach usuwane przycięciem, wewnętrzne `\n`/`\r\n`/`\r` → `NoteMultiline`, granica 200/201 znaków liczona w znakach (polskie znaki, emoji).
- Brak wartości w błędach: Display, Debug, JSON i `kind()` bez sentinela; kształt JSON nowych wariantów.
- `work_time` (`src-tauri/src/work_time/tests.rs`): zapis i odczyt notatki, zastąpienie i wyczyszczenie przy edycji, zachowanie instantów, trwający wpis, START bez notatki, odmowy bez zmian w tabeli, pierwszeństwo błędu notatki.

### Integration Tests:

- `tests/work_time.rs`: upgrade v10→v11 z wierszami zamkniętym, trwającym i usuniętym; CHECK na surowym SQL (200/201 znaków, `\n`, `\r`); błąd CHECK bez wartości; trwałość po ponownym otwarciu; brak notatki w dzienniku zdarzeń; odmowy bez sentinela.
- `tests/work_entry_note_crash.rs`: harness z `tests/common/crash.rs`, model „wpis k z notatką k”.
- Tylko wymyślone dane („Wymyślona notatka”, `SENTINEL-invented-…`).

### Manual Testing Steps:

0. Zamknij zainstalowaną aplikację i skopiuj katalog danych aplikacji na bok (procedura „Ręczne przebiegi dzielą prawdziwą bazę”); po ostatnim kroku przywróć kopię, jeśli wydanie z v11 nie jest jeszcze zainstalowane.
1. Uruchom wersję z fazą 1 na prawdziwych danych: wpisy jak dotąd, „Dane aplikacji” sprawne.
2. Po fazie 2: dopisz wpis z notatką, popraw samą notatkę, wyczyść ją; ustaw notatkę trwającemu wpisowi.
3. Wklej ponad 200 znaków: komunikat i brak zapisu.
4. Zwęź okno do 800 px: wielokropek i podpowiedź.
5. Zrestartuj aplikację: notatka zostaje. Przejrzyj plik logu aplikacji i panel „Dziennik zdarzeń”: bez treści notatki.

## Performance Considerations

Jedna kolumna TEXT ≤ 200 znaków na wiersz; zapytania dnia i miesiąca czytają ją przy okazji tych samych SELECT-ów. Bez nowych indeksów.

## Migration Notes

`ALTER TABLE … ADD COLUMN` z `DEFAULT ''` nie przepisuje danych; istniejące wiersze dostają `''`, które spełnia CHECK. Dołączony SQLite to 3.54.1 (`libsqlite3-sys 0.40.0`, `SQLITE_VERSION` w jego `sqlite3/sqlite3.h`), więc CHECK dodanej kolumny jest sprawdzany na istniejących wierszach (od 3.37); test upgrade'u v10→v11 z danymi to utrwala. Build deweloperski migruje prawdziwą bazę: zob. „Ręczne przebiegi dzielą prawdziwą bazę”. Starsza wersja aplikacji odmówi bazy v11 (`NewerSchema`), jak przy poprzednich migracjach. Przywracanie kopii sprzed v11 działa jak dotąd (migracja przy otwarciu). Kopie zapasowe w `backups/` zawierają notatki, tak jak inne dane aplikacji.

## References

- Research: `context/changes/work-entry-note/research.md`
- Roadmap: `context/foundation/roadmap.md` (S-06, M-3), PRD MS-27, GitHub issue #27
- Precedens notatki i testów wycieku: `src-tauri/src/own_tasks.rs:122-141`, `:498-515`, `src-tauri/tests/store.rs:290-307`, `context/archive/2026-11-23-own-tasks/plan.md`
- Wzór testu upgrade'u: `src-tauri/tests/events.rs:62-96`; harness awarii: `src-tauri/tests/common/crash.rs`, `src-tauri/tests/days_off_crash.rs`
- Pole z licznikiem: `src/own-tasks/OwnTaskEditPanel.tsx:76-106`, `src/own-tasks/labels.ts:31-37`
- Design system Konsola: https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY

## Decisions (defaults chosen by agent, user may revise)

Zmiana była planowana bez dostępu do użytkownika. Decyzje użytkownika (stałe): notatki wpisów dopisane do listy w AGENTS.md w tej zmianie; slice zostaje przy MS-27 bez nowych FR w PRD; notatka w typie wpisu w Rust czytanym przez raport miesiąca (pod S-05). Poniżej wartości domyślne agenta:

1. Limit 200 znaków liczony w Rust przez `chars().count()`; CHECK w SQL jako druga linia obrony; UI bez `maxLength`, z licznikiem liczącym punkty kodowe.
2. Jedna linia: po przycięciu `\r` lub `\n` → odmowa `NoteMultiline` (bez cichego sklejania). Inne separatory Unicode (np. U+2028) nie są odrzucane.
3. Przycięcie z obu stron; pusta notatka = `""` (nie `NULL`), w JSON zawsze `note: string`.
4. `note: String` wymagany w `add_work_entry` i `update_work_entry`; edycja zastępuje notatkę; START i przyszły zasobnik (S-03) tworzą wpis z `""`.
5. Walidacja notatki przed walidacją czasów (błąd notatki ma pierwszeństwo).
6. Nagłówek, przypomnienia i dziennik zdarzeń nie pokazują notatki.
7. Wyświetlanie: osobna linia pod godzinami, klasa tekstowa `.body` z `tokens.css`, kolor przez istniejący token `var(--ink-faint)` w `WorkTime.css` (precedens `.wt-duration`), wielokropek, pełna treść w `title`. Bez literalnych kolorów.
8. AGENTS.md:11: zdanie „Never user content or vault data there.” zastąpione doprecyzowaniem: tylko wymienione dane, bez danych vaulta i treści źródeł pracy, a notatki użytkownika (zadania, wpisy) tylko w katalogu danych, nigdy w logu ani dzienniku zdarzeń.
9. Test awarii w nowym pliku `tests/work_entry_note_crash.rs` (zamiast rozszerzania `work_time_crash.rs`) — najmniej konfliktów przy merge'u.
10. Reguła w osobnym pliku `work_time/note.rs`, żeby nie powiększać hunków w `mod.rs`, który mogą ruszać równoległe slice'y.
11. Bez zmian w README i bez synchronizacji statusu roadmapy (zakaz edycji `roadmap.md` w tej zmianie).
12. Trzy fazy: odczyt (bez zmiany komend) → zapis z kontraktem i UI w jednym commicie → dowód awaryjny i dokumentacja.
13. (Przegląd planu, default chosen by agent, user may revise) Ręczne przebiegi na prawdziwych danych tylko z kopią katalogu danych odłożoną przed pierwszym przebiegiem i przywróconą po testach, dopóki wydanie z v11 nie jest zainstalowane (precedens own-tasks).
14. (Przegląd planu, default chosen by agent, user may revise) Gałąź scalana druga dopasowuje nowe wywołania `work_time::add`/`update` z drugiej gałęzi (`""`) i ewentualny numer migracji; nowe reguły CSS stoją obok `.wt-row-error` i `.wt-form-actions`.

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Kolumna notatki i ścieżka odczytu

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy migracji v10→v11, ograniczenia CHECK i błędu bez wartości w `tests/work_time.rs`, `schema::validate(MIGRATIONS)` oraz istniejące testy upgrade'u (`tests/events.rs`, `tests/hidden_issues.rs`, `tests/new_assignments.rs`, `tests/todo.rs`)
- [x] 1.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- [x] 1.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- [x] 1.4 `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css` nic nie znajduje

#### Manual

- [x] 1.5 Po pierwszym uruchomieniu wersji z v11 na prawdziwych danych użytkownika (z kopią katalogu danych wg „Ręczne przebiegi dzielą prawdziwą bazę”) widok „Czas pracy” pokazuje dotychczasowe wpisy bez zmian i bez dodatkowej linii pod godzinami, a „Stan źródeł” pokazuje „Dane aplikacji” jako sprawne — checked by the user on 2026-11-26 on m3-wave1-integration

### Phase 2: Zapis notatki — walidacja, komendy i formularz

#### Automated

- [x] 2.1 `cargo test` w `src-tauri/` przechodzi, w tym testy jednostkowe `work_time::note` (przycięcie, pusta, 200/201 znaków wielobajtowych i emoji, `\n`/`\r\n`/`\r`, brak wycieku, kształt JSON), nowe testy w `src-tauri/src/work_time/tests.rs` (zapis, zachowanie instantów, czyszczenie, trwający wpis, START bez notatki, odmowy bez zmian w tabeli, kolejność walidacji) i w `src-tauri/tests/work_time.rs` (trwałość, brak notatki w dzienniku zdarzeń, odmowy bez sentinela)
- [x] 2.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- [x] 2.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- [x] 2.4 `git grep -n "log::" -- src-tauri/src/work_time` pokazuje dokładnie trzy linie, wszystkie w `commands.rs` (dotychczasowe: dwa logi rodzaju błędu i błąd emitu)
- [x] 2.5 `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css` nic nie znajduje, a `git grep -n "invoke" -- src/worktime` znajduje `invoke` tylko w `src/worktime/api.ts`

#### Manual

- [x] 2.6 „Dopisz brakujący wpis” z notatką „Wymyślona notatka” zapisuje wpis, a lista pokazuje notatkę w linii pod godzinami; wpis bez notatki nie ma dodatkowej linii — checked by the user on 2026-11-26 on m3-wave1-integration
- [x] 2.7 Edycja wpisu pokazuje zapisaną notatkę; zmiana samej notatki nie zmienia godzin ani czasu trwania; wyczyszczenie pola usuwa linię notatki — checked by the user on 2026-11-26 on m3-wave1-integration
- [x] 2.8 Notatka trwającego wpisu (po START) daje się ustawić w jego formularzu edycji, a licznik w nagłówku dalej biegnie — checked by the user on 2026-11-26 on m3-wave1-integration
- [x] 2.9 Wklejenie ponad 200 znaków daje komunikat „Notatka może mieć najwyżej 200 znaków” i nic się nie zapisuje; licznik pokazuje np. `201/200` — checked by the user on 2026-11-26 on m3-wave1-integration
- [x] 2.10 Długa notatka w wąskim oknie (800 px) ucina się wielokropkiem, pełna treść jest w podpowiedzi po najechaniu — checked by the user on 2026-11-26 on m3-wave1-integration
- [x] 2.11 Notatka zostaje po restarcie aplikacji; plik logu aplikacji z tej sesji i panel „Dziennik zdarzeń” nie zawierają jej treści — checked by the user on 2026-11-26 on m3-wave1-integration

### Phase 3: Dowód awaryjny i dokumentacja

#### Automated

- [x] 3.1 `cargo test` w `src-tauri/` przechodzi, w tym `tests/work_entry_note_crash.rs` (wszystkie rundy zabicia, `quick_check`, brak utraconego commita, notatki zgodne z modelem)
- [x] 3.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- [x] 3.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- [x] 3.4 `git grep -n "optional one-line note" -- AGENTS.md` i `git grep -n "one-line note" -- CHANGELOG.md` znajdują nowe brzmienie, a `git grep -n "Never user content or vault data there" -- AGENTS.md` nic nie znajduje
