# Etykiety GitLaba jako kolorowe chipy i filtr widoku „Praca (GitLab)” — plan implementacji

## Overview

W „Praca (GitLab)” etykiety każdej pozycji pojawiają się jako chipy w kolorach z GitLaba: tło w kolorze etykiety, tekst w `text_color`, a etykiety scoped `klucz::wartość` są dzielone tak jak w GitLabie. Nad listą dochodzi filtr widoku. Użytkownik wybiera w nim projekty, etykiety i kamienie milowe z wartości obecnych na liście, a klik w chip etykiety albo w nazwę projektu w wierszu przełącza tę wartość w filtrze. Filtr widoku zawęża to, co zostało po filtrze z `settings.json`, i działa do zamknięcia aplikacji.

## Current State Analysis

- Wywołania listy (`src-tauri/src/cli/mod.rs:207-215`, `:271-278`) pytają `issues?…` / `merge_requests?…` bez `with_labels_details`, więc GitLab zwraca `labels` jako tablicę nazw. Kolorów w danych nie ma.
- `GitlabItem.labels: Vec<String>` (`src-tauri/src/gitlab/mod.rs:75`), `RawItem.labels: Vec<String>` (`:213`). Nazwy etykiet czytają dwie reguły domenowe: filtr z ustawień (`gitlab/filter.rs:110-122`) i wykrywanie zmiany statusu ukrytych przez etykiety scoped (`hidden_issues/rules.rs:80`, `:93`; zapis w `hidden_issues/mod.rs:103`).
- `GitlabItem.milestone` jest parsowany, ale `#[serde(skip)]` (`gitlab/mod.rs:79-82`), więc UI go nie zna.
- Strażnik read-only (`cli/mod.rs:609-691`) ma własne, celowo zduplikowane kopie kształtów endpointów: `ALLOWED_ENDPOINTS`, `closed_endpoint_allowed` (literał `&per_page=100`) i kształt z userem (`'&'` liczone na 3).
- `fake-cli` (`src-tauri/src/bin/fake_cli.rs:60-80`, `:210-250`) rozpoznaje wywołania po `starts_with` / `contains`, a etykiety GitLaba zwraca jako nazwy.
- UI: kolumna „Etykiety” (`src/gitlab/GitlabView.tsx:173`, `:213-220`) to tekst po przecinku, 150 px, ukrywana poniżej 900 px (`GitlabView.css`). Jedyny filtr w widoku to `Segmented` typu i „Pokaż ukryte” w `useState` widoku (`GitlabView.tsx:529-530`), więc zmiana zakładki je kasuje. `GitlabFilterBar` pokazuje filtr z `settings.json`.
- Brak wspólnego komponentu wyboru wielokrotnego / popovera w `src/components/`.

## Desired End State

- Każdy wiersz w sekcjach „Przypisane”, ukrytych i „Zamknięte” ma etykiety jako chipy w kolorach GitLaba. Scoped etykieta ma wypełniony klucz i obrysowaną wartość. Etykieta bez poprawnego koloru dostaje neutralny chip z tokenów. Chipy się zawijają, a kolumna znika poniżej 900 px jak dziś.
- W pasku narzędzi są trzy przyciski: „Projekty”, „Etykiety” i „Kamienie milowe”, każdy z liczbą wybranych. Otwierają listy wartości obecnych na liście, z licznikami i polami wyboru. Pod paskiem stoją aktywne wybory jako usuwalne chipy oraz „Wyczyść”.
- W obrębie wymiaru wybory łączy LUB, między wymiarami I. Typ (Wszystko / Zgłoszenia / MR-y) działa jak dziś, łącznie z filtrem widoku.
- Klik w chip etykiety lub w nazwę projektu w wierszu przełącza tę wartość. Wybrana wartość ma widoczny stan „wciśnięty”.
- Stopka każdej sekcji podaje osobno „Ukryte filtrem: N” (ustawienia) i „Poza filtrem widoku: N”. Gdy filtr widoku nie zostawia nic, sekcja mówi „Brak pozycji pasujących do filtra widoku.” i ma przycisk „Wyczyść filtr widoku”.
- Typ, „Pokaż ukryte” i wybory filtra przeżywają przejście na inną zakładkę i z powrotem, a po restarcie aplikacji wracają do domyślnych.
- Wywołania `glab` różnią się tylko parametrem `with_labels_details=true`, dalej tylko czytają i przechodzą test strażnika.

Weryfikacja: bramki z AGENTS.md są zielone, a ręczny przegląd na prawdziwym GitLabie (VPN) pokazuje kolory i działanie filtra.

### Key Discoveries:

- GitLab REST `GET /issues` i `GET /merge_requests` przyjmują `with_labels_details=true`: `labels` staje się tablicą obiektów `{id, name, color, description, text_color, …}`. Serwer, który nie zna parametru, go ignoruje i zwraca nazwy, dlatego parser przyjmuje obie postaci.
- Reguła scoped w `hidden_issues/rules.rs:80` porównuje zapisane nazwy etykiet scoped. Kształt etykiety w UI nie może zmienić tych nazw, bo inaczej każde ukryte zgłoszenie wróciłoby jako „zmienione” (`rules.rs:93`).
- GitLab dzieli etykietę scoped na ostatnim `::` (`a::b::c` → klucz `a::b`, wartość `c`). Reguła w `rules.rs:80` patrzy tylko na obecność `::`, więc obie zgadzają się co do tego, które etykiety są scoped.
- `tokens.css` jest jedynym miejscem na literalne kolory (AGENTS.md). Kolor z danych trafia więc do zmiennej CSS w atrybucie `style`, a CSS komponentu ma zapas z tokenów.
- `WorkTimerProvider` (`src/App.tsx:60-78`) to wzorzec kontekstu nad widokami. AGENTS.md wymaga planu dla nowego stanu globalnego i ten plan to zapisuje.

## What We're NOT Doing

- Zmiany filtra z `settings.json` (`gitlabFilter`), jego reguł w Rust i paska `GitlabFilterBar` poza zmianą podpisu na „Filtr z ustawień”.
- Opisu ustawień wewnątrz aplikacji: docs to README na GitHubie, a aplikacja tylko do niego linkuje.
- Zapisu wyborów filtra widoku na dysk (ustawienia, baza) i migracji bazy.
- Filtrowania powiadomień, „Do zrobienia”, pickera, śledzenia ukrytych i licznika w nagłówku widoku. Filtr widoku zmienia tylko to, co widać w tabelach.
- Wyszukiwania po tytule, filtrów po flagach (nowe / zmienione / draft), opcji „bez kamienia” i „bez etykiet” oraz przełącznika LUB/I.
- Dopasowania projektu po prefiksie grupy (filtr widoku dopasowuje dokładne ścieżki z listy).
- Kolorów etykiet w „Projekty (GitHub)” i w „Do zrobienia”.
- Aktualizacji artefaktu design systemu o `LabelChip` i `MultiSelect` (osobna zmiana, jak F-05).
- Kolorów etykiet w zapisie ukrytych zgłoszeń.

## Implementation Approach

Dane najpierw: Rust pobiera szczegóły etykiet, sprawdza kolory i wysyła do UI `GitlabLabel` oraz kamień milowy. Reguły, które czytają nazwy, dostają nazwy przez metodę pomocniczą. Potem prezentacja: wspólny `LabelChip` w kolumnie etykiet. Na koniec filtr widoku: czysta logika w `src/gitlab/viewFilter.ts`, stan w kontekście `GitlabListViewProvider`, wspólny `MultiSelect` w pasku narzędzi i przełączanie z wierszy.

Filtr widoku zostaje w UI, tak jak dziś przełącznik typu. To wybór prezentacji, a nie reguła domenowa: nie zmienia tego, co aplikacja śledzi, powiadamia ani odhacza. Reguły domenowe (filtr z ustawień, ukryte, „Do zrobienia”) zostają w Rust.

## Critical Implementation Details

- **Strażnik ma własne kopie kształtów.** Nowy parametr trzeba dopisać niezależnie w stałych produkcyjnych (`ASSIGNED_*_ENDPOINT`, `USER_SUFFIX`, `CLOSED_SUFFIX`) i w strażniku (`ALLOWED_ENDPOINTS` przez stałe, literał w `closed_endpoint_allowed` i liczba `'&'` w kształcie z userem: 3 → 4). Strażnik nie może zacząć ufać stałym produkcyjnym tam, gdzie dziś ich nie używa.
- **Kolor trafia do CSS tylko po walidacji w Rust.** Akceptowany jest `#` i 3 albo 6 cyfr szesnastkowych, zapisany małymi literami. Gdy `color` albo `text_color` jest niepoprawny lub go brak, oba są `None` i chip jest neutralny. Wypełnione tło z nieznanym kolorem tekstu bywa nieczytelne.

## Phase 1: Etykiety z kolorami i kamień milowy w danych

### Overview

Wywołania listy pobierają szczegóły etykiet. Parser buduje `GitlabLabel` ze sprawdzonym kolorem. Kamień milowy trafia do migawki. Lustro TS i `fake-cli` idą w tym samym commicie, a reguły czytające nazwy zachowują zachowanie.

### Changes Required:

#### 1. Parametr `with_labels_details` w wywołaniach listy

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Przypisane i zamknięte (MR-y i zgłoszenia, z userem i bez) pytają o szczegóły etykiet. Strażnik akceptuje dokładnie nowe kształty i odrzuca stare oraz każdy inny dodatkowy parametr.

**Contract**: `&with_labels_details=true` stoi bezpośrednio przed `&per_page=100` w `ASSIGNED_ISSUES_ENDPOINT`, `ASSIGNED_MRS_ENDPOINT`, `USER_SUFFIX` i `CLOSED_SUFFIX`, np. `issues?scope=assigned_to_me&state=opened&with_labels_details=true&per_page=100`. Endpoint notatek się nie zmienia. W testach strażnik ma zaktualizowane `ALLOWED_ENDPOINTS`, literał sufiksu w `closed_endpoint_allowed` oraz `'&'` == 4 w kształcie z userem. Testy dokładnych argumentów i argv procesu dostają nowe ciągi, a dochodzi przypadek: stary kształt bez parametru i kształt z `with_labels_details=false` są odrzucane.

#### 2. `GitlabLabel` i parsowanie

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Etykieta niesie nazwę i opcjonalną parę kolorów. Parser przyjmuje obiekt ze szczegółami albo samą nazwę (serwer bez obsługi parametru). Inny typ jest `ResponseInvalid` jak dziś, a błąd nie niesie treści.

**Contract**: `pub struct GitlabLabel { pub name: String, pub color: Option<String>, pub text_color: Option<String> }` z `Serialize`, `#[serde(rename_all = "camelCase")]`. `GitlabItem.labels: Vec<GitlabLabel>`. Do tego `impl GitlabItem { pub fn label_names(&self) -> impl Iterator<Item = &str> }`. `RawItem.labels` to `#[serde(untagged)]` enum `Name(String) | Detailed { name: String, #[serde(default)] color: Option<String>, #[serde(default)] text_color: Option<String> }` z `#[serde(default)]` na polu. `fn checked_color(&str) -> Option<String>`: `#` + 3 lub 6 cyfr hex, wynik małymi literami. Oba kolory zostają tylko wtedy, gdy oba przejdą. Kolejność etykiet jak przysłał GitLab.

#### 3. Kamień milowy do UI

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: UI potrzebuje tytułu kamienia milowego do filtra widoku.

**Contract**: Z `GitlabItem.milestone` znika `#[serde(skip)]`, a pole serializuje się jako `milestone: string | null`. Test `milestone_title_is_read_when_present_and_not_serialized` zmienia nazwę i sprawdza, że pole jest serializowane. Komentarz o prywatności (dane pracy: tylko serde do UI) zostaje przy polu.

#### 4. Reguły czytające nazwy

**Files**: `src-tauri/src/gitlab/filter.rs`, `src-tauri/src/hidden_issues/rules.rs`, `src-tauri/src/hidden_issues/mod.rs`, testy i miejsca budujące `GitlabItem` (`gitlab_refresh.rs`, `todo/commands.rs`, `new_assignments/tests.rs`, `hidden_issues/tests.rs`, `gitlab/filter.rs` tests)

**Intent**: Filtr z ustawień i wykrywanie zmiany statusu ukrytych działają na nazwach jak dotąd.

**Contract**: `labels_allow` iteruje `item.label_names()`. `scoped_labels` przyjmuje iterator nazw (albo `&[GitlabLabel]`) i zwraca te same `Vec<String>` co dziś dla tych samych nazw. Zapisane rekordy ukrytych się nie zmieniają, więc nie ma migracji. Test: dla tej samej listy nazw `scoped_labels` przed i po zmianie daje identyczny wynik, a ukryte zgłoszenie z kolorowymi etykietami nie wraca jako „zmienione”.

#### 5. `fake-cli`

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Wymyślone dane pokrywają obie postaci etykiet i niepoprawny kolor.

**Contract**: Przypisane pozycje GitLaba zwracają etykiety jako obiekty `{id, name, color, text_color, description}` z wymyślonymi wartościami. Jedna pozycja ma etykietę scoped (`workflow::doing`), jedna niepoprawny kolor (np. `"red"`), a jedna etykiety jako same nazwy. Rozpoznawanie wywołań działa z nowym sufiksem. Marker wycieku zostaje tam, gdzie testy go oczekują.

#### 6. Lustro TS

**File**: `src/gitlab/types.ts`

**Intent**: UI zna nowy kształt w tym samym commicie co Rust.

**Contract**: `export type GitlabLabel = { name: string; color: string | null; textColor: string | null }`, `GitlabItem.labels: GitlabLabel[]`, `GitlabItem.milestone: string | null`. `GitlabView.tsx` do czasu fazy 2 łączy `labels.map(l => l.name)`, żeby build przeszedł.

### Success Criteria:

#### Automated Verification:

- Strażnik i testy argumentów przyjmują nowe kształty z `with_labels_details=true` i odrzucają stary kształt oraz `with_labels_details=false`: `cargo test` w `src-tauri/`
- Parser: obiekt ze szczegółami daje nazwę i kolory małymi literami; sama nazwa daje kolory `null`; niepoprawny `color` lub `text_color` daje oba `null`; etykieta zła typem daje `ResponseInvalid`: `cargo test`
- `scoped_labels` daje ten sam wynik dla tych samych nazw, a ukryte zgłoszenie z kolorowymi etykietami nie wraca jako zmienione: `cargo test`
- `milestone` jest serializowany w migawce (test serializacji migawki zaktualizowany): `cargo test`
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` są czyste
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą

#### Manual Verification:

- Na prawdziwym GitLabie (VPN) lista ładuje się jak wcześniej, a w kolumnie „Etykiety” są te same nazwy co przed zmianą

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Chipy etykiet w wierszach

### Overview

Wspólny `LabelChip` pokazuje etykietę jak GitLab. Kolumna „Etykiety” w tabeli GitLaba (przypisane, ukryte, zamknięte) używa chipów zamiast tekstu.

### Changes Required:

#### 1. Komponent `LabelChip`

**Files**: `src/components/LabelChip/LabelChip.tsx`, `src/components/LabelChip/LabelChip.css`, `src/components/index.ts`

**Intent**: Jeden komponent rysuje etykietę: wypełnione tło w kolorze etykiety z jej kolorem tekstu, albo neutralnie, gdy koloru brak. Scoped `klucz::wartość` ma wypełniony klucz i wartość obrysowaną kolorem etykiety, z tekstem w kolorze aplikacji. Komponent przygotowuje też tryb przycisku na fazę 3.

**Contract**: `LabelChipProps = { name: string; color: string | null; textColor: string | null; pressed?: boolean; onToggle?: () => void; toggleLabel?: string }`. Z `onToggle` renderuje `<button type="button" aria-pressed>` z `aria-label={toggleLabel}`, bez niego `<span>`. Podział scoped na ostatnim `::`, gdy obie części są niepuste. Kolory idą przez `style={{ "--dn-label-bg": color, "--dn-label-ink": textColor }}`, a CSS używa `var(--dn-label-bg, var(--label-fallback-bg))` i `var(--dn-label-ink, var(--label-fallback-ink))`. Klasa `dn-label-chip`, stan wciśnięty to obwódka `--accent`, focus widoczny. `title={name}`, a długą nazwę ucina wielokropek.

#### 2. Tokeny zapasowe

**File**: `src/styles/tokens.css`

**Intent**: Neutralny chip bez literalnych kolorów poza tokenami.

**Contract**: `--label-fallback-bg` i `--label-fallback-ink` (np. odcienie `--surface-active` / `--ink-soft`) w bloku kolorów.

#### 3. Kolumna „Etykiety” w tabeli GitLaba

**Files**: `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`

**Intent**: Komórka pokazuje chipy zawijane w kilku liniach zamiast uciętego tekstu. Pełna lista nazw jest w podpowiedzi komórki.

**Contract**: Komórka bez `truncate`, z `display: flex; flex-wrap: wrap; gap`. Kolumna szablonu `--dn-table-cols` rośnie ze 150 px do ok. 200 px (reszta szablonu bez zmian). Zasady 900 px / 600 px bez zmian. Komórka bez etykiet jest pusta jak dziś.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- Brak literalnych kolorów poza `tokens.css`: `rg -n "#[0-9a-fA-F]{3,6}\b" src --glob "*.css" --glob "!src/styles/tokens.css"` nie zwraca trafień

#### Manual Verification:

- Na prawdziwym GitLabie chipy mają kolory jak w GitLabie, a scoped etykiety są dzielone na klucz i wartość
- Etykieta z niepoprawnym kolorem (scenariusz `fake-cli` albo podmieniona odpowiedź) ma neutralny chip i jest czytelna
- Wiersz z wieloma etykietami zawija chipy bez poziomego przewijania strony; poniżej 900 px kolumna znika
- Sekcja „Zamknięte” i ukryte wiersze też mają chipy

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Filtr widoku

### Overview

Stan listy (typ, „Pokaż ukryte”, wybory filtra) przenosi się do kontekstu nad widokami. Pasek narzędzi dostaje trzy listy wyboru wielokrotnego. Filtr działa na wszystkie sekcje z licznikami i pustym stanem z wyjściem, a chipy i projekty w wierszach przełączają wybory.

### Changes Required:

#### 1. Czysta logika filtra widoku

**File**: `src/gitlab/viewFilter.ts`

**Intent**: Jedno miejsce na dopasowanie i listy wartości, bez Reacta.

**Contract**: `type ViewSelection = { projects: string[]; labels: string[]; milestones: string[] }`, `EMPTY_SELECTION`, `isSelectionActive(sel)`. `splitReference` przenosi się tu z `GitlabView.tsx`. `matchesView(item, sel)`: pusty wymiar to brak ograniczenia; w wymiarze LUB, między wymiarami I; projekt to dokładna ścieżka z `reference`; etykiety to dokładne nazwy; pozycja bez kamienia nie przechodzi niepustego wyboru kamieni. `viewOptions(items, sel)` zwraca dla każdego wymiaru `{ value, count, color?, textColor? }[]`. Liczy po pozycjach unikalnych po `kind:reference` ze wszystkich sekcji (MR-y, zgłoszenia, ukryte, zamknięte). Sortuje `localeCompare(…, "pl")`. Wartość wybrana, której nie ma na liście, występuje z `count: 0`.

#### 2. Kontekst stanu listy

**Files**: `src/gitlab/GitlabListViewProvider.tsx`, `src/App.tsx`

**Intent**: Typ, „Pokaż ukryte” i wybory przeżywają zmianę zakładki i znikają po restarcie.

**Contract**: `GitlabListViewProvider` (React context, bez biblioteki) i `useGitlabListView()` zwracające `{ kind, setKind, showHidden, setShowHidden, selection, toggle(dimension, value), clear() }`. Provider stoi w `App.tsx` wewnątrz `WorkTimerProvider` wokół widoków. `GitlabView` czyta z niego zamiast z własnego `useState`, a typ `ListView` w widoku znika.

#### 3. Komponent `MultiSelect`

**Files**: `src/components/MultiSelect/MultiSelect.tsx`, `src/components/MultiSelect/MultiSelect.css`, `src/components/index.ts`

**Intent**: Przycisk z liczbą wybranych otwiera listę pól wyboru z licznikami. To wspólny komponent, bo `Segmented` nie skaluje się do wielu wartości.

**Contract**: `MultiSelectProps = { label: string; options: { value: string; count: number; adornment?: ReactNode }[]; selected: readonly string[]; onToggle: (value: string) => void; emptyText?: string }`. Przycisk ma `aria-expanded` i `aria-haspopup`, tekst `label` albo `label (n)`. Panel to lista `<label><input type="checkbox">…</label>` z licznikiem w `.meta` i ma `max-height` z przewijaniem. Escape i klik poza panelem go zamykają, a focus wraca na przycisk. Bez opcji przycisk jest wyłączony. Style z tokenów, prefiks `dn-`.

#### 4. Pasek narzędzi i aktywne wybory

**Files**: `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`, `src/gitlab/GitlabFilterBar.tsx`

**Intent**: Wybór w pasku i wyraźny zapis tego, co zawęża listę.

**Contract**: W `.gitlab-toolbar` przed `Segmented` stoją trzy `MultiSelect`: „Projekty”, „Etykiety” (opcje z `adornment` = mały `LabelChip` albo kropka koloru) i „Kamienie milowe”. Gdy `isSelectionActive`, pod paskiem jest wiersz „Filtr widoku:” z usuwalnymi chipami (`aria-label="Usuń z filtra widoku: <wartość>"`) i przyciskiem „Wyczyść”. `GitlabFilterBar` zmienia podpis z „Filtr” na „Filtr z ustawień”.

#### 5. Filtrowanie sekcji, liczniki, pusty stan

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: Filtr widoku działa łącznie z typem na przypisane, ukryte (gdy pokazane) i zamknięte. Liczba pominiętych jest widoczna, a pusta tabela nigdy nie udaje braku przypisań.

**Contract**: `Lists` i `ClosedBlock` filtrują `matchesKind && matchesView`. Stopka sekcji łączy `Ukryte filtrem: N` (ustawienia, jak dziś) i `Poza filtrem widoku: M` (liczone po typie, przed filtrem widoku), pomijając zerowe części. Gdy wiersze są puste przy aktywnym filtrze widoku i przed nim coś było, sekcja pokazuje „Brak pozycji pasujących do filtra widoku.” i przycisk „Wyczyść filtr widoku”. W pozostałych przypadkach `emptyText` działa jak dziś. Licznik w nagłówku widoku się nie zmienia.

#### 6. Przełączanie z wierszy

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: Klik w etykietę lub projekt w wierszu dodaje albo usuwa tę wartość.

**Contract**: Chipy w kolumnie etykiet dostają `onToggle={() => toggle("labels", name)}`, `pressed` i `toggleLabel="Filtruj po etykiecie: <nazwa>"`. Komórka projektu staje się `button.link-button` z `aria-pressed`, `title` = pełna ścieżka i `aria-label="Filtruj po projekcie: <ścieżka>"`, przełączając `toggle("projects", path)`. Tytuł dalej otwiera pozycję w GitLabie.

#### 7. Link do dokumentacji ustawień

**Files**: `src/gitlab/GitlabFilterBar.tsx`, `src/gitlab/api.ts`, `README.md`

**Intent**: Kto chce zmienić filtr z `settings.json`, znajduje opis ustawień jednym klikiem. Docs ustawień zostają w README na GitHubie (sekcja „Configuration”), a aplikacja tylko do nich linkuje. Nie powstaje drugi opis ustawień w aplikacji.

**Contract**: Stała `SETTINGS_DOCS_URL = "https://github.com/sebastian-kostecki/day-navigator#configuration"` w `src/gitlab/api.ts` i `openSettingsDocs()` przez `openUrl` z `@tauri-apps/plugin-opener` (jak `openGitlabItem`), bez adresu z danych. W `GitlabFilterBar` obok przycisku otwierającego `settings.json` stoi przycisk „Dokumentacja ustawień”, a błąd otwarcia pokazuje się tak jak błąd otwarcia pliku. Sprawdzić, czy `capabilities/default.json` pozwala pluginowi opener otworzyć ten adres; dodać uprawnienie tylko, jeśli go brakuje. README zostaje źródłem opisu: sekcja „Configuration” dostaje zdanie, że filtr widoku nie jest w `settings.json` i działa do zamknięcia aplikacji.

#### 8. Konwencja w AGENTS.md

**File**: `AGENTS.md`

**Intent**: Zapis, że stan listy GitLaba żyje w kontekście, tak jak stan timera.

**Contract**: Jedno zdanie w „Frontend Conventions” przy zdaniu o `WorkTimerProvider`: stan listy „Praca (GitLab)” (typ, ukryte, filtr widoku) to `GitlabListViewProvider` (`src/gitlab/`), czytany przez `useGitlabListView()`, tylko w pamięci.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- Pełny zestaw bramek z AGENTS.md jest zielony: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- „Dokumentacja ustawień” w pasku filtra z ustawień otwiera sekcję „Configuration” README na GitHubie w przeglądarce
- Wybór dwóch etykiet pokazuje pozycje z jedną LUB drugą; dodanie projektu zawęża do pozycji z tego projektu z którąkolwiek z tych etykiet
- Kamień milowy w filtrze ukrywa pozycje bez kamienia i z innym kamieniem
- Klik w chip etykiety lub w nazwę projektu w wierszu przełącza wartość, a stan wciśnięty i licznik na przycisku się zgadzają; klik w tytuł dalej otwiera GitLaba
- Stopki pokazują osobno „Ukryte filtrem” i „Poza filtrem widoku”; filtr, który nie zostawia nic, pokazuje komunikat i „Wyczyść filtr widoku”
- Filtr działa w sekcji „Zamknięte” i na ukrytych po „Pokaż ukryte”, łącznie z przełącznikiem typu
- Przejście na „Do zrobienia” i z powrotem zachowuje typ, „Pokaż ukryte” i wybory; restart aplikacji je czyści
- Lista wyboru zamyka się Escape i klikiem poza nią, działa z klawiatury, a wartość, która zniknęła z listy po odświeżeniu, zostaje z licznikiem 0 i da się ją odznaczyć
- „Do zrobienia” (picker GitLaba) i powiadomienia działają jak przed zmianą, niezależnie od filtra widoku

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- `cli`: dokładne argumenty czterech wywołań z nowym sufiksem; strażnik odrzuca stary kształt, `with_labels_details=false`, dodatkowe `&` i parametr w innym miejscu.
- `gitlab`: parser etykiet w obu postaciach; walidacja koloru (`#abc`, `#AABBCC` → małe litery, `red`, `#abcd`, `#ggg000`, brak `text_color`); zła postać → `ResponseInvalid` bez treści; serializacja `labels` jako obiektów i `milestone`.
- `filter`, `hidden_issues`: te same wyniki dla tych samych nazw; ukryte zgłoszenie z kolorowymi etykietami nie wraca jako zmienione.

### Integration Tests:

- Testy z `fake-cli` dla przypisanych i zamkniętych przechodzą z etykietami-obiektami i etykietami-nazwami; marker wycieku dalej nie trafia do błędów ani logów.

### Manual Testing Steps:

1. Z VPN: lista się ładuje, chipy mają kolory GitLaba, scoped etykiety są podzielone.
2. Wybrać etykietę z listy „Etykiety”, potem projekt klikiem w wierszu i sprawdzić zawężenie i liczniki w stopkach.
3. Włączyć „Pokaż ukryte” i sekcję „Zamknięte” (`closedDays`) i sprawdzić, że filtr działa także tam.
4. Przejść na inną zakładkę i wrócić (stan zostaje), potem zrestartować aplikację (stan czysty).
5. Wybrać wartość, która nie zostawia nic: widać komunikat i „Wyczyść filtr widoku”.
6. Bez VPN: karta „GitLab niedostępny” jak dziś, a filtr nie psuje widoku po powrocie sieci.

## Performance Considerations

Listy mają rząd setek pozycji. Filtrowanie i liczenie opcji przy renderze jest liniowe. `viewOptions` można zapamiętać przez `useMemo` po `snapshot.seq`, nic więcej nie jest potrzebne. `with_labels_details` nieznacznie powiększa odpowiedź GitLaba.

## Migration Notes

Bez migracji bazy. Rekordy ukrytych przechowują nazwy etykiet scoped, a te się nie zmieniają. Wybory filtra widoku są tylko w pamięci.

## References

- Etykiety GitHub (wzorzec zmiany wywołania, strażnika i parsera): `context/archive/2026-11-24-github-issue-labels/plan.md`
- Filtr z ustawień: `context/archive/2026-11-23-gitlab-visibility-filter/plan.md`, `src-tauri/src/gitlab/filter.rs`
- Pasek filtra: `context/archive/2026-11-24-gitlab-filter-bar-retry/plan.md`, `src/gitlab/GitlabFilterBar.tsx`
- Wywołania i strażnik: `src-tauri/src/cli/mod.rs:207-300`, `:609-691`
- Reguła scoped ukrytych: `src-tauri/src/hidden_issues/rules.rs:80-93`
- Widok: `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`, kontekst wzorcowy `src/worktime/WorkTimerProvider.tsx`
- GitLab REST: parametr `with_labels_details` w `GET /issues` i `GET /merge_requests`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Etykiety z kolorami i kamień milowy w danych

#### Automated

- [x] 1.1 Strażnik i testy argumentów przyjmują nowe kształty z `with_labels_details=true` i odrzucają stary kształt oraz `with_labels_details=false` — f1aadd4
- [x] 1.2 Parser etykiet: obie postaci, walidacja kolorów, zła postać daje `ResponseInvalid` — f1aadd4
- [x] 1.3 `scoped_labels` bez zmiany wyniku, ukryte z kolorowymi etykietami nie wraca jako zmienione — f1aadd4
- [x] 1.4 `milestone` serializowany w migawce — f1aadd4
- [x] 1.5 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` czyste — f1aadd4
- [x] 1.6 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — f1aadd4

#### Manual

- [ ] 1.7 Na prawdziwym GitLabie lista ładuje się jak wcześniej z tymi samymi nazwami etykiet

### Phase 2: Chipy etykiet w wierszach

#### Automated

- [x] 2.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 78996ff
- [x] 2.2 Brak literalnych kolorów poza `tokens.css` — 78996ff

#### Manual

- [ ] 2.3 Chipy w kolorach GitLaba, scoped etykiety dzielone
- [ ] 2.4 Niepoprawny kolor daje czytelny neutralny chip
- [ ] 2.5 Wiele etykiet zawija się bez przewijania w bok, kolumna znika poniżej 900 px
- [ ] 2.6 Chipy w sekcji „Zamknięte” i w ukrytych wierszach

### Phase 3: Filtr widoku

#### Automated

- [x] 3.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- [x] 3.2 Pełny zestaw bramek Rust z AGENTS.md zielony

#### Manual

- [ ] 3.3 LUB w wymiarze, I między wymiarami
- [ ] 3.4 Kamień milowy ukrywa pozycje bez kamienia i z innym
- [ ] 3.5 Klik w chip lub projekt przełącza wartość, tytuł dalej otwiera GitLaba
- [ ] 3.6 Osobne liczniki w stopkach i pusty stan z „Wyczyść filtr widoku”
- [ ] 3.7 Filtr działa w „Zamknięte” i na ukrytych, łącznie z typem
- [ ] 3.8 Stan przeżywa zmianę zakładki, restart go czyści
- [ ] 3.9 Lista wyboru: Escape, klik poza, klawiatura, zniknięta wartość z licznikiem 0
- [ ] 3.10 „Do zrobienia” i powiadomienia bez zmian
- [ ] 3.11 „Dokumentacja ustawień” otwiera README (sekcja „Configuration”) na GitHubie
