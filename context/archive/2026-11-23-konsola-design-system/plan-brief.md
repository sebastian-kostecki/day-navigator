# Design system Konsola w aplikacji — Plan Brief

> Full plan: `context/changes/konsola-design-system/plan.md`

## What & Why

Aplikacja dostaje spójny wygląd z design systemu „Day Navigator — Konsola”: ciemny, gęsty interfejs, w którym bursztyn oznacza tylko aktywną zakładkę, główną akcję i pozycję już w „Do zrobienia”, „dobrze” jest niebieskie, a czerwień zarezerwowana dla błędów i REC. Dziś każdy widok ma własne klasy w jednym pliku CSS i nie ma wspólnych komponentów, więc wygląd się rozjeżdża, a spokojny stan „brak VPN” nie różni się wyraźnie od błędu.

## Starting Point

Jeden globalny `src/App.css` (721 linii): jasny domyślnie, ciemny tylko przez `prefers-color-scheme`. Jedyny wspólny komponent to `ErrorState`. Okno ma 800×600, a fonty nie są dołączone.

## Desired End State

Aplikacja zawsze jest ciemna, z Plex i JetBrains Mono działającymi offline, otwiera się w 1280×800 (min 800×600) bez mignięcia i pamięta rozmiar okna. Wszystkie sześć widoków składa się z 12 komponentów w `src/components/` na tokenach z `src/styles/tokens.css`, a `App.css` nie istnieje. Zachowanie, dostępne nazwy i blokady są takie jak dziś.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Motyw | Zawsze ciemny, `prefers-color-scheme` usunięte | Design system ma tylko ciemny motyw sprawdzony pod kontrast; mniej CSS do utrzymania. |
| Fonty | Pełne pliki woff2 z oficjalnych wydań w `src/assets/fonts`, z licencjami OFL, bez podzbiorów | Działa offline, bez nowej zależności ani narzędzia, poniżej 1 MB. |
| Okno | 1280×800, min 800×600, plugin `tauri-plugin-window-state`, start z `visible: false` i `show_main` po przywróceniu | Panele obok siebie jak w makietach, rozmiar zapamiętany, bez mignięcia. |
| Plugin bez npm | Tylko crate + `.plugin(...)`, bez pakietu JS i uprawnień | UI nie woła API pluginu; capability zostaje minimalna (wyjątek dopisany w AGENTS.md). |
| Zakres danych | Tylko istniejące dane | Zero zmian w Rust i logice; dziennik zdarzeń i tabela dzień po dniu wypadają. |
| Organizacja CSS | `tokens.css` + CSS przy każdym komponencie (`dn-*`), CSS widoku tylko dla układu | Zgodne z pakietem design systemu, bez nowych narzędzi. |
| ErrorState | Zostaje z tym samym API i rysuje `SourceState kind="error"`; ładowanie i pusty stan idą przez `dn-loading`, `dn-empty` i `SourceState` | Jedno miejsce błędów z AGENTS.md, zero zmian w wywołaniach. |
| Komponenty bez użycia | `Tabs` i `HoursBar` nie są portowane | Żaden widok nie używa `Tabs`, a `HoursBar` wymagałby normy dziennej, której Rust nie podaje (8 h w UI łamałoby AGENTS.md). |
| Stary CSS w przejściu | Globalne reguły `App.css` w `:where()`, a każda faza widoku usuwa swój blok | Stare selektory nie przebijają klas `dn-*`, a `App.css` kurczy się z fazy na fazę. |
| Kolejność | Fundament, powłoka, potem widoki w grupach, sprzątanie na końcu | Każda faza to działająca aplikacja do ręcznej kontroli i osobny commit. |
| Numery zakładek | Nie renderujemy 1–6 | Aplikacja nie ma skrótów klawiszowych, więc numery by kłamały. |

## Scope

**In scope:**
- Tokeny CSS, lokalne fonty, baza dokumentu i widoczny fokus
- 12 komponentów (Icon, Button, SourceChip, Flag, StatusPill, StatStrip, Panel, ItemRow, Segmented, SourceState, Field, AppHeader)
- Restyl sześciu widoków i nagłówka z licznikiem
- Okno 1280×800 / min 800×600 i plugin window-state
- Usunięcie `App.css` i nowe zasady UI w AGENTS.md

**Out of scope:**
- Jasny motyw i przełącznik motywu
- Dziennik zdarzeń, tabela godzin dzień po dniu, flaga NOWE, nowe komendy Rust
- Komponenty `Tabs` i `HoursBar` (zostają tylko w design systemie)
- Skróty klawiszowe, router, CSS Modules, Tailwind, runner testów UI
- Pakiet npm i uprawnienia dla window-state
- Zmiany treści komunikatów

## Architecture / Approach

`src/styles/tokens.css` (zmienne 1:1 z `tokens.json`, `@font-face`, baza) i `src/styles/layout.css` (kontener, kolumny paneli) są importowane w `main.tsx`. Komponenty to port pakietu `DayNav` z artefaktu na TSX z tymi samymi klasami `dn-*`, każdy z własnym CSS, z kilkoma korektami pod prawdziwe dane: `ItemRow` z jawnym `id` i `checkboxLabel`, `Flag` z tonem `info`, przekazywane `disabled`/`title`. Widoki zachowują hooki i `api.ts`; zmienia się tylko markup i klasy. Do czasu migracji stary `App.css` działa w trybie „zawsze ciemny”, a jego globalne reguły są w `:where()`, żeby niezmigrowane widoki były czytelne, a nowe komponenty nie dziedziczyły starych obrysów i przezroczystości.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Fundament | Tokeny, fonty, 11 komponentów, ErrorState na SourceState, App.css w trybie przejściowym | Stare selektory przebijające klasy `dn-*` |
| 2. Powłoka | AppHeader z licznikiem, układ strony, okno + window-state bez mignięcia | Stan okna zapisany przy ukrytym oknie |
| 3. Do zrobienia | Lista dnia, Zrobione dziś, picker „Do wyboru” | Zgubienie blokad limitu lub `aria-label` |
| 4. GitLab + GitHub | Sekcje, flagi, etykiety, Ukryte, stany źródeł | Spokojny „brak VPN” wyglądający jak błąd |
| 5. Moje zadania | Formularz, filtr, lista, edycja, notatki | ResizeObserver notatki i reset formularza |
| 6. Czas pracy + Źródła + sprzątanie | StatStrip, wpisy, pigułki stanów, usunięty App.css, AGENTS.md | Puste pola raportu (`null`, przyszły miesiąc) |

**Prerequisites:** zgoda na pobranie plików woff2 (oficjalne wydania IBM Plex i JetBrains Mono); działające `glab`/`gh` i dostęp do VPN do ręcznego sprawdzenia stanów.
**Estimated effort:** ~4–6 sesji w 6 fazach.

## Open Risks & Assumptions

- Brak runnera testów UI: regresje wyłapie tylko ręczne przejście przez stany; plan wymienia je w każdej fazie.
- Zakładamy, że restyl nie zmienia przepływów; tam, gdzie markup się zmienia (picker, edycja w miejscu, potwierdzenie usunięcia), nazwy dostępne i blokady są przenoszone 1:1.
- Plugin window-state zapisuje stan przy `RunEvent::Exit` (potwierdzone: „Zakończ” → `app.exit(0)`), ale wtedy okno jest zwykle ukryte w zasobniku. Odczyt rozmiaru z ukrytego okna nie jest sprawdzony; ręczne kryterium 2.7 to wykryje.
- Artefakt design systemu rozjedzie się w drobnych API (Flag `info`, ItemRow `id`, brak Tabs i HoursBar w aplikacji); synchronizacja po wdrożeniu.

## Success Criteria (Summary)

- Każdy widok wygląda jak w design systemie i działa dokładnie jak dziś, także przy 800×600 i bez sieci.
- „Brak VPN” jest spokojny i szary, prawdziwy błąd jest czerwony i ogłaszany czytnikowi, a czas ostatniego udanego odświeżenia jest widoczny.
- W CSS poza `tokens.css` nie ma literalnych kolorów, `App.css` nie istnieje, a AGENTS.md opisuje nowe zasady.
