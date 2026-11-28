# Settings dialog Implementation Plan

## Overview

A gear button in the app header opens a modal "Ustawienia" dialog: one place to view and change every setting the app has. Seven sections (GitLab, GitHub, Do zrobienia, Czas pracy, Przypomnienia, Powiadomienia, Autostart) cover all nine existing `settings.json` blocks plus one new block, `notices`, with on/off switches for three kinds of Windows notices. Each section saves on its own through a new Rust writer. The writer keeps the file's key order and unknown keys, refuses to overwrite a block that changed on disk since the dialog loaded it, and reports per-field errors from the same Rust validators the loaders use.

This lifts the roadmap's parked item "Editing settings in the app" (`context/foundation/roadmap.md:217`).

## Current State Analysis

- `settings.json` lives at `<app config dir>/settings.json` (`src-tauri/src/settings_file.rs:14`). It is documented as hand-written: "The app reads it and never changes it" (`README.md:102`, `src-tauri/src/settings.rs:1`). No production code writes it, and every `fs::write` of it sits in `#[cfg(test)]`. AGENTS.md already lists `settings.json` in the app config dir as an allowed app-owned write location.
- Each top-level block has its own loader working on a `serde_json::Value`, with value-free errors. The loaders and their parse functions:
  - `gitlab`: `parse_gitlab_settings` (`settings.rs:118`)
  - `github`: `parse_github_settings` (`settings.rs:170`)
  - `todo`: `parse_todo_limit` (`settings.rs:231`)
  - `reminders`: `parse_reminder_settings` (`settings.rs:351`)
  - `gitlabRefresh` / `githubRefresh`: `parse_refresh_minutes` (`settings.rs:446`)
  - `gitlabFilter`: `parse_gitlab_filter` (`settings.rs:541`)
  - `autostart`: `parse_autostart_settings` (`settings/autostart.rs:56`)
  - `workTime`: `parse_work_time_settings` (`settings/work_time.rs:76`)

  Each `parse_*` takes the whole file's text (`&str`), parses it, checks `version` and then reads its block. Inside a block, `deny_unknown_fields` applies. Unknown top-level keys, such as an old `spaces` block, are ignored and must survive a rewrite.
- Production callers of the loaders (signatures stay unchanged, so none of them changes):
  - `load_gitlab_settings`, `load_gitlab_refresh_minutes`, `load_gitlab_filter`: `gitlab_refresh.rs:509`, `:524`, `:358`
  - `load_github_settings`, `load_github_refresh_minutes`: `github_refresh.rs:227`, `:243`
  - `load_todo_limit`: `todo/commands.rs:27`
  - `load_reminder_settings`: `work_reminders.rs:53`, `health/probe.rs:144`. `load_reminder_settings_detailed`: `work_reminders.rs:174`, the `reminder_settings` command, which also returns where each value came from.
  - `load_work_time_settings`: `work_time/commands.rs:44`, through `work_time_settings()`, which `work_time/commands.rs:91`, `day_summary/commands.rs:22,42` and `work_reminders.rs:63` use.
  - `load_autostart_settings`: `autostart/startup.rs:68`
  - `check_settings`: `health/probe.rs:17`, reached from `runtime.rs:381`
- Every reader re-reads the file each time it runs, so a valid edit applies without a restart:
  - GitLab and GitHub blocks: at the start of each refresh round (`gitlab_refresh.rs:500`, `github_refresh.rs:218`).
  - Reminders: every 60 s (`work_reminders.rs:29`).
  - `todo` and `workTime`: on each command.
  - Health check: every 30 s (`runtime.rs:38`).

  The exception is `autostart`. It syncs the HKCU Run entry once per start on the `autostart-sync` thread (`autostart/startup.rs:57`) through the pure `run(settings, is_release, &RunEntry, exe)`.
- Every Windows notice goes through `runtime::toast` (`runtime.rs:362`). The call sites are:
  - new GitLab assignments and review requests (`gitlab_refresh.rs:827`)
  - source down or back (`runtime.rs:472`)
  - the end-of-day toast after a tray STOP (`runtime.rs:214`)
  - a failed tray START/STOP (`runtime.rs:219`)
  - the health-check stall alert (`runtime.rs:540`)
  - work reminders (`work_reminders.rs:87`)

  Only reminders can be turned off today.
- No settings write command exists. The existing settings commands are `open_settings_file`, which opens the file in the default program and never creates it (`settings_file.rs:45`), and `reminder_settings`, which is read-only (`work_reminders.rs`). Both buttons that open the file are labelled "Ustawienia" or link to settings: `src/worktime/RemindersPanel.tsx:88` and `src/gitlab/GitlabFilterBar.tsx:48`.
- `serde_json` is used without `preserve_order` (`src-tauri/Cargo.toml:44`), so a `Value` round-trip would sort keys alphabetically. No atomic-write helper exists. The closest pattern is the temp file plus `sync_all` in `store/backup.rs:125-155`, which uses no-replace hard-link semantics and so does not fit here.
- The UI has no modal, `<dialog>`, scrim, z-index scale, shadow token or gear icon. A previous plan avoided modals on purpose (`context/archive/2026-11-26-end-of-day-summary/plan.md:45`). The digit shortcuts 1–6 sit on a global `keydown` listener (`src/App.tsx:42`). Reusable pieces exist: `Panel`, `Notice`, `Field` (input, textarea, select), `Segmented`, `Tabs` (roving focus), `Button` (`iconOnly`), `Icon`, `ErrorState`. The write-feedback pattern (pending, saved, error) is in `src/own-tasks/OwnTaskEditPanel.tsx:188-304`.

## Desired End State

- A gear icon button, "Ustawienia", sits in the header on every view. It opens a modal dialog. Esc or "Zamknij" closes it, and the digit view shortcuts do nothing while it is open.
- The dialog shows seven tabbed sections. They show every field of every block with its current value, or its default when the block is absent. Each section has its own "Zapisz", which validates in Rust and writes only that section's blocks in one atomic write. A tab whose section has unsaved edits is marked, and closing the dialog with unsaved edits asks for confirmation.
- A bad value shows a Polish message under its field (for a list, on its chip) and nothing is written. The message never repeats the value.
- If a section's block changed on disk after the dialog loaded it, Save writes nothing and the section offers "Wczytaj ponownie".
- If a block on disk is invalid, the section shows that, starts from defaults, and Save first asks "Zastąpić nieprawidłowy blok?".
- If the file is missing, the dialog says it will be created on Save, and Save creates it with `"version": 1`. If the config dir itself is missing, Save fails with an error and nothing is created.
- If the file is not valid JSON or has the wrong `version`, the whole dialog shows an error with "Otwórz settings.json", and nothing can be saved.
- A save keeps key order, unknown top-level keys and a BOM, writes 2-space pretty JSON, and takes effect without a restart:
  - Autostart saves sync the Windows Run entry right away (release builds).
  - GitLab and GitHub saves wake their refresh round.
  - Views showing settings-derived values update.
- The new `notices` block (`newAssignments`, `sourceProblems`, `daySummary`, all `true` by default) gates the three matching toasts. The stall alert, the tray-failure toast and reminders keep their current behaviour.
- The "Ustawienia" button in Przypomnienia and the settings link in the GitLab filter bar open the dialog at their section. "Otwórz settings.json" stays available inside the dialog.

How to verify: run `cargo test`. In `npm run tauri dev`, walk through the Manual Testing Steps below against a backed-up `settings.json`.

### Key Discoveries:

- Loaders are separate per block (`settings.rs:118`, `:170`, `:231`, `:351`, `:446`, `:541`). Each parses the file text and then deserializes its block from a `Value`, so the block part can move into a `validate_<block>(&Value)` while the loader keeps the file parse and the `version` check. One validator per block can therefore serve both the loader (mapping errors to today's value-free errors) and the save path (returning field errors), so the accept set is identical by construction.
- Some loaders are strict and some are lenient. File-level problems are errors for gitlab, github, todo, reminders, autostart and workTime, but defaults for the refresh blocks and gitlabFilter (`settings.rs:419-449`, `:528-545`). The refactor must keep each loader's current file-level behaviour.
- `autostart::run` is pure and already tested against the in-memory `RunEntry` fake. Only the thread wrapper in `autostart/startup.rs` is Tauri-bound, so a save can reuse the same sync plus `store` plus `refresh_tray`.
- `health::notices_allowed()` (`health/mod.rs:388`) is the existing global gate inside `toast`. The per-kind gates belong at the three call sites, not inside `toast`, so the stall and tray-failure toasts stay ungated.
- A native `<dialog>` opened with `showModal()` renders in the top layer above the sticky header (z-index 10), so no z-index token is needed. Only a `--scrim` colour token for `::backdrop` is new.
- `GitlabFilterBar` and `RemindersPanel` already depend on `openSettingsFile` from `src/gitlab/api.ts:72`, which is how settings get opened today.

## What We're NOT Doing

- No credential, token or `glab`/`gh` auth fields, and no CLI arguments from the UI (AGENTS.md hard rules).
- No other new options: no start-view setting and no close-button behaviour (considered and not chosen).
- No separate toggle for review requests. They stay under "Nowe przypisania", like the combined FR-004 notice today.
- No toggle for the health-check stall alert or the tray START/STOP failure toast. These are app-fault and action feedback, so they stay always on. The same goes for health notices about `Settings`, `AppData`, `Reminders` and `Autostart`: `sourceProblems` covers only GitLab and GitHub.
- No editing of a file that is not valid JSON or has the wrong `version`. It is never overwritten, and the user fixes it by hand.
- No `notices` source in "Źródła". An invalid `notices` block shows in the dialog's Powiadomienia section, and all notices stay on.
- No file watcher. Changes made outside the dialog are caught by the fingerprint check at Save time and by "Wczytaj ponownie".
- No keyboard shortcut to open the dialog, no per-field autosave, and no undo history.
- No `tauri-plugin-fs`. All file access stays in Rust commands.
- No version bump or release. That stays the user's decision per the release flow in AGENTS.md.
- Work data (GitLab host, user, filter values, GitHub repos) never goes into logs, error messages, the app data store or tests. Tests use invented values only.

## Implementation Approach

The work runs bottom-up, Rust first, so every UI phase stands on tested contracts.

1. Turn each block's parse into a shared validator that returns field errors. Keep each loader's public signature and behaviour, and add the `notices` loader.
2. Add one settings document module that reads the file into an order-preserving `Value` with its BOM and line-ending style, and a section save that:
   - builds block `Value`s from typed form input
   - runs the shared validators on them
   - checks per-block fingerprints and the invalid-block confirmation
   - writes atomically under a process-wide write lock
3. Add the effects of a save (autostart sync, notice gates, `settings-changed` event, refresh wakes) on top of that.
4. Add the generic UI pieces (Dialog, ChipListField, gear icon, scrim token, provider).
5. Build the seven sections on the typed API.
6. Update the docs and the policy lines that said "never writes".

## Critical Implementation Details

- **State sequencing (save):** keep this order, all under one `Mutex` so two saves never interleave a read-modify-write:
  1. Take the write lock.
  2. Re-read and parse the file.
  3. Compare each target block's fingerprint with the one the dialog sent.
  4. Check the invalid-block confirmation.
  5. Build and validate the new blocks.
  6. Splice them into the `Value`.
  7. Write a temp file in the same directory, `sync_all` it on a write handle, then `fs::rename` it over `settings.json`.
  8. Release the lock.
  9. Run the side effects: autostart sync, event, wakes.

  Any failure before the rename removes the temp file and leaves `settings.json` untouched.
- **Fingerprints:** a fingerprint is an opaque string derived from a block's serialized JSON. An absent block has a fixed marker, so "absent then, present now" is a conflict too. The fingerprint must never be the block text itself. Values are still sent to the UI, but only in typed fields and never as an echo inside errors.
- **Field errors:** a field error is a JSON key path relative to the file plus an optional list index, for example `gitlabFilter.projects.include` with index `3`, and a Polish message. Tests assert the message never contains the offending value; use marker strings like the `LEAK-MARKER-*` convention in `src/bin/fake_cli.rs`.
- **Numbers travel as strings:** numeric form fields go to Rust as strings. Rust parses them, accepting `,` or `.` for `dailyNormHours`, and reports "to nie jest liczba"-style field errors. Parsing rules then live only in Rust. An empty string means "default", so the key is left out.

## Phase 1: Field-level validators and the `notices` block

### Overview

Every block gets one validator that returns either the typed settings or a list of field errors. The existing loaders call the validators and keep their exact signatures, value-free errors and file-level behaviour. A new `notices` block loader is added.

### Changes Required:

#### 1. Shared field-error type and validators

**File**: `src-tauri/src/settings.rs`, `src-tauri/src/settings/autostart.rs`, `src-tauri/src/settings/work_time.rs`, new `src-tauri/src/settings/field_error.rs`

**Intent**: Replace each private `parse_*` block body with a `validate_<block>(block: &Value) -> Result<T, Vec<FieldError>>` that collects every bad field, not just the first. Loaders map `Err(_)` to their existing value-free error, so nothing changes for the refresh threads, commands or health.

**Contract**:
- `FieldError { field: String, index: Option<usize>, message: String }` derives `Serialize` (camelCase) and `Clone`. The message is Polish and never contains the value.
- One validator for each of `gitlab`, `github`, `gitlabRefresh` / `githubRefresh` (shared), `gitlabFilter`, `todo`, `reminders`, `workTime` and `autostart`.
- A block that is not an object, or has unknown keys, is one field error on the block key.
- Public loader signatures in `settings.rs` and `settings::{autostart, work_time}` are unchanged. The `reminders` validator also yields the per-value origin that `load_reminder_settings_detailed` returns today.

#### 2. `notices` block

**File**: new `src-tauri/src/settings/notices.rs`, re-exported from `settings.rs`

**Intent**: Add the new block shape and a lenient loader. If anything is wrong, all three notices stay on, because missing a notice is worse than getting one.

**Contract**: `"notices": { "newAssignments": bool, "sourceProblems": bool, "daySummary": bool }`.
- Each field is `true` when absent or `null`.
- `NoticeSettings { new_assignments, source_problems, day_summary }` defaults to all `true`.
- `load_notice_settings(path) -> NoticeSettings` returns all `true` for any file-level or block problem, with one `log::warn!` per process naming only the block.
- `validate_notices(&Value) -> Result<NoticeSettings, Vec<FieldError>>` is used by the dialog.

### Success Criteria:

#### Automated Verification:

- Every existing loader test in `settings.rs`, `settings/autostart.rs` and `settings/work_time.rs` passes unchanged: `cargo test settings`
- New tests show that each validator reports the right field path (and list index) for every invalid field, and that several bad fields in one block are all reported
- New tests show that no field-error message contains the offending value (marker strings)
- New tests cover the `notices` loader: defaults when absent or `null`, all on when invalid, and `false` respected per field
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase. Phase blocks use plain bullets — the corresponding `- [ ]` checkboxes for these items live in the `## Progress` section at the bottom of the plan.

---

## Phase 2: Settings writer and read/save commands

### Overview

A settings document module reads the file into an order-preserving `Value` and writes one section atomically. Two commands expose this to the UI: `settings_view` and `save_settings_section`. A new `src/settings/` feature folder holds their TS mirrors.

### Changes Required:

#### 1. Order-preserving JSON

**File**: `src-tauri/Cargo.toml`

**Intent**: Enable `serde_json`'s `preserve_order` feature so a rewrite keeps the hand-written key order.

**Contract**: `serde_json = { version = "1", features = ["preserve_order"] }`. `Cargo.lock` is refreshed by `cargo build`. Existing `Value` equality and test behaviour must not change.

#### 2. Settings document and section model

**File**: new `src-tauri/src/settings/document.rs`, new `src-tauri/src/settings/sections.rs`

**Intent**: The document module reads the raw bytes and records what a save needs to reproduce: BOM present or not, CRLF or LF, the parsed `Value`, and file missing vs. config dir missing. Its writer splices blocks and writes atomically. The sections module maps the seven UI sections to their blocks and turns typed form input into block `Value`s, then into the Phase 1 validators.

**Contract**:
- Sections and their blocks:
  - `gitlab` → `gitlab`, `gitlabRefresh`, `gitlabFilter`
  - `github` → `github`, `githubRefresh`
  - `todo` → `todo`
  - `workTime` → `workTime`
  - `reminders` → `reminders`
  - `notices` → `notices`
  - `autostart` → `autostart`
- Write rules:
  - Empty optional values leave their key out: refresh minutes, limit and norm (default), GitLab `user`, `closedDays` (off), and a filter dimension whose two lists are empty.
  - An empty GitLab host removes the `gitlab` key, and empty GitHub repos remove the `github` key. Both mean "not configured".
  - A reminder that is off is written as `false`.
  - `workTime.holidays`: form `true` leaves the key out (default `"pl"`), form `false` is written as `false`.
  - `refreshMinutes` is written as `{ "minutes": N }` in `gitlabRefresh` / `githubRefresh`, and an empty field leaves that block out.
  - Removing a top-level key uses `Map::shift_remove`, never `remove`. With `preserve_order` on, `remove` is `swap_remove` and moves the last key into the gap.
  - A rebuilt block uses the writer's key order inside it. Only the top-level order of the file is kept.
- View rules: forms show values as normalised by the validators (trimmed, and lowercased for `gitlabFilter` entries), so saving an unchanged form writes the normalised values.
- File rules:
  - A new file starts as `{"version": 1}` and is written without a BOM, with LF line endings.
  - An existing file keeps its BOM and line-ending style.
  - Output is 2-space pretty JSON with a trailing newline.
  - The temp file is `settings.json.tmp` in the same directory and is removed on any failure.
- The write lock is a `Mutex` held in managed state (`SettingsWriteLock`).

#### 3. Commands

**File**: new `src-tauri/src/settings/commands.rs`; `src-tauri/src/lib.rs` (register)

**Intent**: `settings_view` returns everything the dialog needs to render. `save_settings_section` saves one section and returns the fresh view. Both are `async` and do file I/O in `spawn_blocking` (the `blocking` helper in `lib.rs:41`).

**Contract** (TS mirrors follow the same shapes):

```rust
#[serde(rename_all = "camelCase")]
struct SettingsView {
    file: FileState,                       // Present | Missing
    sections: SectionViews,                // one SectionView<Form> per section
}
struct SectionView<F> {
    form: F,                               // current values, defaults when absent/invalid
    invalid_blocks: Vec<String>,           // block keys that fail their validator on disk
    fingerprints: BTreeMap<String, String>,// per block key
}
struct SaveRequest {
    section: SectionInput,                 // #[serde(tag = "section", content = "form")]
    fingerprints: BTreeMap<String, String>,
    replace_invalid: bool,
}
struct SaveResult { view: SettingsView, autostart: Option<SourceStatus> }

#[serde(tag = "kind", content = "detail")]
enum SettingsViewError { ConfigDirUnresolved, FileInvalid, Io }
#[serde(tag = "kind", content = "detail")]
enum SaveSettingsError {
    ConfigDirUnresolved, ConfigDirMissing, FileInvalid,
    Conflict { blocks: Vec<String> },
    InvalidOnDisk { blocks: Vec<String> },   // replace_invalid was false
    Invalid { fields: Vec<FieldError> },
    Io,
}
```

Form inputs:
- `gitlab`: `{ host, user, refreshMinutes, filter: { projects|labels|milestones: { include: string[], exclude: string[] }, closedDays } }`
- `github`: `{ repos: string[], refreshMinutes }`
- `todo`: `{ limit }`
- `workTime`: `{ holidays: boolean, dailyNormHours }`
- `reminders`: `{ longRunningOn, longRunningHours, notRunningAfterOn, notRunningAfter }`
- `notices`: `{ newAssignments, sourceProblems, daySummary }`
- `autostart`: `{ enabled }`

Numeric fields are strings, and `"HH:MM"` is a string. Errors carry no path, value or system text. Logs name only the section and the error kind.

#### 4. TS mirrors and API

**File**: new `src/settings/types.ts`, new `src/settings/api.ts`

**Intent**: Typed mirrors of the shapes above, plus one function per command, following the feature-folder convention (`api.ts` is the only `invoke` importer). Add a `settingsErrorMessage(e)` that switches on `kind` to Polish text, like `src/worktime/api.ts:83`.

**Contract**: `settingsView(): Promise<SettingsView>`, `saveSettingsSection(req: SaveRequest): Promise<SaveResult>`.

### Success Criteria:

#### Automated Verification:

- Writer tests with invented data show that unknown top-level keys, key order (also after removing a block in the middle), a BOM and CRLF are preserved, and that no temp file is left behind on failure
- Writer tests show that a new file is created only when the config dir exists, a missing config dir gives `ConfigDirMissing` with nothing created, and a malformed or wrong-version file is never overwritten
- Save tests show that a fingerprint mismatch gives `Conflict` and field errors give `Invalid`, both with the file bytes unchanged
- Save tests show that an invalid block on disk gives `InvalidOnDisk` without `replaceInvalid` and is replaced with it, and that an empty host or empty repos removes the block
- A round-trip test: for every section, `settings_view` → `save_settings_section` with the unchanged form gives a file that every existing loader reads to the same settings
- Serialization tests: errors serialize as `{kind, detail}`, and no error or log text contains a host, repo or filter value
- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo audit` pass
- `npm run build`, `npm run lint` and `npm run format:check` pass

#### Manual Verification:

- On a backed-up real `settings.json`, calling `window.__TAURI_INTERNALS__.invoke('save_settings_section', {...})` for `todo` from DevTools (there is no `withGlobalTauri`) changes only the `todo` block. A diff of the file shows no reordering beyond whitespace normalisation.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Save side effects

### Overview

A successful save takes effect without a restart. Autostart syncs right away, the three notice kinds obey the `notices` block, the UI is told settings changed, and the affected background rounds wake up.

### Changes Required:

#### 1. Autostart applied on save

**File**: `src-tauri/src/autostart/startup.rs`, `src-tauri/src/settings/commands.rs`

**Intent**: Pull the body of the `autostart-sync` thread into a reusable `sync_now(app, path) -> SourceStatus`: load, `run`, log, `store`, `refresh_tray`. Startup and an `autostart` section save both call it. The save returns the status so the section can show the outcome, including the existing dev-only note in debug builds.

**Contract**: Startup behaviour is unchanged. `SaveResult.autostart` is `Some` only for the `autostart` section. It stays the only registry writer (`hkcu.rs`).
- `app.manage(AutostartState(...))` (`startup.rs:58`) stays in `start` and does not move into `sync_now`.
- `sync_now` holds a sync `Mutex` (a field next to the status in `AutostartState`) for the whole load, `run`, `store` sequence. A save that lands during the startup sync then waits instead of racing on the Run entry.

#### 2. Notice gates

**File**: `src-tauri/src/gitlab_refresh.rs` (`toast_new_assignments`, around line 818), `src-tauri/src/runtime.rs` (health notice around line 472, day-summary toast around line 214), `src-tauri/src/settings/notices.rs`

**Intent**: Before each of the three toasts, read `load_notice_settings` and skip the toast when its kind is off. Pure decisions go in a small tested function; the call sites only ask it. Event-log records, tray and snapshots are unaffected: only the Windows toast is skipped.

**Contract**:
- `newAssignments` gates `toast_new_assignments`, which covers assignments and review requests.
- `sourceProblems` covers only the `Gitlab` and `Github` sources. The health notice at `runtime.rs:472` is one toast built by `health::notice_for(&transitions)` for every `SourceId` (`health/mod.rs:25-38`, `:158-177`). So when `sourceProblems` is off, a pure function in `health/` (for example `filter_transitions(&[Transition], NoticeSettings) -> Vec<Transition>`) drops the `Gitlab` and `Github` transitions before `notice_for` runs. `Settings`, `AppData`, `Reminders` and `Autostart` transitions always toast. The UI label is "Problemy ze źródłami (GitLab, GitHub)".
- `daySummary` gates the tray STOP `stop_notice` toast.
- `runtime.rs:219` (tray failure), `runtime.rs:540` (stall) and `work_reminders.rs:87` stay ungated by this block.

#### 3. Change notification and wakes

**File**: `src-tauri/src/settings/commands.rs`, `src/worktime/useReminderSettings.ts`, `src/settings/api.ts` (event name constant)

**Intent**: After a successful write:
- emit a `settings-changed` event with the section id
- wake GitLab (`GitlabRefreshState::wake`, `gitlab_refresh.rs:136`) for `gitlab`. The round re-reads `gitlab`, `gitlabRefresh` and `gitlabFilter` (`load_filter`, `gitlab_refresh.rs:358`).
- wake GitHub (`github_refresh.rs:95`) for `github`
- run a health round (same wake as `check_health_now`, `runtime.rs:118`) for every section

Per section, the UI gets the new values like this:
- `reminders`: `useReminderSettings.ts` listens to no event today (only focus and visibility). It gets a `settings-changed` listener that refetches when `section === "reminders"`.
- `todo`: Rust emits the existing `todo-changed` event, which `src/todo/useTodo.ts:94` already listens to.
- `workTime`: Rust calls the existing work-time `notify()` (`work_time/commands.rs:109-115`), which emits `WORK_TIME_CHANGED`. `useMonthReport.ts:62` (norm, tiles in `todo/summary.ts`) and `useWeekReport.ts:65` refetch on it. The other listeners (`useWorkTimer.ts:97`, `useWorkEntries.ts:61`) only refetch, which is harmless.
- `gitlab` and `github`: no UI refetch. The lists change through the woken round and its `gitlab-changed` / `github-changed` push (`useGitlab.ts:42`, `useGithub.ts:40`).
- `DaySummaryPanel` reads work settings only when it opens, so it needs nothing.

**Contract**: The event name is `settings-changed`, with payload `{ section: string }` and no values. Wake failures are logged by kind and never fail the save.

### Success Criteria:

#### Automated Verification:

- An autostart test: the save path's sync against the in-memory `RunEntry` fake writes the entry when on, removes it when off, and leaves it unchanged in a debug build (`DevOnly`)
- Notice-gate tests: each kind off skips only its toast. With `sourceProblems` off, `Gitlab` and `Github` transitions are dropped while `Settings`, `AppData`, `Reminders` and `Autostart` transitions still toast (one case per `SourceId`). Absent or invalid `notices` keeps all on. The stall, tray-failure and reminder toasts are never gated
- `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- `npm run build`, `npm run lint` and `npm run format:check` pass

#### Manual Verification:

- In a built release-mode app (`npm run tauri build -- --no-bundle`), saving Autostart on or off adds or removes the Run value without a restart, and "Źródła" shows the new autostart state
- With `daySummary: false`, a tray STOP shows no toast but the tray updates. With `sourceProblems: false`, GitLab or GitHub going down changes the tray and panel with no toast, while a broken `settings.json` still toasts

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 4: Dialog groundwork

### Overview

Add the generic UI pieces the sections need: a shared modal `Dialog`, a shared `ChipListField` list editor, the gear icon, the scrim token, a provider that lets any view open the dialog at a section, and the header button.

### Changes Required:

#### 1. Dialog component

**File**: new `src/components/Dialog/Dialog.tsx`, `src/components/Dialog/Dialog.css`, `src/components/index.ts`

**Intent**: A thin wrapper over the native `<dialog>`. `showModal()` and `close()` follow an `open` prop. The `cancel` event (Esc) calls `onClose` instead of closing by itself, so the caller can guard unsaved edits. Focus returns to the opener on close.

**Contract**: Props are `{ open, onClose, title, children, footer? }`. A heading labels the dialog through `aria-labelledby`. CSS uses `dn-dialog`, tokens only, and the `::backdrop` uses `--scrim`.

#### 2. ChipListField component

**File**: new `src/components/ChipListField/ChipListField.tsx`, `.css`, `src/components/index.ts`

**Intent**: Edit a list of strings as removable chips with an add input.
- Enter adds the trimmed text.
- Pasting text with line breaks or commas adds each piece, so 100 entries can be pasted at once.
- × removes a chip, and Backspace in an empty input removes the last chip.
- Exact duplicates after trimming are not added again.
- A chip with an error from Rust is highlighted and its message is shown.

All normalising beyond trimming (lowercasing, path rules) stays in Rust.

**Contract**: Props are `{ label, values: string[], onChange(values), placeholder?, max?, errors?: Record<number, string>, disabled? }`. The visible label is linked to the input. Each chip's × has `aria-label` "Usuń <entry>".

#### 3. Gear icon and scrim token

**File**: `src/components/Icon/Icon.tsx` (`IconName` union and `PATHS`), `src/styles/tokens.css`

**Intent**: Add a stroke-style `settings` (gear) icon, and add `--scrim` as the backdrop colour token. It is the only new token.

**Contract**: `IconName` gains `"settings"`. `--scrim` is defined in `tokens.css` only.

#### 4. Provider, header button and shortcut guard

**File**: new `src/settings/SettingsDialogProvider.tsx`, new `src/settings/useSettingsDialog.ts`, `src/main.tsx`, `src/App.tsx`

**Intent**: A React context, like `WorkTimerProvider`, holding `{ open: SectionId | null, openAt(section?), close() }`. `main.tsx` renders `SettingsDialogProvider` around `<App />`, because the digit-shortcut `keydown` handler is defined in `App` itself (`src/App.tsx:42-62`) and must read the context. `App.tsx` renders the dialog once.
- The header gets a `Button iconOnly icon="settings" aria-label="Ustawienia"` after `WorkTimerHeader`.
- The digit shortcut handler reads `useSettingsDialog().open` and returns early while it is not `null`. It already skips editable targets, modifier keys and `defaultPrevented` (`App.tsx:28-45`).
- Phase 4 renders a placeholder body. Phase 5 fills it.

**Contract**: `useSettingsDialog()` is the only way to open the dialog. AGENTS.md gets the provider line in Phase 6.

### Success Criteria:

#### Automated Verification:

- `npm run build` (tsc strict plus Vite) shows zero errors
- `npm run lint` and `npm run format:check` pass
- `src/components/Dialog/Dialog.css` and `src/components/ChipListField/ChipListField.css` have no literal colours: `rg "#[0-9a-fA-F]{3,8}|rgba?\(" src/components/Dialog src/components/ChipListField` finds nothing

#### Manual Verification:

- In `npm run tauri dev`, the gear button opens the dialog over the sticky header from every view. Esc and "Zamknij" close it, focus returns to the gear, and pressing 1–6 while it is open does not switch views
- `ChipListField`, tried in the placeholder body: Enter adds, multi-line paste adds several, × and Backspace remove, duplicates are ignored, and it works with keyboard only
- In a built app (`npm run tauri build -- --debug --no-bundle`), the DevTools console shows no CSP violation and no "IPC custom protocol failed"

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 5: The seven sections

### Overview

Fill the dialog with the seven sections on top of the Phase 2 API, each with its own save, errors, conflict and replace flows. Point the existing settings entry points at the dialog.

### Changes Required:

#### 1. Dialog screen and state

**File**: new `src/settings/SettingsDialog.tsx`, new `src/settings/useSettings.ts`, new `src/settings/SettingsDialog.css` (layout only)

**Intent**: `useSettings` loads `settings_view` when the dialog opens and keeps, per section, a draft form, a dirty flag, a pending flag, field errors, and the last error kind. It reloads one section on "Wczytaj ponownie" and replaces the whole view from each `SaveResult`. The screen shows the following:
- `Tabs` for the seven sections, with a dirty marker on the tab label.
- A loading state, and an `ErrorState` for a view error, with retry and "Otwórz settings.json".
- A "Brak pliku — zostanie utworzony przy zapisie" notice when the file is missing.
- A footer with "Otwórz settings.json" and "Zamknij".
- Closing with dirty sections shows an inline confirmation naming them: "Niezapisane zmiany w: …" with "Zamknij bez zapisu" / "Wróć".

**Contract**: Loading, error and data states as in the AGENTS.md frontend conventions. No section ever renders an empty form in place of a failed load.

#### 2. Section components

**File**: new `src/settings/sections/*.tsx`

**Intent**: One component per section, using `Field`, `Segmented` or switch-style `Button pressed`, `ChipListField` and `Notice`. The fields per section:
- **GitLab**: host, user, refresh minutes, and a filter with include and exclude chip lists for projects, labels and milestones, plus "zamknięte z ostatnich N dni" (empty = off).
- **GitHub**: a repos chip list (max 50) and refresh minutes.
- **Do zrobienia**: limit.
- **Czas pracy**: Polish holidays on/off and the daily norm.
- **Przypomnienia**: two reminders, each on/off with a value.
- **Powiadomienia**: three switches with one-line explanations, plus a note that the stall alert is always on.
- **Autostart**: a switch, the status returned by the save, and the dev-only note.

Placeholders show the defaults. Each section's "Zapisz" handles these cases:
- **Pending:** disables the section's fieldset.
- **`Invalid`:** maps field errors to their fields or chips.
- **`Conflict`:** shows "Zmieniono w pliku — wczytaj ponownie" with the action.
- **Invalid block on disk:** with `invalidBlocks` non-empty, asks "Zastąpić nieprawidłowy blok?" inline before sending `replaceInvalid: true`.
- **Success:** shows a "Zapisano" status in an always-mounted `role="status"` region, the pattern from `OwnTaskEditPanel.tsx:291`.

**Contract**: Field-error keys from Rust (`field` plus `index`) map 1:1 to section inputs. An unmapped key is shown at the top of the section, never dropped.

#### 3. Existing entry points

**File**: `src/worktime/RemindersPanel.tsx`, `src/gitlab/GitlabFilterBar.tsx`

**Intent**: The "Ustawienia" button in Przypomnienia opens the dialog at `reminders`. The filter bar's settings link opens it at `gitlab`. `openSettingsFile` stays in use inside the dialog footer and error state.

**Contract**: Both use `useSettingsDialog().openAt(section)`.

### Success Criteria:

#### Automated Verification:

- `npm run build` shows zero errors
- `npm run lint` and `npm run format:check` pass
- `cargo test` passes, as a regression check

#### Manual Verification:

- Every section shows the current values from a filled `settings.json` and the defaults from an empty one, and saving each section changes only its blocks in the file
- Each save shows up without a restart: the GitLab list after the woken round, the GitHub list, the "To do" limit, the norm in the month report, the Przypomnienia panel and the autostart status
- A bad value in each section (for example limit 25, norm 7.33, host with a space, repo `bad`, a project chip `/x`) shows its Polish message on the right field or chip, and the file is unchanged
- Editing the same block in an editor while the dialog is open, then pressing Save, shows the conflict and writes nothing. "Wczytaj ponownie" shows the new value
- A typo block on disk shows the invalid state, and Save asks before replacing it
- With `settings.json` deleted, the notice shows and Save creates the file. With it made malformed, the whole dialog shows the error and nothing can be saved
- The dirty tab marker and the close confirmation work. The Przypomnienia and filter-bar entry points open the right section
- In a built app (`npm run tauri build -- --debug --no-bundle`), the DevTools console shows no CSP violation

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 6: Docs and policy

### Overview

Update every place that says the app never writes `settings.json`, document the `notices` block and the dialog, and record that the parked roadmap item is lifted.

### Changes Required:

#### 1. User docs

**File**: `README.md` ("Configuration"), `CHANGELOG.md`

**Intent**: Replace "The app reads it and never changes it" with how the dialog edits the file:
- per-section save
- key order and unknown keys kept, formatting normalised
- created on Save when missing
- a malformed file is never overwritten
- a conflict is refused

Add `notices` to the example and the table, including that an invalid block keeps all notices on and is shown in the dialog. Remove the "restart the app after a change" note for autostart saved from the dialog; hand edits still need a restart. Add an unreleased CHANGELOG entry.

**Contract**: The README example stays valid for every loader. Check this by pasting it into a temp file read by the loaders in a Rust test, or by eye; no README-parsing test exists today.

#### 2. Agent rules and code comments

**File**: `AGENTS.md`, `src-tauri/src/settings.rs:1`, `src-tauri/src/settings_file.rs:1-3`

**Intent**:
- AGENTS.md says `settings.json` is written only by `save_settings_section`: per section, atomic, preserving unknown keys, and never when the file is malformed.
- AGENTS.md lists `src/settings/` and `SettingsDialogProvider` / `useSettingsDialog()` in the frontend conventions, and the shared `Dialog` as the one modal pattern.
- AGENTS.md notes that the `notices` block gates three toasts.
- The module doc comments drop "read-only" and "never creates it", or say where the writer lives.

**Contract**: The hard rules stay intact: no secrets, read-only CLIs, work data never in logs or tests.

#### 3. Product docs

**File**: `context/foundation/roadmap.md`, `context/foundation/prd.md`

**Intent**:
- In the roadmap, remove "Editing settings in the app" from Parked and note it was lifted on 2026-11-28 by this change.
- In the PRD, add a one-line note, following the M-4 practice of PRD notes without new FR numbers, that settings can be edited in the app and three notice kinds can be turned off.

**Contract**: No FR numbers added or changed.

#### 4. Design system

**File**: design-system artifact https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY

**Intent**: Add `Dialog`, `ChipListField`, the gear icon and `--scrim`, so the artifact keeps matching the app as the M-2 practice requires.

**Contract**: The artifact is read first, then republished with the additions only.

### Success Criteria:

#### Automated Verification:

- `npm run format:check` passes (Prettier covers `src/` only; Markdown is checked by eye)
- `rg -n "never changes it|never writes it|Read-only loader" README.md AGENTS.md context/foundation/roadmap.md src-tauri/src/settings.rs src-tauri/src/settings_file.rs` finds nothing
- `cargo test` passes

#### Manual Verification:

- README, AGENTS.md, roadmap and PRD read consistently with the shipped behaviour
- The design-system artifact shows `Dialog`, `ChipListField`, the gear icon and `--scrim`

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Validators: field path, index, and several errors per block; parity with each loader's existing tests; no value in messages.
- `notices`: defaults, per-field `false`, invalid keeps all on.
- Writer: order, unknown keys, BOM, CRLF, new file, missing config dir, malformed file never touched, temp cleanup.
- Save: conflict, invalid on disk with and without replace, invalid input, block removal on an empty host or repos, round-trip parity for every section.
- Autostart save sync against the `RunEntry` fake.
- Notice gates per kind.

### Integration Tests:

- A `src-tauri/tests/` test that drives `save_settings_section` logic end to end on a temp config dir. It follows the `test_store_dir()` style and uses invented values only (`gitlab.example.com`, `octo-org/alpha`).

### Manual Testing Steps:

1. Back up `%APPDATA%\com.sebas.day-navigator\settings.json` and the app data dir (see memory note: back up the DB before dev runs).
2. Open the dialog from each view. Check Esc, close and focus return, and that the digit keys are blocked.
3. For each section: change a value, save, and confirm the file diff and the live effect.
4. For each section: enter a bad value and confirm the field message and that the file is unchanged.
5. Edit a block in an editor while the dialog is open, then save. Confirm the conflict, then reload.
6. Put a typo in `gitlabFilter`, then open the dialog. Confirm the invalid state and the replace confirmation.
7. Delete the file and save `todo`. Confirm it is created with `version: 1`. Make the file malformed and confirm the whole-dialog error.
8. Release build: toggle autostart and check the Run value with `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v com.sebas.day-navigator`.
9. Turn each notice kind off and trigger it (tray STOP, a source down, a new assignment via `fake-cli` in dev). Confirm no toast, while tray, panel and event log still update.
10. Restore the backup.

## Performance Considerations

The notice gates read `settings.json` once per toast, which is rare and small. Saves are user-driven. No caching is needed.

## Migration Notes

No store migration. Existing hand-written files keep working unchanged. The first save from the dialog normalises the whitespace of the whole file and the key order inside the saved blocks, but keeps the top-level key order and unknown keys, so a user who wants their exact formatting back should keep a copy (README note). A missing `notices` block means all notices stay on, which is today's behaviour.

## References

- Roadmap parked item: `context/foundation/roadmap.md:217`
- Loaders: `src-tauri/src/settings.rs:93-623`, `src-tauri/src/settings/autostart.rs`, `src-tauri/src/settings/work_time.rs`
- Autostart sync: `src-tauri/src/autostart/startup.rs:57`, `src-tauri/src/autostart/mod.rs`
- Toast sites: `src-tauri/src/runtime.rs:214`, `:219`, `:362`, `:472`, `:540`; `src-tauri/src/gitlab_refresh.rs:827`; `src-tauri/src/work_reminders.rs:87`
- Write-feedback UI pattern: `src/own-tasks/OwnTaskEditPanel.tsx:188-304`
- Existing settings entry points: `src/worktime/RemindersPanel.tsx:88`, `src/gitlab/GitlabFilterBar.tsx:48`, `src-tauri/src/settings_file.rs:45`
- No-modal precedent: `context/archive/2026-11-26-end-of-day-summary/plan.md:45`
- Temp-file + `sync_all` reference: `src-tauri/src/store/backup.rs:125-155`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Field-level validators and the `notices` block

#### Automated

- [x] 1.1 Every existing loader test in `settings.rs`, `settings/autostart.rs` and `settings/work_time.rs` passes unchanged: `cargo test settings` — 4b9fe8a
- [x] 1.2 New tests show that each validator reports the right field path (and list index) for every invalid field, and that several bad fields in one block are all reported — 4b9fe8a
- [x] 1.3 New tests show that no field-error message contains the offending value (marker strings) — 4b9fe8a
- [x] 1.4 New tests cover the `notices` loader: defaults when absent or `null`, all on when invalid, and `false` respected per field — 4b9fe8a
- [x] 1.5 `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean — 4b9fe8a

### Phase 2: Settings writer and read/save commands

#### Automated

- [x] 2.1 Writer tests with invented data show that unknown top-level keys, key order (also after removing a block in the middle), a BOM and CRLF are preserved, and that no temp file is left behind on failure
- [x] 2.2 Writer tests show that a new file is created only when the config dir exists, a missing config dir gives `ConfigDirMissing` with nothing created, and a malformed or wrong-version file is never overwritten
- [x] 2.3 Save tests show that a fingerprint mismatch gives `Conflict` and field errors give `Invalid`, both with the file bytes unchanged
- [x] 2.4 Save tests show that an invalid block on disk gives `InvalidOnDisk` without `replaceInvalid` and is replaced with it, and that an empty host or empty repos removes the block
- [x] 2.5 A round-trip test: for every section, `settings_view` → `save_settings_section` with the unchanged form gives a file that every existing loader reads to the same settings
- [x] 2.6 Serialization tests: errors serialize as `{kind, detail}`, and no error or log text contains a host, repo or filter value
- [x] 2.7 `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo audit` pass
- [x] 2.8 `npm run build`, `npm run lint` and `npm run format:check` pass

#### Manual

- [ ] 2.9 On a backed-up real `settings.json`, calling `window.__TAURI_INTERNALS__.invoke('save_settings_section', {...})` for `todo` from DevTools (there is no `withGlobalTauri`) changes only the `todo` block. A diff of the file shows no reordering beyond whitespace normalisation.

### Phase 3: Save side effects

#### Automated

- [ ] 3.1 An autostart test: the save path's sync against the in-memory `RunEntry` fake writes the entry when on, removes it when off, and leaves it unchanged in a debug build (`DevOnly`)
- [ ] 3.2 Notice-gate tests: each kind off skips only its toast. With `sourceProblems` off, `Gitlab` and `Github` transitions are dropped while `Settings`, `AppData`, `Reminders` and `Autostart` transitions still toast (one case per `SourceId`). Absent or invalid `notices` keeps all on. The stall, tray-failure and reminder toasts are never gated
- [ ] 3.3 `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [ ] 3.4 `npm run build`, `npm run lint` and `npm run format:check` pass

#### Manual

- [ ] 3.5 In a built release-mode app (`npm run tauri build -- --no-bundle`), saving Autostart on or off adds or removes the Run value without a restart, and "Źródła" shows the new autostart state
- [ ] 3.6 With `daySummary: false`, a tray STOP shows no toast but the tray updates. With `sourceProblems: false`, GitLab or GitHub going down changes the tray and panel with no toast, while a broken `settings.json` still toasts

### Phase 4: Dialog groundwork

#### Automated

- [ ] 4.1 `npm run build` (tsc strict plus Vite) shows zero errors
- [ ] 4.2 `npm run lint` and `npm run format:check` pass
- [ ] 4.3 `src/components/Dialog/Dialog.css` and `src/components/ChipListField/ChipListField.css` have no literal colours: `rg "#[0-9a-fA-F]{3,8}|rgba?\(" src/components/Dialog src/components/ChipListField` finds nothing

#### Manual

- [ ] 4.4 In `npm run tauri dev`, the gear button opens the dialog over the sticky header from every view. Esc and "Zamknij" close it, focus returns to the gear, and pressing 1–6 while it is open does not switch views
- [ ] 4.5 `ChipListField`, tried in the placeholder body: Enter adds, multi-line paste adds several, × and Backspace remove, duplicates are ignored, and it works with keyboard only
- [ ] 4.6 In a built app (`npm run tauri build -- --debug --no-bundle`), the DevTools console shows no CSP violation and no "IPC custom protocol failed"

### Phase 5: The seven sections

#### Automated

- [ ] 5.1 `npm run build` shows zero errors
- [ ] 5.2 `npm run lint` and `npm run format:check` pass
- [ ] 5.3 `cargo test` passes, as a regression check

#### Manual

- [ ] 5.4 Every section shows the current values from a filled `settings.json` and the defaults from an empty one, and saving each section changes only its blocks in the file
- [ ] 5.5 Each save shows up without a restart: the GitLab list after the woken round, the GitHub list, the "To do" limit, the norm in the month report, the Przypomnienia panel and the autostart status
- [ ] 5.6 A bad value in each section (for example limit 25, norm 7.33, host with a space, repo `bad`, a project chip `/x`) shows its Polish message on the right field or chip, and the file is unchanged
- [ ] 5.7 Editing the same block in an editor while the dialog is open, then pressing Save, shows the conflict and writes nothing. "Wczytaj ponownie" shows the new value
- [ ] 5.8 A typo block on disk shows the invalid state, and Save asks before replacing it
- [ ] 5.9 With `settings.json` deleted, the notice shows and Save creates the file. With it made malformed, the whole dialog shows the error and nothing can be saved
- [ ] 5.10 The dirty tab marker and the close confirmation work. The Przypomnienia and filter-bar entry points open the right section
- [ ] 5.11 In a built app (`npm run tauri build -- --debug --no-bundle`), the DevTools console shows no CSP violation

### Phase 6: Docs and policy

#### Automated

- [ ] 6.1 `npm run format:check` passes (Prettier covers `src/` only; Markdown is checked by eye)
- [ ] 6.2 `rg -n "never changes it|never writes it|Read-only loader" README.md AGENTS.md context/foundation/roadmap.md src-tauri/src/settings.rs src-tauri/src/settings_file.rs` finds nothing
- [ ] 6.3 `cargo test` passes

#### Manual

- [ ] 6.4 README, AGENTS.md, roadmap and PRD read consistently with the shipped behaviour
- [ ] 6.5 The design-system artifact shows `Dialog`, `ChipListField`, the gear icon and `--scrim`
