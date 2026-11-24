# Tabela dni miesiąca w „Czas pracy” — Plan Brief

> Full plan: `context/changes/worktime-days-table/plan.md`

## What & Why

W „Czas pracy” pojawia się tabela wszystkich dni miesiąca: liczba wpisów, pasek na tle normy dnia, suma, odchylenie od normy dnia i przełącznik „Wolne”. Dziś jest wyróżniony, a dni wolne zakreskowane (FR-020, issue #2, slice S-05). Użytkownik widzi, który dzień ciągnie odchylenie miesiąca w dół, i oznacza urlop albo święto prosto z tabeli. Reguła normy zostaje w Rust (`AGENTS.md`).

## Starting Point

Rust liczy tylko liczby miesiąca (`report::compute`), bez sum dziennych, dlatego w 0.2.0 tabeli nie ma. Widok ma kafelki miesiąca, wiersz linków „Dni wolne:”, panel „Wybrany dzień” z checkboxem i panel wpisów. `HoursBar` istnieje tylko w design systemie, z normą 8 h wpisaną w UI.

## Desired End State

Panel „Dni miesiąca” w kolumnie głównej ma po jednym wierszu na dzień (`Dzień | Wpisy | Na tle normy | Suma | ± | Wolne`). Wiersz dziś biegnie na żywo ze wspólnego licznika. Klik w datę wybiera dzień w panelu wpisów. Suma odchyleń wierszy równa się kafelkowi „Odchylenie”, a suma sum kafelkowi „Miesiąc”. Wiersz linków „Dni wolne:” znika.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Źródło danych | Nowe pole `days` w istniejącym `MonthReport` | Jeden snapshot, istniejące odświeżanie `useMonthReport`, bez nowej komendy i subskrypcji. |
| Kolumna „Wpisy” | Liczba: `1 wpis` / `2 wpisy` / `5 wpisów` (`plural`) | Krótka komórka; zakresy pokazuje panel wpisów wybranego dnia. |
| Odchylenie dnia | Jak w miesiącu: dni ≤ dziś = przepracowane − norma dnia; przyszłe puste; dzień bez normy i bez godzin `—` | Wiersze sumują się do kafelka „Odchylenie”, więc obowiązuje jedna reguła. |
| Norma dnia | 8 h w dzień roboczy, 0 h w dzień wolny i weekend, z Rust (`norm::day_norm_ms`) | UI nie zna liczby 8 h. `HoursBar.norm` jest wymagane. |
| Wiersze | Wszystkie dni; dni wolne kreskowane, weekendy wyciszone bez przełącznika | Kreskowanie znaczy „oznaczone przez ciebie”, a praca w weekend zostaje widoczna. |
| Istniejące kontrolki | Checkbox w „Wybrany dzień” zostaje; wiersz linków „Dni wolne:” znika | Brak zdublowanej listy; klik w datę w tabeli zastępuje linki. |
| Wiersz dziś na żywo | Ekstrapolacja wartości z Rust o `todayTotalMs` z `useWorkTimer()` | Ten sam wzorzec co nagłówek, bez drugiej subskrypcji i bez reguły w UI. |
| Style | Stany `current`/`hatched` i tony `info`/`warn` w `DataTable`, token `--hatch-bg` | CSS widoku nie może mieć kolorów. |

## Scope

**In scope:**
- `DayReport` w Rust (`norm.rs`, `report.rs`) z testami jednostkowymi i proptestem, plus lustro TS.
- `HoursBar` w `src/components/`, rozszerzenie `DataTable`, token kreskowania.
- `DaysTable`, `useDayOffToggle`, zmiany w `WorkTimeView`/`MonthReport`, akapit „Jak liczę”.
- Linijka w F-05 Outcome w roadmapie.

**Out of scope:**
- Nowa komenda, zdarzenie, migracja; konfigurowalna norma, kalendarz świąt.
- Zakresy godzin w tabeli, edycja wpisów z tabeli, sortowanie.
- Panel przypomnień (S-06), synchronizacja artefaktu (F-05).

## Architecture / Approach

`compute` wczytuje wpisy i dni wolne raz. Dla każdego lokalnego dnia miesiąca bierze `day_bounds` i sumuje `portion`. Normę i odchylenie dnia dają czyste `norm::day_norm_ms` i `norm::day_deviation_ms`. Wynik `days` płynie w `work_month_report` do `useMonthReport`. `DaysTable` renderuje go przez `DataTable` + `HoursBar`, a wiersz dziś przesuwa o żywą sumę z `useWorkTimer()`. Przełącznik woła `set_day_off`, a `work-time-changed` odświeża raport, nagłówek i wpisy.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Raport dzienny w Rust i lustro TS | `MonthReport.days` z normą i odchyleniem dnia, testy i proptest sum | Doby DST i wpisy przez północ lub granicę miesiąca |
| 2. Tabela dni w „Czas pracy” | Panel „Dni miesiąca” z paskiem, kreskowaniem, wierszem dziś na żywo i przełącznikiem | Przełącznik odskakujący przed przeładowaniem; szerokość przy 800 px |

**Prerequisites:** F-01 i F-02 w `develop` (są). Brak migracji, więc baza użytkownika (v9) nie jest dotykana.
**Estimated effort:** ~1 sesja, 2 fazy.

## Open Risks & Assumptions

- Przy zegarze cofniętym za zapisane wpisy przyszły dzień może mieć godziny bez odchylenia. Wtedy wiersze nie sumują się do kafelka. To ten sam skrajny przypadek co dziś w raporcie, akceptowany.
- S-06 edytuje ten sam widok po tym slice'ie. `WorkTimeView.tsx` będzie miejscem konfliktu, jeśli S-06 zacznie się przed scaleniem.
- Równoległe S-03 i S-08 mogą zderzyć się z tym slice'em tylko w wierszach statusu w `roadmap.md`.

## Success Criteria (Summary)

- Użytkownik widzi każdy dzień miesiąca z sumą, paskiem i odchyleniem, a wiersze sumują się do kafelków.
- Dzień wolny oznaczony z tabeli od razu zmienia normę, kafelki i kreskowanie, bez odskoku przełącznika.
- Wszystkie kontrole z `AGENTS.md` są zielone, a reguła normy dnia ma testy w Rust.
