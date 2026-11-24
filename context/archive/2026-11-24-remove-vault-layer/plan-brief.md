# Usunięcie warstwy vaultów — Plan Brief

> Full plan: `context/changes/remove-vault-layer/plan.md`

## What & Why

Warstwa vaultów Obsidian (`src-tauri/src/vault/`) leży nieużywana od M-1, bo vaulty są poza zakresem MVP. Od 0.2.0 w „Stanie źródeł” są już tylko „nieskonfigurowane”. Usuwamy ją, żeby zestaw źródeł health był ustalony, zanim S-07 (szczegóły „Dane aplikacji”) i S-08 (dziennik zdarzeń) zaczną na nim budować. To foundation F-03 z M-2, issue #11.

## Starting Point

Źródło „Ustawienia” i cały `health::probe` opierają się na `VaultError` i `VaultRegistry`. `settings.rs` waliduje blok `spaces` (ścieżki vaultów, wykluczenia). `VaultState` w `lib.rs` jest zapisywany w każdej rundzie, ale nikt go nie czyta. Testy vaultów, fixture'y i trzy crate'y istnieją tylko dla tej warstwy.

## Desired End State

Kod nie zawiera modułu, typu, źródła, testu, fixture'a ani zależności vaultów. „Stan źródeł” pokazuje Ustawienia, Dane aplikacji, GitLab, GitHub i Przypomnienia. Stary `settings.json` z blokiem `spaces` wczytuje się bez błędu. AGENTS.md mówi tylko, że aplikacja nie czyta i nie zapisuje vaultów.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Zależności w Cargo.toml | Usunąć `dunce`, `unicode-normalization`, dev `junction`, feature `Win32_Storage_FileSystem`; `tempfile` do dev-deps; poprawić komentarz `test-support` | Zero martwych zależności i odwołań do vaultów. Sekcja `[package]` sesji app-metadata zostaje nietknięta. |
| Blok `spaces` w settings.json | Całkowicie ignorowany, w dowolnej postaci | Nic go nie czyta, a stary plik ma się zawsze wczytać. |
| Błąd ustawień | Nowy `SettingsError` (NotConfigured / SettingsInvalid / Io) z tymi samymi kindami i tekstami | Karta „Ustawienia” i toasty zachowują się tak jak dziś. |
| `VaultState` | Usunąć | Nikt go nie czyta. |
| `Space` | Zostaje w `space.rs` | Używają go zadania własne i „To do”. |
| Brak `settings.json` | Nadal błąd „brak pliku ustawień” | Poza zakresem tej zmiany. |

## Scope

**In scope:**
- `src-tauri/src/vault/`, `VaultState`, źródła `vault:*` / `exclusions:*`, ich kindy w `short_reason`
- `settings.rs` bez `Settings`/`SpaceSettings`, z `check_settings` i `SettingsError`
- Testy `vault_guard.rs`, `vault_write.rs`, fixtures, helpery w `tests/common`, `.gitattributes`
- Przepisane testy health na źródła, które zostają, i test zgodności starego `settings.json`
- `Cargo.toml`/`Cargo.lock` (zależności), AGENTS.md, wpis Parked w roadmapie, komentarz w `src/health/types.ts`

**Out of scope:**
- Jakikolwiek nowy dostęp do vaultów; zmiany `Space`; widoki gitlab/github/todo; `ci.yml`, `[package]`, wersja
- Dokumenty historyczne (`context/archive/`, `notes/`, PRD)

## Architecture / Approach

`probe(settings_path)` zwraca `Vec<SourceStatus>` z jednym wpisem `settings`. `probe_round` w `runtime.rs` dokłada do niego app-data, gitlab, github i reminders jak dotąd, ale nie zapisuje już rejestru. Loadery bloków w `settings.rs` zostają bez zmian i są niezależne od `spaces`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Usunięcie warstwy vaultów z Rusta | Kod, testy, fixtures i zależności bez vaultów; health na 5 źródłach | Utrata pokrycia reguł health przy przepisywaniu testów z `SourceId::Vault` |
| 2. Dokumentacja i UI | AGENTS.md bez kontraktu vaultów, Parked w roadmapie, komentarz TS | Konflikt statusów w `roadmap.md` z równoległymi sesjami |

**Prerequisites:** brak (F-03 nie ma prerekwizytów).
**Estimated effort:** jedna sesja, dwie fazy, głównie usuwanie.

## Open Risks & Assumptions

- `Cargo.toml`/`Cargo.lock` zmienia też sesja `app-metadata`. Zmieniamy inne sekcje, więc ewentualny konflikt w locku rozwiązuje `cargo build`.
- Zakładamy, że poza `vault/fsops` żaden kod produkcyjny nie importuje `tempfile`. Weryfikuje to `cargo build`.

## Success Criteria (Summary)

- „Stan źródeł” działa bez kart vaultów, a stary `settings.json` się wczytuje
- Wszystkie kontrole sprzed commita z AGENTS.md przechodzą
- `git grep vault` w kodzie trafia tylko w test zgodności starego `settings.json`
