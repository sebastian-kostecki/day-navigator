# Timer Reminders Implementation Plan

## Overview

Roadmap slice S-10 (PRD FR-018): remind the user when the work-time counter has been running too long ("czy nadal pracujesz?", a forgotten STOP) and when, on a workday after a set hour, no work has been recorded yet (a forgotten START). Both triggers live in a new `reminders` block of `settings.json`, default to 10 h and 09:00 (Open Roadmap Question #3), and can each be switched off. Reminders are Windows toasts sent from a Rust background thread, so they fire while the window is hidden in the tray.

## Current State Analysis

- S-08 and S-09 are done. Work entries live in the SQLite store (`work_entries`, UTC epoch ms, one running row enforced by a partial unique index, `src-tauri/src/store/schema.rs:23-24`). Days off live in `days_off` (migration 5, `schema.rs:90-99`).
- The `work_time` module is tauri-free and never reads the clock: functions take `now: i64` and `tz: &Tz` (`src-tauri/src/work_time/mod.rs:1-9`). Unit tests run in `Europe/Warsaw` with a fixed "now" (`work_time/tests.rs:14-28`).
- There is no public way to ask "running since when" without computing the whole `status()` (which also builds the month report): `running_entry(conn)` is private (`work_time/mod.rs:105-112`) and `is_running` returns only a bool (`mod.rs:240-243`).
- The workday rule exists and S-09 asked S-10 to reuse it: `report::is_workday` (`work_time/report.rs:165-171`) and `report::day_kind(conn, day)` (`report.rs:74-82`, `pub(super)`, so usable from a sibling inside `work_time`).
- Toasts are sent only from the private `runtime::toast` (`src-tauri/src/runtime.rs:234-241`), gated by `health::notices_allowed()` (`src-tauri/src/health/mod.rs:362-367`, always `true` for now). Text is Polish and built in Rust; toast clicks are not handled.
- Background work runs on named `std::thread`s that loop on `recv_timeout` (`runtime.rs:263-282`, `gitlab_refresh.rs:168-200`) and re-read their own settings block every round. Closing the window hides it to the tray; only "Zakończ" quits (`lib.rs:189-196`, `lib.rs:227-236`). No autostart.
- `settings.json` is hand-edited and read-only for the app. Each feature reads its own top-level block; `load_todo_limit` (`src-tauri/src/settings.rs:137-187`) is the template: absent/`null` = default, anything else invalid = value-free `SettingsInvalid`, `deny_unknown_fields`, BOM stripped, `version == 1` required.
- The health round assembles sources in `runtime::probe_round` (`runtime.rs:247-261`); `SourceId` serializes as a string key (`health/mod.rs:22-54`), and the TS mirror types `id` as `string` (`src/health/types.ts`), so a new source needs no TS change. `short_reason("SettingsInvalid")` already maps to "nieprawidłowe ustawienia" (`health/mod.rs:179`).

## Desired End State

- With the app running (window open or hidden in the tray), an entry that has been running for 10 h produces a toast "Nawigator dnia — czy nadal pracujesz?", then one more every full hour while the same entry keeps running.
- On a workday (Mon–Fri, not marked off), from 09:00 local time, if the counter is stopped and today has no counted work yet, a toast "Nawigator dnia — licznik nie biegnie" appears, then one more every full hour until START or the end of the day.
- `settings.json` may hold `"reminders": { "longRunningHours": 1..=24 | false, "notRunningAfter": "HH:MM" | false }`; absent or `null` fields mean the defaults, `false` turns that reminder off.
- An invalid `reminders` block stops reminders and shows a "Przypomnienia" source in error in the tray and the health panel (one toast on the transition, as for every source). Fixing the file recovers on the next round.
- `cargo test` proves the rules: thresholds, hourly slots, no repeat within a slot, midnight, DST, day off, weekend, restart and sleep.

Worked example (invented data): Monday 2026-11-23, entries 08:00–12:00 and 13:00–(running), defaults. At 19:00 the running entry has run 6 h, so there is no toast. At 23:00 it has run 10 h, so the first toast fires ("od 13:00, 10 h"). At 23:40 no toast (same slot). At 2026-11-24 00:00 the second toast fires ("od wczoraj 13:00, 11 h"). On Tuesday 2026-11-24 with the counter stopped at 00:30 after a STOP, no "nie biegnie" toast that day, because today already has counted work (00:00–00:30).

### Key Discoveries:

- `work_time` purity rule (`work_time/mod.rs:1-9`): the new rules take `now` and `tz` as parameters; only the thread reads `Utc::now()` and `Local`.
- `report::day_kind` (`work_time/report.rs:74-82`) gives Workday/DayOff/Weekend inside one `store.read`, so the facts query and the workday check share a single read.
- `rules::day_bounds` (`work_time/rules.rs:43`) and the private `in_day` (`work_time/mod.rs:115`) already give today's counted entries split at local midnight, DST-safe.
- `StoreHandle::with_store(today, f)` (`src-tauri/src/store/mod.rs:233-239`) is how a thread reaches the store; `work_time/commands.rs:36-47` shows the call shape.
- `runtime.rs:5-9`: never hold a lock across tray, notification or emit calls, since they block on the main thread.

## What We're NOT Doing

- No settings screen and no settings writes: the block is hand-edited like `todo` and `gitlab`.
- No toast click handling, no toast action buttons, no tray START/STOP items, no tray tooltip line, no header hint.
- No "today's total too long" (overwork) reminder: the long-run trigger measures the running entry only.
- No reminder after a break: "nie biegnie" fires only while today has no counted work.
- No persistence of sent reminders: the memory is in-process; a restart may re-send the current slot once.
- No catch-up of missed slots after sleep: one toast for the current slot, not a backlog.
- No autostart, no change to `notices_allowed()` (pause/day-off modes stay for later), no wake channel for this thread.
- No health reporting of a dead reminder thread (a spawn failure or panic is only logged, unlike GitLab's `ThreadDown`); accepted as rare.
- No change to the norm, days off, or work-time commands and UI; no TS changes.

## Implementation Approach

Split, as in `source-health-status` and `work-timer`, into a pure, tauri-free rule layer and a thin side-effect loop. Phase 1 builds and proves everything that decides *whether* and *what* to remind (settings parsing, facts query, slot rules, Polish text). Phase 2 adds the thread that reads the clock, settings and store every 60 s, sends due toasts through a shared gated helper, and adds the "Przypomnienia" health source.

Repetition uses **hourly slots** relative to the trigger instant, not wall-clock timers. A slot counter makes "every hour, never twice per slot, no backlog after sleep" a pure comparison, which makes it testable without a fake clock:

- long run: `elapsed = now − start_ms`; due when `elapsed ≥ threshold`; `slot = (elapsed − threshold) / 1 h`; `slot_start = start_ms + threshold + slot · 1 h`; key `(entry_id, slot_start)`.
- not running: `anchor = today at notRunningAfter (local)`; due when the counter is stopped, today is a workday, today has no counted work and `now ≥ anchor`; `slot = (now − anchor) / 1 h`; `slot_start = anchor + slot · 1 h`; key `(today, slot_start)`.
- A reminder fires when its key is due and is newer than the key last sent for the same entry/day (a different entry or day always counts as new; the same one only when `slot_start` is greater, so a clock moved backwards never repeats).
- The key is the slot's start **instant** (UTC epoch ms), not its index, because the reference point can move: the running entry's start can be edited (`work_time::update` keeps it running) and `longRunningHours` / `notRunningAfter` can change mid-day. With an index, moving the start later after slot 2 was sent would silence the reminder until slot 3 (three hours late); with an instant, the new threshold lies after `now`, so it is always newer than anything already sent. Moving the reference earlier gives no burst: the current slot fires only when its start is newer than the last one sent, otherwise the next full hour does.

## Critical Implementation Details

- **Timing & lifecycle:** read the facts inside `with_store`, then drop the store lock before sending any toast (`runtime.rs:5-9`: no lock across tray, notification or emit calls; the notification `show()` itself spawns async, but the rule stays). Update the in-memory "last sent" keys only for reminders actually handed to the toast helper. A suppressed (`notices_allowed() == false`) reminder still counts as sent, so nothing is replayed later, matching the health rule.
- **DST:** the "not running" anchor is a local time on today's date. Resolve it with the time zone: single instant as is, ambiguous → earlier instant, nonexistent (spring-forward gap) → the first instant after the gap. Never refuse it, unlike `rules::resolve_local`, which refuses user input in a gap.

## Phase 1: Reminder rules and settings (tauri-free)

### Overview

Everything that decides whether a reminder is due and what it says, with no `tauri` import and no clock reads, fully covered by unit and integration tests.

### Changes Required:

#### 1. Reminder settings loader

**File**: `src-tauri/src/settings.rs`

**Intent**: Read the top-level `reminders` block on its own, following `load_todo_limit` exactly (missing file → defaults, BOM strip, `version == 1`, absent or `null` block or field → default, `deny_unknown_fields`, no value echoed in errors). Document the block's shape in the function's doc comment, since there is no other settings documentation. The read/BOM/version boilerplate is copied, not shared: extracting a helper would refactor the shipped `todo` and `gitlab` loaders, which is out of scope for this change.

**Contract**:
- `pub struct ReminderSettings { pub long_running: Option<std::time::Duration /* or ms i64 */>, pub not_running_after: Option<NaiveTime> }`, where `None` means switched off.
- `pub fn load_reminder_settings(path: &Path) -> Result<ReminderSettings, ReminderSettingsError>` plus a private `parse_reminder_settings(text)` for tests.
- `ReminderSettingsError::SettingsInvalid`: a unit variant with no detail (a small `thiserror` enum, or reuse an existing value-free error if one fits without coupling to `TodoError`).
- JSON fields (camelCase):
  - `longRunningHours`: an integer `1..=24`, or `false`. Default 10.
  - `notRunningAfter`: a `"HH:MM"` string with 24-hour hours `00..=23` and minutes `00..=59`, zero-padded, or `false`. Default `"09:00"`.
  - `true`, other types, out-of-range values and unknown keys are `SettingsInvalid`.

#### 2. Facts query

**File**: `src-tauri/src/work_time/reminders.rs` (new; `mod reminders;` and tests wired in `work_time/mod.rs` the way `norm_tests.rs` is)

**Intent**: In one `store.read`, collect what the rules need: the running entry (id and true `start_ms`), today's local date, today's `DayKind`, and whether today has any counted work (a non-removed closed entry with a positive portion inside today's local bounds). Reuse `running_entry`, `in_day`/`day_bounds` and `report::day_kind`. Make the smallest visibility changes needed inside `work_time`.

**Contract**: `pub fn reminder_facts<Tz: TimeZone>(store: &Store, now: i64, tz: &Tz) -> Result<ReminderFacts, WorkTimeError>`; `pub struct ReminderFacts { pub running: Option<RunningEntry>, pub today: NaiveDate, pub day_kind: DayKind, pub worked_today: bool }`.

#### 3. Slot rules and Polish text

**File**: `src-tauri/src/work_time/reminders.rs`

**Intent**: A pure function decides which reminders are due now, given the facts, the settings, `now`, `tz` and the in-memory record of what was last sent. It returns at most one long-run and one not-running reminder with their Polish title and body, and the updated record. The slot rule is the one in "Implementation Approach".

**Contract**:
- `pub struct ReminderMemory { long: Option<(i64 /*entry id*/, i64 /*slot_start ms*/)>, idle: Option<(NaiveDate, i64 /*slot_start ms*/)> }` (`Default`).
- `pub fn due_reminders<Tz: TimeZone>(facts: &ReminderFacts, settings: &ReminderSettings, now: i64, tz: &Tz, memory: &ReminderMemory) -> (Vec<Reminder>, ReminderMemory)`.
- `pub struct Reminder { pub title: String, pub body: String }`.
- Text, titles prefixed with `health::APP_NAME` ("Nawigator dnia"):
  - Long run: title "Nawigator dnia — czy nadal pracujesz?"; body "Licznik czasu pracy biegnie od {start} ({h} h). Zatrzymaj go albo popraw wpis w „Czas pracy”."
    - `{start}` is "HH:MM" when the entry started today, "wczoraj HH:MM" when it started yesterday, otherwise "DD.MM HH:MM".
    - `{h}` is the whole hours elapsed.
  - Not running: title "Nawigator dnia — licznik nie biegnie"; body "Dziś jest dzień roboczy, a czas pracy nie jest liczony. Kliknij START albo oznacz dzień jako wolny."
- Not-running anchor resolution follows the DST rule in "Critical Implementation Details".

### Success Criteria:

#### Automated Verification:

- Settings parsing tests pass (`cargo test settings`):
  - defaults for a missing file, an absent or `null` block, and absent or `null` fields;
  - `false` turns each reminder off separately;
  - limits 1, 24 and "00:00"/"23:59" are accepted;
  - 0, 25, 1.5, `true`, "9:00", "24:00", "09:60", a wrong type, an unknown key and a wrong `version` give `SettingsInvalid`;
  - a BOM is stripped;
  - an invalid `todo` or `gitlab` block does not affect `reminders`.
- Reminder rule unit tests pass (`cargo test work_time::reminders`, Europe/Warsaw):
  - the worked example in "Desired End State", hour by hour;
  - the threshold boundary: 9:59:59 gives nothing, 10:00:00 fires;
  - no repeat within a slot; a new entry starts a fresh slot sequence;
  - an entry started yesterday is described as "wczoraj HH:MM";
  - not running: silent before 09:00, fires at 09:00, silent at 09:30, fires at 10:00;
  - silent on a weekend, on a day off, when today has counted work (including a midnight-spanning portion), and while running;
  - each reminder is silent when switched off;
  - an empty memory (restart) fires the current slot only once;
  - a jump of several hours (sleep) fires once for the current slot;
  - a clock moved backwards does not repeat;
  - the running entry's start edited later after slot 2 was sent: the reminder fires again when the new 10 h threshold is reached, not three hours later;
  - `notRunningAfter` moved from 09:00 to 10:30 after the 11:00 toast fires at 11:30; moved earlier, it sends no burst;
  - a DST day: the 2026-03-29 gap with `notRunningAfter` "02:30" anchors at 03:00, and on 2026-10-25 the long-run elapsed time counts the extra hour. Both DST days are Sundays, so these cases build `ReminderFacts` directly with `DayKind::Workday` to exercise the anchor rule.
- Facts query integration test passes (`cargo test --test work_time`) against a temp store: running entry with its true start, `worked_today` ignoring removed entries, and the day off reported as `DayKind::DayOff`.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/`.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase. Phase blocks use plain bullets — the corresponding `- [ ]` checkboxes for these items live in the `## Progress` section at the bottom of the plan.

---

## Phase 2: Reminder thread, shared toast and health source

### Overview

Wire the rules into the running app: a background thread that checks every 60 s and sends toasts, a shared gated toast helper, and a "Przypomnienia" health source for an invalid settings block.

### Changes Required:

#### 1. Shared toast helper

**File**: `src-tauri/src/runtime.rs`

**Intent**: Make the existing toast helper usable from the new thread without duplicating the `notices_allowed()` gate. Keep the health call sites behaving the same; only the log prefix becomes per-caller.

**Contract**: `pub(crate) fn toast(app: &AppHandle, area: &str, title: &str, body: &str)`, which logs `"{area}: toast failed: {e}"` on error. The health call sites pass `"health"`.

#### 2. Reminder thread

**File**: `src-tauri/src/work_reminders.rs` (new; `pub mod work_reminders;` in `lib.rs`)

**Intent**: A named thread `work-reminders` loops every 60 s (`REMINDER_INTERVAL` const) and does the following in each round:
- re-reads the settings block (a missing settings path means defaults);
- on `SettingsInvalid`, skips the round, because health surfaces it;
- reads the facts with `StoreHandle::with_store(Local::now().date_naive(), …)` using `Utc::now()` and `chrono::Local`;
- on a store error, skips the round and logs at `debug` level, as `reconcile_todo` does (`gitlab_refresh.rs:357-372`), because health already reports "Dane aplikacji";
- calls `due_reminders`, sends each due reminder through `runtime::toast` after the store lock is released, and keeps `ReminderMemory` in the loop's local state.

A failure to spawn the thread, or a panic in the loop, is logged and does not stop the app. It is not surfaced in the "Przypomnienia" source (accepted risk; see "What We're NOT Doing").

**Contract**: `pub fn start(app: &AppHandle, settings_path: Option<PathBuf>)`, called in `lib.rs` `setup` after `runtime::start`. `with_store` opens the store lazily for whichever caller comes first (`store/mod.rs:205-230`: open, daily backup, migrations), so an early reminder round may open it itself. That is harmless, and a failed open just skips the round. `settings_path` is moved into `gitlab_refresh::start` (`lib.rs:186`), so clone it for this call before that line.

#### 3. "Przypomnienia" health source

**Files**: `src-tauri/src/health/mod.rs`, `src-tauri/src/health/probe.rs`, `src-tauri/src/runtime.rs`

**Intent**: Surface an invalid `reminders` block in the tray and health panel instead of letting reminders vanish silently:
- add `SourceId::Reminders` (key `"reminders"`);
- add `probe::reminders_status(settings_path: &Path) -> SourceStatus`, labelled "Przypomnienia": Ok when the block parses (including absent or a missing file), otherwise `Error { kind: "SettingsInvalid", message }`;
- the Polish message names the accepted shape without echoing values: "Blok „reminders” w settings.json jest nieprawidłowy: longRunningHours to liczba godzin 1–24 albo false, notRunningAfter to godzina „GG:MM” albo false.";
- append the source in `probe_round` after the store and GitLab sources when a settings path exists, and omit it when the config dir is unresolved, since the Settings source already reports that.

**Contract**:
- A new `SourceId` variant and key.
- One more source in `HealthSnapshot.sources`.
- No TS type change, because `id` is a string in `src/health/types.ts`.
- The existing transition, tooltip and notice rules apply unchanged.

### Success Criteria:

#### Automated Verification:

- Health tests pass (`cargo test health`):
  - `reminders_status` is Ok for a missing file, an absent block and a valid block;
  - it gives `SettingsInvalid` with the fixed message (no user values) for an invalid block;
  - an Ok → error transition yields a notice line "Przypomnienia: nieprawidłowe ustawienia".
- The full suite passes: `cargo test` in `src-tauri/`.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass in `src-tauri/`.
- `npm run lint`, `npm run format:check` and `npm run build` pass (no TS changes expected; guards against accidental drift).

#### Manual Verification:

- In `npm run tauri dev`, set `"reminders": { "longRunningHours": 1 }` and START, then hide the window to the tray. Within about a minute after 1 h, a "czy nadal pracujesz?" toast appears; no second one comes within that hour, and another comes after the next full hour. (Under `tauri dev` toasts show as "Windows PowerShell".)
- On a workday with no entries today, set `notRunningAfter` to a minute just past now: a "licznik nie biegnie" toast appears within about a minute. After START, no further "nie biegnie" toasts that day. Marking today as "Dzień wolny" silences it.
- Setting `"longRunningHours": 0` turns the tray red with "Przypomnienia: nieprawidłowe ustawienia", the health panel shows the fixed message, and no reminders are sent. Restoring a valid value turns it green on the next round.
- Setting both values to `false` sends no reminders, and the "Przypomnienia" source stays Ok.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase. Phase blocks use plain bullets — the corresponding `- [ ]` checkboxes for these items live in the `## Progress` section at the bottom of the plan.

---

## Testing Strategy

### Unit Tests:

- `settings.rs` tests for the `reminders` block, next to the `todo` tests (`settings.rs:450-545`).
- `work_time` reminder rules in a new `work_time/reminders_tests.rs` (wired like `norm_tests.rs`), with `Europe/Warsaw`, fixed instants built by the existing `t(local)` style helpers, and a fresh or carried `ReminderMemory` to cover the slot rules, midnight, DST, restart and sleep.
- `health` tests for `reminders_status` and for the notice line.

### Integration Tests:

- `src-tauri/tests/work_time.rs`: `reminder_facts` against a temp store (`tests/common::test_store_dir()`), including a removed entry, a midnight-spanning entry and a day off. Invented data only.

### Manual Testing Steps:

1. Short long-run threshold, START, hide to the tray, and wait for the first and second hourly toasts.
2. `notRunningAfter` just past now on a workday with no entries: toast; START silences it; "Dzień wolny" silences it.
3. Invalid block: tray error and panel message; fix it and see recovery.
4. Both values `false`: silence, with the source Ok.

## Performance Considerations

One settings read and one small indexed store read every 60 s. This is negligible next to the 30 s health round, and the store lock is held only for the read.

## Migration Notes

None. There is no schema change. Existing `settings.json` files without a `reminders` block get the defaults, so reminders start working on upgrade with 10 h / 09:00.

## References

- Roadmap: `context/foundation/roadmap.md`, S-10 and Open Roadmap Question #3 (defaults).
- PRD: `context/foundation/prd.md`, FR-018 and the "Norma" rules.
- Prior plans: `context/archive/2026-11-23-work-timer/plan.md`, `context/archive/2026-11-23-work-norm-report/plan.md` (reuse of `is_workday`/`days_off`), `context/archive/2026-11-20-source-health-status/plan.md` (toast and transition rules).
- Similar implementation: `src-tauri/src/settings.rs:137-187` (`load_todo_limit`), `src-tauri/src/runtime.rs:263-282` (thread loop), `src-tauri/src/gitlab_refresh.rs:168-200`.

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Reminder rules and settings (tauri-free)

#### Automated

- [x] 1.1 Settings parsing tests pass for the reminders block
- [x] 1.2 Reminder rule unit tests pass (slots, midnight, DST, day off, restart, sleep)
- [x] 1.3 Facts query integration test passes
- [x] 1.4 cargo fmt --check and cargo clippy pass

### Phase 2: Reminder thread, shared toast and health source

#### Automated

- [ ] 2.1 Health tests pass for the Przypomnienia source
- [ ] 2.2 Full cargo test suite passes
- [ ] 2.3 cargo fmt --check and cargo clippy pass
- [ ] 2.4 npm run lint, format:check and build pass

#### Manual

- [ ] 2.5 Long-run reminder fires at threshold and hourly while hidden in tray
- [ ] 2.6 Not-running reminder fires after the set hour; START and day off silence it
- [ ] 2.7 Invalid reminders block shows Przypomnienia error in tray and panel, then recovers
- [ ] 2.8 Both reminders switched off send nothing
