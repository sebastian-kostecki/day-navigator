---
date: 2026-11-26T14:18:15+01:00
researcher: Claude (Opus 5.5)
git_commit: d34904a4dc9f9400c8b386cc935a845b7ed6762b
branch: slice/windows-autostart
repository: day-navigator
topic: "Windows autostart into the tray, switched in settings.json (S-04, MS-17, issue #17)"
tags: [research, autostart, tray, settings, health, registry, tauri-plugin-autostart]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (Opus 5.5)
---

# Research: Windows autostart into the tray (S-04)

**Date**: 2026-11-26T14:18:15+01:00
**Researcher**: Claude (Opus 5.5)
**Git Commit**: d34904a (develop tip, merge of M-3 wave 1)
**Branch**: slice/windows-autostart
**Repository**: day-navigator

## Research Question

Roadmap slice S-04 (`context/foundation/roadmap.md:134-146`, issue #17): the user can have the app start with Windows into the tray, without showing the window, controlled by a switch in `settings.json`. What does the code base offer for this, which autostart mechanism fits, and how are the sync, the hidden start, the display and the failures best built? The research gives evidence for the open defaults: the mechanism, the settings key, how the app knows that Windows autostart started it, what the UI shows, and how errors surface.

### Decisions already made by the user (input, not re-opened)

- Autostart is **off by default**. A key in `settings.json` turns it on. On each start the app syncs the Windows entry with the key: on → make sure the entry exists, off → remove it.
- A start by Windows autostart goes **straight to the tray with the window hidden**. A manual start shows the window as today.
- The registry entry is a write outside the app's own folders. The same change adds an explicit, narrow exception to the AGENTS.md hard rule.
- Anchor: MS-17. **No new FR in the PRD.** The PRD wording that lists autostart as considered but not covered is updated.

## Summary

- **Mechanism: write the one HKCU Run value directly, with `winreg 0.56`.** Do not use `tauri-plugin-autostart`. Plugin 2.7.0 fits tauri 2.12, but on Windows its `auto-launch 0.6` backend has three problems:
  - it writes the exe path **unquoted** (`format!("{} {}", path, args.join(" "))`);
  - it tries **HKLM before HKCU**;
  - it **overwrites the Task Manager "disabled" flag** (`StartupApproved\Run`) on every `enable()`.

  The installed exe sits in `%LOCALAPPDATA%\Nawigator dnia\day-navigator.exe`, a path with a space (observed on this machine). `winreg 0.56` is already in `Cargo.lock` as a build-only dependency, has a safe API and no RustSec advisory. Using it adds no new crate.
- **Hidden start: one condition at `lib.rs:173`.** Today `setup` calls `runtime::show_main` unconditionally (`src-tauri/src/lib.rs:172-173`). The window is created hidden (`tauri.conf.json` `"visible": false`). The window-state plugin does not restore visibility (`StateFlags::all() & !StateFlags::VISIBLE`, `lib.rs:139-143`). Skipping `show_main` when the process was started with a fixed `--autostart` argument is therefore enough to start in the tray. All background work (health, reminders, refreshes, tray) runs on Rust threads started in `setup` (`lib.rs:166-171`), so it runs with the window hidden.
- **Settings: a new block `"autostart": { "enabled": true }`, read by a new loader `settings/autostart.rs`.** It copies the `workTime` loader pattern (`src-tauri/src/settings/work_time.rs:31-88`). `check_settings` ignores unknown top-level keys (`settings.rs:50-53`), so the new block breaks no existing loader.
- **Display and errors: a new health source "Autostart" (`SourceId::Autostart`, key `autostart`).** It shows in "Źródła" under "Pozostałe źródła" with no UI change:
  - TS `SourceStatus.id` is a plain `string` (`src/health/types.ts:13-17`), and the panel renders every non-card source as a generic row (`src/health/HealthPanel.tsx:118,177-187`).
  - A failed sync becomes `Error`: red tray, a toast on the transition, a log line and an event-log entry. This reuses the existing health machinery (`runtime.rs:439-483`), so a failure is never silent.
- **Tests: a trait seam over the Run value plus pure functions.** The pure functions are `sync_action(desired, current, expected) -> {Nothing, Write, Remove}` and `started_by_autostart(args)`. Tests run against an in-memory fake. No test touches the real HKCU.

## Detailed Findings

### 1. Settings: loading, validation and display

- There is no top-level settings struct and no "get settings" command. Each block has its own loader. A loader re-reads the file, parses it as `serde_json::Value`, checks `version == 1`, then deserializes only its block into a private `Raw*` struct with `deny_unknown_fields` (`settings.rs:91-559`; worker survey, spot-checked at `settings/work_time.rs:64-88`).
- `check_settings` (`settings.rs:50-89`) validates only the whole file: present, readable, UTF-8 (BOM allowed), JSON, `version` 1. Its doc comment says "any other top-level key … is ignored whatever its shape" (`settings.rs:50-53`).
- **The template is the most recent block, `workTime`** (archived change `2026-11-26-public-holidays-norm`), in `settings/work_time.rs`:
  - a unit error with no detail (`:26-29`);
  - `RawWorkTime` with `#[serde(deny_unknown_fields, rename_all = "camelCase")]` (`:31-36`);
  - a missing file gives the default, any other read error is `Invalid` (`:49-62`);
  - an absent or `null` block gives the default, a non-object block is `Invalid` (`:64-79`);
  - the value match accepts `null`/absent, `false` and `"pl"`, and anything else is `Invalid` (`:81-86`).

  No key in the file today is a pure `true`/`false` boolean. The closest are `workTime.holidays` (`"pl"` | `false`) and the `reminders.*` values (value | `false`, `settings.rs:370-395`).
- The module wiring sits in `settings.rs:16-17` (`mod work_time; pub use work_time::{…}`). An `autostart` loader would add one `mod` line and one `pub use` line next to it.
- **When the file is read:** settings are never cached. `setup` resolves the path once (`lib.rs:157`, `settings_file.rs:13-19`) and hands it to each thread, which re-reads the file. Nothing parses `settings.json` on the main thread in `setup` (`lib.rs:156-175`). The autostart sync is the first consumer that must act once at start, so it needs its own read (see §6).
- **Documentation of keys:** README "## Configuration" (`README.md:92-125`) has a JSON example (`:98-113`) and a block table (`:115-123`, the `workTime` row is `:123`). There is no example settings file in the repo. The README says the app "reads it and never changes it" (`README.md:94-95`); this change keeps that true.
- **Read-only display today:** there is no general settings view. The one read-only display of settings values is the "Przypomnienia" panel in "Czas pracy" (`src/worktime/RemindersPanel.tsx:10-36,63-80`). It is fed by the `reminder_settings` command (`work_reminders.rs:165-191`) and shows a "domyślnie" label when a value comes from the default.
- **How settings errors surface:**
  - The "Ustawienia" health source reports the whole file (`health/probe.rs:16-35`). The "Przypomnienia" source reports the `reminders` block with fixed Polish text that never echoes user values (`health/probe.rs:136-156`).
  - The app always starts, whatever the state of `settings.json`. The window is shown regardless (`lib.rs:173`).

### 2. Startup, the window and the tray

- `run()` registers plugins in this order (`lib.rs:135-155`): single-instance, window-state, opener, notification, log.
  - **single-instance** (`lib.rs:136-138`): the callback ignores `_args` and calls `runtime::show_main(app)`. A second launch shows and focuses the running window.
  - **window-state** (`lib.rs:139-143`) restores every flag except `VISIBLE`, so it never shows the window by itself.
- `setup` (`lib.rs:156-175`), in order:
  1. store handle;
  2. GitLab and GitHub refresh state;
  3. `runtime::build_tray` (`runtime.rs:220-273`; the menu has START/STOP, "Otwórz", "Sprawdź teraz", "Zakończ");
  4. `runtime::start`, the health and watchdog threads (`runtime.rs:277-292`);
  5. the reminders thread;
  6. the refresh threads;
  7. finally `runtime::show_main(app.handle())` with the comment "The window starts hidden; show it once the saved size is restored" (`lib.rs:172-173`).
- `show_main` runs show, unminimize and set_focus on the window `main` (`runtime.rs:150-156`). These callers use it:
  - setup (`lib.rs:173`);
  - single-instance (`lib.rs:137`);
  - tray "Otwórz" (`runtime.rs:252`);
  - tray left click (`runtime.rs:261-269`).
- The window is created hidden: `tauri.conf.json` `app.windows[0].visible: false`.
- The close button hides the window and emits `main-hidden` (`lib.rs:176-187`). `ExitRequested` with `code: None` is prevented, so only "Zakończ" (`app.exit(0)`) quits (`lib.rs:226-235`). A start in the tray therefore stays alive with no window ever shown.
- **Hidden start and the webview:** the GitLab "seen" mark is sent only when the user leaves the GitLab view or hides the window while it is active, never on mount (`src/gitlab/useGitlabSeenMark.ts:10-37`). The initial view is "todo" (`src/App.tsx:35`). A hidden start therefore marks nothing as seen. Data hooks refetch on `visibilitychange`/`focus` (for example `src/worktime/useWorkTimer.ts:84-120`), so the first show after a hidden start loads fresh data.
- **CLI arguments:** a search of `src-tauri/src/` for `env::args` and `args_os` finds hits only in the test-only `bin/fake_cli.rs`, so the app itself never reads its command line. Its one `std::env` read in `runtime.rs` is the debug-only `DAY_NAVIGATOR_STALL_CHECKS` variable (`runtime.rs:394-400`). The single-instance callback receives the second process's args (`lib.rs:136`, `_args`) and ignores them.
- **Archived tray slice:** S-03 deliberately left start-up order unchanged for S-04 (`context/archive/2026-11-26-tray-timer-control/plan.md:47`). It kept shared-file edits small and new rules in new files (`plan.md:406`, A12).

### 3. `tauri-plugin-autostart` v2 (primary sources)

The source of tauri-plugin-autostart 2.7.0 and auto-launch 0.6.0 was downloaded and read (worker report). Sources: https://docs.rs/tauri-plugin-autostart/2.7.0, https://github.com/tauri-apps/plugins-workspace/tree/d4835d0e947179bac24a383212792d74be3ebe4f/plugins/autostart, https://github.com/zzzgydi/auto-launch/tree/28816b4717227dfd9359830c0bf086cd1d2f638e, https://v2.tauri.app/plugin/autostart/.

- **Versions:**
  - 2.7.0 is the latest 2.x (2026-10-01): MSRV 1.90, needs `tauri ^2.12`, uses `auto-launch ^0.6`.
  - 2.6.0 also needs tauri 2.12 but uses `auto-launch ^0.5`, which pulls in `winreg ^0.10` and old `winapi`.
  - 3.0.0-alpha is for tauri 3.
  - So **2.7.0** is the only fitting choice if the plugin is used. On Windows it adds `tauri-plugin-autostart`, `auto-launch`, `windows-registry 0.6.1` and `os_info 3.x` to the lock.
- **Rust API (2.7.0):**
  - `init(MacosLauncher, Option<Vec<&'static str>>)`, or `Builder::new().args(..).app_name(..).build()`;
  - `ManagerExt::autolaunch()` → `AutoLaunchManager::{enable, disable, is_enabled}`;
  - `Error::{Io, Anyhow(String)}`.
  - The Builder cannot choose the HKCU-only mode of auto-launch.
- **What it writes on Windows** (`auto-launch-0.6.0/src/windows.rs`):
  - Value under `SOFTWARE\Microsoft\Windows\CurrentVersion\Run`. The default `Dynamic` mode tries **HKLM first** and falls back to HKCU only on access denied (`:48-57`, `lib.rs:347-361`).
  - Value name = `app_name`, which defaults to `package_info().name`, i.e. productName "Nawigator dnia".
  - Value data = `format!("{} {}", app_path, args.join(" "))`: **no quoting** (`:75-78`).
  - `enable()` also rewrites `Explorer\StartupApproved\Run` to "enabled" (`:80-96`), which overrides a user's Task Manager "disable".
  - `disable()` treats a missing value as Ok (`:106-144`).
  - The exe path is `current_exe().display()`, with no `canonicalize` on Windows.
- **Permissions:**
  - `autostart:default` = allow-enable/disable/is-enabled.
  - Without a capability grant the webview's calls are refused by the ACL (`tauri-2.12.7/src/webview/mod.rs:2076-2108`).
  - The Rust API is plain managed `State`, so a Rust-only use needs no npm package and no permission, which matches the AGENTS.md rule for `window-state`.
- **Docs page:** setup is `cargo add` plus `.plugin(init(MacosLauncher::LaunchAgent, Some(vec![...])))` in setup, then `ManagerExt` for use from Rust. The page says nothing about Windows registry details or quoting.
- **Risk of the unquoted path:**
  - Fact: the installed path contains a space (`%LOCALAPPDATA%\Nawigator dnia\day-navigator.exe`, observed in the local install).
  - Inference, not verified: Windows resolves an unquoted command line by trying `…\Nawigator.exe` first. That is a classic unquoted-path hazard, and Rust's argv would split the path into extra arguments.
  - A similar report exists: an unquoted Run value lost its startup argument (https://github.com/localsend/localsend/issues/1293).

### 4. Writing the HKCU Run value directly (comparison)

- **`winreg 0.56.0`** (worker read the source in the local cargo registry):
  - Already in `Cargo.lock` (`cargo tree -i winreg` shows it under `embed-resource` → `tauri-winres` → `tauri-build`, build-dependency only).
  - It depends on `windows-sys >=0.59,<=0.61`, matching the direct `windows-sys 0.61` dependency (`src-tauri/Cargo.toml`, `[target.'cfg(windows)'.dependencies]`).
  - API:
    - `RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN)` or `open_subkey_with_flags`;
    - `set_value(name, &String)` writes REG_SZ;
    - `get_value::<String, _>(name)`;
    - `delete_value(name)` returns `io::ErrorKind::NotFound` for a missing value.
  - Safe Rust, no `unsafe`.
- **`windows-registry`:** 0.6.1 (MSRV 1.82) fits. 0.100.0 needs MSRV 1.95 and does not fit rustc 1.90.
- **`windows-sys` feature `Win32_System_Registry`:** no new crate, but `unsafe` code with UTF-16 buffers.
- **RustSec:** no advisories for tauri-plugin-autostart, auto-launch, winreg, windows-registry, os_info or windows-sys (advisory-db `crates/<name>` returns 404 for each, worker check).
- **A direct write controls everything the plugin gets wrong:**
  - quoting: `"\"<exe>\" --autostart"`;
  - HKCU only;
  - the value name;
  - it leaves `StartupApproved` alone, so a Task Manager "disable" by the user is respected.

### 5. How the app knows Windows autostart started it

- The Run value carries one fixed argument, `--autostart`. A pure `started_by_autostart(args: impl Iterator<Item = OsString>) -> bool` checks `args_os().skip(1)` for an exact match. It never parses other arguments, matching the spirit of "fixed argument lists".
- `setup` calls `show_main` only when that function returns false (`lib.rs:173`).
- **Single-instance:** a second launch with `--autostart` while the app already runs should not pop the window up. The callback can skip `show_main` when its `args` contain `--autostart` (`lib.rs:136-138`). At login the app is normally not running yet, so this guards an edge case only.
- Starting with `--autostart` hides the window **whatever the settings key says**. The flag records how the process was launched; the sync then removes the entry if the key is off.

### 6. The sync at start

- The desired state is `autostart.enabled` from settings. Absent, `null` or `false` means off; `true` means on; anything else is invalid.
- The expected value is `"\"" + current_exe + "\" --autostart"`.
- `sync_action(desired, current: Option<String>, expected) -> Nothing | Write | Remove`:
  - on and value equal → Nothing;
  - on and value missing or different (for example after a reinstall elsewhere) → Write;
  - off and value present → Remove;
  - off and value missing → Nothing;
  - invalid → Nothing, so the entry is left as it is and an error is reported.
- **Where it runs:** on its own short thread started from `setup`, or as part of the first health round. Either way the result is stored in managed state that `probe_round` reads (`runtime.rs:365-387`). The main thread stays free, as for the store (`lib.rs:159-162`). Registry calls are fast, but the settings read is file I/O.
- **Debug builds:** `current_exe()` in `npm run tauri dev` is `src-tauri/target/debug/day-navigator.exe`, inside a worktree. A sync there would point Windows at a dev binary that later disappears. Recommendation: under `cfg(debug_assertions)` the sync never writes or removes the registry value and reports the source as calm `Unavailable` ("tylko w zainstalowanej wersji"). The pure logic stays fully tested.
- **Uninstall:** the NSIS uninstaller does not know about the Run value. A leftover value would point to a deleted exe.
  - Simple fix: a README line ("set `enabled: false` and start the app once before uninstalling").
  - Robust fix: an NSIS uninstall hook. Tauri 2 offers `bundle.windows.nsis.installerHooks`; not verified in this repo.
  - The plan decides.

### 7. Test seam (no real HKCU in tests)

- Trait `RunEntry { fn read(&self) -> io::Result<Option<String>>; fn write(&self, value: &str) -> io::Result<()>; fn remove(&self) -> io::Result<()>; }`.
  - The production implementation `HkcuRun` lives in a `#[cfg(windows)]` module with a fixed key path and a fixed value name.
  - Tests use an in-memory fake that can also inject `io::Error`s.
- Unit tests cover:
  - `sync_action` as a full table;
  - `started_by_autostart`;
  - the settings parser: absent, `null`, `true`, `false`, wrong type, unknown key, file-level errors, BOM, and isolation from other blocks, mirroring `settings/work_time.rs:119-243`;
  - the mapping from sync result to `SourceStatus`.
- A test against a scratch HKCU subkey would still be a registry write outside the app's folders. Avoid it; the real `HkcuRun` is checked by a manual step in the installed build.

### 8. The hard rule, README and PRD wording

- **AGENTS.md:11** is the "Outside folders configured in settings, the app writes only its own state under its bundle identifier: …" rule. The exception belongs there, worded narrowly, for example: "…and, only while `autostart.enabled` is `true`, one value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` named after the bundle identifier, holding the quoted exe path and `--autostart`; it is removed when the key is off." Searches of README.md, CHANGELOG.md and `.github/workflows/ci.yml` for "autostart", "registry", "rejestr" and "outside folders" found no other copy of this rule. README states only that settings are read and never changed (`README.md:94-95`).
- **PRD:** `context/foundation/prd.md:233-234` ("Rozważone i świadomie NIE wykluczone") lists "autostart" among the considered items that no FR covers. The change removes it from that list and adds one sentence: autostart with Windows is delivered in M-3 as MS-17 (roadmap), off by default, a switch in `settings.json`, no new FR. The PRD has no rule about writes outside the app folders; its guardrails concern vaults, GitLab/GitHub and work data (`prd.md:184-200,222-226`).
- **Roadmap:** S-04 lists both unknowns (default; AGENTS.md exception, `roadmap.md:142-144`), and Open Roadmap Question 3 names the S-04 registry exception (`roadmap.md:218`). The user settled both; this slice does not edit the roadmap.

### 9. Error surfacing

- Existing pattern: each health source is a `SourceStatus { id, label, state }` (`health/mod.rs:24-48,66-78,93-100`). The state is one of `Ok`, `NotConfigured`, calm `Unavailable { message }` (never red, never a toast, `mod.rs:69-73`) or `Error { kind, message }`.
  - An `Error` turns the overall state red, which drives the tray icon and tooltip (`runtime.rs:294-329`).
  - A transition gives a toast (`runtime.rs:459-461`), an event-log entry (`:462-474`) and a log line (`:475-483`).
- The UI legend already explains `NotConfigured` as "Brak wpisu w settings.json — źródło jest pominięte." and `Error` as red tray plus notice (`src/health/HealthPanel.tsx:40-47`). `rowMessage` shows text only for error and unavailable (`src/health/sourceView.ts:86-88`).
- **Proposed states for the "Autostart" source:**
  - `Ok`: on, and the entry is correct after the sync.
  - `NotConfigured`: off (absent or `false`), entry absent.
  - `Unavailable`: debug build.
  - `Error` with kind `SettingsInvalid`: invalid block. Fixed Polish text naming the accepted shape, like `REMINDERS_INVALID_PL` (`health/probe.rs:136-141`); the entry is left untouched.
  - `Error` with kind `Registry`: write or remove failed, or `current_exe` failed. Fixed text plus the `io::ErrorKind`; no path, no value data.
- The sync runs once per start, so an `Error` stays until the next start. The message should say "zmiana zadziała po ponownym uruchomieniu" ("takes effect after a restart"), like the timing note the user expects for an edited key.
- Adding `SourceId::Autostart` changes `health/mod.rs:25-47` (variant and key). The UI needs no change, because `SourceStatus.id` is `string` and non-card sources render generically (`src/health/types.ts:13-17`, `src/health/sourceView.ts:50-52`).

## Code References

- `src-tauri/src/lib.rs:136-138` - single-instance callback, ignores args, shows the window
- `src-tauri/src/lib.rs:139-143` - window-state without `VISIBLE`
- `src-tauri/src/lib.rs:156-175` - `setup`; `:173` the unconditional `show_main`
- `src-tauri/src/lib.rs:176-187,226-235` - close hides; only "Zakończ" quits
- `src-tauri/src/runtime.rs:150-156` - `show_main`
- `src-tauri/src/runtime.rs:220-273` - tray build and menu
- `src-tauri/src/runtime.rs:365-387` - `probe_round`, where a new source is pushed
- `src-tauri/src/runtime.rs:439-483` - transitions → toast, event log, log line
- `src-tauri/tauri.conf.json` - `"visible": false`, productName "Nawigator dnia", identifier `com.sebas.day-navigator`
- `src-tauri/src/settings.rs:16-17` - block module wiring
- `src-tauri/src/settings.rs:50-89` - `check_settings` ignores unknown top-level keys
- `src-tauri/src/settings/work_time.rs:31-88` - loader template (Raw struct, defaults, value match)
- `src-tauri/src/health/mod.rs:24-48` - `SourceId` and its keys
- `src-tauri/src/health/mod.rs:66-78` - `SourceState`
- `src-tauri/src/health/probe.rs:136-156` - fixed-text block source (`reminders_status`)
- `src-tauri/Cargo.toml` - tauri 2 crates; `windows-sys 0.61` under `cfg(windows)`
- `src-tauri/Cargo.lock` - tauri 2.12.7, `winreg 0.56.0` (build-only today)
- `src/health/types.ts:13-17`, `src/health/HealthPanel.tsx:118,177-187`, `src/health/sourceView.ts:27-38,86-88` - generic source rows
- `src/gitlab/useGitlabSeenMark.ts:10-37` - no "seen" mark on mount
- `AGENTS.md:11` - the write rule that gets the exception
- `README.md:92-125` - settings documentation
- `context/foundation/prd.md:233-234` - autostart in "considered, not covered"

## Architecture Insights

- Settings blocks are independent. Each is parsed on its own, is strict inside the block (`deny_unknown_fields`) and never errors on the others. A new block is a new file under `settings/` plus two lines in `settings.rs`.
- Every failure a user must act on is a health source. That one path gives the tray colour, the toast, the event log and the "Źródła" row, so a new feature does not need its own error UI.
- Start-up does no settings I/O on the main thread. Work is pushed to named threads, and results reach the UI through managed state read by the health round.
- Rust owns the rules; the UI only displays them. The sync decision is a pure function, and the OS write is behind a trait.

## Historical Context (from prior changes)

- `context/archive/2026-11-26-tray-timer-control/plan.md:47` - "No change to autostart or start-up order (S-04)". Supported: S-03 left `lib.rs` setup order as it was.
- `context/archive/2026-11-26-tray-timer-control/research.md:318` - predicted the S-04 overlap with `lib.rs` setup, `runtime.rs`, `Cargo.toml` and `settings.rs`. Partially supported: `runtime.rs` needs only one `probe_round` line, and `health/mod.rs` also changes.
- `context/archive/2026-11-26-public-holidays-norm/` - origin of the `workTime` loader pattern and of the README/PRD wording updates done in the same change.
- `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md:142-151` - the old M-1 slice "autostart-plan-nudge", never built. Its risk note "silent startup failures are covered by F-02" (health) still matches the health-source recommendation.

## Related Research

- `context/archive/2026-11-26-tray-timer-control/research.md` - tray and start-up mapping.
- `context/archive/2026-11-26-public-holidays-norm/research.md` - settings block conventions.

## Parallel slices and shared files

These slices are open in parallel: S-02 configurable-daily-norm, S-08 week-summary, S-05 month-report-copy, S-07 end-of-day-summary.

- S-02 extends `settings/work_time.rs` and the `workTime` README row (`settings/work_time.rs:12`, "S-02 adds its daily-norm field here"). S-04 does not touch either; it adds `settings/autostart.rs`. Both slices add a line in `settings.rs:16-17` and a README table row and example key, so expect small, adjacent-line merges.
- S-07 hooks `stop_timer` in `work_time/commands.rs`; S-04 does not touch it.
- S-05 and S-08 live in `work_time/` and `src/worktime/`; no overlap.
- **Keep shared-file edits to a few lines:**

  | File | S-04 edit |
  |---|---|
  | `lib.rs` | `pub mod`, the `show_main` condition, the single-instance guard, starting the sync |
  | `runtime.rs` | one push in `probe_round` |
  | `health/mod.rs` | one variant and one key |
  | `Cargo.toml` | one line under `cfg(windows)` |
  | `Cargo.lock` | one edge |
  | README | example key, table row, uninstall note |
  | CHANGELOG | one bullet under `[Unreleased]` |
  | AGENTS.md | the exception sentence |

- **Put the new logic in new files:** `src-tauri/src/autostart.rs` (or `autostart/` with `sync.rs` and `hkcu.rs`) and `settings/autostart.rs`.

## Recommended defaults (for `/10x-plan`)

| Choice | Recommendation | Evidence |
|---|---|---|
| Mechanism | Direct HKCU Run value via `winreg 0.56` (target `cfg(windows)`); no plugin | §3, §4 |
| Value name | `com.sebas.day-navigator` (the bundle identifier, which keeps the AGENTS.md exception "under its bundle identifier") | `tauri.conf.json`, `AGENTS.md:11` |
| Value data | `"<current_exe>" --autostart`, quoted | §3 risk, §4 |
| Settings key | Block `"autostart": { "enabled": true }`; absent/`null`/`false` = off; anything else invalid | §1 |
| Detection | Fixed `--autostart` argument; skip `show_main` in `setup`; single-instance ignores `--autostart` relaunches | §2, §5 |
| Sync | At start, off the main thread; write when missing or different, remove when off; leave untouched when the block is invalid; no-op in debug builds | §6 |
| UI display | "Autostart" row in "Źródła" (new health source), no new view; README documents the key; the app never writes `settings.json` | §9 |
| Errors | `Error` source (red tray, toast, event log, log line) with fixed Polish text, `io::ErrorKind` only | §9 |
| Tests | Pure `sync_action` / `started_by_autostart` / parser + fake `RunEntry`; no real-registry test | §7 |

## Open Questions

None blocks the plan. Choices left to `/10x-plan` (agent defaults above):

1. Uninstall leftover: README note only, or an NSIS uninstall hook (`installerHooks`, not verified here)?
2. Should the "Autostart" source also read `StartupApproved\Run` to show "wyłączony w Menedżerze zadań"? The byte format is undocumented by Microsoft; the recommendation is no for this slice.
3. Not verified: how Explorer launches the Run value at login. The recommendation (a quoted path) is the documented-safe form (https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys). Check it manually in the installed build: log off and on, the app is in the tray, and the window stays hidden.
