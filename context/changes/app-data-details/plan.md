# Szczegóły „Dane aplikacji” w „Stanie źródeł” — plan implementacji

## Overview

Karta „Dane aplikacji” w widoku „Stan źródeł” pokazuje dziś tylko stan bazy i „Sprawdzono”. Po tej zmianie pokazuje też czas ostatniej kopii (z ostrzeżeniem, gdy dzisiejsza kopia nie powstała), liczbę zachowanych kopii „N z 7”, wersję schematu oraz przycisk „Otwórz folder kopii”. Slice S-07, PRD FR-022, GitHub issue #4.

## Current State Analysis

- Kopia dzienna: `Store::backup_if_due` (`src-tauri/src/store/backup.rs:34`) robi jedną kopię `day-navigator-YYYY-MM-DD.db` na dzień w `<app data dir>/backups/`; `prune` (`backup.rs:89`) zostawia 7 najnowszych z datą ≤ dziś (`KEEP`, `backup.rs:13`), kopie z datą z przyszłości zostają i nie zajmują slotu. Nieudana kopia trafia tylko do logu (`backup.rs:46`) — w UI nie ma po niej śladu.
- `backup::dated` (`backup.rs:26`) rozpoznaje nazwę kopii bez otwierania bazy; `has_dated_backup` (`store/mod.rs:36`) już tak czyta folder.
- Wersja schematu = `PRAGMA user_version` otwartej bazy; po udanym otwarciu wszystkie migracje są zastosowane (`store/mod.rs:119`), na `develop` to v10.
- Runda health co 30 s (i po „Sprawdź wszystkie”) woła `health::store_status` (`src-tauri/src/health/probe.rs:41`) → `StoreHandle::status` (`store/mod.rs:246`), które trzyma blokadę store podczas otwarcia, sprawdzenia i kopii. `probe_round` (`src-tauri/src/runtime.rs:250`) zbiera `Vec<SourceStatus>`, `check_round` (`runtime.rs:294`) składa `HealthSnapshot` (`health/mod.rs:110`) i publikuje go zdarzeniem `health-changed`.
- `StoreHandle::dir()` (`store/mod.rs:197`) daje folder danych bez blokady.
- Otwieranie pliku z Rust: `settings_file::open_settings_file` (`src-tauri/src/settings_file.rs:44`) — stała ścieżka, brak pliku = błąd (nic nie tworzy), błąd bez ścieżki, `app.opener().open_path(...)`. Plugin opener jest już w `Cargo.toml`, `package.json` i `lib.rs:142`; wywołanie z Rust nie potrzebuje uprawnień w `capabilities/default.json`.
- UI: `HealthPanel` (`src/health/HealthPanel.tsx:103`) przekazuje karcie `facts`; dla `app-data` tylko „Sprawdzono”. `SourceCard` (`src/health/SourceCard.tsx`) renderuje `<dl>` z `SourceFact { label, value }`. `useHealth.ts` woła `invoke` bezpośrednio — w `src/health/` nie ma `api.ts`, wbrew konwencji z `AGENTS.md`.
- Wzorzec komunikatu błędu otwarcia: `openSettingsErrorMessage` (`src/gitlab/api.ts:77`); inline błąd: `GitlabFilterBar.tsx:48`.

## Desired End State

- `HealthSnapshot` ma pole `appData` (`null`, gdy folderu danych nie da się ustalić). W nim `schemaVersion` (liczba, tylko gdy baza jest otwarta, inaczej `null`) i `backups`: albo `listed` (`kept`, `keep` = 7, `last` = `{ date, atMs }` lub `null`, `stale`), albo `unreadable`.
- Liczenie jak w `prune`: tylko kopie z datą ≤ dziś. 7 zwykłych + 1 z przyszłości daje `kept: 7`, `last` = najnowsza ≤ dziś. Brak folderu `backups/` = `listed` z `kept: 0`, `last: null`, `stale: true`. Inny błąd odczytu folderu = `unreadable` (nigdy pusta lista zamiast błędu).
- `stale` = brak kopii albo najnowsza kopia (≤ dziś) ma datę wcześniejszą niż dziś. Reguła w Rust, z testami.
- Szczegóły kopii są wypełniane także wtedy, gdy baza jest w błędzie (uszkodzona, brak pliku, nowsza wersja, nieudana migracja, zajęta); stan źródła, zasobnik, toasty i dziennik zdarzeń nie zależą od `appData`.
- Karta „Dane aplikacji”: „Sprawdzono”, „Ostatnia kopia” (`formatAgo(atMs)`; bez `atMs` data `DD.MM.YYYY`; bez kopii „brak”; ton ostrzeżenia, gdy `stale`), „Kopie” „N z 7” (przy `unreadable`: „nie można odczytać folderu” w tonie ostrzeżenia), „Schemat” „v10” (tylko gdy `schemaVersion` nie jest `null`), przycisk „Otwórz folder kopii”, a pod nim błąd inline, jeśli otwarcie się nie uda.
- Komenda `open_backups_folder` otwiera wyłącznie `<app data dir>/backups/` w Eksploratorze; brak folderu = błąd `Missing`, folder nie jest tworzony.
- W danych `appData` i w błędach nie ma ścieżek ani wartości wierszy.

### Key Discoveries:

- `prune` liczy sloty tylko dla dat ≤ dziś (`backup.rs:96-99`) — ta sama reguła daje „N z 7” zawsze ≤ 7.
- Blokada store jest już trzymana w `status()`, więc wersja schematu czytana tam nie dokłada drugiego wejścia w blokadę ani `spawn_blocking`.
- `store` nie importuje `tauri` (`store/mod.rs:3`), więc komenda otwierania folderu jest w osobnym module, jak `settings_file.rs`.
- `std::fs::File::set_modified` (stabilne) pozwala w testach ustawić czas pliku kopii na wymyśloną wartość.

## What We're NOT Doing

- Żadnej migracji ani nowej tabeli; żadnego nowego pluginu ani zmiany `capabilities/default.json`.
- Brak dzisiejszej kopii nie zmienia stanu źródła (nie ma czerwonego zasobnika, toastu, nowego `kind` ani zdarzenia w dzienniku) — tylko ton ostrzeżenia w karcie.
- Nie zmieniamy reguł kopii (`backup_if_due`, `prune`, `KEEP`), nie dodajemy przycisku „zrób kopię teraz” ani przywracania z UI.
- Kopie z datą z przyszłości nie są pokazywane w karcie.
- Ścieżka folderu nie trafia do UI w `appData` (komunikaty błędów bazy z `store_message_pl` zostają bez zmian).
- Nie zmieniamy pozostałych kart ani wierszy „Pozostałe źródła”.

## Implementation Approach

Backend najpierw i w całości (kontrakt Rust + lustro TS w jednym commicie, zgodnie z `AGENTS.md`), potem karta. Szczegóły liczy runda health w miejscu, które już trzyma blokadę store; folder kopii jest czytany przez czysty odczyt systemu plików (bez otwierania bazy), więc działa także przy bazie w błędzie. Otwieranie folderu kopiuje wzorzec `settings_file.rs`.

## Critical Implementation Details

- **Kolejność w rundzie**: podsumowanie folderu kopii czytaj *po* `StoreHandle::status`, bo `status` może w tej samej rundzie utworzyć dzisiejszą kopię — odczyt przed nim pokazałby fałszywe `stale` przez 30 s po północy lub po starcie.
- **Prywatność logów**: odczyt folderu w każdej rundzie (co 30 s) przy `unreadable` loguje tylko rodzaj błędu i tylko na poziomie `debug`, żeby nie zalewać logu.

## Phase 1: Kontrakt backendu (Rust + lustro TS)

### Overview

Podsumowanie kopii i wersja schematu trafiają do `HealthSnapshot.appData`, nowa komenda `open_backups_folder`, lustro TS i `src/health/api.ts`. Testy Rust pokrywają reguły liczenia, `stale`, stan błędu bazy, kształt JSON i brak ścieżek.

### Changes Required:

#### 1. Podsumowanie folderu kopii

**File**: `src-tauri/src/store/backup.rs`, `src-tauri/src/store/mod.rs`

**Intent**: Czysta funkcja czytająca `backups/` (bez bazy, bez blokady), która zwraca liczbę kopii ≤ dziś, najnowszą z nich (data z nazwy, czas z `modified()` pliku) i flagę `stale`. `StoreHandle` wystawia ją dla folderu danych oraz ścieżkę `backups/` wyłącznie do otwarcia.

**Contract**:
- `pub enum BackupSummary` (serde `#[serde(tag = "state", rename_all = "camelCase")]`, pola camelCase): `Listed { kept: u32, keep: u32, last: Option<LastBackup>, stale: bool }` | `Unreadable`.
- `pub struct LastBackup { date: NaiveDate /* serializowana jako "YYYY-MM-DD" */, at_ms: Option<u64> }` (`atMs` = `modified()` w ms od epoki; `None`, gdy czasu nie da się odczytać).
- `pub(crate) fn summarize(dir: &Path, today: NaiveDate) -> BackupSummary` w `backup.rs`: `NotFound` → `Listed { kept: 0, last: None, stale: true, keep: KEEP }`; inny błąd `read_dir` → `Unreadable` (log `debug`, sam `kind`); ignoruje pliki, które nie pasują do `dated`, i `.tmp`; liczy tylko daty ≤ `today`; `stale = last.map_or(true, |l| l.date < today)`.
- `StoreHandle::backup_summary(&self, today) -> Option<BackupSummary>` (`None` = folder danych nieustalony) i `StoreHandle::backups_dir(&self) -> Option<PathBuf>` (tylko do otwierania; doc-komentarz: nigdy do logów ani błędów).

#### 2. Wersja schematu ze `status`

**File**: `src-tauri/src/store/mod.rs`

**Intent**: Runda health dostaje wersję schematu w tym samym wejściu w blokadę, w którym sprawdza bazę.

**Contract**: `Store::schema_version(&self) -> Result<u32, StoreError>` (`PRAGMA user_version`); `StoreHandle::status(&self, today) -> Result<u32, StoreError>` zwraca ją po udanym `check` i `backup_if_due`. Istniejące testy i wywołania `status` dostosować (`is_ok()` zostaje poprawne).

#### 3. `appData` w migawce health

**File**: `src-tauri/src/health/mod.rs`, `src-tauri/src/health/probe.rs`, `src-tauri/src/runtime.rs`

**Intent**: Sonda store zwraca stan źródła i szczegóły; runda wkłada szczegóły do migawki. Stan źródła i przejścia działają jak dziś.

**Contract**:
- `health::AppDataDetails { schema_version: Option<u32>, backups: BackupSummary }` (serde camelCase).
- `HealthSnapshot` dostaje `pub app_data: Option<AppDataDetails>` (JSON `appData`, `null` gdy `None`); migawka startowa i watchdog mają `None`/zachowują poprzednią wartość jak resztę pól.
- `probe.rs`: `store_status` zwraca `(SourceStatus, Option<AppDataDetails>)` (albo nowa `store_probe` z tą parą i `store_status` jako cienka nakładka — do wyboru implementującego, byle istniejące testy `tests/health_probe.rs` dalej sprawdzały to samo). Kolejność: najpierw `handle.status(today)`, potem `handle.backup_summary(today)`; `schema_version` = `Some(v)` tylko przy `Ok(v)`. `None` dla całego `AppDataDetails`, gdy `backup_summary` zwraca `None`.
- `runtime.rs`: `probe_round` zwraca też szczegóły; `check_round` ustawia `app_data` w nowym `HealthSnapshot`.

#### 4. Komenda `open_backups_folder`

**File**: `src-tauri/src/backups_folder.rs` (nowy), `src-tauri/src/lib.rs`

**Intent**: Otwiera `<app data dir>/backups/` w domyślnym programie (Eksplorator). Kopia wzorca `settings_file.rs`: stała ścieżka, brak argumentu z UI, brak folderu = błąd, nic nie jest tworzone, błąd bez ścieżki i tekstu systemu.

**Contract**:
- `#[derive(Debug, thiserror::Error, Serialize)] #[serde(tag = "kind", content = "detail")] pub enum OpenBackupsError { DataDirUnresolved, Missing, OpenFailed }`.
- Czysta `fn backups_folder_to_open(path: Option<PathBuf>) -> Result<PathBuf, OpenBackupsError>` (`None` → `DataDirUnresolved`, nie-katalog → `Missing`).
- `#[tauri::command] pub async fn open_backups_folder(app: AppHandle) -> Result<(), OpenBackupsError>`: ścieżka z `app.state::<StoreHandle>().backups_dir()` (bez blokady store, więc bez `on_store`), `app.opener().open_path(...)`, błąd → `log::warn!("backups: open failed")` + `OpenFailed`.
- `pub mod backups_folder;` i rejestracja w `generate_handler!` obok `settings_file::open_settings_file`.

#### 5. Lustro TS i API

**File**: `src/health/types.ts`, `src/health/api.ts` (nowy), `src/health/useHealth.ts`

**Intent**: Typy TS odpowiadają serde w tym samym commicie; `api.ts` staje się jedynym plikiem `src/health/` importującym `invoke`.

**Contract**:
- `types.ts`: `BackupSummary` (unia na `state`: `"listed"` z `kept`, `keep`, `last: { date: string; atMs: number | null } | null`, `stale` | `"unreadable"`), `AppDataDetails { schemaVersion: number | null; backups: BackupSummary }`, `HealthSnapshot.appData: AppDataDetails | null`, `OpenBackupsError` (unia na `kind`).
- `api.ts`: `healthStatus()`, `checkHealthNow()`, `openBackupsFolder()` oraz `openBackupsErrorMessage(e: unknown): string` po polsku, wzorem `openSettingsErrorMessage` („Nie można ustalić folderu danych aplikacji.”, „Folder kopii nie istnieje.”, „Nie udało się otworzyć folderu kopii.”).
- `useHealth.ts` woła `healthStatus`/`checkHealthNow` z `api.ts` (`listen` zostaje w haku); zachowanie bez zmian.

#### 6. Testy Rust

**File**: `src-tauri/src/store/backup.rs` (testy modułu), `src-tauri/tests/store_backup.rs` lub `src-tauri/tests/health_probe.rs`, `src-tauri/src/backups_folder.rs`, `src-tauri/src/health/mod.rs`

**Intent**: Reguły liczenia i `stale` są udowodnione na wymyślonych plikach; stan błędu bazy dalej daje szczegóły kopii; JSON ma ustalony kształt i nie niesie ścieżek.

**Contract** (przypadki):
- brak `backups/` → `Listed { kept: 0, last: None, stale: true, keep: 7 }`;
- 3 kopie z dzisiejszą → `kept: 3`, `last.date` = dziś, `last.at_ms` = czas ustawiony przez `File::set_modified`, `stale: false`;
- 7 kopii ≤ dziś + 1 z przyszłości → `kept: 7`, `last.date` = dziś;
- najnowsza kopia z wczoraj → `stale: true`;
- pliki obce i `.day-navigator-….db.tmp` są pomijane;
- `backups` jest plikiem, nie folderem → `Unreadable`;
- `status` po otwarciu zwraca `MIGRATIONS.last().version`;
- baza uszkodzona (scenariusz jak w `tests/health_probe.rs:83`) → stan `Error { kind: "Damaged" }`, `schema_version: None`, `backups` = `Listed` z kopiami;
- `HealthSnapshot` serializuje `appData` (camelCase, `backups.state == "listed"`, `last.atMs`, `appData: null` przy `None`);
- JSON `appData` dla folderu o wymyślonej nazwie `Klient-Testowy` nie zawiera tej nazwy ani `C:\`;
- `backups_folder_to_open`: `None` → `DataDirUnresolved`; brak folderu → `Missing` i folder nadal nie istnieje; istniejący folder → zwrócona ścieżka; `OpenBackupsError::Missing` serializuje się jako `{"kind":"Missing"}`.

### Success Criteria:

#### Automated Verification:

- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy podsumowania kopii, `stale`, stanu błędu bazy, kształtu JSON, braku ścieżek i `backups_folder_to_open`
- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzą
- `npm run build`, `npm run lint` i `npm run format:check` przechodzą
- `rg "invoke" src/health` pokazuje import `invoke` tylko w `src/health/api.ts`
- `git diff --stat` nie pokazuje zmian w `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml` ani `src-tauri/src/store/schema.rs`

**Implementation Note**: Po przejściu bramek automatycznych przejdź do fazy 2 (ta faza nie ma kroków ręcznych; UI jeszcze nie pokazuje danych).

---

## Phase 2: Karta „Dane aplikacji”

### Overview

Karta pokazuje fakty z `appData` i przycisk „Otwórz folder kopii” z błędem inline, także gdy baza jest w błędzie.

### Changes Required:

#### 1. Fakty karty

**File**: `src/health/HealthPanel.tsx`, `src/health/sourceView.ts`

**Intent**: Czyste funkcje w `sourceView.ts` budują fakty z `appData`; `HealthPanel.cardFacts` dokłada je do „Sprawdzono” dla `app-data` niezależnie od stanu źródła.

**Contract**:
- `appDataFacts(appData: AppDataDetails, nowMs: number): SourceFact[]` w kolejności: „Ostatnia kopia” (`formatAgo(atMs, now)`; bez `atMs` data jako `DD.MM.YYYY`; `last === null` → „brak”; `tone: "warn"` przy `stale`), „Kopie” (`${kept} z ${keep}`; `unreadable` → „nie można odczytać folderu” z `tone: "warn"` i bez faktu „Ostatnia kopia”), „Schemat” (`v${schemaVersion}`, pominięty przy `null`).
- Brak `appData` (`null`) → tylko „Sprawdzono”, jak dziś.

#### 2. Ton faktu i akcja w karcie

**File**: `src/health/SourceCard.tsx`, `src/health/Health.css`

**Intent**: `SourceFact` dostaje opcjonalny ton ostrzeżenia; karta dostaje opcjonalne miejsce na akcję pod faktami.

**Contract**:
- `SourceFact = { label: string; value: string; tone?: "warn" }`; `<dd>` z klasą `is-warn` koloruje wartość tokenem ostrzeżenia z `src/styles/tokens.css` (bez literalnych kolorów).
- `SourceCard` przyjmuje opcjonalne `actions?: ReactNode` renderowane pod `<dl>`; inne karty bez zmian.

#### 3. Przycisk „Otwórz folder kopii”

**File**: `src/health/HealthPanel.tsx` (lub mały komponent `src/health/OpenBackupsButton.tsx`)

**Intent**: Przycisk widoczny w karcie `app-data`, gdy `appData !== null`, w każdym stanie źródła. Kliknięcie woła `openBackupsFolder()`; błąd pokazuje się inline pod przyciskiem i znika przy kolejnej udanej próbie.

**Contract**: `Button size="sm" variant="quiet" icon="folder"` z tekstem „Otwórz folder kopii”; `aria-disabled`/`aria-busy` w trakcie, jak „Sprawdź wszystkie” (`HealthPanel.tsx:124`); błąd `openBackupsErrorMessage(e)` w elemencie z `role="alert"` w stylu `health-card-note` z klasą błędu.

### Success Criteria:

#### Automated Verification:

- `npm run build`, `npm run lint` i `npm run format:check` przechodzą
- `cargo test`, `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` nadal przechodzą
- `rg "#[0-9a-fA-F]{3,6}\b" src/health/Health.css` nie znajduje literalnych kolorów

#### Manual Verification:

- W `npm run tauri dev` karta „Dane aplikacji” pokazuje „Sprawdzono”, „Ostatnia kopia” (np. „2 godz. temu”, bez tonu ostrzeżenia), „Kopie N z 7” zgodne z liczbą plików `day-navigator-*.db` w `backups/` i „Schemat v10” (lub aktualną wersję)
- „Otwórz folder kopii” otwiera Eksploratora dokładnie w `<app data dir>/backups/`
- Po tymczasowej zmianie nazwy folderu `backups/` (aplikacja działa) i „Sprawdź wszystkie”: „Ostatnia kopia: brak” w tonie ostrzeżenia, „Kopie 0 z 7”, a przycisk pokazuje inline „Folder kopii nie istnieje.” i nie tworzy folderu; po przywróceniu nazwy i „Sprawdź wszystkie” wartości wracają, a błąd znika po kolejnym kliknięciu
- Pozostałe karty (GitLab, GitHub) i wiersze „Pozostałe źródła” wyglądają jak przed zmianą; zasobnik i toasty bez zmian

**Implementation Note**: Po przejściu bramek automatycznych zatrzymaj się na ręczne potwierdzenie; to ostatnia faza.

---

## Testing Strategy

### Unit Tests:

- `store::backup::summarize`: brak folderu, kopia dzisiejsza, kopia z przyszłości, najnowsza z wczoraj, pliki obce i `.tmp`, folder będący plikiem.
- `backups_folder_to_open`: nieustalony folder, brak folderu (nie jest tworzony), istniejący folder, serializacja błędu.
- Serializacja `HealthSnapshot.appData` i brak ścieżek w JSON.

### Integration Tests:

- `tests/health_probe.rs`: otwarta baza daje `schema_version = MIGRATIONS.last().version` i `Listed` z dzisiejszą kopią (`stale: false`); uszkodzona baza daje `Error` + `schema_version: None` + `Listed` z kopiami.

### Manual Testing Steps:

1. Uruchom `npm run tauri dev`, otwórz „Stan źródeł”, sprawdź fakty karty „Dane aplikacji” i porównaj z plikami w `backups/`.
2. Kliknij „Otwórz folder kopii” — otwiera się `backups/`.
3. Zmień nazwę `backups/` (np. na `backups-x`), kliknij „Sprawdź wszystkie” i „Otwórz folder kopii”; potem przywróć nazwę i sprawdź ponownie.

## Performance Considerations

Jeden `read_dir` na folder z ~7 plikami i jeden `PRAGMA user_version` na rundę (co 30 s) — pomijalne. Migawka rośnie o kilka pól.

## Migration Notes

Brak migracji danych. `HealthSnapshot` zyskuje pole; jedyny konsument to UI w tym samym commicie.

## References

- Roadmap: `context/foundation/roadmap.md` (S-07), PRD FR-022, GitHub issue #4
- Wzorzec otwierania: `src-tauri/src/settings_file.rs:44`, `src/gitlab/api.ts:77`, `src/gitlab/GitlabFilterBar.tsx:48`
- Kopie: `src-tauri/src/store/backup.rs:26-113`; runda: `src-tauri/src/runtime.rs:250-323`; sonda: `src-tauri/src/health/probe.rs:41`
- Karta: `src/health/SourceCard.tsx`, `src/health/HealthPanel.tsx:103`
- Poprzednia zmiana store: `context/archive/2026-11-23-app-data-store/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Kontrakt backendu (Rust + lustro TS)

#### Automated

- [x] 1.1 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy podsumowania kopii, `stale`, stanu błędu bazy, kształtu JSON, braku ścieżek i `backups_folder_to_open`
- [x] 1.2 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzą
- [x] 1.3 `npm run build`, `npm run lint` i `npm run format:check` przechodzą
- [x] 1.4 `rg "invoke" src/health` pokazuje import `invoke` tylko w `src/health/api.ts`
- [x] 1.5 `git diff --stat` nie pokazuje zmian w `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml` ani `src-tauri/src/store/schema.rs`

### Phase 2: Karta „Dane aplikacji”

#### Automated

- [ ] 2.1 `npm run build`, `npm run lint` i `npm run format:check` przechodzą
- [ ] 2.2 `cargo test`, `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` nadal przechodzą
- [ ] 2.3 `rg "#[0-9a-fA-F]{3,6}\b" src/health/Health.css` nie znajduje literalnych kolorów

#### Manual

- [ ] 2.4 Karta pokazuje „Sprawdzono”, „Ostatnia kopia”, „Kopie N z 7” zgodne z `backups/` i „Schemat v10” (lub aktualną wersję)
- [ ] 2.5 „Otwórz folder kopii” otwiera Eksploratora dokładnie w `<app data dir>/backups/`
- [ ] 2.6 Bez folderu `backups/`: „Ostatnia kopia: brak” z ostrzeżeniem, „Kopie 0 z 7”, przycisk pokazuje „Folder kopii nie istnieje.” i nie tworzy folderu; po przywróceniu wartości wracają
- [ ] 2.7 Pozostałe karty, wiersze „Pozostałe źródła”, zasobnik i toasty bez zmian
