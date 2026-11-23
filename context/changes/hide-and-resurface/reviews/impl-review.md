<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Hide and Resurface GitLab Issues

- **Plan**: context/changes/hide-and-resurface/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 2 warnings, 6 observations

Automated gates re-run during review: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (all suites green), `npm run lint`, `npm run format:check`, `npm run build` — all pass. Manual rows 4.3–4.8 are unticked (pending, not falsely checked).

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Findings

### F1 — A slow notes call starves later comment checks every round

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/round.rs:45
- **Detail**: The runner turns its 20 s deadline into `CliError::Unavailable` (`cli/runner.rs:158-160`), which maps to `GitlabError::Unavailable`, and `stops_round` (`round.rs:45-49`) stops all further notes calls on it. Records are always iterated `ORDER BY issue_id` (`hidden_issues/mod.rs:77`), and a failed check never advances the baseline, so the issue stays due. A hidden issue whose `--paginate` notes fetch takes over 20 s (long-lived issue, slow VPN) times out first in the same order every round. Every due issue after it is then counted failed forever: a permanent "Nie udało się sprawdzić N ukrytych", and comment resurfacing silently stops for those issues. The plan chose "stop at the first unavailable" (Performance Considerations), so this is partly a plan flaw: it did not separate "VPN dropped" from "this one call was slow".
- **Fix A ⭐ Recommended**: Rotate the check order. Keep an in-memory cursor in `GitlabRefreshState` and start the next round's notes calls after the issue the previous round stopped on.
  - Strength: Keeps the plan's "a VPN drop costs one 20 s timeout" guarantee. Every due issue gets a turn within a few rounds. No schema change.
  - Tradeoff: The slow issue itself can still never complete its check. `check_round` gains a cursor parameter/return, and the round tests need a rotation case.
  - Confidence: HIGH — the change is local to `check_round` and `refresh_round`.
  - Blind spot: How often the real GitLab notes call nears 20 s is unmeasured.
- **Fix B**: Don't stop the round on a notes-call `Unavailable` when this round's list fetch just succeeded. Stop only on `LoggedOut` / `ToolMissing`.
  - Strength: Every due issue is attempted every round, and a slow one doesn't block the others.
  - Tradeoff: A VPN drop mid-round costs one 20 s timeout per due issue, which contradicts the plan's Performance Considerations. "Odświeżam…" can stay up for minutes.
  - Confidence: MED — the same timeout-vs-unreachable ambiguity remains.
  - Blind spot: The runner cannot tell a timeout from a connect failure today.
- **Decision**: FIXED — via Fix A: `check_round` takes `resume_after` and returns `stopped_at`; the refresh loop carries the cursor (plan Addendum A). Test: `a_call_that_keeps_failing_does_not_starve_the_checks_after_it`.

### F2 — A failed check still resurfaces "returned" and is still counted as failed

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/hidden_issues/rules.rs:165
- **Detail**: The plan says "A failed check (`Err`) yields no transition for that issue". `plan_round` still yields `Resurface{Returned}` for an absent-then-returned issue whose check failed (the doc comment says this is deliberate, since returning needs no notes). `check_round` (`round.rs:105`) still counts that issue in `failed`, so the notice says "Nie udało się sprawdzić N ukrytych" for an issue that is now visible. Related: `act_on_issue` (`gitlab_refresh.rs:88-124`) leaves `hidden_check_failed` unchanged after an unhide, so the count can go stale for up to 5 min. The behaviour is defensible; the plan wording and the counter are what drift.
- **Fix**: In `commit_round`, compute `check_failed` only over `Err` checks whose issue is still hidden after `apply_round`; in `act_on_issue`, clamp it to the new hidden count. Amend the plan line to "yields no comment-based transition (status/returned need no notes)".
- **Decision**: FIXED — `commit_round` counts only failures on rows still hidden; `act_on_issue` clamps the count; plan wording amended. Test: `a_returned_issue_with_a_failed_check_is_not_counted_as_failed`.

### F3 — The command path (`act_on_issue`) has no tests

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:88
- **Detail**: The concurrency-critical path behind hide/unhide/dismiss has no test: `NotInList` for an MR id or an unknown id, `ListUnavailable` with no kept list, `seq + 1`, `hidden_error` cleared, and re-partition from the returned rows. It needs no `tauri`, so `GitlabRefreshState::new()` plus a temp store are enough.
- **Fix**: Add unit tests in `gitlab_refresh::tests` covering those five behaviours.
- **Decision**: FIXED — four `act_on_issue_*` tests in `gitlab_refresh::tests`.

### F4 — Pages shifting under `--paginate` can cause a false "wróciło na listę"

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/rules.rs:151
- **Detail**: With more than 100 open assigned issues, the list is read in offset pages. If an item on page 1 closes while glab is between pages, the next item shifts onto the page already read and is skipped. A skipped hidden issue gets `MarkAbsent`, and the next round resurfaces it as `returned`. Before S-03 a skipped item vanished for one round; now it leaves a mark that stays until dismissed. Unlikely for a personal list.
- **Fix**: Record it as an accepted limitation in the plan. (Alternative if it ever bites: require two consecutive absent rounds before `MarkAbsent`.)
- **Decision**: FIXED — recorded as an accepted known limitation in plan Addendum A.

### F5 — `StoreError` and `storeErrorMessage` are copied, not shared

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/api.ts:41
- **Detail**: `storeErrorMessage` and the `StoreError` TS mirror are copied verbatim from `src/own-tasks/api.ts:36` and `src/own-tasks/types.ts`; `src/worktime/types.ts` holds a further, looser mirror. A wording or variant change must now be made in several places. (The Rust `HiddenIssueError::message_pl` copy is by plan: snapshot `hiddenError` is fixed text.)
- **Fix**: Move `StoreError` + `storeErrorMessage` into a shared `src/components/storeError.ts` and import it from both features.
- **Decision**: FIXED — `src/components/storeError.ts` shared by own-tasks and gitlab (worktime keeps its looser mirror; out of scope).

### F6 — GitLab action errors handled differently from own-tasks

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/useGitlab.ts:87
- **Detail**: `useOwnTasks.mutate` reloads on `NotFound`. The GitLab `act` only shows "…odśwież listę" for `NotFound` / `NotInList` and doesn't call `refreshGitlab()`. `actionError` is cleared only by the next action, so it outlives a fresh snapshot. The notice reuses the `own-task-error` CSS class (`GitlabView.tsx:263`) across features.
- **Fix**: On `NotFound` / `NotInList` call `refresh()`; clear `actionError` when a newer snapshot arrives; rename the class to a shared one (e.g. `action-error`).
- **Decision**: FIXED — refresh on `NotFound`/`NotInList`; a finished round clears the action error; the notice uses the existing shared `.task-error` class.

### F7 — The guard proptest's "every IssueRef passes" claim doesn't hold for id 0

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/mod.rs:783
- **Detail**: The proptest ranges over `1u64..`, but `RawItem` accepts `project_id`/`iid` = 0. `projects/0/...` fails `guard_number_ok` (no leading zero). There is no injection risk (digits only), but the stated invariant is false for a value the parser accepts.
- **Fix**: Reject `id` / `iid` / `project_id == 0` as `ResponseInvalid` in `parse_items` (GitLab ids are ≥ 1), and add a parse test.
- **Decision**: FIXED — `parse_items` refuses zero ids; test `a_zero_identity_field_is_response_invalid`.

### F8 — The crash test covers hide, not dismiss/unhide

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/tests/hidden_issues_crash.rs:56
- **Detail**: Testing Strategy says "crash harness over hide/dismiss writes"; the writer loop only calls `hidden_issues::hide`. Criterion 2.3 ("never loses or half-applies a hide record") is met, so this is a gap against the strategy, not a failed gate.
- **Fix**: Extend the writer loop to alternate hide with dismiss/unhide (or `apply_round`), and assert row invariants after the crash.
- **Decision**: FIXED — the writer also marks (`apply_round`), dismisses and unhides; every reported step is checked as fully applied.
