<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Wspólny stan licznika czasu pracy

- **Plan**: context/changes/shared-work-timer-state/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

## Scope and evidence

- Commits b90031c (p1), 004aa9f (p2), 6c3a9bd (epilogue) on top of 46bacfe. Every changed file is in the plan; none is missing and there are no extras.
- Plan adherence: all planned changes MATCH. The old `useWorkTimer` body moved 1:1 into `useWorkTimerSource` (subscribe before load, `cancelled`, `attempt` deps, day-end timeout, `refetchedFor`, act/start/stop/retry unchanged). The four replaced "today" formulas are equivalent to `total.ts`. `App` does not call `useWorkTimer()`. The START/STOP confirmation code is unchanged.
- Automated criteria rerun: `npm run lint`, `npm run format:check`, `npm run build` PASS; `rg -c "listen\(WORK_TIME_CHANGED"` = 1; one `useWorkTimerSource(` call site (Provider); one `<WorkTimerProvider` (App); one `Math.min(nowMs` (total.ts:9); no `useNow` in `src/worktime`; `src-tauri/` empty diff against merge-base; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (531 passed, 0 failed) PASS.
- Manual rows 1.5–1.7 and 2.5–2.7 are unchecked (pending, not falsely ticked).
- Re-render cost: a tick re-renders only the context consumers (header and the active dashboard or "Czas pracy"), as before; App and AppHeader do not re-render.

## Findings

### F1 — Entries list freezes when only the timer status fails

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src/worktime/WorkTimeView.tsx:83, :187
- **Detail**: The entries list now takes `nowMs` from the timer, which ticks only while the timer status is loaded and running. The plan accepted that the list stops while the timer is in error. New evidence: this is less rare than assumed. `status()` (src-tauri/src/work_time/mod.rs:166-200) also computes the month report, while `entries_for_day()` (:204-240) does not. A month-report failure or a failed timer `listen` therefore puts the timer in error while the entries list still loads, and a running row with "Razem" then stops at the last status time. Before the change the list kept ticking on its own `useNow(1000)`. When both calls succeed they agree: today ticks, past days do not.
- **Fix A ⭐ Recommended**: Keep the accepted trade-off and add a comment at the `EntriesPanel` `nowMs` explaining that the list follows the timer tick and stops while the timer is unavailable (the header shows the error and its retry).
  - Strength: Keeps the decision from planning ("one tick") and the code as reviewed; the header already shows the error and its retry.
  - Tradeoff: In that state the list shows a stale running time until retry or focus.
  - Confidence: HIGH — matches the decision recorded in the plan and brief.
  - Blind spot: How often a month-report failure happens in practice is unmeasured.
- **Fix B**: Fall back to a local tick only when the timer is not ticking: `useNow(entries loaded && runningFromMs !== null && !timerRunning ? 1000 : null)` and pass `Math.max(timerNowMs, localNowMs)`.
  - Strength: The list never freezes, exactly as before the change.
  - Tradeoff: A second, conditional tick comes back, against the "one tick" goal; one more code path to check by hand.
  - Confidence: MED — simple, but only exercised in an error state that is hard to reproduce.
  - Blind spot: The error state cannot be triggered from the UI for a manual check.
- **Decision**: FIXED via Fix A. The user delegated the choice ("napraw samodzielnie"). Added a comment at `src/worktime/WorkTimeView.tsx:83-85`; the single tick stays, so the plan decision and criterion 2.4 still hold.

### F2 — The source hook is exported; only a comment stops a second call

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src/worktime/useWorkTimer.ts:44
- **Detail**: `useWorkTimerSource` is exported as the plan's contract asked, so a future caller could start a second listener and tick. The AGENTS.md sentence and the doc comment are the only guard.
- **Fix**: Leave as planned; if wanted, move `useWorkTimerSource` and the context into `WorkTimerProvider.tsx` as non-exported code and keep only `useWorkTimer` and the types in `useWorkTimer.ts`.
- **Decision**: FIXED differently. The user delegated the choice. Instead of moving the source, a `no-restricted-imports` rule in `eslint.config.js` now forbids importing `useWorkTimerSource` anywhere except `src/worktime/WorkTimerProvider.tsx`. Break-checked: a temporary import in `src/todo/summary.ts` failed lint, and the file was restored. Lint, format:check and build pass afterwards.
