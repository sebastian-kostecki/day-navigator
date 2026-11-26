---
date: 2026-11-26T12:23:12+01:00
researcher: Claude (10x-new-research, non-interactive subagent run)
git_commit: 455cbeda5879b666ed52021fe1b2bc2a02748807
branch: worktree-agent-acfc681f9238d2e85
repository: day-navigator
topic: "S-03 tray-timer-control: START/STOP in the tray menu and today's work time in the tray tooltip"
tags: [research, codebase, tray, runtime, work-time, event-log, worktime-provider, health]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (10x-new-research, non-interactive subagent run)
---

# Research: START/STOP from the tray and today's work time in the tray tooltip (S-03)

**Date**: 2026-11-26T12:23:12+01:00
**Researcher**: Claude (10x-new-research, non-interactive subagent run)
**Git Commit**: 455cbeda5879b666ed52021fe1b2bc2a02748807
**Branch**: worktree-agent-acfc681f9238d2e85
**Repository**: day-navigator

## Research Question

Roadmap `context/foundation/roadmap.md` slice S-03 (milestone M-3), change-id `tray-timer-control`, GitHub issue #23, PRD refs MS-23 and FR-013. The outcome: the user can start and stop the work-time counter from the tray menu (a "START"/"STOP" item) and see today's work time in the tray tooltip next to the source state, within the Windows limit of 127 characters. Constraints:

- The tray uses the same counter state as the header. State lives in Rust; there is no second counter.
- START/STOP from the tray goes to the event log as it does today.
- The header updates when the tray changes the state, through the one existing timer subscription in `WorkTimerProvider`.

What the research must answer:

1. How the current tray works: menu, tooltip, icon choice, icon assets, and how the tooltip length is handled.
2. How the current work timer works: Rust state, the start/stop/status commands, START/STOP in the event log, how the frontend subscribes, and how the header's STOP confirmation works.
3. How the UI can learn of a change made from the tray, and how Rust can compute today's work time for the tooltip and keep it fresh.
4. Which AGENTS.md rules apply, and which parallel slices touch the same files.

### Decisions given with the task (recorded, not researched)

The user made these decisions before this run. They were passed in by the calling agent, since no user was reachable:

- **D1.** STOP from the tray needs **no confirmation**. It works at once and is logged in the event log.
- **D2.** The tray icon gets a **"counter running" variant**. A source error keeps priority in the icon's colour and meaning.
- **D3.** The slice stays anchored as **MS-23**. No new FRs are added to the PRD.
- **D4.** A later slice, S-07 end-of-day-summary, will show a Windows notice after a tray STOP. S-04 autostart will touch start-up. So the STOP path (and START, for symmetry) must be **one clear Rust function shared by the header command and the tray**, so that both slices can hook into it.

Defaults this run chose where the skill would have asked the user:

- The scope is the whole slice. No scope interview was held.
- No `lessons.md` exists under `context/foundation/` (checked), so no prior lessons applied.

## Summary

- **The counter's state already lives only in Rust.** It is a single row in `work_entries` with `end_ms IS NULL`, and a unique partial index guarantees at most one such row (`src-tauri/src/store/schema.rs:23-24`).
- **START/STOP already write the event in the same transaction.** `work_time::start` / `stop` (`src-tauri/src/work_time/mod.rs:259-294`) write the `workStarted` / `workStopped` event in the same `store.write` as the entry. A tray action that calls these same functions satisfies "no second counter" and "logged like today" with no new storage and no migration.
- **The UI already learns about changes through Tauri events, not polling.** `start_work` / `stop_work` (`src-tauri/src/work_time/commands.rs:68-87`) emit `work-time-changed` and `events-changed` right after the commit. `useWorkTimerSource` (`src/worktime/useWorkTimer.ts:85-122`) is the single `listen(WORK_TIME_CHANGED)` behind `WorkTimerProvider`. It refetches the status on that event, on `visibilitychange` and on `focus`.
  - If the tray path emits the same two events, the header, "Czas pracy" and the dashboard's event log update without any frontend change. The rule against a second subscription is kept.
- **The header's confirmation question already handles a change from elsewhere.** `WorkTimerHeader` (`src/worktime/WorkTimerHeader.tsx:21-24`) drops a pending "Zakończyć pracę?" when the counter changes elsewhere, so a tray STOP during an open question just closes the question.
- **The tray today is static except for the health snapshot.**
  - The menu has three items, "Otwórz", "Sprawdź teraz" and "Zakończ", built once (`src-tauri/src/runtime.rs:153-156`).
  - Only `update_tray` (`runtime.rs:206-216`) changes the tray. It is called from `publish` (`runtime.rs:228-237`) by the health check thread (every 30 s, or on wake) and by the watchdog thread.
  - The icon is chosen from `Overall` alone (`runtime.rs:139-145`).
  - The tooltip comes from the pure `health::tooltip` (`src-tauri/src/health/mod.rs:257-289`), capped at 127 UTF-16 units by `cap_utf16` (`health/mod.rs:19`, `:291-`). A proptest checks the cap (`health/mod.rs:723-737`).
  - A work-time line and a "running" icon variant must be composed with the health state in that single writer. Otherwise the health thread and the timer path overwrite each other.
- **Nothing in Rust returns today's total including the running entry.** `WorkTimerStatus` carries `today_closed_ms`, `running_from_ms` and `day_end_ms` (`work_time/mod.rs:42-56`), and the UI adds the live span (`src/worktime/total.ts:8-18`). `status()` also runs the month report (`work_time/mod.rs:188`).
  - For the tooltip, a small pure Rust function is needed: today's closed time plus the running span, capped at the day end and clamped at zero, as `total.ts` does. It can be built from `rules::portion` (`src-tauri/src/work_time/rules.rs:57-63`) over `in_day`.
- **There are hard constraints on threads.**
  - Tray menu callbacks run on the main thread (`runtime.rs:163-172`).
  - Store access must run on the blocking pool (AGENTS.md; `src-tauri/src/store/mod.rs:189-190`), because the health round holds the store lock during reopen and backup.
  - So the tray handler must hand the START/STOP work to `tauri::async_runtime` and not touch the store inline.
  - Tray and menu setters run inline when called on the main thread and block on a channel otherwise (`tauri-2.12.7/src/lib.rs:1112-1124`; `tauri-runtime-wry-2.12.5/src/lib.rs:259-274`). The existing rule applies: no lock held across tray, emit or toast calls (`runtime.rs:5-9`).
- **No change is needed to capabilities or CSP.** Rust→webview `emit` needs only the already granted `core:event:allow-listen` / `allow-unlisten` (`src-tauri/capabilities/default.json:7-8`). Tray control stays Rust-only, so no `core:tray:*` is added. Icons are compiled in with `tauri::include_image!`, not loaded by the webview.

## Detailed Findings

### 1. Current tray: menu, icon, tooltip

**Construction.** `runtime::build_tray` (`src-tauri/src/runtime.rs:149-185`) does the following:

- It registers `HealthState`.
- It builds `MenuItem`s with the ids `open`, `check` and `quit` (`:153-156`) into one `Menu`.
- It creates the tray with `TrayIconBuilder::with_id("health")` (`TRAY_ID`, `:30`), using the idle icon and the `Starting` tooltip (`:158-160`).
- It sets `show_menu_on_left_click(false)`. A left click opens the window (`:173-182`).
- Menu events are matched by id in `on_menu_event` (`:163-172`).
- It is called from `setup` in `src-tauri/src/lib.rs:165`. `HealthState` and the tray exist before the health thread starts (`lib.rs:166`).

Today no menu item is kept as a handle. Changing the item text later (START↔STOP) needs a stored `MenuItem` clone, either in managed state or inside `HealthState`. Tauri 2 menu items are `Send + Sync` (`tauri-2.12.7/src/menu/mod.rs:94-95`) and offer `set_text` / `set_enabled` (`tauri-2.12.7/src/menu/normal.rs:94`, `:105`).

**Icon selection.** `tray_image(overall)` (`runtime.rs:139-145`) maps:

| `Overall` | Icon file |
|---|---|
| `Ok` | `icons/tray/ok.png` |
| `Error` | `icons/tray/error.png` |
| `Starting`, `NotRunning` | `icons/tray/idle.png` |

The images are compiled in by `tauri::include_image!`, which needs RGBA PNGs at compile time.

**Icon assets and generation.**

- The sources are SVGs in `src-tauri/icons/source/`: `tray-ok.svg`, `tray-error.svg`, `tray-idle.svg` and `app-icon.svg`.
- The regeneration recipe is in the comment at `src-tauri/icons/source/app-icon.svg:5-7`: `icons/tray/<state>.png` is the `32x32.png` output of `npx tauri icon icons/source/tray-<state>.svg`.
- `src-tauri/build.rs` only adds `rerun-if-changed=icons` (per the archive search). Nothing is generated at build time.
- The states differ by colour and shape:
  - `tray-ok.svg` is an amber `#F5A524` ring and needle (`tray-ok.svg:12-15`);
  - `tray-error.svg` is the glyph in `#FF7B72` with a cut-out and a filled badge dot at the top right (`tray-error.svg:5-9`, `:18`).
- The earlier Pillow-generated PNGs (`context/archive/2026-11-20-source-health-status/make-tray-icons.py`) were replaced later. Per the archive search, commits 4c817e6 and be2f01a did this, and no archived plan covers the change.

A "counter running" variant (D2) therefore means:

- a new `icons/source/tray-running.svg`;
- a generated `icons/tray/running.png`;
- a third arm in `tray_image`, which then needs the running flag as an input.

Constraints on the design:

- The top-right badge and the `#FF7B72` colour already mean "error". The header's "REC" marker is drawn in `var(--danger)` (`src/worktime/WorkTimerHeader.css:24-30`), but the tray must not copy that red. D2 says the error state keeps the colour's meaning.
- A running variant should keep the "ok" amber and add a distinct shape mark, for example a dot or a play mark in another corner.
- The tray icon states are documented in the Konsola design-system artifact. Per the archive search: `context/archive/2026-11-24-design-system-artifact-sync/plan.md:31`, `sync-checklist.md:22,26-27`. A new state is a design-system sync item.
- The SVGs carry literal hex colours. The AGENTS.md "no literal colours" rule covers CSS outside `tokens.css`, not icon sources.

**Tooltip.**

- `health::tooltip(overall, sources)` (`src-tauri/src/health/mod.rs:257-289`) builds Polish text that starts with `APP_NAME` ("Nawigator dnia", `:21`). For example:
  - "Nawigator dnia — OK";
  - "… — OK (GitLab niedostępny)";
  - "… — błąd: Dane aplikacji i 1 więcej";
  - "… — sprawdzanie nie działa";
  - "… — uruchamianie…".
- It ends with `cap_utf16(text, TOOLTIP_MAX_UTF16)`, where `TOOLTIP_MAX_UTF16 = 127` (`:17-19`). The cap counts UTF-16 units, reserves one unit for "…", and never splits a character (`:291-`).
- Tests:
  - exact strings, `health/mod.rs:~690-712`;
  - truncation with an ellipsis, `:714-721`;
  - a proptest that every `Overall` stays within 127 and starts with "Nawigator dnia", `:723-737`.
- The underlying Windows limit: `szTip` holds 128 UTF-16 units including the terminator. Per the archive search, this is cited in `context/archive/2026-11-20-source-health-status/plan.md:43` and `tray-icon-0.25.1` `windows/mod.rs:239-244`.
- Data boundary from the same plan: no paths, reasons or messages in the tooltip (per the archive search, `source-health-status/plan.md:112,146`). A work time like "3:42" is not work data in the AGENTS.md sense; it is the user's own counter. No item titles may appear.

**Who writes the tray.**

- `update_tray(app, snap)` (`runtime.rs:206-216`) sets the icon and the tooltip from the `HealthSnapshot` and logs setter errors at warn level.
- It is called only from `publish` (`runtime.rs:228-237`). `publish` holds the `published` mutex, skips snapshots whose `seq` was already pushed, and then emits `health-changed`.
- `publish` is called by `check_round` (`runtime.rs:329`, every `CHECK_INTERVAL` = 30 s at `:31`, or right away on `HealthState.wake`, `:288-292`) and by `watchdog_loop` (`runtime.rs:396`).
- The locking rule is in the module comment (`runtime.rs:5-9`): the `snapshot` lock is never held across tray, notification or emit calls. `published` is the one lock held across them, and only the check and watchdog threads take it, never the main thread.

### 2. Current work timer: Rust state, commands, event log

**State model (Rust only).**

- A running counter is a row in `work_entries` with `end_ms IS NULL AND removed_at_ms IS NULL`, found by `running_entry` (`src-tauri/src/work_time/mod.rs:109-116`).
- At most one such row can exist: unique partial index `work_entries_one_running` (`src-tauri/src/store/schema.rs:23-24`, per the Rust research).
- The `work_time` domain has no clock and no `tauri` import. `now: i64` (UTC ms) and `tz: &Tz` are parameters (`work_time/mod.rs:1-9`, per the Rust research). Only `commands.rs` imports `tauri` (`work_time/commands.rs:1-2`).

**Domain functions.**

- `status(store, now, tz) -> WorkTimerStatus` (`work_time/mod.rs:166-202`) works out:
  - today's closed portion;
  - the running entry, even when it started before today (`:183-187`);
  - `running_from_ms = max(start, day_start)` (`:190`);
  - the day bounds;
  - the month deviation, by running the full `report::compute` (`:188`).
- `start(store, now, tz)` (`:257-276`) does all of this in one `store.write`:
  - refuses with `AlreadyRunning` when a running entry exists;
  - checks overlap;
  - INSERTs the entry;
  - calls `events::record_in(tx, local.date(), local, &NewEvent::WorkStarted)` (`:273`).
- `stop(store, now, tz)` (`:278-294`) does all of this in one `store.write`:
  - refuses with `NotRunning` or `EndNotAfterStart`;
  - UPDATEs `end_ms`;
  - calls `record_in(..., WorkStopped)` (`:291`).
- `is_running(store)` (`:244-247`).
- `WorkTimeError` derives `thiserror` + `Serialize` with `tag = "kind", content = "detail"` (`work_time/error.rs:10-35`, per the Rust research). Its variants include `AlreadyRunning`, `NotRunning`, `Store` and `Background`.

**Commands** (`src-tauri/src/work_time/commands.rs`, registered in `lib.rs:212-220`).

- `run(app, name, f)` (`commands.rs:19-43`) runs `f(store, Utc::now())` through `crate::blocking`, which is `spawn_blocking` (`lib.rs:35-46`). The store is reached through `state::<StoreHandle>().with_store(...)`.
  - It logs `Store` / `Background` failures at warn level, by kind only.
  - It logs refusals at debug level (`commands.rs:33-41`).
- `notify` (`commands.rs:45-49`) emits `work-time-changed` (`EVENT`, `:17`) with no payload.
- `start_work` (`commands.rs:70-79`) and `stop_work` (`commands.rs:81-87`):
  1. run the domain write;
  2. on success, call `notify(&app)` and `crate::events::commands::notify(&app)` (`events-changed`, `src-tauri/src/events/mod.rs:26`);
  3. only then call `work_timer_status`.
  - The comment at `:68-69` explains the order: a failed read can't hide a committed change from the listeners.
- This command body is the logic a tray action has to share (D4). Today it is inlined in the two `#[tauri::command]` functions, which take an `AppHandle`. A tray handler also has an `AppHandle`, so the body can move into a shared `pub(crate)` function for both:
  - `start_and_notify(app)` / `stop_and_notify(app)`, or one function with an action argument;
  - returning `Result<WorkTimerStatus, WorkTimeError>`, or `()` plus a separate status read.

**Event log.**

- `NewEvent::WorkStarted` / `WorkStopped` are stored as kind `workStarted` / `workStopped` (`src-tauri/src/events/mod.rs:72-92`, `:223-230`, per the Rust research).
- `record_in` (`events/mod.rs:253-282`) first deletes the rows of other days, then INSERTs with a local `%Y-%m-%dT%H:%M:%S` stamp.
- The schema CHECK lists the allowed kinds (migration v10, `src-tauri/src/store/schema.rs:226-232`, per the archive search). There is **no** "origin" (header vs tray) column.
  - Recording the origin would need a new migration and an AGENTS.md wording change. No source asks for it. Issue #23 says "trafiają do dziennika zdarzeń tak jak dziś".
  - So the tray path should produce the same rows: it should call the same domain functions.
- The integration test `start_and_stop_store_events_in_local_time_and_refusals_store_none` (`src-tauri/tests/work_time.rs:212-240`, per the Rust research) already covers the domain path the tray would reuse.

### 3. Frontend subscription and the header confirmation

**One subscription.**

- `useWorkTimerSource` (`src/worktime/useWorkTimer.ts:43-185`) is called only by `WorkTimerProvider` (`src/worktime/WorkTimerProvider.tsx:5-12`).
- It subscribes to `WORK_TIME_CHANGED` (`src/worktime/types.ts:2`) **before** the first load (`useWorkTimer.ts:82-122`). It also refetches on `visibilitychange` and `focus` (`:89-94`) and at the day end (`:127-133`).
- It ticks every 1 s only while running (`:139-154`).
- A latest-request-wins guard drops stale responses (`:52-65`).
- Other listeners of `work-time-changed`: `useWorkEntries.ts:61` and `useMonthReport.ts:62`, per the Rust research. These are data views, not timer subscriptions. `events-changed` is listened to by `src/events/useEventLog.ts:79`.

**Consequence.** If a tray START/STOP emits `work-time-changed` and `events-changed` exactly as `start_work` / `stop_work` do, all of the following refetch through their existing listeners:

- the header;
- "Czas pracy" (entries, month report);
- the dashboard's event log.

No new frontend subscription, command or TS type is needed for the header to follow the tray. While the window is hidden, the webview stays mounted (`lib.rs:175-185`: the close button only hides it), and `focus` / `visibilitychange` refetch on show as a second safety net.

**Header confirmation (`src/worktime/WorkTimerHeader.tsx`).**

- START/STOP first asks "Rozpocząć pracę?" / "Zakończyć pracę?" (`:81-99`), with a 6 s timeout, Escape to cancel, and a 400 ms double-click guard (`:8-11`, `:26-46`). The reason is in the comment at `:17`: a stray click must not open or close a work entry.
- A question that no longer fits the state is dropped (`:21-24`). If the tray stops the counter while "Zakończyć pracę?" is open, `running` turns false on refetch and the question disappears. No change is needed here.
- A user who then clicks "TAK, STOP" on a stale view gets `NotRunning`. `act` in `useWorkTimer.ts:156-176` already turns `AlreadyRunning` / `NotRunning` into a plain refresh.
- D1 (no confirmation for a tray STOP) fits the tray's own interaction. A menu item is a deliberate open-then-click, unlike a stray click on a large header button. The header keeps its confirmation unchanged.

### 4. Learning of tray changes, today's total, and freshness

**Threads.**

- `on_menu_event` (`runtime.rs:163-172`) runs on the main thread.
- AGENTS.md: store commands are `async` and call `with_store` inside `spawn_blocking`, "never from a sync command: the health round holds the store lock during reopen and backup". The same reason is given in `src-tauri/src/store/mod.rs:189-190`.
- So the tray's "work" item must do this:
  1. Spawn: `tauri::async_runtime::spawn(async move { shared_stop(&app).await })`, or `spawn_blocking`.
  2. Inside the spawn, run the store write, the two emits and the tray refresh.
- Calling `set_text` / `set_icon` / `set_tooltip` from a pool thread works. They post a task to the main thread and block on its reply (`tauri-2.12.7/src/lib.rs:1112-1124`). On the main thread they run inline (`tauri-runtime-wry-2.12.5/src/lib.rs:259-269`), so there is no self-deadlock either way.
- The locking rule from `runtime.rs:5-9` still applies: do not hold the store lock or the health `snapshot` lock across these calls. The `published` mutex may be taken by the pool thread; only the main thread must never take it.

**Today's work time in Rust.**

- No function returns the live total today.
- `WorkTimerStatus` documents the formula `today_closed_ms + (now - running_from_ms)` (`work_time/mod.rs:42`). The UI adds the day-end cap and the clamp at zero (`src/worktime/total.ts:3-18`).
- `rules::portion(start, end, now, day_start, day_end)` (`rules.rs:57-63`) already counts a running entry up to `min(now, day_end)` and clamps at 0.
- A small read function like `today_total(store, now, tz) -> { running: bool, today_ms: i64 }` could:
  - sum `portion` over `in_day(...)` (`work_time/mod.rs:119-141`);
  - check `running_entry`;
  - return the result.
- It would be cheaper than `status()`, which also computes the month report (`mod.rs:188`). It would be testable with the existing fixed-instant fixtures (`work_time/tests.rs`, "now" = 2026-11-23 12:00 Warsaw, per the Rust research).
- `reminder_facts` (`work_time/reminders.rs:51-73`) is the existing read of this kind for a background loop.
- The midnight rule is already covered by `in_day` / `portion`: a running entry from yesterday counts only from midnight (test `running_entry_from_before_midnight_counts_only_from_midnight`, `work_time/tests.rs:469`, per the Rust research).
- A tooltip formatter like `Praca: H:MM` should be pure and live next to `health::tooltip` or in `work_time`. It needs no `tauri`.

**Freshness: existing loops.**

| Loop | Cadence | Reads the store? | Writes the tray? |
|---|---|---|---|
| Health check thread (`runtime.rs:275-294`) | 30 s, or on `wake` | yes (`probe_round` → `health::store_probe`, `:262`) | yes, through `publish` |
| Watchdog (`runtime.rs:357-404`) | 15 s; publishes only on stall/clear | no | yes, through `publish` |
| `work-reminders` (`src-tauri/src/work_reminders.rs:40-46`) | 60 s | yes (`reminder_facts`) | no (only toasts; it imports no `Emitter`, `:14`) |

The health check round is the natural place to read the work-time line for the tooltip. It already runs every 30 s on its own thread, already holds a store read, and is already the tray's writer. A 30 s cadence keeps an `H:MM` display at most about 30 s late.

**Options for keeping one tray writer.** These are for `/10x-plan` to choose; the research does not pick one.

- **A. Wake the health round after START/STOP.**
  - Both the shared START/STOP function (command and tray) call `HealthState.wake.send(())` after the commit.
  - `check_round` reads the work-time facts with the store probe, keeps them next to the snapshot (not in the serialised `HealthSnapshot`, so the TS mirror of `health-changed` stays unchanged), bumps `seq`, and `publish` → `update_tray` sets the icon, the tooltip and the START/STOP item text together.
  - The watchdog's `publish` reuses the last stored work facts.
  - Pros: there is a single writer and the seq guard is unchanged.
  - Cons: each START/STOP triggers a full probe round (settings read, store check). If the check thread is stalled, the tray label lags. The tooltip then already says "sprawdzanie nie działa".
- **B. A direct tray refresh from the START/STOP path.**
  - The shared function reads the new facts and calls a `refresh_tray(app)` that composes the last health snapshot with the new work facts under the `published` mutex.
  - The seq guard must be changed, because `publish` returns early when `snap.seq <= last` (`runtime.rs:231-233`), and a work-only change has no new health seq.
  - Pros: an immediate update, even when the health thread hangs.
  - Cons: a second caller of the tray setters that the locking comment has to cover.

In both options, `update_tray` must take `(health snapshot, work facts)` so that one composition decides the icon and the tooltip.

**Icon priority (D2).** A sketch of the mapping:

| Health state | Counter | Icon |
|---|---|---|
| `Error` | any | `error` |
| `Starting` / `NotRunning` | any | `idle` |
| `Ok` | running | `running` |
| `Ok` | stopped | `ok` |

Whether `idle` + running needs its own variant is a plan detail. The safe default is that health states other than `Ok` win.

**Menu item and the stale-label race.**

- The visible label (START or STOP) can be older than the store, for example right after a header action, before the tray refresh.
- A handler that reads the store and toggles could therefore **stop** a counter the user wanted to **start**.
- A race-safe choice is that the handler acts on the label's intent, and treats `AlreadyRunning` / `NotRunning` as "state already changed elsewhere": refresh the tray and do nothing else, exactly like `useWorkTimer.ts:164-167`. The intent can be carried by:
  - two items (`work-start`, `work-stop`), with only the fitting one enabled; or
  - one item plus an intent flag stored with the label.
- Before the first status read (the store opens lazily in the first health round, `lib.rs:158-161`), or when the status read fails:
  - the item should be disabled with explicit text, for example "Czas pracy niedostępny";
  - the tooltip should say the time is unavailable, not "Praca: 0:00". AGENTS.md: never show an empty or zero state in place of a failed source.
  - A store failure is already reported by the "Dane aplikacji" source, so the icon shows `error`.

**Failure feedback from the tray.** A refused or failed tray action has no window to show an error in. The existing channel is `runtime::toast(app, area, title, body)` (`runtime.rs:239-247`), gated by `health::notices_allowed()` (`health/mod.rs:379-384`, currently always `true`). The toast text must be value-free, like all toasts and logs: kind only, no paths.

### 5. AGENTS.md constraints that apply

- **Store:** only through `StoreHandle`, `async` / `spawn_blocking`, and never from the main-thread tray callback (`store/mod.rs:189-190`).
- **No migration needed:** START/STOP reuse the existing entry and event rows. A migration would be needed only if a tray "origin" were recorded. That is not requested and would conflict with "tak jak dziś".
- **Contracts:**
  - If `WorkTimerStatus` or the error enum do not change, there is nothing to mirror in TS.
  - If `HealthSnapshot` gained a field, `src/health` types would need the same commit. Option A avoids this by keeping the work facts outside the serialised snapshot.
- **Tauri 2 / capabilities:**
  - Events emitted from Rust need no new permission (`capabilities/default.json:7-8`).
  - Do not add `core:tray:*` or `core:event:allow-emit`. Per the archive search, the capability description (`default.json:4`) records that the webview must not forge `health-changed` or drive the tray (source: `context/archive/2026-11-20-source-health-status/reviews/impl-review-round1.md:104-112`).
- **CSP:** no new webview resource. Icons are compiled in with `include_image!`, and events go through IPC (per the archive search, `context/archive/2026-11-25-tauri-config-hardening/research.md:63-65`).
- **Styles/tokens:** no frontend change is expected. If one is made, the header CSS must keep using tokens.
- **Data boundary:** the tooltip and toast show only the app name, the source state and the user's own counter value. No GitLab or GitHub titles, hostnames or paths.
- **Tests:** invented data only. The pure rules (today's total, tooltip composition within 127 UTF-16 units, icon priority) are unit-testable without `tauri`, following the existing tooltip tests and proptest (`health/mod.rs:714-737`).

### 6. Parallel slices and shared files

| Slice | Likely overlap with S-03 | Risk |
|---|---|---|
| S-01 public-holidays-norm | `work_time/{norm,report,rules}.rs`, settings, the "Czas pracy" UI. Not `runtime.rs`, the commands' START/STOP or capabilities. | Low |
| S-04 windows-autostart | `lib.rs` setup (plugin, hidden start), possibly tray start-up in `runtime.rs`, `Cargo.toml`, `settings.rs`. The roadmap says to plan S-03 and S-04 one after the other (`roadmap.md:69`, `:145`). | High |
| S-06 work-entry-note | A new migration in `store/schema.rs`; `work_time/mod.rs` / `commands.rs` (entry DTO, add/update); `src/worktime/WorkTimeEntries.tsx`, `types.ts`, `api.ts`. | Medium (the same `commands.rs`; S-03 adds no migration, so there is no number clash) |
| S-07 end-of-day-summary | The STOP path: the shared START/STOP function S-03 creates (D4), `runtime.rs` if a tray STOP shows a toast or the window, and possibly the header. The roadmap's open question: "what happens on STOP from the tray (S-03)?" (`roadmap.md:184`). | High (semantic coupling; mitigated by D4) |

These overlaps are inferred from each slice's outcome and the code layout. No plan exists yet for S-01, S-04, S-06 or S-07.

- Per the archive search, no branch exists for any of these slices (`git branch -a`).
- Other worktrees hold untracked `change.md` files only:
  - `.claude/worktrees/agent-ab918e4ae3c0dbddc/context/changes/public-holidays-norm/change.md`;
  - `.claude/worktrees/agent-a52bf1d753e9204cc/context/changes/work-entry-note/change.md`.

## Code References

- `src-tauri/src/runtime.rs:5-9`: locking rule for the tray, emit and toast calls.
- `src-tauri/src/runtime.rs:30-36`: `TRAY_ID`, `CHECK_INTERVAL` (30 s), `WATCHDOG_INTERVAL`, `health-changed`.
- `src-tauri/src/runtime.rs:139-145`: `tray_image(overall)`, three `include_image!` PNGs.
- `src-tauri/src/runtime.rs:149-185`: `build_tray`, the menu (open / check / quit), the main-thread `on_menu_event`.
- `src-tauri/src/runtime.rs:206-237`: `update_tray`, `emit_snapshot`, `publish` (seq guard under `published`).
- `src-tauri/src/runtime.rs:239-247`: `toast`.
- `src-tauri/src/runtime.rs:275-355`: `check_loop` / `check_round` (store probe, publish, event-log transitions).
- `src-tauri/src/health/mod.rs:17-21`: `TOOLTIP_MAX_UTF16 = 127`, `APP_NAME`.
- `src-tauri/src/health/mod.rs:257-300`: `tooltip`, `cap_utf16`.
- `src-tauri/src/health/mod.rs:379-384`: `notices_allowed`.
- `src-tauri/src/health/mod.rs:714-737`: tooltip truncation test and proptest.
- `src-tauri/src/work_time/mod.rs:42-56`: `WorkTimerStatus`.
- `src-tauri/src/work_time/mod.rs:109-141`: `running_entry`, `in_day`.
- `src-tauri/src/work_time/mod.rs:166-202`: `status` (includes `report::compute`).
- `src-tauri/src/work_time/mod.rs:257-294`: `start` / `stop` with the event in the same write.
- `src-tauri/src/work_time/rules.rs:57-63`: `portion`.
- `src-tauri/src/work_time/reminders.rs:51-73`: `reminder_facts` (a background read pattern).
- `src-tauri/src/work_time/commands.rs:17-49`: `EVENT`, `run`, `notify`.
- `src-tauri/src/work_time/commands.rs:68-87`: `start_work` / `stop_work`, the logic to share (D4).
- `src-tauri/src/events/mod.rs:26`: `CHANGED_EVENT = "events-changed"`.
- `src-tauri/src/lib.rs:35-70`: `blocking`, `on_store`.
- `src-tauri/src/lib.rs:155-174`: setup order (store handle, tray, health, reminders, refresh threads).
- `src-tauri/src/store/mod.rs:189-190`, `:241-250`: the store-lock rule, `with_store`.
- `src-tauri/src/work_reminders.rs:26-82`: the 60 s reminder loop.
- `src-tauri/capabilities/default.json:7-8`: `core:event:allow-listen` / `allow-unlisten` only (no emit, no tray).
- `src-tauri/icons/source/tray-{ok,error,idle}.svg`, `src-tauri/icons/tray/{ok,error,idle}.png`: tray assets.
- `src-tauri/icons/source/app-icon.svg:5-7`: the regeneration recipe.
- `src/worktime/useWorkTimer.ts:82-122`: the single `listen(WORK_TIME_CHANGED)`, focus and visibility refetch.
- `src/worktime/useWorkTimer.ts:156-176`: `act`, `AlreadyRunning` / `NotRunning` → refresh.
- `src/worktime/WorkTimerProvider.tsx:5-12`: the provider.
- `src/worktime/WorkTimerHeader.tsx:8-46`: the confirmation and the stale-question drop.
- `src/worktime/total.ts:8-18`: live total (day-end cap, zero clamp).
- `src/events/useEventLog.ts:79`: the `events-changed` listener.
- Tauri internals: `tauri-2.12.7/src/lib.rs:1112-1124` (`run_main_thread!`), `tauri-runtime-wry-2.12.5/src/lib.rs:259-274` (inline on the main thread), `tauri-2.12.7/src/menu/normal.rs:94,105` (`set_text`, `set_enabled`), `tauri-2.12.7/src/tray/mod.rs:513,537` (`set_icon`, `set_tooltip`).

## Architecture Insights

- **Domain/side-effect split:** `health/` and `work_time/` (except `commands.rs`) are free of `tauri` and take `now`/`tz` as inputs. `runtime.rs` and `commands.rs` own the side effects. S-03 fits this split:
  - pure: today's total, tooltip composition, icon choice;
  - side effects (`runtime.rs` / `commands.rs`): menu handling, spawn, emits, tray setters.
- **Events, not polling:** every Rust write site emits a payload-less `*-changed` event, and each UI hook refetches on it. Tray-initiated changes need the same emits and nothing new in the UI.
- **The tray has one writer,** keyed by the health `seq`. Any new tray content (work line, running icon, menu label) must be composed in that writer or must extend its guard. Two independent writers would overwrite each other's tooltip.
- **Refusal as refresh:** the UI already treats `AlreadyRunning` / `NotRunning` as "someone else changed it". The tray path should do the same.

## Historical Context (from prior changes)

- `context/archive/2026-11-20-source-health-status/plan.md:42-43,149,203,281` covers the tray icon states, the 127-unit tooltip cap, the pure tooltip with its proptest, and the rule against blocking setters under a lock. All of these are still the case in code.
- `context/archive/2026-11-20-source-health-status/reviews/impl-review-round1.md:104-112`: `core:default` was replaced by explicit event permissions so the webview cannot drive the tray. This is still the case (`default.json:7-8`).
- `context/archive/2026-11-23-work-timer/plan.md:41`: a tray START/STOP item was deferred. `plan.md:54,161,187` set up `work-time-changed` after each write. Supported: the event is in code.
- `context/archive/2026-11-23-work-timer/reviews/impl-review.md:97-107`: emit right after the commit, then read the status. Supported (`commands.rs:68-87`).
- `context/archive/2026-11-23-timer-reminders/plan.md:39`: tray START/STOP and a tooltip line were deferred. That is this slice.
- `context/archive/2026-11-24-shared-work-timer-state/plan.md:5-26,65,91-97`: one `WorkTimerProvider` and one `listen`. That plan made no Rust or event changes (`:33`). Supported: the code matches.
- `context/archive/2026-11-24-event-log/plan.md:30-31,55,79,204`: today-only pruning, START/STOP in the same write, the kinds CHECK, `events-changed` at every write site. Supported. A header-vs-tray origin was never discussed (`plan.md:269` only checks header START/STOP).
- `context/archive/2026-11-24-design-system-artifact-sync/plan.md:24-25,31`: the header confirmation and the three tray icon states are documented in the Konsola artifact. A fourth tray state needs a sync.
- The header confirmation and the SVG tray icons came from commits 4c817e6 and be2f01a, which have no archived plan (per the archive search). The only recorded reason is the code comment at `WorkTimerHeader.tsx:17`.

The anchors in this section were reported by a read-only sub-agent that searched the archive. The code claims they support were re-read in this run; the plan line numbers were not all re-opened.

## Related Research

- `context/archive/2026-11-20-source-health-status/`: tray and tooltip.
- `context/archive/2026-11-23-work-timer/`: timer and events.
- `context/archive/2026-11-24-shared-work-timer-state/`: one subscription.
- `context/archive/2026-11-24-event-log/`: START/STOP rows.
- `context/archive/2026-11-25-tauri-config-hardening/research.md`: CSP and IPC.

## Open Questions

None of these blocks planning. Each has a safe default that `/10x-plan` can confirm.

1. **Tray writer design: option A (wake the health round) or B (direct refresh with a changed seq guard).**
   - Default: A, because it keeps one writer and the existing guard.
   - Check during the plan that a full probe round per START/STOP is acceptable.
2. **Menu shape: one item whose text changes, or two items (START, STOP) with only the fitting one enabled.**
   - Either way, the handler must act on the shown intent and treat `AlreadyRunning` / `NotRunning` as a refresh, never as a blind toggle.
   - Default: one item, "START" / "STOP", with its intent stored alongside it.
3. **Tooltip layout: one line with a separator (for example "Nawigator dnia — OK · Praca: 3:42") or two lines with `\n`.**
   - Windows tray tooltips usually show a line break, but this was **not verified** on this machine. Check it manually in a built app.
   - Whichever layout is chosen, the work line must survive truncation. Cap the health part to the space left after the work line, and extend the existing proptest to the composed string.
4. **The running icon's shape and colour.** It must not use the error red or the top-right badge (D2). A design-system (Konsola artifact) sync of the fourth tray state follows. The exact glyph is a design choice for the plan.
5. **Whether the header START/STOP should also refresh the tray at once.** It should, so that the tray label and icon follow the header. This comes free if the shared function from D4 triggers the tray refresh for both callers.
6. **The text of a failure toast** for a refused or failed tray action (value-free), and the text of the disabled item while the store is unavailable or still loading. Polish copy is for the plan to settle.
