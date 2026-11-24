<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: GitLab — pasek wartości filtra i czas następnej próby

- **Plan**: context/changes/gitlab-filter-bar-retry/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 4 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

Sprawdzenia automatyczne (ponownie uruchomione 2026-11-24): `npm run lint`, `format:check`, `build`, `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test` (565 passed, 0 failed) — wszystkie PASS. Wiersze Manual 2.3–2.7 potwierdzone przez użytkownika.

## Findings

### F1 — Po uśpieniu karta stoi na „lada chwila” do 14 min

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:708 (z :476)
- **Detail**: `next_refresh_at_ms` to czas ścienny, a wątek czeka `recv_timeout(wait)` na zegarze, który na Windows nie liczy uśpienia. Nic nie budzi wątku GitLaba po wybudzeniu systemu (health wykrywa resume tylko dla własnego watchdoga). Po uśpieniu dłuższym niż oczekiwanie karta pokazuje „następna próba lada chwila.” przez resztę interwału. Przyczyna jest starsza niż S-03; nowa linia ją uwidacznia.
- **Fix A ⭐ Recommended**: Przy wykrytym resume w health obudzić też wątek GitLaba (`GitlabRefreshState::wake()`).
  - Strength: Godzina na karcie znów jest prawdziwa, a po wybudzeniu z VPN lista odświeża się od razu.
  - Tradeoff: Zmiana poza `src/gitlab/` (health/runtime), możliwa kolizja przy mergu z S-08.
  - Confidence: MED — wykrywanie resume już istnieje (`health/mod.rs:299`), nie sprawdzono, czy watchdog ma dostęp do `AppHandle`.
  - Blind spot: Zachowanie `recv_timeout` przy uśpieniu nie zostało zmierzone na maszynie.
- **Fix B**: Zostawić zachowanie, zapisać follow-up do osobnego slice'a.
  - Strength: S-03 zostaje w zakresie widoku.
  - Tradeoff: Mylący tekst po każdym uśpieniu, aż follow-up wejdzie.
  - Confidence: HIGH — nic nie psuje, „Sprawdź teraz” zawsze działa.
  - Blind spot: None significant.
- **Decision**: DISMISSED — the premise was wrong: the health watchdog already wakes the GitLab thread when it detects a resume (`src-tauri/src/runtime.rs:351-357`, gap > 30 s, ticking every 15 s). So after waking a round starts within one tick, and "lada chwila" is accurate for that short time.

### F2 — Możliwe zduplikowane klucze React w pasku

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/gitlab/GitlabFilterBar.tsx:26,32
- **Detail**: Klucze `in:${value}` / `ex:${value}`; `parse_filter_list` zamienia wpisy na małe litery, ale nie usuwa duplikatów (`["Acme/X", "acme/x"]` → dwa takie same klucze).
- **Fix**: Dodać indeks do klucza (`in:${i}:${value}`).
- **Decision**: FIXED

### F3 — Długa wartość bez spacji rozpycha pasek

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/gitlab/GitlabView.css:40-43
- **Detail**: `.gitlab-filter-chip { max-width: 100% }` nie łamie tekstu; ścieżka projektu do 255 znaków bez spacji wychodzi poza chip i może dać poziomy scroll.
- **Fix**: Dodać `overflow-wrap: anywhere` do `.gitlab-filter-chip`.
- **Decision**: FIXED

### F4 — `openSettingsErrorMessage` bez gałęzi `default`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/api.ts:77-89
- **Detail**: Nieznany `kind` spada do `Nieoczekiwany błąd: [object Object]`; sąsiednie helpery zwracają `Nieoczekiwany błąd (${kind})`.
- **Fix**: Dodać `default` jak w `hiddenIssueErrorMessage`.
- **Decision**: FIXED

### F5 — Pasek może pokazać stary filtr w stanie błędu bez rekordów ukrytych

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:323-330
- **Detail**: Gdy runda kończy się błędem (nie offline), lista zostaje, a rekordów ukrytych nie da się odczytać, `reshape_kept` wraca wcześnie: `closed` może zniknąć, a `filter` (z `closedDays` albo przy nieprawidłowym bloku) zostaje stary. Pasek zgadza się z listą, którą ukształtował; rozjazd dotyczy tylko „Zamknięte: N dni” i paska obok ostrzeżenia o nieprawidłowym bloku.
- **Fix**: Przy czyszczeniu `next.closed` wyczyścić też `next.filter.closed_days`; przypadek z nieprawidłowym blokiem zaakceptować jako zgodny z listą.
- **Decision**: FIXED — `reshape_kept` clears `next.filter.closed_days` together with `closed`, and the `reshape_kept_without_records_keeps_the_shown_list` test asserts it (break-check: red without the fix). A stale filter next to the invalid-block warning is ACCEPTED: the bar describes the list as it is shown.

### F6 — Druga kopia `settings_path` w `todo/commands.rs`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/todo/commands.rs:25-30
- **Detail**: Plan mówi o jednej funkcji liczącej ścieżkę ustawień; `todo` wciąż ma prywatną kopię.
- **Fix**: Użyć `settings_file::settings_path` i usunąć kopię.
- **Decision**: FIXED

### F7 — Helpery eksportowane z plików komponentów bez potrzeby

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/GitlabView.tsx:65, src/gitlab/GitlabFilterBar.tsx:11
- **Detail**: `retryText` i `hasFilterValues` są `export`, choć używane tylko lokalnie; żaden inny plik widoku nie eksportuje helperów.
- **Fix**: Usunąć `export`.
- **Decision**: FIXED
