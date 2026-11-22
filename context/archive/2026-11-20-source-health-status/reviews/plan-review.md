<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Source Health Status Implementation Plan

- **Plan**: context/changes/source-health-status/plan.md
- **Mode**: Deep
- **Date**: 2026-11-20
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 7 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding
6/6 paths ✓, 3/3 symbols ✓ (2 line refs had drifted: `unmatched_exclusions` mod.rs:302, `panic = "abort"` Cargo.toml:53; corrected in the plan), brief↔plan ✓, Progress↔Phase ✓. Tauri 2.12.7 tray/exit APIs, `include_image!`, and the log/notification/single-instance plugin claims were verified against crate sources.

## Findings

### F1 — Transition rules for disappearing and unconfigured sources

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 1 §1 `transitions`, §2
- **Detail**: Sources absent from `next` were undefined. A vault that stayed broken through a settings outage would re-toast as Unknown → Error, and Error → NotConfigured would send a false "działa ponownie" toast.
- **Fix**: Absence gives no transition, and the last status stays in a diff `baseline`. Error → NotConfigured is silent. Both cases are tested.
- **Decision**: FIXED

### F2 — Exclusions state when the vault itself fails is unspecified

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §2
- **Detail**: Without a `Vault`, `unmatched_exclusions()` cannot run, and the plan didn't say what that row shows.
- **Fix**: The exclusions source is omitted while its vault is in Error. Tested.
- **Decision**: FIXED

### F3 — Zero configured spaces shows a green icon

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: End-State Alignment
- **Location**: Desired End State; Phase 1 §2
- **Detail**: `{"version":1,"spaces":{}}` gave overall Ok, but PRD NFR (prd.md:162) counts a missing configured path as a failure.
- **Fix**: The settings source becomes Error with kind `NoSpaceConfigured`. Tested.
- **Decision**: FIXED

### F4 — The toast's "short reason" is undefined and can leak exclusion entries

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §1 `notice_for`, §4
- **Detail**: `SettingsInvalid.reason` embeds the bad excluded entry (settings.rs:108). If the toast reused the message, that entry would leak.
- **Fix**: `Transition` carries id, label and kinds only. The short reason is a fixed Polish phrase per kind. The test asserts that no entry or reason text appears in the notice.
- **Decision**: FIXED

### F5 — Possible deadlock: tray setters block on the main thread

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 2 §2; Phase 3 §1
- **Detail**: `set_icon`/`set_tooltip` block on the main thread, and sync commands run there. Holding the snapshot lock across a tray call can deadlock.
- **Fix**: Locking rule added: drop guards before any tray, notification or emit call. Commands are `async`. Tray errors are logged, never unwrapped. The `Sender` is held without a `Mutex`, and queued wake-ups are drained.
- **Decision**: FIXED

### F6 — tauri-plugin-log defaults would log every line twice

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §3
- **Detail**: `Builder::new()` already has Stdout and LogDir, and `.target()` adds to them.
- **Fix**: `.clear_targets()`, then LogDir, plus Stdout in debug only.
- **Decision**: FIXED

### F7 — The log folder is written outside the configured folders

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architectural Fitness
- **Location**: Desired End State "Log"; Phase 2 §3
- **Detail**: The AGENTS.md hard rule allows only the distraction blocker outside configured folders. tauri-plugin-log creates `%LOCALAPPDATA%\…\logs`.
- **Fix**: New Phase 2 item 4 amends AGENTS.md to name app-owned folders under the bundle identifier as an explicit exception. The brief is updated.
- **Decision**: FIXED

### F8 — "A second `npm run tauri dev` launch" cannot work

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 Manual; Manual Testing step 4
- **Detail**: Vite's strictPort 1420 makes the second dev launch fail.
- **Fix**: Run `src-tauri/target/debug/day-navigator.exe` instead.
- **Decision**: FIXED

### F9 — Tray icon generation method is unspecified

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §1
- **Detail**: No tool was named, and `include_image!` fails the build on a non-RGBA PNG.
- **Fix**: A one-off Pillow script kept in the change folder. RGBA requirement stated.
- **Decision**: FIXED

### F10 — `useHealth` invoke/event race and StrictMode

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §2
- **Detail**: A slow initial invoke can overwrite a newer event, and async `listen` can leak a listener under StrictMode's double mount.
- **Fix**: Subscribe first and keep the snapshot with the newer `checkedAtMs`. Unlisten with a cancelled-flag guard.
- **Decision**: FIXED

## Notes

- The worktree `.claude/worktrees/work-tasks-from-template` also removes the greet demo from `src/App.tsx`, so expect a merge conflict in Phase 3.
