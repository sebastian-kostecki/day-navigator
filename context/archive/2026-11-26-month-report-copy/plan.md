# Kopiowanie raportu miesiąca do schowka — plan implementacji

## Overview

W widoku „Czas pracy” przycisk „Kopiuj miesiąc” kopiuje do schowka raport pokazanego miesiąca jako tekst rozdzielany tabulatorami (TSV), który wkleja się prosto w komórki Excela i Arkuszy Google. Tekst zawiera podsumowanie miesiąca (sumy, norma, odchylenie, dni robocze, dni wolne), jeden wiersz na każdy dzień (rodzaj dnia, nazwa święta, suma, norma, odchylenie) i jeden wiersz na każdy wpis w dniu (godziny, czas, notatka). Slice S-05 (M-3), MS-24, issue #24; bez nowych FR w PRD.

Tekst buduje Rust w chwili kliknięcia z tego samego `report::compute` i tych samych ustawień (`Policy::Strict`), z których powstaje raport na ekranie; budowanie jest przetestowane jednostkowo na wymyślonych danych. UI tylko woła komendę, zapisuje zwrócony tekst przez `navigator.clipboard.writeText` i pokazuje „Skopiowano.” albo błąd. Bez pliku, bez nowej zależności, uprawnienia ani zmiany CSP, bez migracji.

## Current State Analysis

- `MonthReport` / `DayReport` / `DayKind` (`src-tauri/src/work_time/report.rs:23-65`) budowane przez `compute` (`report.rs:122-183`, `pub(super)`) i podawane przez jedyną komendę `work_month_report(day)` (`commands.rs:213-223`, `Policy::Strict`). Raport nie niesie wpisów ani notatek, tylko `entry_count` (`report.rs:59-60`).
- `compute` sam czyta wpisy miesiąca przez prywatne `in_day` (`report.rs:138`, `mod.rs:131-153`), a `Entry { id, start, end, note }` (`mod.rs:95-100`) ma już notatkę (S-06). Moduł potomny `work_time` widzi `in_day` i pola `Entry` bez poszerzania widoczności (precedens `today.rs:10-11`, `report.rs:15`).
- `Store::read` to po prostu `f(&self.conn)` (`store/mod.rs:142-147`); komendy trzymają wyłączną blokadę store'u w `with_store` (`commands.rs:91-93`), więc dwa SELECT-y w jednym `read` widzą ten sam stan.
- Normy i odchylenia: `NormFigures` (`norm.rs:101-152`), dzień `day_norm_ms` / `day_deviation_ms` (`norm.rs:46-74`), `MonthPhase` (`norm.rs:76-92`), `norm::month_days` jest `pub` (`norm.rs:17`). Stała `DAILY_NORM_MS` może zmienić się w S-02.
- Pomocnicze reguły: `day_bounds`, `portion`, `local_string` (`rules.rs:24-63`), `parse_day` (`report.rs:200-206`).
- Wrapper `run_with_settings` → `execute` loguje tylko nazwę komendy i rodzaj błędu (`commands.rs:71-106`); `notify` (`commands.rs:108-114`) jest tylko dla zapisów. Dziś w `src-tauri/src/work_time` są dokładnie cztery linie `log::`, wszystkie w `commands.rs` (`:48`, `:100`, `:102`, `:110`).
- Notatka: `normalize_note` przycina, odrzuca `\r`/`\n`, ≤ 200 znaków (`note.rs:10-23`) — **tabulator i inne znaki sterujące przechodzą**, więc TSV musi je czyścić.
- Ekran: `formatHm` obcina w dół do minuty, `formatSigned` obcina w stronę zera, `U+2212` (`src/worktime/format.ts:17-31`); skróty dni `WEEKDAYS` (`format.ts:44`); reguły komórek tabeli dni (`DaysTable.tsx:44,82-86,104`); kafelki miesiąca per faza (`MonthReport.tsx:8-49`); dzisiejszy wiersz na ekranie jest nakładką z licznika (`DaysTable.tsx:13-26`).
- UI: `useMonthReport(day)` klucz `month = day.slice(0, 7)`, pobiera `${month}-01`, stan `loading | error | loaded` (`useMonthReport.ts:11-20,39`). Nagłówek widoku `dn-view-head` ma tytuł i przełącznik miesiąca, bez `dn-spacer` (`WorkTimeView.tsx:173-196`). Wzór akcji w nagłówku: `GithubView.tsx:182-195`; wzór „Zapisano.”: zawsze zamontowany `role="status"` z `Notice tone="info" compact role={null}` (`OwnTaskEditPanel.tsx:134-141`). `Notice` ma `tone`, `compact`, `role` (`components/Notice/Notice.tsx:15-30`). Brak ikony kopiowania w `IconName`.
- `api.ts` jest jedynym plikiem z `invoke` (`src/worktime/api.ts:1`); komunikaty błędów `workTimeErrorMessage` (`api.ts:120-161`). W `src/worktime` nie ma żadnego `console.`.
- CSP i uprawnienia: `tauri.conf.json:26-27`, `capabilities/default.json` — bez wtyczki schowka.

## Desired End State

- W nagłówku „Czas pracy”, po prawej, przycisk „Kopiuj miesiąc”, aktywny tylko gdy raport pokazanego miesiąca jest wczytany. Kliknięcie kopiuje TSV pokazanego miesiąca i pokazuje „Skopiowano.”; błąd komendy albo schowka pokazuje polski komunikat, nigdy cichą porażkę i nigdy pusty tekst w miejsce nieudanego źródła.
- Wklejony tekst to trzy bloki rozdzielone pustą linią (podsumowanie, dni, wpisy) w formacie z sekcji „Format TSV”. Liczby każdej komórki są tymi samymi liczbami, które pokazuje ekran (te same reguły zaokrąglenia), a dzisiejszy wiersz odpowiada wartości na żywo w chwili kliknięcia. Kopia jest liczona w chwili kliknięcia, a kafelki miesiąca nie są na żywo (odświeżane najwyżej co 60 s, po `work-time-changed` i po powrocie do okna), więc przy biegnącym liczniku albo tuż po zmianie z zasobnika podsumowanie kopii może być o te minuty świeższe od kafelków; kafelki dochodzą do tej samej wartości przy następnym odświeżeniu.
- Kopiowanie jest odczytem: nie emituje `work-time-changed`, nie zapisuje zdarzenia, niczego nie loguje poza istniejącą linią rodzaju błędu w `execute`; notatka trafia tylko do schowka na kliknięcie użytkownika.
- `report.rs`, `norm.rs`, `types.ts` bez zmian; brak nowych crate'ów, pakietów npm, uprawnień i zmian CSP.
- README i CHANGELOG mają po jednym nowym punkcie, z informacją, że przecinek dziesiętny jest dla arkuszy pl-PL.
- Weryfikacja: testy z Fazy 1 i bramki `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` zielone po każdej fazie.

### Key Discoveries:

- Jeden `store.read` może zawołać `compute` i `in_day` za cały miesiąc; obie ścieżki dzielą `day_bounds` i `portion`, więc filtr `portion > 0` daje dokładnie `entry_count` wierszy wpisów na dzień (`report.rs:145-155`).
- Pusta „Suma” na ekranie (`DaysTable.tsx:44`) to dokładnie `worked_ms == 0 && deviation_ms.is_none()` (z `norm.rs:63-74`: gdy `worked_ms == 0`, `deviation_ms` jest `None` wtedy i tylko wtedy, gdy dzień jest po dziś albo norma dnia to 0). Builder nie potrzebuje więc „dziś” ani strefy do tej reguły.
- Komórki zaczynające się od `= + - @` byłyby formułą; `+1:30` Excel policzyłby jako czas, a `-1:30` jako ujemny czas (`####`). Dlatego odchylenie idzie tylko jako liczba dziesiętna z ASCII `-`, bez `+`.
- Obcinanie do minuty w każdym wierszu osobno (jak na ekranie) sprawia, że suma kolumny może różnić się o kilka minut od wiersza miesiąca, gdy wpisy START/STOP mają sekundy — ten sam efekt jest na ekranie, a kolumna minut by go nie usunęła.

## What We're NOT Doing

- Bez zapisu do pliku, eksportu CSV/XLSX i okna „Zapisz jako” (decyzja użytkownika: tylko schowek).
- Bez przekazywania raportu z UI do Rust (UI nie ma wpisów ani notatek; drugi, niezweryfikowany kontrakt wejścia).
- Bez zmian w `report.rs`, `norm.rs`, `types.ts`, w kafelkach i w tabeli dni; bez nowych pól w `MonthReport`/`DayReport`.
- Bez `tauri-plugin-clipboard-manager` i bez `arboard` w tej zmianie — tylko zapisany wariant awaryjny (sekcja „Wariant awaryjny schowka”).
- Bez kopiowania z zasobnika, skrótu klawiszowego i kopiowania tygodnia/dnia.
- Bez wariantu dla arkuszy en-US (kropka dziesiętna) i bez ustawienia formatu.
- Bez nowego wariantu `WorkTimeError`, nowego `NewEvent` i bez `notify` w nowej komendzie.
- Bez ikony kopiowania w design systemie (przycisk tekstowy).
- Bez migracji, bez zmian w AGENTS.md (schowek to nie folder), bez edycji `context/foundation/roadmap.md` (także bez synchronizacji statusu przez `/10x-plan`) i PRD.

## Implementation Approach

Dwie fazy, każda zielona sama w sobie i commitowana osobno:

1. **Rust**: nowy moduł `work_time/month_text.rs` (czyste formatowanie + jedna funkcja z jednym `store.read`), testy w `month_text_tests.rs`, komenda `work_month_report_text(day)` dopisana na końcu komend w `commands.rs`, jedna linia w `lib.rs`, dwie linie w `mod.rs`. Komenda istnieje, UI jeszcze jej nie woła.
2. **UI i dokumentacja**: `workMonthReportText` w `api.ts`, hook `useMonthCopy.ts`, komponent `MonthCopyButton.tsx`, jedna reguła układu w `WorkTime.css`, dwie linie + import w `WorkTimeView.tsx`, punkty w README i CHANGELOG.

Nowa logika jest w nowych plikach, a pliki współdzielone z S-02/S-04/S-07/S-08 dostają tylko dopisane linie.

## Format TSV (default chosen by agent, user may revise)

Kodowanie UTF-8 (jak każdy string JS), bez BOM. Wiersze łączone `\n` (LF), bez końcowego `\n`. Bloki rozdzielone dokładnie jedną pustą linią. Kolumny rozdzielone `\t`; każdy wiersz bloku ma tyle samo kolumn (pusta komórka = pusty string).

**Liczby** (jedna implementacja w Rust, z komentarzem wskazującym `src/worktime/format.ts`):
- Czas nieujemny: minuty = `max(ms, 0) / 60_000` (w dół, jak `formatHm`). `H:MM` = godziny bez zera wiodącego, minuty dwucyfrowo (`0:00`, `7:30`, `152:05`).
- Godziny dziesiętne z tych minut: dwa miejsca, przecinek, zaokrąglenie połówek w górę w arytmetyce całkowitej `(min * 100 + 30) / 60` (remis nie występuje): `450 → 7,50`, `440 → 7,33`, `1 → 0,02`, `0 → 0,00`.
- Odchylenie: minuty = `ms / 60_000` obcięte w stronę zera (jak `formatSigned`), zapis tylko dziesiętny jak wyżej z ASCII `-` dla ujemnych, bez `+`; zero to `0,00` (także poniżej minuty, np. `-59_999 ms`). Nigdy `U+2212`, nigdy kropka dziesiętna.
- Daty ISO `YYYY-MM-DD`; chwila lokalna `YYYY-MM-DD HH:MM` (spacja zamiast `T` z `local_string`).

**Komórki tekstowe** (notatka, nazwa święta) przez jedną funkcję `cell`: każdy `\t`, każdy znak `char::is_control()` oraz `U+2028`/`U+2029` → spacja; potem przycięcie; jeśli pierwszy znak to `=`, `+`, `-`, `@`, `"`, `'` albo pełnej szerokości `＝` `＋` `－` `＠`, poprzedza go `'`. `"` w środku zostaje. Pusty zostaje pusty.

**Blok 1 — podsumowanie** (3 kolumny, stały zestaw wierszy w stałej kolejności, żeby odwołania do komórek nie zależały od fazy miesiąca; wartości `None` → puste komórki):

| Wiersz | Kolumna 2 | Kolumna 3 |
|---|---|---|
| `Podsumowanie` (nagłówek) | `H:MM` | `h` |
| `Miesiąc` | `month` (`2026-11`) | — |
| `Stan` | `zamknięty` / `bieżący` / `przyszły` (`phase`, match wyczerpujący) | — |
| `Stan na` | `as_of_ms` jako `YYYY-MM-DD HH:MM` | — |
| `Przepracowano` | `worked_ms` H:MM | h |
| `Norma miesiąca` | `norm_ms` H:MM | h |
| `Norma do dziś` | `norm_to_date_ms` | h |
| `Odchylenie` | pusta | `deviation_ms` dziesiętnie ze znakiem |
| `Pozostało` | `remaining_ms` | h |
| `Średnio / dzień` | `needed_per_day_ms` | h |
| `Dni robocze` | `workdays` | — |
| `Dni robocze po dziś` | `workdays_after_today` | — |
| `Dni wolne` | `days_off.len()` (jak kafelek: oznaczone ręcznie, bez świąt) | — |

**Blok 2 — dni** (10 kolumn, jeden wiersz na każdy `DayReport`, rosnąco):
`Data	Dzień	Rodzaj	Święto	Wpisy	Suma	Suma h	Norma	Norma h	Odchylenie h`
- `Dzień`: `pn wt śr cz pt sb nd` (match wyczerpujący po `chrono::Weekday`, jak `WEEKDAYS` w `format.ts:44`).
- `Rodzaj`: `Workday → roboczy`, `DayOff → wolny`, `Holiday → święto`, `Weekend → weekend` — match wyczerpujący po `DayKind`, **bez** gałęzi `_`.
- `Święto`: `cell(holiday)`; gdy `day_off_owed`, `"{nazwa} · należy się inny dzień wolny"` (jak `DaysTable.tsx:84`); bez święta pusta.
- `Wpisy`: `entry_count` (także `0`).
- `Suma` / `Suma h`: z `worked_ms`; obie puste dokładnie gdy `worked_ms == 0 && deviation_ms.is_none()` (ekranowe „—”).
- `Norma` / `Norma h`: z `norm_ms` dnia, zawsze wypełnione (`0:00` / `0,00` w weekend).
- `Odchylenie h`: z `deviation_ms`; pusta przy `None`.

**Blok 3 — wpisy** (6 kolumn, jeden wiersz na każdą część wpisu w dniu, dni rosnąco, w dniu kolejność `in_day` — start, id):
`Data	Od	Do	Czas	Czas h	Notatka`
- Wiersz powstaje tylko dla `portion(start, end, now, day_start, day_end) > 0` (ta sama reguła co `entry_count`); wpis przez północ jest w obu dniach, każdy z własną częścią.
- `Od` / `Do`: lokalny czas wpisu (nie przycięty do dnia): `HH:MM`, gdy lokalna data z `local_string` jest datą wiersza, w przeciwnym razie `YYYY-MM-DD HH:MM` (ta sama reguła co `localLabel` w `WorkTimeEntries.tsx:22-26`, więc koniec dokładnie o północy, np. 22:00–00:00, ma w `Do` datę następnego dnia); `Do` trwającego wpisu to `trwa`.
- `Czas` / `Czas h`: część wpisu w dniu (trwający do `now`).
- `Notatka`: `cell(note)`; bez notatki pusta (wiersz kończy się wtedy `\t`).
- Miesiąc bez wpisów: blok ma tylko wiersz nagłówka.

Przykład (wymyślone dane, `now` = pon. 2026-11-23 12:00, strefa Warszawa):
```
2026-11-11	śr	święto	Święto Niepodległości	1	2:00	2,00	0:00	0,00	2,00
2026-11-19	22:00	2026-11-20 02:00	2:00	2,00	'=SUMA(A1)
2026-11-20	2026-11-19 22:00	02:00	2:00	2,00	'=SUMA(A1)
2026-11-23	09:00	trwa	3:00	3,00	
```
(Nazwa święta w teście brana z `DayReport.holiday`, nie wpisana na sztywno. Ostatni wiersz kończy się `\t`: pusta notatka.)

Znane ograniczenie (default chosen by agent, user may revise): komórkę tekstową, która wygląda jak liczba, data albo czas (np. notatka `12.11` albo `7:30`, a także `Miesiąc` = `2026-11`), arkusz zamieni na liczbę lub datę; prefiks `'` dostają tylko znaki formuł i `"`, `'`, żeby zwykłe notatki nie zaczynały się od apostrofu.

## Critical Implementation Details

- **Jeden snapshot, ta sama reguła.** `month_report_text` w jednym `store.read` woła `compute(c, day, now, tz, settings)` i `in_day(c, month_start, month_end, now)` (granice z `norm::month_days` + `day_bounds`, tak jak `report.rs:130-132`), a części dni liczy tym samym `portion` i `day_bounds` co `compute`. Liczby dni i podsumowania czyta wyłącznie z pól raportu (`worked_ms`, `norm_ms`, `deviation_ms`, `kind`, `holiday`, `day_off_owed`, `figures`, `days_off`), nigdy z `DAILY_NORM_MS` ani z własnego przeliczenia norm.
- **Schowek w WebView2.** `writeText` ma iść zaraz po `await` komendy w tym samym handlerze kliknięcia, bez dodatkowego `await` pomiędzy i bez `setTimeout`. Twardym warunkiem Chromium jest skupiony dokument: bez skupienia `writeText` odrzuca `NotAllowedError` („Document is not focused”), np. gdy użytkownik przełączy okno w trakcie komendy albo gdy skupione jest okno DevTools. Przejściowa aktywacja z kliknięcia trwa w Chromium ok. 5 s (stała Chromium, niesprawdzona w WebView2) i `writeText` jej nie zużywa; komenda czyta jeden miesiąc w milisekundach (najdłużej czeka na blokadę store'u, gdy runda zdrowia robi kopię zapasową), więc po `await` aktywacja jest jeszcze ważna. Odrzucenie `writeText` (także brak `navigator.clipboard`, czyli `TypeError`) łapać osobno od błędu komendy i pokazać z nazwą błędu (`e.name`, sama nazwa, bez treści), żeby ręczne sprawdzenie odróżniło `NotAllowedError` od braku API i mogło zdecydować o wariancie awaryjnym.
- **Pola raportu bez `..`.** `month_text.rs` czyta `MonthReport`, `NormFigures` i `DayReport` przez destrukturyzację bez `..` (np. `let DayReport { day, kind, worked_ms, norm_ms, deviation_ms, entry_count, holiday, day_off_owed } = d;`), tak jak `match` bez `_`: nowe pole z równoległego slice'a (S-02, S-08) to błąd kompilacji przy merge'u, a nie cicho pominięta wartość.
- **Kontrakt przy merge'u**: zob. „Merge notes”.

## Phase 1: Budowanie TSV w Rust i komenda

### Overview

Nowy moduł formatujący raport miesiąca jako TSV, z testami na wymyślonych danych, oraz komenda `work_month_report_text(day)` zarejestrowana w `lib.rs`. UI bez zmian.

### Changes Required:

#### 1. Moduł `month_text`

**File**: `src-tauri/src/work_time/month_text.rs` (nowy)

**Intent**: Zbudować tekst z sekcji „Format TSV” z raportu i wpisów miesiąca z jednego snapshotu; bez importu `tauri`, bez zegara, bez logowania (wzór `today.rs`).

**Contract**:
- `pub fn month_report_text<Tz: TimeZone>(store: &Store, day: NaiveDate, now: i64, tz: &Tz, settings: &WorkTimeSettings) -> Result<String, WorkTimeError>` — jeden `store.read`, w nim `super::report::compute` i `super::in_day`.
- Pomocnicze funkcje `pub(super)` (dla testów w module rodzeństwa): `cell(&str) -> String`, `hm(ms) -> String` (H:MM, w dół), `hours(ms) -> String` (dziesiętnie, w dół), `signed_hours(ms) -> String` (obcięte do zera, ASCII `-`; zaokrąglenie `(m * 100 + 30) / 60` liczone na `|m|`, znak dokładany potem — ten sam wzór na ujemnym `m` z obcinającym `/` dałby `-1,49` dla −90 min). Reszta wewnętrzna według uznania implementującego.
- Mapowania `MonthPhase`, `DayKind`, `Weekday` przez `match` bez gałęzi `_`; pola `MonthReport`, `NormFigures`, `DayReport` przez destrukturyzację bez `..` (sekcja „Critical Implementation Details”).
- Komentarz modułu: tekst trafia tylko do schowka na kliknięcie użytkownika; nigdy do logu ani dziennika zdarzeń (notatka to treść użytkownika).

#### 2. Rejestracja modułu

**File**: `src-tauri/src/work_time/mod.rs`

**Intent**: Dwie linie w istniejących posortowanych listach.

**Contract**: `pub mod month_text;` (między `pub mod holidays;` a `pub mod norm;`) i `#[cfg(test)] mod month_text_tests;` (między `holidays_tests` a `norm_tests`). Nic więcej w `mod.rs`.

#### 3. Komenda

**File**: `src-tauri/src/work_time/commands.rs`

**Intent**: Cienki wrapper dopisany na końcu komend (po `set_day_off`, przed `#[cfg(test)]`), z tą samą polityką ustawień co `work_month_report`, żeby kopia i ekran miały te same liczby i ten sam błąd przy złym bloku `workTime`.

**Contract**: `#[tauri::command] pub async fn work_month_report_text(app: AppHandle, day: String) -> Result<String, WorkTimeError>`: `report::parse_day(&day)?`, potem `run_with_settings(&app, "month_report_text", Policy::Strict, move |store, now, settings| super::month_text::month_report_text(store, day, now, &Local, settings))`. Bez `notify`, bez `events::`, bez nowych linii `log::`. `execute` bez zmian.

#### 4. Rejestracja komendy

**File**: `src-tauri/src/lib.rs`

**Intent**: Jedna linia w `generate_handler!`.

**Contract**: `work_time::commands::work_month_report_text,` zaraz po `work_time::commands::work_month_report,`.

#### 5. Testy

**File**: `src-tauri/src/work_time/month_text_tests.rs` (nowy)

**Intent**: Udowodnić format i zgodność z raportem na wymyślonych danych w `chrono_tz::Europe::Warsaw` (wzór fixture'u `today_tests.rs:1-29`: `resolve_local`, `Store::open(tempdir, schema::MIGRATIONS, day)`, `WorkTimeSettings::default()`).

**Contract** (nazwy przykładowe):
- `cell_*`: `\t`, `\u{7}`, `\u{2028}` → spacja; przycięcie; prefiks `'` dla każdego z `= + - @ " '` i `＝ ＋ － ＠`; `"` w środku bez zmian; `""` → `""`; zwykły tekst bez zmian.
- `numbers_*`: `hm`: `0`, `59_999` → `0:00`, `7 h 30 min` → `7:30`, `40 h 15 min` → `40:15`, wartość ujemna → `0:00`; `hours`: `7:30 → 7,50`, `7:20 → 7,33`, `0:01 → 0,02`, `0 → 0,00`; `signed_hours`: `-90 min → -1,50`, `-59_999 ms → 0,00`, `-61_000 ms → -0,02`, `+61_000 ms → 0,02` (bez `+`); żaden wynik nie zawiera `.` ani `U+2212`.
- `current_month_matches_the_report`: listopad 2026, `now` = 2026-11-23T12:00; wpisy: zamknięty w dzień roboczy z notatką z tabulatorem, wpis przez północ 2026-11-19T22:00–2026-11-20T02:00 z notatką `=SUMA(A1)`, 2 h w święto 2026-11-11, dzień wolny 2026-11-13 (`report::set_day_off`), trwający od 09:00 (`start`). Wywołać `month_report` i `month_report_text` z tym samym `now`; sparsować TSV i sprawdzić: 3 bloki; blok 1 ma 13 wierszy (z nagłówkiem), blok 2 ma 31 (nagłówek + 30 dni); każda komórka dnia równa sformatowanemu polu `DayReport` (w tym puste „Suma” dokładnie wg reguły); `Rodzaj` 11.11 = `święto`, 13.11 = `wolny`; liczba wierszy wpisów dnia = `entry_count`; wpis przez północ w obu dniach z `Od`/`Do` w formie z datą po drugiej stronie północy i `'=SUMA(A1)`; trwający z `Do` = `trwa` i `Czas` = `3:00`; podsumowanie równe polom `MonthReport`/`NormFigures` (`Stan` = `bieżący`, `Dni wolne` = `1`). Wiersze z przykładu w sekcji „Format TSV” występują dosłownie (nazwa święta z raportu).
- `past_month_with_dst_and_a_saturday_holiday`: sierpień 2026 (15.08 to sobota): `Święto` = `"{holiday} · należy się inny dzień wolny"`, `Rodzaj` = `weekend`. Październik 2026 (zmiana czasu 25.10): wpis 01:00–04:00 lokalnie liczy się jako `4:00`, 31 wierszy dni; `Stan` = `zamknięty`; `Norma do dziś` = `Norma miesiąca`; `Pozostało`, `Średnio / dzień`, `Dni robocze po dziś` puste; `Odchylenie` = `signed_hours(deviation_ms)`.
- `future_month_is_empty_but_shaped`: grudzień 2026: `Stan` = `przyszły`, `Odchylenie`, `Norma do dziś`, `Pozostało`, `Średnio / dzień` i `Dni robocze po dziś` puste, `Norma miesiąca` i `Dni robocze` wypełnione, wszystkie „Suma” i „Odchylenie h” dni puste, blok wpisów to tylko nagłówek.
- `shape_is_rectangular`: w każdym teście z danymi: brak `\r`, brak końcowego `\n`, `text.split("\n\n")` daje 3 bloki, każdy wiersz bloku 1/2/3 ma odpowiednio 2/9/5 tabulatorów.
- `sentinel_note_reaches_only_the_text`: notatka `SENTINEL-invented-month-copy` jest w TSV (po `cell`), a JSON błędu `SettingsInvalid`/`InvalidLocalTime` z `serde_json::to_string` jej nie zawiera (wzór S-06).

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym `work_time::month_text_tests` (komórki, liczby, zgodność z raportem w bieżącym, zamkniętym z DST, z sobotnim świętem i przyszłym miesiącu, kształt bloków, sentinel)
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -n "log::" -- src-tauri/src/work_time` pokazuje dokładnie cztery dotychczasowe linie, wszystkie w `commands.rs`
- `git grep -nE "_ =>|DAILY_NORM_MS|use tauri|log::|notify\(" -- src-tauri/src/work_time/month_text.rs` nic nie znajduje, a `git diff --quiet d34904a -- src-tauri/src/work_time/report.rs src-tauri/src/work_time/norm.rs src/worktime/types.ts src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities src-tauri/tauri.conf.json` kończy się kodem 0

**Implementation Note**: Po zielonej weryfikacji commit tylko tej fazy, np. `feat(month-report-copy): build the month report as TSV in Rust`. Bez podbijania wersji.

---

## Phase 2: Przycisk „Kopiuj miesiąc” i dokumentacja

### Overview

UI woła nową komendę dla pokazanego miesiąca, zapisuje tekst do schowka i pokazuje potwierdzenie albo błąd; README i CHANGELOG opisują funkcję.

### Changes Required:

#### 1. API

**File**: `src/worktime/api.ts`

**Intent**: Jedna typowana funkcja dla nowej komendy (zwraca `string`, więc bez zmian w `types.ts`).

**Contract**: `/** \`day\` is any \`YYYY-MM-DD\` within the month; TSV for the clipboard. */ export function workMonthReportText(day: string): Promise<string>` → `invoke<string>("work_month_report_text", { day })`, zaraz po `workMonthReport`.

#### 2. Hook

**File**: `src/worktime/useMonthCopy.ts` (nowy)

**Intent**: Stan jednego kopiowania: zajęty, skopiowano, błąd — przypisany do miesiąca, w którym kliknięto, żeby potwierdzenie nie przechodziło na inny miesiąc.

**Contract**: `useMonthCopy(month: string)` zwraca `{ copy: () => Promise<void>, busy: boolean, copied: boolean, error: string | null }` (`copied`/`error` tylko dla bieżącego `month`). Wynik trzymany w `useState` razem z miesiącem kliknięcia (`{ month, copied } | { month, error } | null`), a `copied`/`error` wyprowadzane w renderze przez porównanie z bieżącym `month`, jak `useMonthReport.ts:95,116`; bez `useEffect` czyszczącego stan przy zmianie miesiąca i bez czytania refów w renderze (reguły `eslint-plugin-react-hooks` 7 w `npm run lint`). `copy`: czyści poprzedni wynik, `busy = true`; `text = await workMonthReportText(\`${month}-01\`)` — błąd → `workTimeErrorMessage(e)`; potem od razu `await navigator.clipboard.writeText(text)` w osobnym `try` — błąd → `Nie udało się skopiować do schowka (${name})`, gdzie `name` to `e.name` dla `Error`/`DOMException` (np. `NotAllowedError`, `TypeError`), inaczej `nieznany błąd`; sukces → `copied`. Tekst nie jest trzymany w stanie ani logowany (żadnego `console.`). Wynik zostaje do następnego kliknięcia lub zmiany miesiąca (jak „Zapisano.”, bez timera).

#### 3. Komponent

**File**: `src/worktime/MonthCopyButton.tsx` (nowy), `src/worktime/WorkTime.css`

**Intent**: Przycisk i komunikaty w nagłówku widoku, logika w hooku.

**Contract**: `MonthCopyButton(props: { month: string; enabled: boolean })` renderuje `<div className="wt-month-copy">`: zawsze zamontowany `<div role="status">` z `Notice tone="info" compact role={null}` „Skopiowano.” gdy `copied`; `Notice tone="error" compact` z komunikatem gdy `error`; `Button variant="quiet" size="sm"` z tekstem „Kopiuj miesiąc” („Kopiuję…” w trakcie), `disabled={!enabled || busy}`. W `WorkTime.css`, zaraz po regule `.wt-month-name`, reguła układu `.wt-month-copy` (flex, `align-items: center`, `flex-wrap: wrap`, `justify-content: flex-end`, `gap: var(--space-2)`, `min-width: 0`) — tylko tokeny.

#### 4. Widok

**File**: `src/worktime/WorkTimeView.tsx`

**Intent**: Wstawić przycisk w nagłówek po prawej; enabled tylko przy wczytanym raporcie pokazanego miesiąca.

**Contract**: import `MonthCopyButton`; w `dn-view-head`, po `div.wt-month-switch`: `<span className="dn-spacer" />` i `<MonthCopyButton month={month} enabled={report.state === "loaded"} />` (`month` i `report` już są w widoku). Nic więcej.

#### 5. Dokumentacja

**File**: `README.md`, `CHANGELOG.md`

**Intent**: Krótkie dopisane punkty, bez przeredagowania cudzych.

**Contract**: README, lista „Czas pracy (Work time)” (`README.md:53-60`): nowy podpunkt, np. „a "Kopiuj miesiąc" button that copies the shown month (summary, days, entries with notes, holidays) as tab-separated text for a spreadsheet; numbers use a decimal comma, for Excel or Google Sheets set to Polish (pl-PL); in a sheet with another locale the decimal hours may paste as text or as a wrong number (7,50 as 750), while the H:MM columns stay correct;”. CHANGELOG `[Unreleased] / Added`: nowy punkt na końcu listy, np. „"Czas pracy": a "Kopiuj miesiąc" button copies the shown month to the clipboard as tab-separated text that pastes into a spreadsheet: a summary, one row per day (kind, holiday name, sum, norm, deviation) and one row per entry with its note. Numbers use a decimal comma, for pl-PL Excel and Google Sheets.”

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą
- `git grep -n "invoke" -- src/worktime` znajduje `invoke` tylko w `src/worktime/api.ts`, `git grep -n "console\." -- src/worktime` nic nie znajduje, a `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css` nic nie znajduje
- `git diff --quiet d34904a -- package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities src-tauri/tauri.conf.json src/worktime/types.ts` kończy się kodem 0
- `git grep -n "Kopiuj miesiąc" -- README.md CHANGELOG.md` i `git grep -n "pl-PL" -- README.md CHANGELOG.md` znajdują nowe punkty w obu plikach

#### Manual Verification:

- W zbudowanej aplikacji (`npm run tauri build -- --debug --no-bundle`) „Kopiuj miesiąc” kliknięty w skupionym oknie aplikacji (nie z konsoli DevTools) pokazuje „Skopiowano.”, a wklejenie do Notatnika daje trzy bloki; konsola DevTools nie pokazuje naruszenia CSP ani „IPC custom protocol failed”
- Wklejenie do Excela z ustawieniami pl-PL: każda wartość we własnej komórce, daty ISO jako daty, `7:30` jako czas, `40:15` w podsumowaniu jako czas trwania, `7,50` i `-1,50` jako liczby, notatka zaczynająca się od `=` lub `"` jako tekst bez formuły, notatka z tabulatorem w jednej komórce
- To samo wklejenie w Arkuszach Google z ustawieniami regionalnymi Polska
- Zapisane w planie, czy w schowku są końce linii LF czy CRLF (np. czy wynik `Get-Clipboard -Raw` w PowerShell zawiera znak CR), a wklejenie działa w obu arkuszach
- Przycisk jest nieaktywny podczas ładowania i przy błędzie raportu; po zepsuciu bloku „workTime” w settings.json przy wczytanym raporcie pierwsze kliknięcie po powrocie do okna pokazuje komunikat błędu albo przycisk jest już nieaktywny (powrót do okna odświeża raport), nigdy „Skopiowano.” (potem przywrócić settings.json)
- Przy biegnącym liczniku dzisiejszy wiersz kopii zgadza się z tabelą dni w chwili kliknięcia; po zmianie miesiąca „Skopiowano.” znika
- Plik logu aplikacji z tej sesji i panel „Dziennik zdarzeń” nie zawierają treści notatek po kopiowaniu

**Implementation Note**: Po zielonej weryfikacji automatycznej commit tylko tej fazy, np. `feat(month-report-copy): copy the month from the work-time view`. Wiersze Manual zostają dla użytkownika. Migracji nie ma, więc zbudowana aplikacja nie zmienia schematu prawdziwej bazy; wklejony tekst zawiera jednak prawdziwe notatki użytkownika — do sprawdzenia w Arkuszach Google użyć miesiąca bez notatek służbowych albo usunąć arkusz po teście. Jeśli pierwszy wiersz Manual nie przejdzie (komunikat z nazwą błędu, np. `NotAllowedError`), nie łatać w tej fazie: zob. „Wariant awaryjny schowka”. Jeśli `npm install` był potrzebny i zmienił w `package-lock.json` tylko linię `"license": "MIT"`, cofnąć tę zmianę przed weryfikacją (inaczej kryterium z `git diff --quiet` nie przejdzie).

---

## Testing Strategy

### Unit Tests:

- `month_text_tests.rs`: `cell`, `hm`, `hours`, `signed_hours` z przypadkami granicznymi; zgodność każdej komórki z `MonthReport` w bieżącym, zamkniętym (DST, sobotnie święto) i przyszłym miesiącu; wpis przez północ i trwający; kształt bloków; sentinel notatki.
- Oczekiwane wartości liczone z pól raportu (np. `hm(day.norm_ms)`), nie wpisywane na sztywno, poza dosłownymi wierszami przykładu — tak, żeby zmiana reguł normy w S-02 nie łamała testów formatu.

### Integration Tests:

- Brak nowych w `tests/`: komenda to cienki wrapper istniejącego `run_with_settings`, a jej logika jest w czystej funkcji z testami. Polityka `Strict` jest już testowana w `commands.rs` (`strict_refuses_*`).

### Manual Testing Steps:

1. Zbuduj aplikację: `npm run tauri build -- --debug --no-bundle`; otwórz „Czas pracy”, otwórz DevTools.
2. Kliknij „Kopiuj miesiąc”: „Skopiowano.”, brak błędów CSP/IPC w konsoli. Wklej do Notatnika.
3. Wklej do Excela (pl-PL) i do Arkuszy Google (Polska); sprawdź typy komórek z listy Manual.
4. Sprawdź LF/CRLF w schowku.
5. Przełącz na miesiąc zamknięty i przyszły; skopiuj; porównaj z kafelkami i tabelą.
6. Zepsuj blok „workTime” po wczytaniu raportu, kliknij: komunikat błędu; przywróć plik.
7. Przejrzyj log aplikacji i „Dziennik zdarzeń”: bez notatek.

## Performance Considerations

Jedno kliknięcie = jeden dodatkowy odczyt miesiąca (te same zapytania co raport plus jedno `in_day` za miesiąc); kilkadziesiąt wierszy, poniżej milisekund. Bez cache'owania.

## Migration Notes

Brak migracji: kopia czyta istniejące `work_entries` (z `note`, v11) i `days_off`, niczego nie zapisuje do store'u ani na dysk.

## Wariant awaryjny schowka (default chosen by agent, user may revise)

Tylko jeśli ręczne sprawdzenie w zbudowanej aplikacji wykaże, że `navigator.clipboard.writeText` nie działa: osobna zmiana dodaje `tauri-plugin-clipboard-manager` 2.5.x używany wyłącznie z Rust (crate w `Cargo.toml`, `.plugin(tauri_plugin_clipboard_manager::init())` w `lib.rs`, zapis przez `ClipboardExt::clipboard().write_text` w komendzie, bez pakietu npm i bez uprawnień — reguła wtyczek tylko z Rust jak `window-state`), `cargo audit` dla nowych crate'ów, nowy wariant `WorkTimeError` bez danych dla błędu schowka z lustrem TS i komunikatem; komenda zwraca wtedy `()`, a hook przestaje wołać `writeText`. Nie implementowane w tej zmianie.

## Merge notes

- **S-02 configurable-daily-norm** (pół dnia wolnego, konfigurowalna norma): norma dnia płynie przez `DayReport.norm_ms`, a miesiąca przez `NormFigures`, więc TSV przejmuje ją bez zmian. Nowy wariant `DayKind` (np. pół dnia wolnego) da błąd kompilacji w `month_text.rs` (match bez `_`) — gałąź scalana druga dopisuje etykietę `Rodzaj` (np. „pół dnia wolnego”) i jeden przypadek w teście; bez nowej kolumny, żeby kształt bloku dni został. Nowe pole w `DayReport`, `MonthReport` albo `NormFigures` (np. pół dnia wolnego jako pole zamiast wariantu, norma dzienna, inny kształt `days_off`) też da błąd kompilacji w `month_text.rs` (destrukturyzacja bez `..`); gałąź scalana druga decyduje: wartość w istniejącej kolumnie (np. `Rodzaj`), wiersz dopisany **na końcu** podsumowania (żeby istniejące odwołania do komórek nie przesunęły się) albo jawne `pole: _` z komentarzem, dlaczego go nie ma w kopii. Zmiana sygnatury `compute` albo `month_report` w S-02 = dopasowanie jednego wywołania w `month_text.rs`. Testy liczą oczekiwane wartości z pól raportu, więc zmiana reguł normy łamie najwyżej dosłowne wiersze przykładu (dla 11.11 norma święta zostaje 0).
- **S-08 week-summary**: może dodać moduł w `mod.rs`, funkcję w `api.ts` i sekcję w `WorkTimeView.tsx` — konflikty tylko kontekstowe w listach; zachować obie linie.
- **S-07 end-of-day-summary**: rusza ścieżkę STOP w `commands.rs:148-156`; ta zmiana dopisuje komendę na końcu pliku — bez nakładania.
- **S-04 windows-autostart**: dodaje wtyczkę w `lib.rs` i `Cargo.toml`; tu tylko jedna linia w `generate_handler!`.
- `CHANGELOG.md` / `README.md`: osobne punkty; przy konflikcie zachować wszystkie.

## Decisions (defaults chosen by agent, user may revise)

Plan powstał bez dostępu do użytkownika. Decyzje użytkownika (stałe): TSV; tylko schowek, bez pliku; tekst budowany w Rust z tego samego raportu, co widok, testowany na wymyślonych danych; UI tylko wyzwala kopię i pokazuje potwierdzenie albo błąd; notatka wpisu i nazwa święta w tekście; MS-24 bez nowych FR. Wartości domyślne agenta:

1. Nowa komenda `work_month_report_text(day)` przelicza raport w chwili kliknięcia (`compute` + `in_day` w jednym `store.read`, `Policy::Strict`); UI nie odsyła raportu.
2. Schowek przez `navigator.clipboard.writeText` zaraz po `await` komendy; wtyczka schowka tylko jako zapisany wariant awaryjny.
3. Trzy bloki (podsumowanie, dni, wpisy) zamiast jednej płaskiej tabeli: blok dni to prostokąt, którego kolumny sumują się do miesiąca, a blok wpisów niesie notatki bez powtarzania kolumn dnia.
4. Podsumowanie ma stały zestaw 13 wierszy w stałej kolejności, z pustymi komórkami dla wartości nieustalonych w danej fazie (zamiast pomijania wierszy jak kafelki), żeby odwołania do komórek były stabilne.
5. Czasy nieujemne w dwóch kolumnach: `H:MM` i godziny dziesiętne z przecinkiem, dwa miejsca; odchylenia tylko dziesiętnie, ASCII `-`, bez `+`; bez kolumny „ekranowej” `−1:30` (Excel nie ma ujemnych czasów, a `+1:30` byłby formułą).
6. Bez kolumny minut: obcinanie każdej komórki do minuty jak na ekranie i tak nie daje dokładnej sumy, gdy wpisy mają sekundy, więc kolumna minut nie usuwa rozbieżności; dokładność gwarantuje „ta sama liczba co na ekranie” w każdej komórce.
7. Pusta komórka dla brakującej wartości; „Suma” dnia pusta dokładnie tam, gdzie ekran pokazuje „—”; norma dnia zawsze wypełniona; „Wpisy” zawsze liczbą.
8. `Od`/`Do` jako `HH:MM` w dniu wiersza, w przeciwnym razie `YYYY-MM-DD HH:MM`; trwający wpis `trwa`.
9. Komórki tekstowe przez jedną funkcję `cell` (tabulator, znaki sterujące, `U+2028`/`U+2029` → spacja; prefiks `'` przed `= + - @ " '` i formami pełnej szerokości).
10. LF, bez BOM, bez końcowego `\n`, jedna pusta linia między blokami; przecinek dziesiętny pod pl-PL, opisany w README i CHANGELOG (README mówi też, że w innym locale godziny dziesiętne mogą wkleić się jako tekst albo zła liczba, a kolumny `H:MM` zostają poprawne).
11. Przycisk tekstowy `quiet`/`sm` „Kopiuj miesiąc” w nagłówku po `dn-spacer`, aktywny tylko przy wczytanym raporcie; „Skopiowano.” w zawsze zamontowanym `role="status"`, błąd jako `Notice tone="error" compact`; komunikat schowka „Nie udało się skopiować do schowka (<nazwa błędu>)”, błąd komendy przez `workTimeErrorMessage`. Wynik zostaje do następnego kliknięcia lub zmiany miesiąca.
12. Nowa logika w nowych plikach (`month_text.rs`, `month_text_tests.rs`, `useMonthCopy.ts`, `MonthCopyButton.tsx`); w plikach współdzielonych tylko dopisane linie; `report.rs`, `norm.rs`, `types.ts` bez zmian.
13. Dwie fazy: Rust (builder, komenda, testy) → UI i dokumentacja.
14. Bez synchronizacji statusu w `roadmap.md` (zakaz edycji w tej zmianie).

## References

- Research: `context/changes/month-report-copy/research.md`
- Roadmap: `context/foundation/roadmap.md` (S-05, M-3), PRD MS-24, GitHub issue #24
- Raport i reguły: `src-tauri/src/work_time/report.rs:23-183`, `norm.rs:46-152`, `rules.rs:24-63`, `mod.rs:95-153`
- Wzór modułu i fixture'u testów: `src-tauri/src/work_time/today.rs`, `today_tests.rs:1-43`
- Wrapper komend: `src-tauri/src/work_time/commands.rs:71-106,213-223`
- Formatowanie ekranu: `src/worktime/format.ts:17-50`, `DaysTable.tsx:44,82-86,104`, `MonthReport.tsx:8-49`
- Wzory UI: `src/github/GithubView.tsx:182-195`, `src/own-tasks/OwnTaskEditPanel.tsx:134-141`, `src/components/Notice/Notice.tsx:15-30`
- Precedens notatki i sentinela: `context/archive/2026-11-26-work-entry-note/plan.md`
- Wtyczka awaryjna: https://v2.tauri.app/plugin/clipboard/

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Budowanie TSV w Rust i komenda

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi, w tym `work_time::month_text_tests` (komórki, liczby, zgodność z raportem w bieżącym, zamkniętym z DST, z sobotnim świętem i przyszłym miesiącu, kształt bloków, sentinel)
- [x] 1.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` bez uwag
- [x] 1.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- [x] 1.4 `git grep -n "log::" -- src-tauri/src/work_time` pokazuje dokładnie cztery dotychczasowe linie, wszystkie w `commands.rs`
- [x] 1.5 `git grep -nE "_ =>|DAILY_NORM_MS|use tauri|log::|notify\(" -- src-tauri/src/work_time/month_text.rs` nic nie znajduje, a `git diff --quiet d34904a -- src-tauri/src/work_time/report.rs src-tauri/src/work_time/norm.rs src/worktime/types.ts src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities src-tauri/tauri.conf.json` kończy się kodem 0

### Phase 2: Przycisk „Kopiuj miesiąc” i dokumentacja

#### Automated

- [x] 2.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą
- [x] 2.3 `git grep -n "invoke" -- src/worktime` znajduje `invoke` tylko w `src/worktime/api.ts`, `git grep -n "console\." -- src/worktime` nic nie znajduje, a `git grep -nE "#[0-9a-fA-F]{3,8}|rgba?\(" -- src/worktime/WorkTime.css` nic nie znajduje
- [x] 2.4 `git diff --quiet d34904a -- package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities src-tauri/tauri.conf.json src/worktime/types.ts` kończy się kodem 0
- [x] 2.5 `git grep -n "Kopiuj miesiąc" -- README.md CHANGELOG.md` i `git grep -n "pl-PL" -- README.md CHANGELOG.md` znajdują nowe punkty w obu plikach

#### Manual

- [x] 2.6 W zbudowanej aplikacji (`npm run tauri build -- --debug --no-bundle`) „Kopiuj miesiąc” kliknięty w skupionym oknie aplikacji (nie z konsoli DevTools) pokazuje „Skopiowano.”, a wklejenie do Notatnika daje trzy bloki; konsola DevTools nie pokazuje naruszenia CSP ani „IPC custom protocol failed” — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.7 Wklejenie do Excela z ustawieniami pl-PL: każda wartość we własnej komórce, daty ISO jako daty, `7:30` jako czas, `40:15` w podsumowaniu jako czas trwania, `7,50` i `-1,50` jako liczby, notatka zaczynająca się od `=` lub `"` jako tekst bez formuły, notatka z tabulatorem w jednej komórce — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.8 To samo wklejenie w Arkuszach Google z ustawieniami regionalnymi Polska — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.9 Zapisane w planie, czy w schowku są końce linii LF czy CRLF (np. czy wynik `Get-Clipboard -Raw` w PowerShell zawiera znak CR), a wklejenie działa w obu arkuszach — checked by the user on 2026-11-27 on m3-wave2-integration: paste worked; line-ending type not recorded
- [x] 2.10 Przycisk jest nieaktywny podczas ładowania i przy błędzie raportu; po zepsuciu bloku „workTime” w settings.json przy wczytanym raporcie pierwsze kliknięcie po powrocie do okna pokazuje komunikat błędu albo przycisk jest już nieaktywny (powrót do okna odświeża raport), nigdy „Skopiowano.” (potem przywrócić settings.json) — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.11 Przy biegnącym liczniku dzisiejszy wiersz kopii zgadza się z tabelą dni w chwili kliknięcia; po zmianie miesiąca „Skopiowano.” znika — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.12 Plik logu aplikacji z tej sesji i panel „Dziennik zdarzeń” nie zawierają treści notatek po kopiowaniu — checked by the user on 2026-11-27 on m3-wave2-integration
