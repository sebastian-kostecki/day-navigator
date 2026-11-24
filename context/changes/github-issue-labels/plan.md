# Etykiety zgłoszeń GitHub — plan implementacji

## Overview

W „Projekty (GitHub)” każdy wiersz zgłoszenia pokazuje jego etykiety jako tekst, tak jak kolumna „Etykiety” w „Praca (GitLab)”. To wycinek S-04 z roadmapy M-2 (issue #7, PRD FR-025). Stałe wywołanie `gh issue list` pobiera dodatkowo pole `labels`. Wywołanie dalej tylko czyta, ma stałe argumenty i jest objęte testem strażnika. `GithubIssue` (Rust i mirror TS) niesie nazwy etykiet, a wspólny `ItemRow` dostaje opcjonalny slot na etykiety, z którego korzysta widok GitHuba.

## Current State Analysis

- Wywołanie: `gh_assigned_issues_args` w `src-tauri/src/cli/mod.rs:185-202` buduje `issue list -R <repo> --assignee @me --state open --limit 500 --json number,title,url,updatedAt`. Ten sam kształt jest zapisany jeszcze trzy razy, celowo osobno: `GH_ISSUES_TAIL` w strażniku (`src-tauri/src/cli/mod.rs:713-722`, sprawdzany w `check_read_only`, `:726-758`), test dokładnych argumentów `gh_assigned_issues_has_exact_arguments_name_and_tool` (`src-tauri/src/cli/mod.rs:1089-1112`) i test argv przez prawdziwy proces w `src-tauri/tests/cli_runner.rs:279-293`.
- Parsowanie: `RawIssue` (`src-tauri/src/github/mod.rs:135-142`) czyta `number`, `title`, `url`, `updatedAt`; `parse_issues` (`:160-184`) zamienia każdy błąd serde na `ResponseInvalid` bez treści odpowiedzi. `GithubIssue` (`:19-33`) nie ma etykiet.
- `GithubIssue` budują ręcznie: test `issue()` w `src-tauri/src/github/mod.rs:475-484`, `src-tauri/src/todo/commands.rs:239`, `src-tauri/src/todo/tests.rs:609-618`, `src-tauri/tests/todo.rs:232-241`, `src-tauri/tests/todo_crash.rs:49-58`. „Do zrobienia” zapisuje z niego tylko referencję i tytuł (`src-tauri/src/todo/mod.rs:388-410`), więc etykiety nie trafiają do bazy.
- `fake-cli`: scenariusz `gh-issues` (`src-tauri/src/bin/fake_cli.rs:414-470`) zwraca zgłoszenia z `gh_issue_value` (`:405-412`) bez etykiet. Test `a_loaded_title_reaches_the_issue_list_and_nothing_else` (`src-tauri/tests/github_fetch.rs:107-132`) pilnuje, żeby treść odpowiedzi nie trafiła do stanu, tray ani tekstu błędu.
- UI GitLaba: `GitlabRow` łączy `item.labels.join(", ")` w komórce `gitlab-col-labels code`, z pełną listą w `title` (`src/gitlab/GitlabView.tsx:155,190-197`). Kolumna ma 150 px i font 11 px (`src/gitlab/GitlabView.css:35,42-44`) i znika poniżej 900 px (`:79-88`).
- UI GitHuba: `IssueRow` (`src/github/GithubView.tsx:34-70`) to wspólny `ItemRow` (`src/components/ItemRow/ItemRow.tsx`): `#nr`, tytuł, flaga „W Do zrobienia”, meta z wiekiem, akcje. `ItemRow` nie ma miejsca na etykiety. Używają go też `OwnTasksView` i `TodoView`.
- UI nie ma runnera testów; weryfikacja UI to bramki lint/format/build i ręczny przegląd.

## Desired End State

Każdy wiersz w „Projekty (GitHub)” pokazuje etykiety zgłoszenia jako tekst po przecinku, między tytułem (z flagą) a wiekiem. Tekst jest mono, 11 px i przyciemniony. Długą listę ucina wielokropek, a pełna lista jest w podpowiedzi. Zgłoszenie bez etykiet nie pokazuje nic. Poniżej 900 px szerokości okna etykiety znikają, jak w GitLabie. Wywołanie `gh` różni się tylko polem `labels` w `--json`, a strażnik, test dokładnych argumentów i test argv przechodzą z nowym kształtem.

Weryfikacja: pełna bramka z AGENTS.md przechodzi, testy z fazy 1 sprawdzają etykiety od `fake-cli` do `GithubIssue`, a ręczny przegląd przy 1280 i 800 px pokazuje etykiety tylko tam, gdzie powinny.

### Key Discoveries:

- `gh issue list --json labels` zwraca obiekty (`{"id", "name", "description", "color"}`); potrzebne jest tylko `name`, reszta jest ignorowana przez serde. Sprawdzone 2026-11-24 na `gh` 2.105.0 w Windows, wywołaniem tylko do odczytu na publicznym repozytorium `cli/cli`.
- GitLab czyta `labels` z `#[serde(default)]` i odrzuca zły typ jako błąd odpowiedzi (`src-tauri/src/gitlab/mod.rs:204,574-589,1117-1120`); GitHub robi tak samo, żeby oba źródła zachowywały się jednakowo.
- Strażnik celowo powtarza ogon argumentów zamiast importować go z kodu produkcyjnego (`src-tauri/src/cli/mod.rs:712-722`); zmiana musi trafić w oba miejsca ręcznie, inaczej `every_call_is_read_only` się wywróci — i dobrze.
- `.dn-sr-only` jest w `src/styles/layout.css:68`, `.code` w tokenach — nowy slot nie potrzebuje nowych wartości poza rozmiarem fontu, który `ItemRow.css` i tak trzyma literalnie (12 px dla ref i meta).

## What We're NOT Doing

- Bez etykiet przy pozycjach GitHub w „Do zrobienia” ani w pickerze. FR-025 dotyczy tylko „Projekty (GitHub)”, a pozycje GitLab też ich tam nie mają.
- Bez kolorów etykiet z GitHuba (`color`) i bez opisów (`description`). Etykiety to tekst, jak w GitLabie.
- Bez filtrowania ani sortowania po etykietach. Kolejność jest taka, jaką przysłał GitHub.
- Bez przepisywania widoku GitHuba na tabelę (`DataTable`).
- Bez zapisu etykiet w bazie aplikacji i bez migracji.
- Bez nowych wywołań `gh` i bez zmian innych argumentów niż pole `--json`.
- Bez zmiany artefaktu design systemu w tej zmianie. Nowy prop `ItemRow.labels` faza 2 dopisuje do zakresu F-05 (`design-system-artifact-sync`) w roadmapie, a sam artefakt aktualizuje F-05.

## Implementation Approach

Dwie fazy: najpierw dane, potem widok. Faza 1 zmienia wywołanie, parsowanie, typ Rust i jego mirror TS w jednym commicie (kontrakt Rust ↔ TS z AGENTS.md). Do tego dochodzą `fake-cli` i testy. Po fazie 1 etykiety są już w snapshocie, ale UI ich nie pokazuje, więc nic widocznego się nie psuje. Faza 2 dodaje opcjonalny prop do `ItemRow` (bez propa wiersz wygląda jak dziś, więc `OwnTasksView` i `TodoView` się nie zmieniają), przekazuje etykiety z `GithubView` i chowa je w wąskim oknie przez CSS widoku.

## Faza 1: Etykiety w danych GitHuba

### Overview

`gh issue list` pobiera `labels`. `parse_issues` wyciąga nazwy etykiet do `GithubIssue.labels`, a mirror TS dostaje to samo pole. Strażnik, oba testy argumentów, `fake-cli` i testy parsowania i pobierania znają nowy kształt.

### Changes Required:

#### 1. Stałe wywołanie i strażnik

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Dopisać `labels` do pola `--json` w wywołaniu oraz w obu jego kopiach testowych (strażnik i test dokładnych argumentów), żeby nowy kształt był jedynym dozwolonym.

**Contract**: Ostatni argument `gh_assigned_issues_args` (`:198-199`), ostatni element `GH_ISSUES_TAIL` (`:713-722`) i ostatni element oczekiwanej listy w `gh_assigned_issues_has_exact_arguments_name_and_tool` (`:1105`) to dokładnie `number,title,url,updatedAt,labels`. Liczba argumentów zostaje 12, a pozostałe argumenty i `GH_ISSUE_LIMIT` się nie zmieniają. `guard_rejects_bad_repos_and_altered_gh_shapes` przechodzi bez zmian, a stary ogon (bez `labels`) jest już odrzucany przez strażnika.

#### 2. Test argv przez prawdziwy proces

**File**: `src-tauri/tests/cli_runner.rs`

**Intent**: Oczekiwany argv w teście `gh issue list` (`:279-293`) odpowiada nowemu wywołaniu.

**Contract**: Ostatni element oczekiwanej tablicy to `number,title,url,updatedAt,labels`.

#### 3. Parsowanie i model zgłoszenia

**File**: `src-tauri/src/github/mod.rs`

**Intent**: `GithubIssue` niesie nazwy etykiet w kolejności z GitHuba; `RawIssue` czyta je z obiektów `gh`. Brak pola to pusta lista (jak w GitLabie), a zły kształt to `ResponseInvalid` dla tego repozytorium, bez treści odpowiedzi w błędzie.

**Contract**:
- `GithubIssue` dostaje `pub labels: Vec<String>` (serializowane jako `labels`), z komentarzem: nazwy etykiet w kolejności z GitHuba, pusta lista, gdy brak.
- `RawIssue` dostaje `#[serde(default)] labels: Vec<RawLabel>`, gdzie `RawLabel { name: String }` (prywatny, inne pola obiektu są ignorowane).
- `parse_issues` przepisuje `name` każdej etykiety bez zmian.
- `labels` niebędące tablicą, element niebędący obiektem oraz obiekt bez `name` albo z `name` innego typu niż tekst dają `ResponseInvalid`.
- Pomocnik testowy `issue()` (`:475-484`) dostaje `labels: vec![]`.

#### 4. Testy jednostkowe parsowania

**File**: `src-tauri/src/github/mod.rs` (moduł `tests`)

**Intent**: Pokryć nowe pole i jego tryby błędu wzorem istniejących testów.

**Contract**: Testy (nazwy dowolne, opisowe):
- etykiety są czytane w kolejności z odpowiedzi, a dodatkowe pola obiektu (`id`, `color`, `description`) są ignorowane;
- brak pola `labels` daje pustą listę, a `"labels":[]` też pustą;
- `invalid_output_is_response_invalid_and_leaks_nothing` dostaje trzy nowe przypadki z `MARKER` w odpowiedzi: `"labels":"<MARKER>"`, `"labels":[{"color":"<MARKER>"}]` (brak `name`) i `"labels":[{"name":1}]`;
- test serializacji snapshotu sprawdza klucz `labels` w JSON zgłoszenia.

Pomocnik `issue_json` dostaje wariant z etykietami albo osobny pomocnik, do wyboru implementującego.

#### 5. Mirror TS

**File**: `src/github/types.ts`

**Intent**: Mirror `GithubIssue` zgadza się z serde (ten sam commit co zmiana Rust).

**Contract**: `GithubIssue` dostaje `labels: string[]` z komentarzem JSDoc: nazwy etykiet w kolejności z GitHuba, pusta tablica, gdy brak.

#### 6. Konstruktory `GithubIssue` w testach „Do zrobienia”

**Files**: `src-tauri/src/todo/commands.rs` (`:239`), `src-tauri/src/todo/tests.rs` (`:609-618`), `src-tauri/tests/todo.rs` (`:232-241`), `src-tauri/tests/todo_crash.rs` (`:49-58`)

**Intent**: Dopisać brakujące pole, żeby testy się kompilowały. Bez zmiany zachowania.

**Contract**: Każdy literał `GithubIssue { … }` ma `labels: vec![]`.

#### 7. `fake-cli`: wymyślone etykiety

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Scenariusz `gh-issues` zwraca etykiety w kształcie, jaki wysyła `gh`, w tym jedną z markerem wycieku, żeby test integracyjny mógł sprawdzić, że etykieta jest danymi listy i niczym więcej.

**Contract**:
- `gh_issue_value` przyjmuje listę nazw i zwraca `labels` jako obiekty `{"id", "name", "description", "color"}` z wymyślonymi wartościami.
- `octo-test/alpha#11` ma `["bug", "needs-triage"]`, a `octo-test/alpha#3` ma jedną etykietę z nowym markerem `GH_LABEL_MARKER` (np. `LEAK-MARKER-GH-LABEL-<4 hex>`) obok istniejącego `GH_TITLE_MARKER`.
- `octo-test/huge` ma puste etykiety. `garbled`, `missing` i `beta` się nie zmieniają.

#### 8. Test integracyjny pobierania

**File**: `src-tauri/tests/github_fetch.rs`

**Intent**: Etykiety przechodzą od procesu do `GithubIssue` i nie wyciekają poza listę.

**Contract**:
- `alpha_and_beta_load_in_settings_order` sprawdza `alpha[0].labels == ["bug", "needs-triage"]`.
- `a_loaded_title_reaches_the_issue_list_and_nothing_else` dostaje `LABEL_MARKER` (ta sama wartość co w `fake-cli`) i sprawdza, że marker jest w `labels` zgłoszenia alpha, a nie ma go w żadnym z już sprawdzanych tekstów (stan, status tray, błąd repozytorium).

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi (w tym `every_call_is_read_only`, `gh_assigned_issues_has_exact_arguments_name_and_tool`, test argv w `cli_runner.rs`, nowe testy parsowania i `github_fetch.rs`)
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzą
- `npm run build` przechodzi (mirror TS się kompiluje)
- `npm run lint` i `npm run format:check` przechodzą
- Stary ogon bez `labels` nie występuje w `src-tauri/` (`rg -F 'number,title,url,updatedAt"' src-tauri` nic nie zwraca)

**Implementation Note**: Po zielonej automatycznej weryfikacji ta faza nie ma kroków ręcznych; przejdź do fazy 2.

---

## Faza 2: Etykiety w wierszu „Projekty (GitHub)”

### Overview

`ItemRow` dostaje opcjonalny slot `labels`. `GithubView` przekazuje do niego etykiety zgłoszenia, a CSS widoku chowa je poniżej 900 px. Nowy prop trafia do zakresu F-05 w roadmapie.

### Changes Required:

#### 1. Slot etykiet we wspólnym wierszu

**File**: `src/components/ItemRow/ItemRow.tsx`

**Intent**: Wiersz może pokazać listę etykiet jako tekst, w tym samym miejscu co kolumna etykiet w GitLabie (po tytule i fladze, przed meta). Bez propa albo z pustą listą wiersz wygląda dokładnie jak dziś.

**Contract**:
- `RowBase` dostaje `labels?: string[]` z komentarzem: nazwy etykiet, pokazywane po przecinku, bez renderu przy braku lub pustej liście.
- Przy niepustej liście renderowany jest `<span className="dn-row-labels code" title={joined}>` z ukrytym prefiksem dla czytnika ekranu (`<span className="dn-sr-only">Etykiety: </span>`) i tekstem `labels.join(", ")`, wstawiony między flagą a meta.

#### 2. Styl slotu

**File**: `src/components/ItemRow/ItemRow.css`

**Intent**: Etykiety wyglądają jak komórka etykiet GitLaba i ustępują miejsca tytułowi.

**Contract**: `.dn-row-labels` ma font 11 px, kolor `var(--ink-faint)`, `white-space: nowrap`, `overflow: hidden`, `text-overflow: ellipsis`, `flex: 0 1 auto`, `min-width: 0` i `max-width: 150px` (szerokość kolumny „Etykiety” w GitLabie). Bez literalnych kolorów.

#### 3. Widok GitHuba

**File**: `src/github/GithubView.tsx`

**Intent**: `IssueRow` przekazuje etykiety zgłoszenia do wiersza.

**Contract**: `<ItemRow … labels={issue.labels}>`. Bez innych zmian w wierszu.

#### 4. Wąskie okno

**File**: `src/github/GithubView.css`

**Intent**: Poniżej 900 px etykiety znikają, tak jak kolumna „Etykiety” w GitLabie. Tytuł zostaje czytelny.

**Contract**: `@media (max-width: 900px) { .github-repos .dn-row-labels { display: none; } }`. To tylko layout, bez wartości wizualnych.

#### 5. Zakres F-05 w roadmapie

**File**: `context/foundation/roadmap.md`

**Intent**: Artefakt design systemu nie zna nowego propa. Dopisanie go do listy w Outcome F-05 sprawia, że synchronizacja artefaktu z aplikacją go obejmie, zamiast zostawić rozjazd.

**Contract**: W `### F-05` → `- **Outcome:**` lista po „match the app:” dostaje pozycję `` `ItemRow` labels `` zaraz po `` `ItemRow` id ``. Inne pola F-05 i innych elementów zostają bez zmian.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą
- `rg -n "dn-row-labels" src` znajduje tylko `ItemRow.tsx`, `ItemRow.css` i `GithubView.css`
- Outcome F-05 w roadmapie wymienia `ItemRow` labels (`rg -F '`ItemRow` labels' context/foundation/roadmap.md` zwraca linię F-05)

#### Manual Verification:

- Przy 1280 px etykiety są po przecinku między tytułem a wiekiem, podpowiedź pokazuje pełną listę
- Zgłoszenie bez etykiet wygląda jak przed zmianą
- Długa lista etykiet ucina się wielokropkiem, tytuł zostaje czytelny
- Przy 800 px etykiet nie ma, wiersz wygląda jak przed zmianą
- „Własne zadania” i „Do zrobienia” wyglądają jak przed zmianą

**Implementation Note**: Po zielonej automatycznej weryfikacji zatrzymaj się na ręczne potwierdzenie użytkownika przed zamknięciem zmiany.

---

## Testing Strategy

### Unit Tests:

- Parsowanie etykiet: kolejność, ignorowane pola, brak pola, pusta tablica.
- Tryby błędu: zły typ `labels`, element bez `name` i `name` nie-tekst dają `ResponseInvalid`, a marker z odpowiedzi nie trafia do `Debug`, `Display`, JSON ani `message_pl`.
- Strażnik i test dokładnych argumentów z nowym ogonem.

### Integration Tests:

- `github_fetch.rs`: etykiety z `fake-cli` docierają do `GithubIssue`; marker w etykiecie jest w liście, a nie ma go w stanie, tray ani błędzie.
- `cli_runner.rs`: argv procesu `gh` to dokładnie nowy kształt.

### Manual Testing Steps:

1. Ustaw w `settings.json` repozytorium z otwartym zgłoszeniem przypisanym do Ciebie z co najmniej dwiema etykietami oraz jednym bez etykiet. Ustaw prawdziwe własne repo albo testowe, nigdy firmowe.
2. Uruchom `npm run tauri dev`, otwórz „Projekty (GitHub)” przy 1280 px i sprawdź tekst i podpowiedź etykiet.
3. Zwęź okno do 800 px i sprawdź, że etykiety zniknęły.
4. Otwórz „Własne zadania” i „Do zrobienia” i sprawdź, że wiersze się nie zmieniły.

## Performance Considerations

Brak istotnych: jedno dodatkowe pole w tej samej odpowiedzi `gh`, bez nowych wywołań.

## Migration Notes

Brak: etykiety żyją tylko w snapshocie w pamięci; baza aplikacji i `settings.json` się nie zmieniają. Wycofanie = revert obu commitów.

## References

- Roadmap: `context/foundation/roadmap.md` → S-04 `github-issue-labels`
- PRD: `context/foundation/prd.md` → FR-025
- Wzorzec etykiet w GitLabie: `src/gitlab/GitlabView.tsx:155,190-197`, `src/gitlab/GitlabView.css:35,42-44,79-88`, `src-tauri/src/gitlab/mod.rs:204,574-589`
- Poprzednia zmiana widoku GitHuba: `context/archive/2026-11-23-github-assigned-issues/`
- Wspólny wiersz: `src/components/ItemRow/ItemRow.tsx`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Etykiety w danych GitHuba

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi (w tym `every_call_is_read_only`, `gh_assigned_issues_has_exact_arguments_name_and_tool`, test argv w `cli_runner.rs`, nowe testy parsowania i `github_fetch.rs`) — 2db6c3f
- [x] 1.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzą — 2db6c3f
- [x] 1.3 `npm run build` przechodzi (mirror TS się kompiluje) — 2db6c3f
- [x] 1.4 `npm run lint` i `npm run format:check` przechodzą — 2db6c3f
- [x] 1.5 Stary ogon bez `labels` nie występuje w `src-tauri/` (`rg -F 'number,title,url,updatedAt"' src-tauri` nic nie zwraca) — 2db6c3f

### Phase 2: Etykiety w wierszu „Projekty (GitHub)”

#### Automated

- [x] 2.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — a0a9534
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą — a0a9534
- [x] 2.3 `rg -n "dn-row-labels" src` znajduje tylko `ItemRow.tsx`, `ItemRow.css` i `GithubView.css` — a0a9534
- [x] 2.9 Outcome F-05 w roadmapie wymienia `ItemRow` labels (`rg -F '`ItemRow` labels' context/foundation/roadmap.md` zwraca linię F-05) — a0a9534

#### Manual

- [ ] 2.4 Przy 1280 px etykiety są po przecinku między tytułem a wiekiem, podpowiedź pokazuje pełną listę
- [ ] 2.5 Zgłoszenie bez etykiet wygląda jak przed zmianą
- [ ] 2.6 Długa lista etykiet ucina się wielokropkiem, tytuł zostaje czytelny
- [ ] 2.7 Przy 800 px etykiet nie ma, wiersz wygląda jak przed zmianą
- [ ] 2.8 „Własne zadania” i „Do zrobienia” wyglądają jak przed zmianą
