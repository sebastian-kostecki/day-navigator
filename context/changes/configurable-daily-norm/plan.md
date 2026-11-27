# Konfigurowalna norma dzienna i pół dnia wolnego — plan implementacji

## Overview

Użytkownik ustawia normę dzienną w `settings.json` (`workTime.dailyNormHours`, domyślnie 8 h) i oznacza w tabeli dni „pół dnia wolnego” jako trzeci stan przełącznika dnia (roboczy → pół dnia → cały dzień). Raport liczy oba: normę miesiąca, odchylenie, „Średnio / dzień”, odchylenie miesiąca w statusie licznika, przypomnienia i kafelki pulpitu. Slice S-02 (M-3), MS-26, issue #26. Reguła zostaje w jednym module Rust (`work_time/norm.rs`) z testami jednostkowymi i proptestem; stan pół dnia jest zapisany w `days_off` (nowa migracja v12) i widoczny w typach raportu Rust (`DayKind::HalfDayOff`) dla S-05.

Jedna wartość normy, bez historii: zmiana w `settings.json` przelicza każdy miesiąc, także zamknięte (decyzja użytkownika; zapisana w README i w „Jak liczę”).

## Current State Analysis

Pełna mapa miejsc: `context/changes/configurable-daily-norm/research.md`. Najważniejsze:

- `norm.rs:14` — stała `DAILY_NORM_MS = 8 h`, czytana w `day_norm_ms` (`:49-59`) i dwa razy w `figures` (`:120`, `:139`): norma miesiąca to liczba dni × stała, więc pół dnia nie da się dziś wyrazić. `day_deviation_ms` (`:63-74`) już przyjmuje normę argumentem. `needed_per_day_ms` = brakujący czas / liczba dni roboczych po dziś (`:145-148`).
- Jedynym konsumentem `norm.rs` w produkcji jest `report::compute` (`report.rs:122-183`), który zasila `work_month_report` (kafelki „Czas pracy”, tabelę dni, kafelki pulpitu `src/todo/summary.ts:23-71`) i `status` (`mod.rs:178-215`, `month_deviation_ms`). Przypomnienie „licznik nie biegnie po GG:MM” działa tylko przy `DayKind::Workday` (`reminders.rs:144`).
- Dni wolne: tabela `days_off(day PK, created_at_ms)` z v5 (`schema.rs:90-100`), bez rodzaju. `set_day_off(store, day, off: bool, now, settings)` (`report.rs:211-238`) odmawia weekendu (`NotAWorkday`) i święta (`Holiday`) w obu kierunkach, przed zapisem. Jedna klasyfikacja `classify` (`report.rs:101-111`): weekend > święto > dzień wolny > roboczy. Ostatnia migracja: v11 (`schema.rs:247-251`).
- `WorkTimeSettings { holidays }` jest `Copy + Eq` (`settings/work_time.rs:13-16`), parsowany z `RawWorkTime` z `deny_unknown_fields` (`:31-36`); komentarz „S-02 adds its daily-norm field here” (`:12`). Odczyt na wywołanie przez `work_time_settings(path, Policy)` (`commands.rs:36-53`): Strict dla raportu i `set_day_off`, Lenient dla statusu, listy wpisów i przypomnień.
- TS: `DayKind` (`src/worktime/types.ts:79`), `MonthReport` (`:97-142`), `setDayOff(day, off: boolean)` (`api.ts:56-58`), komunikat `SettingsInvalid` tylko o `holidays` (`api.ts:149`). Dwa przełączniki boolowskie: tabela (`useDayOffToggle.ts`, `DaysTable.tsx:106-119`) i „Wybrany dzień” (`useWorkEntries.ts:124-162`, `WorkTimeView.tsx:150-169`). Tekst „Jak liczę” ma „8 h” na sztywno (`WorkTimeView.tsx:65`, `:71`, `:75`).
- Testy: `norm_tests.rs` (m.in. proptest „wiersze dni sumują się do liczb miesiąca” `:566-617`), `tests/work_time.rs:286` (`-8 * 3_600_000 * 7`), sześć literałów `WorkTimeSettings { .. }` (`settings/work_time.rs:20`, `:108`, `:114`; `norm_tests.rs:23`; `tests/common/mod.rs:20`; `tests/work_time.rs:302`). Wzór testu awarii: `tests/days_off_crash.rs`; wzór testu upgrade'u: `Store::open(&dir, &schema::MIGRATIONS[..n], …)` (`tests/hidden_issues.rs:110`, `tests/events.rs:71`).
- `HoursBar` ma domyślne `max = 10` h (`src/components/HoursBar/HoursBar.tsx:17`) — górna granica normy. Istnieje komponent `Segmented` (`src/components/Segmented/Segmented.tsx`) bez stanu `disabled`.

## Desired End State

- „Odchylenie w nagłówku” z outcome to pole statusu `WorkTimerStatus.monthDeviationMs` (`mod.rs:211`, liczone z `report::compute`), więc dostaje normę z ustawień i pół dnia bez osobnego kodu i jest pokryte testami (faza 1 pkt 6, faza 2 pkt 7). Nagłówek (`WorkTimerHeader.tsx`) pokazuje dziś tylko sumę dnia i ten slice nie dodaje do niego odchylenia.
- `"workTime": { "dailyNormHours": 7.5 }` daje normę 7:30 na dzień roboczy we wszystkich liczbach raportu, w statusie i w kafelkach pulpitu; brak klucza lub `null` = 8 h. Wartość spoza 1–10 h albo niebędąca pełną liczbą minut unieważnia blok `workTime`: raport i przełącznik dnia pokazują błąd (Strict), START/STOP działa dalej (Lenient).
- W tabeli dni i w panelu „Wybrany dzień” dzień roboczy ma trzystanowy przełącznik: roboczy / pół dnia wolnego / cały dzień wolny. Pół dnia ma normę równą połowie normy dziennej, pasek z kreską normy w połowie i dopisek „pół dnia wolnego”; cały dzień wolny jest zakreskowany jak dziś. Weekend i święto nie mają przełącznika i są odrzucane przez Rust.
- „Dni wolne” liczy pół dnia jako 0,5; „Średnio / dzień” podaje czas na pełny dzień roboczy (pół dnia wolnego wymaga połowy), a podpis mówi, ile z pozostałych dni to pół dnia.
- „Jak liczę” pokazuje bieżącą normę z raportu, opisuje pół dnia i mówi, że zmiana normy przelicza wszystkie miesiące.
- Baza v11 migruje do v12 bez utraty wierszy; istniejące dni wolne są całymi dniami.
- README, CHANGELOG `[Unreleased]` i sformułowania PRD o „8 h” opisują normę z ustawień, pół dnia i brak historii normy.
- Weryfikacja: bramki `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` zielone po każdej fazie.

### Key Discoveries:

- `figures` musi sumować normy dni zamiast mnożyć liczbę dni (`norm.rs:119-139`); po tej zmianie norma miesiąca, tygodnia (S-08) i dnia mają jedno źródło: `day_norm_ms` / `day_norms`.
- Klucz główny `days_off.day` daje wzajemne wykluczenie „cały / pół” bez reguły między tabelami; `ALTER TABLE … ADD COLUMN kind … DEFAULT 'full'` zostawia istniejące wiersze jako całe dni (wzór `ALTER` w v4 i v11).
- `reminders.rs:144` porównuje `!= DayKind::Workday`, więc nowy wariant `HalfDayOff` sam wycisza przypomnienie startu — wystarczy test.
- `Copy + Eq` w `WorkTimeSettings` wyklucza `f64`; norma trzymana jako całkowite ms, przeliczana z liczby JSON przy parsowaniu (precedens zakresu: `reminders.longRunningHours`, `settings.rs:370-379`).
- `useMonthReport` odświeża raport po focusie okna i co 60 s (`useMonthReport.ts:51-112`), więc ręczna edycja `settings.json` pojawia się bez restartu — jak przy kalendarzu świąt.
- Integracyjne asercje `user_version == MIGRATIONS.len()` (`tests/work_time.rs:383-385`) same przyjmą v12.

## What We're NOT Doing

- Brak historii normy i normy per miesiąc/okres (decyzja użytkownika): jedna wartość, działająca wstecz.
- Brak nowych FR w PRD; slice zostaje przy MS-26 (w PRD zmieniają się tylko istniejące sformułowania o 8 h).
- Brak edytora ustawień w UI: `settings.json` edytuje się ręcznie jak dotąd.
- Brak odchylenia w nagłówku: nagłówek pokazuje tylko sumę dnia (`WorkTimerHeader.tsx`); pokazanie odchylenia miesiąca należy do konsumenta (np. S-07). Ten slice zapewnia tylko, że `monthDeviationMs` liczy nową regułę.
- Brak pół dnia w weekend i w święto; brak „ćwiartek” i dowolnych godzin wolnych.
- Brak zmiany polityki Lenient: przy zepsutym bloku `workTime` status, lista wpisów dnia i przypomnienia liczą z domyślnymi 8 h (`commands.rs:47-50`); żaden widok nie pokazuje dziś `monthDeviationMs`. Konsument z S-07 decyduje sam o Strict.
- Brak zmian w `lib.rs` (nazwa komendy `set_day_off` zostaje), w `context/foundation/roadmap.md` (bez synchronizacji statusu przez `/10x-plan` — zakaz edycji w tej zmianie) i w `shape-notes.md` (historia).
- Brak nowego wariantu `WorkTimeError`: niepoprawny `mark` z UI odrzuca deserializacja Tauri, a TS go typuje.

## Implementation Approach

Cztery fazy, każda zielona sama w sobie i commitowana osobno; typ Rust i jego lustro TS zmieniają się zawsze w tej samej fazie:

1. **Norma z ustawień** — pole `daily_norm_ms` w `WorkTimeSettings`, klucz `dailyNormHours`, parametr normy w czystych funkcjach `norm.rs`, `figures` liczone z norm dni, `MonthReport.dailyNormMs` (Rust + TS) i „Jak liczę” bez „8 h”.
2. **Pół dnia w danych i regule** — migracja v12, `DayOff { Half, Full }` w `norm.rs`, `DayKind::HalfDayOff` (Rust + TS), `DayMark` jako wejście `report::set_day_off`, `half_days_off` i `half_days_after_today` w raporcie (Rust + TS), test awarii. Komenda Tauri dalej przyjmuje `off: bool` (mapowane na `DayMark`), więc UI kompiluje się bez zmian.
3. **Trzystanowy przełącznik w UI** — komenda `set_day_off(day, mark)` i `setDayOff(day, mark)` w jednym commicie, oba przełączniki na `Segmented`, kafelki „Dni wolne” i „Średnio / dzień”, tekst o pół dnia.
4. **Dokumentacja** — README, CHANGELOG, PRD.

Edycje plików współdzielonych z równoległymi slice'ami (`settings/work_time.rs`, `report.rs`, `WorkTimeView.tsx`, `DaysTable.tsx`, README, CHANGELOG) są małymi, lokalnymi hunkami; nowy kod idzie do nowych plików tam, gdzie to możliwe (`tests/half_day_off_crash.rs`).

## Critical Implementation Details

- **Średnio / dzień na pełny dzień.** `needed_per_day_ms = ceil_to_minute(ceil(remaining × daily_norm / norm_after_today))`, gdzie `norm_after_today` = suma norm dni po dziś. Bez pół dni to dokładnie dzisiejszy wynik (`ceil(remaining / after)`), więc istniejące testy liczb zostają. Iloczyn licz w `i128` (albo z udokumentowaną granicą), żeby clippy i overflow nie były tematem. Przykład kontrolny: norma 8 h, po dziś 2 pełne dni + 1 pół dnia (20 h), brakuje 25 h → 10:00 na pełny dzień (10 + 10 + 5 = 25).
- **Pół normy jest dokładne.** Norma ma pełne minuty, więc `daily_norm_ms / 2` jest całkowite (30 s × minuty); proptest wiersze=miesiąc nie traci ms. Przy nieparzystej liczbie minut pół dnia ma 30 s nadwyżki ponad pełną minutę (7:01 → 3:30:30 = 12_630_000 ms): Rust liczy dokładnie w ms, a UI pokazuje jak dziś (`formatHm` obcina do minuty, czyli 3:30; odchylenie poniżej minuty to „0:00”). Bez zaokrąglania normy w Rust.
- **Średnio / dzień — warunek kontrolny.** Dla `after > 0` i `remaining > 0`: `needed × norm_after_today ≥ remaining × daily_norm` oraz `(needed − 1 min) × norm_after_today < remaining × daily_norm` (w `i128`); `needed` to najmniejsza pełna minuta spełniająca pierwszy warunek. Ten warunek sprawdza proptest (faza 1 pkt 5, faza 2 pkt 7).
- **Kolejność w `classify`:** weekend > święto > cały dzień wolny > pół dnia > roboczy. Wiersz `days_off` (całego lub pół dnia) w święto jest zacieniony, nie usuwany, jak dziś (`report.rs:42-43`, `:178`).
- **Zapis stanu w jednym `write`:** `Workday` → `DELETE`; `HalfDayOff`/`DayOff` → `INSERT … ON CONFLICT(day) DO UPDATE SET kind = excluded.kind` (wartości zawsze jako parametry `?N`, `created_at_ms` pierwszego oznaczenia zostaje). Idempotentne.
- **Ręczne przebiegi dzielą prawdziwą bazę.** Każdy worktree, `npm run tauri dev` i zainstalowane wydanie używają identyfikatora `com.sebas.day-navigator` (`tauri.conf.json:4`). Po pierwszym przebiegu z v12 wydanie bez v12 odmawia bazy jako `NewerSchema`, a po ewentualnym przenumerowaniu migracji przy merge'u baza z `kind` jako v12 nie przejdzie innej v12 (`MigrationFailed`) i zostanie odrzucona, nigdy zresetowana. Przed pierwszym ręcznym przebiegiem fazy 2/3 zamknąć aplikację i skopiować cały katalog danych aplikacji na bok; po testach przywrócić kopię (`day-navigator.db*` i `backups/`), dopóki nie jest zainstalowane wydanie z v12 (precedens `context/archive/2026-11-26-work-entry-note/plan.md`, „Ręczne przebiegi dzielą prawdziwą bazę”). Agenci nie uruchamiają `npm run tauri dev` (port 1420 jest wspólny); wiersze Manual są dla użytkownika.

## Phase 1: Norma dzienna z settings.json

### Overview

Norma dzienna przestaje być stałą: przychodzi z `workTime.dailyNormHours` (domyślnie 8 h) przez `WorkTimeSettings` do czystych funkcji `norm.rs`, a `figures` sumuje normy dni. Raport niesie normę dzienną do UI, a „Jak liczę” przestaje wypisywać „8 h”.

### Changes Required:

#### 1. Ustawienie normy

**File**: `src-tauri/src/settings/work_time.rs`

**Intent**: Dodać do `WorkTimeSettings` normę dzienną w ms i klucz `dailyNormHours` w `RawWorkTime`, z walidacją zakresu i pełnych minut; zaktualizować komentarz bloku i doc parsera.

**Contract**: `WorkTimeSettings { holidays: HolidayCalendar, daily_norm_ms: i64 }`, nadal `Copy + PartialEq + Eq`, `Default` = `Pl` + `norm::DEFAULT_DAILY_NORM_MS`. `RawWorkTime` dostaje `#[serde(default)] daily_norm_hours: Option<serde_json::Value>`. Reguła: brak / `null` → domyślne; liczba JSON `h`, dla której `h × 60` jest całkowite (tolerancja zaokrąglenia ~1e-6) i mieści się w 60..=600 minut → `minutes × 60_000`; cokolwiek innego (string, bool, tablica, 0, 0.99, 10.01, 7.33, ujemne) → `WorkTimeSettingsInvalid`. Stałe zakresu (`DAILY_NORM_MIN_MINUTES = 60`, `DAILY_NORM_MAX_MINUTES = 600`) w tym pliku. Testy: domyślne 8 h; 1, 7.5, 8, 10 przyjęte (7.5 → 27_000_000); wartości odrzucone jak wyżej; razem z `holidays: false`; literówka klucza nadal odrzucona; zepsuty `dailyNormHours` nie rusza bloków `reminders`/`todo`. Helpery `pl()`/`off()` przez `..WorkTimeSettings::default()`.

#### 2. Reguła normy z parametrem

**File**: `src-tauri/src/work_time/norm.rs`

**Intent**: Zastąpić stałą `DAILY_NORM_MS` domyślną `DEFAULT_DAILY_NORM_MS` i przekazywać normę dzienną parametrem; `figures` liczy normę miesiąca i normę do dziś jako sumę norm dni, a „Średnio / dzień” jako czas na pełny dzień (patrz „Critical Implementation Details”). Dodać publiczny helper norm dni zakresu dla S-08.

**Contract**:
- `pub const DEFAULT_DAILY_NORM_MS: i64 = 8 * 3_600_000;` (bez `DAILY_NORM_MS`).
- `pub fn day_norm_ms(day, days_off: &BTreeSet<NaiveDate>, holidays: HolidayCalendar, daily_norm_ms: i64) -> i64`.
- `pub fn day_norms(first, end_exclusive, days_off, holidays, daily_norm_ms) -> Vec<(NaiveDate, i64)>` — dni zakresu z normą > 0, rosnąco; dowolny zakres (miesiąc, tydzień). Doc: „jedno źródło norm dla miesiąca i tygodnia”.
- `pub fn figures(day_norms: &[(NaiveDate, i64)], today, phase, worked_ms, daily_norm_ms) -> NormFigures`; `NormFigures` bez zmian pól. `workdays` = liczba dni z normą > 0; `workdays_after_today` = takie dni po dziś.
- `workdays(...)` zostaje z obecną sygnaturą (nie zależy od normy; używają go testy `norm_tests.rs:109-132`, `:630-639`); `report::compute` przestaje go wołać i bierze `day_norms`.

#### 3. Raport z normą dzienną

**File**: `src-tauri/src/work_time/report.rs`

**Intent**: `compute` bierze `settings.daily_norm_ms`, buduje wiersze dni i `figures` z `day_norms`; `MonthReport` niesie normę dzienną dla tekstu „Jak liczę”.

**Contract**: `MonthReport.daily_norm_ms: i64` (serde `dailyNormMs`). `DayReport` bez zmian.

#### 4. Literały ustawień w testach

**File**: `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/tests/common/mod.rs`, `src-tauri/tests/work_time.rs`

**Intent**: Literały `WorkTimeSettings { holidays: … }` uzupełnić przez `..WorkTimeSettings::default()`; odwołania do `DAILY_NORM_MS` zamienić na `DEFAULT_DAILY_NORM_MS`; bezpośrednie wywołania `norm::figures` i `norm::day_norm_ms` w `norm_tests.rs` (np. `:163`, `:418-438`, `:625-650`) dostają normy dni i `DEFAULT_DAILY_NORM_MS`; `tests/work_time.rs:286` może zostać liczbą albo użyć stałej.

**Contract**: Istniejące przykłady liczbowe (8 h) przechodzą bez zmiany oczekiwań.

#### 5. Nowe testy normy

**File**: `src-tauri/src/work_time/norm_tests.rs`

**Intent**: Utrwalić normę inną niż 8 h na znanych datach i rozszerzyć proptest o losową normę.

**Contract**:
- Listopad 2026, kalendarz `pl`, norma 7.5 h, bez dni wolnych: 20 dni roboczych, `norm_ms = 150 h` (540_000_000); z `manual_only()` 21 dni = 157.5 h. Norma wiersza dnia roboczego = 7:30, weekendu/święta = 0.
- `needed_per_day_ms` przy normie 6 h w bieżącym miesiącu równa się wzorowi `ceil_to_minute(ceil(remaining / after))` (zgodność wsteczna dla samych pełnych dni).
- Proptest `day_rows_add_up_to_the_month_figures`: dodatkowy parametr `norm_minutes in 60..=600`, ustawienia z tą normą; asercje jak dziś (Σ worked, Σ odchyleń dni do dziś = odchylenie miesiąca, Σ norm dni = norma miesiąca) plus Σ norm dni ≤ dziś = `norm_to_date_ms`. Po zmianie `figures` suma wierszy = miesiąc jest prawdziwa z konstrukcji (oba liczą z `day_norm_ms`), więc proptest dostaje niezależną wyrocznię liczoną w teście z flag i kalendarza: `norm_ms = daily_norm × liczba dni pon–pt bez święta i bez oznaczenia` oraz warunek kontrolny „Średnio / dzień” z „Critical Implementation Details”.

#### 6. Testy komend i statusu

**File**: `src-tauri/src/work_time/commands.rs` (testy), `src-tauri/tests/work_time.rs`

**Intent**: Strict/Lenient obejmują nowy klucz: poprawny `dailyNormHours` czytany w obu politykach, niepoprawny → `SettingsInvalid` (Strict) / domyślne (Lenient). Integracyjnie: `status.month_deviation_ms` przy normie 7 h liczy 7 h × dni do dziś.

**Contract**: bez zmian sygnatur komend.

#### 7. Lustro TS i tekst „Jak liczę”

**File**: `src/worktime/types.ts`, `src/worktime/api.ts`, `src/worktime/WorkTimeView.tsx`

**Intent**: `MonthReportBase.dailyNormMs: number`. Komunikat `SettingsInvalid` wymienia też `dailyNormHours` (liczba godzin 1–10 w pełnych minutach, np. 7.5). `HowCounted` dostaje normę dzienną z raportu (`number | null`; przy ładowaniu/błędzie neutralne „norma dzienna z settings.json („workTime”)”, bez liczby: UI nie zna wartości domyślnej) i używa `formatHm` zamiast „8 h” w trzech akapitach; dopisuje zdanie, że zmiana normy przelicza wszystkie miesiące, także zamknięte.

**Contract**: `HowCounted({ dailyNormMs: number | null })`; żadnej liczby normy wyliczanej w TS.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy `settings::work_time` (zakres, pełne minuty, null/brak, izolacja bloków), testy normy 7.5 h na listopadzie 2026 i proptest z losową normą 60–600 min
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -nw "DAILY_NORM_MS" -- src-tauri` i `git grep -n "8 h" -- src/worktime` nic nie znajdują

#### Manual Verification:

- Z `"workTime": { "dailyNormHours": 7.5 }` w `settings.json` po powrocie do okna „Czas pracy” pokazuje „z … normy” jako 7:30 × dni robocze, pasek dnia roboczego ma kreskę normy na 7:30, a „Jak liczę” podaje 7:30; kafelki pulpitu pokazują te same liczby
- `"dailyNormHours": 11` (albo `"8"`) daje w „Czas pracy” błąd raportu z komunikatem o `dailyNormHours`, a kafelki pulpitu pokazują „raport niedostępny”; START/STOP w nagłówku działa dalej; usunięcie klucza przywraca 8 h

**Implementation Note**: Po fazie i zielonych bramkach automatycznych zatrzymaj się na ręczne potwierdzenie przez użytkownika. Bloki faz mają zwykłe punktory; checkboxy są tylko w `## Progress`.

---

## Phase 2: Pół dnia wolnego w danych i regule

### Overview

Dzień może być oznaczony jako cały albo pół dnia wolnego. Rodzaj zapisuje kolumna `days_off.kind` (v12); reguła normy liczy pół dnia jako połowę normy dziennej; raport i status rozróżniają `HalfDayOff`. Komenda Tauri jeszcze przyjmuje `off: bool`, więc UI kompiluje się bez zmian; nowy stan da się ustawić z UI dopiero w fazie 3.

### Changes Required:

#### 1. Migracja v12

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Dopisać migrację z rodzajem dnia wolnego; v5 zostaje nietknięta. Komentarz: S-02, „cały/pół dnia, klucz główny wyklucza oba naraz; planowana jako v12, przy merge'u bierze kolejny wolny numer”.

**Contract**: `Migration { version: 12, sql: "ALTER TABLE days_off ADD COLUMN kind TEXT NOT NULL DEFAULT 'full' CHECK (kind IN ('full','half'));" }`.

#### 2. Rodzaj dnia wolnego w regule

**File**: `src-tauri/src/work_time/norm.rs`

**Intent**: Mapa dni wolnych z rodzajem zamiast zbioru; pół dnia = połowa normy; pół dnia zostaje dniem z normą (liczy się w `workdays`). `figures` podaje liczbę pół dni po dziś.

**Contract**:
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DayOff { Half, Full }`, `pub type DaysOff = BTreeMap<NaiveDate, DayOff>`.
- `day_norm_ms(day, &DaysOff, holidays, daily_norm_ms)`: weekend / święto / `Full` → 0; `Half` → `daily_norm_ms / 2`; inaczej `daily_norm_ms`. `day_norms` i `workdays` przyjmują `&DaysOff` (pół dnia jest dniem roboczym).
- `NormFigures.half_days_after_today: Option<u32>` (serde `halfDaysAfterToday`; `Some` tylko w bieżącym miesiącu) — dni po dziś z normą mniejszą niż `daily_norm_ms`.

#### 3. Raport, klasyfikacja i zapis stanu

**File**: `src-tauri/src/work_time/report.rs`

**Intent**: Czytać `kind`, klasyfikować pół dnia, wystawić go w typach raportu (S-05) i przyjmować trzy stany w `set_day_off`.

**Contract**:
- `DayKind::HalfDayOff` (serde `"halfDayOff"`); `classify(day, mark: Option<DayOff>, holidays)` z kolejnością weekend > święto > `Full` → `DayOff` > `Half` → `HalfDayOff` > `Workday`.
- `days_off_between(...) -> DaysOff` (`SELECT day, kind …`; nieznany rodzaj lub zła data pominięte jak dziś zła data); `is_day_off` → `day_off(conn, day) -> Result<Option<DayOff>, _>`.
- `MonthReport.days_off` = tylko całe dni (bez zacienionych świętem); nowe `MonthReport.half_days_off: Vec<String>` (serde `halfDaysOff`), ten sam filtr i sortowanie.
- `pub enum DayMark { Workday, HalfDayOff, DayOff }` (`Debug, Clone, Copy, PartialEq, Eq`); `pub fn set_day_off(store, day, mark: DayMark, now, settings)` — odmowy weekendu i święta dla każdego `mark` przed zapisem; zapis wg „Critical Implementation Details”, jeden `store.write`. Doc: idempotentne.
- `is_workday` (tylko testy) zwraca `false` dla pół dnia — doc to mówi.

#### 4. Komenda bez zmiany kontraktu

**File**: `src-tauri/src/work_time/commands.rs`

**Intent**: `set_day_off(app, day, off: bool)` mapuje `true → DayMark::DayOff`, `false → DayMark::Workday`; kontrakt z UI zmienia się dopiero w fazie 3.

**Contract**: sygnatura komendy bez zmian.

#### 5. Lustro TS typów raportu

**File**: `src/worktime/types.ts`

**Intent**: Lustro nowych pól i wariantu w tym samym commicie co Rust.

**Contract**: `DayKind` += `"halfDayOff"`; `MonthReportBase.halfDaysOff: string[]` (doc: pół dni, zacienione świętem pominięte); w unii `MonthReport`: `halfDaysAfterToday: null` dla `past`/`future`, `number` dla `current`. Pozostałe pliki UI bez zmian (porównania `=== "dayOff"` dalej działają).

#### 6. Wywołania w testach

**File**: `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/tests/work_time.rs`, `src-tauri/tests/days_off_crash.rs`

**Intent**: `set_day_off(..., true/false, ...)` → `DayMark::DayOff` / `DayMark::Workday`; zbiory dni wolnych w testach czystych funkcji → `DaysOff`.

**Contract**: dotychczasowe oczekiwania bez zmian.

#### 7. Nowe testy reguły i zapisu

**File**: `src-tauri/src/work_time/norm_tests.rs`, `src-tauri/src/work_time/reminders_tests.rs`

**Intent**: Utrwalić pół dnia w regule, zapisie i przypomnieniach.

**Contract**:
- Norma 8 h, listopad 2026, `manual_only()`, pół dnia 2026-11-12: norma miesiąca 21 × 8 − 4 = 164 h, wiersz 12.11 `kind = HalfDayOff`, `norm_ms = 4 h`; to samo przy 7.5 h → wiersz 3:45; przy nieparzystej normie 7:01 (421 min) wiersz pół dnia = 12_630_000 ms (3:30:30), a norma miesiąca = 20 × 25_260_000 + 12_630_000.
- Status: `status(...).month_deviation_ms` przy pół dniu przed dziś równa się `figures.deviation_ms` raportu i liczy dla tego dnia połowę normy (rozszerzenie `status_month_deviation_equals_the_report_deviation`, `norm_tests.rs:323`).
- Przejścia `Workday → HalfDayOff → DayOff → Workday` i `DayOff → HalfDayOff` dają po jednym wierszu z właściwym `kind`; powtórzenie tego samego stanu nic nie zmienia; weekend i święto odrzucone dla każdego `mark` bez wiersza.
- Wiersz pół dnia w święto (oznaczony przy wyłączonym kalendarzu) jest zacieniony: `Holiday`, norma 0, brak w `half_days_off`.
- `needed_per_day_ms` na przykładzie kontrolnym (2 pełne + 1 pół dnia po dziś, brakuje 25 h → 10:00) i `half_days_after_today = Some(1)`.
- Proptest: `off_flags` jako losowy wybór z {brak, pół, cały} na dzień (przy losowej normie z fazy 1); te same sumy wiersze = miesiąc, a niezależna wyrocznia z fazy 1 rozszerzona o pół dni: `norm_ms = daily_norm × dni bez oznaczenia + daily_norm / 2 × pół dni` (dni pon–pt bez święta) i warunek kontrolny „Średnio / dzień” z `norm_after_today` liczonym w teście.
- `reminders_tests.rs`: `DayKind::HalfDayOff` na liście „cicho” przypomnienia startu.

#### 8. Upgrade i odmowa CHECK

**File**: `src-tauri/tests/work_time.rs`

**Intent**: Baza v11 z wymyślonymi dniami wolnymi po otwarciu z pełną listą migracji ma `user_version` = `MIGRATIONS.len()`, a jej wiersze są całymi dniami (`DayKind::DayOff`, `days_off` w raporcie). Bezpośredni `INSERT` z `kind = 'x'` kończy się błędem store, który nie niesie wartości (wzór `tests/store.rs:290-307`). Dzień oznaczony jako pół dnia przetrwa ponowne otwarcie.

**Contract**: wzór `Store::open(&dir, &schema::MIGRATIONS[..11], …)`.

#### 9. Test awarii

**File**: `src-tauri/tests/half_day_off_crash.rs` (nowy)

**Intent**: Dowód, że zabicie procesu oznaczającego pół i całe dni zostawia spójny store. Kopia kształtu `tests/days_off_crash.rs` (harness `tests/common/crash.rs`): pisarz oznacza kolejne wymyślone dni robocze, n-ty dzień jako `HalfDayOff` dla nieparzystych n, `DayOff` dla parzystych, jeden commit na dzień; rodzic w 10 rundach sprawdza `PRAGMA quick_check = ok`, liczba wierszy ≥ ostatni zgłoszony commit, wiersze to ciągły ciąg dni roboczych, a `kind` każdego wiersza zgadza się ze wzorem (nigdy brak ani inny rodzaj).

**Contract**: nowy plik; `days_off_crash.rs` zmienia się tylko o `DayMark` (pkt 6).

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy pół dnia w `norm_tests.rs` i `reminders_tests.rs`, upgrade v11→v12 i odmowa CHECK w `tests/work_time.rs`, `schema::validate(MIGRATIONS)`, istniejące testy upgrade'u (`tests/events.rs`, `tests/hidden_issues.rs`, `tests/new_assignments.rs`, `tests/todo.rs`) i nowy `tests/half_day_off_crash.rs`
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git diff <commit fazy 1>.. -- src-tauri/src/store/schema.rs` pokazuje tylko dopisaną migrację v12 (bez zmian w v1–v11)

#### Manual Verification:

- Po pierwszym uruchomieniu wersji z v12 na prawdziwych danych (z kopią katalogu danych wg „Ręczne przebiegi dzielą prawdziwą bazę”) „Czas pracy” pokazuje dotychczasowe dni wolne zakreskowane jak przedtem i te same liczby miesiąca, a „Stan źródeł” pokazuje „Dane aplikacji” jako sprawne

**Implementation Note**: Po fazie i zielonych bramkach automatycznych zatrzymaj się na ręczne potwierdzenie przez użytkownika.

---

## Phase 3: Trzystanowy przełącznik dnia w UI

### Overview

Komenda przyjmuje stan dnia zamiast boola, a tabela dni i panel „Wybrany dzień” mają jeden trzystanowy przełącznik (roboczy / pół dnia / cały dzień). Kafelki pokazują pół dni w „Dni wolne” i w podpisie „Średnio / dzień”.

### Changes Required:

#### 1. Kontrakt komendy

**File**: `src-tauri/src/work_time/report.rs`, `src-tauri/src/work_time/commands.rs`, `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: Wejście komendy staje się stanem dnia; Rust i TS w jednym commicie. Nazwa komendy i rejestracja w `lib.rs` bez zmian.

**Contract**: `DayMark` dostaje `Deserialize` z `#[serde(rename_all = "camelCase")]` (`"workday" | "halfDayOff" | "dayOff"`); `set_day_off(app, day: String, mark: DayMark)`. TS: `export type DayMark = "workday" | "halfDayOff" | "dayOff"`; `setDayOff(day: string, mark: DayMark)` → `invoke("set_day_off", { day, mark })`; doc o odmowach weekendu/święta zostaje. Test Rust: `serde_json::from_str::<DayMark>` dla trzech wartości i odmowa innej.

#### 2. `Segmented` z blokadą

**File**: `src/components/Segmented/Segmented.tsx`, `src/components/Segmented/Segmented.css`

**Intent**: Opcjonalny `disabled` (żądanie w toku) wyłącza przyciski; styl tylko z tokenów.

**Contract**: `SegmentedProps.disabled?: boolean` → `disabled` na każdym `button`; `.dn-seg-btn:disabled` z istniejących tokenów (`var(--ink-faint)`, bez literalnych kolorów), kursor domyślny.

#### 3. Stan przełącznika tabeli

**File**: `src/worktime/useDayOffToggle.ts`

**Intent**: Żądanie w toku na dzień to `DayMark`; zapisany stan to `row.kind` zawężony do trzech stanów; żądanie znika, gdy przeładowany raport się zgadza (bez cofania się przełącznika).

**Contract**: `DayOffToggle { mark(day): DayMark; busy(day): boolean; set(day, mark): Promise<void>; error }`.

#### 4. Tabela dni

**File**: `src/worktime/DaysTable.tsx`, `src/worktime/WorkTime.css`

**Intent**: Dla `workday`, `halfDayOff` i `dayOff` kolumna „Wolne” pokazuje `Segmented` (`size="sm"`, `label` „Dzień wolny DD.MM”, opcje „nie” / „½” / „tak”, `disabled` przy żądaniu w toku). Pół dnia nie jest zakreskowany; w komórce paska ma dopisek „pół dnia wolnego” (ten sam styl co nazwa święta); pasek dostaje normę wiersza jak dziś. Kolumna przełącznika szersza w `--dn-table-cols`: szerokość dobrana do trzech przycisków `Segmented size="sm"` (każdy ma `padding: 0 var(--space-3)`, czyli 24 px plus etykieta, więc około 120 px, a nie 104 px), a zwęża się kolumna paska `minmax(0, 1fr)`; sprawdzone w oknie 800 px.

**Contract**: `DaysTable` props bez zmian poza typem `dayOff: DayOffToggle`; tylko layout w `WorkTime.css`.

#### 5. Panel „Wybrany dzień”

**File**: `src/worktime/useWorkEntries.ts`, `src/worktime/WorkTimeView.tsx`

**Intent**: Checkbox „Dzień wolny” zastąpiony tym samym trzystanowym kontrolerem (`Segmented`, `label` „Dzień”, opcje „roboczy” / „pół dnia wolnego” / „wolny”), widocznym dla trzech stanów; logika żądania w toku jak dziś, na `DayMark`.

**Contract**: `useWorkEntries` zwraca `dayMark: DayMark`, `dayMarkBusy: boolean`, `setDayMark(mark): Promise<unknown>` zamiast `dayOffChecked`/`dayOffBusy`/`setDayOff`; `WorkTimeView` tylko podmienia kontroler i wywołanie.

#### 6. Kafelki

**File**: `src/worktime/format.ts`, `src/worktime/MonthReport.tsx`, `src/todo/summary.ts`

**Intent**: „Dni wolne” = całe + 0,5 × pół (polski przecinek, np. „2,5”), podpis „oznaczone ręcznie”, a przy pół dniach „w tym N × pół dnia”. Podpis „Średnio / dzień” przy pół dniach po dziś: „przez 5 dni roboczych, w tym 1 × pół dnia”; wartość to czas na pełny dzień. Jeden helper podpisu dzielony przez widok i pulpit.

**Contract**: `formatDaysAfterToday(workdays: number, halfDays: number): string` w `format.ts`; `formatWorkdays` bez zmian.

#### 7. „Jak liczę”

**File**: `src/worktime/WorkTimeView.tsx`

**Intent**: Dopisać, że pół dnia wolnego ma połowę normy dziennej, a „Średnio / dzień” to czas na pełny dzień roboczy (pół dnia wolnego — połowa).

**Contract**: tylko tekst `HowCounted`.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym test deserializacji `DayMark`
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css src/components/Segmented/Segmented.css` nic nie znajduje, a `git grep -n "invoke" -- src/worktime` znajduje `invoke` tylko w `src/worktime/api.ts`

#### Manual Verification:

- W tabeli dni dzień roboczy przełączony na „½” pokazuje dopisek „pół dnia wolnego”, kreskę normy w połowie paska i normę dnia o połowę mniejszą; norma miesiąca i odchylenie zmieniają się o pół normy; „tak” zakreskowuje dzień; „nie” przywraca dzień roboczy; przełącznik nie „odskakuje” w trakcie żądania
- Panel „Wybrany dzień” pokazuje ten sam stan co tabela i zmiana w jednym miejscu jest widoczna w drugim; weekend i święto nie mają przełącznika w obu miejscach
- „Dni wolne” pokazuje np. „1,5” z „w tym 1 × pół dnia”, a „Średnio / dzień” na pulpicie i w „Czas pracy” ma ten sam podpis z pół dniem po dziś
- Tabela dni mieści się bez poziomego przewijania w oknie 800 px; przełącznik działa z klawiatury (Tab, Enter/Spacja)
- W dzień oznaczony jako pół dnia wolnego przypomnienie „licznik nie biegnie po GG:MM” się nie pokazuje

**Implementation Note**: Po fazie i zielonych bramkach automatycznych zatrzymaj się na ręczne potwierdzenie przez użytkownika.

---

## Phase 4: Dokumentacja

### Overview

README, CHANGELOG i PRD opisują normę z ustawień, pół dnia wolnego i brak historii normy.

### Changes Required:

#### 1. README

**File**: `README.md`

**Intent**: `:55` — norma dzienna z ustawień (domyślnie 8 h) i pół dnia wolnego; przykład JSON `:111` z `"dailyNormHours": 8`; wiersz `workTime` `:123` z kluczem `dailyNormHours` (liczba godzin 1–10 w pełnych minutach, np. 7.5; brak = 8) i zdaniem, że jedna wartość działa wstecz: zmiana przelicza wszystkie miesiące, także zamknięte.

**Contract**: tylko te miejsca; `:125` (gdzie widać błąd bloku) bez zmian.

#### 2. CHANGELOG

**File**: `CHANGELOG.md`

**Intent**: Wpis w `[Unreleased]` → `### Added`: norma dzienna w `settings.json` (`workTime.dailyNormHours`, domyślnie 8 h, przelicza wszystkie miesiące) i pół dnia wolnego w tabeli dni i w panelu „Wybrany dzień” (połowa normy). Historyczne wpisy bez zmian.

**Contract**: jeden punkt (albo dwa) dopisany na końcu listy `Added`.

#### 3. PRD

**File**: `context/foundation/prd.md`

**Intent**: Zamienić „8 h” na normę dzienną z ustawień (domyślnie 8 h) w `:68`, `:117`, `:155` (FR-014), `:176` (FR-020: „pasek na tle normy dnia”, przełącznik dnia z pół dnia), `:205`, `:211` („Norma = suma norm dni roboczych: norma dzienna z ustawień, pół dnia wolnego liczy połowę”). Bez nowych FR, bez MS-26 w PRD.

**Contract**: zmiana sformułowań w istniejących punktach; numeracja FR bez zmian.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą; w `src-tauri/` `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` przechodzą
- `git grep -n "8 h" -- context/foundation/prd.md` znajduje tylko wzmianki „domyślnie 8 h”, a `git grep -n "dailyNormHours" -- README.md CHANGELOG.md` znajduje oba pliki
- `git diff <commit fazy 3>.. --stat` nie pokazuje `context/foundation/roadmap.md`

---

## Testing Strategy

### Unit Tests:

- Parser `dailyNormHours`: granice 1 i 10 h, pełne minuty (7.5 tak, 7.33 nie), typy niebędące liczbą, `null`/brak, izolacja od innych bloków.
- `norm.rs`: norma dnia dla czterech rodzajów dnia przy 8 h i 7.5 h; `day_norms` dla zakresu tygodnia przecinającego miesiąc (pod S-08); `figures` z pół dniami; „Średnio / dzień” na pełny dzień i zgodność wsteczna bez pół dni.
- Proptest: losowe wpisy, losowy stan dnia {roboczy, pół, cały}, losowa norma 60–600 min, kalendarz włączony/wyłączony → Σ wierszy = liczby miesiąca, norma miesiąca = niezależna wyrocznia z flag, „Średnio / dzień” spełnia warunek kontrolny.
- Status: `monthDeviationMs` z pół dniem równa się odchyleniu raportu (to jest „odchylenie w nagłówku” z outcome).
- `set_day_off` z `DayMark`: przejścia, idempotencja, odmowy weekendu i święta bez zapisu; klasyfikacja z zacienieniem przez święto.
- Przypomnienia: `HalfDayOff` wycisza przypomnienie startu.

### Integration Tests:

- `tests/work_time.rs`: status przy normie 7 h; upgrade v11→v12 z istniejącymi dniami wolnymi; CHECK `kind` bez wartości w błędzie; pół dnia przetrwa ponowne otwarcie.
- `tests/half_day_off_crash.rs`: 10 rund zabicia pisarza, spójność i rodzaj każdego wiersza.

### Manual Testing Steps:

1. Odłożyć kopię katalogu danych aplikacji; uruchomić wersję z gałęzi.
2. Ustawić `"dailyNormHours": 7.5`, wrócić do okna: liczby i „Jak liczę” w 7:30.
3. Ustawić błędną wartość (`11`): błąd raportu w „Czas pracy” i na pulpicie, START/STOP działa.
4. Oznaczyć dzień jako „½”, potem „tak”, potem „nie” w tabeli i w panelu „Wybrany dzień”; sprawdzić normę dnia, miesiąca, odchylenie, „Dni wolne” i „Średnio / dzień”.
5. Zamknąć aplikację i przywrócić kopię danych.

## Performance Considerations

Bez znaczenia: raport liczy ≤ 31 dni; jedna dodatkowa kolumna w odczycie `days_off` w tym samym `store.read`.

## Migration Notes

- v12 dopisana po v11; istniejące wiersze `days_off` dostają `kind = 'full'` (cały dzień, jak dotąd). Odinstalowanie zmiany wymaga przywrócenia kopii z `backups/` (baza v12 jest dla starszego wydania „nowsza” i odrzucana, nigdy resetowana).
- Brak ustawienia = 8 h, więc istniejące `settings.json` działają bez zmian.

## References

- Research: `context/changes/configurable-daily-norm/research.md`
- Plan świąt (wspólny `WorkTimeSettings`, Strict/Lenient): `context/archive/2026-11-26-public-holidays-norm/plan.md`
- Plan notatki (wzór faz, migracji `ALTER`, prawdziwa baza w ręcznych przebiegach): `context/archive/2026-11-26-work-entry-note/plan.md`
- Wzór testu awarii: `src-tauri/tests/days_off_crash.rs`, harness `src-tauri/tests/common/crash.rs`
- Wzór zakresu w ustawieniach: `src-tauri/src/settings.rs:370-379` (`longRunningHours`)
- Roadmapa: S-02 (`context/foundation/roadmap.md:106-118`), MS-26, issue #26

## Decisions (defaults chosen by agent, user may revise)

Plan powstał bez dostępu do użytkownika. Decyzje użytkownika (stałe): jedna norma bez historii (zmiana działa wstecz, zapisane w README i „Jak liczę”); pół dnia wolnego jako trzeci stan przełącznika dnia; rozszerzenie `WorkTimeSettings` i bloku `workTime`; MS-26 bez nowych FR, tylko sformułowania PRD o 8 h. Poniżej wartości domyślne agenta:

1. Klucz `workTime.dailyNormHours`: liczba godzin JSON, domyślnie 8, zakres 1–10 h, krok pełna minuta (7.5 tak, 7.33 nie); wewnętrznie całkowite ms (`Copy + Eq`). Górna granica 10 h = skala `HoursBar`. (research D1)
2. Pół dnia = dokładnie połowa normy dziennej. (D2)
3. Pół dnia odrzucany w weekend (`NotAWorkday`) i w święto (`Holiday`) jak cały dzień; wiersz pół dnia w święto jest zacieniony, nie usuwany. (D3)
4. Zapis: v12 `ALTER TABLE days_off ADD COLUMN kind TEXT NOT NULL DEFAULT 'full' CHECK (kind IN ('full','half'))`; odznaczenie usuwa wiersz; zmiana rodzaju to `ON CONFLICT DO UPDATE`. (D4)
5. Q1: „Średnio / dzień” = brakujący czas na pełny dzień roboczy (pół dnia liczy się jako 0,5 dnia); `workdays` i `workdaysAfterToday` liczą dni z normą (pół dnia to 1 dzień), a nowe `halfDaysAfterToday` mówi, ile z nich to pół dnia; podpis „przez N dni roboczych, w tym M × pół dnia”.
6. Q2: `daysOff` zostaje listą całych dni (zgodność dla S-05 i kafelka), nowe `halfDaysOff`; kafelek „Dni wolne” = całe + 0,5 × pół z polskim przecinkiem i podpisem „w tym M × pół dnia”.
7. Q3: przypomnienie startu nie odzywa się w pół dnia wolnego (jak w dzień wolny; pracująca połowa może być popołudniem) — bez zmiany kodu, z testem.
8. Q4: panel „Wybrany dzień” dostaje ten sam trzystanowy kontroler co tabela (jeden kontrakt `DayMark`).
9. Q6: polityka Lenient bez zmian (status, lista wpisów, przypomnienia liczą z 8 h przy zepsutym bloku); żaden widok nie pokazuje `monthDeviationMs`; S-07 wybiera Strict, jeśli pokaże odchylenie miesiąca.
10. Kontroler: istniejący `Segmented` (z nową opcją `disabled`) zamiast cyklicznego przycisku — trzy widoczne stany są czytelniejsze; etykiety w tabeli „nie” / „½” / „tak” pod nagłówkiem „Wolne”, w panelu „roboczy” / „pół dnia wolnego” / „wolny”. Pół dnia nie jest zakreskowany; ma dopisek „pół dnia wolnego” i kreskę normy w połowie paska. (D5 w wersji z jawnymi stanami)
11. Kontrakt komendy: nazwa `set_day_off` zostaje (bez zmian w `lib.rs`), argument `off: bool` → `mark: DayMark` (`"workday" | "halfDayOff" | "dayOff"`), zgodny z `DayKind`, więc UI porównuje żądanie z `row.kind` wprost.
12. `norm.rs` jedynym źródłem: publiczne `day_norm_ms` (norma jednego dnia) i `day_norms` (normy dni dowolnego zakresu) dla S-08; czyste funkcje biorą `HolidayCalendar` i `daily_norm_ms`, funkcje ze store biorą `&WorkTimeSettings` (konwencja z planu świąt).
13. `MonthReport.dailyNormMs` niesie normę dzienną do tekstu „Jak liczę”; UI nie zna liczby 8 h.
14. Q5: v12 należy do S-02 (jedyny równoległy slice z migracją); przy kolizji przy merge'u migracja bierze kolejny wolny numer, a test upgrade'u (`MIGRATIONS[..11]`) zmienia się razem z nią — ryzyko merge'u, nie decyzja.
15. Cztery fazy: norma → pół dnia w danych i regule (komenda jeszcze na boolu) → kontrakt komendy i UI → dokumentacja; bez synchronizacji statusu w `roadmap.md` (zakaz edycji w tej zmianie).

## Merge Risks

- **Numer migracji v12** — gdyby inna gałąź też dopisała v12, gałąź scalana druga przenumerowuje swoją migrację i asercję w teście upgrade'u.
- **S-08 week-summary** woła funkcje normy: faza 1–2 zmieniają sygnatury `day_norm_ms`, `workdays`, `figures` (parametr `daily_norm_ms`, `&DaysOff`). Gałąź scalana druga dopasowuje wywołania; S-08 powinien używać `day_norms` zamiast własnego mnożenia.
- **S-05 month-report-copy** używa typów raportu: nowy wariant `DayKind::HalfDayOff` złamie wyczerpujące `match` (zamierzone — kopia musi rozróżnić pół dnia), nowe pola `MonthReport.daily_norm_ms`/`half_days_off` i `NormFigures.half_days_after_today` złamią literały tych struktur w testach.
- **Nowe pole `WorkTimeSettings.daily_norm_ms`** łamie literały `WorkTimeSettings { holidays }` dodane przez inne gałęzie; poprawka: `..WorkTimeSettings::default()`.
- **`report::set_day_off` z `DayMark`** zamiast `bool` — każde nowe wywołanie z innej gałęzi (np. testy) dostaje `DayMark::DayOff`/`Workday`.
- **Pliki współdzielone**: `WorkTimeView.tsx` (`HowCounted`, kontroler dnia), `DaysTable.tsx`, `WorkTime.css` (`--dn-table-cols`), `MonthReport.tsx`, `summary.ts`, `types.ts`, `api.ts`, README, CHANGELOG — konflikty tekstowe, małe hunki.

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Norma dzienna z settings.json

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy `settings::work_time` (zakres, pełne minuty, null/brak, izolacja bloków), testy normy 7.5 h na listopadzie 2026 i proptest z losową normą 60–600 min — 3f24630
- [x] 1.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag — 3f24630
- [x] 1.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 3f24630
- [x] 1.4 `git grep -nw "DAILY_NORM_MS" -- src-tauri` i `git grep -n "8 h" -- src/worktime` nic nie znajdują — 3f24630

#### Manual

- [ ] 1.5 Z `"workTime": { "dailyNormHours": 7.5 }` w `settings.json` po powrocie do okna „Czas pracy” pokazuje „z … normy” jako 7:30 × dni robocze, pasek dnia roboczego ma kreskę normy na 7:30, a „Jak liczę” podaje 7:30; kafelki pulpitu pokazują te same liczby
- [ ] 1.6 `"dailyNormHours": 11` (albo `"8"`) daje w „Czas pracy” błąd raportu z komunikatem o `dailyNormHours`, a kafelki pulpitu pokazują „raport niedostępny”; START/STOP w nagłówku działa dalej; usunięcie klucza przywraca 8 h

### Phase 2: Pół dnia wolnego w danych i regule

#### Automated

- [x] 2.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy pół dnia w `norm_tests.rs` i `reminders_tests.rs`, upgrade v11→v12 i odmowa CHECK w `tests/work_time.rs`, `schema::validate(MIGRATIONS)`, istniejące testy upgrade'u (`tests/events.rs`, `tests/hidden_issues.rs`, `tests/new_assignments.rs`, `tests/todo.rs`) i nowy `tests/half_day_off_crash.rs` — 7e02106
- [x] 2.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag — 7e02106
- [x] 2.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 7e02106
- [x] 2.4 `git diff <commit fazy 1>.. -- src-tauri/src/store/schema.rs` pokazuje tylko dopisaną migrację v12 (bez zmian w v1–v11) — 7e02106

#### Manual

- [ ] 2.5 Po pierwszym uruchomieniu wersji z v12 na prawdziwych danych (z kopią katalogu danych wg „Ręczne przebiegi dzielą prawdziwą bazę”) „Czas pracy” pokazuje dotychczasowe dni wolne zakreskowane jak przedtem i te same liczby miesiąca, a „Stan źródeł” pokazuje „Dane aplikacji” jako sprawne

### Phase 3: Trzystanowy przełącznik dnia w UI

#### Automated

- [x] 3.1 `cargo test` w `src-tauri/` przechodzi, w tym test deserializacji `DayMark` — 6968a74
- [x] 3.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag — 6968a74
- [x] 3.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 6968a74
- [x] 3.4 `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css src/components/Segmented/Segmented.css` nic nie znajduje, a `git grep -n "invoke" -- src/worktime` znajduje `invoke` tylko w `src/worktime/api.ts` — 6968a74

#### Manual

- [ ] 3.5 W tabeli dni dzień roboczy przełączony na „½” pokazuje dopisek „pół dnia wolnego”, kreskę normy w połowie paska i normę dnia o połowę mniejszą; norma miesiąca i odchylenie zmieniają się o pół normy; „tak” zakreskowuje dzień; „nie” przywraca dzień roboczy; przełącznik nie „odskakuje” w trakcie żądania
- [ ] 3.6 Panel „Wybrany dzień” pokazuje ten sam stan co tabela i zmiana w jednym miejscu jest widoczna w drugim; weekend i święto nie mają przełącznika w obu miejscach
- [ ] 3.7 „Dni wolne” pokazuje np. „1,5” z „w tym 1 × pół dnia”, a „Średnio / dzień” na pulpicie i w „Czas pracy” ma ten sam podpis z pół dniem po dziś
- [ ] 3.8 Tabela dni mieści się bez poziomego przewijania w oknie 800 px; przełącznik działa z klawiatury (Tab, Enter/Spacja)
- [ ] 3.9 W dzień oznaczony jako pół dnia wolnego przypomnienie „licznik nie biegnie po GG:MM” się nie pokazuje

### Phase 4: Dokumentacja

#### Automated

- [x] 4.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą; w `src-tauri/` `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` przechodzą
- [x] 4.2 `git grep -n "8 h" -- context/foundation/prd.md` znajduje tylko wzmianki „domyślnie 8 h”, a `git grep -n "dailyNormHours" -- README.md CHANGELOG.md` znajduje oba pliki
- [x] 4.3 `git diff <commit fazy 3>.. --stat` nie pokazuje `context/foundation/roadmap.md`
