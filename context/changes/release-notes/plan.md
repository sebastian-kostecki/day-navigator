# Opis wydania z CHANGELOG — plan implementacji

## Overview

Opis GitHub Release ma pokazywać, co zmieniło się od poprzedniego tagu `app-v*` (S-09, issue #14, MS-14). Źródłem jest sekcja wersji w `CHANGELOG.md`, pisana ręcznie po angielsku przed podbiciem wersji; CI ją wycina, dokleja link do porównania z poprzednim tagiem i niezmienioną uwagę o niepodpisanym instalatorze.

## Current State Analysis

- `.github/workflows/ci.yml`, job `release`: `tauri-apps/tauri-action@v1` z `releaseBody` jako stałym tekstem o SmartScreen, `tagName: app-v__VERSION__`.
- Job `version` (ubuntu, tylko `main`) czyta wersję z `src-tauri/Cargo.toml` przez `awk` i ustala `publish` przez `gh release view`; `actions/checkout@v7` bez `fetch-depth`, czyli płytko i bez tagów.
- Tagi `app-v0.1.0` (d8a2e22) i `app-v0.2.0` (670d06e) istnieją na zdalnym repo. Historia jest w Conventional Commits, ale tematy to tytuły faz („(p3)”, „impl review fixes”), więc użytkownik wybrał ręczny CHANGELOG.
- Wzorzec self-testu skryptu powłoki: `scripts/test-pre-push.sh` (repo tymczasowe, wymyślone dane, `PASS`/`FAIL`), uruchamiany w jobie `checks` krokiem „Pre-push hook self-test”.
- Brak `CHANGELOG.md` w repo.

## Desired End State

- `CHANGELOG.md` w korzeniu repo, po angielsku, w formacie Keep a Changelog: krótki nagłówek z instrukcją wydania, `## [Unreleased]`, `## [0.2.0] - 2026-11-24`, `## [0.1.0] - 2026-11-21` (sekcje `### Added` / `### Fixed`, jedna linia na zmianę z roadmapy).
- `scripts/release-notes.sh <version>` wypisuje na stdout: treść sekcji `## [<version>]`, link „Full changelog” (compare od poprzedniego tagu `app-v*` do `app-v<version>`, gdy poprzedni tag istnieje) i uwagę o SmartScreen dosłownie jak dziś. Brak sekcji albo pusta sekcja → komunikat na stderr i kod wyjścia ≠ 0.
- `scripts/test-release-notes.sh` sprawdza to na repo tymczasowym z wymyślonymi danymi; job `checks` go uruchamia.
- Job `version`: `fetch-depth: 0`; gdy `publish=true`, krok generuje notatki skryptem i wystawia je jako output `notes`; brak sekcji wywala job, więc `release` się nie uruchamia. Gdy `publish=false`, nic się nie zmienia.
- Job `release`: `releaseBody: ${{ needs.version.outputs.notes }}`.

### Key Discoveries:

- Poprzedni tag = najnowszy (sortowanie wersji) tag `app-v*` osiągalny z HEAD, z pominięciem `app-v<version>`: przy uzupełnianiu wydania bez instalatora tag bieżącej wersji już istnieje i nie może być „poprzednim”.
- Wielolinijkowy output w `$GITHUB_OUTPUT` wymaga składni z ogranicznikiem (`name<<DELIM`); ogranicznik losowy, żeby treść CHANGELOG nie mogła go zamknąć.
- `${{ needs.version.outputs.notes }}` w `with:` nie przechodzi przez powłokę, więc treść CHANGELOG nie jest wykonywana.

## What We're NOT Doing

- Bez generowania opisu z commitów i bez narzędzi typu git-cliff.
- Bez sprawdzania CHANGELOG przy każdym pushu (decyzja użytkownika: tylko przy wydaniu).
- Bez zmiany warunku publikacji (`publish` nadal tylko dla nowej wersji z `Cargo.toml`) i bez zmiany `version`.
- Bez zmiany tekstu uwagi o SmartScreen i bez podpisu (S-11).
- Bez edycji `AGENTS.md` (zmieniany równolegle przez F-03); instrukcja wydania trafia do nagłówka `CHANGELOG.md`, a dopisek w `AGENTS.md` to follow-up.
- Bez aktualizacji opisu już istniejących wydań 0.1.0 i 0.2.0.

## Implementation Approach

Cała logika w jednym skrypcie bash testowalnym lokalnie; workflow tylko go wywołuje i przekazuje wynik. Jedna faza, jeden commit.

## Critical Implementation Details

- Koniec sekcji: następny nagłówek `## ` albo koniec pliku. Dopasowanie nagłówka dokładne co do wersji (`## [0.2.0]` nie pasuje do `## [0.2.10]`), z dowolnym sufiksem po `]` (data).
- Skrypt nie może zależeć od `gh` ani sieci: URL repozytorium przychodzi z env (`REPO_URL`, w CI `${{ github.server_url }}/${{ github.repository }}`); bez niego link compare jest pomijany.

## Phase 1: Opis wydania z CHANGELOG

### Overview

Dodać CHANGELOG, skrypt z self-testem i podpiąć go w CI.

### Changes Required:

#### 1. CHANGELOG

**File**: `CHANGELOG.md`

**Intent**: Ręczne źródło opisu wydania, z historią 0.1.0 i 0.2.0 uzupełnioną wstecz z commitów `feat`/`fix`.

**Contract**: Keep a Changelog 1.1; nagłówki `## [Unreleased]`, `## [X.Y.Z] - YYYY-MM-DD`; podsekcje `### Added`, `### Changed`, `### Fixed`, `### Removed`. Nagłówek pliku mówi: przed podbiciem wersji przenieś wpisy z `[Unreleased]` do `## [<nowa wersja>] - <data>`, bo bez tej sekcji wydanie się nie uda.

#### 2. Skrypt notatek

**File**: `scripts/release-notes.sh`

**Intent**: Złożyć treść opisu wydania dla podanej wersji albo jawnie odmówić.

**Contract**: `release-notes.sh <version>`; env: `CHANGELOG` (domyślnie `CHANGELOG.md` w korzeniu repo), `REPO_URL` (opcjonalny). Stdout: sekcja bez linii nagłówka, przycięta z pustych linii; pusta linia; `**Full changelog:** <REPO_URL>/compare/app-v<prev>...app-v<version>` (gdy jest `REPO_URL` i poprzedni tag); pusta linia; uwaga o SmartScreen. Kody: 0 OK, 1 brak/pusta sekcja albo brak pliku, 2 złe argumenty.

#### 3. Self-test

**File**: `scripts/test-release-notes.sh`

**Intent**: Sprawdzić logikę bez wydania, na wymyślonych danych.

**Contract**: Przypadki: sekcja środkowa i ostatnia w pliku; dokładne dopasowanie wersji (0.2.0 vs 0.2.10); brak sekcji → ≠0; pusta sekcja → ≠0; brak pliku → ≠0; poprzedni tag pomija tag bieżącej wersji i tagi nieosiągalne z HEAD oraz sortuje wersje (0.10.0 > 0.9.0); brak poprzedniego tagu → bez linku; brak `REPO_URL` → bez linku; uwaga o SmartScreen zawsze na końcu.

#### 4. Workflow

**File**: `.github/workflows/ci.yml`

**Intent**: Generować opis w jobie `version` tylko przy publikacji i przekazać go do `tauri-action`.

**Contract**: `checks`: nowy krok `bash scripts/test-release-notes.sh`. `version`: checkout z `fetch-depth: 0`; output `notes`; krok `if: steps.decide.outputs.publish == 'true'` uruchamia skrypt z `REPO_URL` i zapisuje wynik do `$GITHUB_OUTPUT` z losowym ogranicznikiem. `release`: `releaseBody: ${{ needs.version.outputs.notes }}`.

### Success Criteria:

#### Automated Verification:

- Self-test przechodzi lokalnie: `bash scripts/test-release-notes.sh`
- Próba na sucho na prawdziwej historii: `REPO_URL=https://github.com/example/repo bash scripts/release-notes.sh 0.2.0` z lokalnymi tagami daje sekcję 0.2.0, link `app-v0.1.0...app-v0.2.0` i uwagę o SmartScreen; `0.9.9` kończy się kodem ≠0
- `ci.yml` parsuje się jako YAML, a `release` dostaje `needs.version.outputs.notes`
- `version` w `src-tauri/Cargo.toml` nadal `0.2.0`
- Kontrole sprzed commita przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- Przy następnym podbiciu wersji na `main` GitHub Release pokazuje sekcję z CHANGELOG, link porównania i uwagę o SmartScreen

---

## Testing Strategy

### Unit Tests:

- `scripts/test-release-notes.sh` (przypadki z punktu 3).

### Manual Testing Steps:

1. Lokalnie: `git fetch --tags`, potem próba na sucho dla 0.2.0 i dla nieistniejącej wersji.
2. Po następnym wydaniu: obejrzeć opis na stronie Release.

## Migration Notes

Istniejące wydania 0.1.0 i 0.2.0 zachowują stary opis. Przy uzupełnianiu wydania, które już istnieje bez instalatora, `tauri-action` może nie nadpisać jego opisu; to zachowanie akcji, niezmienione.

## References

- Issue #14; roadmap `### S-09` w `context/foundation/roadmap.md`
- `.github/workflows/ci.yml` (joby `checks`, `version`, `release`)
- `scripts/test-pre-push.sh` (wzorzec self-testu)
- https://keepachangelog.com/en/1.1.0/

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Opis wydania z CHANGELOG

#### Automated

- [x] 1.1 Self-test przechodzi lokalnie
- [x] 1.2 Próba na sucho na prawdziwej historii
- [x] 1.3 ci.yml parsuje się, release dostaje notes (2026-11-24: CI [run 45352441666](https://github.com/sebastian-kostecki/day-navigator/actions/runs/45352441666) dla `9eff6c4` — checks, build-test, version i release zielone)
- [x] 1.4 version nadal 0.2.0
- [x] 1.5 Kontrole sprzed commita przechodzą

#### Manual

- [x] 1.6 Następne wydanie pokazuje opis z CHANGELOG (2026-11-24: [app-v0.2.1](https://github.com/sebastian-kostecki/day-navigator/releases/tag/app-v0.2.1) ma sekcję `[0.2.1]` z CHANGELOG, link porównania `app-v0.2.0...app-v0.2.1` i uwagę o niepodpisanym instalatorze)
