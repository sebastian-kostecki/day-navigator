<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Hide and resurface GitHub issues (S-01 `github-hide-resurface`)

- **Plan**: context/changes/github-hide-resurface/plan.md
- **Mode**: Deep
- **Date**: 2026-11-27
- **Verdict**: SOUND
- **Findings**: 0 critical, 3 warnings, 2 observations

Run non-interactively: the owner could not be asked, so the reviewer took each triage decision. Accepted findings fix real defects (a build failure, an ambiguous error path with no test, a verification step that can fail for the wrong reason, a merge risk); rejected ones are recorded with the reason. Every accepted finding has been applied to `plan.md` as a minimal, targeted edit. Progress step titles are unchanged.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

Grounding: 14/14 paths ✓ (`cli/mod.rs`, `bin/fake_cli.rs`, `github_refresh.rs`, `github/mod.rs`, `github/fetch.rs`, `hidden_issues/{rules,round,mod,commands,error}.rs`, `todo/mod.rs`, `src/github/*`, `src/components/ItemRow/*`, `src/todo/TodoView.tsx`); 14/14 symbols ✓ (`VARIANT_COUNT` 10, `EXAMPLE_COUNT` 14, 10-entry `seen`, `guard_number_ok`, `guard_repo_ok`, `check_read_only`, `CliCall::ALL` used only by the guard test and `tests/cli_real.rs`, `FAKE_CLI_CALL_LOG`, `GH_NO_NETWORK`, `reconcile_github`'s per-repo `collect()`, `plan_round`'s `MarkAbsent`, `commit_round`'s still-hidden `check_failed` filter, `ChangeReason` at `gitlab/mod.rs:49`, `formatDay` in `src/components/time.ts`); schema head is v14, so v15 is next ✓; struct-literal file list for `GithubIssue`/`RepoView`/`GithubSnapshot` matches the code 7/7 ✓; brief↔plan ✓.

## Team questions and user decisions (checked)

- **Rules shared without a copy (Q1)**: sound. The algorithm in `rules.rs` touches GitLab types only through item id, `updated_at`, scoped labels, note author/system/`created_at` and the user; two traits plus a neutral `Comment` cover all of them, and the only external callers of `comment_verdict`/`status_changed` are `round.rs` and the unit tests in `hidden_issues/tests.rs`, both listed in Phase 2.
- **Storage shape for S-07 (Q2)**: sound. `(source, scope, kind, item_id)` with nullable `project_id`/`iid` (required for `gitlab` rows) and a JSON fingerprint holds GitLab MRs (host scope, global id, scoped labels, notes call by project id + iid) without a migration. v15 is a pure `CREATE TABLE`.
- **User decisions**: `gh api user` as a new fixed, read-only `CliCall` covered by the guard ✓; only a comment by someone else or leaving and re-joining the open list resurfaces (empty GitHub fingerprint, `createdAt` only) ✓; GitLab look mirrored ✓. No finding contradicts them.
- **Merge section**: S-02 and S-06 overlaps are named with a resolution each; edits to shared files are append-only. F1 below makes two verification steps robust to a develop that moved on.

## Findings

### F1 — `git diff develop` checks are not relative to the merge base

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 Automated Verification (row 2.3), Phase 5 Automated Verification (row 5.2)
- **Detail**: `git diff develop --stat -- src-tauri/src/gitlab_refresh.rs src-tauri/src/gitlab/ src/gitlab/` compares the working tree with develop's tip. If develop takes S-06 (which changes exactly those paths) before this slice's Phase 2 runs, the diff shows S-06's changes reversed and row 2.3 fails although this slice touched nothing. The Phase 5 colour check has the same flaw (a reversed S-02 change in `src/github` would be scanned), and it also skipped `src/todo/`, which Phase 5 edits.
- **Fix**: Diff against `$(git merge-base develop HEAD)` (keeps uncommitted work in the comparison) and add `src/todo` to the colour scan.
- **Decision**: FIXED — applied to `plan.md` (both criteria). Reason: a verification step that can fail for a reason outside the slice is a merge risk.

### F2 — Behaviour after a `Failed` `gh api user` is ambiguous and untested

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 — GitHub round (`check_github_round` contract) and its tests
- **Detail**: The contract said "a failed login makes every due check `Err` (when it is `Unavailable`/`LoggedOut`/`ToolMissing` no issue call starts ...)". For a `Failed` or `ResponseInvalid` login it left open whether `gh issue view` calls still start; an implementer could run every comment call and discard the results (wasted processes, and an `Err` verdict that does not match a real call). The `gh-hidden-user-fails` test (non-zero exit, i.e. `Failed`) asserted only the verdicts, not that no comment call ran.
- **Fix**: On any login failure no `gh issue view` starts, every due check is `Err`, `stopped_at` keeps `resume_after`; the `gh-hidden-user-fails` test also asserts the call log holds no `issue view` call.
- **Decision**: FIXED — applied to `plan.md` (contract bullet and test bullet). Reason: an unspecified error path with no test.

### F3 — Phase 3 imports `HiddenIssueError` into `types.ts` where nothing uses it

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 — TS mirror (`src/github/types.ts`)
- **Detail**: The contract said "`ChangeReason` and `HiddenIssueError` are imported from `../gitlab/types`". No field of the GitHub mirror has the type `HiddenIssueError` (`hiddenError` is `string | null`), and `tsconfig.json` sets `noUnusedLocals: true`, so the unused import fails `npm run build` and the Phase 3 gates. `HiddenIssueError` is first needed by `api.ts` in Phase 5.
- **Fix**: Phase 3 imports only `ChangeReason`; Phase 5's `api.ts` imports `HiddenIssueError` from `../gitlab/types`.
- **Decision**: FIXED — applied to `plan.md` (Phase 3 TS mirror, Phase 5 `api.ts` contract). Reason: an infeasible step as written (gate failure).

### F4 — `CHANGE_LABEL` is local to `GitlabView.tsx`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 5 — View (`IssueRow`)
- **Detail**: The plan said the "Zmienione" tooltip comes "from the same `CHANGE_LABEL` texts as GitLab". `CHANGE_LABEL` is a non-exported const in `src/gitlab/GitlabView.tsx:62`. Reusing it means exporting it from a file S-06 changes in parallel (a merge point the plan otherwise avoids, see its "no shared row-button refactor" default).
- **Fix**: `GithubView.tsx` keeps its own copy of the three texts; `GitlabView.tsx` is neither edited nor imported from.
- **Decision**: FIXED — applied to `plan.md` (Phase 5 `IssueRow` bullet). Reason: merge risk with S-06.

### F5 — The GitHub "Pokaż ukryte" toggle is not remembered, unlike GitLab's

- **Severity**: 💡 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: End-State Alignment
- **Location**: What We're NOT Doing (in-memory toggle); Phase 5 AGENTS.md note
- **Detail**: GitLab remembers its hidden expansion in the app data store (`gitlab_view_state`, per AGENTS.md), so "mirror the GitLab behaviour" could include remembering the GitHub toggle. The plan keeps it in memory only and documents that default.
- **Fix**: Remember the toggle in a GitHub view-state record (would extend v15 with one more table).
  - Strength: Full behavioural parity with GitLab.
  - Tradeoff: More schema, store ops, commands and crash tests in a slice that is already five phases; S-03 is the slice that adds a GitHub view record.
  - Confidence: MED — the roadmap gives S-03 "a GitHub view mark" migration, not explicitly the toggle.
  - Blind spot: Whether the owner cares about the toggle surviving a restart.
- **Decision**: DISMISSED (rejected) — a scope choice the plan states and justifies as a default, not a defect; the visible behaviour (toggle, count, dimmed rows, resurfacing) mirrors GitLab. It can be picked up with S-03's GitHub view state.

## Considered and not raised

- `pub use crate::gitlab::ChangeReason` next to the existing private `use crate::gitlab::{ChangeReason, ...}` in `hidden_issues/mod.rs` needs the private import trimmed; the compiler reports it at once, so it is left to the implementer (cosmetic).
- Bot comments (for example CI bots) resurface a hidden issue: this follows the user's decision ("a comment by someone other than me") and matches GitLab, where only system notes are excluded.
- `todo::commands::find_github_issue` reads only `repos[].issues`, so a hidden GitHub issue cannot be newly added to "To do": the same as GitLab (`items.issues` holds visible issues only) and in line with the plan's picker rule.
