# GitLab — pasek wartości filtra i czas następnej próby: plan implementacji

## Overview

Nad listą „Praca (GitLab)” pojawia się pasek FILTR z wartościami aktywnego filtra (`gitlabFilter`) i przyciskiem „Ustawienia”, który otwiera `settings.json` w domyślnym edytorze. W karcie „GitLab niedostępny” zamiast ogólnego „sprawdzi ponownie przy następnym automatycznym odświeżeniu” stoi interwał i czas następnej próby: „Odświeżam co 10 min — następna próba za 4 min (14:32).” To slice S-03 z milestone'u M-2, issue #6, PRD FR-024.

## Current State Analysis

- UI dostaje tylko `filterActive`, `filterInvalid` i `filteredOut` (`src/gitlab/types.ts:91-96`). Wartości filtra żyją wyłącznie w Rust: `GitlabFilter` / `DimensionRule` w `src-tauri/src/gitlab/filter.rs:14-58`, z nieprzezroczystym `Debug` (prywatność). Wpisy są przy parsowaniu przycinane i zamieniane na małe litery (`src-tauri/src/settings.rs:548-577`); porównanie jest bez rozróżniania wielkości liter.
- Filtr „należy do listy”: ustawia go udana runda (`gitlab_refresh.rs:644-650`), `reshape_kept` przy nieudanej rundzie z zachowaną listą (`gitlab_refresh.rs:313-335`), `act_on_issue` z `kept_filter` (`gitlab_refresh.rs:150-180`), a `next_snapshot` przenosi go z poprzedniej migawki tylko razem z listą (`gitlab/mod.rs:517-528`).
- Interwał: `refresh_interval` czyta `gitlabRefresh.minutes` w każdej rundzie (`gitlab_refresh.rs:518-527`), wątek czeka `recv_timeout(wait)` po zakończeniu rundy (`gitlab_refresh.rs:460-480`). `checked_at_ms` to koniec rundy (`gitlab/mod.rs:475`). Wybudzenie (Odśwież, wejście w widok) zaczyna oczekiwanie od nowa. Nic z tego nie trafia do UI.
- Karta offline: `SourceState kind="calm"` w `src/gitlab/GitlabView.tsx:504-516`. Gdy GitLab jest niedostępny, lista (i tym samym przyszły pasek) jest ukryta (`GitlabView.tsx:556`).
- Aplikacja nie ma ekranu ustawień; ustawienia to `settings.json` w katalogu konfiguracji (`lib.rs:153-157`). Capability celowo nie daje webview `open-path` ani `reveal-in-dir` (`capabilities/default.json`). Plugin opener jest zarejestrowany (`lib.rs:140`); `OpenerExt::open_path` wywołany z Rust nie przechodzi przez capability (`tauri-plugin-opener-2.7.3/src/lib.rs:128`).
- `useNow` i `RELATIVE_TICK_MS` są w `src/components/` (F-01); `time.ts` ma prywatny `clock()` (HH:MM), ale nie eksportuje godziny chwili (`formatHm` to czas trwania).

## Desired End State

- Gdy działa filtr (dowolny wpis include/exclude) albo `closedDays`, nad listą stoi pasek: „FILTR · Projekty: a, b · bez: c · Etykiety: … · Kamienie milowe: … · Zamknięte: 7 dni · [Ustawienia]”. Bez filtra paska nie ma; przy nieprawidłowym bloku jest tylko dotychczasowe ostrzeżenie.
- „Ustawienia” otwiera `settings.json` w domyślnym programie. Brak pliku albo błąd otwarcia to komunikat pod paskiem; plik nigdy nie jest tworzony.
- Karta „GitLab niedostępny” mówi, co ile minut aplikacja próbuje i kiedy następna próba (odliczanie co 30 s plus godzina); w trakcie rundy „sprawdzam teraz…”.
- Wartości filtra trafiają tylko do webview: `Debug` migawki ich nie zawiera, żaden log ich nie formatuje, testy używają zmyślonych wartości.

### Key Discoveries:

- `GitlabSnapshot` derive'uje `Debug` (`gitlab/mod.rs:396`) — nowe pole z wartościami filtra musi mieć nieprzezroczysty `Debug`; `GitlabFilter` już go ma, więc używamy go wprost.
- Flagi rundy (`refresh_setting_invalid`, `filter_invalid`) są przenoszone w `next_snapshot` z `prev` i ustawiane przez rundę (`gitlab/mod.rs:481-484`, `gitlab_refresh.rs:651-652`) — tym samym wzorcem idą `refresh_minutes` i `next_refresh_at_ms`.
- Polski tekst „1–14” / „10 min” w ostrzeżeniu o interwale lustrzy stałe z `settings.rs:368-371` (`GitlabView.tsx:525`).

## What We're NOT Doing

- Brak nowego wywołania `glab`/`gh` i nowego `CliCall`; brak migracji magazynu.
- Brak ekranu ustawień ani edycji filtra w aplikacji; brak nowych uprawnień webview (otwieranie idzie przez komendę Rust).
- Wartości pokazujemy w postaci, w jakiej filtr działa (przycięte, małe litery) — bez przechowywania oryginalnej pisowni.
- Pasek nie pokazuje się bez aktywnego filtra, przy niedostępnym GitLabie ani przy nieskonfigurowanym GitLabie (wtedy nie ma listy).
- Nie zmieniamy logiki oczekiwania wątku, ostrzeżenia o nieprawidłowym `gitlabRefresh.minutes` ani widoku „Stan źródeł”.

## Implementation Approach

Rust liczy wszystko, co jest regułą (który filtr ukształtował listę, efektywny interwał, kiedy kończy się oczekiwanie); UI tylko formatuje. Faza 1 rozszerza kontrakt migawki i dodaje komendę razem z lustrem TS (jeden commit, zgodnie z AGENTS.md). Faza 2 to sam widok.

## Critical Implementation Details

- **Timing & lifecycle** — `next_refresh_at_ms` ustaw tuż przed `*state.write() = next` w `refresh_round`, z `now_ms()` z tego miejsca plus `wait`: wtedy wątek zaczyna `recv_timeout` kilka ms później, a nie przed rundą. Migawka „refreshing” na początku rundy zostawia starą wartość — UI i tak pokazuje wtedy „sprawdzam teraz…”.
- **Prywatność** — nie dodawaj `log::*` z filtrem ani ścieżką; błąd otwarcia loguje tylko rodzaj (`settings: open failed`), bez tekstu błędu systemowego (może zawierać ścieżkę).

## Phase 1: Kontrakt migawki i komenda otwierania ustawień

### Overview

Migawka GitLaba niesie wartości filtra, efektywny interwał i czas następnej próby; nowa komenda `open_settings_file` otwiera `settings.json`. Lustro TS i funkcje `api.ts` w tym samym commicie.

### Changes Required:

#### 1. Serializacja filtra

**File**: `src-tauri/src/gitlab/filter.rs`

**Intent**: `GitlabFilter` i `DimensionRule` stają się serializowalne, żeby migawka mogła je wysłać do webview; `Debug` zostaje nieprzezroczysty. Komentarz modułu mówi, że wartości wychodzą tylko przez serde do UI.

**Contract**: `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]` na obu typach; JSON: `{ projects: {include: [], exclude: []}, labels: {…}, milestones: {…}, closedDays: number | null }`.

#### 2. Pola migawki

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Trzy nowe pola `GitlabSnapshot`. `filter` należy do listy (jak `filter_active`): domyślny przy braku listy, przenoszony z `prev` tylko razem z listą. `refresh_minutes` i `next_refresh_at_ms` to flagi rundy: przenoszone z `prev` w `next_snapshot`, ustawiane przez rundę.

**Contract**:
- `pub filter: GitlabFilter` — filtr, który ukształtował `items`; `GitlabFilter::default()` w `starting()` i w każdej gałęzi `next_snapshot` bez zachowanej listy; `prev.filter.clone()` w gałęzi `keeps`.
- `pub refresh_minutes: u32` — efektywny interwał ostatniej rundy; `DEFAULT_REFRESH_MINUTES` w `starting()`.
- `pub next_refresh_at_ms: Option<u64>` — kiedy kończy się oczekiwanie po ostatniej rundzie; `None` w `starting()`.
- Testy (zmyślone dane, `gitlab.example.invalid`): nieudana runda z zachowaną listą zachowuje `filter`; nieudana bez listy daje domyślny; obie przenoszą pola interwału; `format!("{:?}", snapshot)` z filtrem zawierającym zmyśloną wartość nie zawiera tej wartości; JSON migawki ma klucze `filter.projects.include`, `filter.closedDays`, `refreshMinutes`, `nextRefreshAtMs`.

#### 3. Runda, `reshape_kept`, `act_on_issue`

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Wszędzie, gdzie dziś ustawiane jest `filter_active`, ustawiany jest też `filter` (ten sam filtr). Runda ustawia `refresh_minutes` i `next_refresh_at_ms` niezależnie od wyniku.

**Contract**:
- `refresh_interval` zwraca `(u32 /* minuty */, bool)`; `refresh_round` liczy z tego `Duration` dla wątku. Istniejący test `refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes` dostosowany do minut.
- `refresh_round`: przy `succeeded` — `next.filter = filter.clone()`; zawsze — `next.refresh_minutes = minutes`, `next.next_refresh_at_ms = Some(now_ms().saturating_add(wait_ms))` tuż przed zapisem migawki.
- `reshape_kept`: `next.filter = filter.clone()` obok `filter_active`.
- `act_on_issue`: `snap.filter = filter.clone()` (z `kept_filter`); pola interwału nietknięte.
- Testy: rozszerzyć `act_on_issue_reapplies_the_kept_filter_and_stores_the_counts` (filter == kept), `act_on_issue_with_the_default_filter_reports_no_filter` (filter domyślny), `act_on_issue_keeps_the_refresh_setting_flag` (pola interwału bez zmian), `reshape_kept_applies_this_rounds_filter_and_hidden_records` (filter == filtr rundy).

#### 4. Komenda `open_settings_file`

**File**: `src-tauri/src/settings_file.rs` (nowy), `src-tauri/src/lib.rs`

**Intent**: Otwiera `settings.json` z katalogu konfiguracji aplikacji w domyślnym programie przez `OpenerExt::open_path` po stronie Rust (stała ścieżka, bez argumentów z UI, bez zmiany capability). Brak pliku to błąd — pliku nie tworzymy. Ścieżkę ustawień liczy jedna funkcja, której używa też `setup` w `lib.rs`.

**Contract**:
- `pub fn settings_path(app: &AppHandle) -> Option<PathBuf>` (`app_config_dir()/settings.json`); `lib.rs` `setup` korzysta z niej zamiast własnej kopii.
- `#[derive(Debug, thiserror::Error, Serialize)] #[serde(tag = "kind", content = "detail")] pub enum OpenSettingsError { ConfigDirUnresolved, Missing, OpenFailed }` — bez ścieżek i tekstów systemowych.
- `fn settings_file_to_open(path: Option<PathBuf>) -> Result<PathBuf, OpenSettingsError>` (czysta: `None` → `ConfigDirUnresolved`, nie-plik → `Missing`).
- `#[tauri::command] pub async fn open_settings_file(app: AppHandle) -> Result<(), OpenSettingsError>`; błąd otwarcia → `OpenFailed` + `log::warn!("settings: open failed")`.
- `mod settings_file;` i rejestracja w `generate_handler!` w `lib.rs`.
- Testy (`tempfile`): `None` → `ConfigDirUnresolved`; brakujący plik → `Missing` i plik nadal nie istnieje; istniejący → `Ok`; `serde_json::to_value(OpenSettingsError::Missing)` == `{"kind":"Missing"}`.

#### 5. Lustro TS i API

**File**: `src/gitlab/types.ts`, `src/gitlab/api.ts`

**Intent**: Typy dla nowych pól i błędu oraz funkcja wywołująca komendę z polskimi komunikatami.

**Contract**:
- `FilterRule = { include: string[]; exclude: string[] }`; `GitlabFilterValues = { projects: FilterRule; labels: FilterRule; milestones: FilterRule; closedDays: number | null }`; `GitlabSnapshot` dostaje `filter: GitlabFilterValues`, `refreshMinutes: number`, `nextRefreshAtMs: number | null`.
- `OpenSettingsError = { kind: "ConfigDirUnresolved" } | { kind: "Missing" } | { kind: "OpenFailed" }`.
- `openSettingsFile(): Promise<void>` (`invoke("open_settings_file")`); `openSettingsErrorMessage(e: unknown): string`: ConfigDirUnresolved → „Nie można ustalić folderu ustawień aplikacji.”, Missing → „Brak pliku settings.json w folderze ustawień aplikacji.”, OpenFailed → „Nie udało się otworzyć settings.json.”, inne → „Nieoczekiwany błąd: …”.

### Success Criteria:

#### Automated Verification:

- `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzą
- `cargo test` w `src-tauri/` przechodzi, w tym nowe testy filtra w migawce, pól interwału, `Debug` bez wartości filtra i `settings_file_to_open`
- `npm run lint`, `npm run format:check` i `npm run build` przechodzą

**Implementation Note**: Po zielonych sprawdzeniach automatycznych przejdź do fazy 2; ta faza nie ma sprawdzeń ręcznych (nic nowego nie jest jeszcze widoczne).

---

## Phase 2: Pasek FILTR i linia następnej próby

### Overview

Widok pokazuje pasek z wartościami filtra i przyciskiem „Ustawienia” nad listą oraz interwał i następną próbę w karcie offline.

### Changes Required:

#### 1. Godzina chwili we wspólnych narzędziach

**File**: `src/components/time.ts`, `src/components/index.ts`

**Intent**: Eksport lokalnej godziny `HH:MM` chwili, opartej na istniejącym `clock()`, zamiast kolejnej lokalnej kopii w widoku.

**Contract**: `export function formatClock(ms: number): string`; w `index.ts` osobna linia `export { formatClock } from "./time";` (osobno, żeby nie kolidować z równoległymi zmianami istniejącej linii).

#### 2. Pasek FILTR

**File**: `src/gitlab/GitlabFilterBar.tsx` (nowy), `src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabView.css`

**Intent**: Pasek nad paskiem narzędzi listy (pierwsze dziecko `gitlab-lists` w `Lists`), więc znika razem z listą offline i przy nieskonfigurowanym GitLabie. Widoczny, gdy `filter` ma jakikolwiek wpis albo `closedDays !== null`. Wymiar bez wpisów jest pomijany. Przycisk „Ustawienia” woła `openSettingsFile`; błąd to `Notice tone="error"` pod paskiem, czyszczony przy kolejnym kliknięciu.

**Contract**:
- `GitlabFilterBar({ filter }: { filter: GitlabFilterValues })`; czysta funkcja `hasFilterValues(filter): boolean`.
- Treść: etykieta „Filtr” (`meta`), potem kolejno „Projekty”, „Etykiety”, „Kamienie milowe”: wartości include jako `Chip`, po nich „bez:” i wartości exclude jako `Chip tone="faint"`; „Zamknięte: N dni” przez `plural(n, "dzień", "dni", "dni")`; na końcu `Button variant="quiet" size="sm"` „Ustawienia” z `title="Otwórz settings.json w domyślnym edytorze"` (ikona tylko z istniejących `IconName`, np. `external`).
- Każdy `Chip` ma pełną wartość w `title`; pasek się zawija (`flex-wrap`). CSS w `GitlabView.css` tylko układ (flex, gap, wrap), wartości wyłącznie z `tokens.css`.
- Region z `aria-label="Aktywny filtr"`.

#### 3. Linia następnej próby w karcie offline

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: Koniec tekstu karty „GitLab niedostępny” zastępuje „aplikacja sprawdzi ponownie przy następnym automatycznym odświeżeniu” zdaniem z interwałem i następną próbą; odliczanie z istniejącego `useNow(RELATIVE_TICK_MS)`.

**Contract**: czysta funkcja `retryText(refreshMinutes, nextRefreshAtMs, refreshing, now): string`:
- `refreshing` → „Odświeżam co {m} min — sprawdzam teraz…”
- `nextRefreshAtMs === null` → „Odświeżam co {m} min.”
- `nextRefreshAtMs - now <= 0` → „Odświeżam co {m} min — następna próba lada chwila.”
- inaczej → „Odświeżam co {m} min — następna próba za {ceil(różnica / 60 000)} min ({formatClock(nextRefreshAtMs)}).”
- Tekst karty: „Brak połączenia z siecią firmową (VPN) albo serwer GitLaba nie odpowiada. To normalny stan poza pracą. {retryText}”. Karta zostaje `kind="calm"`.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` i `npm run build` przechodzą
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą (pełny zestaw przed commitem)

#### Manual Verification:

- Z filtrem w `settings.json` (np. projekty include + etykiety exclude + `closedDays`) pasek nad listą pokazuje te wartości; bez bloku `gitlabFilter` paska nie ma; przy nieprawidłowym bloku jest tylko ostrzeżenie, bez paska
- „Ustawienia” otwiera `settings.json` w domyślnym programie
- Bez VPN karta „GitLab niedostępny” pokazuje „Odświeżam co N min — następna próba za X min (HH:MM)”, odliczanie maleje, a po „Sprawdź teraz” widać „sprawdzam teraz…”, potem nową godzinę
- Po zmianie `gitlabRefresh.minutes` (np. na 3) karta po następnej rundzie podaje nowy interwał
- Log aplikacji po kilku rundach i otwarciu ustawień nie zawiera wartości filtra ani ścieżki ustawień

**Implementation Note**: Po zielonych sprawdzeniach automatycznych zatrzymaj się na potwierdzenie sprawdzeń ręcznych przez użytkownika.

---

## Testing Strategy

### Unit Tests:

- `gitlab/mod.rs`: `next_snapshot` — filtr zachowany z listą, domyślny bez listy; pola interwału przenoszone; `Debug` migawki bez wartości filtra; kształt JSON nowych pól.
- `gitlab_refresh.rs`: `act_on_issue` i `reshape_kept` ustawiają `filter`; `act_on_issue` nie rusza pól interwału; `refresh_interval` w minutach.
- `settings_file.rs`: trzy przypadki `settings_file_to_open` (bez tworzenia pliku) i serializacja błędu.

### Integration Tests:

- Brak nowych: runda z `AppHandle` nie jest testowana jednostkowo; istniejące testy integracyjne GitLaba muszą przejść bez zmian.

### Manual Testing Steps:

1. Ustaw w `settings.json` blok `gitlabFilter` z wymyślonymi wpisami w kilku wymiarach i `closedDays`, otwórz „Praca (GitLab)” z VPN — sprawdź pasek.
2. Kliknij „Ustawienia” — otwiera się `settings.json`.
3. Rozłącz VPN, „Sprawdź teraz” — karta offline z interwałem i odliczaniem; odczekaj minutę, aby zobaczyć zmianę.
4. Przejrzyj log aplikacji.

## Performance Considerations

Brak: trzy małe pola w migawce, odliczanie korzysta z istniejącego 30-sekundowego ticku widoku.

## Migration Notes

Brak migracji danych. Zmiana kontraktu migawki (Rust + TS) w jednym commicie.

## References

- Roadmap: `context/foundation/roadmap.md` (S-03), PRD FR-024, issue #6
- Wzorzec flag rundy: `src-tauri/src/gitlab/mod.rs:481-484`, `src-tauri/src/gitlab_refresh.rs:651-652`
- Filtr w liście: `src-tauri/src/gitlab_refresh.rs:150-180`, `:313-335`, `:644-650`
- Poprzedni slice w tym widoku: `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Kontrakt migawki i komenda otwierania ustawień

#### Automated

- [x] 1.1 `cargo fmt --check` i `cargo clippy --all-targets -- -D warnings` w `src-tauri/` przechodzą — b64aaa4
- [x] 1.2 `cargo test` w `src-tauri/` przechodzi, w tym nowe testy filtra w migawce, pól interwału, `Debug` bez wartości filtra i `settings_file_to_open` — b64aaa4
- [x] 1.3 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — b64aaa4

### Phase 2: Pasek FILTR i linia następnej próby

#### Automated

- [x] 2.1 `npm run lint`, `npm run format:check` i `npm run build` przechodzą — 250d709
- [x] 2.2 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` i `cargo test` w `src-tauri/` przechodzą (pełny zestaw przed commitem) — 250d709

#### Manual

- [ ] 2.3 Z filtrem w `settings.json` pasek nad listą pokazuje te wartości; bez bloku `gitlabFilter` paska nie ma; przy nieprawidłowym bloku jest tylko ostrzeżenie, bez paska
- [ ] 2.4 „Ustawienia” otwiera `settings.json` w domyślnym programie
- [ ] 2.5 Bez VPN karta „GitLab niedostępny” pokazuje interwał i następną próbę, odliczanie maleje, po „Sprawdź teraz” widać „sprawdzam teraz…”, potem nową godzinę
- [ ] 2.6 Po zmianie `gitlabRefresh.minutes` karta po następnej rundzie podaje nowy interwał
- [ ] 2.7 Log aplikacji po kilku rundach i otwarciu ustawień nie zawiera wartości filtra ani ścieżki ustawień
