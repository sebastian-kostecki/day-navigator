# Usunięcie warstwy vaultów — plan implementacji

## Overview

Usuwamy nieużywaną warstwę vaultów Obsidian (`src-tauri/src/vault/`), a z nią źródła `vault:*` / `exclusions:*` w „Stanie źródeł”, ich testy, fixtures, helpery testowe i zależności, z których korzystała tylko ta warstwa. Źródło „Ustawienia” sprawdza odtąd tylko sam plik `settings.json` (istnieje, UTF-8, JSON, `version: 1`). Blok `spaces` ze starego pliku jest ignorowany. AGENTS.md traci kontrakt kodu vaultów, ale zasada „aplikacja nie czyta i nie zapisuje vaultów” zostaje. Foundation F-03 z M-2 (issue #11, MS-11) odblokowuje S-07 i S-08.

## Current State Analysis

- `settings::load_settings` (`src-tauri/src/settings.rs:63`) zwraca `Result<Settings, VaultError>`. `Settings { work, private }` niesie `SpaceSettings { vault, excluded: Vec<VaultPath> }` (`:19-29`). `parse_settings`/`convert` (`:582-618`) walidują `version` i kształt `spaces` (bezwzględna ścieżka vaultu, poprawne `excluded`, `deny_unknown_fields` na `RawSpace`).
- `health::probe` (`src-tauri/src/health/probe.rs:18`) zwraca `(Result<VaultRegistry, VaultError>, Vec<SourceStatus>)`. Po `settings` dokłada `vault:work`, `exclusions:work`, `vault:private` i `exclusions:private` (`probe_space`, `:188-243`). `unresolved_config_dir` (`:43`) buduje `VaultError::SettingsInvalid`. `error_status`/`kind_name`/`message_pl` (`:245-289`) działają na `VaultError`.
- `SourceId::Vault(Space)` / `Exclusions(Space)` i klucze `vault:{s}` / `exclusions:{s}` są w `src-tauri/src/health/mod.rs:24-49`. `short_reason` ma kindy wyłącznie vaultowe: `RootMissing`, `RootsOverlap`, `UnmatchedExclusions` (`:185-189`). Prawie wszystkie testy jednostkowe w `health/mod.rs` używają `SourceId::Vault` jako przykładowego źródła.
- `lib.rs:29-47` definiuje `VaultState` (RwLock z wynikiem budowy rejestru), zakładany w `setup` (`:175-185`) i podmieniany w `runtime.rs:310`. Nikt nie wywołuje `VaultState::current()`, więc stan jest martwy.
- `Space` żyje w `src-tauri/src/space.rs` i używają go zadania własne i „To do”. Zostaje. `vault/mod.rs` tylko go re-eksportuje.
- Testy: `tests/vault_guard.rs` i `tests/vault_write.rs` dotyczą wyłącznie vaultów. `tests/health_probe.rs` ma testy ustawień, vaultów i store. W `tests/common/mod.rs` poza vaultami używane są tylko `test_store_dir` i moduł `crash`. Fixtures `tests/fixtures/vault/` czytają tylko testy vaultów i `include_bytes!` w `vault/edit.rs`/`markdown.rs`. `.gitattributes:1-2` (`src-tauri/tests/fixtures/** -text`) dotyczy tylko tych fixtures.
- Zależności używane wyłącznie przez vault lub jego testy: `dunce`, `unicode-normalization`, feature `Win32_Storage_FileSystem` w `windows-sys` (`fake_cli.rs` używa tylko `Win32_Foundation`), dev `junction`. `tempfile` jest w `[dependencies]`, a poza `vault/fsops` używają go tylko testy. `sha2` zostaje (`gitlab/mod.rs`). Komentarz feature `test-support` (`Cargo.toml:60`) wymienia `vault::fsops`, `Vault::apply`.
- UI: jedyne odwołanie to komentarz w `src/health/types.ts:13`. Widok health renderuje karty z listy `sources` i nie zna kluczy vaultów.

## Desired End State

Kod (`src/`, `src-tauri/`) nie ma modułu, typu, źródła, testu, fixture'a ani zależności vaultów. Jedyne wystąpienia słowa „vault” to test zgodności, który dowodzi, że stary `settings.json` z blokiem `spaces` się wczytuje, i komentarz przy nim. „Stan źródeł” pokazuje w tej kolejności: Ustawienia, Dane aplikacji, GitLab, GitHub (po pierwszych rundach) i Przypomnienia. Brakujący plik ustawień, zły JSON i zła `version` nadal dają błąd „Ustawienia” z tymi samymi kindami (`NotConfigured`, `SettingsInvalid`, `Io`) i tekstami. AGENTS.md mówi, że aplikacja nie czyta i nie zapisuje vaultów, bez opisu kontraktu kodu vaultów. Roadmapa ma rozwiązany wpis w `## Parked`.

### Key Discoveries:

- Loadery bloków `gitlab`, `github`, `todo`, `reminders`, `gitlabRefresh` i `gitlabFilter` czytają plik samodzielnie i ignorują `spaces` (`settings.rs:98-580`). Pominięcie `spaces` ich nie zmienia.
- `RawSettings` nie ma `deny_unknown_fields`, więc nieznane klucze najwyższego poziomu (w tym `spaces`) są już tolerowane.
- Wzorzec błędów: `thiserror::Error` + `Serialize` z `#[serde(tag = "kind", content = "detail")]`. `kind_name` w `probe.rs:257` bierze kind z tagu serde, co zadziała tak samo dla nowego typu.
- `notice_never_carries_message_text` (`health/mod.rs:590`) i tekst `short_reason` są gwarancją braku danych w toastach. Test zostaje, ale na źródłach Settings/GitLab zamiast vaultów.

## What We're NOT Doing

- Żadnego nowego dostępu do vaultów ani zastępczego mechanizmu. Zasada z PRD Non-Goals zostaje.
- Nie zmieniamy typu `Space` ani jego użyć w zadaniach własnych i „To do”.
- Nie walidujemy już bloku `spaces` w żadnej postaci (decyzja: całkowicie ignorowany, także gdy nie jest obiektem).
- Nie zmieniamy zachowania brakującego `settings.json` (nadal błąd „brak pliku ustawień”).
- Nie ruszamy archiwów w `context/archive/`, `notes/idea.md` ani PRD: to dokumenty historyczne, nie kod.
- Nie podbijamy wersji, nie zmieniamy metadanych `[package]` w `Cargo.toml` ani `ci.yml` (obszar sesji `app-metadata` / `release-notes`).
- Nie zmieniamy widoków `src/gitlab/`, `src/github/` ani `src/todo/` (obszar sesji S-01).

## Implementation Approach

Najpierw nowy, mały typ błędu ustawień, który zastępuje `VaultError` w jedynym miejscu poza vaultem, które go potrzebuje (źródło „Ustawienia”). Potem health i runtime bez rejestru, na koniec usunięcie modułu, testów, fixtures i zależności. Wszystko w jednej fazie, bo kod kompiluje się dopiero po całości. Dokumentacja jest w osobnej fazie.

## Critical Implementation Details

- **Kolejność w Cargo:** po usunięciu zależności uruchom `cargo build`, żeby odświeżyć `Cargo.lock`. Zmieniaj tylko sekcje `[dependencies]`, `[features]` (komentarz), `[dev-dependencies]` i `[target.*]`, nie `[package]`, bo tam pracuje równoległa sesja `app-metadata`.
- **`tempfile` do dev-dependencies:** przed przeniesieniem potwierdź `cargo build` (bez testów), że żaden kod produkcyjny poza usuniętym `vault/fsops` go nie importuje.

## Phase 1: Usunięcie warstwy vaultów z Rusta

### Overview

Kod, testy, fixtures i zależności bez vaultów. „Stan źródeł” działa na źródłach, które zostają.

### Changes Required:

#### 1. Błąd i sprawdzenie ustawień

**File**: `src-tauri/src/settings.rs`

**Intent**: Zastąpić `Settings`/`SpaceSettings`/`RawSpace`/`convert` i zależność od `VaultError` własnym `SettingsError`. Sprawdzenie pliku obejmuje istnienie, odczyt, BOM, UTF-8, JSON i `version == 1`, a `spaces` jest ignorowany. Przeredagować komentarze, które mówią o vaultach (np. przy `load_gitlab_settings`).

**Contract**: `pub enum SettingsError { NotConfigured { settings_path: PathBuf }, SettingsInvalid { reason: String }, Io { path: String, kind: String } }`, derive `Debug, Clone, thiserror::Error, serde::Serialize`, `#[serde(tag = "kind", content = "detail")]`. Teksty `#[error]` takie jak dziś w `VaultError`. `pub fn check_settings(path: &Path) -> Result<(), SettingsError>` zamiast `load_settings`. `Io::path` to nadal tylko nazwa pliku. Testy: usunąć te, które sprawdzają ścieżki vaultów i `excluded`. Testy niezależności loaderów przejść na `check_settings(..).is_ok()`. Dodać test zgodności: stary plik z `spaces` (bezwzględna i względna ścieżka, `excluded` z `../up`, stare `tasks`, nieznany klucz, `"spaces": "x"`, `null`) przechodzi `check_settings`.

#### 2. Model źródeł bez vaultów

**File**: `src-tauri/src/health/mod.rs`

**Intent**: Usunąć warianty `SourceId::Vault` / `Exclusions`, import `Space` i kindy `RootMissing`, `RootsOverlap`, `UnmatchedExclusions` z `short_reason`. Testy przepisać na źródła, które zostają (Settings, AppData, GitLab, GitHub, Reminders), z zachowaniem tego, co sprawdzają (przejścia, baseline po awarii ustawień, toast bez treści komunikatu, tooltip, limit UTF-16).

**Contract**: `SourceId = Settings | AppData | Gitlab | Github | Reminders`, klucze bez zmian. Przykład w komentarzu doc: `settings`, `app-data`.

#### 3. Probe bez rejestru

**File**: `src-tauri/src/health/probe.rs`

**Intent**: `probe` sprawdza tylko ustawienia i zwraca listę statusów. `unresolved_config_dir` daje błąd „Ustawienia”. `error_status`, `kind_name` i `message_pl` działają na `SettingsError`. Usunąć `probe_space` i `space_name`. Komentarze modułu i `store_status` bez odwołań do vaultów.

**Contract**: `pub fn probe(settings_path: &Path) -> Vec<SourceStatus>` (jeden wpis `settings`), `pub fn unresolved_config_dir() -> Vec<SourceStatus>`, `pub fn message_pl(e: &SettingsError) -> String` z tymi samymi tekstami dla `NotConfigured`, `SettingsInvalid` i `Io`. Testy jednostkowe dostosowane.

#### 4. Runtime i builder

**Files**: `src-tauri/src/runtime.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/space.rs`

**Intent**: Usunąć `VaultState` (definicja, `manage` w `setup`, `replace` w `check_round`), `pub mod vault` i importy. `probe_round` zwraca `Vec<SourceStatus>`. Komentarz o regule blokad w `runtime.rs` i doc `space.rs` bez wzmianki o vaultach.

**Contract**: `type ProbeResult` znika (albo staje się `Vec<SourceStatus>`). Kolejność źródeł: settings, app-data, gitlab, github, reminders. Nieużywane importy (`RwLock`, `Arc`) usunąć, jeśli nic innego w `lib.rs` z nich nie korzysta.

#### 5. Usunięcie modułu, testów i fixtures

**Files**: `src-tauri/src/vault/` (cały), `src-tauri/tests/vault_guard.rs`, `src-tauri/tests/vault_write.rs`, `src-tauri/tests/fixtures/` (cały), `src-tauri/tests/common/mod.rs`, `src-tauri/tests/health_probe.rs`, `.gitattributes`

**Intent**: Usunąć kod i testy vaultów. W `tests/common/mod.rs` zostają `pub mod crash` i `test_store_dir`, z nowym doc modułu. `health_probe.rs` zachowuje testy store, a testy ustawień przechodzą na `Vec<SourceStatus>`: brak pliku → `NotConfigured`, zły JSON → `SettingsInvalid`, stary plik z `spaces` → tylko `settings` w stanie `Ok`. Z `.gitattributes` usunąć komentarz i regułę `src-tauri/tests/fixtures/** -text`.

**Contract**: Po zmianie żaden test nie importuje `day_navigator_lib::vault`.

#### 6. Zależności

**Files**: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`

**Intent**: Usunąć zależności bez użytkowników i nieaktualną wzmiankę w komentarzu `test-support`.

**Contract**: Usunąć `dunce`, `unicode-normalization`, feature `Win32_Storage_FileSystem` (zostaje `Win32_Foundation`), dev `junction` z sekcją `[target.'cfg(windows)'.dev-dependencies]`. `tempfile` przenieść do `[dev-dependencies]`. Komentarz `test-support`: „Exposes `CliRunner::with_programs` and the `fake-cli` bin to the integration tests only; production builds never enable it.” `Cargo.lock` odświeżony przez `cargo build`.

### Success Criteria:

#### Automated Verification:

- `cargo fmt --check` przechodzi w `src-tauri/`
- `cargo clippy --all-targets -- -D warnings` przechodzi w `src-tauri/`
- `cargo test` przechodzi w `src-tauri/`
- `git grep -n -i -E "vault|exclusion" -- src-tauri src .gitattributes` pokazuje tylko test zgodności starego `settings.json` i komentarz przy nim
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą

#### Manual Verification:

- W `npm run tauri dev` „Stan źródeł” pokazuje Ustawienia, Dane aplikacji, GitLab, GitHub i Przypomnienia, bez kart vaultów i wykluczeń
- Ze starym `settings.json` z blokiem `spaces` karta „Ustawienia” jest OK, a tray nie jest czerwony z tego powodu

**Implementation Note**: Po zielonych kontrolach automatycznych zatrzymaj się na ręczne potwierdzenie, zanim przejdziesz do fazy 2.

---

## Phase 2: Dokumentacja i UI

### Overview

Dokumenty i komentarz UI opisują stan bez kodu vaultów.

### Changes Required:

#### 1. AGENTS.md

**File**: `AGENTS.md`

**Intent**: Twardą regułę o vaultach skrócić do „Vaults are out of scope: the app never reads or writes a vault.” (bez kontraktu istniejącego kodu). Przykład enuma błędu wskazać na plik, który istnieje (`src-tauri/src/settings.rs`, `SettingsError`). Z „Testing” usunąć zdania o fixtures vaultów i `test_vault()`, a `tests/common` opisać jako helpery store/crash.

**Contract**: Sekcje `Hard Rules`, `Rust ↔ TypeScript Contracts`, `Testing`.

#### 2. Roadmapa

**File**: `context/foundation/roadmap.md`

**Intent**: Wpis `Obsidian vault access` w `## Parked`: kod usunięty w F-03, każdy przyszły dostęp do vaultów zaczyna się od nowego planu, a zasada PRD się nie zmienia. Status F-03 aktualizują skille (`planning` → `in-progress`). Poza tym bez zmian, żeby nie zderzyć się ze statusami innych sesji.

**Contract**: `## Parked` i wiersz/blok F-03 (tylko `Status`).

#### 3. Komentarz typu UI

**File**: `src/health/types.ts`

**Intent**: Przykład kluczy w komentarzu `id` bez `vault:work` / `exclusions:private`.

**Contract**: `/** `id` is a stable key such as `settings`, `app-data`, `gitlab`. */`

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `git grep -n -i "vault" -- AGENTS.md` pokazuje tylko zasadę „never reads or writes a vault” (i ewentualnie słowo w regule danych służbowych)

#### Manual Verification:

- AGENTS.md i wpis w `## Parked` czytają się spójnie z PRD Non-Goals

---

## Testing Strategy

### Unit Tests:

- `settings.rs`: `check_settings` dla braku pliku, złego JSON, złej/brakującej `version`, BOM i niepoprawnego UTF-8. Test zgodności starego bloku `spaces` w różnych postaciach. Niezależność loaderów bloków od siebie.
- `health/mod.rs`: te same reguły przejść, baseline, toastów, tooltipa i watchdoga, na źródłach bez vaultów.
- `health/probe.rs`: `message_pl` i `kind_name` dla `SettingsError`, `unresolved_config_dir`.

### Integration Tests:

- `tests/health_probe.rs`: probe ustawień (brak, zły, stary plik z `spaces`) i dotychczasowe testy store.

### Manual Testing Steps:

1. `npm run tauri dev` z bieżącym `settings.json`: „Stan źródeł” bez kart vaultów.
2. Dodaj do `settings.json` blok `"spaces": {"work": {"vault": "C:\\nie-istnieje", "excluded": ["x"]}}`: Ustawienia nadal OK.
3. Zepsuj JSON: karta „Ustawienia” pokazuje błąd, tray robi się czerwony, toast nie zawiera treści pliku.

## Performance Considerations

Brak. Runda health robi mniej pracy, bo nie listuje folderów vaultów.

## Migration Notes

Stary `settings.json` wczytuje się bez zmian, a `spaces` jest ignorowany. Kluczy `vault:*` / `exclusions:*` nie ma już w zrzucie health. UI nie miało dla nich specjalnej logiki, a baseline przejść jest tylko w pamięci, więc znikające źródła nie wywołują toastu (`transitions` milczy dla źródeł nieobecnych w `next`).

## References

- Issue: GitHub #11 „Usunięcie nieużywanej warstwy vaultów”
- Roadmapa: `context/foundation/roadmap.md` (`### F-03`, `## Parked`)
- Wzorzec błędu z tagiem: `src-tauri/src/vault/error.rs` (usuwany) → `SettingsError`
- Pierwotna warstwa: `context/archive/2026-11-20-safe-vault-access/`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Usunięcie warstwy vaultów z Rusta

#### Automated

- [x] 1.1 `cargo fmt --check` przechodzi w `src-tauri/` — 773a7d8
- [x] 1.2 `cargo clippy --all-targets -- -D warnings` przechodzi w `src-tauri/` — 773a7d8
- [x] 1.3 `cargo test` przechodzi w `src-tauri/` — 773a7d8
- [x] 1.4 `git grep` vault/exclusion w kodzie pokazuje tylko test zgodności starego `settings.json` — 773a7d8
- [x] 1.5 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 773a7d8

#### Manual

- [x] 1.6 „Stan źródeł” bez kart vaultów i wykluczeń w `npm run tauri dev` — 773a7d8
- [x] 1.7 Stary `settings.json` z blokiem `spaces` daje „Ustawienia” OK — 773a7d8

### Phase 2: Dokumentacja i UI

#### Automated

- [x] 2.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 9a9660d
- [x] 2.2 `git grep` vault w AGENTS.md pokazuje tylko zasadę nieczytania i niezapisywania vaultów — 9a9660d

#### Manual

- [x] 2.3 AGENTS.md i wpis w `## Parked` spójne z PRD Non-Goals — 9a9660d
