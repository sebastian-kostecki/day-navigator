Konsola to ciemny, gęsty interfejs Day Navigatora — osobistej aplikacji Windows, która pokazuje, co dziś zrobić i ile brakuje do normy czasu pracy. Jeden użytkownik, programista: ekran ma dawać odpowiedź na pierwszy rzut oka, bez dekoracji.

## Zasady treści

- Cały interfejs po polsku. Zdania krótkie, w drugiej osobie bez zaimka: „Zaloguj się w terminalu”, „Wybierz z listy obok”.
- Nazwy widoków zawsze te same: „Do zrobienia”, „Praca (GitLab)”, „Projekty (GitHub)”, „Moje zadania”, „Czas pracy”, „Stan źródeł”. W zakładkach skróty: GitLab, GitHub, Źródła.
- Czas zapisuj jako `G:MM` w `mono` (`7:35`, `+2:30`, `−1:10` ze znakiem minus U+2212). W zdaniach słownie: „2 h 30 min”.
- Daty krótko: `23.11`, `wczoraj 22:10`, `4 min temu`.
- Flagi i pigułki stanu WERSALIKAMI w `chip`: `NOWE`, `ZMIENIONE`, `DRAFT`, `W DO ZROBIENIA`, `Z PIĄTKU`, `NIEDOSTĘPNY`, `BŁĄD`. Wersaliki ustawia CSS (`text-transform: uppercase`), więc tekst możesz zapisać „Nowe”.
- Bez emoji. Glify stanu to znaki tekstowe: `●` OK, `○` niedostępny, `▲` błąd.
- Brak VPN to stan normalny, nie awaria: „GitLab niedostępny — brak VPN”, nigdy „Błąd połączenia”.
- Puste dane mówią to wprost: „Brak otwartych zgłoszeń przypisanych do Ciebie.” Nigdy pusty panel w miejscu błędu.

## Kolor

- Ziemia `bg`, nagłówek `bg-chrome`, panele `surface` z obrysem `line`. Zaznaczony wiersz `surface-raised` plus wewnętrzna kreska 2px w `accent` po lewej.
- Tekst `ink`; etykiety i meta `ink-faint`; drugi plan `ink-muted`. `ink-faint` nigdy na `surface-active`.
- `accent` (bursztyn) oznacza tylko trzy rzeczy: aktywną zakładkę (podkreślenie 2px), jedną główną akcję na widok, pozycję już dodaną do „Do zrobienia” (wypełniony przycisk +). Na wypełnieniu `accent` tekst `on-accent`.
- Dobrze = `info` (niebieski): źródło OK, odchylenie dodatnie, dzień wolny, flaga NOWE. Pod normą = `warn` (pomarańcz) ze znakiem `−`. Nie używaj zieleni ani pary zielony/czerwony.
- `danger` tylko dla prawdziwych błędów (wylogowane CLI, uszkodzona baza) i dla `● REC`. Blok błędu: `danger-bg`, obrys `danger-line`, nagłówek `danger-ink`.
- Źródła pozycji: chip `GL` w `src-gitlab`, `GH` w `src-github`, `MY` w `src-own`; typ `MR` w `kind-mr`, `ISS` w `src-gitlab`.
- Jeden motyw: ciemny. Jasnego nie ma.

## Typografia

- Dwie rodziny: `sans` (IBM Plex Sans) dla treści, `mono` (JetBrains Mono) dla wszystkiego, co jest liczbą, numerem, godziną, kodem lub stanem. Obie dołączone lokalnie jako woff2 (licencja OFL), bez sieci: IBM Plex Sans 400/500/600, JetBrains Mono 400/500/700 — lista w tokenach (`type.fonts`).
- Cyfry zawsze tabelaryczne (`font-variant-numeric: tabular-nums`).
- Skala: `view-title` raz na widok, `panel-title` w nagłówkach paneli, `item` dla tytułów pozycji, `body` bazowo, `meta` podpisy, `label-caps` etykiety kafli i kolumn. Liczby: `clock`, `stat`, `code`, `chip`, `brand`.

## Układ i gęstość

- Strona: maks. 1360px, padding `space-5`, odstęp paneli `space-5`. Panele w rzędzie `flex-wrap`, główny `flex: 3 1 440px`, boczne węższe; na wąskim oknie spadają pod spód.
- Panel: nagłówek `space-3`/`space-4`, kreska `line`, wiersze `size-row` (52px) z dzielnikiem `line-subtle`. Tabele gęste: `size-row-dense`. Szeroka tabela w pudełku z przewijaniem poziomym.
- Pasek wskaźników (`StatStrip`): kafle rozdzielone 1px szczeliną w kolorze `line`, bez cieni.
- Promienie małe: `radius-sm` chipy, `radius-md` kontrolki, `radius-lg` panele. Bez cieni, bez gradientów. Jedyny wzór to ukośne paski dnia wolnego w wierszach `DataTable`.
- Cele kliknięcia: nawigacja i START/STOP `size-touch`, przyciski-ikony `size-control` z `aria-label`.

## Flagi

- Wiersz może mieć kilka flag, zawsze w tej kolejności: `NOWE`, `ZMIENIONE`, `DRAFT`, `W DO ZROBIENIA`. W `ItemRow` pierwsza idzie do `flag`, pozostałe do `meta`.
- Ton `info` (niebieski) znaczy „zamknięte w źródle”. `new` też jest niebieski; `change` ma kolor `flag`.
- `Chip` to znacznik priorytetu (tony `warn`, `soft`, `faint`); sygnałem zmiany pozostaje `Flag`.

## Nawigacja i licznik

- Sześć numerowanych widoków: 1 Do zrobienia, 2 GitLab, 3 GitHub, 4 Moje zadania, 5 Czas pracy, 6 Źródła. Numer stoi przed etykietą i jest ustawiony jako `aria-keyshortcuts`.
- Klawisze 1–6 przełączają widok: bez modyfikatora, bez powtarzania przy przytrzymaniu, nie podczas pisania w polu. Fokus przechodzi na wybraną zakładkę.
- Licznik czasu pracy jest zawsze widoczny w slocie `AppHeader` (`children`).
- START/STOP najpierw pyta: „Rozpocząć pracę?” / „Zakończyć pracę?” z przyciskami [TAK, START | TAK, STOP] [Anuluj]. „Anuluj” stoi w miejscu START/STOP i dostaje fokus. Pytanie wygasa po 6 s lub po Escape i ignoruje kliknięcia przez pierwsze 400 ms (ochrona przed dwuklikiem).
- Wczytywanie: „Czas pracy: wczytywanie…”. Błąd: kompaktowy `Notice` tonu `error` z „Spróbuj ponownie”.

## Komunikaty

- `SourceState` / `ErrorState` zastępują całą listę źródła. `Notice` to baner w widoku, w tonach `calm` (niedostępny, nigdy czerwony), `info`, `warn`, `error`.
- Warianty `Notice`: `compact` (pod polem, w wierszu, w nagłówku) i `inset` (wewnątrz treści `Panel`).
- `lastOk` / ostatni sukces to gotowe zdanie, np. „ostatnio OK 08:52” — komponent nic nie dopisuje.

## Stany źródła

Każdy widok źródła ma cztery stany i pokazuje czas ostatniego udanego odświeżenia:
1. **dane** — lista;
2. **pusto** — `SourceState kind="empty"`, jedno zdanie;
3. **niedostępny** (tylko GitLab bez VPN) — `SourceState kind="calm"`, `role="status"`: szary `○`, bez alarmu, „Sprawdź teraz”;
4. **błąd** — `ErrorState` (`SourceState kind="error"`), `role="alert"`, `▲` w `danger`, jak to naprawić (np. `gh auth login`), „Spróbuj ponownie”.

## Dzień wolny

- Jedyny wzór to ukośne paski na wierszach `DataTable` (`hatched`): `--hatch-bg` na tokenie `hatch-stripe`.
- `current` oznacza wiersz „teraz” (tło `surface-raised` i kreska 2px `accent` z lewej). Oba atrybuty łączą się.

## Fokus i interakcja

- Fokus: obrys 2px `accent`, odsunięty 2px — widoczny na każdej powierzchni.
- Najechanie na link: `accent-hover`. Wciśnięty segment: `surface-active` + `ink-strong` + `aria-pressed`.
- Odhaczona pozycja: tytuł `ink-done` z przekreśleniem, zostaje w miejscu.
- Bez animacji poza zmianą koloru.

## Ikony

Kreskowe SVG 24×24, obrys 2px, zaokrąglone końce, `currentColor`, rozmiar 14px w przyciskach — komponent `Icon`: `plus`, `close`, `refresh`, `edit`, `eye`, `eye-off`, `external`, `chevron-left`, `chevron-right`, `wifi-off`, `alert-triangle`, `check-circle`, `repo`, `clock`, `folder`. Aliasy: `hide` = `eye-off`, `show` = `eye`, `alert` = `alert-triangle`.

Ikona aplikacji: igła kompasu skierowana na północny wschód w pierścieniu dnia — pierścień i czubek igły `accent`, ogon `ink-faint`, tło przezroczyste. Plik `app-icon.svg` w grupie zasobów „Ikony aplikacji”. Ikona w zasobniku ma trzy stany: `tray-ok.svg` (bursztyn, wszystko OK), `tray-error.svg` (`danger` z kropką-plakietką) i `tray-idle.svg` (szarości; start lub sprawdzanie nie działa). Napis `DAY/NAV` w `brand`, kolor `accent`, zostaje w nagłówku.

## Komponenty

Przestrzeń nazw `DayNav` (18): `AppHeader`, `Button`, `Chip`, `DataTable` (+ `DataTableRow`, `DataTableCell`), `ErrorState`, `Field`, `Flag`, `HoursBar`, `Icon`, `ItemRow`, `Notice`, `Panel`, `Segmented`, `SourceChip`, `SourceState`, `StatStrip`, `StatusPill`, `Tabs`. Ekrany składaj z nich; układ i treść to zwykły markup na tokenach. `plural`, pomocniki czasu `format*` i `useNow` to pomocniki aplikacji — nie ma ich w paczce.

## Wzorce widoków

- Karta `RemindersPanel`: „Przypomnienia” w widoku „Czas pracy” — `Panel` z listą definicji, pigułką WŁ./WYŁ. i stopką „Ustawienia”.
- Karta `SourceCard`: karty w „Stanie źródeł”, np. „Dane aplikacji” z faktami i przyciskiem „Otwórz folder kopii”.
- Licznik z pytaniem START/STOP pokazuje karta `AppHeader`.
