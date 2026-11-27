<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Windows autostart into the tray Implementation Plan

- **Plan**: `context/changes/windows-autostart/plan.md`
- **Mode**: Deep
- **Date**: 2026-11-26
- **Verdict**: REVISE (SOUND after the fixes applied below)
- **Findings**: 1 critical, 3 warnings, 3 observations

Non-interactive run: no user was reachable, so every finding was fixed directly in the plan (and in `plan-brief.md` where it has to stay consistent). No user decision (D1–D4) was changed.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | FAIL (F1, F4), PASS after fixes |
| Blind Spots | WARNING (F3, F5, F7) |
| Plan Completeness | WARNING (F2, F6) |

## Grounding

Grounding: 9/9 paths ✓ (`lib.rs`, `runtime.rs`, `settings.rs`, `settings/work_time.rs`, `health/mod.rs`, `health/probe.rs`, `Cargo.toml`, `README.md`, `prd.md:234`), 8/8 symbols ✓ (`show_main`, `probe_round`, `check_round`, `refresh_tray`, `short_reason`, `reminders_status`, `source_keys_are_stable`, `winreg 0.56.0` in `Cargo.lock`), brief↔plan ✓.

Verified against the code and the locked crate sources:

- `tauri-plugin-single-instance` 2.5.5 (Windows) forwards `std::env::args_os()` of the second process, program path first, and that process exits before `setup` runs. `args.iter().skip(1)` is right, and a second process never starts a second sync.
- `tauri-plugin-window-state` 2.5.0 restores size, position and maximized state on the hidden window. tao 0.37.3 skips `ShowWindow(SW_MAXIMIZE)` while the window is hidden, so a saved maximized state does not show it. On exit the plugin saves the restored values.
- `winreg` 0.56.0 is already in `Cargo.lock` (deps: `cfg-if`, `windows-sys 0.61.2`). `String: FromRegValue` strips trailing NULs and accepts `REG_SZ` and `REG_EXPAND_SZ`, so the exact string comparison is sound.
- CI: `cargo audit` runs on `ubuntu-latest` from the lock file only. Rust is built, linted and tested only on `windows-latest`, debug profile, with `--locked`. The `cfg(windows)` dependency is the right shape.
- `probe_round` is touched by no other open slice. S-07 edits only `run_tray_action` in `runtime.rs`; S-02, S-05 and S-08 do not touch `runtime.rs`. The signature change carries no merge risk.
- The test seam holds. `HkcuRun` is constructed only in `startup.rs`. `run(.., apply = false, ..)` makes no entry call. `cargo test` is a debug build. No integration test can reach `startup::start`, because it needs an `AppHandle`.
- No code depends on the working directory (the CLI runner sets `current_dir(temp_dir())`), so a Run-key start with cwd `System32` is harmless.

## Findings

### F1 — New `SourceId` breaks today's event log; TS mirror missing

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 1 §2, "What We're NOT Doing", success criterion 1.6
- **Detail**: Every health transition is also written to the event log by `SourceId::key()`. The log is read back through a closed list, `events::source_from_key` (`src-tauri/src/events/mod.rs:167-177`), and an unknown key fails the whole read as `Damaged` ("event record is unreadable", `:350-355`, `:416-426`). So the first "Autostart" error or recovery would make today's event log unreadable for the rest of the day. The TS mirror `EventSourceId` is a closed union (`src/events/types.ts:8`), and `tests/events.rs:180-205` has an exhaustive match. The plan said "no TS type change", and criterion 1.6 forbade any change under `src`.
- **Fix**: Phase 1 §2 now also adds `SourceId::Autostart` to `source_from_key`, extends `every_source_id_round_trips` (match arm and array), and adds `"autostart"` to `EventSourceId` in the same commit. Criterion 1.6 now allows only `src/events/types.ts`, and a new criterion 1.7 greps the three places. "NOT Doing", Current State, A9, Testing Strategy, References and the brief were updated to match.
- **Decision**: FIXED

### F2 — "No test uses the real registry" was not checkable by a command

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §2, criterion 2.7
- **Detail**: Criterion 2.7 needed a person to check that no `HkcuRun` occurrence is "inside a `#[cfg(test)]` module". `hkcu.rs` was planned to have a test module of its own (the identifier test), so a grep could not prove the seam.
- **Fix**: `RUN_KEY` and `VALUE_NAME` move to `autostart/mod.rs` (Phase 1, plain strings), together with the identifier test. `hkcu.rs` and `startup.rs` have no `#[cfg(test)]` module. Criterion 2.7 is now two greps: `HkcuRun` appears only in `hkcu.rs` and `startup.rs`, and `cfg(test)` appears in neither. The plan also states the `pub mod startup;` declaration explicitly. A2 is marked as revised.
- **Decision**: FIXED

### F3 — Release builds abort on panic; the sync path had no no-panic rule

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details; Phase 2 §3
- **Detail**: `[profile.release] panic = "abort"` (`src-tauri/Cargo.toml:81`). A panic in `autostart-sync`, or in `startup::status` (called from the health thread every round), would end the app at sign-in with no window and no message.
- **Fix**: A new Critical Implementation Detail requires no `unwrap`/`expect`/indexing in `startup.rs` and `hkcu.rs`, and locking the mutex with `unwrap_or_else(|p| p.into_inner())`, as `runtime.rs` does. The rule is repeated in the Phase 2 §3 contract.
- **Decision**: FIXED

### F4 — README edits were placed on lines that parallel slices edit

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 3 §1; Critical Implementation Details ("Shared-file merges")
- **Detail**: Adding the example key as the "new last line" of the JSON means adding a comma to the `workTime` line, which S-02 rewrites. The table row and the paragraph would sit next to the `workTime` row (S-02) and the `workTime` paragraph at `README.md:125` (S-08). Adjacent edits conflict in git.
- **Fix**: The example line `"autostart": { "enabled": true },` and the table row now go between the `todo` and `reminders` lines. The paragraph goes after the `workTime` paragraph, with the blank line kept between them. The plan also notes that `probe_round` has no competing edits.
- **Decision**: FIXED

### F5 — "Release build" stands in for "installed"

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: A8; Manual Testing Steps; Migration Notes; README paragraph
- **Detail**: Every build shares one `settings.json`, because they share the identifier. A release exe started from `src-tauri/target/release` with the key on therefore writes its own path into the Run value, and keeps it until the installed app starts again (A3 overwrites a different value). The plan's own manual step 1 builds such an exe.
- **Fix**: Manual Testing step 1 says to start only the installed exe. Migration Notes describe the proxy and the self-heal. The README paragraph says the entry points at the copy that last started with the key on.
- **Decision**: FIXED

### F6 — `Cargo.lock` must be committed: CI runs with `--locked`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §1, criterion 2.8
- **Detail**: Adding `winreg` as a normal dependency changes `day-navigator`'s dependency list in `Cargo.lock`. CI runs clippy, build and test with `--locked`, so a lock file left out of the commit fails CI. The local gates do not use `--locked`.
- **Fix**: The Phase 2 §1 contract says the lock file is committed with `Cargo.toml`, and explains why `cfg(windows)` and `cargo audit` on Ubuntu are fine. Criterion 2.8 adds `cargo build --locked` and "only the `winreg` line in `day-navigator`'s dependencies".
- **Decision**: FIXED

### F7 — Hidden start after a maximized session was unchecked

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Key Discoveries; manual row 2.12
- **Detail**: Window-state calls `maximize()` on the hidden window when the last session ended maximized. tao 0.37.3 skips the `ShowWindow` while the window is hidden, so D2 holds. The plan had neither evidence nor a manual check for this case.
- **Fix**: Key Discoveries now cite the tao behaviour and the single-instance argument handling. Manual row 2.12 also covers a session that ended maximized (the Progress title was updated before implementation).
- **Decision**: FIXED

## Agent defaults revised

- **A2**: the `VALUE_NAME` constant and its identifier test move from `hkcu.rs` to `autostart/mod.rs` (F2).
- **A9**: the event log learns the `autostart` key, and TS `EventSourceId` gains it (F1).
- All other defaults (A1, A3–A8, A10–A17) were checked and kept.

## Open questions for the user

None.
