# Panel „Przypomnienia” w „Czas pracy” — Plan Brief

> Full plan: `context/changes/worktime-reminders-panel/plan.md`

## What & Why

Roadmap S-06, PRD FR-021, issue #3. The work-time reminders already fire ("czy nadal pracujesz?" after N hours, "licznik nie biegnie" after a set time on a workday), but the UI never shows their settings. The user can't see the thresholds, whether each reminder is on, or that a broken `reminders` block has paused them.

## Starting Point

Rust reads the `reminders` block of `settings.json` with `load_reminder_settings` (defaults 10 h and 09:00, where `false` = off, and an invalid block = `SettingsInvalid`). The 60 s `work-reminders` thread uses it and skips a round when the block is invalid. No command exposes these values. After S-05, "Czas pracy" has the days table in the main column and only "Wpisy" in the side column.

## Desired End State

Under "Wpisy" in the side column there is a "Przypomnienia" panel with two rows: "Licznik biegnie dłużej niż `10:00`" and "Brak START-u w dzień roboczy po `09:00`". Each row has a `WŁ.`/`WYŁ.` pill, a `domyślnie` note when the value is not in the file, and a one-line rule explanation. A "Ustawienia" button opens `settings.json`. An invalid block shows an error state saying that reminders are paused, with a retry. After editing the file, returning to the app window shows the new values.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Placement | Side column, under "Wpisy" | A short settings panel fits the narrow column and keeps the long main column (days table) unchanged. |
| Default vs set | Show `domyślnie` when the field is absent or `null` | Explains where 10 h / 09:00 come from when the file says nothing. |
| Origin source | Same parse pass as the thread's loader (`load_reminder_settings_detailed`) | One parser, so the panel can never disagree with what the thread uses. |
| Settings link | "Ustawienia" button reusing `open_settings_file` | Editing stays in the file, so one click to the file; no new Rust. |
| Refresh | Mount + window focus/visibility + retry | Picks up an edit when the user comes back from the editor; no polling, no watcher. |
| Invalid block | `ErrorState`, "przypomnienia wstrzymane" | Matches the thread (it skips rounds) and the rule never to show a failure as data. |
| Missing config dir | Defaults, as the thread does | The panel shows what actually runs. |

## Scope

**In scope:**
- Origin-aware reminder parsing, the `reminder_settings` command, and a serializable error.
- The TS mirror and the `api.ts` wrapper, in the same commit as the Rust types.
- The `useReminderSettings` hook, the `RemindersPanel` component and the side-column wiring.

**Out of scope:**
- Editing settings, creating `settings.json`, a file watcher or polling.
- Changes to the reminder rules, the thread, the toasts or the health source.
- `notices_allowed()`, OS notification permission, and a "next reminder at" time.
- `src/health/` and "Stan źródeł" (owned by S-07).

## Architecture / Approach

`parse_reminder_settings` returns `(ReminderSettings, ReminderOrigin)`. `load_reminder_settings` drops the origin for the thread, and the new `load_reminder_settings_detailed` keeps it. The async `reminder_settings` command (in `work_reminders.rs`, via `blocking`) maps the result to `ReminderThresholds { longRunning: { hours | null, fromFile }, notRunningAfter: { "HH:MM" | null, fromFile } }`. The UI hook loads it, and the panel only formats and displays it (`formatHm`, `StatusPill`, `ErrorState`).

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Reminder-settings command and TS contract | Command, origin-aware parser, Rust tests, TS mirror + api | Changing the thread loader's behaviour (guarded by an agreement test over existing fixtures) |
| 2. „Przypomnienia” panel | Hook, panel, side-column wiring, manual checks | Layout at 800 px; showing an invalid block as "off" instead of an error |

**Prerequisites:** S-05 merged (it is, in `develop`). Konsola components and tokens are in place.
**Estimated effort:** ~1 session, 2 phases.

## Open Risks & Assumptions

- No screen mockup of the panel exists in the design-system artifact. The row wording and pill texts (`WŁ.`/`WYŁ.`) are this plan's choice and are checked by hand.
- The rest of a broken `settings.json` (another block invalid) does not affect the panel, because the loader reads only `reminders`. That matches the thread.

## Success Criteria (Summary)

- The user can see both thresholds, their on/off state and whether each one is the default, without opening the file.
- A broken `reminders` block reads as "paused", never as "off" or as an empty panel.
- All gates are green (lint, format, build, rustfmt, clippy, `cargo test`), and the manual rows are confirmed on the real machine.
