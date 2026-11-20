# Source Health Status Implementation Plan

## Overview

F-02 makes the app live in the Windows tray and makes every source failure visible without opening the window. A tray icon shows the overall state: OK, source error, or not running. A Polish Windows toast fires when a source breaks and again when it recovers. Settings and the vault roots are re-probed every 30 s, so a failure shows within 1 minute. A `health_status` command, a `health-changed` event and a health panel prove the "error instead of an empty list" rule end to end. Later slices (S-01, S-07, S-09, S-14, S-17) build on these.

## Current State Analysis

- The app is the Tauri 2 starter plus F-01. `src-tauri/src/lib.rs:18-36` builds the app with the opener plugin and one demo command, `greet`. It loads `settings.json` once in `setup` and stores `VaultState(Result<VaultRegistry, VaultError>)`, which never changes afterwards (`lib.rs:10`, `lib.rs:22-31`).
- There is no tray, notification, single-instance, logging or background thread. `tauri = { version = "2", features = [] }` (`src-tauri/Cargo.toml:22`).
- Closing the window exits the app (default behaviour).
- The only sources that exist today:
  - the settings file (`settings::load_settings`, `settings.rs:52`);
  - the two vault roots (`VaultRegistry::vault`, `vault/mod.rs:61-85`, which returns `SpaceNotConfigured`, `RootMissing` or `RootsOverlap`);
  - stale exclusions (`Vault::unmatched_exclusions`, `vault/mod.rs:302-310`).

  `glab` and `gh` are not called anywhere yet.
- `VaultError` (`vault/error.rs`) holds no note content. Its only absolute paths are `RootMissing::root` and `NotConfigured::settings_path`.
- The UI is the Vite/React demo (`src/App.tsx`).
- The release profile uses `panic = "abort"` (`Cargo.toml:53`), so in release builds a panic kills the whole process. It never kills just one thread.

## Desired End State

Running the app (dev or installed) gives:

- **Tray:** one tray icon in one of three states. Left-click and "Otwórz" open the window. "Sprawdź teraz" re-checks at once. "Zakończ" quits.
- **Window lifecycle:** closing the window hides it to the tray. A second launch focuses the running instance instead of starting another one.
- **Checks:** every 30 s, and on demand, the app re-reads `settings.json`, rebuilds the vault registry in shared state and probes each source:
  - the settings source;
  - each vault root;
  - each space's exclusions.
- **Overall state:** any source in error → red icon. Unconfigured spaces stay neutral. A stalled check loop → grey "nie działa" icon.
- **Toasts:** a source entering an error, or changing error kind, gives one Polish toast per check. A source leaving an error gives one "działa ponownie" toast. A source that stays broken gives no repeats. Toasts pass through a `notices_allowed()` gate that later pause/day-off slices plug into. The tray is never gated.
- **Log:** a rotating file in `%LOCALAPPDATA%\com.sebas.day-navigator\logs` records transitions as source id and error kind only.
- **Window:** the health panel replaces the demo. It lists every source with its Polish state and message, has "Sprawdź teraz", and shows an `ErrorState` (never a blank list) when the health data itself cannot be loaded.

Verify with `cargo test`, `cargo build`, `npm run build` and the manual steps under each phase. The core one: rename the configured vault folder and watch the icon turn red and a toast appear within 60 s. Rename it back and watch the icon turn green and the recovery toast appear.

### Key Discoveries:

- **Tray feature and API:** the tray needs `tauri` feature `tray-icon`. `TrayIconBuilder::with_id(..)` builds it and `app.tray_by_id(..)` finds it again. `TrayIcon::set_icon` and `set_tooltip` are `Send + Sync` and hand off to the main thread, so the background threads can call them (tauri-2.12.7 `src/tray/mod.rs:216-393, 513, 537`).
- **Tray icon images:** `tauri::include_image!` embeds a PNG at compile time with no extra feature (tauri-macros-2.7.6 `src/lib.rs:340-366`).
- **Tooltip limit:** `szTip` holds 128 UTF-16 units. A tooltip of 128 or more loses its terminator, so cap it at 127 (tray-icon-0.25.1 `src/platform_impl/windows/mod.rs:239-244`). Do not also set `app.trayIcon` in `tauri.conf.json`, or a second icon appears.
- **Close to tray:** `WindowEvent::CloseRequested` → `api.prevent_close()` + `window.hide()`. `RunEvent::ExitRequested { code: None, api }` → `api.prevent_exit()`. "Zakończ" calls `app.exit(0)`, where `code` is `Some`. This needs `.build(ctx)?.run(cb)` instead of `.run(ctx)` (tauri `app.rs:86-103, 220-232`).
- **tauri-plugin-notification 2.8.1** (needs tauri ^2.12):
  - `show()` returns `Ok` before Windows displays anything, and OS errors are dropped (`desktop.rs:435-442`). The tray icon is therefore the reliable channel and the toast is a courtesy.
  - Under `tauri dev` toasts show as "Windows PowerShell". Installed NSIS/MSI builds set the AUMID to `com.sebas.day-navigator` on the shortcuts (`desktop.rs:506-518`; bundler `nsis/includes/shortcuts.nsh:52-79`).
  - Focus Assist sends toasts straight to the notification centre without reporting an error.
- **tauri-plugin-single-instance 2.5.5:** it must be the first plugin registered. Its callback gets `(app, args, cwd)`.
- **tauri-plugin-log 2.10.2:**
  - Default file size is 40 KB with `KeepOne`. It also supports `KeepSome(n)`.
  - On Windows it writes to `%LOCALAPPDATA%\{identifier}\logs`.
  - Rust code logs through the `log` macros.
- **No direct `tokio` dependency:** Tauri's tokio has no `time` feature. A `std::thread` loop with `mpsc::Receiver::recv_timeout` gives both the 30 s interval and the "check now" wake-up without adding `tokio`.

## What We're NOT Doing

- `glab`/`gh` login probes and any CLI calls. Those ship with S-07 and S-16, which report into the registry built here. The `CREATE_NO_WINDOW` console-flash concern belongs to them.
- An external watchdog or scheduled task for a dead process. A dead process shows as a missing tray icon (accepted). Autostart is S-02.
- Toast click or action handling (`on_action`), and opening the window from a toast.
- Pause and day-off modes themselves (FR-023, FR-024). Only the `notices_allowed()` gate exists, and it always returns `true` for now.
- Repeated reminders while a source stays broken, and replaying toasts suppressed by the gate.
- Real screens ("Today", lists). Only the health panel and the reusable `ErrorState`.
- Writing settings, a settings UI or a wizard (S-06).
- File-system watchers. Polling every 30 s is enough for the 1-minute rule.
- npm `@tauri-apps/plugin-*` packages. Every plugin here is driven from Rust.
- Changing `productName` or the bundle identifier.

## Implementation Approach

The health logic is a pure Rust module, `health/`, with no `tauri` import, following F-01's "Nothing here imports `tauri`" convention (`vault/mod.rs:1`, `settings.rs:1`). It has three parts:

- **Probes:** settings path → registry result + source statuses.
- **Transition diff:** previous statuses + next statuses → transitions → at most one toast.
- **Presentation rules:** overall state, tooltip text and heartbeat staleness, all without Tauri.

A thin Tauri runtime module (`runtime.rs`) owns the side effects:

- the check thread (30 s `recv_timeout`, woken by "check now");
- the watchdog thread (heartbeat staleness);
- tray icon, tooltip and menu;
- toasts, log lines and the `health-changed` event.

The frontend talks to it only through `health_status`, `check_health_now` and the event. Polish user-facing text is produced in Rust, so the tray, the toast and the panel always say the same thing.

Status model:

- `SourceId`: `Settings`, `Vault(Space)`, `Exclusions(Space)`. Stable keys are `settings`, `vault:work`, `exclusions:private`. Later slices add variants.
- `SourceState`:
  - `Ok`;
  - `NotConfigured` (neutral; never red);
  - `Error { kind, message }`, where `kind` is the English variant name (e.g. `RootMissing`) and `message` is Polish text that may include the user's own configured path, for local UI only.
- `Overall`:
  - `Starting` (grey; before the first check completes);
  - `Ok`;
  - `Error` (any source in `Error`);
  - `NotRunning` (grey; stale heartbeat).

## Critical Implementation Details

**Timing & lifecycle.** The watchdog must not report a system resume as a stall. After sleep or hibernate, the watchdog may wake before the check thread and see an old heartbeat. Treat staleness as real only when the heartbeat age exceeds 75 s **and** the watchdog's own gap since its previous wake-up stayed below that threshold. Otherwise skip that round.

In release builds (`panic = "abort"`), a panicking check kills the process: no icon, the accepted "dead process" case. The grey state therefore covers hangs, such as a canonicalise blocked on an unreachable drive, and thread death in debug builds. Do not add `catch_unwind`.

**State sequencing.** The tray is built in `setup` in the grey `Starting` state before the check thread starts, so there is always an icon. The first check's transitions are computed against an empty previous map:

- `Unknown → Error` notifies;
- `Unknown → Ok` and `Unknown → NotConfigured` are silent.

The tray is updated before the toast, so the reliable channel never lags the courtesy one.

**Data boundary.** Log lines and toast bodies carry source ids, space names and error kinds only. Absolute paths and exclusion entries appear only in the panel's per-source message, never in the tooltip, toast or log. The vault root path may name a client, so it must never reach the log file.

## Phase 1: Health core

### Overview

Pure, tested building blocks: source and status types, the probes over settings and vaults, the transition diff and toast text, the overall-state, tooltip and staleness rules, and a swappable `VaultState` that the periodic reload can replace.

### Changes Required:

#### 1. Health types and rules

**File**: `src-tauri/src/health/mod.rs`

**Intent**: Define the status model and every decision the runtime needs, as pure functions, so they can be unit-tested without Tauri.

**Contract**:
- Module doc says nothing here imports `tauri`.
- Serializable types (serde, `camelCase` for the frontend):
  - `SourceId` (serialised as its stable key);
  - `SourceStatus { id, label, state }`, where `label` is Polish, e.g. "Ustawienia", "Vault Praca", "Wykluczenia Prywatne";
  - `SourceState`;
  - `Overall`;
  - `HealthSnapshot { overall, sources: Vec<SourceStatus>, checked_at_ms: Option<u64> }`.
- `fn transitions(prev: &[SourceStatus], next: &[SourceStatus]) -> Vec<Transition>`. It reports a transition when a source enters `Error`, changes error `kind`, or leaves `Error` for `Ok` (recovery). Message-only changes with the same `kind` are not transitions. Edge rules:
  - a source missing in `prev` counts as Unknown;
  - a source present in `prev` but absent from `next` (e.g. vault sources hidden while settings fail) gives no transition. The caller keeps its last status in the diff baseline, so when it reappears it is compared against that status, not Unknown (a vault that stayed broken through a settings outage does not toast again);
  - `Error → NotConfigured` (a broken space removed from settings) is silent, not a recovery.
- `Transition { id, label, from_kind: Option<String>, to_kind: Option<String> }` carries no message text.
- `fn baseline(prev_baseline, next) -> Vec<SourceStatus>`: `next` plus every baseline entry whose id is absent from `next`. The runtime diffs against this, not against the last snapshot.
- `fn notice_for(&[Transition]) -> Option<Notice { title, body }>`:
  - one toast per call, Polish;
  - errors take precedence in the title, e.g. "Nawigator dnia — błąd źródła", with recovery-only "Nawigator dnia — źródła działają";
  - one body line per transition (`"<label>: <short reason>"` / `"<label>: działa ponownie"`), at most 3 lines, then "i N więcej";
  - the short reason is a fixed Polish phrase chosen by `to_kind` (e.g. `RootMissing` → "folder niedostępny", `SettingsInvalid` → "nieprawidłowe ustawienia", `UnmatchedExclusions` → "wykluczenia nic nie chronią"), never the `message`. `SettingsInvalid.reason` can embed an excluded entry (`settings.rs:108`), so the message must not reach the toast;
  - no absolute paths, exclusion entries or `reason` text.
- `fn overall(sources, heartbeat_stale: bool, started: bool) -> Overall`.
- `fn tooltip(overall, sources) -> String`. Examples: "Nawigator dnia — OK", "Nawigator dnia — błąd: Vault Praca i 1 więcej", "Nawigator dnia — sprawdzanie nie działa", "Nawigator dnia — uruchamianie…". It is capped at 127 UTF-16 units and never cuts a character in half.
- `fn is_stale(heartbeat_age, watchdog_gap, threshold) -> bool`, per the resume rule in Critical Implementation Details.
- `fn notices_allowed() -> bool`, which returns `true` and documents the FR-023/FR-024 rule: the tray is never gated, and suppressed toasts are not replayed.

#### 2. Probes

**File**: `src-tauri/src/health/probe.rs`

**Intent**: Turn the settings file and vault roots into source statuses on each check, reusing the F-01 guards instead of duplicating path logic.

**Contract**: `pub fn probe(settings_path: &Path) -> (Result<VaultRegistry, VaultError>, Vec<SourceStatus>)`.

- **Settings source:**
  - `NotConfigured` → `Error` with the message "Brak pliku ustawień: <path>";
  - `SettingsInvalid` → `Error` with the message "Ustawienia są nieprawidłowe: <reason>";
  - when settings fail, no vault or exclusion sources are listed;
  - settings load but neither space is configured → `Error` with kind `NoSpaceConfigured` and the message "Nie skonfigurowano żadnego vaultu" (PRD NFR "Widoczny stan zdrowia" counts a missing configured path as a failure; a single unconfigured space stays neutral).
- **Per space:** `SpaceNotConfigured` → `NotConfigured` for both the vault and exclusions sources. Otherwise:
  - `registry.vault(space)` errors (`RootMissing`, `RootsOverlap`) → vault `Error`;
  - a root that exists but cannot be listed (`vault.list(None)` fails) → vault `Error`;
  - when the vault source is `Error`, that space's exclusions source is omitted (exclusions cannot be checked without a vault; the absence rule in item 1 applies);
  - non-empty `unmatched_exclusions()` → exclusions `Error` (kind `UnmatchedExclusions`), with the message listing the vault-relative entries and saying they protect nothing;
  - otherwise `Ok`.
- Output order is fixed: settings, vault:work, exclusions:work, vault:private, exclusions:private (omitted sources are skipped).
- A single `fn message_pl(&VaultError) -> String` maps error kinds to Polish text. It covers at least `NotConfigured`, `SettingsInvalid`, `RootMissing` ("Folder vaultu nie istnieje lub jest niedostępny: <root>"), `RootsOverlap` and `Io`. Every other variant falls back to its `Display` text.

#### 3. Swappable vault state

**File**: `src-tauri/src/lib.rs`

**Intent**: Let each health check replace the registry built from a freshly read `settings.json`, so fixing settings turns the icon green within one check without a restart.

**Contract**:
- `VaultState` wraps `RwLock<Arc<Result<VaultRegistry, VaultError>>>`.
- `fn current(&self) -> Arc<Result<..>>` takes a cheap snapshot that later slices call once per command.
- `fn replace(&self, next: Result<..>)`.
- `pub mod health;` is added. `setup` still stores an initial `VaultState`; the runtime in Phase 2 replaces it on each check.

#### 4. Tests

**Files**: unit tests inside `src-tauri/src/health/mod.rs` and `probe.rs`; integration test `src-tauri/tests/health_probe.rs`

**Intent**: Prove the rules and probes on invented data only.

**Contract**:
- **Transitions and notices:**
  - every case named in item 1;
  - first check from an empty `prev`;
  - same kind with a new message → no transition;
  - mixed error and recovery in one tick → one notice with error title;
  - more than 3 lines → "i N więcej";
  - source absent from `next` → no transition; it reappears with the same error kind after a settings outage → no transition (via `baseline`);
  - `Error → NotConfigured` → no transition;
  - no absolute path, exclusion entry or `SettingsInvalid` reason text in any notice body (use an invented entry such as `Klient-Testowy/hasla.md` in the reason).
- **Tooltip:** a proptest that the length is ≤ 127 UTF-16 units and the result is valid for long Polish labels.
- **Staleness:**
  - fresh heartbeat → not stale;
  - old heartbeat with a normal watchdog gap → stale;
  - old heartbeat after a long watchdog gap (resume) → not stale.
- **Probes**, using temp dirs and `tests/common::test_vault()` (never the fixture itself):
  - missing or invalid settings;
  - unconfigured space → `NotConfigured` and the overall state is not `Error`;
  - no space configured → settings `Error` with kind `NoSpaceConfigured`;
  - missing root → vault `Error` and no exclusions source for that space;
  - overlapping roots;
  - unmatched exclusion;
  - a fully OK configuration;
  - a missing root is reported without creating the folder.

### Success Criteria:

#### Automated Verification:

- Health unit and integration tests pass: `cargo test` in `src-tauri/`
- Rust build succeeds: `cargo build` in `src-tauri/`
- `health/` imports nothing from `tauri`: `grep -rn "tauri" src-tauri/src/health/` returns no `use tauri` line
- Frontend still type-checks and builds: `npm run build`

#### Manual Verification:

- Reading the Polish strings in `message_pl`, `notice_for` and `tooltip` confirms they read naturally and expose no absolute path outside the panel message

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Tray and background runtime

### Overview

Wire the pure core into the running app. This phase adds:

- the tray, single instance and close-to-tray;
- the check thread and the watchdog thread;
- tray updates, gated toasts and sanitised log lines.

### Changes Required:

#### 1. Dependencies and assets

**Files**: `src-tauri/Cargo.toml`, `src-tauri/icons/tray/ok.png`, `src-tauri/icons/tray/error.png`, `src-tauri/icons/tray/idle.png`

**Intent**: Enable the tray and add the three plugins and the per-state icons.

**Contract**:
- Cargo changes:
  - `tauri` features `["tray-icon"]`;
  - `tauri-plugin-notification = "2.8"`;
  - `tauri-plugin-single-instance = "2.5"`;
  - `tauri-plugin-log = "2.10"`;
  - `log = "0.4"`;
  - no `tokio`.
- Each icon is a 32×32 RGBA PNG that can be told apart without colour: a check on green, an exclamation mark on red, and a dash on grey. Generate them with a one-off script kept at `context/changes/source-health-status/make-tray-icons.py` (Pillow, `Image.new("RGBA", (32, 32))`) and commit both the script and the PNGs. Embed them with `tauri::include_image!`; it decodes at compile time and fails the build on a non-RGBA PNG (tauri-codegen-2.7.6 `src/image.rs:76-84`).
- `cargo audit` stays clean.

#### 2. Runtime: threads, tray, notices, log

**File**: `src-tauri/src/runtime.rs`

**Intent**: Own every side effect of health. This covers:

- running checks every 30 s and on demand;
- detecting a stalled loop;
- updating the tray before toasting;
- emitting `health-changed`;
- logging transitions.

**Contract**:
- Managed `HealthState`:
  - `snapshot: RwLock<HealthSnapshot>`;
  - `wake: mpsc::Sender<()>` (`Sender` is `Sync`; no `Mutex`);
  - `heartbeat` (monotonic timestamp of the last finished check).
- **Locking rule:** `TrayIcon::set_icon`/`set_tooltip` block until the main thread runs them (tauri `menu/mod.rs:26-39`), and sync commands run on the main thread. Never hold the `snapshot` or `VaultState` lock across a tray, notification or emit call: write the snapshot, drop the guard, then touch the tray. Tray setter errors (returned after the event loop ends) are logged and ignored, never unwrapped.
- **Check thread:** loops on `recv_timeout(30 s)`, draining any extra queued wake-ups before each round. Each round:
  1. runs `health::probe` with the app-config `settings.json`;
  2. calls `VaultState::replace`;
  3. computes transitions against the diff baseline (`health::baseline`, kept by the check thread), then updates the baseline;
  4. stores the snapshot and updates the tray icon and tooltip;
  5. sends `notice_for(..)` through `tauri-plugin-notification` when `notices_allowed()`;
  6. logs each transition at `info` as `health: <source-key> <old-kind> -> <new-kind>`;
  7. emits `health-changed` with the snapshot;
  8. stamps the heartbeat.
- **Watchdog thread:** wakes every 15 s. When `is_stale(..)` first becomes true, it:
  - sets `Overall::NotRunning`;
  - updates the tray;
  - logs it;
  - emits the event;
  - sends one gated toast, "Nawigator dnia — sprawdzanie stanu nie działa".

  It clears the state when the heartbeat resumes.
- A debug-only switch, `#[cfg(debug_assertions)]` env `DAY_NAVIGATOR_STALL_CHECKS=1`, makes the check thread block forever after its first round, for manual verification.

#### 3. App builder

**File**: `src-tauri/src/lib.rs`

**Intent**: Register plugins in the right order, build the tray, hide on close, keep running with the window hidden, and remove the `greet` demo command.

**Contract**:
- Plugin order:
  1. `tauri_plugin_single_instance::init` first; its callback shows, unminimises and focuses the `main` window;
  2. opener;
  3. notification;
  4. log: `Builder::new().clear_targets()` (the default builder already has Stdout and LogDir, and `.target()` adds to them), then a LogDir target plus Stdout under `cfg(debug_assertions)`, max file size 1 MB, `RotationStrategy::KeepSome(3)`, global level `Warn` and `day_navigator_lib` at `Info`.
- `setup`:
  - builds the initial `VaultState`;
  - builds the tray `with_id("health")` in the `Starting` state with a menu of "Otwórz", "Sprawdź teraz" and "Zakończ";
  - sets `show_menu_on_left_click(false)`, and a left-button-up click shows the window;
  - then starts the runtime threads.
- `on_window_event`: `CloseRequested` on `main` → `prevent_close` + `hide`.
- Switch to `.build(ctx)` + `.run(|app, e| ..)`, preventing `ExitRequested` when `code` is `None`.
- "Zakończ" calls `app.exit(0)`.
- No `trayIcon` in `tauri.conf.json`. The window title becomes "Nawigator dnia".

#### 4. Project rule for app-owned folders

**File**: `AGENTS.md`

**Intent**: The hard rule "Read and write only inside folders configured in settings" allows only the distraction blocker outside those folders, but tauri-plugin-log creates and writes `%LOCALAPPDATA%\com.sebas.day-navigator\logs`, and the app already reads `settings.json` from its config dir. Make that exception explicit instead of contradicting the rule silently.

**Contract**:
- Amend the hard rule to name the app's own folders under its bundle identifier (the app config dir holding `settings.json`, and the app log dir) as allowed, alongside the distraction blocker.
- Keep the wording narrow: app-owned state only, never user content or vault data.

### Success Criteria:

#### Automated Verification:

- Rust build succeeds with the tray feature and plugins: `cargo build` in `src-tauri/`
- All tests still pass: `cargo test` in `src-tauri/`
- No new advisories: `cargo audit` in `src-tauri/`
- Frontend still builds: `npm run build`

#### Manual Verification:

- `npm run tauri dev` with a valid settings file shows one green tray icon with tooltip "Nawigator dnia — OK" and no startup toast
- Renaming the configured work vault folder turns the icon red within 60 s and shows one toast (labelled "Windows PowerShell" in dev, expected); renaming it back turns it green with one "działa ponownie" toast; no repeat toasts while it stays broken
- Breaking `settings.json` (invalid JSON) turns the icon red within 60 s; fixing it turns it green without restarting the app
- Closing the window hides it to the tray; left-click and "Otwórz" bring it back; running `src-tauri/target/debug/day-navigator.exe` while dev is running (a second `npm run tauri dev` fails on strictPort 1420) focuses the existing window and adds no second icon; "Zakończ" exits and removes the icon
- With `DAY_NAVIGATOR_STALL_CHECKS=1`, the icon turns grey with tooltip "Nawigator dnia — sprawdzanie nie działa" within about 90 s
- The log file in `%LOCALAPPDATA%\com.sebas.day-navigator\logs` contains transition lines with source keys and error kinds and no absolute paths or exclusion entries
- Idle in the tray with the window hidden, Task Manager shows < 1% CPU for the app processes
- An installed NSIS build (`npm run tauri build`, then install) shows the toast under the app's name instead of "Windows PowerShell"

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Frontend contract and health panel

### Overview

Expose the health snapshot to the webview and replace the demo UI with a health panel. Add a reusable `ErrorState` component, so later screens render an explicit error instead of an empty list.

### Changes Required:

#### 1. Commands

**File**: `src-tauri/src/runtime.rs` (handlers), `src-tauri/src/lib.rs` (registration)

**Intent**: Let the UI read the current snapshot and request an immediate check.

**Contract**:
- `#[tauri::command] async fn health_status(state) -> HealthSnapshot` returns the stored snapshot (`Starting` before the first check). Both commands are `async` so they never run on the main thread (see the Phase 2 locking rule).
- `#[tauri::command] async fn check_health_now(state) -> Result<(), String>` sends on `wake`; if the channel is closed, it returns an error string.
- The `health-changed` event payload is `HealthSnapshot`.
- Registered in `generate_handler!` in place of `greet`. No capability changes, because `core:default` already covers commands and event listening.

#### 2. Frontend health module

**Files**: `src/health/types.ts`, `src/health/useHealth.ts`, `src/health/HealthPanel.tsx`, `src/components/ErrorState.tsx`, `src/App.tsx`, `src/App.css`

**Intent**:
- Show every source with its Polish state and message, plus a "Sprawdź teraz" button and the last-check time.
- Give later screens one hook and one component for the empty-vs-error rule.

**Contract**:
- `types.ts` mirrors `HealthSnapshot`, `SourceStatus`, `SourceState` and `Overall` exactly (camelCase).
- `useHealth()` returns `{ snapshot, loadError, checkNow, source(id) }`. On mount it subscribes to `health-changed` with `listen` first, then invokes `health_status`, and keeps whichever snapshot has the newer `checkedAtMs` (a slow invoke must not overwrite a newer event). On unmount it unlistens through the resolved promise, guarded by a cancelled flag, so StrictMode's double mount leaks no listener. It sets `loadError` when the invoke fails.
- `ErrorState({ title, message, onRetry? })` is the only error rendering.
- `HealthPanel`:
  - renders `ErrorState` when `loadError` is set, never an empty list;
  - otherwise shows one row per source (label, state, message);
  - shows "Sprawdzam…" while the state is `Starting`.
- `App.tsx` renders `HealthPanel`. The demo markup, `reactLogo` import and demo styles are removed. All UI text is Polish.

### Success Criteria:

#### Automated Verification:

- Type check and build pass with strict settings: `npm run build`
- Rust build succeeds: `cargo build` in `src-tauri/`
- All tests pass: `cargo test` in `src-tauri/`
- Demo command removed: `grep -rn "greet" src src-tauri/src` returns nothing

#### Manual Verification:

- The window shows the health panel with settings, vault and exclusions rows in Polish; an unconfigured private space shows as "nie skonfigurowano" without red styling
- Renaming the work vault folder updates the panel row to the error message within 60 s without reloading the window; "Sprawdź teraz" updates it immediately after renaming it back
- With `settings.json` removed, the panel shows the settings error row (not an empty panel) and the tray is red

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Transition diff: enter, kind change, recovery, Unknown at startup, same-kind message change ignored
- Notice text: error-first title, 3-line cap with "i N więcej", no absolute paths
- Tooltip: ≤ 127 UTF-16 units (proptest), Polish characters intact
- Staleness rule including the resume case
- `message_pl` coverage for every listed `VaultError` kind

### Integration Tests:

- `tests/health_probe.rs`: probe over a `test_vault()` copy, covering:
  - OK;
  - missing root, with no folder created;
  - overlap;
  - unmatched exclusion;
  - unconfigured space;
  - missing and invalid settings.

### Manual Testing Steps:

1. Start `npm run tauri dev` with a valid hand-written settings file pointing at a scratch copy of the invented vault; confirm a green icon and no toast.
2. Rename the scratch vault folder; within 60 s confirm a red icon, one toast, the panel error row and a log line without paths.
3. Rename it back; confirm green, one recovery toast and the panel row back to OK.
4. Close the window, reopen from the tray, launch a second instance (`src-tauri/target/debug/day-navigator.exe`), then quit from the tray.
5. Run with `DAY_NAVIGATOR_STALL_CHECKS=1` and confirm the grey state.
6. Install the NSIS build once and confirm the toast carries the app name.

## Performance Considerations

There are two sleeping threads, the check thread (30 s) and the watchdog (15 s). Each check is a settings read plus a few `canonicalize`/`read_dir` calls, so idle CPU should stay well under 1%. RAM is dominated by WebView2; hiding the window does not free it. Recreating the window to save memory is out of scope unless the < 150 MB target is missed.

## Migration Notes

No data migration. `VaultState` changes shape (now swappable), but nothing outside `lib.rs` reads it yet. The `greet` demo command and demo UI are removed.

## References

- Roadmap item: F-02 in `context/foundation/roadmap.md`
- PRD: NFR "Widoczny stan zdrowia", FR-001 (Socrates resolution), US-01 acceptance criterion (missing vault path) in `context/foundation/prd.md`
- Prior change: `context/archive/2026-11-20-safe-vault-access/plan.md` (F-01 deferred "file watching and logging" to F-02)
- Guards reused: `src-tauri/src/vault/mod.rs:61-85` (`VaultRegistry::vault`), `src-tauri/src/vault/mod.rs:302-310` (`unmatched_exclusions`), `src-tauri/src/settings.rs:52` (`load_settings`)
- Tauri docs: https://v2.tauri.app/learn/system-tray/ · https://v2.tauri.app/plugin/notification/ · https://v2.tauri.app/plugin/single-instance/ · https://v2.tauri.app/plugin/logging/

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Health core

#### Automated

- [x] 1.1 Health unit and integration tests pass: `cargo test` in `src-tauri/`
- [x] 1.2 Rust build succeeds: `cargo build` in `src-tauri/`
- [x] 1.3 `health/` imports nothing from `tauri`
- [x] 1.4 Frontend still type-checks and builds: `npm run build`

#### Manual

- [ ] 1.5 Polish strings read naturally and expose no absolute path outside the panel message

### Phase 2: Tray and background runtime

#### Automated

- [ ] 2.1 Rust build succeeds with the tray feature and plugins: `cargo build`
- [ ] 2.2 All tests still pass: `cargo test`
- [ ] 2.3 No new advisories: `cargo audit`
- [ ] 2.4 Frontend still builds: `npm run build`

#### Manual

- [ ] 2.5 Valid settings show one green tray icon, OK tooltip and no startup toast
- [ ] 2.6 Renaming the work vault turns the icon red with one toast within 60 s; renaming back turns it green with one recovery toast; no repeats
- [ ] 2.7 Broken settings.json turns red within 60 s and recovers without restart
- [ ] 2.8 Close hides to tray; tray reopens; second launch focuses; "Zakończ" exits
- [ ] 2.9 Stall switch turns the icon grey within about 90 s
- [ ] 2.10 Log file has transition lines with no absolute paths or exclusion entries
- [ ] 2.11 Idle in tray stays under 1% CPU
- [ ] 2.12 Installed NSIS build shows the toast under the app's name

### Phase 3: Frontend contract and health panel

#### Automated

- [ ] 3.1 Type check and build pass: `npm run build`
- [ ] 3.2 Rust build succeeds: `cargo build`
- [ ] 3.3 All tests pass: `cargo test`
- [ ] 3.4 Demo command removed: `grep -rn "greet" src src-tauri/src` returns nothing

#### Manual

- [ ] 3.5 Health panel lists settings, vault and exclusions rows in Polish; unconfigured space is neutral
- [ ] 3.6 Panel row updates within 60 s after renaming the vault, and immediately on "Sprawdź teraz"
- [ ] 3.7 With settings.json removed, the panel shows the settings error row and the tray is red
