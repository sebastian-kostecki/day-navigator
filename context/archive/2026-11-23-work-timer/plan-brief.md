# Work Timer — Plan Brief

> Full plan: `context/changes/work-timer/plan.md`

## What & Why

Roadmap slice S-08 (FR-013, FR-017): the user starts and stops work time with START/STOP from a counter always visible at the top of the window, and can correct, add or remove entries. The PRD guardrail "Czas pracy nie ginie" means a running counter must survive closing the app, a Windows restart and a crash. The new time record starts in December, so this slice should not ship late.

## Starting Point

The F-01 store (SQLite, WAL, migrations, crash-test harness) exists but has no tables, and no command uses it yet. The window has no header or navigation: `App.tsx` stacks the vault Tasks screen and the Health panel. Closing the window hides the app to the tray, and only "Zakończ" quits.

## Desired End State

A fixed header shows "Dziś H:MM:SS" (today's total, ticking while running) and one START/STOP button, plus a switch between the Zadania, Czas pracy and Stan źródeł views. A started entry keeps running through window close, "Zakończ", a process kill and a Windows restart. The "Czas pracy" view lists a chosen day's entries and lets the user add, edit and remove them. Every refusal is shown in Polish and saves nothing.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| What the header shows | Today's total (all of today's portions + the running one) | Answers "how much today" at a glance, and S-09 reuses the same day rule. |
| Quit / restart with the counter running | The entry stays running; downtime counts until STOP and correction | Matches the PRD ("the counter survives a restart"); forgotten-STOP reminders are S-10. |
| Overlapping entries | Refused with a message naming the other entry | Sums never count a minute twice, so the norm report stays trustworthy. |
| Removing an entry | Allowed with inline confirmation; row kept as soft-removed | Fixes mistakes while keeping "time is never lost". |
| Entry across midnight | Split at local midnight for day totals; still one row | Exact day and month sums, also across month ends and DST changes. |
| Where corrections live | Build the AGENTS.md shell now: fixed header + view switch + "Czas pracy" view | The slice needs a screen now; S-01 adds its view to the same shell. |
| 2026-12-01 start | No date gate | Works whether the slice ships before or after 1 Dec; late entries are added by hand. |
| Storage of times | UTC epoch ms; local times resolved in Rust (`Local` in prod, `Europe/Warsaw` in tests) | Durations stay exact over DST, and the time zone rule lives in Rust, not the UI. |
| Durability mechanism | Commit at START/STOP; no heartbeat, no shutdown hook | The app has no reliable exit hook, and a committed start instant loses nothing. |

## Scope

**In scope:**
- Migration 1 `work_entries` (start, end or running, soft removal), with a single-running-entry index
- A tauri-free `work_time` module: start/stop/add/edit/remove, overlap/range/future rules, midnight split, `WorkTimeError`, `is_running` for S-06
- Six async commands, the `work-time-changed` event, and a generic `blocking` helper
- The app shell (view state, sticky header), the header counter, and the "Czas pracy" view
- Unit tests (including DST), reopen tests, and a crash test on the existing harness

**Out of scope:**
- Reminders (S-10), month report and norm (S-09), heartbeat/"last seen"
- A tray START/STOP item, restoring removed entries, hard deletion, merging overlaps
- A 2026-12-01 gate, seconds in corrections, a router or date library, the GitLab view (S-01)

## Architecture / Approach

The UI calls typed `api.ts` functions. Async commands take `now` and today's date and run `StoreHandle::with_store` on the blocking pool. Each mutation is one IMMEDIATE `Store::write` that reads its neighbours, validates and writes, so a refusal changes nothing. After a write, the command emits `work-time-changed`, and both the header hook and the entries hook refetch. The header ticks locally (`todayClosedMs + now − runningFromMs`). It refetches when the window becomes visible and when the local day ends, because the app lives in the tray for days.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Work-time domain and store | Migration 1, rules, error type, unit + reopen + crash tests | Getting the midnight/DST and overlap rules exactly right |
| 2. Commands and event | Generic `blocking`, 6 commands, event, TS mirrors | Changing `blocking` must not alter vault task errors |
| 3. App shell and header counter | View switch, sticky header with today's total and START/STOP | Stale total after midnight or after days hidden in the tray |
| 4. "Czas pracy" view | Day list with add/edit/remove and Polish refusals | Local-time inputs vs Rust resolution (DST gap, ambiguous hour) |

**Prerequisites:** F-01 app-data-store (done). Coordinate the shell with S-01 (gitlab-assigned-list), which is not planned yet.
**Estimated effort:** ~3–4 sessions across 4 phases.

## Open Risks & Assumptions

- S-01 runs in parallel and the roadmap gives it the header and navigation. Whichever slice lands second must adopt the other's shell instead of building a second one.
- A forgotten STOP before a shutdown produces a long entry until the user corrects it. This is accepted until S-10's reminders.
- `chrono-tz` is a dev-only dependency and is assumed to pass `cargo audit`; if it doesn't, the DST tests use a hand-built offset table.
- WebView2 may throttle timers in a hidden window. The visibility-change refetch covers that.

## Success Criteria (Summary)

- START, then close, "Zakończ", kill or restart Windows: the counter is still running with the same start, and today's total is right after midnight.
- The user can fix a forgotten START/STOP or a missing day in the "Czas pracy" view. Overlaps, impossible ranges and future times are refused with a clear Polish message.
- `cargo test` proves the rules (including midnight and DST) and that a hard kill never loses or half-applies an entry.
