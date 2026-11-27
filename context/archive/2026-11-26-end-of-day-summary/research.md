---
date: 2026-11-26T14:14:28+01:00
researcher: Claude (Opus 5.5), non-interactive research run
git_commit: d34904a4dc9f9400c8b386cc935a845b7ed6762b
branch: slice/end-of-day-summary
repository: day-navigator
topic: "End-of-day summary after STOP (roadmap S-07, MS-19, issue #19)"
tags: [research, work-time, todo, events, tray, notification, ui]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (Opus 5.5)
---

# Research: End-of-day summary after STOP

**Date**: 2026-11-26T14:14:28+01:00
**Researcher**: Claude (Opus 5.5), non-interactive run (no user reachable; fixed decisions from `change.md` applied)
**Git Commit**: d34904a4dc9f9400c8b386cc935a845b7ed6762b (develop merge of M-3 wave 1)
**Branch**: slice/end-of-day-summary
**Repository**: day-navigator

All paths are relative to the worktree root. Line numbers are at commit `d34904a`.

## Research Question

What does the codebase offer, and where would it hook in, for S-07: after STOP the user sees a summary of the day (time worked today, the month's deviation, what was ticked off in "Do zrobienia", what carries over to tomorrow), with no forced decision? STOP in the window shows a dismissible in-window summary; STOP from the tray shows a short Windows notification with today's time and the month's deviation (fixed wording, no titles). Data comes from existing sources, computed in Rust, no new store or migration.

## Summary

- **Hook point.** Both STOPs run one function, `work_time::commands::stop_timer` (`src-tauri/src/work_time/commands.rs:148-156`), whose doc comment names the place for this slice: "for every STOP" in `stop_timer`, "tray STOP only" in the tray caller. Header STOP = command `stop_work` (`commands.rs:166-170`) → `stop_timer` → `work_timer_status`. Tray STOP = `run_tray_action` (`src-tauri/src/runtime.rs:204-216`) → `stop_timer`; on success it shows nothing today (tray-timer-control decision A11, "S-07 adds the end-of-day notice later", `context/archive/2026-11-26-tray-timer-control/plan.md:44,405`). The tray knows it is the tray because `run_tray_action` is the tray-only caller; no `origin` parameter exists or is needed (`plan.md:45`).
- **Time and deviation: reuse `work_time::status`.** `status(store, now, tz, settings)` (`src-tauri/src/work_time/mod.rs:179-214`) returns `WorkTimerStatus` with `today_closed_ms` and `month_deviation_ms` (`mod.rs:52-64`), the latter from `report::compute` → `norm::figures` (`mod.rs:202,211`; `report.rs:122-180`; `norm.rs:113-145`). The 8 h norm lives only in `norm.rs:14` (`DAILY_NORM_MS`) and is consumed through `figures`; S-02 will change that rule, so the summary must take its figures from `status`/`report::compute`, never multiply 8 h itself. `status` needs `WorkTimeSettings` (holiday calendar), read through `run_with_settings` with `Policy::Lenient` (`commands.rs:71-78,117-122`); `stop_timer` itself uses `run` without settings (`commands.rs:56-68,152`).
- **"Do zrobienia": reuse `todo::list`.** `todo::list(store, today, now, limit)` (`src-tauri/src/todo/mod.rs:267-290`) is one write: roll-over, then today's visible list. Items with `ticked == true` are "ticked off today" (user tick or source close, told apart by `closed_in_source`); items with `ticked == false` carry over automatically (open entries are never closed by roll-over, `roll_over_in`, `mod.rs:119-137`). The `limit` argument does not change `items`. The todo store, not the event log, is the reliable source for ticks (see Detailed Findings).
- **Event log.** `events::list_today(store, today)` (`src-tauri/src/events/mod.rs:389-428`) is a read-only Rust function over today's rows. It does not log unticks or "done in Moje zadania", so it over- and under-counts ticks; use it only for optional facts such as the times of today's START/STOP rows. `stop` records `WorkStopped` in the same write as the entry end (`work_time/mod.rs:295-309`).
- **Notification convention.** One helper, `runtime::toast(app, area, title, body)` (`runtime.rs:353-361`), gated by `health::notices_allowed()` (`health/mod.rs:381-386`, always `true` today), errors logged as a warning only. Texts are fixed Polish strings built by pure, `tauri`-free functions returning `Notice { title, body }` (`health/mod.rs:145-149`), titled `"{APP_NAME} — …"` with `APP_NAME = "Nawigator dnia"` (`health/mod.rs:21`); the tray's failure toast `tray_view::failure_notice` (`src-tauri/src/tray_view.rs:124-155`) is the closest precedent. Plugin `tauri-plugin-notification` is Rust-only (no npm package, no capability).
- **UI.** No dialog/modal/overlay component exists in `src/components/`; the shared building blocks are `Notice` (tones calm/info/warn/error, one action button, `role="status"` unless error), `Panel`, `StatStrip`, `ItemRow`, `Button` + `Icon "close"`. tokens.css has no z-index, shadow or scrim tokens. The header STOP confirmation is inline in `src/worktime/WorkTimerHeader.tsx:41-46,81-99`; `stop()` from `useWorkTimer` returns `Promise<void>` and swallows the result inside `act` (`src/worktime/useWorkTimer.ts:156-179`), so the header does not learn whether its STOP succeeded.
- **PRD.** MS-19 is defined only in the roadmap (`context/foundation/roadmap.md:32`); the PRD lists "zamknięcie dnia" among "Rozważone i świadomie NIE wykluczone … żadne FR ich nie obejmuje" (`context/foundation/prd.md:233-234`). The plan should reword that line (built as MS-19, no new FR), as S-01 did for MS-25 (`context/archive/2026-11-26-public-holidays-norm/plan.md:565-597,668`).

## Detailed Findings

### 1. STOP paths, events and how the frontend learns of STOP

- `stop_timer(app)` (`src-tauri/src/work_time/commands.rs:148-156`): `run(app, "stop", |store, now| super::stop(store, now, &Local))`, then `notify(app)` (emits `work-time-changed` and wakes the tray, `commands.rs:108-114`), then `crate::events::commands::notify(app)` (`events-changed`). Returns `()`. Nothing is emitted on failure.
- Header: `#[tauri::command] stop_work` (`commands.rs:166-170`) = `stop_timer` then `work_timer_status` (`commands.rs:117-122`), returning `WorkTimerStatus` (with `today_closed_ms` and `month_deviation_ms`). The comment at `commands.rs:158-159` says: notify as soon as the write commits, then read.
- Tray: `run_tray_action(app, action)` (`src-tauri/src/runtime.rs:204-216`). `Err` → `tray_view::failure_notice` toast (`None` for `AlreadyRunning`/`NotRunning`) and `refresh_tray`. `Ok` → nothing (the tray was already refreshed by `notify`).
- Domain write: `work_time::stop` (`src-tauri/src/work_time/mod.rs:295-309`) sets `end_ms` on the running entry and records `NewEvent::WorkStopped` with `today = local.date()` in the same `store.write`.
- Frontend: one subscription to the timer status, in `useWorkTimerSource` (`src/worktime/useWorkTimer.ts`), shared by `WorkTimerProvider` (`src/worktime/WorkTimerProvider.tsx:5-12`) as `{timer, nowMs, busy, actionError, start, stop, retry}`. `act` (`useWorkTimer.ts:156-177`) applies the returned status; on `AlreadyRunning`/`NotRunning` it refreshes; other errors go to `actionError`. `stop = () => act("stop", stopWork)` (`:179`) resolves to `void` in all cases. A tray STOP reaches the window only as a `work-time-changed` event → refetch; the window cannot tell header from tray except by the fact that its own `stop()` call was the trigger.
- `WorkTimerHeader.confirm` (`src/worktime/WorkTimerHeader.tsx:41-46`) calls `void stop()`; the inline confirmation group "Zakończyć pracę?" [TAK, STOP] / [Anuluj] is at `:81-99`, with a 6 s give-up (`:9`) and a 400 ms click guard (`:11`).
- `App.tsx:61-81`: `<WorkTimerProvider>` → `<AppHeader>` with `<WorkTimerHeader />` (`:71`) → `<GitlabListViewProvider>` with the active view (`:73-80`). An app-level summary can mount inside `WorkTimerProvider` between the header and the view, or inside `WorkTimerHeader`.

### 2. Today's total, month report, norm and deviation (reuse, no hard-coded 8 h)

- `work_time::status` (`mod.rs:179-214`): today's closed time (`portion` per entry within `day_bounds(today)`), running entry, and `report::compute(c, today, now, tz, settings)` (`:202`) → `month_deviation_ms = month.figures.deviation_ms.unwrap_or(0)` (`:211`).
- `report::compute` (`src-tauri/src/work_time/report.rs:122-180`) builds the month from entries, manual days off and the holiday calendar (`settings.holidays`), and calls `norm::figures(&workdays, today, phase, worked_ms)`.
- `norm::figures` (`src-tauri/src/work_time/norm.rs:113-145`): for the current month `deviation = worked_ms - norm_to_date`, where `norm_to_date` counts workdays `<= today` in full (`:137-142`). So a STOP mid-afternoon after 7 h on a workday gives a deviation that already includes today's full norm (inference from the formula; this is the existing header semantics, which the summary should show unchanged).
- `DAILY_NORM_MS = 8 * 3_600_000` (`norm.rs:14`) is used at `norm.rs:57,120,139` only (inspected file). S-02 (configurable-daily-norm) will replace this rule; any summary that reads `status` / `report::compute` follows it automatically.
- `work_time::today::today_work` (`src-tauri/src/work_time/today.rs:21-40`) is the tray's cheap read (today total + running, no month report) — it lacks the deviation, so the tray notice needs `status` (or `report::compute`) instead.
- Settings: `status` needs `WorkTimeSettings`; commands read it through `run_with_settings(app, name, Policy::Lenient, f)` (`commands.rs:71-78`), which reads `settings.json` before taking the store lock (`commands.rs:80-106`).

### 3. "Do zrobienia" model, ticks and carry-over

(From the todo/event-log sub-investigation; decisive lines re-read: `todo/mod.rs:267-290`, `todo/commands.rs:80-114`, `lib.rs:92-107`.)

- Table `todo_items`, current shape migration v8 (`src-tauri/src/store/schema.rs:151-207`); a tick is a timestamp (`ticked_at`, `ticked_by` = `user`/`source`) for GitLab/GitHub entries (`todo/mod.rs:517-528`), and `own_tasks.done_at` for own entries (`own_tasks.rs:314-331`). Latest migration is v11 (`schema.rs:248`); this slice needs none.
- Types: `TodoList { day, limit, limit_error, open_count, items }`, `TodoItem { id, picked_on, carried_over, ticked, picked_at, entry }` (`todo/mod.rs:41-61`), `TodoEntry::{Own{task_id, space, title}, Gitlab{item_kind, reference, title, web_url, closed_in_source}, Github{…}}` (`:63-89`). TS mirrors in `src/todo/types.ts`.
- Visibility: an entry is on day D's list when live and either open or ticked on D (`visible`, `todo/mod.rs:248-261`); `carried_over = picked_on != day` (`:205`).
- Carry-over: `roll_over_in(tx, today)` (`todo/mod.rs:123-137`) closes live entries ticked on any other day; open entries stay and appear the next day with `carried_over = true`. It runs at the start of every todo write, `list` included, and in `set_own_task_done` (`lib.rs:92-107`). Decision record: `context/archive/2026-11-23-todo-list/plan.md:55,57,377`.
- Limit: `DEFAULT_TODO_LIMIT = 5` (`todo/mod.rs:39`), `settings.json` `todo.limit`; gates adding only.
- Commands: `list_todo` etc. go through `todo::commands::run` (`todo/commands.rs:70-114`), computing `today = Local::now().naive_local().date()` and calling `super::list` inside `with_store`. Error type `TodoError` (`src-tauri/src/todo/error.rs:12-26`) differs from `WorkTimeError` (`work_time/error.rs:12-37`); both carry `Store(StoreError)`, so a summary command needs either its own error enum or a mapping.
- "Ticked off today" source choice:
  - **Todo store (reliable):** `todo::list(...)` returns exactly what the user sees in "Do zrobienia": ticked rows = done today (including closed in source), open rows = carry over.
  - **Event log (unreliable for ticks):** unticks are not logged (`context/archive/2026-11-24-event-log/plan.md:47`), so tick→untick leaves a stale "Ticked"; "done" in "Moje zadania" (`set_own_task_done`) is not logged, so it under-counts; source closes are `ClosedInSource`, not `Ticked`; the read is capped at 200 rows (`events/mod.rs:389-428`).
- Edge cases for any source: a STOP after midnight runs against the new `today`, whose first write rolls over yesterday's ticks and prunes yesterday's events (`events/mod.rs:253-262`), so "ticked today" is near empty then; several STOPs per day each give the cumulative day; `todo::list` is a write, so it must run inside `with_store`, after the STOP write, never from a sync command (AGENTS.md store rule).

### 4. Today's event log

- `NewEvent` (`src-tauri/src/events/mod.rs:72-92`): `Assigned`, `Ticked(Subject)`, `Removed(Subject)`, `ClosedInSource(Subject)`, `WorkStarted`, `WorkStopped`, `SourceError`, `SourceRecovered`. `Subject { source, reference: Option, title }` (`:62-69`). Table `events`, migration v10 (`schema.rs:221-242`).
- `record_in(tx, today, now, &event)` (`events/mod.rs:253-282`) prunes rows of earlier days and inserts.
- `list_today(store, today) -> Result<EventLog, EventError>` (`events/mod.rs:389-428`): read-only, newest first, at most 200 rows + `truncated`; callable from Rust (used in `todo/tests.rs:931-932`). UI: `src/todo/EventLogPanel.tsx`, mounted in `TodoView`.
- Useful for the summary only optionally (e.g. first START / last STOP time today). Not needed for time worked (entries are the source) or ticks (todo store is the source).

### 5. Notification conventions

(From the notifications/UI sub-investigation; `run_tray_action`, `failure_notice`, `format_hm` re-read.)

- Plugin: `tauri-plugin-notification = "2.8"` (`src-tauri/Cargo.toml:47`), registered at `src-tauri/src/lib.rs:145`; no npm package and no capability entry (`src-tauri/capabilities/default.json:6-13`), matching the AGENTS.md rule for Rust-only plugins.
- Helper: `toast(app, area, title, body)` (`runtime.rs:353-361`): returns early when `!health::notices_allowed()`, calls `notification().builder().title().body().show()`, logs `"{area}: toast failed: {e}"` on error. Untested by design; the texts are tested through pure builders.
- Senders today (all fixed Polish text, titles `"Nawigator dnia — …"`): source-health transitions (`health/mod.rs:217-243`, sent `runtime.rs:459-461`), stalled check loop (`runtime.rs:44,528`), new GitLab assignments with `reference — title` bodies (`src-tauri/src/new_assignments/notice.rs:38-61`, sent `src-tauri/src/gitlab_refresh.rs:836-845`), timer reminders (`src-tauri/src/work_time/reminders.rs:127-134,160-164`, sent `src-tauri/src/work_reminders.rs:86-88`), failed tray START/STOP (`tray_view.rs:124-155`).
- Rules from archived plans: texts in Polish and built in Rust (`context/archive/2026-11-23-timer-reminders/plan.md:13,106-110`); never hold the store lock across a toast (`timer-reminders/plan.md:62`); logs carry counts/kinds only (`context/archive/2026-11-23-new-assignment-notify/plan.md:20,25,47`); no click handling on desktop toasts (`new-assignment-notify/plan.md:40,45`); Focus Assist can swallow toasts and `show()` reports success before Windows shows anything (`timer-reminders/plan-brief.md:81`).
- Formatting: Rust has `tray_view::format_hm(ms)` = unsigned, floored `H:MM` (`tray_view.rs:88-92`), and no signed formatter. TS `formatHm(ms, signed)` in `src/components/time.ts:79-89` uses `+H:MM` / `−H:MM` (U+2212), rounded toward zero, plain `0:00` under a minute; `src/worktime/format.ts:27` `formatSigned` follows the same rule. A Rust signed formatter for the tray notice should mirror that rule so the tray and the window show the same value.
- Privacy: the fixed decision says the tray notice has no item titles; the app log stays at kinds and counts (todo `plan.md:187`; event-log `plan.md:32`). User-typed work-entry notes never go into logs or the event log (`schema.rs:243-245`; work-entry-note archive).

### 6. Shared UI and design system

- `src/components/` (barrel `index.ts`): `AppHeader`, `Button` (primary/secondary/quiet/dashed; icon/iconOnly), `Chip`, `DataTable`/`Row`/`Cell`, `ErrorState`, `Field`, `Flag`, `HoursBar`, `Icon` (includes `close`), `ItemRow` (source chip, ref, flags, meta, optional checkbox), `LabelChip`, `MultiSelect`, `Notice` (`src/components/Notice/Notice.tsx`, component at `:35`; each component has its own folder), `Panel` (title, meta, footer, tone), `Segmented`, `SourceChip`, `SourceState`, `StatStrip`, `StatusPill`, `Tabs`; helpers `plural`, `storeError.ts`, `useNow`, time formatters.
- No `Dialog`/`Modal`, no `<dialog>`, `role="dialog"` or `position: fixed` in `src/` (sub-investigation grep); only `AppHeader` (z-index 10) and `MultiSelect` (z-index 5) stack. tokens.css (`src/styles/tokens.css:52-123`) has colour, spacing, radius and size tokens, but no z-index, shadow or scrim tokens; an overlay would need new tokens (literal colours are banned outside tokens.css).
- Existing "today + deviation" tiles: `src/todo/summary.ts:9-50` (`Dziś`, `Odchylenie` with `formatHm(deviationMs, true)`, tone warn when negative), rendered as `StatStrip` at `src/todo/TodoView.tsx:190`. Ticked rows in "Do zrobienia" use `ItemRow` (`TodoView.tsx:285-308`).
- Design-system rules: `Notice` is the in-view banner, `SourceState`/`ErrorState` are whole-source states (`context/archive/2026-11-24-design-system-artifact-sync/plan.md:155`); calm messages keep `role="status"` (`context/archive/2026-11-23-konsola-design-system/plan.md:27`). A new component needs a card in the design-system artifact (AGENTS.md), which earlier slices left as a manual follow-up for the user (tray-timer-control A13).

### 7. PRD wording

- `context/foundation/prd.md:233-234`: "### Rozważone i świadomie NIE wykluczone — … zamknięcie dnia, widok tygodnia, … autostart. Nie są non-goals, ale żadne FR ich nie obejmuje …". This is the line to reword in the plan (not now). S-04 (autostart) and S-08 (week view) name items in the same sentence, so the edit is a merge hotspot.
- MS-19 text lives in `context/foundation/roadmap.md:32`; roadmap S-07 at `:175-186` (PRD refs MS-19, FR-013, FR-014; risk: no second store of the day's state). FR-013 and FR-014 at `prd.md:153,155`.
- Precedent: S-01 kept MS-25 with no new FR and edited only the relevant PRD clause (`context/archive/2026-11-26-public-holidays-norm/plan.md:565-597,668`).

### 8. Merge risk with parallel slices

At this commit every parallel slice branch (`slice/configurable-daily-norm`, `slice/week-summary`, `slice/month-report-copy`, `slice/windows-autostart`) points at `d34904a` with no diff against develop (`git diff --stat develop...<branch>` empty), so the risks below are predicted from the roadmap, not observed.

- **S-02 configurable-daily-norm:** changes `norm.rs` (`DAILY_NORM_MS`, `figures`), `WorkTimeSettings`/`settings.rs`, likely `report.rs`. Mitigation: call `work_time::status` (or `report::compute`) and never touch `norm.rs`; keep the summary DTO's deviation a pass-through of `figures.deviation_ms`.
- **S-08 week-summary:** adds week figures in `work_time` and the "Work time" view; likely touches `report.rs`/`mod.rs`, `src/worktime/`. Mitigation: no edits in `report.rs`; new Rust in a new module.
- **S-05 month-report-copy:** touches `src/worktime/` month report and maybe `report.rs`. Same mitigation.
- **S-04 windows-autostart:** touches `runtime.rs`, `lib.rs`, `settings.rs`, the PRD sentence at `prd.md:234`. Mitigation: in `runtime.rs` add only a few lines in `run_tray_action`'s `Ok` branch; register one command in `lib.rs`'s `generate_handler!`; reword only the "zamknięcie dnia" fragment of `prd.md:234`.
- Shared files this slice will likely touch: `work_time/commands.rs` (hook or new command), `runtime.rs` (tray `Ok` branch), `lib.rs` (handler list), `src/worktime/useWorkTimer.ts` / `WorkTimerHeader.tsx` (header trigger), `App.tsx` (mount), `CHANGELOG.md`, `README.md` (if it lists notices), `prd.md:234`. New logic belongs in new files (e.g. `src-tauri/src/day_summary/` or `work_time/day_summary.rs`, `src/day-summary/`), following tray-timer-control A12 (`plan.md:406`).

## Code References

- `src-tauri/src/work_time/commands.rs:148-156` — `stop_timer`, the one STOP path and named hook point
- `src-tauri/src/work_time/commands.rs:166-170` — `stop_work` header command
- `src-tauri/src/work_time/commands.rs:56-106` — `run`, `run_with_settings`, `execute` (blocking + `with_store`, settings before lock)
- `src-tauri/src/runtime.rs:204-216` — `run_tray_action`, tray-only caller, `Ok` branch empty
- `src-tauri/src/runtime.rs:353-361` — `toast` helper
- `src-tauri/src/tray_view.rs:88-92,124-155` — `format_hm`, `failure_notice` (pure notice precedent)
- `src-tauri/src/health/mod.rs:21,145-149,381-386` — `APP_NAME`, `Notice`, `notices_allowed`
- `src-tauri/src/work_time/mod.rs:52-64,179-214,295-309` — `WorkTimerStatus`, `status`, `stop`
- `src-tauri/src/work_time/report.rs:122-180` — `report::compute`
- `src-tauri/src/work_time/norm.rs:14,101-145` — norm constant, `NormFigures`, `figures`
- `src-tauri/src/work_time/today.rs:21-40` — tray's cheap today total
- `src-tauri/src/todo/mod.rs:39-89,123-137,248-290` — todo types, roll-over, visibility, `list`
- `src-tauri/src/todo/commands.rs:70-114` — todo command runner
- `src-tauri/src/events/mod.rs:72-92,253-282,389-428` — event kinds, `record_in`, `list_today`
- `src-tauri/src/lib.rs:53-71,92-107,145` — `on_store`, `set_own_task_done` roll-over, notification plugin
- `src/worktime/useWorkTimer.ts:156-184` — `act`, `stop` (returns void)
- `src/worktime/WorkTimerHeader.tsx:41-46,81-99` — inline STOP confirmation
- `src/worktime/WorkTimerProvider.tsx:5-12`, `src/App.tsx:61-81` — provider and mount points
- `src/components/time.ts:79-89` — signed `formatHm`
- `src/todo/summary.ts:9-50` — existing today/deviation tiles
- `context/foundation/prd.md:233-234` — "zamknięcie dnia" considered-not-excluded

## Architecture Insights

- Pure rules live in `tauri`-free files and are unit-tested; commands are thin async wrappers that run on the blocking pool and call `with_store` (AGENTS.md; `work_time/commands.rs:1-7`). A summary fits as a pure function over `(WorkTimerStatus or its figures, TodoList)` → `DaySummary`, plus a pure `stop_notice(today_ms, deviation_ms) -> Notice` for the tray, with a thin command/caller.
- One STOP path, one timer subscription: the summary must not add a second status subscription in the UI; a separate one-shot command call after STOP is a read, not a subscription.
- Toasts are sent after the store lock is released (`timer-reminders/plan.md:62`); `stop_timer` already returns before any toast could be sent from `run_tray_action`.
- Errors are tagged unions with value-free variants; logs carry kinds only.

## Historical Context (from prior changes)

- `context/archive/2026-11-26-tray-timer-control/plan.md:40-45,405-406` — no tray STOP confirmation; no toast on success ("S-07 adds the end-of-day notice later"); no origin parameter; pure rules in new files to keep shared-file edits small (all still hold at this commit).
- `context/archive/2026-11-26-public-holidays-norm/plan.md:565-597,668` — holidays enter the norm through `WorkTimeSettings`; PRD edited narrowly, MS-25 with no new FR (precedent for MS-19).
- `context/archive/2026-11-26-work-entry-note/plan-brief.md:22-29` — `work_entries.note` (v11) is user content; it stays out of logs and the event log; not needed by the summary.
- `context/archive/2026-11-24-event-log/plan.md:20-32,47` — logged kinds; untick, add, roll-over and own-task done are not logged.
- `context/archive/2026-11-23-todo-list/plan.md:55,57,187,377` — visibility and roll-over rules; no titles in logs.
- `context/archive/2026-11-23-new-assignment-notify/`, `context/archive/2026-11-23-timer-reminders/` — notification wording and privacy conventions (section 5).

## Related Research

- `context/archive/2026-11-26-tray-timer-control/` (plan), `context/archive/2026-11-26-public-holidays-norm/research.md`, `context/archive/2026-11-26-work-entry-note/research.md`.

## Open Questions (agent defaults suggested for `/10x-plan`)

1. **Header trigger.** How does the window know its own STOP succeeded? Suggested default: let `act`/`stop` in `useWorkTimer.ts` resolve to a boolean (or the status) and have `WorkTimerHeader.confirm` open the summary on success; then call a new one-shot command (e.g. `day_summary`) for the todo part. A tray STOP never opens the in-window summary (it gets the toast), which follows from the trigger being the header's own call.
2. **Command shape.** New command (e.g. `day_summary()`) vs. extending `stop_work`'s return type. Suggested default: a new async command in a new module that, in one `with_store`, calls `work_time::status` and `todo::list`, mapping both errors into a new tagged `DaySummaryError` (or reusing `WorkTimeError::Store`); `stop_work` and `WorkTimerStatus` stay unchanged (smaller diff against S-02/S-08).
3. **Where the summary appears.** Suggested default: a dismissible `Panel` (or `Notice` tone info with a "Zamknij" action) mounted inside `WorkTimerProvider` between `AppHeader` and the active view, `role="status"`, no overlay (no new z-index/scrim tokens). Content: `StatStrip` (Dziś, Odchylenie), "Zrobione dziś" list and "Przechodzi na jutro" list with titles, explicit empty messages, loading and `ErrorState` for a failed read.
4. **Reopen later today.** Suggested default: no persistence; the summary lives in React state until dismissed or the view reloads; a later STOP shows it again. Reopening from "Do zrobienia" is optional and can be left out.
5. **Tray notice wording.** Suggested default: title `Nawigator dnia — koniec pracy`, body e.g. `Dziś: H:MM. Odchylenie miesiąca: +H:MM.` using a new Rust signed formatter that mirrors TS `formatHm(ms, true)`; no titles or counts of items. If the status read after STOP fails, either skip the toast or send a fixed text without figures (plan decides; never a silent wrong figure).
6. **Deviation semantics.** The current-month deviation counts today's full norm (`norm.rs:137-142`); the summary should show the same value as the header, labelled as "do dziś". Confirm in the plan that this is intended.
7. **STOP after midnight.** `today` is then the new day: time worked shows only the part after midnight and the todo list has rolled over. Suggested default: accept and document (the header has the same behaviour).
8. **PRD and docs.** Reword `prd.md:234` to drop "zamknięcie dnia" from the not-built list (built as MS-19, no new FR); add a `CHANGELOG.md` `[Unreleased]` entry; Konsola design-system card as a manual follow-up if a new component is added.
