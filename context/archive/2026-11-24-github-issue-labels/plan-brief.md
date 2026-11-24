# Etykiety zgłoszeń GitHub — Plan Brief

> Full plan: `context/changes/github-issue-labels/plan.md`

## What & Why

W „Projekty (GitHub)” użytkownik widzi przy każdym zgłoszeniu jego etykiety jako tekst, tak jak w „Praca (GitLab)”. To wycinek S-04 z roadmapy M-2 (issue #7, PRD FR-025). Dziś zgłoszenia GitHub nie mają etykiet, więc oba widoki źródeł mówią różnie dużo o tej samej pracy.

## Starting Point

Stałe wywołanie `gh issue list … --json number,title,url,updatedAt` (`src-tauri/src/cli/mod.rs`) nie pobiera etykiet, a `GithubIssue` ich nie ma. Widok GitHuba buduje wiersze ze wspólnego `ItemRow` (nr, tytuł, flaga, wiek, akcje), który nie ma miejsca na etykiety. GitLab pokazuje je w kolumnie tabeli: tekst po przecinku, 11 px, ucinanie z podpowiedzią, kolumna znika poniżej 900 px.

## Desired End State

Każdy wiersz zgłoszenia GitHub pokazuje etykiety po przecinku między tytułem a wiekiem. Tekst jest mono i przyciemniony, długą listę ucina wielokropek, a pełna lista jest w podpowiedzi. Zgłoszenie bez etykiet wygląda jak dziś. W wąskim oknie (poniżej 900 px) etykiety znikają. Wywołanie `gh` różni się tylko polem `labels`, dalej tylko czyta i przechodzi test strażnika.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Miejsce w wierszu | Nowy opcjonalny prop `labels` we wspólnym `ItemRow` | Wygląd jak kolumna GitLaba, styl w CSS komponentu; bez propa inne widoki się nie zmieniają. |
| Wąskie okno | Etykiety chowane poniżej 900 px | Tak samo jak w GitLabie; tytuł zostaje czytelny. |
| Zakres | Tylko „Projekty (GitHub)”, bez „Do zrobienia” | FR-025 mówi tylko o tym widoku; GitLab też nie ma etykiet w „Do zrobienia”. |
| Kolejność etykiet | Jak przysłał GitHub | GitLab też nie sortuje; brak powodu do własnej reguły. |
| Brak pola / zły kształt | Brak → pusta lista; zły typ lub brak `name` → `ResponseInvalid` dla repozytorium | To samo zachowanie co parser GitLaba; błąd nie niesie treści odpowiedzi. |
| Kontrakt Rust ↔ TS | `GithubIssue.labels` i mirror TS w jednym commicie (faza 1) | Wymóg AGENTS.md. |

## Scope

**In scope:**
- pole `labels` w stałym wywołaniu `gh issue list` i w trzech testach, które sprawdzają pełną listę argumentów (strażnik, dokładne argumenty, argv procesu);
- parsowanie nazw etykiet do `GithubIssue.labels` i mirror w `src/github/types.ts`;
- wymyślone etykiety w `fake-cli` (w tym marker wycieku) i testy jednostkowe i integracyjne;
- prop `labels` w `ItemRow` z CSS komponentu, użycie w `GithubView`, chowanie poniżej 900 px;
- dopisanie `ItemRow` labels do zakresu F-05 w roadmapie.

**Out of scope:**
- etykiety w „Do zrobienia” i w pickerze;
- kolory i opisy etykiet, filtrowanie i sortowanie po etykietach;
- przepisanie widoku GitHuba na tabelę;
- zapis etykiet w bazie i migracje;
- aktualizacja samego artefaktu design systemu (robi to F-05).

## Architecture / Approach

`gh` → `CliRunner` → `parse_issues` (nowe `RawLabel { name }`, `#[serde(default)]`) → `GithubIssue.labels: Vec<String>` → snapshot (bez zmian w logice) → `types.ts` → `IssueRow` → `ItemRow labels` → `<span class="dn-row-labels code">` z prefiksem „Etykiety:” dla czytnika ekranu. Widok GitHuba dokłada tylko regułę layoutu dla progu 900 px.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Etykiety w danych GitHuba | Wywołanie z `labels`, strażnik i oba testy argumentów, parsowanie, `GithubIssue.labels` + mirror TS, `fake-cli`, testy | Pominięcie jednej z czterech kopii argumentów, ale testy to od razu wyłapią. |
| 2. Etykiety w wierszu „Projekty (GitHub)” | Prop `labels` w `ItemRow`, użycie w `GithubView`, chowanie poniżej 900 px, wpis w zakresie F-05, przegląd ręczny | Zmiana wspólnego komponentu mogłaby ruszyć „Własne zadania” lub „Do zrobienia”; prop jest opcjonalny, a przegląd to sprawdza. |

**Prerequisites:** brak (S-04 nie ma zależności); `gh` zalogowany w Windows do ręcznego przeglądu.
**Estimated effort:** ~1 sesja, 2 fazy.

## Open Risks & Assumptions

- Kształt etykiet z `gh` (tablica obiektów z `name`) sprawdzony 2026-11-24 na `gh` 2.105.0 w Windows. Gdyby przyszła wersja `gh` go zmieniła, repozytorium pokaże `ResponseInvalid`, a nie pustą listę.
- Artefakt design systemu nie zna jeszcze `ItemRow.labels`. Faza 2 dopisuje prop do zakresu F-05, ale jeśli F-05 zostanie zaplanowany wcześniej, jego plan trzeba będzie uzupełnić.

## Success Criteria (Summary)

- W „Projekty (GitHub)” etykiety są widoczne w wierszu tak jak w GitLabie, a w wąskim oknie znikają.
- Wywołanie `gh` dalej tylko czyta, ma stałe argumenty i przechodzi test strażnika; testy używają wyłącznie wymyślonych danych.
- Pozostałe widoki z `ItemRow` wyglądają jak przed zmianą.
