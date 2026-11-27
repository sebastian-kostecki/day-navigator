# Windows autostart into the tray Implementation Plan

## Overview

Roadmap S-04 (M-3), issue #17, anchor MS-17 (no new FR). The user can have Nawigator dnia start with Windows into the tray, with the window hidden, by setting `"autostart": { "enabled": true }` in `settings.json`. Autostart is off by default. On every start the app syncs one Windows Run entry with that key (on: make sure the entry exists and is correct; off: remove it). A start by Windows autostart goes straight to the tray; a manual start shows the window as today. The result of the sync, including any failure, is a new health source "Autostart" in "Źródła", so it is never silent.

## Current State Analysis

- `setup` shows the window unconditionally as its last step (`src-tauri/src/lib.rs:172-173`). The window is created hidden (`tauri.conf.json` `app.windows[0].visible: false`), and window-state restores every flag except `VISIBLE` (`lib.rs:139-143`). Skipping that one `show_main` call is enough to start in the tray; all background work (health, reminders, refreshes, tray) already runs on threads started earlier in `setup` (`lib.rs:163-171`).
- The single-instance callback ignores the second process's arguments and always shows the window (`lib.rs:136-138`). The plugin passes the full command line, program path first (`tauri-plugin-single-instance` Windows impl collects `std::env::args_os()`).
- The app never reads its own command line today (only the test-only `bin/fake_cli.rs` does).
- Close hides the window and only "Zakończ" quits (`lib.rs:176-187`, `:226-235`), so a tray-only process stays alive.
- Settings are read per block, never cached, never written. The newest block loader is `settings/work_time.rs:31-88` (Raw struct with `deny_unknown_fields`, missing file / absent / `null` = default, everything else invalid, unit error type without detail); it is wired with one `mod` and one `pub use` line (`src-tauri/src/settings.rs:16-17`). `check_settings` ignores unknown top-level keys (`settings.rs:50-53`), so a new block breaks nothing.
- Health: `SourceId` with stable keys (`src-tauri/src/health/mod.rs:24-48`), `SourceState` `Ok | NotConfigured | Unavailable{message} | Error{kind,message}` (`:62-78`), fixed toast phrases per error kind in `short_reason` (`:192-215`). `probe_round` assembles the sources each round (`src-tauri/src/runtime.rs:363-387`); `check_round` feeds it the GitLab/GitHub snapshots from managed state (`runtime.rs:410-419`). Transitions give the red tray, toast, event-log entry and log line with no new code. The fixed-text pattern for a block source is `reminders_status` / `REMINDERS_INVALID_PL` (`src-tauri/src/health/probe.rs:132-156`).
- The UI renders any non-card source as a generic row under "Pozostałe źródła" (`src/health/HealthPanel.tsx:118,177`; `src/health/sourceView.ts:50-57,86-88`); `SourceStatus.id` is a plain `string` (`src/health/types.ts:13-17`). The health view needs no change.
- The event log stores a source transition by `SourceId::key()` and reads it back through a closed list, `events::source_from_key` (`src-tauri/src/events/mod.rs:167-177`); an unknown key makes the whole of today's list fail as `Damaged` ("event record is unreadable", `:320-324`, `:350-355`, `:416-426`). Its TS mirror `EventSourceId` is a closed union (`src/events/types.ts:7-8`), and `tests/events.rs:180-205` (`every_source_id_round_trips`) matches every `SourceId` with no wildcard. A new `SourceId` must be added in all three places.
- The release profile sets `panic = "abort"` (`src-tauri/Cargo.toml:77-82`): a panic on any thread ends the whole app.
- `winreg 0.56.0` is already in `Cargo.lock` as a build-only dependency (`tauri-build` → `tauri-winres` → `embed-resource`); `[target.'cfg(windows)'.dependencies]` holds `windows-sys 0.61` (`src-tauri/Cargo.toml:55-56`).
- CI builds, lints and tests Rust only on `windows-latest`, in the debug profile (`.github/workflows/ci.yml:59-60`).
- No lessons file exists (`context/foundation/lessons.md` absent). No metadata helper script ships with the skill in this repo; `change.md` is patched by hand.

## Desired End State

- `settings.json` accepts a new optional block `"autostart": { "enabled": true }`. Absent, `null`, `{}`, `"enabled": null` and `"enabled": false` mean off; `true` means on; anything else (another type, an unknown key, a non-object block, a file-level problem) is invalid.
- In an installed (release) build, at start, on its own short thread:
  - on: the value `com.sebas.day-navigator` under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` holds exactly `"<current exe path>" --autostart` (written when missing or different);
  - off: that value is removed if present;
  - invalid block or unreadable value: nothing is written or removed.
- A process started with the argument `--autostart` skips `show_main` and stays in the tray, whatever the key says. A second launch with `--autostart` while the app runs does not show the window; any other second launch shows it as today.
- "Źródła" → "Pozostałe źródła" has a row "Autostart":

  | Situation | State | Text |
  |---|---|---|
  | key off, value absent (or removed now) | `NotConfigured` | (legend: "Brak wpisu w settings.json — źródło jest pominięte.") |
  | key on, value correct (or written now) | `Ok` | — |
  | debug build, key on | `Unavailable` (calm) | `AUTOSTART_DEV_PL` |
  | invalid block | `Error`, kind `SettingsInvalid` | `AUTOSTART_INVALID_PL` |
  | registry read/write/remove failed, exe path unknown, or sync thread not started | `Error`, kind `AutostartFailed` | `AUTOSTART_FAILED_PL` with the `io::ErrorKind` name only |

  An `Error` turns the tray red and toasts once ("Autostart: nieprawidłowe ustawienia" / "Autostart: nie udało się ustawić autostartu") through the existing health transitions.
- Debug builds (`npm run tauri dev`, `tauri build --debug`, `cargo test`) never read or write the registry.
- AGENTS.md carries one narrow exception for that one Run value; the PRD no longer lists autostart as "not covered"; README documents the key, the restart timing, Task Manager and uninstall; CHANGELOG has one bullet.

Verify with `cargo test` (pure rules with an in-memory fake) and in an installed release build (sign out/in, `reg query`).

### Key Discoveries:

- One condition at `lib.rs:173` plus one in the single-instance callback (`lib.rs:136-138`) is the whole hidden-start change.
- `tauri-plugin-autostart` 2.7.0 (via `auto-launch 0.6`) writes the exe path unquoted, tries HKLM first and overwrites the Task Manager "disabled" flag (`research.md` §3); the installed path `%LOCALAPPDATA%\Nawigator dnia\day-navigator.exe` contains a space. A direct HKCU write with `winreg` avoids all three and adds no new crate.
- The window-state plugin never restores `VISIBLE`, so it cannot pop the window up after a hidden start (`lib.rs:141`). A saved maximized state does not show it either: the plugin calls `maximize()` on the hidden window, and tao 0.37 skips `ShowWindow(SW_MAXIMIZE)` while the window is not visible (`tao-0.37.3/src/platform_impl/windows/window_state.rs`, "Skip this if the window is hidden"). On "Zakończ" the plugin saves the restored size and position of the never-shown window.
- The single-instance plugin (2.5.5, Windows) forwards `std::env::args_os()` of the second process, program path first, and that process exits before `setup` runs, so it never starts a second sync.
- The GitLab "seen" mark is never sent on mount (`src/gitlab/useGitlabSeenMark.ts:10-37`), and data hooks refetch on `visibilitychange`/`focus`, so a hidden start marks nothing as seen and the first show loads fresh data.
- In this crate, `pub` items in `pub mod`s are never dead code, so the pure rules can land in Phase 1 before anything calls them.

## What We're NOT Doing

- No `tauri-plugin-autostart`, no npm package, no capability or CSP change, no Tauri command, no UI change. The only TS change is the key `"autostart"` added to the `EventSourceId` mirror (`src/events/types.ts`), in the same commit as `SourceId::Autostart`.
- The app never writes `settings.json`; there is no in-app switch.
- No HKLM entry, no Task Scheduler task, no Startup-folder shortcut.
- No read or write of `Explorer\StartupApproved\Run` (the Task Manager "disabled" flag): the user's Task Manager choice is respected and not shown.
- No NSIS uninstall hook; README tells the user how to clean up before uninstalling.
- No re-sync while the app runs: the key is read once per start; an edit takes effect at the next start.
- No test against the real registry, not even a scratch HKCU subkey.
- No new FR, no roadmap edit (the task forbids editing `context/foundation/roadmap.md`, so the skill's roadmap status sync is skipped).

## Implementation Approach

Pure rules first, side effects last, following the repo split (`settings/` and `health/` are `tauri`-free; side effects live in thin glue):

1. **Phase 1** adds the `autostart` settings block, the `Autostart` source id (with its event-log key and TS mirror) and a `tauri`-free `autostart` module: the argument check, the sync decision, the sync driven through a `RunEntry` trait, and the mapping from the result to a `SourceStatus`. Everything is tested with an in-memory fake. Nothing calls it yet.
2. **Phase 2** adds the one production `RunEntry` (HKCU Run value via `winreg`), the start-up glue (managed state, one thread, wake the health round), the few lines in `lib.rs` / `runtime.rs`, and the AGENTS.md exception in the same commit, so the rule and the first registry write land together.
3. **Phase 3** updates README, CHANGELOG and PRD wording.

New logic sits in new files (`settings/autostart.rs`, `autostart/mod.rs`, `autostart/hkcu.rs`, `autostart/startup.rs`); shared files (`lib.rs`, `runtime.rs`, `settings.rs`, `health/mod.rs`, `events/mod.rs`, `tests/events.rs`, `src/events/types.ts`, `Cargo.toml`, `Cargo.lock`, README, CHANGELOG, AGENTS.md, PRD) get a few local lines each, because S-02, S-05, S-07 and S-08 edit some of them in parallel.

## Critical Implementation Details

- **Debug no-op without dead code.** CI runs `cargo clippy --all-targets -- -D warnings` in the debug profile only. Choose "apply to the registry" with the `cfg!(debug_assertions)` macro as a plain `bool` (`let apply = !cfg!(debug_assertions);`), never with a `#[cfg(...)]` attribute on the call, so the `HkcuRun` path is compiled, type-checked and linted in debug builds and is not flagged as dead code.
- **Ordering in `setup`.** `autostart::startup::start` must run before `runtime::start` (it manages `AutostartState`, which the first health round reads), and `check_round` reads that state with `try_state`, so a missing state can never panic the health thread.
- **Argument check.** The single-instance callback's `args` start with the program path; both callers pass the arguments after the program name. The check is an exact match on `--autostart`; no other argument is parsed.
- **No panic on the sync path.** The release profile aborts on panic (`Cargo.toml:81`), so a panic in `autostart-sync` (or in `startup::status`, which the health thread calls) would end the app at sign-in with nothing visible. `startup.rs` and `hkcu.rs` use no `unwrap`/`expect`/indexing; the `AutostartState` mutex is locked with `unwrap_or_else(|p| p.into_inner())`, as in `runtime.rs`.
- **Shared-file merges.** `settings.rs:16-17`, the README example/table and the PRD line `prd.md:234` are also touched by parallel slices (S-02 edits the `workTime` example line and table row; S-08 edits the `workTime` paragraph at `README.md:125`; S-07/S-08 reword the same PRD line, which also lists "zamknięcie dnia" and "widok tygodnia"). Add new lines next to them rather than reflowing existing ones, and never on a line adjacent to one another slice edits: in README insert the example line and the table row between the `todo` and `reminders` lines (so no existing line gains a comma), and the paragraph after the `workTime` paragraph with the blank line kept between them; in the PRD remove only the word "autostart" from that sentence and put the new sentence on its own line. `probe_round` is touched by no other open slice (S-07 edits only `run_tray_action` in `runtime.rs`).

## Phase 1: Autostart settings block and pure rules

### Overview

The `autostart` block loader, the `Autostart` source id, and a `tauri`-free module that decides and performs the sync against any `RunEntry` and turns the result into a `SourceStatus`. Fully unit-tested with an in-memory fake; not wired into start-up yet.

### Changes Required:

#### 1. Settings block loader

**File**: `src-tauri/src/settings/autostart.rs` (new), `src-tauri/src/settings.rs`

**Intent**: Read only the top-level `autostart` block, on its own, exactly like `settings/work_time.rs`, so a broken block never affects other blocks and the file stays read-only.

**Contract**: `pub struct AutostartSettings { pub enabled: bool }` (`Default` = `enabled: false`); unit error `pub struct AutostartSettingsInvalid` (no detail, serde errors can quote values); `pub fn load_autostart_settings(path: &Path) -> Result<AutostartSettings, AutostartSettingsInvalid>`. Raw struct `#[serde(deny_unknown_fields, rename_all = "camelCase")] { #[serde(default)] enabled: Option<serde_json::Value> }`. Value match: absent / `null` / `false` → off, `true` → on, anything else → invalid. Missing file → default; any other read error, non-UTF-8, bad JSON, `version` ≠ 1, non-object block → invalid; UTF-8 BOM accepted. Doc comment shows the JSON shape. In `settings.rs`, one `mod autostart;` and one `pub use autostart::{AutostartSettings, AutostartSettingsInvalid, load_autostart_settings};` line next to the `work_time` lines.

#### 2. Source id, toast phrase and event-log key

**File**: `src-tauri/src/health/mod.rs`, `src-tauri/src/events/mod.rs`, `src-tauri/tests/events.rs`, `src/events/types.ts`, `src-tauri/src/store/schema.rs` (migration v13 after the M-3 wave 2 integration, written as v12 in p1: rebuilds `events` so its `source_id` CHECK accepts `'autostart'`)

**Intent**: Give the new source a stable key and a fixed toast phrase for its failure kind, and make the event log read back the transitions the health round records for it.

**Contract**: `SourceId::Autostart` (doc: the `autostart` block and the Windows Run entry, "Autostart"), key `"autostart"`; `short_reason("AutostartFailed") = "nie udało się ustawić autostartu"`. Extend `source_keys_are_stable` and add a notice-line test ("Autostart: nie udało się ustawić autostartu", "Autostart: nieprawidłowe ustawienia"). Add `SourceId::Autostart` to the list in `events::source_from_key`; in `tests/events.rs` `every_source_id_round_trips` add the `SourceId::Autostart => "autostart"` arm and the variant to `sources`. In the same commit, `EventSourceId` in `src/events/types.ts` gains `| "autostart"` (AGENTS.md: Rust type and TS mirror together). Without this, the first Autostart error or recovery makes today's event log unreadable (`Damaged`) for the rest of the day.

#### 3. Pure autostart rules

**File**: `src-tauri/src/autostart/mod.rs` (new; `pub mod autostart;` added to `lib.rs` in this phase so the module compiles and its tests run)

**Intent**: Hold every autostart decision in plain Rust with no `tauri` import, so the whole behaviour except the real registry call is unit-tested.

**Contract**:

- `pub const AUTOSTART_ARG: &str = "--autostart";`
- `pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";` and `pub const VALUE_NAME: &str = "com.sebas.day-navigator";` (plain strings, no registry type; `hkcu.rs` uses them). A unit test here asserts `VALUE_NAME` equals the `identifier` in `tauri.conf.json` (read with `include_str!("../../tauri.conf.json")` and `serde_json`).
- `pub fn started_by_autostart<I, S>(args: I) -> bool where I: IntoIterator<Item = S>, S: AsRef<std::ffi::OsStr>` — true when any given argument equals `AUTOSTART_ARG` exactly (callers pass the arguments after the program name).
- `pub fn expected_value(exe: &Path) -> Option<String>` — `"\"<exe>\" --autostart"`; `None` when the path is not valid UTF-8.
- `pub trait RunEntry { fn read(&self) -> io::Result<Option<String>>; fn write(&self, value: &str) -> io::Result<()>; fn remove(&self) -> io::Result<()>; }` — `read` of a missing value (or missing key) is `Ok(None)`; `remove` of a missing value is `Ok(())`.
- `pub enum SyncAction { Nothing, Write, Remove }` and `pub fn sync_action(enabled: bool, current: Option<&str>, expected: Option<&str>) -> SyncAction`: on + equal → `Nothing`; on + missing or different → `Write`; off + present → `Remove`; off + missing → `Nothing`. (`expected` is only consulted when on.)
- `pub enum Outcome { On, Off, DevOnly, Invalid, Failed { step: FailedStep, kind: io::ErrorKind } }` with `FailedStep { Read, Write, Remove, ExePath, Thread }`.
- `pub fn run(settings: Result<AutostartSettings, AutostartSettingsInvalid>, apply: bool, entry: &dyn RunEntry, exe: Option<&Path>) -> Outcome`:
  - `Err(_)` → `Invalid`, no `entry` call;
  - `apply == false` → `DevOnly` when on, `Off` when off, no `entry` call;
  - on with `exe` `None` or `expected_value` `None` → `Failed { ExePath, .. }` (kind `NotFound` / `InvalidData`), no `entry` call;
  - otherwise `read`; a read error → `Failed { Read, kind }`, nothing written; then the `sync_action` result is applied once; a write/remove error → `Failed { Write | Remove, kind }`; success → `On` / `Off`.
- `pub fn status(outcome: &Outcome) -> SourceStatus` with label `"Autostart"` and id `SourceId::Autostart`: `On` → `Ok`; `Off` → `NotConfigured`; `DevOnly` → `Unavailable { AUTOSTART_DEV_PL }`; `Invalid` → `Error { "SettingsInvalid", AUTOSTART_INVALID_PL }`; `Failed` → `Error { "AutostartFailed", AUTOSTART_FAILED_PL with the step phrase and "({kind:?})" }`.
- Fixed Polish texts (no path, no value data, no user values):
  - `AUTOSTART_INVALID_PL`: "Blok „autostart” w settings.json jest nieprawidłowy: enabled to true albo false. Wpis autostartu w Windows zostaje bez zmian."
  - `AUTOSTART_DEV_PL`: "Autostart działa tylko w zainstalowanej wersji; ta kompilacja nie zmienia wpisu w Windows."
  - `AUTOSTART_FAILED_PL`: "Nie udało się {step} ({kind}). Aplikacja spróbuje ponownie przy następnym uruchomieniu." with step phrases "odczytać wpisu autostartu w rejestrze Windows", "zapisać wpisu autostartu w rejestrze Windows", "usunąć wpisu autostartu z rejestru Windows", "ustalić ścieżki programu", "uruchomić sprawdzania autostartu".

#### 4. Unit tests

**File**: `src-tauri/src/settings/autostart.rs`, `src-tauri/src/autostart/mod.rs` (`#[cfg(test)]` modules)

**Intent**: Cover every branch with invented data and an in-memory fake, never the real registry.

**Contract**:

- Loader, mirroring `settings/work_time.rs:119-243`: missing file; absent / `null` block; `{}`; `enabled: null`; `true`; `false`; refused values (`"true"`, `1`, `0`, `"yes"`, `[]`, `{}`); non-object block; unknown key (`"enable"`); file-level problems (bad JSON, no or wrong `version`, non-UTF-8); a directory as path; BOM; a broken `autostart` block leaves `workTime` / `reminders` / `todo` loading, and a broken `reminders` block leaves `autostart` loading.
- `started_by_autostart`: empty, exact match, among other args, near misses (`--Autostart`, `--autostart=1`, `-autostart`, `autostart`), `OsString` and `String` inputs.
- `expected_value`: an invented path with a space (`C:\Users\Test User\AppData\Local\Nawigator dnia\day-navigator.exe`) → quoted value with ` --autostart`.
- `sync_action`: full table of `(enabled, current, expected)`.
- `run` with a fake `RunEntry` (a `RefCell<Option<String>>` plus injectable `io::Error`s for read/write/remove, and a call counter): every `Outcome` branch; invalid and `apply == false` make no entry call; an existing equal value is not rewritten; a different value is overwritten; read failure writes nothing.
- `status`: every `Outcome` maps to the table in "Desired End State"; messages contain no invented path fragment (`Test User`, `C:\`) and serialize with `id == "autostart"`.
- `VALUE_NAME` equals the `tauri.conf.json` identifier (no registry access).
- Event log: `every_source_id_round_trips` covers `Autostart` (see §2).

### Success Criteria:

#### Automated Verification:

- Rust tests pass, including the new `settings::autostart` and `autostart` tests: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- The new rule files import nothing from `tauri`: `git grep -n -E "use tauri|tauri::" -- src-tauri/src/autostart/mod.rs src-tauri/src/settings/autostart.rs` returns nothing
- The only frontend change is the `EventSourceId` key; no capability, config or dependency change: `git diff --name-only HEAD -- src` lists only `src/events/types.ts`, and `git diff --quiet HEAD -- src-tauri/capabilities src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock`
- The event log knows the new key: `git grep -n "SourceId::Autostart" -- src-tauri/src/events/mod.rs src-tauri/tests/events.rs` finds `source_from_key` and both lines of `every_source_id_round_trips`, and `git grep -n "\"autostart\"" -- src/events/types.ts` finds `EventSourceId`

**Implementation Note**: Commit as `feat(windows-autostart): settings block and pure autostart rules (p1)`.

---

## Phase 2: Registry entry, start-up sync and hidden start

### Overview

The one production `RunEntry` (the HKCU Run value via `winreg`), the start-up glue that runs the sync once on its own thread and feeds the health round, the hidden start for `--autostart`, and the narrow AGENTS.md exception, all in one commit.

### Changes Required:

#### 1. Dependency

**File**: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`

**Intent**: Use the already-locked `winreg` as a normal Windows-only dependency instead of adding a plugin.

**Contract**: one line `winreg = "0.56"` under `[target.'cfg(windows)'.dependencies]`; `Cargo.lock` only gains the `winreg` edge on `day-navigator` (no new package) and is committed with `Cargo.toml` (CI runs clippy, build and test with `--locked`, `ci.yml`). `cargo audit` stays clean (it runs on `ubuntu-latest` from the lock file; `winreg 0.56.0` is already in it). Only the `windows-latest` job compiles Rust, so the `cfg(windows)` dependency is always built and linted in CI.

#### 2. HKCU Run value

**File**: `src-tauri/src/autostart/hkcu.rs` (new, `#[cfg(windows)] mod hkcu;` in `autostart/mod.rs`)

**Intent**: The only code in the app that touches the registry: one fixed value under one fixed key in HKCU.

**Contract**: `pub struct HkcuRun;` implementing `RunEntry` with the fixed `super::RUN_KEY` and `super::VALUE_NAME` (Phase 1); REG_SZ; `read` maps a missing key or value (`io::ErrorKind::NotFound`) to `Ok(None)`; `remove` maps `NotFound` to `Ok(())`; HKCU only, never HKLM, never `StartupApproved`. `hkcu.rs` has no `#[cfg(test)]` module (the identifier test lives in `autostart/mod.rs`), so no test can reach `HkcuRun` through it.

#### 3. Start-up glue

**File**: `src-tauri/src/autostart/startup.rs` (new, `pub mod startup;` in `autostart/mod.rs`; the declaration imports nothing, so `mod.rs` stays `tauri`-free)

**Intent**: Run the sync once per start off the main thread and make its result a health source, waking the health round when it is known.

**Contract**:

- No `#[cfg(test)]` module and no `unwrap`/`expect` (release builds abort on panic); the mutex is locked with `unwrap_or_else(|p| p.into_inner())`.

- `pub struct AutostartState(Mutex<Option<SourceStatus>>)`.
- `pub fn start(app: &AppHandle, settings_path: Option<PathBuf>)`: manages `AutostartState(None)`; with `None` settings path does nothing more (the "Ustawienia" source already reports it, like `reminders`); otherwise spawns a thread named `autostart-sync` that loads the block, calls `autostart::run(settings, !cfg!(debug_assertions), &HkcuRun, std::env::current_exe().ok().as_deref())` (on non-Windows targets, which the app does not ship for, `apply` is always `false` and a private no-op `RunEntry` stands in for `HkcuRun`), stores `status(&outcome)`, logs one value-free line (`info` for On/Off/DevOnly with the action taken, `warn` with the step and `io::ErrorKind` for failures), and calls `runtime::refresh_tray(app)`. A failed thread spawn stores `Failed { Thread, kind }` instead of staying silent.
- `pub fn status(app: &AppHandle) -> Option<SourceStatus>` via `try_state` (absent until the sync finishes, which `health::transitions` treats as silent).

#### 4. Wiring

**File**: `src-tauri/src/lib.rs`, `src-tauri/src/runtime.rs`

**Intent**: Hide the window on an autostart launch and add the source to every health round, with a few local lines.

**Contract**:

- `lib.rs` single-instance callback: show the window only when `!autostart::started_by_autostart(args.iter().skip(1))`.
- `lib.rs` `setup`: call `autostart::startup::start(app.handle(), settings_path.clone())` before `runtime::start`; replace the final unconditional `runtime::show_main` with a call guarded by `!autostart::started_by_autostart(std::env::args_os().skip(1))`, keeping the existing comment.
- `runtime.rs`: `probe_round` takes one more `autostart: Option<SourceStatus>` and appends it after reminders (`sources.extend(autostart)`); `check_round` passes `autostart::startup::status(&app)`; update the `probe_round` doc comment.

#### 5. AGENTS.md exception

**File**: `AGENTS.md`

**Intent**: Record the one write outside the app's folders as a narrow, explicit exception in the hard rule, and name the only module allowed to do it.

**Contract**: Append to the "Outside folders configured in settings…" rule (line 11): "The one write outside these folders: in a release build, while `autostart.enabled` is `true`, one value `com.sebas.day-navigator` under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` holding the quoted exe path and `--autostart`; it is removed when the key is off. No other registry value is read or written (`StartupApproved` included)." Add a Project Structure bullet: "`src-tauri/src/autostart/` — the only code that touches the Windows registry (`hkcu.rs`); tests use the in-memory `RunEntry` fake, never the real registry."

### Success Criteria:

#### Automated Verification:

- Rust tests pass: `cargo test` (in `src-tauri/`)
- Formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Dependency audit passes: `cargo audit` (in `src-tauri/`)
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Registry code lives only in `hkcu.rs`: `git grep -n -E "winreg|HKEY_" -- src-tauri/src` lists only `src-tauri/src/autostart/hkcu.rs`
- No test uses the real registry: `git grep -n "HkcuRun" -- src-tauri/src src-tauri/tests` lists only `autostart/hkcu.rs` and `autostart/startup.rs`, and `git grep -n "cfg(test)" -- src-tauri/src/autostart/hkcu.rs src-tauri/src/autostart/startup.rs` returns nothing
- `winreg` is a normal dependency of the app with no new package: `cargo tree -i winreg -e normal` (in `src-tauri/`) shows `day-navigator`, `git diff HEAD -- src-tauri/Cargo.lock` adds no `[[package]]` and only the `winreg` line in `day-navigator`'s dependencies, and `cargo build --locked` passes (CI uses `--locked`)
- No frontend, capability or config change: `git diff --quiet HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json`

#### Manual Verification:

- Installed release build without an `autostart` block: the "Autostart" row reads NIESKONFIGUROWANE and `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v com.sebas.day-navigator` finds nothing
- With `"autostart": { "enabled": true }` and the app restarted: the row reads OK and the value is `"<install dir>\day-navigator.exe" --autostart`, quoted
- Sign out and in: the app is in the tray, the window stays hidden, the tray tooltip is OK; opening it from the tray shows fresh data. Repeat once after a session that ended with the window maximized: it still stays hidden until opened, then opens maximized
- A manual start shows the window as before; a second start while running shows it; running the exe with `--autostart` while it runs does not show it
- After a tray-only session and "Zakończ", the next manual start restores the saved window size and position
- With `"enabled": false` and the app restarted: the value is gone and the row reads NIESKONFIGUROWANE
- With autostart disabled in Task Manager → Startup apps: after a restart of the app it is still disabled there and the app does not start at the next sign-in
- With `"enabled": "yes"`: red tray, toast "Autostart: nieprawidłowe ustawienia", the row shows the fixed text, and the Run value is unchanged
- Debug build (`npm run tauri build -- --debug --no-bundle`) with the key on: the row reads NIEDOSTĘPNY with the "tylko w zainstalowanej wersji" text and the Run value is untouched

**Implementation Note**: Commit as `feat(windows-autostart): sync the Windows Run entry and start hidden (p2)`. The manual rows need an installed release build and a Windows sign-in; they stay unchecked until the user runs them.

---

## Phase 3: User docs and PRD wording

### Overview

README, CHANGELOG and PRD describe the new key; no code change.

### Changes Required:

#### 1. README

**File**: `README.md` ("## Configuration", lines 92-125)

**Intent**: Document the key where the other blocks are documented, including the timing and the clean-up steps.

**Contract**: add the line `"autostart": { "enabled": true },` to the JSON example between the `todo` and `reminders` lines (no existing line changes; S-02 edits the `workTime` line); add an `autostart` table row between the `todo` and `reminders` rows ("`enabled`: `true` starts the app with Windows into the tray, window hidden; off by default. Read at start: restart the app after a change."); add a short paragraph after the `workTime` paragraph, keeping the blank line between them (S-08 edits that paragraph): the app keeps one entry named `com.sebas.day-navigator` in the current user's Run key, pointing at the copy of the app that last started with the key on, and removes it when the key is off; disabling it in Task Manager → Startup apps also works and the app leaves that choice alone (the row still reads OK); before uninstalling, set `enabled` to `false` and start the app once, or remove the entry in Task Manager; the result shows as "Autostart" in **Źródła**.

#### 2. CHANGELOG

**File**: `CHANGELOG.md`

**Intent**: One user-facing bullet for the next release.

**Contract**: one bullet under `## [Unreleased]` → `### Added`: the app can start with Windows into the tray (window hidden) with `"autostart": { "enabled": true }` in `settings.json`; off by default; the state shows as "Autostart" in "Źródła".

#### 3. PRD wording

**File**: `context/foundation/prd.md` (line 234, "Rozważone i świadomie NIE wykluczone")

**Intent**: Stop listing autostart as "not covered", without adding an FR.

**Contract**: remove only "autostart" from the list sentence (keep the other items and the closing sentence as they are); add on its own line: "Autostart z Windows jest dostarczany w M-3 jako MS-17 (roadmapa): domyślnie wyłączony, przełącznik `autostart.enabled` w `settings.json`, start do zasobnika bez okna; bez nowego FR."

### Success Criteria:

#### Automated Verification:

- README documents the key: `git grep -n "\"autostart\"" -- README.md` finds the example line and `git grep -n "com.sebas.day-navigator" -- README.md` finds the paragraph
- CHANGELOG has the bullet under `[Unreleased]`: `git grep -n -i "autostart" -- CHANGELOG.md` finds it above the first released `## [` heading
- PRD no longer lists autostart as not covered and names MS-17: `git grep -n "MS-17" -- context/foundation/prd.md` finds the new line, and the "Reszta planu z PRD v2" line no longer contains "autostart"
- Only docs changed in this phase: `git diff --quiet HEAD -- src src-tauri`
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`)

**Implementation Note**: Commit as `docs(windows-autostart): document the autostart key (p3)`.

---

## Testing Strategy

### Unit Tests:

- `settings::autostart`: the loader table (defaults, accepted values, refused values, unknown key, non-object block, file-level errors, BOM, block isolation).
- `autostart`: `started_by_autostart`, `expected_value` (path with a space), the `sync_action` table, `run` against the fake `RunEntry` (every `Outcome`, no entry call when invalid or not applying, no rewrite of an equal value, overwrite of a different value, nothing written after a read failure), `status` mapping and value-free messages.
- `autostart`: `VALUE_NAME` equals the bundle identifier in `tauri.conf.json` (no registry access).
- `health`: the new key and toast lines.
- `events` (`tests/events.rs`): `every_source_id_round_trips` includes `Autostart`.

### Integration Tests:

- None: the only integration point is the real HKCU Run key, which tests must never write. It is covered by the Phase 2 manual rows.

### Manual Testing Steps:

1. Build and install a release build (`npm run tauri build`, then the NSIS installer). Start only the installed exe for these rows: a release exe started from `src-tauri/target/release` reads the same `settings.json` (same identifier) and, with the key on, repoints the Run value at itself until the installed app starts again.
2. Run the Phase 2 manual rows in order: no block → on → sign out/in → manual and second starts → quit and restart → off → Task Manager disabled → invalid block.
3. Build a debug app (`npm run tauri build -- --debug --no-bundle`) with the key on and confirm the calm NIEDOSTĘPNY row and an untouched Run value.

## Performance Considerations

One settings read and at most one registry read plus one write/remove per start, on a short background thread; nothing on the main thread and nothing per health round beyond cloning one stored status.

## Migration Notes

Migration v13 (written as v12 in p1, renumbered at the M-3 wave 2 integration after S-02's v12) rebuilds `events` so its `source_id` CHECK accepts `'autostart'`; same columns and CHECKs otherwise; found in p1. Existing users see a new "Autostart" row reading NIESKONFIGUROWANE until they add the key. A Run value left behind by an uninstall points to a missing exe; Windows skips it, and README tells how to remove it. "Release build" is the proxy for "installed": any release copy started with the key on (for example from `target/release`) writes its own path, and the next start of the installed app writes the installed path back (A3).

## Decisions

User decisions (binding, given with the task):

- **D1.** Autostart is off by default and turned on by a key in `settings.json`; at each start the app syncs the Windows entry with the key (on → ensure, off → remove).
- **D2.** An autostart launch goes straight to the tray with the window hidden; a manual start shows the window as today.
- **D3.** The Run registry entry is a write outside the app's folders: this change adds an explicit, narrow exception to the AGENTS.md hard rule.
- **D4.** Anchor MS-17; no new FR in the PRD; the PRD wording that lists autostart as not covered is updated.

### Defaults chosen by agent (user may revise)

No user was reachable during planning; each default follows `research.md` unless noted.

- **A1.** Mechanism: a direct HKCU Run value via `winreg 0.56` (already locked); no `tauri-plugin-autostart` (unquoted path, HKLM first, overrides Task Manager). This departs from the plugin suggested in issue #17's comment.
- **A2.** Value name `com.sebas.day-navigator` (the bundle identifier), guarded by a test against `tauri.conf.json`; the constant and its test live in `autostart/mod.rs`, so `hkcu.rs` has no test module (revised in plan review, F2).
- **A3.** Value data `"<current_exe>" --autostart`, quoted; compared as an exact string; a different value (for example after a reinstall elsewhere) is overwritten.
- **A4.** Settings shape `"autostart": { "enabled": true }`; absent / `null` / `false` = off; any other value, unknown key or non-object block = invalid.
- **A5.** Detection: the fixed `--autostart` argument. It hides the window whatever the key says; a single-instance relaunch with it does not show the window.
- **A6.** The sync runs once per start, on its own `autostart-sync` thread; an edited key takes effect at the next start (documented in README); no re-sync while running.
- **A7.** Invalid block or unreadable Run value: nothing is written or removed; the source reports an error.
- **A8.** Debug builds never touch the registry: key on → calm `Unavailable` ("tylko w zainstalowanej wersji"), key off → `NotConfigured`; chosen with `cfg!(debug_assertions)` so the release path is still compiled and linted. Non-Windows builds never apply (same calm states) and compile no registry code.
- **A9.** A new health source "Autostart" (key `autostart`) shown as a generic row in "Źródła"; no view change. Absent until the sync finishes. The event log learns the key (`source_from_key`, TS `EventSourceId`) in the same commit (revised in plan review, F1).
- **A10.** Error kinds: `SettingsInvalid` for an invalid block, new `AutostartFailed` ("nie udało się ustawić autostartu") for registry read/write/remove failures, an unknown exe path and a failed thread spawn; messages carry only the `io::ErrorKind` name.
- **A11.** Fixed Polish texts as in Phase 1 §3 (no paths, no values).
- **A12.** Uninstall: README note only (set `enabled: false` and start once, or remove the entry in Task Manager); no NSIS uninstall hook.
- **A13.** Task Manager "disabled" (`StartupApproved\Run`) is neither read nor written; the row still reads OK; README explains it.
- **A14.** Tests never touch the real registry; `HkcuRun` is checked only by manual rows in an installed build.
- **A15.** The AGENTS.md exception lands in Phase 2, in the same commit as the first registry write, and AGENTS.md names `src-tauri/src/autostart/` as the only registry code.
- **A16.** No roadmap edit and no roadmap status sync (the task forbids editing `context/foundation/roadmap.md`).
- **A17.** PRD: remove only the word "autostart" from the "considered, not covered" sentence and add one separate sentence about MS-17, to keep merges with parallel slices small.

## References

- Research: `context/changes/windows-autostart/research.md`
- Start-up and single instance: `src-tauri/src/lib.rs:136-143`, `:156-175`, `:176-187`, `:226-235`
- Health round: `src-tauri/src/runtime.rs:148-156`, `:275-292`, `:363-387`, `:410-419`
- Health model: `src-tauri/src/health/mod.rs:24-48`, `:62-78`, `:151-175`, `:192-215`; `src-tauri/src/health/probe.rs:132-156`
- Settings block template: `src-tauri/src/settings/work_time.rs:31-243`; wiring `src-tauri/src/settings.rs:16-17`, `:50-53`
- Event log source keys: `src-tauri/src/events/mod.rs:167-177`, `:320-355`; `src-tauri/tests/events.rs:180-205`; `src/events/types.ts:7-8`
- Plan review: `context/changes/windows-autostart/reviews/plan-review.md`
- UI rows: `src/health/HealthPanel.tsx:118,177`, `src/health/sourceView.ts:27-57,86-88`, `src/health/types.ts:13-17`
- Docs: `AGENTS.md:11`, `AGENTS.md:22`, `README.md:92-125`, `CHANGELOG.md` `[Unreleased]`, `context/foundation/prd.md:233-234`
- Roadmap: `context/foundation/roadmap.md:30`, `:134-146`; issue #17
- Prior changes: `context/archive/2026-11-26-tray-timer-control/` (start-up left for S-04), `context/archive/2026-11-26-public-holidays-norm/` (settings block and docs pattern)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Autostart settings block and pure rules

#### Automated

- [x] 1.1 Rust tests pass, including the new `settings::autostart` and `autostart` tests: `cargo test` (in `src-tauri/`) — b27f063
- [x] 1.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`) — b27f063
- [x] 1.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`) — b27f063
- [x] 1.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — b27f063
- [x] 1.5 The new rule files import nothing from `tauri` — b27f063
- [x] 1.6 The only frontend change is the `EventSourceId` key; no capability, config or dependency change — b27f063
- [x] 1.7 The event log knows the new key — b27f063

### Phase 2: Registry entry, start-up sync and hidden start

#### Automated

- [x] 2.1 Rust tests pass: `cargo test` (in `src-tauri/`) — f6b4cbd
- [x] 2.2 Formatting passes: `cargo fmt --check` (in `src-tauri/`) — f6b4cbd
- [x] 2.3 Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`) — f6b4cbd
- [x] 2.4 Dependency audit passes: `cargo audit` (in `src-tauri/`) — f6b4cbd
- [x] 2.5 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — f6b4cbd
- [x] 2.6 Registry code lives only in `hkcu.rs` — f6b4cbd
- [x] 2.7 No test uses the real registry — f6b4cbd
- [x] 2.8 `winreg` is a normal dependency of the app with no new package — f6b4cbd
- [x] 2.9 No frontend, capability or config change — f6b4cbd

#### Manual

- [x] 2.10 Installed build without an `autostart` block: row NIESKONFIGUROWANE, no Run value — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.11 Key on and app restarted: row OK, Run value is the quoted exe path with `--autostart` — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.12 Sign out and in: app in the tray, window hidden (also after a maximized session), fresh data when opened — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.13 Manual start and second start show the window; a `--autostart` relaunch does not — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.14 After a tray-only session and "Zakończ", the next start restores window size and position — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.15 Key off and app restarted: Run value removed, row NIESKONFIGUROWANE — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.16 Task Manager "disabled" stays disabled after a restart and the app does not start at sign-in — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.17 Invalid block: red tray, toast, fixed text, Run value unchanged — checked by the user on 2026-11-27 on m3-wave2-integration
- [x] 2.18 Debug build with the key on: calm NIEDOSTĘPNY row, Run value untouched — checked by the user on 2026-11-27 on m3-wave2-integration

### Phase 3: User docs and PRD wording

#### Automated

- [x] 3.1 README documents the key — 8eb30de
- [x] 3.2 CHANGELOG has the bullet under `[Unreleased]` — 8eb30de
- [x] 3.3 PRD no longer lists autostart as not covered and names MS-17 — 8eb30de
- [x] 3.4 Only docs changed in this phase — 8eb30de
- [x] 3.5 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build` — 8eb30de
- [x] 3.6 Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (in `src-tauri/`) — 8eb30de
