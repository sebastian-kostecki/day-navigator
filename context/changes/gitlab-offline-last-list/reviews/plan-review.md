<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Last GitLab list without VPN

- **Plan**: context/changes/gitlab-offline-last-list/plan.md
- **Mode**: Deep (risky claims verified directly against the code; no sub-agent, the change is two hunks in one file)
- **Date**: 2026-11-27
- **Verdict**: SOUND
- **Findings**: 0 critical, 0 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | PASS (1 observation) |
| Plan Completeness | PASS (2 observations) |

## Grounding

Grounding: 6/6 paths ✓ (`src/gitlab/GitlabView.tsx`, `GitlabView.css`, `src/github/GithubView.tsx`, `GithubView.css`, `src-tauri/src/gitlab/mod.rs`, `src-tauri/src/hidden_issues/commands.rs`, `src-tauri/src/todo/commands.rs`), 9/9 line refs ✓ (`GitlabView.tsx:483,602,604-613,644,697,700`, `GitlabView.css:9-11`, `GithubView.tsx:168,172-179`, `gitlab/mod.rs:593-622`, tests at `1576,1606,1835,1906`), brief↔plan ✓, Progress↔Phase ✓ (one `## Progress`, 7 automated + 6 manual rows match the phase criteria, no checkboxes in the phase block).

## Verified claims

1. **Stale state reaches the UI with items after an Unavailable round.** `refresh_round` (`src-tauri/src/gitlab_refresh.rs:535-720`) calls `next_snapshot` with the kept config; on `GitlabError::Unavailable` with an unchanged config `next_snapshot` copies `items`, `last_success_ms`, hidden state, filter fields, `closed` and `account` (`gitlab/mod.rs:593-622`), keeps `kept_full` (`gitlab_refresh.rs:680-701`) and publishes. The "refreshing" pre-publish (`gitlab_refresh.rs:550-555`) touches only the flag. `useGitlab` keeps the newest snapshot by `seq`. So the UI receives `state: "unavailable"` with `items` set; only the `&& !offline` gate (`GitlabView.tsx:697`) hides it. Confirmed.
2. **Row actions on stale rows never call GitLab.** Hide/unhide/dismiss go through `act_on_issue` (`gitlab_refresh.rs:135-186`) on `kept_full` + store; `ListUnavailable` only when no list is kept. "+" (`todo/commands.rs:135-155`) reads `snapshot.items` and `snapshot.account`, both kept. Todo reconcile and notifications are skipped when the state is not Ok. Leaving the view (`mark_gitlab_seen`) is store-only and saves `shown_until` of the last successful round, so NOWE flags on the stale list behave correctly (and now the user actually sees them before they are marked seen; previously they were marked seen offline without being shown). Hiding a stale row stores a baseline from the stale item; if the issue changed while offline it reappears as changed on the next Ok round, which is the correct "changed since what you saw" semantics. Confirmed, no misbehaviour.
3. **Header text after the reorder** (branches: items && configured → counts; offline → "źródło niedostępne"; error → "błąd źródła"):
   - ok + items → counts (unchanged); ok without items → none (unchanged; not reachable after a success).
   - error + items → counts (unchanged, already today); error without items → "błąd źródła" (unchanged).
   - unavailable + items → counts (new, matches `GithubView.tsx:172-179`); unavailable without items → "źródło niedostępne" (unchanged).
   - notConfigured → none (branch 1 excludes it; unchanged). Starting with items → counts (as today). Correct.
4. **AGENTS.md "Never render an empty plan or list in place of a failed source".** An empty kept list renders its normal `emptyText` dimmed *under* the calm "GitLab niedostępny" card, never instead of it; FR-015 ("zamiast pustej listy") is satisfied for the same reason. GitHub does the same: in a non-ok state `allEmpty` is false, so dimmed `RepoGroup`s with "Brak otwartych zgłoszeń…" render under its card (`GithubView.tsx:168-170,221-230`), and the GitLab `error` state already does it today. The default is acceptable. See F1 for the missing manual row.
5. **Accessibility.** The calm card is `role="status"` (`src/components/SourceState/SourceState.tsx:38-40`) and precedes the list in DOM order, so assistive tech hears "GitLab niedostępny" before the rows. Dimming is `opacity` only: everything stays focusable and operable, which matches the "actions stay enabled" decision. See F3.
6. **Blast radius.** Both edits are local to `GitlabView`; `Lists`, `GitlabView.css`, `useGitlab.ts`, `GitlabListViewProvider.tsx` and Rust are untouched. Overlap with S-05 is limited to the two hunks the plan names.

## Findings

### F1 — Manual rows miss the empty-kept-list and NOWE cases

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 — Manual Verification; Progress 1.8–1.13
- **Detail**: The plan's one judgement call against the AGENTS.md empty-list rule ("A kept list that is empty shows its normal dimmed empty message under the card") has no manual row, and neither has the NOWE flag on a stale list (archived `gitlab-new-and-hidden-date` plan expects it to stay). Row 1.13 ("No CSP check needed") is a statement, not a check the user can tick.
- **Fix**: Add manual rows "Successful round with an empty list, then VPN off: card on top, dimmed empty message under it, header `0 widoczne · 0 ukryte`" and "NOWE flags from the last round still show on the dimmed list"; turn 1.13 into a note under the phase or drop it.
- **Decision**: FIXED

### F2 — Criterion 1.7 depends on a single implementation commit

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 — Automated Verification, row 1.7
- **Detail**: `git diff --stat HEAD~1 -- src src-tauri` only proves the scope if the implementation lands as exactly one commit right after the docs commits. A fix-up commit or a rebase onto S-05 makes `HEAD~1` the wrong base.
- **Fix**: Use the branch base: `git diff --stat develop...HEAD -- src src-tauri` lists only `src/gitlab/GitlabView.tsx`.
- **Decision**: FIXED

### F3 — Staleness of the list is visual only

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1, change 2 (`<Lists>` condition)
- **Detail**: Dimming is `opacity: 0.55` (hidden rows ≈ 0.33); nothing in the list itself tells assistive tech or a low-contrast reader that the rows are stale. Mitigated: the `role="status"` card is announced first and sits above the list, and GitHub and the GitLab error state behave identically. Changing it here would widen the diff in a file shared with S-05 and break GitHub parity, so it is not a reason to revise this plan.
- **Fix**: Accept for this slice (parity with GitHub); if wanted, a later cross-view follow-up can add an sr-only "nieaktualne" to both stale wrappers. Row 1.10 already covers the hidden-row readability check.
- **Decision**: ACCEPTED
