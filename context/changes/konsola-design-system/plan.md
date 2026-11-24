# Design system Konsola w aplikacji — plan implementacji

## Overview

Wymieniamy warstwę wizualną Day Navigatora na design system „Day Navigator — Konsola” (artefakt https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY): tokeny jako zmienne CSS, lokalnie dołączone fonty IBM Plex Sans i JetBrains Mono, komponenty współdzielone w `src/components/` i zawsze ciemny motyw. Sześć istniejących widoków przechodzi na te komponenty. Logika domenowa, hooki `use*.ts`, `api.ts` i komendy Rust zostają bez zmian. Jedyna zmiana po stronie Rust to plugin `window-state`, który zapamiętuje rozmiar okna.

## Current State Analysis

- Cały wygląd to jeden globalny `src/App.css` (721 linii), importowany w `src/App.tsx:9`. Domyślnie jest jasny, ciemny wariant działa tylko przez `@media (prefers-color-scheme: dark)` (`src/App.css:536`). Klasy są per widok (`.gitlab-item`, `.todo-row`, `.own-task-row`, `.entry-row`, `.source`).
- Jedyny współdzielony komponent UI to `src/components/ErrorState.tsx:10-22`: `div.error-state role="alert"` z tytułem, komunikatem, opcjonalnym `lastSuccess` i przyciskiem „Spróbuj ponownie”.
- Nagłówek: `src/App.tsx:27-42` (przyciski `.nav-button` z `aria-current="page"` w `nav aria-label="Widoki"`) plus `src/worktime/WorkTimerHeader.tsx`. Licznik pokazuje „Dziś”, „Miesiąc ±”, przycisk START/STOP blokowany przy `busy` i własne błędy `role="alert"` z ponowieniem (`:13-21`, `:56-60`).
- Okno ma 800×600, bez min/max (`src-tauri/tauri.conf.json:12-16`); `.container` ma maks. 760px. Media queries szerokości nie ma.
- Fonty: `src/App.css:2` deklaruje Inter bez dołączonego pliku. Nie ma `public/` ani `src/assets/`. CSP jest wyłączone (`tauri.conf.json:18-20`). Vite haszuje pliki z `url()` w CSS importowanym z `src/` do `dist/assets`.
- Model stanów źródeł: `ok | notConfigured | unavailable{message} | error{kind,message}`, dla GitLaba i GitHuba dodatkowo `starting` (`src-tauri/src/health/mod.rs:66-82`, `src/gitlab/types.ts:36-42`, `src/github/types.ts:30-36`). `unavailable` nigdy nie jest alarmem (`health/mod.rs:71-73`). Zrzuty GitLaba i GitHuba mają `lastSuccessMs`; Health ma tylko globalne `checkedAtMs`.
- Pluginy są rejestrowane w `src-tauri/src/lib.rs:152-158`; `tauri` 2.12.7, pluginy na `2.x`. Capability `default.json` jest minimalna i opisana.
- UI nie ma runnera testów (`AGENTS.md`); bramki to lint, format:check, build i kontrole cargo.

## Desired End State

Aplikacja zawsze uruchamia się w ciemnym motywie Konsoli, z fontami Plex i JetBrains Mono także offline. Startuje w oknie 1280×800 (min 800×600), bez mignięcia, i pamięta ostatni rozmiar i pozycję. Każdy widok używa 12 komponentów z `src/components/` i tokenów z `src/styles/tokens.css`, a `src/App.css` nie istnieje. Każdy stan źródła wygląda jak w design systemie: dane, pusto, spokojny „niedostępny” (bez czerwieni), błąd (`role="alert"`), z czasem ostatniego udanego odświeżenia tam, gdzie dane go mają. Zachowanie, dostępne nazwy, blokady `busy`/limitu i przepływy potwierdzeń są takie jak dziś.

Weryfikacja: bramki automatyczne z AGENTS.md przechodzą, w CSS poza `tokens.css` nie ma literalnych kolorów, a ręczne przejście przez sześć widoków i ich stany wypada zgodnie z design systemem.

### Key Discoveries:

- `ErrorState` ma ~15 wywołań w widokach; zachowanie jego API ogranicza diff (`src/components/ErrorState.tsx:1-8`).
- Spokojne komunikaty to `p.gitlab-notice role="status"` / `p.notice` (`src/gitlab/GitlabView.tsx:293-307`); po restylu `role="status"` musi zostać.
- „Do wyboru” już istnieje jako picker z trzema grupami i własnymi stanami (`src/todo/TodoView.tsx:446-598`), łącznie z częściowymi błędami repozytoriów (`:515-524`).
- `NotePreview` mierzy przepełnienie przez ResizeObserver (`src/own-tasks/OwnTasksView.tsx:28-62`); zmiana `line-clamp`, paddingu lub line-height zmienia, kiedy pojawia się przełącznik „więcej”.
- Wpis czasu ma trzy tryby wiersza (widok / edycja / potwierdzenie usunięcia) (`src/worktime/WorkTimeView.tsx:124-181`), a „Razem” tyka co sekundę (`:212`).
- Nie ma skrótów klawiszowych ani własnego zarządzania fokusem; dostępność opiera się na natywnych `button`/`input`/`details`.

## What We're NOT Doing

- Nie dodajemy danych, których aplikacja nie ma: dziennika zdarzeń, tabeli godzin dzień po dniu, flagi NOWE, paska wskaźników poza Czasem pracy. Żadnych nowych komend Rust ani zmian logiki domenowej.
- Nie portujemy `HoursBar`: Rust nie podaje normy dziennej, a wpisanie 8 h w UI przeniosłoby regułę normy do frontendu (AGENTS.md). Nie portujemy też `Tabs`, bo żaden widok ich nie używa (filtr zadań to `Segmented`). Oba komponenty zostają w design systemie na przyszłość.
- Nie robimy jasnego motywu ani przełącznika motywu; `prefers-color-scheme` znika.
- Nie dodajemy skrótów klawiszowych 1–6 widocznych w makietach, więc numery zakładek nie są renderowane.
- Nie wprowadzamy routera, biblioteki stanu, CSS Modules, Tailwinda ani runnera testów UI.
- Nie dodajemy pakietu npm `@tauri-apps/plugin-window-state` ani uprawnienia `window-state:*`, bo UI nie woła API JS pluginu.
- Nie zmieniamy treści komunikatów ani podpowiedzi o `settings.json`; zmienia się tylko ich wygląd.
- Nie synchronizujemy automatycznie artefaktu design systemu; ewentualne rozbieżności API (np. tony `Flag`) są notowane w Migration Notes.

## Implementation Approach

Najpierw fundament: tokeny, fonty, komponenty i `ErrorState` na `SourceState`. Potem powłoka (nagłówek, układ, okno). Dalej widoki w czterech grupach i na końcu sprzątanie. Między fazami stary `App.css` żyje obok nowych stylów, więc niezmigrowane widoki wyglądają inaczej, ale muszą być czytelne na ciemnym tle (patrz Critical Implementation Details). Komponenty to port pakietu `DayNav` z artefaktu na React 19 + TS: te same nazwy klas `dn-*`, te same props z `index.d.ts`, plus korekty wymuszone przez prawdziwe dane, wymienione w fazie 1. Style komponentów leżą obok komponentów, a style układu widoku w folderze funkcji.

## Critical Implementation Details

**Czytelność między fazami.** Zaraz po wprowadzeniu `tokens.css` (ciemne `body`) stare jasne reguły w `App.css` dałyby jasny tekst na białych kartach. W fazie 1 trzeba usunąć z `App.css` jasne wartości domyślne (`:root` z kolorami i fontem) i zrobić dotychczasowy blok `@media (prefers-color-scheme: dark)` bezwarunkowym, żeby niezmigrowane widoki używały swoich starych ciemnych kolorów do czasu migracji.

**Specyficzność starego CSS.** Globalne reguły `App.css` biją pojedynczą klasę `dn-*`: `button:hover:not(:disabled)` (0,2,1, niebieski obrys, `App.css:248`), `input:disabled, select:disabled, button:disabled` (0,1,1, opacity 0.6, `:252-254`), a także reguły wierszy na checkboksach i przyciskach (`.todo-row > input[type="checkbox"]`, `.own-task-row > button` i podobne). W fazie 1 globalne reguły elementów (`input/select/button`, ich `:hover`/`:disabled`, `fieldset`, `h1`, `h2`) trzeba owinąć w `:where()`, żeby miały specyficzność 0. Reguły per widok znikają razem z blokiem widoku w fazie, która go migruje.

**Plugin window-state i zasobnik.** Zamknięcie okna ukrywa je do zasobnika (`lib.rs:196-203`), a wyjście idzie przez „Zakończ” → `app.exit(0)` → `RunEvent::Exit` (`runtime.rs:170`, `lib.rs:237-246`), gdzie plugin zapisuje stan. Flagi stanu są bez `VISIBLE`: przy wyjściu okno jest zwykle ukryte, więc zapisana widoczność uruchamiałaby aplikację schowaną. Dlatego okno startuje z `"visible": false`, a `setup` pokazuje je przez `runtime::show_main` po przywróceniu stanu, żeby nie było mignięcia w domyślnym rozmiarze. `single-instance` zostaje pierwszym pluginem.

## Faza 1: Fundament — tokeny, fonty, komponenty

### Overview

Wprowadza tokeny i fonty, port 11 komponentów i `ErrorState` oparty na `SourceState`. Nie zmienia jeszcze wyglądu widoków poza tym, że wszystko staje się ciemne.

### Changes Required:

#### 1. Tokeny i baza

**File**: `src/styles/tokens.css` (nowy), `src/main.tsx`

**Intent**: Jedno źródło wartości: zmienne CSS przepisane 1:1 z `tokens.json` design systemu (kolory, `--font-sans`, `--font-mono`, odstępy, promienie, rozmiary), `@font-face` dla lokalnych fontów i baza dokumentu (tło `--bg`, tekst `--ink`, `body` 13px/1.45, `tabular-nums` dla `mono`, fokus 2px `--accent` z odsunięciem 2px, `color-scheme: dark`).

**Contract**: nazwy zmiennych identyczne z tokenami design systemu (`--bg`, `--surface`, `--ink-faint`, `--accent`, `--info`, `--danger`, `--space-3`, `--radius-lg`, `--size-row`…), aliasy rozwinięte do wartości (`--src-gitlab` = `--warn`). Klasy stylów tekstu z design systemu (`.view-title`, `.panel-title`, `.item`, `.body`, `.meta`, `.label-caps`, `.clock`, `.stat`, `.code`, `.chip`, `.brand`). Pomocnicze klasy `dn-loading` (tekst ładowania w `--ink-faint`) i `dn-empty` (jednozdaniowy pusty stan w panelu). Import w `main.tsx` przed `App`.

#### 2. Fonty

**File**: `src/assets/fonts/` (nowy): `IBMPlexSans-{400,500,600}.woff2`, `JetBrainsMono-{400,500,700}.woff2`, `OFL-IBMPlexSans.txt`, `OFL-JetBrainsMono.txt`

**Intent**: Fonty działają offline i przy wyłączonej sieci. Bierzemy tylko używane grubości, jako pełne pliki woff2 z oficjalnych wydań (IBM/plex, JetBrains/JetBrainsMono), bez podzbiorów (podzbiory wymagałyby nowego narzędzia), razem z licencją OFL.

**Contract**: `@font-face` w `tokens.css` z `font-display: swap` i względnym `url()` (Vite haszuje pliki do `dist/assets`). Łącznie poniżej 1 MB. Plik musi zawierać polskie znaki (sprawdzenie ręczne 1.5). Pobranie plików wymaga zgody użytkownika w trakcie implementacji.

#### 3. Komponenty współdzielone

**File**: `src/components/<Name>/<Name>.tsx` + `<Name>.css` dla: `Icon`, `Button`, `SourceChip`, `Flag`, `StatusPill`, `StatStrip`, `Panel`, `ItemRow`, `Segmented`, `SourceState`, `Field`; eksport zbiorczy `src/components/index.ts`

**Intent**: Port pakietu `DayNav` na TSX z tymi samymi klasami `dn-*` i props z `index.d.ts` design systemu, żeby wytyczne z artefaktu dało się stosować wprost. Każdy komponent importuje swój CSS. `AppHeader` powstaje w fazie 2.

**Contract** — odstępstwa od pakietu wymuszone przez prawdziwe dane i dostępność:
- `Button`: przekazuje `disabled`, `title` i `aria-*` (blokady `busy`/limitu i podpowiedź „Limit N otwartych pozycji” muszą działać); `type="button"` domyślnie, `type="submit"` przekazywalny dla formularzy.
- `ItemRow`: `id` wymagane, gdy jest checkbox (bez losowego id przy każdym renderze). Przyjmuje `checkboxLabel` (dzisiejsze dynamiczne `aria-label` checkboxa), `title` jako `ReactNode` (tytuł bywa przyciskiem-linkiem otwierającym URL) i `checkboxDisabled`.
- `Flag`: tony `change` (bursztyn: Zmienione, Z <dnia>, Draft) i `info` (niebieski: zamknięte w źródle). Ton `new` z design systemu nie jest potrzebny, bo nie ma danych „nowe”.
- `SourceState`: rodzaje `empty | calm | error`; `calm` ma `role="status"`, `error` ma `role="alert"`; `action`/`onAction` jako przycisk drugorzędny; `lastOk` jako gotowy tekst.
- `Panel`: `meta` i `footer` jako `ReactNode`; poziom nagłówka `h2` domyślnie.
- `Field`: obsługuje `textarea` (notatka) oraz `type="date"` i `datetime-local`; etykieta zawsze widoczna i powiązana przez `htmlFor`.
- `Icon`: zestaw ikon z design systemu jako inline SVG w TSX, bez pluginu do importu SVG.

#### 4. ErrorState na SourceState

**File**: `src/components/ErrorState.tsx`

**Intent**: Zostaje jedynym miejscem renderowania błędów (kontrakt z AGENTS.md), ale rysuje `SourceState kind="error"`.

**Contract**: props bez zmian (`title`, `message`, `onRetry?`, `lastSuccess?`); `onRetry` daje `Button variant="secondary"` „Spróbuj ponownie”; `role="alert"` zostaje. Żadne wywołanie w widokach się nie zmienia. `ErrorState` obsługuje tylko błędy; ładowanie to `dn-loading`, pusty stan to `dn-empty` albo `SourceState kind="empty"`, a spokojny „niedostępny” to `SourceState kind="calm"`. To świadome doprecyzowanie notatki z `change.md`, zgodne z AGENTS.md („error via ErrorState”).

#### 5. App.css — tryb przejściowy

**File**: `src/App.css`

**Intent**: Stare style zostają tylko do czasu migracji widoków i muszą być czytelne na ciemnym tle.

**Contract**: usunięte jasne `:root` (kolor, tło, font); zawartość bloku `prefers-color-scheme: dark` staje się bezwarunkowa, a jasne odpowiedniki tych reguł są usunięte. Globalne reguły elementów (`input, select, button`, `button:hover:not(:disabled)`, `:disabled`, `fieldset`, `h1`, `h2`) są owinięte w `:where()` i mają specyficzność 0 (patrz Critical Implementation Details). Każda kolejna faza widoku usuwa z `App.css` blok swojego widoku.

### Success Criteria:

#### Automated Verification:

- Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- `dist/assets` zawiera sześć plików `.woff2` po `npm run build`
- W `src/` nie ma odwołań do Google Fonts: `git grep --untracked -n "fonts.googleapis" -- src index.html` nic nie zwraca
- `src/App.css` nie zawiera `prefers-color-scheme`

#### Manual Verification:

- `npm run tauri dev` z wyłączoną siecią: tekst w Plex, liczby w JetBrains Mono, polskie znaki (ąęłńóśźż) w tym samym foncie (sprawdzone w DevTools → Computed → Rendered fonts)
- Wszystkie sześć widoków jest czytelnych na ciemnym tle, bez jasnych kart z jasnym tekstem; przyciski `dn-btn` nie dostają niebieskiego obrysu po najechaniu
- Błąd (np. „Stan źródeł” przy zatrzymanym sprawdzaniu lub GitHub wylogowany) renderuje się jako czerwony blok `SourceState` z „Spróbuj ponownie” i czasem ostatniego sukcesu

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie przed kolejną fazą.

---

## Faza 2: Powłoka aplikacji — nagłówek, układ, okno

### Overview

Nowy nagłówek z nawigacją i licznikiem, wspólny układ strony, większe okno z zapamiętywaniem rozmiaru.

### Changes Required:

#### 1. AppHeader

**File**: `src/components/AppHeader/AppHeader.tsx` + `.css`, `src/App.tsx`

**Intent**: Znak `DAY/NAV`, sześć zakładek i miejsce na licznik po prawej, zawijane do 800px. Zakładki zostają przyciskami (bez routera) z `aria-current="page"` w `nav aria-label="Widoki"`.

**Contract**: `AppHeader` przyjmuje `items: {id, label, active}[]`, `onSelect(id)` i `children` (slot licznika). Etykiety zakładek jak dziś (`VIEWS` w `App.tsx`). Numerów 1–6 nie renderujemy. Nagłówek przyklejony u góry jak dziś.

#### 2. Licznik w stylu Konsoli

**File**: `src/worktime/WorkTimerHeader.tsx` (+ `WorkTimerHeader.css`)

**Intent**: Te same dane i zachowanie, nowy wygląd: suma dnia w `clock`, „● REC” w `--danger` tylko gdy licznik biegnie, odchylenie miesiąca w `mono` (`--info` nad normą, `--warn` ze znakiem `−` pod), START/STOP jako `Button size="lg"` (primary przy START, secondary przy STOP), błędy inline zostają w nagłówku z `role="alert"` i ponowieniem.

**Contract**: bez zmian w `useWorkTimer.ts`; `disabled={busy}`, `title` z tooltipem miesiąca i oba komunikaty błędu zachowane.

#### 3. Układ strony

**File**: `src/styles/layout.css` (nowy, import w `main.tsx`), widoki nadal używają `main.container`

**Intent**: `.container` dostaje maks. 1360px, padding `--space-5` i odstępy `--space-5`. Klasy układu `dn-columns` (rząd paneli `flex-wrap`, główny `flex: 3 1 440px`, boczny `flex: 1 1 280px`) i `dn-view-head` (tytuł widoku + meta + akcje w jednym zawijanym rzędzie).

**Contract**: `h1` widoku w stylu `view-title`. Poniżej ~800px panele układają się w jednej kolumnie.

#### 4. Okno i window-state

**File**: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`

**Intent**: Domyślnie 1280×800, minimum 800×600; aplikacja pamięta rozmiar i pozycję między uruchomieniami.

**Contract**: `width: 1280, height: 800, minWidth: 800, minHeight: 600, visible: false` w oknie `main`. Crate `tauri-plugin-window-state = "2"` i `.plugin(tauri_plugin_window_state::Builder::default().with_state_flags(StateFlags::all() & !StateFlags::VISIBLE).build())` po `single-instance`. Na końcu `setup` wywołanie `runtime::show_main(app.handle())` (`runtime.rs:124`), żeby okno pokazało się dopiero po przywróceniu stanu. Bez pakietu npm i bez wpisu w `capabilities/default.json`, bo UI nie woła API JS. Plik stanu pluginu trafia do katalogu konfiguracji aplikacji.

### Success Criteria:

#### Automated Verification:

- Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- Bramki Rust przechodzą w `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- `cargo audit` w `src-tauri/` nie zgłasza nowych ostrzeżeń po dodaniu crate'a

#### Manual Verification:

- Nagłówek przy 1280px mieści się w jednym rzędzie; przy 800px zakładki zawijają się pod znak, a licznik zostaje cały
- START/STOP przełącza się, „● REC” pojawia się tylko przy biegnącym liczniku, przycisk jest zablokowany w trakcie akcji
- Zakładka aktywna ma podkreślenie bursztynem i `aria-current="page"`; nawigacja klawiaturą (Tab/Enter) działa z widocznym fokusem
- Po zmianie rozmiaru okna i wyjściu z aplikacji (także przez zasobnik) kolejne uruchomienie otwiera okno w tym samym rozmiarze i miejscu
- Pierwsze uruchomienie bez zapisanego stanu otwiera okno 1280×800; zmniejszenie poniżej 800×600 nie jest możliwe
- Przy starcie z zapisanym stanem okno pojawia się od razu w zapamiętanym rozmiarze, bez mignięcia w 1280×800; drugie uruchomienie (single-instance) pokazuje istniejące okno

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie przed kolejną fazą.

---

## Faza 3: Do zrobienia

### Overview

Widok „Do zrobienia” na komponentach: lista dnia, „Zrobione dziś” i picker „Do wyboru”.

### Changes Required:

#### 1. Lista dnia i zrobione

**File**: `src/todo/TodoView.tsx` (+ `TodoView.css` tylko dla układu)

**Intent**: Pozycje jako `ItemRow` w `Panel` „Do zrobienia” z meta `[otwarte/limit]` (dzisiejszy licznik `role="status"` zostaje jako tekst meta w `role="status"`). Chip źródła: GitLab MR/zgłoszenie → `SourceChip mr|iss`, GitHub → `gh`, własne → `my`. Przestrzeń własnego zadania (Praca/Prywatne) jako `meta`. „od DD.MM” → `Flag change`, zamknięte w źródle → `Flag info`. „Zdejmij” to `Button quiet` z dzisiejszym `aria-label`. „Zrobione dziś” to osobny `Panel`.

**Contract**: dynamiczne `aria-label` checkboxów i przycisków („Dodaj:/Zdejmij:”), blokady `busy || atLimit` i `title` limitu bez zmian. `limitError` i `notice` zostają `role="alert"`; nieaktualny `ErrorState` (`:260`) zostaje.

#### 2. Picker „Do wyboru”

**File**: `src/todo/TodoView.tsx`

**Intent**: Panel boczny „Do wyboru” w `dn-columns`, z trzema grupami (GitLab, GitHub, Moje zadania) jako nagłówki `label-caps` z liczbą. Wiersze `ItemRow` bez checkboxa z `Button quiet iconOnly icon="plus"` i dzisiejszym `aria-label`. Stany grup: brak konfiguracji i „brak VPN” jako `SourceState calm`, błąd przez `ErrorState`, częściowe błędy repozytoriów jako `role="alert"` w `--danger`, warianty pustych jako `dn-empty`.

**Contract**: wszystkie dzisiejsze warianty statusów grup zachowane (nieskonfigurowane, niedostępne, startuje, błąd, częściowy błąd, odfiltrowane N).

#### 3. Blok widoku w App.css

**File**: `src/App.css`

**Intent**: Sekcja `/* To do view */` znika razem z migracją widoku.

**Contract**: usunięte reguły `.todo-*`; wspólne `.badge`, `.link-button` i `.empty` zostają, dopóki używają ich niezmigrowane widoki.

### Success Criteria:

#### Automated Verification:

- Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- Ani widok, ani `App.css` nie zawierają starych klas widoku: `git grep --untracked -nE "todo-row|todo-meta|todo-closed|todo-picker" -- src` nic nie zwraca

#### Manual Verification:

- Dodawanie z „Do wyboru”, odhaczanie i zdejmowanie działa; przy osiągniętym limicie przyciski „+” są zablokowane z podpowiedzią limitu
- Pozycja przeniesiona z poprzedniego dnia ma flagę „od DD.MM”, a zamknięta w źródle ma niebieską flagę i jest odhaczona
- GitLab bez VPN w pickerze pokazuje spokojny szary stan bez czerwieni; GitHub z błędem pokazuje czerwony blok
- Przy 800px panel „Do wyboru” schodzi pod listę

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie przed kolejną fazą.

---

## Faza 4: Praca (GitLab) i Projekty (GitHub)

### Overview

Oba widoki źródeł na wspólnym wzorcu: nagłówek widoku z odświeżaniem, stany źródła, sekcje jako panele, wiersze z flagami.

### Changes Required:

#### 1. GitLab

**File**: `src/gitlab/GitlabView.tsx` (+ `GitlabView.css`)

**Intent**: Sekcje (`<h2>` z liczbą) jako `Panel` z meta liczby; wiersze jako `ItemRow` bez checkboxa: `SourceChip mr|iss`, numer, tytuł-link, „Draft” i „Zmienione: …” jako `Flag change`, etykiety jako neutralne chipy (klasa widoku na tokenach chipa), czas aktualizacji jako meta, akcje OK/Ukryj/Pokaż jako `Button quiet`. „Ukryte (n)” zostaje natywnym `<details>` ostylowanym jako panel. Starting → `dn-loading`, notConfigured → `SourceState calm` z obecną treścią i `<code>`, unavailable → `SourceState calm` z `lastOk`, error/refreshError → `ErrorState`. Stopka „Odśwież” → przycisk w `dn-view-head` obok czasu ostatniego sukcesu. Nieaktualna lista dalej przyciemniona (klasa widoku, opacity).

**Contract**: akcje tylko dla zgłoszeń i `disabled` per wiersz przy `busy` jak dziś; `FilteredCount` jako `meta`; `role="status"` na spokojnych komunikatach, `role="alert"` na `actionError`.

#### 2. GitHub

**File**: `src/github/GithubView.tsx` (+ `GithubView.css` tylko jeśli potrzebny)

**Intent**: Ten sam szkielet stanów co GitLab. Sekcja repozytorium jako `Panel` z nazwą w `mono` i liczbą; wiersze `ItemRow` z `SourceChip gh`, `#numer`, tytułem-linkiem i czasem; błąd repozytorium `role="alert"` w panelu; pusta sekcja jako `dn-empty`.

**Contract**: bez akcji i flag (dane ich nie mają).

#### 3. Blok widoku w App.css

**File**: `src/App.css`

**Intent**: Sekcja `/* GitLab view */` (używana też przez GitHub) znika razem z migracją obu widoków.

**Contract**: usunięte reguły `.gitlab-*`, `.badge-changed` i ich ciemne odpowiedniki; `.badge` i `.link-button` zostają, jeśli używa ich jeszcze Moje zadania.

### Success Criteria:

#### Automated Verification:

- Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- Ani widoki, ani `App.css` nie zawierają starych klas: `git grep --untracked -nE "gitlab-item|gitlab-notice|badge-changed|gitlab-section" -- src` nic nie zwraca

#### Manual Verification:

- GitLab z VPN: sekcje, flagi „Zmienione”, etykiety, ukrywanie i przywracanie przez „Ukryte” działa
- GitLab bez VPN: spokojny szary blok z czasem ostatniego udanego odświeżenia, bez czerwieni; nieaktualna lista przyciemniona
- GitHub: wylogowane `gh` daje czerwony blok z instrukcją; repozytorium bez zgłoszeń ma jednozdaniowy pusty stan
- „Odśwież” jest zablokowany w trakcie odświeżania i pokazuje „Odświeżam…”

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie przed kolejną fazą.

---

## Faza 5: Moje zadania

### Overview

Formularz, filtr, lista, edycja w miejscu i notatki na komponentach.

### Changes Required:

#### 1. Formularz

**File**: `src/own-tasks/OwnTaskForm.tsx` (+ `OwnTasks.css`)

**Intent**: Pola jako `Field` (tytuł z `grow`, notatka jako `textarea`, termin `date`, priorytet `select`). Przestrzeń jako `Segmented` Praca/Prywatne zamiast selecta (`OwnTaskForm.tsx:88-95`), wywołujący to samo `setSpace` z wartościami `"work" | "private"`. „Dodaj” jako `Button primary`. Licznik znaków zostaje jako `meta` pod notatką.

**Contract**: wartość domyślna przestrzeni dalej pochodzi z jednorazowego `ownTaskDefaultSpace()` w `OwnTasksView` (`OwnTasksView.tsx:71-90`, przy błędzie „private”), a `resetOnSuccess` nadal nie resetuje przestrzeni (`OwnTaskForm.tsx:54-59`). `fieldset disabled={pending}`, kontrolowane pola, reset tylko po sukcesie, brak `maxLength`, błąd `role="alert"` bez zmian. Ten sam komponent formularza służy do edycji w miejscu.

#### 3. Blok widoku w App.css

**File**: `src/App.css`

**Intent**: Sekcja `/* Own tasks view */` i jej ciemne odpowiedniki znikają; wspólne `.badge`, `.link-button`, `.toggle` i `.task-error` zostają, jeśli używa ich jeszcze Czas pracy lub Stan źródeł.

**Contract**: usunięte reguły `.own-task-*`.

#### 2. Filtr i lista

**File**: `src/own-tasks/OwnTasksView.tsx`

**Intent**: Filtr przestrzeni jako `Segmented` (`aria-pressed` jak dziś); „Pokaż zrobione” zostaje checkboxem z etykietą w stylu tokenów. Wiersze jako `ItemRow` z checkboxem (zablokowanym per zadanie przy `busy`), `SourceChip my`, flagą terminu (`Flag change` „dziś”, przeterminowany w `--danger`), priorytetem w `meta` i `Button quiet iconOnly icon="edit"`. Edytowany wiersz jest zastępowany formularzem (`selected`).

**Contract**: `NotePreview` zachowuje mechanizm ResizeObserver i `aria-expanded`; line-clamp 3 linie i line-height w CSS są dobrane tak, żeby przełącznik pojawiał się tylko przy przepełnieniu. Przycisk edycji ma unikalną nazwę dostępną z tytułem zadania (naprawia uwagę F8 z `context/archive/2026-11-23-own-tasks/reviews/impl-review.md:131`).

### Success Criteria:

#### Automated Verification:

- Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- Ani widok, ani `App.css` nie zawierają starych klas: `git grep --untracked -nE "own-task-row|own-task-badge|own-task-filter" -- src` nic nie zwraca

#### Manual Verification:

- Dodanie samym tytułem działa; przy biegnącym liczniku domyślnie wybrana jest Praca, a bez niego Prywatne
- Błąd zapisu zostawia wpisane dane w formularzu i pokazuje komunikat
- Odhaczenie, filtr przestrzeni i „Pokaż zrobione” działają; edycja w miejscu zapisuje tytuł i notatkę
- Długa notatka ma przełącznik „więcej”, a krótka go nie ma
- Każdy przycisk edycji ma inną nazwę w czytniku ekranu (DevTools → Accessibility)

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie przed kolejną fazą.

---

## Faza 6: Czas pracy, Stan źródeł i sprzątanie

### Overview

Ostatnie dwa widoki, usunięcie `App.css` i aktualizacja zasad w AGENTS.md.

### Changes Required:

#### 1. Czas pracy

**File**: `src/worktime/WorkTimeView.tsx`, `src/worktime/MonthReport.tsx` (+ `WorkTime.css`)

**Intent**: `MonthReport` na górze jako `StatStrip` z pól, które już ma (`src/worktime/types.ts:61-103`), dopasowany do fazy miesiąca tak jak dzisiejsze linie tekstu (`MonthReport.tsx:27-48`):
- „Przepracowane” = `workedMs`, z podpisem „z `normMs` normy” (każda faza);
- „Odchylenie” = `deviationMs` ze znakiem i tonem `info`/`warn` (przeszły i bieżący; w przyszłym kafla nie ma);
- „Średnio / dzień” = `neededPerDayMs` (tylko bieżący; gdy `null`, wartość „—” z podpisem „brak dni roboczych”);
- „Dni robocze po dziś” = `workdaysAfterToday` (tylko bieżący; bez dzisiejszego dnia, więc etykieta tak to mówi).

Linki dni wolnych jako `Button quiet size="sm"`. Wybór dnia ◀ ▶ jako `Button quiet iconOnly` (`chevron-left/right`) wokół `Field type="date"`. Dzień wolny jako checkbox z etykietą. Wpisy jako `Panel` z wierszami w `mono` (zakres, „trwa”, czas trwania), edycją i potwierdzeniem „Usunąć wpis? Tak/Anuluj” w tym samym wierszu. „Razem” jako stopka panelu w `mono`, bez paska normy (patrz „What We're NOT Doing”). Formularz nowego wpisu na `Field datetime-local` i `Button dashed` „Dopisz”.

**Contract**: bez zmian w hookach; trzy tryby wiersza, `step=60`, `required`, blokady `busy`/`dayOffBusy`, klucz formularza po dniu, ukrywanie dnia wolnego w weekend i tykanie „Razem” zachowane.

#### 2. Stan źródeł

**File**: `src/health/HealthPanel.tsx` (+ `Health.css`)

**Intent**: Źródła jako lista wierszy w `Panel`: nazwa, `StatusPill` (`ok` → ok, `unavailable` i `notConfigured` → calm z właściwym słowem, `error` → error) i komunikat w `meta` / `--danger-soft`. „Sprawdź teraz” jako `Button` w `dn-view-head` z czasem ostatniego sprawdzenia; `notRunning` i błąd sprawdzenia przez `ErrorState`.

**Contract**: `StatusPill` dla `notConfigured` pokazuje słowo „NIESKONFIGUROWANE” (nie „NIEDOSTĘPNY”); mapowanie stanów 1:1 z `src/health/types.ts`.

#### 3. Sprzątanie i zasady

**File**: `src/App.css` (usunięty), `src/App.tsx`, `AGENTS.md`

**Intent**: Po migracji nic nie korzysta ze starych klas, więc plik znika. AGENTS.md opisuje nowe zasady, żeby kolejne zmiany ich trzymały.

**Contract**: w AGENTS.md, w „Frontend Conventions”: style w `src/styles/tokens.css` (jedyne miejsce wartości), CSS komponentu obok komponentu z prefiksem `dn-`, CSS widoku tylko dla układu, zawsze ciemny motyw, żadnych literalnych kolorów poza `tokens.css`, `ErrorState` rysuje `SourceState kind="error"`, design system jako link do artefaktu. W „Tauri 2 Only” dopisek: plugin używany wyłącznie z Rust nie dostaje pakietu npm ani uprawnień (window-state). W „Hard Rules” do listy własnego stanu w katalogu konfiguracji dopisany plik stanu okna.

### Success Criteria:

#### Automated Verification:

- Pełne bramki z AGENTS.md przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- `src/App.css` nie istnieje i nic go nie importuje
- Poza `tokens.css` nie ma literalnych kolorów w CSS: `git grep --untracked -nE "#[0-9a-fA-F]{3,8}([^0-9A-Za-z_-]|$)|rgba?\(" -- "src/**/*.css" ":!src/styles/tokens.css"` nic nie zwraca
- W `src/` nie ma `prefers-color-scheme`: `git grep --untracked -n "prefers-color-scheme" -- src` nic nie zwraca

#### Manual Verification:

- Czas pracy: pasek wskaźników zgadza się z dzisiejszymi liczbami raportu w bieżącym, przeszłym i przyszłym miesiącu (w przyszłym bez kafla odchylenia, ostatniego dnia roboczego „Średnio / dzień” pokazuje „—”); odchylenie ujemne jest pomarańczowe ze znakiem `−`
- Edycja, usuwanie z potwierdzeniem i dopisanie wpisu działają; „Razem” tyka przy biegnącym liczniku
- Stan źródeł: GitLab bez VPN ma szarą pigułkę NIEDOSTĘPNY, a nieskonfigurowane źródło ma pigułkę NIESKONFIGUROWANE; błąd jest czerwony
- Przejście przez wszystkie sześć widoków przy 1280px i 800px: brak resztek starego wyglądu, brak poziomego przewijania strony

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie; to ostatnia faza.

---

## Testing Strategy

### Unit Tests:

- Brak nowych testów jednostkowych: logika domenowa się nie zmienia, a UI nie ma runnera testów (decyzja repozytorium).
- `cargo test` pilnuje, że rejestracja pluginu nie psuje istniejących testów Rust.

### Integration Tests:

- Bez zmian; istniejące testy integracyjne Rust (fake-cli, crash harness) muszą przechodzić.

### Manual Testing Steps:

1. Uruchom `npm run tauri dev` z wyłączoną siecią i sprawdź fonty.
2. Przejdź przez sześć widoków przy 1280px, potem zmniejsz okno do 800×600.
3. GitLab: z VPN (dane, ukryte, flagi), potem bez VPN (spokojny stan, przyciemniona lista).
4. GitHub: normalnie, potem `gh auth logout` w terminalu (czerwony blok), potem z powrotem `gh auth login`.
5. Do zrobienia: dojdź do limitu, przenieś pozycję na kolejny dzień (lub użyj istniejącej), zamknij pozycję w źródle.
6. Moje zadania: dodaj, edytuj, odhacz, długa i krótka notatka, błąd zapisu.
7. Czas pracy: START/STOP, edycja i usunięcie wpisu, dzień wolny, poprzedni i przyszły miesiąc.
8. Zamknij aplikację przez zasobnik po zmianie rozmiaru okna i uruchom ponownie.
9. Nawigacja wyłącznie klawiaturą po każdym widoku, z widocznym fokusem.

## Performance Considerations

Fonty dodają poniżej 1 MB do instalatora (pełne pliki woff2, sześć grubości) i ładują się lokalnie (`font-display: swap`). Komponenty są lekkie i bezstanowe; „Razem” i licznik tykają co sekundę jak dziś, więc komponenty w tych wierszach nie mogą robić nic kosztownego przy renderze.

## Migration Notes

- Rozmiar okna: pierwsze uruchomienie po aktualizacji otworzy 1280×800; potem obowiązuje zapamiętany stan. Usunięcie pliku stanu pluginu z katalogu konfiguracji przywraca domyślny rozmiar.
- Design system: `Flag` w aplikacji ma ton `info` zamiast `new`, a `ItemRow` wymaga `id` przy checkboxie; `Tabs` i `HoursBar` nie są portowane. Po wdrożeniu warto zsynchronizować wytyczne w artefakcie (Flag, ItemRow, AppHeader bez numerów zakładek).
- Wycofanie: każda faza to osobny commit; cofnięcie fazy 6 przywraca `App.css`.

## References

- Design system: https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY (tokeny, README, 14 komponentów)
- Makiety widoków: https://claude.ai/artifact/HHDxK1jkgqtD8t4nsPUQhe (rząd „Konsola — pozostałe widoki”)
- Obecny wygląd: `src/App.css`, `src/components/ErrorState.tsx:10-22`, `src/App.tsx:27-42`
- Model stanów: `src-tauri/src/health/mod.rs:66-111`, `src/gitlab/types.ts:36-74`
- Wcześniejsze decyzje o CSS: `context/archive/2026-11-23-work-timer/plan.md:18`, `context/archive/2026-11-23-own-tasks/plan.md:287`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Fundament — tokeny, fonty, komponenty

#### Automated

- [x] 1.1 Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build` — e5a646d
- [x] 1.2 `dist/assets` zawiera sześć plików `.woff2` po `npm run build` — e5a646d
- [x] 1.3 W `src/` nie ma odwołań do Google Fonts: `git grep --untracked -n "fonts.googleapis" -- src index.html` nic nie zwraca — e5a646d
- [x] 1.4 `src/App.css` nie zawiera `prefers-color-scheme` — e5a646d

#### Manual

- [ ] 1.5 `npm run tauri dev` z wyłączoną siecią: tekst w Plex, liczby w JetBrains Mono, polskie znaki (ąęłńóśźż) w tym samym foncie (sprawdzone w DevTools → Computed → Rendered fonts)
- [ ] 1.6 Wszystkie sześć widoków jest czytelnych na ciemnym tle, bez jasnych kart z jasnym tekstem; przyciski `dn-btn` nie dostają niebieskiego obrysu po najechaniu
- [ ] 1.7 Błąd (np. „Stan źródeł” przy zatrzymanym sprawdzaniu lub GitHub wylogowany) renderuje się jako czerwony blok `SourceState` z „Spróbuj ponownie” i czasem ostatniego sukcesu

### Phase 2: Powłoka aplikacji — nagłówek, układ, okno

#### Automated

- [x] 2.1 Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build` — f932290
- [x] 2.2 Bramki Rust przechodzą w `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — f932290
- [x] 2.3 `cargo audit` w `src-tauri/` nie zgłasza nowych ostrzeżeń po dodaniu crate'a — f932290

#### Manual

- [ ] 2.4 Nagłówek przy 1280px mieści się w jednym rzędzie; przy 800px zakładki zawijają się pod znak, a licznik zostaje cały
- [ ] 2.5 START/STOP przełącza się, „● REC” pojawia się tylko przy biegnącym liczniku, przycisk jest zablokowany w trakcie akcji
- [ ] 2.6 Zakładka aktywna ma podkreślenie bursztynem i `aria-current="page"`; nawigacja klawiaturą (Tab/Enter) działa z widocznym fokusem
- [ ] 2.7 Po zmianie rozmiaru okna i wyjściu z aplikacji (także przez zasobnik) kolejne uruchomienie otwiera okno w tym samym rozmiarze i miejscu
- [ ] 2.8 Pierwsze uruchomienie bez zapisanego stanu otwiera okno 1280×800; zmniejszenie poniżej 800×600 nie jest możliwe
- [ ] 2.9 Przy starcie z zapisanym stanem okno pojawia się od razu w zapamiętanym rozmiarze, bez mignięcia w 1280×800; drugie uruchomienie (single-instance) pokazuje istniejące okno

### Phase 3: Do zrobienia

#### Automated

- [x] 3.1 Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build` — 3573a54
- [x] 3.2 Ani widok, ani `App.css` nie zawierają starych klas widoku: `git grep --untracked -nE "todo-row|todo-meta|todo-closed|todo-picker" -- src` nic nie zwraca — 3573a54

#### Manual

- [ ] 3.3 Dodawanie z „Do wyboru”, odhaczanie i zdejmowanie działa; przy osiągniętym limicie przyciski „+” są zablokowane z podpowiedzią limitu
- [ ] 3.4 Pozycja przeniesiona z poprzedniego dnia ma flagę „od DD.MM”, a zamknięta w źródle ma niebieską flagę i jest odhaczona
- [ ] 3.5 GitLab bez VPN w pickerze pokazuje spokojny szary stan bez czerwieni; GitHub z błędem pokazuje czerwony blok
- [ ] 3.6 Przy 800px panel „Do wyboru” schodzi pod listę

### Phase 4: Praca (GitLab) i Projekty (GitHub)

#### Automated

- [ ] 4.1 Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 4.2 Ani widoki, ani `App.css` nie zawierają starych klas: `git grep --untracked -nE "gitlab-item|gitlab-notice|badge-changed|gitlab-section" -- src` nic nie zwraca

#### Manual

- [ ] 4.3 GitLab z VPN: sekcje, flagi „Zmienione”, etykiety, ukrywanie i przywracanie przez „Ukryte” działa
- [ ] 4.4 GitLab bez VPN: spokojny szary blok z czasem ostatniego udanego odświeżenia, bez czerwieni; nieaktualna lista przyciemniona
- [ ] 4.5 GitHub: wylogowane `gh` daje czerwony blok z instrukcją; repozytorium bez zgłoszeń ma jednozdaniowy pusty stan
- [ ] 4.6 „Odśwież” jest zablokowany w trakcie odświeżania i pokazuje „Odświeżam…”

### Phase 5: Moje zadania

#### Automated

- [ ] 5.1 Bramki frontendu przechodzą: `npm run lint`, `npm run format:check`, `npm run build`
- [ ] 5.2 Ani widok, ani `App.css` nie zawierają starych klas: `git grep --untracked -nE "own-task-row|own-task-badge|own-task-filter" -- src` nic nie zwraca

#### Manual

- [ ] 5.3 Dodanie samym tytułem działa; przy biegnącym liczniku domyślnie wybrana jest Praca, a bez niego Prywatne
- [ ] 5.4 Błąd zapisu zostawia wpisane dane w formularzu i pokazuje komunikat
- [ ] 5.5 Odhaczenie, filtr przestrzeni i „Pokaż zrobione” działają; edycja w miejscu zapisuje tytuł i notatkę
- [ ] 5.6 Długa notatka ma przełącznik „więcej”, a krótka go nie ma
- [ ] 5.7 Każdy przycisk edycji ma inną nazwę w czytniku ekranu (DevTools → Accessibility)

### Phase 6: Czas pracy, Stan źródeł i sprzątanie

#### Automated

- [ ] 6.1 Pełne bramki z AGENTS.md przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [ ] 6.2 `src/App.css` nie istnieje i nic go nie importuje
- [ ] 6.3 Poza `tokens.css` nie ma literalnych kolorów w CSS: `git grep --untracked -nE "#[0-9a-fA-F]{3,8}([^0-9A-Za-z_-]|$)|rgba?\(" -- "src/**/*.css" ":!src/styles/tokens.css"` nic nie zwraca
- [ ] 6.4 W `src/` nie ma `prefers-color-scheme`: `git grep --untracked -n "prefers-color-scheme" -- src` nic nie zwraca

#### Manual

- [ ] 6.5 Czas pracy: pasek wskaźników zgadza się z dzisiejszymi liczbami raportu w bieżącym, przeszłym i przyszłym miesiącu (w przyszłym bez kafla odchylenia, ostatniego dnia roboczego „Średnio / dzień” pokazuje „—”); odchylenie ujemne jest pomarańczowe ze znakiem `−`
- [ ] 6.6 Edycja, usuwanie z potwierdzeniem i dopisanie wpisu działają; „Razem” tyka przy biegnącym liczniku
- [ ] 6.7 Stan źródeł: GitLab bez VPN ma szarą pigułkę NIEDOSTĘPNY, a nieskonfigurowane źródło ma pigułkę NIESKONFIGUROWANE; błąd jest czerwony
- [ ] 6.8 Przejście przez wszystkie sześć widoków przy 1280px i 800px: brak resztek starego wyglądu, brak poziomego przewijania strony
