<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Dziennik zdarzeń na dashboardzie

- **Plan**: context/changes/event-log/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-24
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 2 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

Success Criteria: every automated row re-run on HEAD fdb4813 is green (lint, format:check, build, cargo fmt/clippy, 579 Rust tests, events_crash, no literal colours, `invoke` only in `src/events/api.ts`). Manual rows 3.5–3.12 are unchecked (pending, by the user's choice), not falsely ticked.

## Findings

### F1 — A writer with yesterday's `today` deletes today's events

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/events/mod.rs:259 (callers: todo/commands.rs:93, gitlab_refresh.rs:774, runtime.rs:331, github_refresh.rs reconcile)
- **Detail**: `record_in` deletes every row whose date is not the caller's `today`, including later days (the plan's clock-set-back rule). Each writer takes `today`/`now` before waiting for the store lock, and the health round can hold that lock for seconds during reopen/backup. If a write computed at 23:59:59 on day N waits and a day-N+1 write commits first, the stale writer then deletes N+1's events and inserts a day-N row the panel never shows. Narrow (a write straddling midnight during a lock wait), but the loss is silent.
- **Fix A ⭐ Recommended**: Prune only past days (`substr(at, 1, 10) < ?today`); the read already shows only today
  - Strength: One-line change; a stale writer can no longer touch newer rows; its own day-N row is invisible on N+1 and pruned by the next write.
  - Tradeoff: After a clock set back by a day, rows stamped "tomorrow" stay until their date passes and then show on that date; the test `a_clock_set_back_removes_tomorrows_events` becomes "…keeps tomorrow's events hidden today".
  - Confidence: HIGH — the read filter `= today` already carries correctness; pruning is only housekeeping.
  - Blind spot: How often the user's clock really jumps a day back (likely never).
- **Fix B**: Take the event clock inside the lock (`Local::now()` inside the `with_store` closure) at every writer
  - Strength: Keeps the plan's "delete any other day" rule and also makes the event time the commit time.
  - Tradeoff: Touches four call sites and the `todo`/`work_time` signatures again; the todo action and its event can carry different instants.
  - Confidence: MED — removes the stale-day window for event writes, but `todo::roll_over_in` still uses the outer `today`.
  - Blind spot: Interaction with `with_store(today, …)` opening/backing up with the outer day.
- **Decision**: FIXED (Fix A) — `record_in` prunes only past days (`< today`); clock-set-back test now asserts "tomorrow" rows stay hidden; new test `a_stale_writer_keeps_the_newer_days_events`.

### F2 — A failed reload shows a bare Notice without "może być nieaktualna" and last-success time

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/todo/EventLogPanel.tsx:81, src/events/useEventLog.ts:104
- **Detail**: When a reload fails over loaded data, the panel shows only `<Notice tone="error">`. AGENTS.md asks for the last successful refresh time on errors, and `TodoView.tsx:361` renders `ErrorState` titled "… — może być nieaktualna" with `lastSuccess` and a non-blanking refresh. After a failed midnight reload, yesterday's events stay on screen with only an error line. The hook's only `retry` resets to loading (blanks the list).
- **Fix**: Render `ErrorState` (title "Nie udało się odświeżyć dziennika — może być nieaktualny", `lastSuccess={lastRefreshLabel(lastSuccessAt)}`, `onRetry={refresh}`) and add a non-blanking `refresh` to `useEventLog`.
- **Decision**: FIXED — `useEventLog` exposes a non-blanking `refresh`; the panel shows `ErrorState` "Nie udało się odświeżyć dziennika — może być nieaktualny" with the last-success time over the kept list.

### F3 — "Nowe przypisanie" is not atomic with the seen set

- **Severity**: 💬 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:774-799
- **Detail**: Planned trade-off: `Assigned` events are a second `write` after `record_round` commits. A crash or `Busy` in between loses that round's log lines for good (the keys are already seen). Toasts and NOWE flags are unaffected.
- **Fix**: Leave as planned; if it ever matters, write the events in a SAVEPOINT inside `record_round`'s transaction so an event failure is still ignorable.
  - Strength: Zero churn now; the window is one write.
  - Tradeoff: Rare missing log lines.
  - Confidence: HIGH — matches the plan's accepted risk.
  - Blind spot: None significant.
- **Decision**: ACCEPTED — planned trade-off; leave as is.

### F4 — `source_id` CHECK hard-codes today's five sources

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/store/schema.rs (v10), src-tauri/src/events/mod.rs:166
- **Detail**: A new `SourceId` without a new migration would make its error events fail the CHECK (logged at warn, nothing shown).
- **Fix**: Add a test that records `SourceError` for every `SourceId` variant (exhaustive match) and lists it back.
- **Decision**: FIXED — new test `every_source_id_round_trips` (exhaustive match over `SourceId`, record → list).

### F5 — `remove` now refuses a damaged live row

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/todo/mod.rs:546
- **Detail**: `remove` reads the subject first via `live_entry`, which fails as corrupt when the joined row lacks its title/ref; before, such a row could be removed. The list itself already fails to render a row like that, so the user can't reach it either way.
- **Fix**: None needed; optionally fall back to removing without an event when the subject can't be read.
- **Decision**: SKIPPED — the list cannot render such a row either.

### F6 — Panel meta shows 200 when the day is truncated

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/todo/EventLogPanel.tsx (meta)
- **Detail**: `meta = events.length` reads "200" on a truncated day; the footer explains it.
- **Fix**: Show "200+" when `truncated`.
- **Decision**: FIXED — meta reads "200+" when `truncated`.

### F7 — Helpers copied instead of shared

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/todo/EventLogPanel.tsx:48, src/events/useEventLog.ts:20
- **Detail**: `lastRefreshLabel` now exists in four files (EventLogPanel, TodoPicksNotices, TodoView, OwnTasksView); `msToNextMidnight` is a verbatim copy from `useTodo.ts:34`.
- **Fix**: Move both into `src/components/` (e.g. `time.ts`) in a follow-up cleanup, not this slice.
- **Decision**: SKIPPED — sharing the helpers is a cleanup outside this slice.

### F8 — Small drifts from plan wording

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/events/commands.rs:33, src/todo/TodoView.css:116,136-137, src/todo/commands.rs:110
- **Detail**: `events: emit failed: {e}` logs the Tauri error text (no event content; same as `todo: emit failed`), which is outside the letter of criterion 2.5; the new CSS repeats the literal `12px`/`11px`/`16px` sizes that `.todo-source*` already uses; `events-changed` also fires after mutations that store no event (the plan allowed it).
- **Fix**: None; accept as consistent with existing code.
- **Decision**: ACCEPTED — consistent with existing code.
