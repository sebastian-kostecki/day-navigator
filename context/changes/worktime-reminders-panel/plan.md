# Panel „Przypomnienia” w „Czas pracy” — Implementation Plan

## Overview

Roadmap S-06, PRD FR-021, GitHub issue #3. The reminders work (FR-018, archived `2026-11-23-timer-reminders`), but the UI never shows their settings. This change adds a read-only Tauri command that returns the two reminder thresholds from `settings.json` and a "Przypomnienia" panel in the "Czas pracy" side column that shows each threshold, whether it is on, and whether the value comes from the file or is the default. There is no editing: a "Ustawienia" button opens `settings.json`, as the GitLab filter bar does.

## Current State Analysis

- `settings::load_reminder_settings(path)` (`src-tauri/src/settings.rs:302`) reads only the top-level `reminders` block and returns `ReminderSettings { long_running: Option<Duration>, not_running_after: Option<NaiveTime> }` (`settings.rs:259`), where `None` means that reminder is off. The defaults (`settings.rs:264`) are 10 h and 09:00. A missing file, a missing or `null` block, or a missing or `null` field gives the default. A field set to `false` turns that reminder off. Anything else is `ReminderSettingsError::SettingsInvalid` (`settings.rs:274`), with no detail. That enum is not `Serialize` today.
- The parse happens in `parse_reminder_settings(text)` (`settings.rs:330`). The per-field match arms (`settings.rs:347-364`) already tell "absent/null" (`None` → default) apart from "set" (`Some(..)`). The panel's "from file" flag can come from that same pass.
- The `work-reminders` thread (`src-tauri/src/work_reminders.rs:43`) uses the defaults when `settings_path` is `None` (unresolved config dir). On an invalid block it skips the round, so reminders are **paused**, not off. The "Przypomnienia" health source reports this with the fixed text `REMINDERS_INVALID_PL` (`src-tauri/src/health/probe.rs:120`).
- `work_reminders.rs` already imports `tauri`. It is the natural home for a reminders command next to the thread. `lib.rs` registers commands in `generate_handler!` (`src-tauri/src/lib.rs:186-219`). The `blocking` helper (`lib.rs:35`) runs work on the blocking pool and needs `E: From<BackgroundFailed>`, a pattern used in `src-tauri/src/events/error.rs:19`. The app's own commands need no capability permission (`src-tauri/capabilities/default.json` description).
- `settings_file::settings_path(&app)` (`src-tauri/src/settings_file.rs:14`) resolves `<config dir>/settings.json`. `open_settings_file` already exists, with a TS wrapper and Polish error messages in `src/gitlab/api.ts:72-92`, used by `src/gitlab/GitlabFilterBar.tsx:45-67`. Feature folders already import from each other (`src/gitlab/GitlabView.tsx:22-25` → `../todo/…`).
- `src/worktime/WorkTimeView.tsx` (after S-05) has a main column (`Wybrany dzień`, `Dni miesiąca`, `Jak liczę`) and a side column holding only `EntriesPanel` (`WorkTimeView.tsx:229-241`). The main column stacks with `.wt-stack` (`src/worktime/WorkTime.css:41`).
- `useMonthReport` (`src/worktime/useMonthReport.ts`) is the hook pattern to follow: `{loading | error(message, lastOkAt) | loaded}`, latest request wins, refetch on `focus` and `visibilitychange`, and `retry`. The reminders panel does not need a Tauri event subscription, since nothing emits on a settings change.
- Shared UI: `Panel` (title / meta / footer), `StatusPill` (`ok` ●, `calm` ○, text override), `ErrorState`, `Notice`, `Button`, and `formatHm` (`src/components/time.ts:82`: `H:MM` from ms, e.g. `10:00`).

## Desired End State

The side column of "Czas pracy" shows a "Przypomnienia" panel under "Wpisy":

| Row | Value (example) | State |
| --- | --- | --- |
| Licznik biegnie dłużej niż | `10:00` · `domyślnie` | `● WŁ.` |
| Brak START-u w dzień roboczy po | `09:30` | `● WŁ.` |

Each row has a short muted line under it that explains the rule, taken from the Rust behaviour: the first reminder repeats every hour while the counter keeps running; the second fires on a workday with no counted work yet and also repeats every hour. When a reminder is set to `false`, its row shows `—` and `○ WYŁ.`. Because the defaults are both on, an off reminder is always from the file, so `domyślnie` never appears next to `WYŁ.`. The footer has a quiet "Ustawienia" button that opens `settings.json`. An open failure shows an inline `Notice`.

An invalid `reminders` block, or a whole-file problem the loader refuses, renders `ErrorState`: "Nie udało się wczytać przypomnień". It says the block is invalid, gives the accepted shape and says reminders are paused until it is fixed. It shows "ostatnio OK HH:MM" when there was an earlier success, plus a retry action. The "Ustawienia" footer button stays available in the error state.

After the user edits `settings.json` and comes back to the app window (focus or visibility), the panel shows the new values without a restart.

Verify with `cargo test` (the new parser-origin, mapping and serialization tests), the standard gates, and the manual checks in Phase 2.

### Key Discoveries:

- One parser for the thread and the panel: extend `parse_reminder_settings` to record where each field came from in the same pass, and keep `load_reminder_settings`'s signature and behaviour for the thread (`settings.rs:302-366`).
- A missing config dir means defaults in the thread (`work_reminders.rs:50`). The command mirrors this so the panel shows what actually runs.
- `REMINDERS_INVALID_PL` (`probe.rs:120`) is the accepted Polish wording for an invalid block. The UI message uses the same wording plus "przypomnienia są wstrzymane".
- Cross-feature import precedent: `src/gitlab/GitlabView.tsx:22-25`.

## What We're NOT Doing

- No editing of reminder settings in the UI. No write to `settings.json`, and no creating the file when it is missing (the "Ustawienia" button reports `Missing`, as today).
- No file watcher and no polling interval. Refresh happens on mount, focus, visibility and retry only.
- No change to the reminder rules, to the thread's 60 s round, to the toasts, or to the "Przypomnienia" health source.
- No showing `notices_allowed()` (always `true` until FR-023/FR-024 exist), no OS notification permission state, and no "next reminder at" time.
- No move of `openSettingsFile` out of `src/gitlab/api.ts`. The panel imports it from there.
- Nothing in `src/health/` or the "Stan źródeł" view (sibling S-07 owns those files), and no `src/components/` change unless a gap turns up.

## Implementation Approach

Rust stays the single source of the values. The loader gains an origin-aware sibling that is built on the same parse function. A thin async command maps the result into a camelCase view type: hours as an integer, the time as zero-padded `"HH:MM"`, and `null` for off, each with `fromFile`. The TS mirror and the `api.ts` wrapper ship in the same commit (Phase 1). Phase 2 adds the hook and the panel, and wires the panel into the side column.

## Phase 1: Reminder-settings command and TS contract

### Overview

Expose the reminder thresholds, their on/off state and their origin through a read-only command, with unit tests, and mirror the types in TS.

### Changes Required:

#### 1. Origin-aware reminder parsing

**File**: `src-tauri/src/settings.rs`

**Intent**: Let one parse pass report, for each of the two fields, whether `settings.json` set it. A value or `false` counts as set. An absent field, a `null` field, an absent or `null` block and a missing file count as not set. The thread's loader keeps its exact behaviour, and the panel gets the extra facts without a second parser that could drift.

**Contract**:
- New `pub struct ReminderOrigin { pub long_running_from_file: bool, pub not_running_after_from_file: bool }` (`Debug, Clone, Copy, PartialEq, Eq, Default`).
- New `pub fn load_reminder_settings_detailed(path: &Path) -> Result<(ReminderSettings, ReminderOrigin), ReminderSettingsError>`, with the same file handling (BOM strip, not-found → defaults with both flags `false`, other IO → `SettingsInvalid`).
- `parse_reminder_settings` returns `(ReminderSettings, ReminderOrigin)`. `load_reminder_settings` keeps its signature and drops the origin.
- `ReminderSettingsError` additionally derives `Serialize` with `#[serde(tag = "kind", content = "detail")]`, so it serializes as `{"kind":"SettingsInvalid"}`. Existing `==` comparisons stay valid.

#### 2. The `reminder_settings` command

**File**: `src-tauri/src/work_reminders.rs`

**Intent**: A read-only command that reads the block off the main thread and returns display-ready values. It mirrors the thread's handling of an unresolved config dir (defaults, both flags `false`). Errors are logged by kind only, without a path or value.

**Contract**:
- `#[serde(rename_all = "camelCase")]` view types (`Serialize`, plus `Debug, PartialEq` for tests):
  - `ReminderThresholds { long_running: LongRunningThreshold, not_running_after: NotRunningAfterThreshold }`
  - `LongRunningThreshold { hours: Option<u32>, from_file: bool }`. `None` = off. Hours are the whole value from the 1..=24 setting.
  - `NotRunningAfterThreshold { time: Option<String>, from_file: bool }`. `None` = off. The time is zero-padded `"HH:MM"`.
- A pure `fn thresholds(settings: &ReminderSettings, origin: ReminderOrigin) -> ReminderThresholds`, unit-tested.
- Error enum `ReminderSettingsQueryError { SettingsInvalid, Background }` (`thiserror::Error` + `Serialize`, `#[serde(tag = "kind", content = "detail")]`), with `From<ReminderSettingsError>` and `From<crate::BackgroundFailed>`.
- `#[tauri::command] pub async fn reminder_settings(app: AppHandle) -> Result<ReminderThresholds, ReminderSettingsQueryError>`: resolves `settings_file::settings_path(&app)`, then `crate::blocking(..)` around `load_reminder_settings_detailed`. `None` path → `ReminderSettings::default()` + `ReminderOrigin::default()`.

#### 3. Registration

**File**: `src-tauri/src/lib.rs`

**Intent**: Make the command callable from the UI.

**Contract**: Add `work_reminders::reminder_settings` to `generate_handler!`. No capability change (app commands need no permission).

#### 4. Rust tests

**Files**: `src-tauri/src/settings.rs` (tests module), `src-tauri/src/work_reminders.rs` (new `#[cfg(test)] mod tests`)

**Intent**: Pin the origin rules and the view mapping with invented data, reusing the existing helpers (`reminders_text`, `write`, `hours`, `hm`).

**Contract**: Cover these cases.
- Origin: missing file → both `false`. Block absent, `null` or `{}` → both `false`. A field `null` → that flag `false`. A field with a value → `true`. A field `false` → `true` and the setting `None`. Mixed (one set, one absent).
- Agreement: for every valid fixture already used by the `reminders_*` tests, `load_reminder_settings_detailed(..).0 == load_reminder_settings(..)`. For every invalid fixture both return `SettingsInvalid`.
- Mapping: defaults → `{ hours: Some(10), fromFile: false }` / `{ time: Some("09:00"), fromFile: false }`. `"09:05"` stays zero-padded. Edges `1` and `24` hours. Off maps to `None`.
- Serialization: `ReminderThresholds` JSON uses `longRunning.hours`, `longRunning.fromFile`, `notRunningAfter.time` and `notRunningAfter.fromFile`, with `null` when off. `ReminderSettingsQueryError::SettingsInvalid` → `{"kind":"SettingsInvalid"}`. `Background` → `{"kind":"Background"}`.

#### 5. TS mirror and API

**Files**: `src/worktime/types.ts`, `src/worktime/api.ts`

**Intent**: Mirror the Rust types in the same commit, and add the typed wrapper with Polish error text.

**Contract**:
- `types.ts`: `ReminderThresholds { longRunning: { hours: number | null; fromFile: boolean }; notRunningAfter: { time: string | null; fromFile: boolean } }` and `ReminderSettingsError = { kind: "SettingsInvalid" } | { kind: "Background" }`.
- `api.ts`: `reminderSettings(): Promise<ReminderThresholds>` (`invoke("reminder_settings")`) and `reminderSettingsErrorMessage(e: unknown): string`. `SettingsInvalid` → the `REMINDERS_INVALID_PL` wording plus "Przypomnienia są wstrzymane, dopóki go nie poprawisz.". `Background` → a short generic Polish failure. Unknown → "Nieoczekiwany błąd (…)", following `openSettingsErrorMessage` (`src/gitlab/api.ts:77`).

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the new origin, agreement, mapping and serialization tests
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/`
- `npm run lint`, `npm run format:check` and `npm run build` pass with zero errors

**Implementation Note**: Phase 1 has no manual rows. Proceed to Phase 2 once automated verification is green.

---

## Phase 2: „Przypomnienia” panel in "Czas pracy"

### Overview

Load the thresholds through a hook, render the panel with loading, error and data states, and place it in the side column under "Wpisy".

### Changes Required:

#### 1. `useReminderSettings` hook

**File**: `src/worktime/useReminderSettings.ts` (new)

**Intent**: Loading, error and data state for the thresholds, refreshed on mount, window focus and visibility, and on retry. The latest request wins. The time of the last success is kept for the error state. No Tauri event subscription and no interval.

**Contract**: `useReminderSettings(): { state: ReminderSettingsState; retry: () => void }`, where `ReminderSettingsState = { state: "loading" } | { state: "error"; message: string; lastOkAt: number | null } | { state: "loaded"; thresholds: ReminderThresholds }`. Follow `useMonthReport.ts` for the request ordering and the focus and visibility listeners.

#### 2. `RemindersPanel` component

**File**: `src/worktime/RemindersPanel.tsx` (new)

**Intent**: The read-only panel. Two rows: "Licznik biegnie dłużej niż" with `formatHm(hours × 3 600 000)` (e.g. `10:00`), and "Brak START-u w dzień roboczy po" with `time` as given. Each row has a muted explanation line ("potem co godzinę, dopóki licznik biegnie" / "gdy dziś nie ma jeszcze wpisu; potem co godzinę"), the `domyślnie` meta text when `fromFile` is `false`, and a `StatusPill`: `ok` "WŁ." when on, `calm` "WYŁ." when off (value `—`). Loading shows `Ładowanie…` (`dn-loading`). An error renders `ErrorState` (title "Nie udało się wczytać przypomnień", message from `reminderSettingsErrorMessage`, `lastSuccess` "ostatnio OK HH:MM" via `formatClock`, `onRetry`). The footer has a quiet `size="sm"` "Ustawienia" button with `icon="external"` and title "Otwórz settings.json w domyślnym edytorze". It calls `openSettingsFile` from `../gitlab/api`, and its error appears as `Notice tone="error"` with `openSettingsErrorMessage`. The footer is present in every state.

**Contract**: `export function RemindersPanel()`. It takes no props, reads its own hook, and never calls `useWorkTimer()` (it does not need the timer).

#### 3. View wiring and layout CSS

**Files**: `src/worktime/WorkTimeView.tsx`, `src/worktime/WorkTime.css`

**Intent**: Put `RemindersPanel` in the side column under `EntriesPanel`, spaced like the main column.

**Contract**: The side column `div` gets the `wt-stack` class (or an equivalent layout-only class) and renders `<RemindersPanel />` after `<EntriesPanel … />`. Component styles, if any, go in a `dn-`/`wt-` prefixed rule using only `src/styles/tokens.css` variables. No literal colours.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` and `npm run build` pass with zero errors
- `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/` (no regression)
- `rg -n "useWorkTimerSource" src` finds it only in `src/worktime/WorkTimerProvider.tsx` and `useWorkTimer.ts` (no second timer subscription)
- `rg -n "#[0-9a-fA-F]{3,8}\b|rgb\(" src/worktime` finds no literal colours

#### Manual Verification:

- Without a `reminders` block, the panel shows `10:00` and `09:00`, both `WŁ.` and both `domyślnie`
- With `"reminders": {"longRunningHours": 6, "notRunningAfter": false}` saved in `settings.json`, focusing the app window shows `6:00` `WŁ.` without `domyślnie` and `—` `WYŁ.` for the second row, with no restart
- With an invalid block (e.g. `"longRunningHours": 99`), the panel shows the error state ("przypomnienia wstrzymane"); after fixing the file and clicking "Spróbuj ponownie" (or refocusing) the values return, with "ostatnio OK" shown while in error after an earlier success
- "Ustawienia" opens `settings.json` in the default editor; with the file renamed away it shows the "Brak pliku settings.json…" notice and creates no file
- The panel sits under "Wpisy" in the side column at 1280 px and wraps sensibly at 800 px; the header counter, START/STOP and the days table behave as before

**Implementation Note**: After automated verification passes, pause for the user's manual confirmation of the rows above before closing the change.

---

## Testing Strategy

### Unit Tests:

- Rust: origin flags for each absent/null/value/false case. Agreement between `load_reminder_settings` and `load_reminder_settings_detailed` over the existing fixtures. The `thresholds` mapping, including the defaults, the 1/24 hour edges, a zero-padded `HH:MM` and off → `null`. JSON shape of the view type and the error enum. All data is invented. No real host names or work data.

### Integration Tests:

- None new. The command is a thin wrapper over the tested loader and the mapping. The UI has no test runner.

### Manual Testing Steps:

1. Start `npm run tauri dev` with no `reminders` block, open "Czas pracy", and check the defaults with `domyślnie`.
2. Edit `settings.json` (values, then `false`, then an invalid value), switching back to the app after each save. Check the values, `WYŁ.`, and the paused error state with retry.
3. Click "Ustawienia" with the file present, then with it moved away.
4. Check the layout at 1280 px and 800 px, and check that the header and the days table did not change.

## Performance Considerations

One small file read per mount, focus or visibility change, run on the blocking pool. This is negligible.

## Migration Notes

None. No store schema change and no settings format change. Existing `settings.json` files keep working.

## References

- Roadmap item: `context/foundation/roadmap.md` → S-06
- PRD: `context/foundation/prd.md` FR-021 (and FR-018 for the reminder rules)
- Reminders change: `context/archive/2026-11-23-timer-reminders/plan.md`
- Loader: `src-tauri/src/settings.rs:253-366`; thread: `src-tauri/src/work_reminders.rs:43`
- Health wording: `src-tauri/src/health/probe.rs:118-135`
- Settings button precedent: `src/gitlab/GitlabFilterBar.tsx:40-67`, `src/gitlab/api.ts:72-92`
- Hook precedent: `src/worktime/useMonthReport.ts`
- View: `src/worktime/WorkTimeView.tsx:229-241`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Reminder-settings command and TS contract

#### Automated

- [x] 1.1 `cargo test` passes in `src-tauri/`, including the new origin, agreement, mapping and serialization tests — 6a87a05
- [x] 1.2 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/` — 6a87a05
- [x] 1.3 `npm run lint`, `npm run format:check` and `npm run build` pass with zero errors — 6a87a05

### Phase 2: „Przypomnienia” panel in "Czas pracy"

#### Automated

- [x] 2.1 `npm run lint`, `npm run format:check` and `npm run build` pass with zero errors
- [x] 2.2 `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/` (no regression)
- [x] 2.3 `rg -n "useWorkTimerSource" src` finds it only in `src/worktime/WorkTimerProvider.tsx` and `useWorkTimer.ts` (no second timer subscription)
- [x] 2.4 `rg -n "#[0-9a-fA-F]{3,8}\b|rgb\(" src/worktime` finds no literal colours

#### Manual

- [ ] 2.5 Without a `reminders` block, the panel shows `10:00` and `09:00`, both `WŁ.` and both `domyślnie`
- [ ] 2.6 With `"reminders": {"longRunningHours": 6, "notRunningAfter": false}` saved, focusing the app window shows `6:00` `WŁ.` without `domyślnie` and `—` `WYŁ.` for the second row, with no restart
- [ ] 2.7 With an invalid block, the panel shows the paused error state; after fixing the file, retry or refocus brings the values back, with "ostatnio OK" shown while in error after an earlier success
- [ ] 2.8 "Ustawienia" opens `settings.json`; with the file moved away it shows the missing-file notice and creates no file
- [ ] 2.9 The panel sits under "Wpisy" at 1280 px and wraps sensibly at 800 px; header counter, START/STOP and the days table behave as before
