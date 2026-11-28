<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Settings dialog Implementation Plan

- **Plan**: context/changes/settings-button/plan.md
- **Mode**: Deep
- **Date**: 2026-11-28
- **Verdict**: REVISE (SOUND after fixes)
- **Findings**: 1 critical, 4 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | FAIL |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

- 16/16 paths ✓
- 14/15 symbols ✓. The `parse_*` functions take `&str`, not `&Value`.
- brief↔plan ✓. Minor: the plan said "ten blocks" where there are nine.

## Findings

### F1: The "sourceProblems" switch would also silence app-fault alerts

- **Severity**: ❌ CRITICAL
- **Impact**: 🔎 MEDIUM (real tradeoff; pause to reason through it)
- **Dimension**: End-State Alignment
- **Location**: Phase 3, §2 Notice gates
- **Detail**:
  - `runtime.rs:472` is one toast from `health::notice_for(&transitions)`.
  - It covers every `SourceId`: Settings, AppData, Gitlab, Github, Reminders and Autostart (`health/mod.rs:25-38`, `:158-177`).
  - Gating that toast would therefore also silence alerts about a damaged database, a broken `settings.json` and a failed autostart sync. That contradicts "app-fault alerts stay always on".
- **Fix**:
  - When `sourceProblems` is off, a pure filter in `health/` drops only the Gitlab and Github transitions before `notice_for` runs. The other sources always toast.
  - The UI label names GitLab and GitHub.
  - There is a test per `SourceId`.
- **Decision**: FIXED. User asked for fixes to be applied directly.

### F2: The list of UI refetches is vague and partly wrong

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Plan Completeness
- **Location**: Phase 3, §3 Change notification and wakes
- **Detail**:
  - The plan said "the work-time and todo hooks" but named no files.
  - `useReminderSettings` listens to no event.
  - `useTodo` listens to `todo-changed`.
  - `useMonthReport` and `useWeekReport` listen to `WORK_TIME_CHANGED`.
  - The GitLab and GitHub lists change only through the woken round.
- **Fix**: Phase 3 now lists each section's path:
  - reminders gets a `settings-changed` listener
  - todo emits `todo-changed`
  - workTime calls the existing `notify()`
  - gitlab and github rely on the wake only
  - `DaySummaryPanel` needs nothing
- **Decision**: FIXED

### F3: Some form-to-block mappings are not stated

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Plan Completeness
- **Location**: Phase 2, §2 Write rules
- **Detail**:
  - `workTime.holidays` is `"pl" | false`, but the form field is a boolean.
  - The refresh blocks are `{ "minutes": N }`.
  - `gitlabFilter` entries are stored lowercased. The plan did not say whether the form shows the raw text or the normalised one.
- **Fix**: Write rules were added:
  - holidays `true` leaves the key out, `false` is written as `false`
  - `refreshMinutes` is written as `{ "minutes": N }`
  - forms show the normalised values
- **Decision**: FIXED

### F4: The digit-shortcut guard cannot read the provider from App

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Architectural Fitness
- **Location**: Phase 4, §4
- **Detail**: The keydown handler is defined in `App` (`src/App.tsx:42-62`). A provider rendered inside App is below it, so App cannot read it.
- **Fix**:
  - `SettingsDialogProvider` wraps `<App />` in `main.tsx`.
  - The handler reads `useSettingsDialog().open`.
- **Decision**: FIXED

### F5: Removing a key with Map::remove reorders the file

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Blind Spots
- **Location**: Phase 2, §1–2
- **Detail**:
  - With `preserve_order` on, `Map::remove` is `swap_remove`, which perturbs the top-level order.
  - Inside a rebuilt block, the writer's own key order replaces the user's.
  - No other code depends on key order; this was verified.
- **Fix**:
  - The writer uses `shift_remove`.
  - Test 2.1 also covers removing a block in the middle.
  - Migration Notes say that only the top-level order is kept.
- **Decision**: FIXED

### F6: Current State anchors and the list of callers are inaccurate

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Plan Completeness
- **Location**: Current State Analysis, Key Discoveries, Phase 1
- **Detail**:
  - The `parse_*` functions take the file text. They do not take a `Value`.
  - These callers were not mentioned:
    - `load_gitlab_filter`
    - `load_todo_limit`
    - `load_reminder_settings_detailed`, which returns the origin
    - `load_work_time_settings`, used through `work_time_settings()`
    - `check_settings`
  - The plan said "ten blocks"; there are nine.
- **Fix**:
  - A list of callers was added and the Key Discovery was corrected.
  - The reminders validator keeps the origin output.
  - The count now says nine in the plan and in the brief.
- **Decision**: FIXED

### F7: An autostart sync during a save can race the startup sync

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Blind Spots
- **Location**: Phase 3, §1
- **Detail**:
  - Nothing serialises access to the Run entry.
  - `app.manage(AutostartState)` must stay in `start`.
- **Fix**:
  - The Contract says `manage` stays in `start`.
  - `sync_now` holds a sync `Mutex` in `AutostartState`.
- **Decision**: FIXED

### F8: The Phase 2 manual check assumes a global invoke

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW (quick decision; fix is obvious and narrowly scoped)
- **Dimension**: Plan Completeness
- **Location**: Phase 2, Manual Verification (2.9)
- **Detail**: There is no `withGlobalTauri`, so DevTools has no `window.__TAURI__`.
- **Fix**: 2.9 now says to use `window.__TAURI_INTERNALS__.invoke(...)`.
- **Decision**: FIXED
