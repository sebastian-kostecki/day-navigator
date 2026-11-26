---
date: 2026-11-26T12:18:40+01:00
researcher: Claude (Opus 5.5) dla Sebastian Kostecki
git_commit: 455cbeda5879b666ed52021fe1b2bc2a02748807
branch: worktree-agent-a52bf1d753e9204cc
repository: day-navigator
topic: "Notatka przy wpisie czasu pracy (S-06, MS-27, issue #27)"
tags: [research, work-time, store, migration, event-log, logging, worktime-ui]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (Opus 5.5)
---

# Research: Notatka przy wpisie czasu pracy (S-06, MS-27, issue #27)

**Date**: 2026-11-26T12:18:40+01:00
**Researcher**: Claude (Opus 5.5) dla Sebastian Kostecki
**Git Commit**: 455cbeda5879b666ed52021fe1b2bc2a02748807 (równy `develop`: `git log HEAD..develop` jest pusty)
**Branch**: worktree-agent-a52bf1d753e9204cc
**Repository**: day-navigator

## Research Question

Co trzeba zmienić, żeby użytkownik mógł dodać, edytować i zobaczyć krótką, opcjonalną notatkę bez formatowania przy wpisie czasu pracy (przy dopisywaniu i przy edycji wpisu), widoczną na liście wpisów? Źródła: roadmapa S-06 (`context/foundation/roadmap.md:162-173`), issue #27 (`gh issue view 27`). Wymagania: nowa migracja dopisana na końcu `src-tauri/src/store/schema.rs`; notatka nie trafia do logu aplikacji ani do dziennika zdarzeń; błędy store'u nie niosą jej wartości. Obszary: moduł `work_time` w Rust (wpisy, komendy, dostęp do store'u, walidacja, typy raportu miesiąca), store (migracje, `StoreHandle`, błędy), ścieżki logu i dziennika zdarzeń, harness awarii, UI w `src/worktime/`, tokeny i konwencje, reguły AGENTS.md oraz konflikty z równoległymi slice'ami (S-01, S-03, S-05).

### Decyzje użytkownika (przekazane przez agenta nadrzędnego, nie do ponownego pytania)

1. W tej samej zmianie dopisać notatki wpisów do listy danych w katalogu danych aplikacji w AGENTS.md (reguła „Outside folders configured in settings…”, `AGENTS.md:11`).
2. Slice zostaje zakotwiczony jako MS-27. **Nie** dodajemy nowych FR do PRD.
3. Wartości domyślne agenta, do zapisania: limit długości sprawdzany w Rust (np. 200 znaków), jedna linia, przycięte białe znaki, pusta notatka oznacza brak notatki.
4. Projekt: późniejszy slice S-05 (month-report-copy) skopiuje raport miesiąca do schowka razem z notatką. Notatka ma więc należeć do typu wpisu w Rust, którego używa raport miesiąca.

## Summary

- **Gdzie siedzi wpis.** Tabela `work_entries` powstała w migracji v1 (`schema.rs:12-26`) i ma tylko instanty: `start_ms`, `end_ms`, `removed_at_ms`, `created_at_ms`, `updated_at_ms`.
  - Ostatnia wydana migracja to **v10** (`events`, `schema.rs:225-242`), więc notatka idzie jako **v11**.
  - `validate` wymaga ciągłej numeracji od 1 (`schema.rs:246-257`).
- **Typy w Rust do zmiany.** Są dwa:
  - Prywatny `Entry { id, start, end }` (`work_time/mod.rs:85-89`). Wypełnia go `row_to_entry`, który czyta kolumny po indeksie (`mod.rs:91-97`), a współdzielą go trzy zapytania: `counted_by_id`, `running_entry` i `in_day` (`mod.rs:100-141`).
  - Publiczny `EntryRow` (`mod.rs:58-67`), który trafia do UI przez `entries_for_day` (`mod.rs:224-231`).
  - Raport miesiąca (`report::compute`, `report.rs:111-166`) czyta wpisy przez `in_day` (`report.rs:125`). `DayReport` ma dziś tylko `entry_count` (`report.rs:45-57`) i nie zawiera listy wpisów.
  - Wniosek: notatka w `Entry` i w `EntryRow` spełnia decyzję 4. S-05 dostaje notatkę z tego samego odczytu `in_day`, bez drugiego zapytania.
- **Zapis.** Notatka powinna wejść jako nowy parametr do istniejących `add` (`mod.rs:297-321`) i `update` (`mod.rs:339-370`) oraz komend `add_work_entry` i `update_work_entry` (`commands.rs:89-116`). Wtedy wpis z notatką to jeden `store.write`. `start` (START, `mod.rs:259-276`) wstawia wpis bez notatki, czyli dostaje wartość domyślną z kolumny.
- **Wycieki: stan obecny jest czysty.** Sprawdzone ścieżki:
  - Log: w `work_time` są trzy wywołania `log::` i wszystkie piszą `e.kind()` albo błąd emitu (`commands.rs:36-39`, `:47`). Żadne nie zawiera pól wpisu ani argumentów komendy.
  - Dziennik zdarzeń: dostaje tylko `WorkStarted`/`WorkStopped` bez danych (`mod.rs:273`, `:291`). `add`, `update` i `remove` nie zapisują zdarzeń.
  - Błędy: `WorkTimeError` nie ma pól tekstowych (`error.rs:3-4`, `:10-35`). `StoreError` usuwa wartości wierszy (`store/error.rs:3-5`, `:48-59`).
  - Ryzyko pojawia się dopiero przy nowym kodzie: `{:?}` wpisu w logu, pole w `NewEvent` albo wariant błędu niosący tekst. W `work_time` nie ma testu wycieku, który by temu zapobiegał. Wzór do skopiowania: `own_tasks.rs:498-515` i `tests/store.rs:290-307`.
- **Precedens w kodzie: notatka zadania własnego.** Ma typ `note: String`, pusty łańcuch `''` oznacza brak notatki, kolumna to `TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 2000)` (`schema.rs:34`), a walidacja jest tylko w Rust (`own_tasks.rs:122-141`). W UI pola nie mają `maxLength`, a obok jest licznik znaków (`src/own-tasks/labels.ts:31-37`, `OwnTaskEditPanel.tsx:76-106`). Notatka wpisu może to powtórzyć, w wersji jednoliniowej jak tytuł zadania.
- **UI.**
  - Jeden komponent `EntryForm` obsługuje dopisywanie i edycję (`WorkTimeEntries.tsx:31-92`). Wiersz listy to `<li className="wt-row">` (`:142-188`).
  - Do zmiany: `SaveFn` (`:29`), propsy `add`/`update` (`:101`, `:196-197`, `:261-262`), wywołania (`:127`, `:239`), `useWorkEntries.ts:113-121`, `api.ts:32-39` i komunikaty w `api.ts:115-148`, `types.ts:8-19` i `:53-61` oraz nowa klasa w `WorkTime.css`.
  - `WorkTimeView.tsx` i `lib.rs` nie wymagają zmian: komendy są już zarejestrowane. To dobrze dla merge'u z S-01 i S-03.

## Detailed Findings

### Store: migracje, `StoreHandle`, błędy

- Migracje są listą `MIGRATIONS` (`schema.rs:9-243`). Komentarz modułu mówi, że wydanej migracji się nie edytuje (`schema.rs:3`). Po scaleniu równoległych slice'ów numery bywały przesuwane, co komentarze odnotowują: v5 (`schema.rs:91-92`), v6 (`:104-105`) i v8 (`:149-150`).
- Proponowana v11, wzorowana na v2 i v4 (`schema.rs:34`, `:86-89`):
  ```sql
  ALTER TABLE work_entries ADD COLUMN note TEXT NOT NULL DEFAULT ''
    CHECK (length(note) <= 200 AND instr(note, char(10)) = 0 AND instr(note, char(13)) = 0);
  ```
  - CHECK to druga linia obrony, tak jak w `own_tasks` (archiwum: `context/archive/2026-11-23-own-tasks/plan.md:44`). SQLite `length()` dla TEXT liczy znaki, a nie bajty, więc zgadza się z `chars().count()` w Rust.
  - Wartość `''` zamiast `NULL` ma dwa powody: jest spójna z `own_tasks.note` i oszczędza `Option` w TS.
  - Niepewność: SQLite sprawdza CHECK dodanej kolumny na istniejących wierszach od wersji 3.37. Repo używa `rusqlite 0.43` z funkcją `bundled` (`src-tauri/Cargo.toml:53`) i `libsqlite3-sys 0.40.0` (`Cargo.lock:2226-2227`). Wersji SQLite z tego pakietu nie odczytałem. Test migracji na store v10 z danymi rozstrzygnie to w praktyce.
- Wzór testu migracji: `tests/hidden_issues.rs:104-135` otwiera store przez `MIGRATIONS[..5]`, wstawia wpis do `work_entries`, otwiera ponownie z pełną listą i sprawdza `user_version` oraz liczbę wierszy. Dla v11 wystarczy `MIGRATIONS[..10]`, wpis, ponowne otwarcie i asercja, że `note = ''`. Inne użycia `MIGRATIONS[..n]` są w `tests/events.rs:71`, `tests/todo.rs:277` i `tests/new_assignments.rs:58`.
- Wymagany wzór dostępu do store'u już działa w module: komendy idą przez `blocking` i `StoreHandle::with_store` (`commands.rs:20-43`), a każda mutacja to jeden `store.write`, który czyta, waliduje i zapisuje (`mod.rs:5-9`).
- `StoreError`: niezmiennik jest w `store/error.rs:3-5`. `reason_of` (`:48-59`) zostawia kod i komunikat SQLite tylko dla `SqliteFailure`, a dla pozostałych wariantów zwraca stały tekst. Komunikat o złamaniu CHECK zawiera wyrażenie ograniczenia, a nie wartość. Warunek: notatka musi być bindowana jako parametr `?N`, nigdy wklejana do SQL przez `format!`.

### Moduł `work_time` (Rust)

- **Typy.**
  - `EntryRow` (`mod.rs:58-67`): `id`, `start_ms`, `end_ms`, `start_local`, `end_local`, `day_portion_ms`. Derive: `Debug, Clone, PartialEq, Eq, Serialize`, plus `rename_all = "camelCase"`. Dodanie `pub note: String` nie zmienia derive'ów.
  - `Entry` (`mod.rs:85-89`) jest prywatny. `row_to_entry` czyta po indeksach 0-2 (`mod.rs:91-97`), więc każde z trzech zapytań SELECT (`mod.rs:101-104`, `:110-113`, `:125-129`) musi dostać kolumnę `note` na tym samym indeksie. Inaczej trzeba mieć dwa mappery.
  - `RunningEntry` i `WorkTimerStatus` (`mod.rs:35-56`) nagłówka nie potrzebują notatki. Domyślnie zostają bez niej.
- **`add`** (`mod.rs:297-321`):
  - Kolejność działań: parsowanie czasów, `EndNotAfterStart`, `InFuture`, potem `write` z `find_overlap` i `INSERT` (`:314-318`).
  - Walidację notatki (czystą, bez store'u) wygodnie umieścić przed `write`.
  - Notatka idzie jako `?4` w `INSERT`.
- **`update`** (`mod.rs:339-370`):
  - Zastępuje `start_ms`, `end_ms` i `updated_at_ms` (`:364-367`).
  - `resolve_or_keep` (`mod.rs:325-335`) zachowuje zapisane instanty, gdy podany czas lokalny się nie zmienił. Dzięki temu edycja samej notatki nie przesuwa czasów.
  - Edycja samej notatki nadal przechodzi przez `find_overlap` i `InFuture` (`:357-363`). Dla istniejącego, poprawnego wpisu oba sprawdzenia przechodzą.
  - Trwający wpis (`end = None`, `:350-352`) da się edytować bez końca, więc da się też ustawić jego notatkę.
- **`start` i `stop`** (`mod.rs:259-294`) nie dotykają notatki. `INSERT` w `start` (`:267-271`) nie podaje kolumny, więc dostaje `DEFAULT ''`.
- **`remove`** (`mod.rs:373-385`) to miękkie usunięcie. Notatka zostaje w wierszu, który przestaje się liczyć. To zgodne z obecnym modelem, a backupy i tak zawierają cały wiersz.
- **Komendy** (`commands.rs:89-116`): `add_work_entry(app, start: String, end: String)` i `update_work_entry(app, id, start, end: Option<String>)`. Nowy parametr `note: String` zmienia kontrakt `invoke`, więc `api.ts` musi zmienić się w tym samym commicie (reguła AGENTS.md o kontraktach). Rejestracja komend w `lib.rs` się nie zmienia.
- **Błędy** (`error.rs:10-54`): proponowane nowe warianty to `NoteTooLong { max: usize }` i `NoteMultiline`, z gałęziami w `kind()`. Odpowiadają `OwnTaskError::NoteTooLong` i `TitleMultiline` (`own_tasks.rs:89-94`). `rename_all_fields = "camelCase"` (`error.rs:11`) daje w TS `detail: { max }`. Warianty niosą tylko limit, a niezmiennik z `error.rs:3-4` zostaje zachowany.
- **Raport miesiąca** (`report.rs`): `compute` iteruje `in_day` i liczy `entry_count` (`report.rs:125-142`). Notatka w `Entry` niczego tu nie zmienia, ale będzie dostępna dla S-05. Według roadmapy (`roadmap.md:159`) S-05 ma brać tekst z tego samego raportu w Rust.
- **Testy, które trzeba poprawić po zmianie sygnatur** (wywołania `add` i `update`): `src/work_time/tests.rs:186`, `:264`, `:350`, `:358`, `:552` oraz `tests/work_time.rs:98`, `:152`, `:178`. Ta lista pochodzi z grepa po wzorcach `add(&mut`, `update(&mut` i `work_time::add`. Pomocnik w `tests/work_time.rs:152` może przekazywać `""`.

### Wycieki: log aplikacji, dziennik zdarzeń, błędy

- **Log aplikacji.**
  - Konfiguracja: `tauri-plugin-log` z celem LogDir oraz Stdout w buildzie debug (`lib.rs:130-132`). Poziom globalny to Warn, a `day_navigator_lib` ma Info (`lib.rs:146-153`). Dzięki globalnemu Warn wewnętrzne logi Tauri na poziomie debug lub trace (np. IPC) nie trafiają do pliku.
  - `work_time` ma trzy wywołania `log::`, wszystkie w `commands.rs`: `:37` (warn, `e.kind()`), `:39` (debug, `e.kind()`) i `:47` (błąd emitu z ładunkiem `()`). Komentarz modułu: „Errors are logged by kind only.” (`commands.rs:6-7`).
  - Typy wpisu derive'ują `Debug` (`mod.rs:35`, `:43`, `:58`, `:69`; `report.rs:21`, `:29`, `:45`), ale nic ich dziś nie formatuje przez `{:?}`.
  - Frontend nie używa `console.*` ani `@tauri-apps/plugin-log` (zbadano `src/`, `package.json` i `capabilities/default.json`).
- **Dziennik zdarzeń.**
  - Wiersz ma kolumny `at`, `kind`, `item_source`, `item_kind`, `reference`, `title`, `source_id`, `source_label` i `error_kind` (`schema.rs:227-241`, `events/mod.rs:184-194`).
  - `NewEvent::WorkStarted` i `WorkStopped` nie niosą danych (`events/mod.rs:81-82`, `:223-230`).
  - Plan dziennika zdarzeń świadomie pominął zdarzenia dla edycji i usuwania wpisu (`context/archive/2026-11-24-event-log/plan.md:47`).
  - Notatka mogłaby tam trafić tylko przez nowe pole lub nowy wariant `NewEvent`. Na sprawdzonej ścieżce nie ma wspólnego typu między `Entry` a `NewEvent`.
- **Inne wyjścia.** Treść przypomnień (toast) używa godzin, a nie tekstu wpisu (`work_time/reminders.rs:128-132`, wg raportu podagenta). Notatka nie powinna tam trafić. Emit `work-time-changed` nie ma ładunku (`commands.rs:45-49`).
- **Testy wycieku do skopiowania.**
  - `own_tasks.rs:498-515` (`errors_never_contain_submitted_values`): sprawdza ``format!("{e} {e:?} {}", serde_json::to_string(&e))`` i asertuje brak sekretu.
  - `tests/store.rs:290-307` (`constraint_error_carries_no_row_values`): wartość SENTINEL nie może pojawić się w JSON błędu ani w `to_string()` błędu ograniczenia.
  - Podobne testy: `todo/tests.rs:907-927`, `hidden_issues/tests.rs:506`, `settings.rs:1650-1654`.
  - W `work_time` takiego testu nie ma.

### Harness awarii

- `tests/common/crash.rs:45-116` uruchamia ponownie bieżącą binarkę testową jako ignorowany test-pisarz i zabija go po N commitach albo po upływie czasu.
- `tests/work_time_crash.rs` zapętla START/STOP (`:49-81`) i sprawdza `quick_check`, maksymalnie jeden trwający wpis, brak nakładania się i brak utraconego commita (`:83-131`). Jego `rows()` czyta jawne kolumny (`:31-41`), więc nowa kolumna go nie psuje.
- Reguła AGENTS.md („Data slices reuse the crash harness”): wzorem jest `tests/days_off_crash.rs:1-30`. Najprostsze rozszerzenie to pisarz, który w pętli robi `update` notatki na zamkniętym wpisie (albo `add` z notatką). Asercje: notatka każdego wiersza mieści się w limicie, jest jednoliniowa i jest jedną z wartości, które pisarz zgłosił. Wybór między nowym plikiem `work_entry_note_crash.rs` a rozszerzeniem `work_time_crash.rs` należy do planu.

### UI (`src/worktime/`)

- **Formularz.**
  - `EntryForm` (`WorkTimeEntries.tsx:31-92`) trzyma `start`, `end`, `saving` i `error` (`:41-44`).
  - Pola to `Field type="datetime-local"`: „Początek” i opcjonalnie „Koniec” (`:58-75`). Przyciski są w `.wt-form-actions` (`:76-83`), a błąd w `Notice tone="error" compact` (`:85-89`).
  - Formularz nie ma walidacji po stronie klienta poza `required`. Reguły zwraca Rust jako `WorkTimeError`.
- **Gdzie wstawić notatkę.**
  - Stan `note` i prop `initialNote`: pusty przy dopisywaniu, `row.note` przy edycji.
  - Jednoliniowe `Field label="Notatka"` (domyślny `<input type="text" className="dn-input">`, `src/components/Field/Field.tsx:62-66`) po polu „Koniec”, z licznikiem `charCount(note)/MAX` jak przy tytule zadania (`OwnTaskEditPanel.tsx:76-90`, `own-counter` w `OwnTasks.css:129-134`).
  - Bez `maxLength`: przeglądarka ucina wklejki i liczy jednostki UTF-16, a limitem jest Rust (`labels.ts:31-37`, archiwum `own-tasks/plan.md:260`).
  - Reguła `.wt-form-fields .dn-field { flex: 1 1 180px }` (`WorkTime.css:195`) położy notatkę w zawijanym rzędzie. Dla pełnej szerokości może być potrzebna osobna klasa z `flex-basis: 100%`.
- **Edycja** (`WorkTimeEntries.tsx:117-134`): wpis trwający ma `showEnd=false`, ale może dostać pole notatki.
- **Dopisywanie** (`:229-248`) wywołuje `add(start, end ?? "")`. Trzeba dodać `note`.
- **Lista.**
  - Wiersz `.wt-row` (`WorkTimeEntries.tsx:142-188`) ma `flex-wrap: wrap` (`WorkTime.css:109-117`). Notatka jako osobny element z `flex-basis: 100%` (wzór `.wt-row-error`, `WorkTime.css:150-152`) albo inline po `.wt-range`.
  - Wzór wyświetlania: `{task.note && <span className="own-row-note">…}` (`OwnTasksView.tsx:146-147`) oraz `.own-row-note`: `--ink-faint`, 13px, ellipsis (`OwnTasks.css:66-75`).
  - `WorkTime.css` jest opisany jako „layout only” (`WorkTime.css:1`), a AGENTS.md mówi, że view CSS to tylko layout. Kolor i rozmiar tekstu można więc wziąć z klasy tekstowej z `tokens.css`, np. `.meta` (`tokens.css:179-183`), albo dopuścić `color: var(--ink-faint)`, jak robi już `.wt-duration` (`WorkTime.css:137-142`). Do decyzji w planie, w zgodzie z design systemem.
- **Typy i API.**
  - `EntryRow` (`types.ts:53-61`) dostaje `note: string`. `WorkTimeError` (`types.ts:8-19`) dostaje `{ kind: "NoteTooLong"; detail: { max: number } }` i `{ kind: "NoteMultiline" }`.
  - `addWorkEntry` i `updateWorkEntry` (`api.ts:32-39`) dostają `note`.
  - `workTimeErrorMessage` (`api.ts:115-148`) dostaje polskie komunikaty. Wzór: „Notatka może mieć najwyżej ${max} znaków.” z `ownTaskErrorMessage`.
  - Przypadek `default` pokazuje tylko `kind`, a przypadek `!err` pokazuje `String(e)` w UI. To nie jest log.
- **Hook**: `useWorkEntries.ts:113-121` (`add` i `update` przez `withBusy`).
- **Testy UI**: repo nie ma runnera testów UI (AGENTS.md, sekcja Testing). Weryfikacja UI jest ręczna.

### AGENTS.md i dokumentacja

- `AGENTS.md:11` wymienia zawartość katalogu danych aplikacji, w tym „work-time entries”. Decyzja 1: dopisać tam notatkę, np. „work-time entries (start, end and an optional short note)”.
- Ta sama linia kończy się zdaniem „Never user content or vault data there.” Notatki zadań własnych to już dziś treść użytkownika trzymana w app data. Zdanie jest więc niejednoznaczne: albo dotyczy logu i katalogów konfiguracji, albo jest nieaktualne. Przy dopisywaniu notatki plan powinien to doprecyzować, nie zostawiać sprzeczności. Zostawiam to jako otwarte pytanie z domyślną propozycją.
- `context/foundation/roadmap.md:80` (Baseline) mówi, że „work-time entries carry only start and end”. Roadmapy nie edytujemy w tej zmianie. Zaktualizuje ją zamknięcie slice'a lub `/10x-roadmap`.
- `CHANGELOG.md:12` (`[Unreleased]`) to miejsce na wpis „Added”. Każdy równoległy slice dopisuje tam linię, co grozi trywialnym konfliktem przy merge'u. README (`README.md:53`) opisuje „Czas pracy” ogólnie, a dopisek o notatce jest opcjonalny.

### Konflikty z równoległymi slice'ami (S-01, S-03, S-05)

- **S-06 nie musi dotykać** `lib.rs` ani `WorkTimeView.tsx` (o ile nazwy propsów `add`/`update` zostaną). To główne pliki S-01 i S-03.
- **Pliki S-06:** `schema.rs`, `work_time/{mod.rs, error.rs, commands.rs, tests.rs}`, `tests/work_time.rs`, nowy lub rozszerzony test awarii, `src/worktime/{WorkTimeEntries.tsx, useWorkEntries.ts, api.ts, types.ts, WorkTime.css}`, `AGENTS.md:11` i `CHANGELOG.md`.
- **Możliwe nakładki:**
  - S-01 (święta w normie): `types.ts` (`DayKind`, `DayReport` i `MonthReport` w `:73-129`), `WorkTime.css` (obszar `.wt-days`), `report.rs` i `norm.rs`. Hunki S-06 są w innych regionach. Jeśli S-01 doda wariant `DayKind`, dotknie też `useWorkEntries.ts:137`.
  - S-03 (tray): `commands.rs` (START/STOP z zasobnika), `api.ts` i `types.ts` w regionie `WorkTimerStatus` (`types.ts:40-51`, `api.ts:23-29`, `:155-164`). S-06 zmienia w `commands.rs` tylko `add_work_entry` i `update_work_entry`.
  - Migracja: według agenta nadrzędnego S-06 jako jedyny równoległy slice dodaje migrację. Gdyby jednak inna gałąź zajęła v11, istnieje precedens przenumerowania (`schema.rs:91-92`, `:104-105`, `:149-150`).
  - S-05: roadmapa mówi wprost: „if S-06 lands first, S-05 copies the note too” (`roadmap.md:70`).

## Code References

- `src-tauri/src/store/schema.rs:12-26` - v1 `work_entries`
- `src-tauri/src/store/schema.rs:34` - wzór kolumny `note` (own_tasks v2)
- `src-tauri/src/store/schema.rs:225-246` - ostatnia migracja v10 i `validate`
- `src-tauri/src/work_time/mod.rs:58-67` - `EntryRow`
- `src-tauri/src/work_time/mod.rs:85-141` - `Entry`, `row_to_entry` i trzy zapytania SELECT
- `src-tauri/src/work_time/mod.rs:205-242` - `entries_for_day`
- `src-tauri/src/work_time/mod.rs:259-294` - `start`/`stop` ze zdarzeniami w tym samym zapisie
- `src-tauri/src/work_time/mod.rs:297-370` - `add`, `resolve_or_keep`, `update`
- `src-tauri/src/work_time/commands.rs:20-49` - `run` (logowanie po `kind`) i `notify`
- `src-tauri/src/work_time/commands.rs:89-116` - `add_work_entry`, `update_work_entry`
- `src-tauri/src/work_time/error.rs:3-54` - niezmiennik, warianty, `kind()`
- `src-tauri/src/work_time/report.rs:45-57`, `:111-166` - `DayReport` i `compute` przez `in_day`
- `src-tauri/src/store/error.rs:3-5`, `:48-59` - niezmiennik `StoreError`, `reason_of`
- `src-tauri/src/events/mod.rs:81-82`, `:223-230` - `WorkStarted`/`WorkStopped` bez danych
- `src-tauri/src/lib.rs:146-153` - poziomy logu
- `src-tauri/src/own_tasks.rs:15-16`, `:82-141`, `:498-515` - limity, błędy, walidacja, test wycieku
- `src-tauri/tests/store.rs:290-307` - test „błąd ograniczenia bez wartości”
- `src-tauri/tests/hidden_issues.rs:104-135` - wzór testu migracji na starszym store
- `src-tauri/tests/common/crash.rs:45-116`, `src-tauri/tests/work_time_crash.rs:31-131` - harness awarii
- `src/worktime/WorkTimeEntries.tsx:29-92`, `:94-189`, `:191-253` - formularz, wiersz, lista
- `src/worktime/useWorkEntries.ts:113-121` - `add`/`update`
- `src/worktime/api.ts:32-39`, `:115-148` - invoke i komunikaty błędów
- `src/worktime/types.ts:8-19`, `:53-61` - `WorkTimeError`, `EntryRow`
- `src/worktime/WorkTime.css:109-152`, `:177-202` - wiersz i formularz
- `src/own-tasks/labels.ts:31-37`, `src/own-tasks/OwnTaskEditPanel.tsx:76-106`, `src/own-tasks/OwnTasks.css:66-75` - wzór pola, licznika i wyświetlania
- `AGENTS.md:11` - lista danych w katalogu danych aplikacji

## Architecture Insights

- Reguły domenowe są w Rust, a UI tylko wyświetla wyniki (AGENTS.md). Walidacja notatki (przycięcie, jedna linia, limit) należy więc do `work_time`, najlepiej jako czysta funkcja `normalize_note(&str) -> Result<String, WorkTimeError>` z testami jednostkowymi. CHECK w SQL to druga linia obrony.
- Jedna mutacja to jeden `store.write` (`mod.rs:5-9`). Dlatego notatka wchodzi w istniejące `add` i `update`, a nie w osobną komendę `set_note`. Osobna komenda dzieliłaby „dopisz wpis z notatką” na dwa zapisy.
- Błędy nazywają regułę i limit, nigdy wartość (`own_tasks.rs:3-4`, `error.rs:3-4`). Ten sam niezmiennik obowiązuje nowe warianty.
- Typy wpisu derive'ują `Debug`. Zabezpieczeniem jest konwencja, że nic nie formatuje wpisu przez `{:?}` w logu, a nie typ. Test wycieku dla błędów oraz jawny punkt przeglądu („żadne `log::` nie formatuje wpisu ani notatki”, jak w `own-tasks/plan.md:208`) utrwalają tę konwencję.

## Historical Context (from prior changes)

- `context/archive/2026-11-23-own-tasks/plan-brief.md:28` - tytuł ma 1–200 znaków i jedną linię, notatka ≤ 2000 znaków z zachowanymi łamaniami linii (popiera wzór limitu i jednej linii)
- `context/archive/2026-11-23-own-tasks/plan.md:131-133` - limity w znakach. Tytuł jest przycinany, `\r`/`\n` daje `TitleMultiline`, notatka z samych białych znaków staje się `""`
- `context/archive/2026-11-23-own-tasks/plan.md:44` - CHECK w SQL jako druga linia obrony
- `context/archive/2026-11-23-own-tasks/plan.md:87` - żadnej treści użytkownika w błędach i logach, nigdy `log::` pola zadania
- `context/archive/2026-11-23-own-tasks/plan.md:260`, `reviews/plan-review.md:74-86` - bez `maxLength` w UI, Rust jest jedynym limitem (poprawka z przeglądu planu F3)
- `context/archive/2026-11-24-event-log/plan.md:47` - brak zdarzeń dla edycji i usuwania wpisu czasu pracy
- `context/archive/2026-11-24-event-log/plan.md:32` - log aplikacji nie dostaje treści zdarzeń
- `context/archive/2026-11-24-event-log/plan.md:254` - wzór dopisywania do listy w `AGENTS.md:11` w tej samej zmianie

## Related Research

- `context/archive/2026-11-23-own-tasks/` - precedens notatki bez formatowania (plan i przegląd)
- `context/archive/2026-11-24-event-log/` - granice dziennika zdarzeń
- Wcześniejszych badań notatki wpisu czasu pracy nie ma: przeszukano `context/changes/` i `context/archive/` pod kątem notatek i `work_entries`.

## Defaults chosen by agent (user may revise)

Zmiana była prowadzona bez dostępu do użytkownika. Poniższe wartości to propozycje dla `/10x-plan`:

1. **Limit: 200 znaków**, liczony przez `chars().count()` w Rust, a w SQL przez `CHECK (length(note) <= 200)`. Komunikat błędu podaje limit.
2. **Jedna linia**: notatka z `\r` lub `\n` (po przycięciu) jest odrzucana jako `NoteMultiline`, zamiast cichego sklejania linii. Tak samo działa `TitleMultiline` w zadaniach własnych.
3. **Przycięcie i pusta notatka**: `trim()` z obu stron. Pusta notatka po przycięciu zapisuje się jako `''` i oznacza brak notatki. W JSON pole jest zawsze `note: string`, nigdy `null`.
4. **Kontrakt**: `note: String` jest wymagany w `add_work_entry` i `update_work_entry`, a edycja zastępuje notatkę. START (nagłówek i przyszły zasobnik z S-03) tworzy wpis z `''`.
5. **Trwający wpis**: notatkę można ustawić w jego formularzu edycji.
6. **Kolejność walidacji**: najpierw notatka (czysta, przed `store.write`), potem czasy. Do zmiany w planie, gdyby komunikat o czasie miał mieć pierwszeństwo.
7. **Nagłówek** (`WorkTimerStatus`) nie pokazuje notatki.
8. **Wyświetlanie**: notatka pod zakresem godzin w wierszu, przyciemniona, ucinana wielokropkiem. Pełna treść jest w `title` lub w formularzu edycji.
9. **Testy**: jednostkowe dla normalizacji i limitu, test wycieku dla nowych wariantów błędu (wzór `own_tasks.rs:498-515`), test migracji v10 → v11 z danymi oraz test awarii z notatką (harness).

## Open Questions

1. **Brzmienie `AGENTS.md:11`**: zdanie „Never user content or vault data there.” wygląda na sprzeczne z już trzymanymi notatkami zadań i z nową notatką wpisu. Propozycja domyślna: przy dopisywaniu notatki doprecyzować je do „No vault data and no work-source content beyond what is listed”. Właściciel: użytkownik, rozstrzygnięcie w `/10x-plan`. Nie blokuje.
2. **Wersja SQLite z `bundled`**: zachowanie `ADD COLUMN … CHECK` na istniejących wierszach wymaga SQLite ≥ 3.37. Test migracji v10 → v11 rozstrzygnie to w implementacji. Nie blokuje.
3. **Styl notatki w `WorkTime.css`**: plik deklaruje „layout only”. Kolor i rozmiar notatki wziąć z klasy tekstowej z `tokens.css` (`.meta`) czy dopuścić `color: var(--ink-faint)`, jak robi już `.wt-duration`? Decyzja w planie, zgodnie z design systemem. Nie blokuje.
4. **Test awarii**: nowy plik czy rozszerzenie `work_time_crash.rs`. Decyzja w planie. Nie blokuje.
