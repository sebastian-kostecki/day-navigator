# Kopiowanie raportu miesiąca do schowka — Plan Brief

> Full plan: `context/changes/month-report-copy/plan.md`
> Research: `context/changes/month-report-copy/research.md`

## What & Why

W „Czas pracy” przycisk „Kopiuj miesiąc” kopiuje raport pokazanego miesiąca (podsumowanie, dni z rodzajem i nazwą święta, wpisy z notatkami, sumy, normy, odchylenia, dni wolne) do schowka jako TSV, który wkleja się w komórki Excela i Arkuszy Google. Slice S-05 (M-3), MS-24, issue #24 — użytkownik chce przenosić miesiąc do arkusza bez przepisywania.

## Starting Point

Raport miesiąca (`report::compute`, komenda `work_month_report`) niesie sumy, normy, odchylenia, rodzaj dnia i nazwę święta, ale nie wpisy ani notatki; `compute` czyta je jednak w tym samym `store.read` (notatka w `Entry` od S-06). UI nie ma żadnego kopiowania ani wtyczki schowka.

## Desired End State

Kliknięcie „Kopiuj miesiąc” (aktywne tylko przy wczytanym raporcie) kopiuje TSV i pokazuje „Skopiowano.” albo polski komunikat błędu. Każda liczba w tekście to ta sama liczba, którą pokazuje ekran; dzisiejszy wiersz odpowiada licznikowi w chwili kliknięcia. Bez pliku, nowej zależności, uprawnienia, zmiany CSP i migracji.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Format i miejsce | TSV, tylko schowek | Wkleja się prosto w komórki, bez pliku na dysku. | User |
| Kto buduje tekst | Rust, nowa komenda `work_month_report_text(day)` przeliczająca raport przy kliknięciu | Te same `compute` i `Policy::Strict` co ekran, wpisy z tego samego snapshotu, testy w Rust. | Research |
| Zapis do schowka | `navigator.clipboard.writeText` po `await` komendy; wtyczka `clipboard-manager` (tylko Rust) jako wariant awaryjny | Zero nowych crate'ów, pakietów, uprawnień i CSP. | Research |
| Układ | Trzy bloki: podsumowanie (13 stałych wierszy), dni (10 kolumn), wpisy (6 kolumn) | Prostokąt dni sumuje się do miesiąca, wpisy niosą notatki; stałe wiersze = stabilne odwołania do komórek. | Plan (agent default) |
| Liczby | `H:MM` + godziny z przecinkiem (`7,50`); odchylenie tylko `-1,50` bez `+` | pl-PL czyta je jako czas i liczby; `±H:MM` byłby formułą albo ujemnym czasem. | Research + Plan |
| Dokładność sum | Bez kolumny minut; każda komórka = wartość ekranowa | Obcinanie do minuty w wierszu i tak nie daje dokładnej sumy przy wpisach z sekundami, jak na ekranie. | Plan (agent default) |
| Godziny wpisu | `HH:MM` w dniu wiersza, inaczej `YYYY-MM-DD HH:MM`; trwający `trwa` | `DD.MM HH:MM` z ekranu Excel czyta różnie zależnie od ustawień regionalnych. | Plan (agent default) |
| Komórki tekstowe | `cell`: tab/sterujące → spacja, prefiks `'` przed `= + - @ " '` i formami pełnej szerokości | Tab dzieli komórkę, a znaki formuł byłyby wykonywane. | Research |
| UI | Przycisk tekstowy w nagłówku po `dn-spacer`; `role="status"` „Skopiowano.”; błąd `Notice` | Istniejące wzory (`GithubView`, `OwnTaskEditPanel`). | Research |

## Scope

**In scope:**
- `work_time/month_text.rs` + `month_text_tests.rs`, komenda w `commands.rs`, linie w `mod.rs` i `lib.rs`
- `api.ts` (jedna funkcja), `useMonthCopy.ts`, `MonthCopyButton.tsx`, reguła `.wt-month-copy`, dwie linie w `WorkTimeView.tsx`
- Punkty w README i CHANGELOG (przecinek dziesiętny dla pl-PL)

**Out of scope:**
- Plik/CSV/XLSX, kopiowanie z zasobnika, tygodnia lub dnia, wariant en-US
- Zmiany w `report.rs`, `norm.rs`, `types.ts`, nowe pola raportu, nowe zdarzenia, `notify`
- Wtyczka schowka (tylko zapisany wariant awaryjny), migracja, AGENTS.md, roadmapa, PRD

## Architecture / Approach

Klik → `useMonthCopy` → `invoke("work_month_report_text", { day: "YYYY-MM-01" })` → `run_with_settings(Strict)` → jeden `store.read`: `compute` + `in_day` za miesiąc → czyste formatowanie z pól raportu (match wyczerpujący po `DayKind`, bez `DAILY_NORM_MS`) → `String` przez IPC → `navigator.clipboard.writeText` → „Skopiowano.” / błąd. Nic nie jest logowane ani zapisywane w dzienniku zdarzeń.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Budowanie TSV w Rust i komenda | Builder z testami zgodności z raportem, komenda zarejestrowana | Rozjazd reguły zaokrąglenia lub pustych komórek z ekranem |
| 2. Przycisk „Kopiuj miesiąc” i dokumentacja | Przycisk, potwierdzenie/błąd, README, CHANGELOG | `writeText` odrzucony w zbudowanej aplikacji (sprawdzenie ręczne) |

**Prerequisites:** S-06 (notatka w `Entry`) i S-01 (święta) scalone w `develop` — spełnione.
**Estimated effort:** ~2 sesje sub-agenta, po jednej na fazę.

## Open Risks & Assumptions

- `navigator.clipboard.writeText` po `await` komendy działa w skupionym oknie WebView2 — do sprawdzenia w zbudowanej aplikacji; w razie porażki osobna zmiana z wtyczką `clipboard-manager` tylko z Rust.
- Nie wiadomo, czy WebView2 zamienia LF na CRLF; oba arkusze przyjmują oba warianty (sprawdzenie ręczne).
- Zachowanie wklejania w Excelu/Arkuszach (czasy, `'`-prefiks) opisane na podstawie dokumentacji i forów, niezweryfikowane na żywo.
- Merge z S-02: nowy `DayKind` = błąd kompilacji do dopisania etykiety; nowe wiersze podsumowania dopisywać na końcu.

## Success Criteria (Summary)

- Jedno kliknięcie wkleja się do pl-PL Excela i Arkuszy Google z czasami, liczbami, datami i notatkami w osobnych komórkach, bez formuł z notatek.
- Każda liczba w kopii zgadza się z ekranem (testy w Rust na wymyślonych danych: bieżący, zamknięty z DST, z sobotnim świętem i przyszły miesiąc).
- Brak nowych zależności, uprawnień i logów z treścią notatek.
