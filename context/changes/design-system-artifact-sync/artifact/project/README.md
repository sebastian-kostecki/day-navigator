Konsola to ciemny, gęsty interfejs Day Navigatora — osobistej aplikacji Windows, która pokazuje, co dziś zrobić i ile brakuje do normy czasu pracy. Jeden użytkownik, programista: ekran ma dawać odpowiedź na pierwszy rzut oka, bez dekoracji.

## Zasady treści

- Cały interfejs po polsku. Zdania krótkie, w drugiej osobie bez zaimka: „Zaloguj się w terminalu”, „Wybierz z listy obok”.
- Nazwy widoków zawsze te same: „Do zrobienia”, „Praca (GitLab)”, „Projekty (GitHub)”, „Moje zadania”, „Czas pracy”, „Stan źródeł”. W zakładkach skróty: GitLab, GitHub, Źródła.
- Czas zapisuj jako `G:MM` w `mono` (`7:35`, `+2:30`, `−1:10` ze znakiem minus U+2212). W zdaniach słownie: „2 h 30 min”.
- Daty krótko: `23.11`, `wczoraj 22:10`, `4 min temu`.
- Flagi i pigułki stanu WERSALIKAMI w `chip`: `NOWE`, `ZMIENIONE`, `Z PIĄTKU`, `NIEDOSTĘPNY`, `BŁĄD`.
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

- Dwie rodziny: `sans` (IBM Plex Sans) dla treści, `mono` (JetBrains Mono) dla wszystkiego, co jest liczbą, numerem, godziną, kodem lub stanem. Obie z Google Fonts:
  `https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600&family=JetBrains+Mono:wght@400;500;700&display=swap`.
- Cyfry zawsze tabelaryczne (`font-variant-numeric: tabular-nums`).
- Skala: `view-title` raz na widok, `panel-title` w nagłówkach paneli, `item` dla tytułów pozycji, `body` bazowo, `meta` podpisy, `label-caps` etykiety kafli i kolumn. Liczby: `clock`, `stat`, `code`, `chip`, `brand`.

## Układ i gęstość

- Strona: maks. 1360px, padding `space-5`, odstęp paneli `space-5`. Panele w rzędzie `flex-wrap`, główny `flex: 3 1 440px`, boczne węższe; na wąskim oknie spadają pod spód.
- Panel: nagłówek `space-3`/`space-4`, kreska `line`, wiersze `size-row` (52px) z dzielnikiem `line-subtle`. Tabele gęste: `size-row-dense`. Szeroka tabela w pudełku z przewijaniem poziomym.
- Pasek wskaźników (`StatStrip`): kafle rozdzielone 1px szczeliną w kolorze `line`, bez cieni.
- Promienie małe: `radius-sm` chipy, `radius-md` kontrolki, `radius-lg` panele. Bez cieni, bez gradientów. Jedyny wzór to ukośne paski dnia wolnego w tabeli miesiąca.
- Cele kliknięcia: nawigacja i START/STOP `size-touch`, przyciski-ikony `size-control` z `aria-label`.

## Stany źródła

Każdy widok źródła ma cztery stany i pokazuje czas ostatniego udanego odświeżenia:
1. **dane** — lista;
2. **pusto** — `SourceState kind="empty"`, jedno zdanie;
3. **niedostępny** (tylko GitLab bez VPN) — `SourceState kind="calm"`: szary `○`, bez alarmu, „Sprawdź teraz”;
4. **błąd** — `SourceState kind="error"`, `role="alert"`, `▲` w `danger`, jak to naprawić (np. `gh auth login`).

## Fokus i interakcja

- Fokus: obrys 2px `accent`, odsunięty 2px — widoczny na każdej powierzchni.
- Najechanie na link: `accent-hover`. Wciśnięty segment: `surface-active` + `ink-strong` + `aria-pressed`.
- Odhaczona pozycja: tytuł `ink-done` z przekreśleniem, zostaje w miejscu.
- Bez animacji poza zmianą koloru.

## Ikony

Kreskowe SVG 24×24, obrys 2px, zaokrąglone końce, `currentColor`, rozmiar 14px w przyciskach — komponent `Icon` (plus, refresh, edit, hide, show, external, chevron-left, chevron-right, wifi-off, alert, check-circle, repo, close). Brak logo: znak to napis `DAY/NAV` w `brand`, kolor `accent`.

## Komponenty

Przestrzeń nazw `DayNav`: `AppHeader`, `Button`, `Icon`, `SourceChip`, `Flag`, `StatusPill`, `StatStrip`, `Panel`, `ItemRow`, `Segmented`, `Tabs`, `SourceState`, `Field`, `HoursBar`. Ekrany składaj z nich; układ i treść to zwykły markup na tokenach.
