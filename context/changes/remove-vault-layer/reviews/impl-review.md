<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Usunięcie warstwy vaultów

- **Plan**: context/changes/remove-vault-layer/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Findings

### F1 — Komentarz `baseline()` i test zakładają, że awaria ustawień ukrywa źródło

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/health/mod.rs:162-164, :513-514
- **Detail**: Doc `baseline()` nadal mówi „a source hidden by a settings outage”, a komentarz testu `absent_source_is_silent_and_baseline_prevents_repeat_toast` mówi, że przypomnienia znikają przy nierozwiązanym folderze konfiguracji. Po usunięciu vaultów awaria ustawień nie ukrywa już żadnego źródła, a `settings_path` jest ustalany raz przy starcie. Jedyny realny przypadek „nieobecne, potem obecne” to GitLab i GitHub przed pierwszą rundą. Logika funkcji jest poprawna, nieaktualne jest tylko uzasadnienie.
- **Fix**: Przeredagować doc `baseline()` na źródło nieobecne w danej rundzie (np. GitLab/GitHub przed końcem pierwszej rundy) i komentarz testu na ten sam przypadek, bez zmiany asercji.
- **Decision**: FIXED

### F2 — Test zgodności w probe nie sprawdza, że nieistniejący folder nie powstaje

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/tests/health_probe.rs:43
- **Detail**: Usunięty `missing_root_is_reported_without_creating_it_*` sprawdzał, że probe nie tworzy folderu z ustawień (reguła AGENTS.md „never create it silently”). Nowy `old_settings_with_spaces_is_only_settings_ok` używa ścieżki `missing`, ale nie sprawdza `!missing.exists()`.
- **Fix**: Dodać `assert!(!missing.exists())` w `old_settings_with_spaces_is_only_settings_ok`.
- **Decision**: FIXED

### F3 — `SettingsError` bez `PartialEq`/`Eq` i `kind()`, kind przez serde round-trip

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/settings.rs:20, src-tauri/src/health/probe.rs (`kind_name`)
- **Detail**: `GitlabError` i `StoreError` mają `kind() -> &'static str`, a `GitlabError` też `PartialEq, Eq`. `SettingsError` ich nie ma, więc `kind_name` w probe dalej wyciąga kind z tagu serde. To pozostałość po wielowariantowym `VaultError`. Działa poprawnie.
- **Fix**: Dodać `PartialEq, Eq` i `fn kind(&self) -> &'static str` do `SettingsError`, a w probe zastąpić `kind_name` wywołaniem `kind()`.
- **Decision**: FIXED

### F4 — Nierozwiązany folder konfiguracji raportowany jako `SettingsInvalid` (sprzed zmiany)

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/health/probe.rs:28-35
- **Detail**: `unresolved_config_dir` daje kind `SettingsInvalid` („Ustawienia są nieprawidłowe: nie można ustalić folderu…”), a `GitlabError`/`GithubError` mają osobny kind `ConfigDirUnresolved`, już obsłużony w `short_reason`. Zachowanie istniało przed tą zmianą i plan go nie obejmuje.
- **Fix**: Zostawić (poza zakresem). Ewentualnie osobna zmiana z wariantem `SettingsError::ConfigDirUnresolved`.
- **Decision**: SKIPPED (pre-existing, out of scope)
