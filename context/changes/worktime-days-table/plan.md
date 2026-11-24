# Tabela dni miesiąca w „Czas pracy” — plan implementacji

## Overview

W widoku „Czas pracy” pojawia się tabela wszystkich dni pokazanego miesiąca: `Dzień | Wpisy | Na tle normy | Suma | ± | Wolne`. Pasek (`HoursBar`) ma kreskę normy dnia, kolumna ± pokazuje odchylenie od normy dnia, a przełącznik „Wolne” oznacza dzień wolny przez istniejące `set_day_off`. Dziś jest wyróżniony, a dni wolne zakreskowane. Zamyka to FR-020 / issue #2 (slice S-05, M-2). Raport dzienny i reguła normy dnia żyją w Rust i mają testy. UI tylko wyświetla wynik i wysyła przełączenie.

## Current State Analysis

- `report::compute` (`src-tauri/src/work_time/report.rs:83-113`) czyta dni wolne i wpisy miesiąca w jednym `store.read` i liczy tylko sumę miesiąca: `in_day(month_start, month_end)` plus `portion()` przycięte do granic miesiąca. Sum dziennych nie ma, dlatego w 0.2.0 tabeli nie ma (issue #2).
- Reguły normy są czyste i bez zegara w `norm.rs`: `DAILY_NORM_MS`, `is_weekend`, `workdays`, `phase`, `figures`. W bieżącym miesiącu `figures` liczy dziś w całości, gdy dziś jest dniem roboczym (`norm.rs:101-104`).
- `day_bounds` (`rules.rs:44-56`) daje granice lokalnego dnia (doba DST ma 23 h albo 25 h). `portion` (`rules.rs:60-64`) dzieli wpis przez północ na kawałki.
- `DayKind` (`report.rs:20-26`, serde camelCase: `workday | dayOff | weekend`) już jest w kontrakcie TS (`src/worktime/types.ts:67`). `day_kind(conn, day)` odpytuje bazę o jeden dzień.
- `status()` (`work_time/mod.rs:183`) też woła `report::compute`, więc rozbicie dzienne policzy się przy każdym odczycie statusu. To 28–31 dni po kilkadziesiąt wpisów, więc koszt jest pomijalny.
- UI: `useMonthReport` (`src/worktime/useMonthReport.ts`) już odświeża raport po `work-time-changed`, przy focusie i co 60 s, z zasadą „najnowsze żądanie wygrywa”. `MonthReport.tsx:83-98` pokazuje pod kafelkami wiersz linków „Dni wolne:”. `WorkTimeView.tsx` ma w kolumnie głównej panele „Wybrany dzień” (z checkboxem „Dzień wolny”) i „Jak liczę”, a w bocznej `EntriesPanel` wybranego dnia.
- Checkbox „Dzień wolny” trzyma żądanie jako oczekujące, dopóki przeładowany dzień się z nim nie zgodzi (`useWorkEntries.ts:132-152`). Tabela potrzebuje tego samego wzorca dla wielu dni.
- `DataTable` (`src/components/DataTable/`) to siatka `role=table`, której szablon kolumn ustawia widok przez `--dn-table-cols`. Tony komórek to tylko `muted | faint`, a stanów wiersza brak. `plural` jest w `src/components/plural.ts`.
- `HoursBar` istnieje tylko w design systemie (artefakt Konsola, `components/HoursBar`). Tor `line-subtle`, wypełnienie `ink-soft` (dziś `accent`), kreska normy `ink-faint`, skala 10 h, `role="img"` z nazwą. Jego `norm` ma domyślnie 8, przez co plan konsoli go nie portował (`context/archive/2026-11-23-konsola-design-system/plan.md:36`).
- Nagłówek już ekstrapoluje wartość z Rust o czas trwającego wpisu (`monthDeviationMs`, `total.ts`). Ten sam wzorzec obsłuży wiersz „dziś”.

## Desired End State

- W kolumnie głównej „Czas pracy”, między „Wybrany dzień” a „Jak liczę”, jest panel „Dni miesiąca” z jednym wierszem na każdy dzień pokazanego miesiąca (listopad 2026: 30 wierszy).
- Kolumny:
  - **Dzień**: przycisk `19.11 cz`, który wybiera dzień w „Wybrany dzień” i w panelu wpisów.
  - **Wpisy**: `1 wpis` / `2 wpisy` / `5 wpisów`, albo `—` przy zerze.
  - **Na tle normy**: `HoursBar` z kreską normy dnia z Rust (8 h w dzień roboczy, brak kreski w dzień wolny).
  - **Suma**: `G:MM`.
  - **±**: odchylenie ze znakiem (`info` nad normą, `warn` pod nią), albo `—`.
  - **Wolne**: przełącznik (checkbox `role="switch"`) dla pn–pt; w weekend brak przełącznika.
- Reguła odchylenia dnia (Rust):
  - Dni do dziś włącznie: przepracowane minus norma dnia.
  - Dni po dziś: brak odchylenia.
  - Dzień bez normy (wolny albo weekend) bez godzin: brak odchylenia (`—`).
  - Godziny w weekend albo w dzień wolny dają dodatnie odchylenie.
  - Suma odchyleń wierszy równa się kafelkowi „Odchylenie” miesiąca, a suma sum wierszy kafelkowi „Miesiąc”.
- Dziś jest wyróżniony (tło `surface-raised`, akcentowa krawędź, `aria-current`). Pasek dziś ma wypełnienie `accent`. Suma, pasek i ± dziś biegną na żywo, gdy licznik biegnie, ze wspólnego stanu `useWorkTimer()`.
- Dni wolne są zakreskowane. Weekendy mają wyciszony tekst, bez kreski normy i bez przełącznika, a pasek tylko gdy są godziny.
- Przełączenie „Wolne” zapisuje dzień przez `set_day_off`. Przełącznik nie odskakuje, dopóki raport się nie przeładuje. Odmowa albo błąd pokazuje `Notice` z dniem i komunikatem. Nagłówek, kafelki, checkbox w „Wybrany dzień” i tabela zgadzają się po `work-time-changed`.
- Wiersz linków „Dni wolne:” pod kafelkami znika. Checkbox w „Wybrany dzień” zostaje.
- Panel tabeli ma stany ładowania i błędu (`ErrorState` z czasem ostatniego udanego odczytu i ponowieniem), nigdy pustej tabeli w miejscu błędu.

Weryfikacja: kontrole sprzed commita z `AGENTS.md` oraz ręczny przebieg w `npm run tauri dev`.

### Key Discoveries:

- Jeden snapshot dla raportu i tabeli: rozbicie dzienne liczy `compute` z tej samej listy wpisów i tego samego zbioru dni wolnych (`report.rs:94-99`). Nie ma nowej komendy ani nowego odczytu.
- Przy wpisach nie późniejszych niż `now` zachodzi równość: Σ odchyleń dni ≤ dziś = przepracowane miesiąca − norma do dziś = `figures.deviation_ms`, w miesiącu bieżącym i przeszłym. Daje to test własności (proptest jest już w `[dev-dependencies]`, `src-tauri/Cargo.toml:62`).
- Klasyfikacja dnia potrzebuje czystej funkcji na zbiorze `days_off`. `day_kind(conn, day)` zostaje dla `entries_for_day` i może jej używać.
- Kolor i kreskowanie nie mogą żyć w CSS widoku („view CSS is layout only”, `AGENTS.md`), więc stany wiersza i tony komórek trafiają do `DataTable`, a wzór kreskowania do `tokens.css`.
- `todayTotalMs(status, nowMs)` (`src/worktime/total.ts:14`) daje żywą sumę dziś bez drugiej subskrypcji. Reguła ESLint `no-restricted-imports` trzyma `useWorkTimerSource` w Providerze.

## What We're NOT Doing

- Bez nowej komendy Tauri, nowego zdarzenia i migracji. `days` jest nowym polem istniejącego `MonthReport`, a `days_off` zostaje (kafelek „Dni wolne”).
- Bez konfigurowalnej normy, półdni wolnych i kalendarza świąt (PRD Non-Goals, FR-014).
- Bez zakresów godzin w kolumnie „Wpisy”: tylko liczba. Zakresy pokazuje panel wpisów wybranego dnia.
- Bez edycji wpisów z tabeli, bez sortowania i filtrów, bez przewijanej sekcji tylko dla tabeli.
- Bez osobnego wyróżnienia wybranego dnia w tabeli: wyróżniony jest tylko dziś.
- Bez zmian w panelu przypomnień (S-06) i w `WorkTimeEntries` poza przeniesieniem nazw dni tygodnia.
- Bez synchronizacji artefaktu design systemu. Robi to F-05, a ten plan tylko dopisuje pozycję do jego Outcome.
- Bez liczby 8 h w kodzie UI jako reguły. Nagłówek kolumny to „Na tle normy”, a kreska idzie z `normMs` z Rust. Tekst „Jak liczę” już opisuje normę słowami i może je powtórzyć.

## Implementation Approach

Rust dokłada do `MonthReport` listę `days: Vec<DayReport>` liczoną w `compute`. Na każdy lokalny dzień miesiąca bierze `day_bounds` i sumuje `portion` wpisów z już wczytanej listy. Normę i odchylenie dnia dają dwie nowe czyste funkcje w `norm.rs`, z testami jednostkowymi i testem własności sum. Lustro TS zmienia się w tym samym commicie.

UI dostaje `HoursBar` w `src/components/` (port z design systemu, `norm` wymagane), stany `current`/`hatched` wiersza i tony `info`/`warn` komórki w `DataTable`, token kreskowania oraz `DaysTable` w `src/worktime/`. Ten ostatni czyta raport z `useMonthReport` (już w widoku) i żywą sumę dziś z `useWorkTimer()`. Przełączanie obsługuje mały hook z oczekującymi żądaniami per dzień.

## Critical Implementation Details

- **Wiersz „dziś” na żywo**: odchylenie dziś to ekstrapolacja wartości z Rust, a nie ponowne liczenie reguły w UI. `liveWorked = todayTotalMs(status, nowMs)`. `liveDeviation = day.deviationMs === null ? null : day.deviationMs + (liveWorked - day.workedMs)`. Tylko gdy `report.phase === "current"`, dzień to dziś i `timer.state === "loaded"`. W przeciwnym razie wiersz pokazuje wartości raportu. Weekend, w którym START nastąpił po ostatnim odczycie raportu, dostaje odchylenie po odświeżeniu na `work-time-changed`.
- **Przełącznik bez odskoku**: żądanie dla dnia jest oczekujące, dopóki `report.days[d].kind` się z nim nie zgodzi (`dayOff` ⇔ `off`). Do tego czasu przełącznik pokazuje żądany stan i jest `disabled`. Błąd usuwa żądanie i ustawia komunikat. Zmiana miesiąca czyści komunikat. Wzorzec jak `useWorkEntries.ts:132-152`.

## Phase 1: Raport dzienny w Rust i lustro TS

### Overview

`MonthReport` niesie rozbicie na dni z normą i odchyleniem dnia, policzone w Rust z jednego snapshotu i przetestowane. UI kompiluje się z nowym typem, ale jeszcze go nie pokazuje.

### Changes Required:

#### 1. Czyste reguły dnia

**File**: `src-tauri/src/work_time/norm.rs`

**Intent**: Norma dnia i odchylenie dnia jako czyste funkcje bez bazy i zegara, obok istniejących reguł miesiąca, żeby reguła żyła w jednym miejscu i miała testy.

**Contract**:
- `pub fn day_norm_ms(day: NaiveDate, days_off: &BTreeSet<NaiveDate>) -> i64`: `DAILY_NORM_MS` dla pn–pt spoza `days_off`, inaczej `0`. Weekend daje zawsze `0`, także gdy jest w `days_off`.
- `pub fn day_deviation_ms(day: NaiveDate, today: NaiveDate, norm_ms: i64, worked_ms: i64) -> Option<i64>`: `None`, gdy `day > today`. `None`, gdy `norm_ms == 0 && worked_ms == 0`. Inaczej `Some(worked_ms - norm_ms)`.
- Niezmiennik: `Σ day_norm_ms` po dniach miesiąca `== figures(...).norm_ms`.

#### 2. Raport dzienny w `compute`

**File**: `src-tauri/src/work_time/report.rs`

**Intent**: Policzyć wiersz na każdy lokalny dzień miesiąca z tej samej listy wpisów i tego samego zbioru dni wolnych, które już wczytuje `compute`, i dołożyć go do `MonthReport`.

**Contract**:
- Nowa struktura (serde camelCase):

  ```rust
  pub struct DayReport {
      pub day: String,              // YYYY-MM-DD
      pub kind: DayKind,
      pub worked_ms: i64,           // Σ portion wpisów w [day_start, day_end), running do now
      pub norm_ms: i64,             // norm::day_norm_ms
      pub deviation_ms: Option<i64>,// norm::day_deviation_ms(day, today, norm_ms, worked_ms)
      pub entry_count: u32,         // wpisy z portion > 0 w tym dniu
  }
  ```

- `MonthReport` dostaje `pub days: Vec<DayReport>`: wszystkie dni `[first, next_first)` rosnąco. Lista wpisów miesiąca jest wczytana raz (już jest). Granice każdego dnia z `day_bounds(d, tz)`.
- Czysta klasyfikacja `fn kind_of(day, &days_off) -> DayKind` (weekend → `Weekend`, w zbiorze → `DayOff`, inaczej `Workday`). `day_kind(conn, day)` może z niej korzystać. Jego zachowanie się nie zmienia.
- Pola `MonthReport` i ich znaczenie się nie zmieniają. `status()` dalej czyta tylko `figures.deviation_ms`.

#### 3. Testy reguł i raportu

**File**: `src-tauri/src/work_time/norm_tests.rs`

**Intent**: Testy jednostkowe reguły dnia i rozbicia raportu na wymyślonych miesiącach w `Europe/Warsaw` (istniejące helpery `t`, `d`, `open`, `add_entry`, `worked_example`, `report_at`) oraz test własności sum.

**Contract**: Nazwane testy (nazwy mogą się różnić, przypadki nie):
- `day_norm_is_eight_hours_on_a_workday_and_zero_otherwise`: dzień roboczy, dzień wolny, sobota, sobota w `days_off`.
- `day_deviation_follows_the_month_rule`: dzień po dziś → `None`. Dziś z 4 h → `Some(-4 h)`. Miniony dzień roboczy z 0 h → `Some(-8 h)`. Weekend z 0 h → `None`. Weekend z 2 h → `Some(+2 h)`. Dzień wolny z 0 h → `None`. Dzień wolny z 3 h → `Some(+3 h)`.
- `worked_example_days_add_up_to_the_month`: listopad 2026, `now` = pn 23.11 12:00. 30 wierszy. 11.11 to `dayOff` z normą 0 i bez odchylenia. 23.11 ma 4 h i `-4 h`. 24.11 ma `None` i 0 h. Σ `worked_ms` == `worked_ms`, Σ odchyleń == `deviation_ms` (−4 h), Σ `norm_ms` == `norm_ms`.
- `an_entry_across_midnight_counts_in_both_days`: 22:00→02:00 daje 2 h i `entry_count` 1 w obu dniach.
- `an_entry_across_the_month_boundary_counts_only_its_part_per_month`: 30.11 22:00 → 1.12 02:00.
- `dst_days_sum_their_real_lengths`: październik 2026 ma 31 wierszy, a wpis 25.10 00:00–06:00 lokalnie daje 7 h w tym dniu (cofnięcie zegara). Σ == `worked_ms`.
- `a_running_entry_counts_up_to_now_today_and_its_part_yesterday`: wpis biegnie od wczoraj 22:00, `now` = dziś 01:00. Wczoraj 2 h, dziś 1 h, `entry_count` 1 w obu dniach.
- `a_past_month_has_every_day_deviation_and_a_future_month_none`: październik widziany z listopada ma odchylenia dla każdego dnia z normą albo godzinami, a Σ == `deviation_ms`. Grudzień ma same `None`, 0 h i normy 8 h w dni robocze.
- `proptest` `day_rows_add_up_to_the_month_figures`: losowe, nienachodzące na siebie, zamknięte wpisy przed `now` w listopadzie 2026 i losowy podzbiór dni wolnych pn–pt. Wtedy `days.len() == 30`, Σ `worked_ms` == `worked_ms`, Σ `deviation_ms.unwrap_or(0)` == `deviation_ms.unwrap()`, Σ `norm_ms` == `norm_ms`.

#### 4. Lustro TS w tym samym commicie

**File**: `src/worktime/types.ts`

**Intent**: Lustro `DayReport` i nowego pola, żeby kontrakt Rust↔TS zmienił się w jednym commicie (`AGENTS.md`).

**Contract**: `export interface DayReport { day: string; kind: DayKind; workedMs: number; normMs: number; deviationMs: number | null; entryCount: number }`. `MonthReportBase` dostaje `days: DayReport[]` (komentarz: wszystkie dni miesiąca, rosnąco).

### Success Criteria:

#### Automated Verification:

- Formatowanie Rust: `cargo fmt --check` (w `src-tauri/`)
- Clippy bez ostrzeżeń: `cargo clippy --all-targets -- -D warnings`
- Nowe testy reguły dnia i raportu dziennego przechodzą: `cargo test work_time::norm_tests`
- Cały zestaw testów Rust przechodzi: `cargo test`
- Typy TS kompilują się z nowym polem: `npm run build`
- Lint i formatowanie frontendu: `npm run lint` oraz `npm run format:check`

**Implementation Note**: Po zielonej weryfikacji automatycznej faza 1 nie ma kroków ręcznych (UI się nie zmienia). Przejdź do fazy 2.

---

## Phase 2: Tabela dni w „Czas pracy”

### Overview

Widok pokazuje raport dzienny jako tabelę z paskiem, wyróżnionym dziś, kreskowanymi dniami wolnymi i przełącznikiem „Wolne”. Wiersz linków „Dni wolne:” znika.

### Changes Required:

#### 1. Token kreskowania

**File**: `src/styles/tokens.css`

**Intent**: Wzór ukośnych pasków dnia wolnego jako wartość w jedynym miejscu na wartości.

**Contract**: Nowy token, np. `--hatch-bg: repeating-linear-gradient(135deg, var(--line-subtle) 0 2px, transparent 2px 8px)`. Paski biorą kolor z istniejącego tokenu bliskiego tłu, więc kontrast tekstu w wierszu się nie zmienia.

#### 2. Stany wiersza i tony komórki w `DataTable`

**File**: `src/components/DataTable/DataTable.tsx`, `src/components/DataTable/DataTable.css`

**Intent**: Wyróżnienie bieżącego wiersza, kreskowanie i kolor liczby jako wspólne stany komponentu, bo CSS widoku nie może mieć kolorów.

**Contract**:
- `DataTableRow` dostaje opcjonalne `current?: boolean` (klasa `is-current`: tło `--surface-raised`, `box-shadow: inset 2px 0 0 var(--accent)`, atrybut `aria-current="true"`) oraz `hatched?: boolean` (klasa `is-hatched`: `background-image: var(--hatch-bg)`). Oba mogą wystąpić naraz (dziś jest dniem wolnym).
- `DataTableCell.tone` rozszerza się o `"info" | "warn"` (`--info`, `--warn`).
- Bez tych propsów nic się nie zmienia w istniejących tabelach.

#### 3. `HoursBar`

**File**: `src/components/HoursBar/HoursBar.tsx`, `src/components/HoursBar/HoursBar.css`, `src/components/index.ts`

**Intent**: Port paska godzin z design systemu, z normą podawaną przez wywołującego (z Rust) zamiast domyślnych 8 h w UI.

**Contract**:
- `HoursBar({ hours, norm, max = 10, today, label })`. `hours` i `norm` w godzinach (liczby). `norm` wymagane, `0` ukrywa kreskę. `max` to skala paska. `label` wymagane (`role="img"`, `aria-label`).
- Wypełnienie = `clamp(hours / max, 0, 1)`, kreska na `norm / max`.
- CSS z artefaktu (`dn-hours`, `dn-hours-fill`, `is-today`, `dn-hours-norm`) na istniejących tokenach.
- Eksport z `src/components/index.ts` razem z `HoursBarProps`.

#### 4. Formatery dnia i liczby wpisów

**File**: `src/worktime/format.ts`, `src/worktime/WorkTimeEntries.tsx`

**Intent**: Etykieta dnia w tabeli i liczba wpisów po polsku, bez kolejnej kopii skrótów dni tygodnia.

**Contract**:
- `WEEKDAYS` przenosi się do `format.ts` (eksport) i `WorkTimeEntries.tsx` go importuje.
- `formatDayLabel(day: string): string` → `19.11 cz`.
- `formatEntryCount(n: number): string` → `1 wpis` / `2 wpisy` / `5 wpisów` przez `plural` z `src/components`.

#### 5. Przełączanie dni wolnych z tabeli

**File**: `src/worktime/useDayOffToggle.ts` (nowy)

**Intent**: Oczekujące żądania per dzień i komunikat błędu dla przełączników tabeli, żeby przełącznik nie odskakiwał i nie zostawał zablokowany (patrz Critical Implementation Details).

**Contract**: `useDayOffToggle(report: MonthReportState)` zwraca `{ checked(day): boolean; busy(day): boolean; toggle(day, off): Promise<void>; error: { day: string; message: string } | null }`. Woła `setDayOff` z `api.ts` i tłumaczy błąd przez `workTimeErrorMessage`. Nie subskrybuje zdarzeń: raport odświeża się sam po `work-time-changed`.

#### 6. Tabela dni

**File**: `src/worktime/DaysTable.tsx` (nowy), `src/worktime/WorkTime.css`

**Intent**: Wiersz na każdy dzień raportu z kolumnami z Desired End State, wyświetlający wartości z Rust i żywą sumę dziś.

**Contract**:
- Propsy: `report: MonthReport` (załadowany), `today: string` (`YYYY-MM-DD`), `liveTodayMs: number | null`, `onPickDay(day)`, wynik `useDayOffToggle`.
- `DataTable label="Dni miesiąca" className="wt-days"`, wiersz nagłówka `Dzień | Wpisy | Na tle normy | Suma | ± | Wolne`.
- Wiersz: `current` dla dziś, `hatched` dla `kind === "dayOff"`, komórki tekstu `tone="muted"` dla weekendu.
- **Dzień**: `Button variant="quiet" size="sm"` z `formatDayLabel`, `aria-label="Pokaż dzień DD.MM"`.
- **Wpisy**: `formatEntryCount` albo `—` (`faint`).
- **Pasek**: `HoursBar` z `hours = workedMs / 3_600_000`, `norm = normMs / 3_600_000`, `today`, etykietą `G:MM na tle normy G:MM` (albo `G:MM, bez normy`). W weekend tylko gdy `workedMs > 0`.
- **Suma**: `formatHm`, albo `—` gdy 0 h i dzień jest po dziś albo bez normy.
- **±**: `formatSigned`, ton `info` dla > 0, `warn` dla < 0, albo `—` dla `null`.
- **Wolne**: dla `kind !== "weekend"` checkbox `role="switch"` (klasa `wt-check`), `aria-label="Dzień wolny DD.MM"`, `checked`/`disabled` z hooka.
- Wiersz dziś na żywo według Critical Implementation Details.
- `WorkTime.css`: tylko szablon kolumn `.wt-days { --dn-table-cols: … }` i układ. Usunąć `.wt-days-off`.

#### 7. Widok i raport miesiąca

**File**: `src/worktime/WorkTimeView.tsx`, `src/worktime/MonthReport.tsx`

**Intent**: Wpiąć tabelę w kolumnę główną i usunąć zdublowany wiersz linków „Dni wolne:”.

**Contract**:
- `WorkTimeView` renderuje `Panel title="Dni miesiąca" meta={formatMonth(month)}` między „Wybrany dzień” a „Jak liczę”.
  - Ładowanie: `dn-loading`.
  - Błąd: `ErrorState` z komunikatem, `ostatnio OK HH:MM` i `retryReport`.
  - Dane: `DaysTable` z `today={localToday()}`, `liveTodayMs` z `todayTotalMs(timer.status, nowMs)`, gdy `timer.state === "loaded"` (inaczej `null`), `onPickDay={setDay}` i hookiem `useDayOffToggle(report)`.
  - `Notice tone="error" compact` z `DD.MM: komunikat`, gdy hook ma błąd.
- `MonthReport` traci wiersz „Dni wolne:” i prop `onPickDay`. Kafelek „Dni wolne” zostaje.
- „Jak liczę” dostaje jeden akapit o tabeli dni: odchylenie dnia to przepracowany czas minus norma dnia (8 h w dzień roboczy, 0 h w dzień wolny i weekend), a odchylenia dni do dziś sumują się do odchylenia miesiąca.

#### 8. Roadmapa: zakres F-05

**File**: `context/foundation/roadmap.md`

**Intent**: Przekazać F-05 (synchronizacja artefaktu) nowe API, żeby artefakt nie rozjechał się z aplikacją.

**Contract**: W `### F-05` → `**Outcome:**` dopisać po „`ItemRow` labels”: „`HoursBar` with a required `norm` (from Rust), `DataTable` row states `current`/`hatched` and cell tones `info`/`warn`”. Bez innych zmian w tym bloku.

### Success Criteria:

#### Automated Verification:

- Lint frontendu: `npm run lint`
- Formatowanie frontendu: `npm run format:check`
- Typy i build: `npm run build`
- Rust bez regresji: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- W UI nie ma reguły 8 h: `rg -n "28800000|28_800_000|8 \* 3600|8 \* 60 \* 60|norm = 8|norm \?\? 8" src/` nic nie zwraca
- Licznik czytany tylko przez kontekst: `rg -n "useWorkTimerSource" src/` znajduje tylko `useWorkTimer.ts` i `WorkTimerProvider.tsx`

#### Manual Verification:

- W `npm run tauri dev` panel „Dni miesiąca” pokazuje wszystkie dni bieżącego miesiąca. Dziś jest wyróżniony, dni wolne zakreskowane bez kreski normy, a weekendy wyciszone bez przełącznika.
- Przy biegnącym liczniku suma, pasek i ± dziś rosną na żywo i zgadzają się z „Dziś” w nagłówku i kafelku. Suma kolumny ± do dziś odpowiada kafelkowi „Odchylenie”.
- Przełącznik „Wolne” na przyszłym dniu roboczym: wiersz staje się zakreskowany, kafelek „Dni wolne” rośnie o 1, a checkbox w „Wybrany dzień” po kliknięciu tego dnia jest zaznaczony. Odznaczenie przywraca stan. Przełącznik nie odskakuje w trakcie.
- Klik w datę w tabeli zmienia „Wybrany dzień” i panel wpisów na ten dzień.
- Przełączanie miesięcy ◀ ▶: miesiąc przeszły ma odchylenia każdego dnia, a przyszły nie ma żadnych. Wiersz linków „Dni wolne:” nie występuje.
- Przy szerokości okna 800 px tabela mieści się w kolumnie bez ucinania liczb.
- Tylko klawiatura: Tab przechodzi przez przyciski dni i przełączniki, a Spacja przełącza „Wolne”.

**Implementation Note**: Po zielonej weryfikacji automatycznej zatrzymaj się na ręcznym potwierdzeniu przez użytkownika przed zamknięciem zmiany.

---

## Testing Strategy

### Unit Tests:

- Reguły dnia (`day_norm_ms`, `day_deviation_ms`) na wszystkich rodzajach dnia i relacjach do dziś.
- Rozbicie raportu: przykład listopadowy, wpis przez północ, przez granicę miesiąca, doba DST 25 h, wpis biegnący od wczoraj, miesiąc przeszły i przyszły.
- Własność: suma wierszy = liczby miesiąca dla losowych wpisów i dni wolnych.

### Integration Tests:

- Bez nowych: kontrakt komend się nie zmienia. Istniejące `tests/work_time.rs` i `tests/days_off_crash.rs` muszą przejść bez zmian.

### Manual Testing Steps:

1. Uruchom `npm run tauri dev` i otwórz „Czas pracy”.
2. Sprawdź wiersze bieżącego miesiąca, wyróżnienie dziś i kreskowanie dni wolnych.
3. Uruchom START i obserwuj wiersz dziś obok „Dziś” w nagłówku.
4. Oznacz i odznacz przyszły dzień roboczy jako wolny z tabeli. Sprawdź kafelki i checkbox w „Wybrany dzień”.
5. Przejdź do poprzedniego i następnego miesiąca.
6. Zmniejsz okno do 800 px i przejdź tabelę klawiaturą.

## Performance Considerations

`compute` liczy 28–31 granic dni i sumuje kilkadziesiąt wpisów na dzień przy każdym `work_timer_status` i `work_month_report`. To pomijalne wobec odczytu SQLite. UI renderuje ≤ 31 wierszy, a wiersz dziś przelicza się co sekundę razem z nagłówkiem (wspólny tick).

## Migration Notes

Brak migracji schematu. Pole `days` jest addytywne w odpowiedzi `work_month_report` i `work_timer_status` go nie zwraca. Baza użytkownika (już v9) nie jest dotykana.

## References

- Issue #2, PRD FR-020 (`context/foundation/prd.md:176`), roadmap S-05 (`context/foundation/roadmap.md:212`)
- Reguły normy: `src-tauri/src/work_time/norm.rs`, raport: `src-tauri/src/work_time/report.rs:83-113`
- Wzorzec oczekującego przełącznika: `src/worktime/useWorkEntries.ts:132-152`
- Wspólny licznik: `src/worktime/WorkTimerProvider.tsx`, `src/worktime/total.ts`
- Design system `HoursBar`: https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY (`components/HoursBar`, `bundle.css` `.dn-hours*`)
- Wcześniejsza decyzja o nieportowaniu `HoursBar`: `context/archive/2026-11-23-konsola-design-system/plan.md:36`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Raport dzienny w Rust i lustro TS

#### Automated

- [x] 1.1 Formatowanie Rust: `cargo fmt --check` — 2e21900
- [x] 1.2 Clippy bez ostrzeżeń: `cargo clippy --all-targets -- -D warnings` — 2e21900
- [x] 1.3 Nowe testy reguły dnia i raportu dziennego przechodzą: `cargo test work_time::norm_tests` — 2e21900
- [x] 1.4 Cały zestaw testów Rust przechodzi: `cargo test` — 2e21900
- [x] 1.5 Typy TS kompilują się z nowym polem: `npm run build` — 2e21900
- [x] 1.6 Lint i formatowanie frontendu: `npm run lint` oraz `npm run format:check` — 2e21900

### Phase 2: Tabela dni w „Czas pracy”

#### Automated

- [x] 2.1 Lint frontendu: `npm run lint` — 6f5f743
- [x] 2.2 Formatowanie frontendu: `npm run format:check` — 6f5f743
- [x] 2.3 Typy i build: `npm run build` — 6f5f743
- [x] 2.4 Rust bez regresji: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 6f5f743
- [x] 2.5 W UI nie ma reguły 8 h (rg nic nie zwraca) — 6f5f743
- [x] 2.6 Licznik czytany tylko przez kontekst (`useWorkTimerSource` tylko w `useWorkTimer.ts` i `WorkTimerProvider.tsx`) — 6f5f743

#### Manual

- [x] 2.7 Panel „Dni miesiąca”: wszystkie dni, dziś wyróżniony, dni wolne zakreskowane, weekendy wyciszone bez przełącznika — 6f5f743
- [x] 2.8 Wiersz dziś na żywo zgodny z nagłówkiem; suma ± do dziś = kafelek „Odchylenie” — 6f5f743
- [x] 2.9 Przełącznik „Wolne” oznacza i odznacza dzień bez odskoku; kafelek i checkbox się zgadzają — 6f5f743
- [x] 2.10 Klik w datę zmienia „Wybrany dzień” i panel wpisów — 6f5f743
- [x] 2.11 Miesiąc przeszły i przyszły; brak wiersza linków „Dni wolne:” — 6f5f743
- [x] 2.12 Tabela mieści się przy 800 px — 6f5f743
- [x] 2.13 Obsługa samą klawiaturą — 6f5f743
