# Wspólne narzędzia UI — plan implementacji

## Overview

Osiem narzędzi, które przy dopasowaniu widoków do makiet Konsoli powstało lokalnie (zegar „N min temu”, polska odmiana, zakładki, siatka tabeli, rozmiar `Segmented`, chip priorytetu, treść błędu jako węzeł, blokada akcji stanu źródła), przenosimy do `src/components/` i przepinamy na nie widoki. To fundament F-01 z roadmapy (issue #10): kolejne wycinki M-2 — pasek filtrów GitLaba (S-03) i tabela dni w „Czasie pracy” (S-05) — budują na wspólnych elementach zamiast dokładać kolejne kopie. Wygląd się nie zmienia poza dwoma uzgodnionymi drobiazgami: `gh auth login` jako `<code>` w błędzie GitHuba i zablokowane „Sprawdź teraz” w trakcie odświeżania.

## Current State Analysis

- Cztery kopie zegara odświeżającego względne etykiety co 30 s: `src/github/GithubView.tsx:139-144`, `src/gitlab/GitlabView.tsx:425-429`, `src/health/HealthPanel.tsx:22-29` (lokalny `useNow`), `src/todo/SourcesPanel.tsx:22-29` (`useMinuteClock`). `src/worktime/WorkTimeView.tsx:88,103-108` ma własny tyk co 1 s, włączany tylko przy biegnącym wpisie.
- Trzy identyczne `plural`: `src/github/GithubView.tsx:26`, `src/gitlab/GitlabView.tsx:39`, eksport w `src/todo/summary.ts:8` (używany przez `src/todo/TodoView.tsx:112,607`).
- Zakładki: `src/own-tasks/OwnTaskTabs.tsx` — poprawny tablist z roving focus, ale z typem `OwnTaskTab` na sztywno, `aria-label="Filtr zadań"` w środku i stylami `.own-tab*` w `src/own-tasks/OwnTasks.css:64-103`.
- Tabela: `GitlabTable`/`GitlabRow` (`src/gitlab/GitlabView.tsx:145-265`) to siatka `div role="table"`; struktura (role, nagłówek `label-caps`, linie, wysokości) i typografia komórek siedzą w `src/gitlab/GitlabView.css:31-134`, razem z szablonem kolumn i progami 900/600 px, które są specyficzne dla GitLaba.
- `Segmented` (`src/components/Segmented/Segmented.tsx`) ma jedną wysokość (`--size-control`, 36 px); `src/own-tasks/OwnTasks.css:41-44` nadpisuje `.dn-seg-btn` z zewnątrz, żeby zrównać go z polami 40 px (`--size-field`).
- Chip priorytetu: `<span className="own-prio own-prio-<p>">` w `src/own-tasks/OwnTasksView.tsx:146` i style w `src/own-tasks/OwnTasks.css:131-149`. `Flag` (`src/components/Flag/`) to sygnał zmiany: mono, WIELKIE LITERY, tony `change`/`info`.
- `ErrorState.message: string` (`src/components/ErrorState.tsx:5`), choć `SourceState.children` już jest `ReactNode`. Komunikat „brak sesji” GitHuba (`src/github/GithubView.tsx:121`) cytuje polecenie w cudzysłowach zamiast `<code>`.
- `SourceState` (`src/components/SourceState/SourceState.tsx`) nie przyjmuje blokady akcji; „Sprawdź teraz” w GitLabie (`src/gitlab/GitlabView.tsx:506-515`) i GitHubie (`src/github/GithubView.tsx:217-224`) da się klikać w trakcie odświeżania, mimo że `snapshot.refreshing` jest w zasięgu (`GitlabView.tsx:453`, `GithubView.tsx:168`).
- UI nie ma runnera testów (AGENTS.md → Testing); weryfikacja to bramki lint/format/build i ręczny przegląd ekranów.

## Desired End State

`src/components/` eksportuje (przez `index.ts`) `useNow`, `RELATIVE_TICK_MS`, `plural`, `Tabs`, `Chip`, `DataTable`/`DataTableRow`/`DataTableCell`, a `Segmented` przyjmuje `size`. Żaden widok nie definiuje własnego zegara względnego czasu, własnego `plural`, własnych zakładek ani chipu, nie nadpisuje `.dn-seg-btn` i nie trzyma struktury tabeli w swoim CSS (zostaje w nim tylko szablon kolumn i boczny odstęp per próg, responsywne ukrywanie i typografia kolumn specyficznych dla GitLaba). `ErrorState.message` przyjmuje `ReactNode`, `SourceState` przyjmuje `actionDisabled`. Ekrany wyglądają jak w 0.2.x, z dwiema uzgodnionymi zmianami: `<code>gh auth login</code>` w błędzie GitHuba oraz „Sprawdź teraz” zablokowane, gdy trwa odświeżanie GitLaba/GitHuba.

Weryfikacja: pełna bramka z AGENTS.md przechodzi, wyszukiwania z kryteriów sukcesu nie znajdują lokalnych kopii, a ręczny przegląd każdego widoku przy 1280 i 800 px nie pokazuje różnic poza dwiema uzgodnionymi.

### Key Discoveries:

- `HealthPanel` już ma docelowy kształt hooka (`src/health/HealthPanel.tsx:22-29`) — wspólny `useNow` to jego uogólnienie o `null` (brak tyku).
- `OwnTaskTabs` ma gotową obsługę klawiatury (strzałki, Home, End) i powiązanie `aria-controls`/`aria-labelledby` z panelem (`src/own-tasks/OwnTasksView.tsx:205-213`); wystarczy uogólnić typ i nazwę grupy.
- Komponenty mają już konwencję rozmiarów `sm` 36 px / `md` 40 px (`src/components/Button/Button.tsx:8-9`) — `Segmented` przejmuje ją zamiast wymyślać nową.
- Tokeny `--size-control`, `--size-field`, `--size-touch`, `--size-row` oraz klasy `.label-caps` i `.code` są w `src/styles/tokens.css` — nowe komponenty nie potrzebują nowych wartości.
- Karta „GitLab nie jest skonfigurowany” już renderuje `<code>` w treści `SourceState` (`src/gitlab/GitlabView.tsx:496-503`), więc styl `code` w stanach źródeł istnieje.

## What We're NOT Doing

- `useWorkTimer`, `WorkTimerHeader` i podwójne subskrypcje licznika — to F-02 (`shared-work-timer-state`). W `WorkTimeView` zmieniamy tylko lokalny tyk `nowMs`, nie `timerNowMs`.
- Sterowana danymi tabela (definicje kolumn, progi w JS) — `DataTable` to szkielet; szablon kolumn i responsywność zostają w CSS widoku.
- Zmiana wyglądu priorytetu na styl `Flag`; nowe tony `Flag` (`new` należy do S-02).
- Nowe stany, teksty i układy poza dwiema uzgodnionymi zmianami; pasek filtrów GitLaba (S-03), tabela dni (S-05).
- Aktualizacja artefaktu design systemu o `Chip`, `Tabs`, `DataTable` i rozmiar `Segmented` — to F-05 (`design-system-artifact-sync`).
- Zmiany w Rust, komendach Tauri i kontraktach TS↔Rust.
- Runner testów dla UI.

## Implementation Approach

Trzy fazy, każda osobno weryfikowalna i commitowana: najpierw narzędzia bez wyglądu (zegar, odmiana), potem komponenty prezentacyjne z przepięciem „Moich zadań” i GitLaba, na końcu rozszerzenia stanów źródeł z dwiema widocznymi zmianami. Każdy nowy element trafia do `src/components/` zgodnie z konwencją folderu komponentu (`Name/Name.tsx` + `Name.css` z prefiksem `dn-`, wartości wyłącznie z tokenów) i jest eksportowany z `src/components/index.ts`. Style przenoszone z CSS widoków zachowują dokładnie te same wartości, żeby wygląd się nie zmienił.

## Critical Implementation Details

- **Kolejność CSS** — CSS komponentów ładuje się przed CSS widoków (`src/App.tsx:2` importuje `./components` przed widokami; w `GitlabView.tsx` `../components` stoi przed `./GitlabView.css`), więc nadpisanie w widoku o równej specyficzności wygrywa kolejnością. Na tym opiera się 11 px etykiet GitLaba wobec globalnego `.code` (12 px); nie zmieniaj kolejności importów.
- **Reguły React Compiler** — `eslint.config.js` włącza `reactHooks.configs.flat.recommended`, w tym `react-hooks/set-state-in-effect`: `useNow` ustawia stan tylko w callbacku interwału, a wartość początkową bierze z leniwego `useState(() => Date.now())`.

## Phase 1: Zegar i odmiana

### Overview

Wspólny hook `useNow` i funkcja `plural` w `src/components/`; widoki przestają trzymać własne `setInterval` dla zegara i własne kopie odmiany.

### Changes Required:

#### 1. Hook zegara

**File**: `src/components/useNow.ts` (nowy), `src/components/index.ts`

**Intent**: Jedno miejsce na „aktualny czas, odświeżany co N ms” dla względnych etykiet i tykających sum, z możliwością wyłączenia tyku.

**Contract**: `useNow(intervalMs: number | null): number` — zwraca `Date.now()` z chwili montowania, potem nową wartość co `intervalMs`; `null` nie uruchamia interwału i zostawia ostatnią wartość (zachowanie dzisiejszego `WorkTimeView`). Zmiana `intervalMs` czyści stary interwał. Bez ustawiania stanu w ciele efektu poza callbackiem interwału (reguły `react-hooks`). Obok eksport `RELATIVE_TICK_MS = 30_000` — interwał dla etykiet „N min temu” / „3 dni”.

#### 2. Odmiana

**File**: `src/components/plural.ts` (nowy), `src/components/index.ts`, `src/todo/summary.ts`

**Intent**: Jedna polska odmiana liczebnika zamiast trzech kopii.

**Contract**: `plural(n: number, one: string, few: string, many: string): string` — przeniesiona bez zmian z `src/todo/summary.ts:7-13` (wraz z komentarzem reguły). `summary.ts` przestaje ją eksportować.

#### 3. Przepięcie widoków

**File**: `src/github/GithubView.tsx`, `src/gitlab/GitlabView.tsx`, `src/health/HealthPanel.tsx`, `src/todo/SourcesPanel.tsx`, `src/todo/TodoView.tsx`, `src/worktime/WorkTimeView.tsx`

**Intent**: Usunąć lokalne zegary i kopie `plural`, użyć wspólnych.

**Contract**: GitHub, GitLab, „Stan źródeł” i `SourcesPanel` wołają `useNow(RELATIVE_TICK_MS)` (lokalne `NOW_TICK_MS`, `useNow` z `HealthPanel` i `useMinuteClock` znikają). `WorkTimeView` zastępuje `useState` + efekt tyku przez `useNow(ticking ? 1000 : null)`; `ticking` liczone jak dziś. GitHub, GitLab i `TodoView` importują `plural` z `../components`; `formatWorkdays` w `src/worktime/format.ts:32-38` też woła `plural` zamiast wpisanej reguły 2–4 / 12–14 (wynik bez zmian). Nazwy zmiennych `now`/`nowMs` w widokach mogą zostać.

### Success Criteria:

#### Automated Verification:

- Pełna bramka z AGENTS.md przechodzi: `npm run lint`, `npm run format:check`, `npm run build`, a w `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Brak lokalnych kopii: `grep -rn "function plural\|function useNow\|useMinuteClock\|tens < 12" src` zwraca tylko `src/components/`
- Brak zegarów w widokach: `grep -rn "setInterval(() => setNow" src` zwraca tylko `src/components/useNow.ts`

#### Manual Verification:

- GitLab, GitHub, „Stan źródeł” i panel źródeł na pulpicie: etykiety „N min temu” / wiek pozycji przesuwają się po ok. 30 s, jak dotąd
- „Czas pracy”, dziś, licznik biegnie: „Razem” tyka co sekundę; po STOP przestaje tykać; dzień przeszły nie tyka
- Liczebniki w nagłówkach list (GitLab „widoczne/ukryte”, GitHub „otwarte/repozytoria”, „Do zrobienia” „wolne miejsca”/„pozycje”) mają poprawne formy dla 1, 2, 5 i 12

**Implementation Note**: Po zielonej bramce automatycznej zatrzymaj się na ręczne potwierdzenie przed fazą 2.

---

## Phase 2: Komponenty prezentacyjne

### Overview

`Tabs`, `Chip`, rozmiar `Segmented` i szkielet `DataTable` w `src/components/`; „Moje zadania” i tabela GitLaba używają ich, a nadpisania i typografia znikają z CSS widoków.

### Changes Required:

#### 1. Zakładki

**File**: `src/components/Tabs/Tabs.tsx`, `src/components/Tabs/Tabs.css` (nowe), `src/components/index.ts`; usunięcie `src/own-tasks/OwnTaskTabs.tsx`; `src/own-tasks/OwnTasksView.tsx`

**Intent**: Uogólnić istniejący tablist „Moich zadań” tak, by dowolny widok mógł go użyć, bez zmiany zachowania klawiatury i wyglądu.

**Contract**: `Tabs<T extends string>({ tabs: { value: T; label: string; count?: number }[]; value: T; onChange: (v: T) => void; label: string; panelId: string; idPrefix: string })` — `label` to `aria-label` grupy (w „Moich zadaniach” „Filtr zadań”), id zakładki `${idPrefix}-${value}` jak dziś, licznik renderowany tylko przy podanym `count`. Style `.own-tabs`, `.own-tab`, `.own-tab.is-on`, `.own-tab:focus-visible`, `.own-tab-count` przechodzą 1:1 jako `.dn-tabs`, `.dn-tab`, `.dn-tab-count`. Typ `OwnTaskTab` przenosi się do `src/own-tasks/types.ts` lub `OwnTasksView.tsx`.

#### 2. Chip

**File**: `src/components/Chip/Chip.tsx`, `src/components/Chip/Chip.css` (nowe), `src/components/index.ts`; `src/own-tasks/OwnTasksView.tsx`

**Intent**: Mały obrysowany znacznik wartości (priorytet), osobny od `Flag`, który pozostaje sygnałem zmiany.

**Contract**: `Chip({ tone?: "warn" | "soft" | "faint"; children: ReactNode; className?: string })` — wygląd dzisiejszego `.own-prio` (11/16 px, `--space-hair`/`--space-1h`, obrys `--line-control`, `--radius-sm`, zwykłe litery), tony = kolory `--warn` / `--ink-soft` / `--ink-faint`. `OwnTasksView` mapuje `high→warn`, `medium→soft`, `low→faint`; ukryty prefiks „Priorytet: ” zostaje w widoku.

#### 3. Rozmiar Segmented

**File**: `src/components/Segmented/Segmented.tsx`, `src/components/Segmented/Segmented.css`; `src/own-tasks/OwnTaskForm.tsx`, `src/own-tasks/OwnTaskEditPanel.tsx`

**Intent**: Segmented równy polom 40 px bez nadpisywania klas komponentu z zewnątrz.

**Contract**: `SegmentedProps.size?: "sm" | "md"`, domyślnie `"sm"` (dzisiejsze `--size-control`); `"md"` daje przyciskom `min-height: calc(var(--size-field) - 2px)` — ta sama wartość co nadpisanie w `OwnTasks.css:42-44`. Formularz i panel „Edycja” przekazują `size="md"`; filtr rodzaju w GitLabie zostaje przy domyślnym.

#### 4. Szkielet tabeli

**File**: `src/components/DataTable/DataTable.tsx`, `src/components/DataTable/DataTable.css` (nowe), `src/components/index.ts`; `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`

**Intent**: Wspólna struktura i typografia tabeli-siatki (role ARIA, nagłówek, linie, wysokości, warianty komórek), a szablon kolumn i responsywne ukrywanie zostają w widoku — tak, by tabela dni w S-05 użyła tego samego szkieletu.

**Contract**:
- `DataTable({ label: string; className?: string; children })` → `div role="table"` z `aria-label`.
- `DataTableRow({ head?: boolean; className?: string; children })` → `role="row"`, siatka `grid-template-columns: var(--dn-table-cols)`; `head` dodaje styl nagłówka (`label-caps`, `--size-control`, linia `--line`), wiersz danych ma `--size-row` i linię `--line-subtle`.
- `DataTableCell({ header?: boolean; tone?: "muted" | "faint"; truncate?: boolean; className?: string; title?: string; children })` → `role="columnheader"` przy `header`, inaczej `role="cell"`; `min-width: 0`.
- `DataTable.css` dostaje z `GitlabView.css:33-82`: siatkę z `--dn-table-cols`, `gap: var(--space-3)`, boczny odstęp przez zmienną `padding: 0 var(--dn-table-pad-x, var(--space-4))`, wysokości i linie nagłówka oraz wiersza, `min-width: 0` komórek, warianty `tone` (`muted` = `--ink-muted`, `faint` = `--ink-faint`) i `truncate` (jedna linia z wielokropkiem).
- `GitlabView.css` zostaje z: `--dn-table-cols` i `--dn-table-pad-x` dla `.gitlab-table` (dzisiejsze 7 kolumn; przy 900 px 5 kolumn; przy 600 px 3 kolumny i `--dn-table-pad-x: var(--space-3)` w miejsce dzisiejszego nadpisania paddingu na `.gitlab-grid`), ukrywaniem kolumn projektu/etykiet/nr/czasu, 11 px dla etykiet, układem komórki tytułu (14/20 px, flagi), `.is-hidden` i układem akcji. `GitlabTable`/`GitlabRow` składają się z `DataTable*`; kolumny, ich kolejność i treść bez zmian.

#### 5. Komentarz CSS widoku

**File**: `src/own-tasks/OwnTasks.css`

**Intent**: Usunąć przeniesione style (zakładki, chip, nadpisanie `.dn-seg-btn`) i nieaktualny komentarz „no shared component yet”.

**Contract**: Plik zawiera wyłącznie układ (zgodnie z regułą „view CSS is layout only”).

### Success Criteria:

#### Automated Verification:

- Pełna bramka z AGENTS.md przechodzi: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Brak nadpisań i starych klas: `grep -rn "dn-seg-btn\|own-tab\|own-prio" src` zwraca tylko `src/components/Segmented/`
- `src/own-tasks/OwnTaskTabs.tsx` nie istnieje, a `src/components/index.ts` eksportuje `Tabs`, `Chip`, `DataTable`, `DataTableRow`, `DataTableCell`

#### Manual Verification:

- „Moje zadania”: zakładki Praca / Prywatne / Zrobione wyglądają jak dotąd (podkreślenie akcentem, liczniki); strzałki, Home i End przełączają i przenoszą fokus; czytnik ekranu ogłasza „Filtr zadań”
- „Moje zadania”: chip priorytetu (wysoki pomarańczowy, normalny, niski) wygląda jak dotąd; „Przestrzeń” w szybkim dodawaniu i w „Edycji” ma wysokość pól obok
- GitLab przy 1280, 900 i 800 px oraz poniżej 600 px: kolumny, nagłówki, ucinanie tytułów i etykiet, ukrywanie kolumn, przyciemnienie ukrytych wierszy i tabela „Zamknięte” bez zmian
- Filtr rodzaju w GitLabie (Segmented) ma dotychczasową wysokość 36 px

**Implementation Note**: Po zielonej bramce automatycznej zatrzymaj się na ręczne potwierdzenie przed fazą 3.

---

## Phase 3: Stany źródeł

### Overview

`ErrorState.message` jako węzeł i blokada akcji w `SourceState`; dwie uzgodnione widoczne zmiany w GitHubie i GitLabie.

### Changes Required:

#### 1. Treść błędu jako węzeł

**File**: `src/components/ErrorState.tsx`

**Intent**: Pozwolić na znaczniki (np. `<code>`) w treści błędu bez obchodzenia `ErrorState`.

**Contract**: `message: ReactNode`; dotychczasowe wywołania z `string` działają bez zmian.

#### 2. Blokada akcji stanu źródła

**File**: `src/components/SourceState/SourceState.tsx`

**Intent**: Akcję karty („Sprawdź teraz”) da się zablokować, gdy jej kliknięcie nic by nie dało.

**Contract**: `SourceStateProps.actionDisabled?: boolean` → `disabled` i `aria-busy` na przycisku akcji; brak propsa = dzisiejsze zachowanie. `ErrorState` nie dostaje nowego propsa (jego „Spróbuj ponownie” zostaje bez zmian).

#### 3. Uzgodnione zmiany w widokach

**File**: `src/github/GithubView.tsx`, `src/gitlab/GitlabView.tsx`

**Intent**: Użyć nowych możliwości tam, gdzie zgłoszenie je wskazuje.

**Contract**: Błąd „GitHub: brak sesji” pokazuje polecenie jako `<code>gh auth login</code>` zamiast „„gh auth login””; reszta tekstu bez zmian. Karta „GitLab niedostępny” i „GitHub niedostępny” przekazują `actionDisabled={refreshing}` dla „Sprawdź teraz”.

### Success Criteria:

#### Automated Verification:

- Pełna bramka z AGENTS.md przechodzi: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- `grep -n "gh auth login" src/github/GithubView.tsx` pokazuje polecenie wewnątrz `<code>`

#### Manual Verification:

- Wylogowane `gh`: czerwony blok „GitHub: brak sesji” z `gh auth login` w kroju mono; „Spróbuj ponownie” działa
- GitLab bez VPN: „Sprawdź teraz” jest nieaktywne, dopóki trwa odświeżanie, i wraca po jego końcu; karta pozostaje spokojna (szara, bez czerwieni)
- Inne błędy (`ErrorState` z tekstem) wyglądają jak dotąd

---

## Testing Strategy

### Unit Tests:

- Brak — UI nie ma runnera testów (AGENTS.md → Testing), a zmiana nie dotyka reguł domenowych w Rust. `cargo test` przechodzi jako część bramki bez zmian.

### Integration Tests:

- Brak nowych; istniejące testy Rust nie są dotykane.

### Manual Testing Steps:

1. `npm run tauri dev`; przejść kolejno pulpit, GitLab, GitHub, „Moje zadania”, „Czas pracy”, „Stan źródeł” przy 1280×800 i zawęzić okno do 800 px — porównać z wyglądem sprzed zmiany.
2. „Moje zadania”: tylko klawiaturą przejść zakładki (Tab, strzałki, Home, End) i formularz z „Przestrzenią”.
3. Odłączyć VPN i kliknąć „Sprawdź teraz” w GitLabie — przycisk blokuje się na czas odświeżania.
4. `gh auth logout` (lub konto bez sesji) i otworzyć GitHub — `gh auth login` w `<code>`; potem zalogować z powrotem.
5. „Czas pracy” z biegnącym licznikiem: „Razem” tyka; po STOP stoi.

## Performance Considerations

Bez zmian: te same interwały (30 s dla etykiet, 1 s tylko przy biegnącym wpisie) i ta sama liczba zegarów na ekranie.

## Migration Notes

Nie dotyczy — brak danych i kontraktów Rust. Usunięty `OwnTaskTabs.tsx` i eksport `plural` z `summary.ts` nie mają innych użytkowników poza przepinanymi widokami.

## References

- Roadmap: `context/foundation/roadmap.md` → F-01 (`shared-ui-tools`), odblokowuje S-03 i S-05
- Issue: GitHub #10 „Wspólne narzędzia UI wyciągnięte z widoków”
- Wzorzec hooka: `src/health/HealthPanel.tsx:22-29`
- Wzorzec zakładek: `src/own-tasks/OwnTaskTabs.tsx`
- Konwencja rozmiarów: `src/components/Button/Button.tsx:8-9`
- Poprzednia zmiana UI: `context/changes/konsola-design-system/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Zegar i odmiana

#### Automated

- [x] 1.1 Pełna bramka z AGENTS.md przechodzi: `npm run lint`, `npm run format:check`, `npm run build`, a w `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — de5842c
- [x] 1.2 Brak lokalnych kopii: `grep -rn "function plural\|function useNow\|useMinuteClock\|tens < 12" src` zwraca tylko `src/components/` — de5842c
- [x] 1.3 Brak zegarów w widokach: `grep -rn "setInterval(() => setNow" src` zwraca tylko `src/components/useNow.ts` — de5842c

#### Manual

- [x] 1.4 GitLab, GitHub, „Stan źródeł” i panel źródeł na pulpicie: etykiety „N min temu” / wiek pozycji przesuwają się po ok. 30 s, jak dotąd — de5842c
- [x] 1.5 „Czas pracy”, dziś, licznik biegnie: „Razem” tyka co sekundę; po STOP przestaje tykać; dzień przeszły nie tyka — de5842c
- [x] 1.6 Liczebniki w nagłówkach list (GitLab „widoczne/ukryte”, GitHub „otwarte/repozytoria”, „Do zrobienia” „wolne miejsca”/„pozycje”) mają poprawne formy dla 1, 2, 5 i 12 — de5842c

### Phase 2: Komponenty prezentacyjne

#### Automated

- [x] 2.1 Pełna bramka z AGENTS.md przechodzi: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [x] 2.2 Brak nadpisań i starych klas: `grep -rn "dn-seg-btn\|own-tab\|own-prio" src` zwraca tylko `src/components/Segmented/`
- [x] 2.3 `src/own-tasks/OwnTaskTabs.tsx` nie istnieje, a `src/components/index.ts` eksportuje `Tabs`, `Chip`, `DataTable`, `DataTableRow`, `DataTableCell`

#### Manual

- [x] 2.4 „Moje zadania”: zakładki Praca / Prywatne / Zrobione wyglądają jak dotąd (podkreślenie akcentem, liczniki); strzałki, Home i End przełączają i przenoszą fokus; czytnik ekranu ogłasza „Filtr zadań”
- [x] 2.5 „Moje zadania”: chip priorytetu (wysoki pomarańczowy, normalny, niski) wygląda jak dotąd; „Przestrzeń” w szybkim dodawaniu i w „Edycji” ma wysokość pól obok
- [x] 2.6 GitLab przy 1280, 900 i 800 px oraz poniżej 600 px: kolumny, nagłówki, ucinanie tytułów i etykiet, ukrywanie kolumn, przyciemnienie ukrytych wierszy i tabela „Zamknięte” bez zmian
- [x] 2.7 Filtr rodzaju w GitLabie (Segmented) ma dotychczasową wysokość 36 px

### Phase 3: Stany źródeł

#### Automated

- [ ] 3.1 Pełna bramka z AGENTS.md przechodzi: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [ ] 3.2 `grep -n "gh auth login" src/github/GithubView.tsx` pokazuje polecenie wewnątrz `<code>`

#### Manual

- [ ] 3.3 Wylogowane `gh`: czerwony blok „GitHub: brak sesji” z `gh auth login` w kroju mono; „Spróbuj ponownie” działa
- [ ] 3.4 GitLab bez VPN: „Sprawdź teraz” jest nieaktywne, dopóki trwa odświeżanie, i wraca po jego końcu; karta pozostaje spokojna (szara, bez czerwieni)
- [ ] 3.5 Inne błędy (`ErrorState` z tekstem) wyglądają jak dotąd
