<!-- PLAN-REVIEW-REPORT -->
# Plan Review: "To do" List Implementation Plan

- **Plan**: context/changes/todo-list/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: SOUND (after fixes; SOUND with 2 warnings before)
- **Findings**: 0 critical, 2 warnings, 4 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

14/14 paths ✓, 6/6 symbols ✓, brief↔plan ✓.

These claims were checked against the code and hold:
- The fetch paginates and never returns a partial list (`gitlab/fetch.rs`).
- The vault and GitLab loaders ignore a top-level `todo` block (`settings.rs:26-30, 111-118`).
- `StoreHandle` is registered before `gitlab_refresh::start` (`lib.rs:170, 176`).
- Calling `with_store` from the refresh thread can't deadlock.
- SQLite `date()` can read the `done_at` format (`%Y-%m-%dT%H:%M:%S`).
- Own tasks are never hard-deleted, and `foreign_keys` is ON.

## Findings

### F1 — The midnight `setTimeout` can miss the morning roll-over

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details "Day boundary"; Phase 3 useTodo.ts
- **Detail**: WebView timers don't advance while the laptop sleeps. After an overnight sleep, the view can show yesterday's ticked items until some action happens. `useWorkTimer.ts:62-72` already refetches on `visibilitychange` and `focus`.
- **Fix**: In useTodo.ts, also reload on `visibilitychange` and `focus`, and keep the midnight timer as a fallback.
- **Decision**: FIXED

### F2 — Roll-over runs only in `list()`

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: End-State Alignment
- **Location**: Implementation Approach "Roll-over"; Phase 1 §3
- **Detail**: Every other write runs on rows that haven't rolled over yet.
  - A GitLab entry ticked yesterday blocks a re-pick through the unique index.
  - A task ticked yesterday and reopened in "Moje zadania" before any list read comes back as open. That breaks the end state "a task reopened later does not come back".
- **Fix A ⭐ Recommended**: Run `roll_over_in` first in every `todo` write and in the `set_own_task_done` command.
  - Strength: It closes both gaps with one small idempotent UPDATE.
  - Tradeoff: The own-tasks command now calls into `todo`.
  - Confidence: HIGH — roll-over is idempotent.
  - Blind spot: Own-task `edit` doesn't touch `done_at`.
- **Fix B**: Run it only in the `todo` writes and accept the reopen window.
  - Strength: `own_tasks` stays untouched.
  - Tradeoff: The reopen-after-midnight case is still possible.
  - Confidence: HIGH.
  - Blind spot: How often the app stays open overnight on another tab.
- **Decision**: FIXED (Fix A)

### F3 — The Phase 2 manual check needs the Phase 3 view

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 and Phase 2 Implementation Notes
- **Detail**: Phase 2 stops for manual confirmation, but 2.3 and 2.4 need the Phase 3 view. Phase 1's note asks for manual testing, yet Phase 1 has no manual items.
- **Fix**: The Phase 2 note now says "2.3–2.4 run after Phase 3", and the Phase 1 note says "no manual gate".
- **Decision**: FIXED

### F4 — The crash-test invariant can never fail

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §5
- **Detail**: The own-entry tick is derived from `done_at`, so "tick equals done_at" is always true.
- **Fix**: Assert instead:
  - acknowledged writes are present and nothing is half-applied;
  - there is at most one live entry per item;
  - `ticked_at` and `ticked_by` are both set or both NULL.
- **Decision**: FIXED

### F5 — `useGitlab()` in the picker starts a GitLab round on every visit

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 3 §3 picker
- **Detail**: `useGitlab` calls `refreshGitlab()` on mount (useGitlab.ts:17-57). "Do zrobienia" is the default tab, so every switch back to it runs `glab` calls.
- **Fix**: Add a `refreshOnMount` option to `useGitlab` (default `true`); the picker passes `false`.
- **Decision**: FIXED

### F6 — An unresolved config dir shows as a "todo block" error

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §1 Commands
- **Detail**: `limit: None` from an unresolved `app_config_dir` would tell the user to fix the `todo` block, which is the wrong cause.
- **Fix**: Add `TodoError::ConfigDirUnresolved` and `TodoList.limit_error`. The limit becomes a `Result` passed into the domain functions.
- **Decision**: FIXED
