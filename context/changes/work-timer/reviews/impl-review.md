<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Work Timer Implementation Plan

- **Plan**: context/changes/work-timer/plan.md
- **Scope**: Full plan (manual checks in phases 2–4 still pending)
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 4 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

Notes:
- Plan adherence: every item in Changes Required is MATCH. There is one harmless drift: `now` is read inside the store lock (`commands.rs:44`), not in the command body.
- Additions not in the plan, all harmless: the 500 ms day-end slack, `resetOnSave`, the extra refetch after a mutation, and the `StoreError` kind texts. Nothing breaks "What We're NOT Doing".
- Automated checks run on 2026-11-23, all passing except one:
  - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass. That is 137 unit tests plus every integration suite, including `work_time` and `work_time_crash`.
  - `npm run build`, `npm run lint` and `npm run format:check` pass.
  - `cargo audit` FAILS (see F4).
- Manual rows 2.4, 3.3–3.10 and 4.3–4.9 are unticked, so they are pending. None is falsely ticked.

## Findings

### F1 — A past day's total leaves out a running entry's portion

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/mod.rs:192-196, src/worktime/WorkTimeView.tsx:242-244
- **Detail**:
  - The Rust side: in `entries_for_day`, once `now >= day_end` a running entry's portion `p` goes into neither `closed_ms` nor `running_from_ms`.
  - Example: START at 23:00, still running at 01:00. Yesterday's view shows the row as 1:00:00 but "Razem 0:00:00".
  - The UI side: the view gates the running part on `isToday`. A view loaded before midnight and still open after it therefore also drops the running part from "Razem".
  - The test at `tests.rs:469-490` asserts `day_portion_ms` but never `closed_ms`.
- **Fix**: Rust adds `p` to `closed_ms` when the entry runs and `now >= day_end`. The UI computes `max(0, min(nowMs, dayEndMs) - runningFromMs)` with no `isToday` gate. Add a `closed_ms` assertion to the midnight test.
- **Decision**: FIXED

### F2 — Header "Dziś" is neither capped at day end nor clamped at zero

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/worktime/WorkTimerHeader.tsx:27
- **Detail**:
  - The header computes `total = todayClosedMs + (nowMs - runningFromMs)`.
  - If the day-end `setTimeout` fires late, "Dziś" keeps counting past midnight. This happens with the window visible and focused, so no visibility or focus refetch runs, and it can also happen after a throttled timer or a sleep. The total then includes yesterday.
  - If the clock moves backwards, the total drops below `todayClosedMs`.
  - The view already clamps at `WorkTimeView.tsx:244`; the header does not.
- **Fix**: Use `todayClosedMs + max(0, min(nowMs, dayEndMs) - runningFromMs)`, and call `refresh()` from the 1 s tick once `nowMs >= dayEndMs`.
- **Decision**: FIXED

### F3 — Retry after a failed subscribe loads data that never updates

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/useWorkTimer.ts:139, src/worktime/useWorkEntries.ts:118
- **Detail**:
  - In both hooks, `retry` only calls `refresh()`. If `listen(WORK_TIME_CHANGED)` failed, Retry shows data that never receives `work-time-changed`.
  - Example: the user removes the running entry in the view, and the header still shows it running. Both are in the same window, so no focus event fires either.
  - `src/health/useHealth.ts:27-29,58` deliberately avoids this with an `attempt` counter: "A failed subscribe never loads".
- **Fix**: Add an `attempt` state to each hook's subscribe effect deps and make `retry` do `setAttempt(n => n + 1)`, as `useHealth` does.
- **Decision**: FIXED

### F4 — `cargo audit` fails on a new advisory for `cookie`

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Success Criteria
- **Location**: src-tauri/Cargo.lock (`cookie` 0.18.2)
- **Detail**:
  - `cargo audit` exits 1 on RUSTSEC-2026-0594, "Private cookie nonces can repeat in processes that fork", published 2026-11-23.
  - `cookie` is a transitive Tauri dependency that was already in the lockfile on main; this branch only added `chrono-tz`, `phf` and `phf_shared`. So the failure is not a regression from this change.
  - Still, criterion 1.6 fails today, and the CI `cargo audit` step will fail the next push.
  - The new dev-dependency `chrono-tz` and `chrono` are also flagged "unmaintained" (RUSTSEC-2026-0555 and -0543). Those are only allowed warnings.
- **Fix A ⭐ Recommended**: `cargo update -p cookie` to a patched version, if one exists within Tauri's semver range. Commit the lockfile.
  - Strength: Removes the advisory, and needs no audit exceptions.
  - Tradeoff: Lockfile churn on this feature branch (could be its own chore commit).
  - Confidence: MED — whether a patched release exists and is semver-compatible has not been checked.
  - Blind spot: Whether Tauri pins `cookie` tightly.
- **Fix B**: Ignore RUSTSEC-2026-0594 in `src-tauri/.cargo/audit.toml` with a written justification (a Windows desktop app that never forks). Revisit when Tauri updates.
  - Strength: Unblocks CI immediately, with no dependency changes.
  - Tradeoff: Adds the repo's first audit exception, and it has to be removed later.
  - Confidence: MED — the fork-only trigger looks irrelevant on Windows, but it has not been verified whether Tauri uses private cookies.
  - Blind spot: The advisory's full affected scope.
- **Decision**: FIXED (Fix A: cookie 0.18.2 → 0.18.3; cargo audit exits 0)

### F5 — START/STOP can commit and still report an error

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/commands.rs:78-97, src/worktime/useWorkTimer.ts:118
- **Detail**:
  - `start`/`stop` commit their write, and then `status` reads in the same closure. If that read fails, the command returns an error and emits no `work-time-changed`.
  - The header then shows an action error over a stale "stopped" state, although the entry is running.
  - Separately, `actionError` is cleared only by the next action, not by a later successful refresh.
- **Fix**: Emit the event once the write succeeds, whatever happens to the status read (for example, split the write and the read into two steps). Clear `actionError` when a refresh succeeds.
- **Decision**: FIXED

### F6 — Entries hook: stale refresh after a mutation and relabelled errors

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/worktime/useWorkEntries.ts:91, src/worktime/useWorkEntries.ts:24-26
- **Detail**:
  - `withBusy` calls the `refresh` captured for the day where the action started. If the user changes day while an edit is in flight, the stale day-A refresh can get the newest request id. The day-B view then stays on "Ładowanie…" until the next event.
  - `toError` maps any rejection without a `kind` (IPC or deserialization strings) to `Background`. That hides the real text, which `workTimeErrorMessage` would otherwise show as "Nieoczekiwany błąd: …".
- **Fix**: Drop the explicit `void refresh()`, since the event already refetches. Return the raw error and let `workTimeErrorMessage` format it.
- **Decision**: FIXED

### F7 — Overlap and lookup checks read every counted row on each write

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/mod.rs:85-92, 129-142, 227-232, 309-312
- **Detail**:
  - `start`, `add` and `update` load all counted rows. `update` does so only to find one id.
  - That is cheap at a few thousand rows a year, but it grows without bound and ignores the `work_entries_start` index the plan added for this.
- **Fix**: Push the predicates into SQL: `WHERE id = ?`, and `start_ms < ?end AND (end_ms IS NULL OR end_ms > ?start) LIMIT 1`.
- **Decision**: FIXED

### F8 — Clock and time-zone edge cases give misleading refusals

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/mod.rs:246-248, src-tauri/src/work_time/rules.rs:48-51
- **Detail**:
  - After a backwards clock change, STOP is refused as `EndNotAfterStart` ("Koniec musi być później niż początek"), and START is refused as `Overlap`, until the clock catches up. No data is lost.
  - A local midnight inside a DST gap would make `day_bounds` fail for the whole day. That cannot happen in Europe/Warsaw.
- **Fix**: Give STOP with `now <= start` its own Polish message about the system clock. Leave the DST-gap midnight case as a documented limitation.
- **Decision**: FIXED

### F9 — Routine refusals are logged as warnings

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/work_time/commands.rs:48
- **Detail**:
  - `run` logs every error at `warn`, including expected user refusals (`Overlap`, `InFuture`, `EndNotAfterStart`, `AlreadyRunning`). That adds noise to the app log. The task commands do not log refusals.
  - The header's inline error, instead of `ErrorState`, is sanctioned by the plan's "compact" contract, so it is not a finding.
- **Fix**: Log domain refusals at `debug`, and keep `warn` for `Store` and `Background`.
- **Decision**: FIXED

### F10 — The crash test never kills inside a transaction

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/tests/work_time_crash.rs:66-76
- **Detail**:
  - Unlike `store_crash.rs`, which sleeps between statements so a kill can land mid-transaction, this writer has no pause inside `write`.
  - Atomicity is already covered by `store_crash.rs`, so this test proves the invariants and that no commit is lost. The doc comment doesn't say so.
- **Fix**: Note in the test's doc comment that mid-transaction atomicity is covered by `store_crash.rs`.
- **Decision**: FIXED

## Triage

The user chose "fix yourself by recommendation" (2026-11-23), and every finding was fixed using its recommended fix:

- F1: Rust counts a running entry's part of a day that has ended in `closed_ms`, and the midnight test asserts `closed_ms`. The view total is capped at `dayEndMs`, with no `isToday` gate.
- F2: The header total is capped at `dayEndMs` and clamped at 0. The 1 s tick refetches once past the day end.
- F3: Both hooks redo subscribe and load on retry, through an `attempt` counter (the `useHealth` pattern).
- F4: `cargo update -p cookie` (0.18.3).
- F5: `start_work`/`stop_work` notify right after the write, then read the status. A successful refresh clears `actionError`.
- F6: Removed the stale post-mutation `refresh`, so the event refetches. Mutations return the raw error to `workTimeErrorMessage`.
- F7: The overlap, START and update lookups run as SQL predicates (`LIMIT 1`, `WHERE id = ?`). `rules::overlaps` is gone because nothing uses it any more.
- F8: `timerActionErrorMessage` gives a clock-behind message for STOP `EndNotAfterStart` and START `Overlap`. The DST-gap midnight is left as a documented limitation; it cannot happen in Europe/Warsaw.
- F9: Domain refusals are logged at `debug`; `warn` is kept for `Store`/`Background`.
- F10: The crash-test doc comment points to `store_crash.rs` for mid-transaction kills.

Verified after the fixes:
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `cargo audit` pass.
- `npm run lint`, `npm run build` and `npm run format:check` pass.
