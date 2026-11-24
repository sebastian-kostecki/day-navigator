# Opis wydania z CHANGELOG — Plan Brief

> Full plan: `context/changes/release-notes/plan.md`

## What & Why

Dziś opis każdego GitHub Release to tylko uwaga o SmartScreen, więc z wydania nie widać, co się zmieniło (issue #14, S-09). Opis ma pokazywać zmiany od poprzedniego tagu `app-v*`.

## Starting Point

Job `release` w `.github/workflows/ci.yml` przekazuje stały `releaseBody` do `tauri-action`. Job `version` decyduje o publikacji na podstawie wersji z `Cargo.toml`. Nie ma CHANGELOG, a tematy commitów to tytuły faz, mało czytelne jako opis.

## Desired End State

Przed podbiciem wersji piszesz sekcję w `CHANGELOG.md`. Wydanie pokazuje tę sekcję, link „Full changelog” do porównania z poprzednim tagiem i dotychczasową uwagę o SmartScreen. Bez sekcji wydanie nie powstaje, a CI mówi dlaczego.

## Key Decisions Made

| Decision          | Choice                                         | Why (1 sentence)                                               |
| ----------------- | ---------------------------------------------- | -------------------------------------------------------------- |
| Źródło opisu      | Ręczny `CHANGELOG.md`                          | Tekst pisany dla czytelnika zamiast tytułów faz.               |
| Format            | Keep a Changelog, po angielsku                 | Kod i GitHub po angielsku; aplikacja zostaje po polsku.        |
| Brak sekcji       | Job `version` failuje, wydania nie ma          | Lepiej brak wydania niż wydanie bez opisu.                     |
| Kiedy sprawdzać   | Tylko przy wydaniu (`publish=true`)            | Gałęzie bez zmian w zachowaniu CI.                             |
| Historia          | Sekcje 0.1.0 i 0.2.0 uzupełnione wstecz        | Kompletny CHANGELOG od pierwszego wydania.                     |
| Uwaga SmartScreen | Dosłownie jak dziś, na końcu opisu             | Zostaje do S-11, które ją usunie.                              |
| Logika            | Skrypt bash + self-test w jobie `checks`       | Da się sprawdzić lokalnie, bez wydania.                        |

## Scope

**In scope:** `CHANGELOG.md`; `scripts/release-notes.sh`; `scripts/test-release-notes.sh`; joby `checks`, `version`, `release` w `ci.yml`.

**Out of scope:** opis z commitów; sprawdzanie przy każdym pushu; zmiana warunku publikacji i `version`; podpis (S-11); edycja `AGENTS.md` (F-03 w toku, follow-up); poprawianie opisów 0.1.0/0.2.0.

## Architecture / Approach

`version` (ubuntu, pełna historia) → przy `publish=true` `release-notes.sh <version>` wycina `## [<version>]`, wyznacza poprzedni tag `app-v*` (osiągalny z HEAD, bez tagu bieżącej wersji) i składa treść → output `notes` → `release` przekazuje go jako `releaseBody`.

## Phases at a Glance

| Phase                        | What it delivers                               | Key risk                                           |
| ---------------------------- | ---------------------------------------------- | -------------------------------------------------- |
| 1. Opis wydania z CHANGELOG  | CHANGELOG, skrypt, self-test, podpięcie w CI   | Przekazanie wielolinijkowego outputu sprawdzone tylko na sucho |

**Prerequisites:** brak.
**Estimated effort:** jedna sesja.

## Open Risks & Assumptions

- Prawdziwego przejścia przez GitHub Actions (output między jobami, render opisu) nie da się sprawdzić bez wydania; logika jest sprawdzona lokalnie.
- Podbicie wersji bez sekcji CHANGELOG da czerwony `main` — tak ma być, ale wymaga poprawki kolejnym commitem.

## Success Criteria (Summary)

- Self-test i próba na sucho dla 0.2.0 przechodzą.
- Następne wydanie pokazuje sekcję z CHANGELOG, link porównania i uwagę o SmartScreen.
