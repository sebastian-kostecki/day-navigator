# Metadane aplikacji — plan implementacji

## Overview

Zastąpić wartości z szablonu w `src-tauri/Cargo.toml` (`description`, `authors`) prawdziwymi i dodać `bundle.copyright` w `src-tauri/tauri.conf.json`, żeby autor był widoczny we właściwościach pliku exe i instalatora NSIS (S-10, issue #15, MS-15).

## Current State Analysis

- `src-tauri/Cargo.toml:4-5`: `description = "A Tauri App"`, `authors = ["you"]`.
- `src-tauri/tauri.conf.json`: `productName` „Nawigator dnia”, identyfikator `com.sebas.day-navigator`, brak `bundle.publisher` i `bundle.copyright`.
- Metadane exe ustawia `tauri-build` 2.7.6 (`src/lib.rs:836-870`): `ProductName` i `FileDescription` = `productName`; `CompanyName` = `bundle.publisher`, a bez niego drugi człon identyfikatora („sebas”); `LegalCopyright` = `bundle.copyright` (tylko gdy ustawione).
- Instalator NSIS (`tauri-bundler` z `@tauri-apps/cli` 2.12.6, `installer.nsi:81,91-95,749`): `ProductName` i `FileDescription` = `productName`; `LegalCopyright` i `BrandingText` = `bundle.copyright`; „Publisher” we wpisie odinstalowania = publisher (fallback „sebas”).
- `description` z Cargo trafia tylko do `short_description` bundlera (deb/rpm); szablon NSIS go nie używa. `authors` też nie jest używane na Windows.

## Desired End State

- `Cargo.toml` ma `description` po polsku i `authors = ["Sebastian Kostecki"]` (bez e-maila); `version` bez zmian (`0.2.0`).
- `tauri.conf.json` ma `bundle.copyright = "© 2026 Sebastian Kostecki"`.
- `VersionInfo.LegalCopyright` pliku exe i instalatora `*-setup.exe` pokazuje „© 2026 Sebastian Kostecki”; `ProductName`/`FileDescription` nadal „Nawigator dnia”; `CompanyName` exe nadal „sebas”.

### Key Discoveries:

- Sama zmiana w `Cargo.toml` nie zmienia żadnej właściwości widocznej na Windows — autor pojawia się tylko przez `bundle.copyright`.
- Publisher wyznacza klucz rejestru `Software\<publisher>\Nawigator dnia` (`installer.nsi:68-69`), z którego instalator czyta katalog poprzedniej instalacji (`installer.nsi:372,943`) i przekazuje go deinstalatorowi przy aktualizacji. Zmiana publishera grozi zepsuciem aktualizacji z 0.2.0.

## What We're NOT Doing

- Nie zmieniamy `version` w `Cargo.toml` (podbicie na `main` publikuje wydanie, FR-016).
- Nie ustawiamy `bundle.publisher` — „Firma”/„Wydawca” zostaje „sebas”, żeby nie zmienić klucza rejestru instalatora.
- Nie dodajemy własnego zasobu VERSIONINFO, żeby `FileDescription` pokazywał opis zamiast nazwy produktu.
- Nie dodajemy `shortDescription`/`longDescription` w bundle (nieużywane przez NSIS).
- Bez adresu e-mail w `authors`.

## Implementation Approach

Dwie edycje konfiguracji w jednym commicie, potem przebudowa i odczyt `VersionInfo` z wygenerowanych plików.

## Phase 1: Metadane i weryfikacja

### Overview

Uzupełnić pola i sprawdzić, co faktycznie pokazują exe i instalator.

### Changes Required:

#### 1. Pakiet Cargo

**File**: `src-tauri/Cargo.toml`

**Intent**: Zastąpić wartości z szablonu prawdziwym opisem i autorem.

**Contract**: `[package]`: `description = "Nawigator dnia: jedno miejsce na dzień pracy (zadania z GitLaba i GitHuba, lista Do zrobienia, czas pracy)"`, `authors = ["Sebastian Kostecki"]`. `version` nietknięte; `Cargo.lock` się nie zmienia (metadane nie wchodzą do locka).

#### 2. Konfiguracja bundla

**File**: `src-tauri/tauri.conf.json`

**Intent**: Dodać copyright, jedyne pole, przez które autor trafia do exe i instalatora bez ryzyka dla aktualizacji.

**Contract**: `bundle.copyright = "© 2026 Sebastian Kostecki"`.

### Success Criteria:

#### Automated Verification:

- `version` w `src-tauri/Cargo.toml` nadal `0.2.0`, a `Cargo.lock` bez zmian: `git diff --stat`
- Kontrole sprzed commita przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- `VersionInfo` debugowego exe po `cargo build` pokazuje `LegalCopyright` „© 2026 Sebastian Kostecki”
- Lokalny `npm run tauri build -- --bundles nsis` tworzy instalator, którego `VersionInfo.LegalCopyright` to „© 2026 Sebastian Kostecki”

#### Manual Verification:

- Właściwości → Szczegóły pliku exe i instalatora w Eksploratorze pokazują „Prawa autorskie: © 2026 Sebastian Kostecki”

---

## Testing Strategy

### Manual Testing Steps:

1. `(Get-Item src-tauri\target\debug\day-navigator.exe).VersionInfo | Format-List *` po `cargo build`.
2. To samo dla `src-tauri\target\release\bundle\nsis\*-setup.exe` po lokalnym buildzie NSIS.
3. Opcjonalnie: Właściwości → Szczegóły w Eksploratorze.

## Migration Notes

Brak. Publisher bez zmian, więc klucze rejestru instalatora zostają te same, a aktualizacja z 0.2.0 działa jak dotąd.

## References

- Issue #15; roadmap `### S-10` w `context/foundation/roadmap.md`
- `tauri-build` 2.7.6 `src/lib.rs:836-870`
- `tauri-bundler` (`@tauri-apps/cli` 2.12.6) `crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi`, `nsis/mod.rs:281-318`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Metadane i weryfikacja

#### Automated

- [x] 1.1 version nadal 0.2.0, Cargo.lock bez zmian
- [x] 1.2 Kontrole sprzed commita przechodzą
- [x] 1.3 LegalCopyright w debugowym exe
- [x] 1.4 LegalCopyright w lokalnym instalatorze NSIS

#### Manual

- [ ] 1.5 Właściwości exe i instalatora w Eksploratorze pokazują autora
