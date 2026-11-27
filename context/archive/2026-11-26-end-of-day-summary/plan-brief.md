# End-of-day summary after STOP — Plan Brief

> Full plan: `context/changes/end-of-day-summary/plan.md`
> Research: `context/changes/end-of-day-summary/research.md`

## What & Why

After STOP the user should see how the day went — time worked today, the month's deviation, what was ticked off in "Do zrobienia" and what carries over to tomorrow — without being asked to decide anything (roadmap S-07, MS-19, issue #19). It closes the working day without adding a ritual.

## Starting Point

Header and tray STOP share one function, `stop_timer`; the tray's success branch shows nothing yet. `work_time::status` already gives today's time and the month's deviation, `todo::list` already gives today's ticked and open items, and toasts already have one helper and a pure-builder convention. The header cannot tell today whether its own STOP succeeded.

## Desired End State

Header STOP opens a calm "Podsumowanie dnia" panel between the header and the view: two figures, two counts, "Zrobione dziś" and "Przechodzi na jutro" lists, and one "Zamknij". Tray STOP shows one toast, "Nawigator dnia — koniec pracy" / "Dziś: 7:45. Odchylenie miesiąca: −0:15." Nothing new is stored.

## Key Decisions Made

| Decision        | Choice                                                               | Why (1 sentence)                                                  | Source               |
| --------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------- | -------------------- |
| Trigger         | After STOP only                                                      | No fixed-hour summary was wanted.                                 | User                 |
| Header STOP     | Dismissible in-window summary, no question                           | Closing must need nothing else; carry-over stays automatic.       | User                 |
| Tray STOP       | Toast with today's time and deviation, no titles                     | Short and private, follows the toast conventions.                 | User                 |
| Data            | Rust over `work_time::status` + `todo::list`, no store, no migration | One source of truth; S-02's norm change flows through.            | User / Research      |
| PRD             | MS-19, no new FR; delete "zamknięcie dnia, " and add one line        | The PRD must not claim it is unbuilt; minimal hotspot edit.       | User                 |
| Panel form      | `Panel` in `App.tsx`, figures in `role="status"`, no overlay         | Shared components only; no new tokens or component.               | Plan (agent default) |
| Reopen          | No persistence; START or "Zamknij" closes; next STOP shows again     | Simplest honest behaviour; no stored "seen" state.                | Plan (agent default) |
| Panel freshness | Re-reads on `todo-changed` while open                                | A tick in "Do zrobienia" below the panel must not contradict it.  | Plan review (F3)     |
| Header signal   | `stop()` resolves `true` → `onStopped` prop → app state              | No second timer subscription, no new context.                     | Plan (agent default) |
| Command         | New `day_summary` command in a new module                            | `stop_work` and `WorkTimerStatus` stay unchanged for S-02/S-08.   | Plan (agent default) |
| Tray read fails | Fixed toast without figures                                          | Never a wrong figure, never a silent skip after a committed STOP. | Plan (agent default) |
| After midnight  | Summary is about the new day                                         | Same behaviour as the header; documented.                         | Plan (agent default) |
| Event log       | Not read                                                             | It misses unticks and "done" in "Moje zadania".                   | Research             |

## Scope

**In scope:** `src-tauri/src/day_summary/` (types, rules, error, toast texts, command, tests); two lines in `lib.rs`, a few in `runtime.rs`; `src/day-summary/` (types, api, hook, panel); small edits in `useWorkTimer.ts`, `WorkTimerHeader.tsx`, `App.tsx`; CHANGELOG, README, PRD sentence.

**Out of scope:** scheduled summary, any forced decision, persistence, modal/overlay, reopen button, toast click handling, titles in toasts or logs, changes to `norm.rs`/`report.rs`/schema, event-log reads, roadmap edits, new FRs.

## Architecture / Approach

Pure Rust (`compose`, `today_ms`, `stop_notice`, `format_signed_hm`) sits in `tauri`-free files and is unit-tested; `day_summary/commands.rs` is the only `tauri` file: the `day_summary` command (settings read, then `status` + `todo::list` in one `with_store`) and the tray helper `stop_notice(app)` (reads `status` only, lock released before `toast`). The window calls the command when the panel mounts after a successful header STOP (`stop()` resolves `true` as soon as `stop_work` resolved, even if a racing refresh already applied the newer status), and again on each `todo-changed` while the panel is open.

## Phases at a Glance

| Phase                 | What it delivers                                               | Key risk                                                           |
| --------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------ |
| 1. Domain and command | `DaySummary`, rules, error, `day_summary` command, TS mirror   | Hard-coding a norm instead of passing the deviation through        |
| 2. Tray notification  | Pure toast texts, signed H:MM, toast on tray STOP              | Holding the store lock across the toast; wording drift from the UI |
| 3. Summary panel      | `stop()` → `true`, `onStopped`, `DaySummaryPanel` in `App.tsx` | A second timer subscription or a stale panel after START           |
| 4. Docs and PRD       | CHANGELOG, README, PRD MS-19 line                              | Merge conflict in the PRD sentence shared with S-04/S-08           |

**Prerequisites:** none beyond develop at `d34904a`; parallel slices S-02, S-04, S-05, S-08 touch nearby files.
**Estimated effort:** ~4 short implementation sessions, one per phase.

## Open Risks & Assumptions

- S-02 changes the norm rule; the tests compare against `work_time::status` instead of literal hours so they survive it. If S-02 changes the signature of `work_time::status` or `work_time_settings`, the two call sites in `day_summary/` need a one-line update at merge.
- `lib.rs` and `CHANGELOG.md` are shared with every parallel slice: the handler goes mid-list (after `own_task_default_space`), not at the end; a CHANGELOG `[Unreleased]` conflict is expected and kept by union.
- Gate hygiene: new files are untracked until committed, so the checks use `git grep --untracked` and `git status --porcelain`; run them from the worktree root after `npm install`.
- S-04 and S-08 edit the same PRD sentence; the merge keeps each slice's own removal.
- Focus Assist can swallow the toast; `show()` reports success before Windows shows anything (known from timer-reminders).
- The deviation counts today's full norm (existing header semantics), labelled "do dziś".

## Success Criteria (Summary)

- After a header STOP the user sees the day's figures and both lists and can dismiss them with one click.
- After a tray STOP the user gets one toast with the same two figures as the header.
- All gates green per phase; norm, report and schema unchanged across the slice.
