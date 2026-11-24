# „+” do „Do zrobienia” z widoków źródeł — plan implementacji

## Overview

W widokach „Praca (GitLab)” i „Projekty (GitHub)” każdy wiersz dostaje przycisk „+”, który dodaje pozycję do „Do zrobienia”. Przy osiągniętym limicie przycisk jest zablokowany z tą samą podpowiedzią co w pickerze. Pozycja już dodana ma flagę W DO ZROBIENIA i wypełniony (bursztynowy) przycisk, którego kliknięcie zdejmuje ją z listy. Zamyka to FR-026 / issue #8 (slice S-01, M-2).

## Current State Analysis

- Komendy `add_todo_gitlab_item` / `add_todo_github_item` i funkcje `addTodoGitlabItem` / `addTodoGithubItem` (`src/todo/api.ts:14-20`) już istnieją; używa ich tylko picker w `TodoView.tsx`.
- Rust ma zdarzenie `todo-changed`, ale emitują je tylko rundy odświeżania (`src-tauri/src/gitlab_refresh.rs:53,549`, `src-tauri/src/github_refresh.rs:31,201`), każda z własną stałą `TODO_EVENT`. Komendy użytkownika (`src-tauri/src/todo/commands.rs`, `run()`) zwracają świeżą listę tylko wywołującemu i niczego nie emitują.
- `useTodo` (`src/todo/useTodo.ts`) wczytuje listę, nasłuchuje `todo-changed`, przeładowuje przy fokusie/widoczności i o północy, oraz serializuje mutacje (`mutate`). Widoki źródeł mogą go użyć bez nowego pollera.
- Reguły w Rust (`src-tauri/src/todo/mod.rs`): `check_limit` liczy otwarte (nieodhaczone) pozycje dnia; `AlreadyOnList` dotyczy każdej „żywej” pozycji (nie zdjętej i nie zamkniętej — także odhaczonej dziś); `find_gitlab_item` (`commands.rs:39`) szuka tylko w MR-ach i widocznych zgłoszeniach, więc ukryte zgłoszenie jest odrzucane jako `NotInGitlabList`.
- Picker (`src/todo/TodoView.tsx`) ma klucze `gitlabKey` / `githubKey` / `entryKey` oraz lokalne `addButton` (quiet, ikona `plus`, `disabled` przy `busy || atLimit`, `title` = `Limit N otwartych pozycji`).
- Design system Konsola: przycisk pozycji już dodanej = `Button variant="primary" iconOnly pressed`, „nadal klikalny (zdejmuje z listy)”; flaga `W DO ZROBIENIA` w `tone="change"`; accent wolno użyć m.in. dla „pozycji już dodanej do Do zrobienia (wypełniony przycisk +)”.
- `GitlabView` ma tabelę w gridzie (`src/gitlab/GitlabView.css`, kolumna akcji 96px) z akcjami tylko dla zgłoszeń; `GithubView` renderuje `ItemRow` z przyciskiem „Otwórz na GitHubie”.

## Desired End State

- W „Praca (GitLab)” każdy MR i każde widoczne zgłoszenie mają „+”. Ukryty wiersz ma flagę i wypełniony przycisk tylko wtedy, gdy jest na liście. Blok „Zamknięte” zostaje bez akcji.
- W „Projekty (GitHub)” każde zgłoszenie ma „+” obok „Otwórz na GitHubie”.
- Pozycja na dzisiejszej liście, także odhaczona, ma flagę W DO ZROBIENIA i wypełniony przycisk (`aria-pressed`), którego klik zdejmuje ją z listy.
- Przy limicie „+” jest wyłączony z podpowiedzią `Limit N otwartych pozycji`, a wypełniony przycisk zostaje aktywny.
- Gdy listy nie da się wczytać, widok pokazuje `ErrorState` z ponowieniem, a przyciski są wyłączone. Błąd akcji i `limitError` widać jako `Notice`.
- Każda udana mutacja listy z dowolnej komendy emituje `todo-changed`, więc każdy słuchacz `useTodo` widzi zmianę.

Weryfikacja: kontrole sprzed commita z `AGENTS.md` oraz ręczny przebieg w `npm run tauri dev`.

### Key Discoveries:

- `src-tauri/src/todo/commands.rs:68-101` (`run`): jedno miejsce, przez które przechodzą wszystkie komendy listy; tu wpinamy emit.
- `src/todo/useTodo.ts:94` już nasłuchuje `todo-changed`; po emisji z komend widok, który wykonał akcję, przeładuje listę raz więcej (tani `list_todo`), co `mutate` już toleruje (porównanie `latest`).
- `src/todo/TodoView.tsx` (`gitlabKey`, `githubKey`, `entryKey`, `addButton` w `Picker`) to wzorzec kluczy i podpowiedzi do współdzielenia.
- Zwykła mutacja (`add_*`, `remove`, `set_ticked`) zapisuje we własnym `store.write`, a `list` w osobnym, więc zmiana może być zatwierdzona, nawet gdy odczyt listy po niej zawiedzie.

## What We're NOT Doing

- Żadnych zmian w regułach limitu, przenoszenia i odhaczania ani w schemacie store.
- Żadnego „+” w bloku „Zamknięte” GitLaba ani „+” dla ukrytych zgłoszeń spoza listy (backend je odrzuca).
- Bez osobnej flagi „ZROBIONE” dla odhaczonych pozycji — wyglądają jak pozycje na liście.
- Bez potwierdzenia przed zdjęciem pozycji wypełnionym przyciskiem.
- Bez zmian w `src-tauri/src/vault/`, health, `ci.yml`, `Cargo.toml` (wersja) i w rejestracji komend w `lib.rs`.
- Bez test runnera dla UI (repo go nie ma).

## Implementation Approach

Zdarzenie zmiany listy już istnieje, więc wystarczy, żeby emitowały je również komendy użytkownika. Wtedy każdy widok z `useTodo()` zna aktualny stan. Logikę przycisku (klucz pozycji, stan „na liście”, limit, podpowiedź, `aria`) wydzielamy do `src/todo`, żeby picker i oba widoki źródeł miały jedno zachowanie. UI tylko wyświetla wynik i wysyła akcje; o limicie i duplikatach rozstrzyga Rust (odmowa wraca jako `Notice` z `todoErrorMessage`).

## Critical Implementation Details

- **Kolejność emisji:** `todo-changed` emitujemy, gdy mutacja `f` się powiodła, także jeśli następny odczyt listy zawiódł, bo zmiana jest już zatwierdzona. `list_todo` nie emituje nigdy, inaczej słuchacz przeładowujący listę wpadłby w pętlę.

## Phase 1: Zdarzenie z komend (Rust)

### Overview

Każda udana mutacja listy z komendy emituje `todo-changed`, a nazwa zdarzenia staje się jedną stałą.

### Changes Required:

#### 1. Stała nazwy zdarzenia

**File**: `src-tauri/src/todo/mod.rs`, `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/github_refresh.rs`

**Intent**: Jedna publiczna stała zamiast dwóch prywatnych kopii napisu, żeby komendy i rundy odświeżania nie mogły się rozjechać.

**Contract**: `pub const CHANGED_EVENT: &str = "todo-changed";` w module `todo`. Lokalne `TODO_EVENT` w obu plikach refresh znikają i są zastąpione `crate::todo::CHANGED_EVENT`. Zachowanie się nie zmienia.

#### 2. Emisja z komend mutujących

**File**: `src-tauri/src/todo/commands.rs`

**Intent**: `run()` wie, czy wywołanie mutuje listę. Po udanej mutacji emituje `CHANGED_EVENT`. Błąd emisji logujemy jako warn bez danych, jak w refresh, i nie przerywamy komendy.

**Contract**: `run` dostaje rozróżnienie listy i mutacji (parametr albo osobny wariant). `list_todo` nie emituje, a `add_todo_own_task`, `add_todo_gitlab_item`, `add_todo_github_item`, `set_todo_ticked` i `remove_todo` emitują, gdy `f` zwróciło `Ok`, niezależnie od wyniku następnego `list`. Sygnatury komend i ich typy wyjścia się nie zmieniają.

### Success Criteria:

#### Automated Verification:

- `cargo fmt --check` w `src-tauri/` przechodzi
- `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzi
- `cargo test` w `src-tauri/` przechodzi
- W `src-tauri/src` nie ma już literału `"todo-changed"` poza `todo::CHANGED_EVENT` (`rg '"todo-changed"' src-tauri/src` daje jedno trafienie)

#### Manual Verification:

- Odhaczenie pozycji w „Do zrobienia” nadal działa i lista nie miga ani nie przeładowuje się w pętli

**Implementation Note**: Po zielonych kontrolach automatycznych przejdź dalej. Kontrole ręczne wszystkich faz zbieramy na końcu w jednym przebiegu `npm run tauri dev`.

---

## Phase 2: Wspólne elementy „Do zrobienia” (UI)

### Overview

Klucze pozycji i przycisk „+”/wypełniony trafiają do `src/todo`, a picker używa nowego przycisku bez zmiany swojego zachowania.

### Changes Required:

#### 1. Klucze pozycji

**File**: `src/todo/keys.ts` (nowy), `src/todo/TodoView.tsx`

**Intent**: Przenieść `gitlabKey`, `githubKey`, `entryKey` z `TodoView.tsx` i dodać pomocnik, który z `TodoList` buduje mapę klucz → `TodoItem`. Widok źródła z jednego wyszukania dostaje i stan „na liście”, i `id` do zdjęcia.

**Contract**: `gitlabKey(kind, reference)`, `githubKey(reference)` (normalizuje do lowercase, jak Rust), `entryKey(entry)`, `itemsByKey(list): Map<string, TodoItem>`. `TodoView` importuje je stamtąd i jego zachowanie się nie zmienia.

#### 2. Przycisk dodawania

**File**: `src/todo/TodoAddButton.tsx` (nowy), `src/todo/TodoView.tsx`

**Intent**: Jeden komponent przycisku pozycji. Poza listą to quiet „+”, wyłączony przy `busy` lub limicie, z podpowiedzią `Limit N otwartych pozycji`. Na liście to `primary` + `pressed`, aktywny także przy limicie, a jego klik zdejmuje pozycję. Picker zastępuje nim lokalne `addButton`.

**Contract**: propsy co najmniej: `label` (tytuł pozycji), `onList: boolean`, `atLimit: boolean`, `limit: number | null`, `disabled: boolean` (busy lub brak listy), `disabledHint?: string`, `onAdd`, `onRemove?`. `aria-label`: „Dodaj do Do zrobienia: <tytuł>” / „Zdejmij z Do zrobienia: <tytuł>”. Ikona `plus` w obu stanach. Bez nowego CSS, bo warianty `primary` i `is-pressed` już istnieją w `Button.css`.

#### 3. Stan listy dla widoków źródeł

**File**: `src/todo/useTodoPicks.ts` (nowy)

**Intent**: Cienka warstwa nad `useTodo()` dla widoków źródeł. Daje lookup pozycji po kluczu, `atLimit`/`limit`, `busy`, ostatni komunikat błędu akcji oraz `add`/`remove`, które ustawiają `busy` i komunikat (wzorzec `act` z `TodoView`). Udostępnia też `load`/`reloadError`/`retry`, żeby widok wyrenderował błąd listy.

**Contract**: zwraca `{ load, reloadError, retry, refresh, lastSuccessAt, find(key): TodoItem | undefined, atLimit, limit, limitError, busy, notice, addGitlab(kind, ref), addGithub(ref), remove(id) }`. `atLimit` liczymy tak jak w `TodoView` (`limit === null || openCount >= limit`). Tylko wyświetlanie, reguły zostają w Rust.

### Success Criteria:

#### Automated Verification:

- `npm run lint` przechodzi
- `npm run format:check` przechodzi
- `npm run build` przechodzi bez błędów

#### Manual Verification:

- Picker w „Do zrobienia” dodaje pozycje jak wcześniej, a przy limicie „+” jest wyłączony z podpowiedzią `Limit N otwartych pozycji`

---

## Phase 3: „+” w widokach źródeł

### Overview

`GitlabView` i `GithubView` pokazują przycisk i flagę W DO ZROBIENIA, błędy listy oraz akcji.

### Changes Required:

#### 1. GitLab

**File**: `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`

**Intent**: `GitlabView` woła `useTodoPicks()`. W tabeli „Przypisane” każdy wiersz MR i widocznego zgłoszenia dostaje `TodoAddButton` w kolumnie akcji, obok istniejących przycisków zgłoszeń. Wiersz ukryty dostaje przycisk tylko w stanie „na liście” (do zdjęcia). Pozycja na liście ma w komórce tytułu `<Flag tone="change">W Do zrobienia</Flag>` (CSS robi wersaliki). Tabela „Zamknięte” bez zmian. Kolumna akcji poszerzona, żeby zmieściła trzy przyciski (OK zauważone, ukryj, +).

**Contract**: `RowActions`/`GitlabRow` dostają dane o „Do zrobienia” (lookup i handlery), przekazywane tylko do tabeli „Przypisane”. Kolumna akcji ma się mieścić w 3 × 40px plus odstępy. Wartość wpisz w `grid-template-columns`, bez literałów kolorów. Nad listą: `Notice tone="error"` z komunikatem akcji, `Notice` z `todoErrorMessage(limitError)` oraz `ErrorState` przy `load.state === "error"` (tytuł „Nie udało się wczytać listy Do zrobienia”, ponowienie `retry`) lub przy `reloadError` (lista może być nieaktualna, ponowienie `refresh`). Gdy lista się nie wczytała, przyciski są wyłączone z podpowiedzią „Lista Do zrobienia niewczytana”.

#### 2. GitHub

**File**: `src/github/GithubView.tsx`

**Intent**: `GithubView` woła `useTodoPicks()`. `IssueRow` dostaje flagę przez `flag`/`flagTone` z `ItemRow` i `TodoAddButton` przed przyciskiem „Otwórz na GitHubie”. Komunikaty i błąd listy działają jak w GitLabie.

**Contract**: `RepoGroup`/`IssueRow` przyjmują lookup i handlery. Klucz to `githubKey(issue.reference)`. Bez nowego CSS widoku, chyba że trzeba wyrównać akcje.

### Success Criteria:

#### Automated Verification:

- `npm run lint` przechodzi
- `npm run format:check` przechodzi
- `npm run build` przechodzi bez błędów
- `cargo test` w `src-tauri/` nadal przechodzi

#### Manual Verification:

- W „Praca (GitLab)” „+” na MR i zgłoszeniu dodaje je do „Do zrobienia”, a wiersz od razu ma flagę W DO ZROBIENIA i wypełniony przycisk
- W „Projekty (GitHub)” „+” dodaje zgłoszenie, a flaga i wypełniony przycisk pojawiają się od razu
- Klik w wypełniony przycisk zdejmuje pozycję; flaga znika, a pozycji nie ma w „Do zrobienia”
- Przy pełnym limicie „+” jest wyłączony z podpowiedzią `Limit N otwartych pozycji`, a wypełnione przyciski nadal zdejmują
- Pozycja odhaczona w „Do zrobienia” nadal ma w widoku źródła flagę i wypełniony przycisk
- Kolumna akcji GitLaba mieści trzy przyciski bez zawijania

---

## Testing Strategy

### Unit Tests:

- Istniejące testy Rust `todo` (limit, `AlreadyOnList`, roll-over) zostają zielone. Reguły się nie zmieniają, więc nie dochodzą nowe testy domeny.
- Emisja zdarzenia to okablowanie Tauri (wymaga `AppHandle`), więc sprawdzamy ją ręcznie.

### Integration Tests:

- Brak nowych. UI nie ma test runnera.

### Manual Testing Steps:

1. `npm run tauri dev`, przy ustawionym limicie (domyślnie 5) i skonfigurowanym GitLabie/GitHubie z wymyślonymi lub własnymi prywatnymi danymi.
2. W GitLabie dodaj MR i zgłoszenie przez „+”, sprawdź flagę i wypełniony przycisk, przejdź do „Do zrobienia” i sprawdź obie pozycje.
3. Odhacz jedną w „Do zrobienia”, wróć do GitLaba: flaga zostaje.
4. Dobij do limitu z GitHuba: „+” jest wyłączony z podpowiedzią, wypełniony przycisk zdejmuje pozycję i „+” znów działa.
5. Ukryj zgłoszenie, które jest na liście, i włącz „Pokaż ukryte”: wiersz ma flagę i wypełniony przycisk.

## Performance Considerations

Każda mutacja daje jedno dodatkowe `list_todo` w widoku, który ją wykonał (po zdarzeniu). To pomijalne: lokalny SQLite, kilka wierszy.

## Migration Notes

Brak: bez zmian schematu i danych.

## References

- Issue: GitHub #8
- PRD: `context/foundation/prd.md` FR-026, FR-011, FR-012, US-01
- Roadmap: `context/foundation/roadmap.md` `### S-01`
- Wzorzec pickera: `src/todo/TodoView.tsx` (`Picker`, `addButton`)
- Zdarzenie: `src-tauri/src/gitlab_refresh.rs:549`, `src/todo/useTodo.ts:94`
- Design system: https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY (Button, Flag)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Zdarzenie z komend (Rust)

#### Automated

- [x] 1.1 `cargo fmt --check` w `src-tauri/` przechodzi — 353e93d
- [x] 1.2 `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzi — 353e93d
- [x] 1.3 `cargo test` w `src-tauri/` przechodzi — 353e93d
- [x] 1.4 W `src-tauri/src` nie ma już literału `"todo-changed"` poza `todo::CHANGED_EVENT` — 353e93d

#### Manual

- [x] 1.5 Odhaczenie pozycji w „Do zrobienia” nadal działa i lista nie miga ani nie przeładowuje się w pętli — 353e93d

### Phase 2: Wspólne elementy „Do zrobienia” (UI)

#### Automated

- [x] 2.1 `npm run lint` przechodzi — 6db27d5
- [x] 2.2 `npm run format:check` przechodzi — 6db27d5
- [x] 2.3 `npm run build` przechodzi bez błędów — 6db27d5

#### Manual

- [x] 2.4 Picker w „Do zrobienia” dodaje pozycje jak wcześniej, a przy limicie „+” jest wyłączony z podpowiedzią — 6db27d5

### Phase 3: „+” w widokach źródeł

#### Automated

- [x] 3.1 `npm run lint` przechodzi — 6ee8a46
- [x] 3.2 `npm run format:check` przechodzi — 6ee8a46
- [x] 3.3 `npm run build` przechodzi bez błędów — 6ee8a46
- [x] 3.4 `cargo test` w `src-tauri/` nadal przechodzi — 6ee8a46

#### Manual

- [x] 3.5 W „Praca (GitLab)” „+” na MR i zgłoszeniu dodaje je, a wiersz ma flagę i wypełniony przycisk — 6ee8a46
- [x] 3.6 W „Projekty (GitHub)” „+” dodaje zgłoszenie, flaga i wypełniony przycisk pojawiają się od razu — 6ee8a46
- [x] 3.7 Klik w wypełniony przycisk zdejmuje pozycję z „Do zrobienia” — 6ee8a46
- [x] 3.8 Przy pełnym limicie „+” jest wyłączony z podpowiedzią, a wypełnione przyciski nadal zdejmują — 6ee8a46
- [x] 3.9 Pozycja odhaczona w „Do zrobienia” nadal ma w widoku źródła flagę i wypełniony przycisk — 6ee8a46
- [x] 3.10 Kolumna akcji GitLaba mieści trzy przyciski bez zawijania — 6ee8a46
