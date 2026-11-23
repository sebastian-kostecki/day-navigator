<!-- PLAN-REVIEW-REPORT -->
# Plan Review: GitLab Visibility Filter Implementation Plan

- **Plan**: context/changes/gitlab-visibility-filter/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical · 4 warnings · 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

12/12 paths ✓, 9/9 symbols ✓ (cited line numbers match), brief↔plan ✓, Progress↔Phase ✓

Confirmed by code check:
- `reconcile_gitlab` gives the same key set on the unpartitioned list (`todo/mod.rs:502-508`).
- `#[serde(skip)] milestone` is safe: `GitlabItem` has no `Deserialize`, is never stored, and is never compared with `==` to detect a change.
- The health status reads only `state`.
- `check_round` and `commit_round` must keep the unfiltered list, and the plan keeps it.

## Findings

### F1 — "Closed in the last N days" is really "updated in the last N days"

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: End-State Alignment
- **Location**: Phase 3 §1–§3
- **Detail**: The fetch filters only by `updated_after`, and the parser checks only `state`.
  - An item closed months ago but commented on or relabelled yesterday shows up in "Zamknięte (ostatnie N dni)".
  - The section is sorted by `updated_at`, not by closing time.
  - A date-only `updated_after` is likely read as UTC midnight while the cutoff is local, so the edge of the window is fuzzy.
- **Fix**: Read the optional `closed_at` / `merged_at` fields and keep only items whose closing time is at or after a local-midnight cutoff. Sort by closing time. `updated_after` stays as a server pre-filter, set one day early.
- **Decision**: FIXED (Phase 3 §2, §3, §4, success criteria, Progress 3.2)

### F2 — "To do" picker still says "Brak przypisanych w GitLabie"

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Phase 4 §2
- **Detail**: The picker reads the filtered `snapshot.items`. When everything is filtered out, `TodoView.tsx:459` says "nothing assigned".
- **Fix**: Phase 4 §3 changes the TodoView empty text when `filterActive` is set. Manual check 4.5 covers it.
- **Decision**: FIXED

### F3 — RawItem `state: String` as a required field breaks the fixtures

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 §2
- **Detail**: These fixtures have no `state`:
  - `fake_cli.rs:77-137`
  - `raw_json` (`gitlab/mod.rs:487`)
  A required field would fail `tests/gitlab_fetch.rs`, `tests/hidden_issues_round.rs`, `tests/new_assignments.rs` and two unit tests.
- **Fix**: `#[serde(default)] Option<String>`. A missing state drops the item in the closed parser.
- **Decision**: FIXED

### F4 — An offline filter edit doesn't apply; the invalid notice lies over the stale list

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §1–§2
- **Detail**: A failed round that keeps the items reuses `prev.items`, which were filtered with the old rule. `filter_invalid` is set every round, so a typo made offline shows "pokazuję wszystkie" over a list still filtered by the old rule.
- **Fix**: On a round that keeps the items, re-derive `snapshot.items` from `kept_full` with this round's filter, using the helper shared with `act_on_issue`. Test added to 2.4.
- **Decision**: FIXED

### F5 — The fake CLI would serve open fixtures to the closed calls

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §4
- **Detail**: `fake_cli.rs:59-65` routes on the `merge_requests?` / `issues?` prefix. `tests/cli_runner.rs` asserts exact args.
- **Fix**: Add closed `Endpoint` variants matched before the prefix checks. Update the `cli_runner.rs` assertions. Add a "closed before cutoff, updated after" scenario.
- **Decision**: FIXED
