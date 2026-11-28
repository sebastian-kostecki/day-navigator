<!-- PLAN-REVIEW-REPORT -->
# Plan Review: GitHub NEW flag since the last visit (S-03 `github-new-and-notify`)

- **Plan**: context/changes/github-new-and-notify/plan.md
- **Mode**: Deep
- **Date**: 2026-11-28
- **Verdict**: REVISE (light: two targeted edits, no blocking finding)
- **Findings**: 0 critical, 2 warnings, 3 observations

Run non-interactively for the M-4 wave 3 orchestrator (no triage). The plan itself was not edited.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

Grounding: 12/12 paths ✓ (`store/schema.rs`, `github_refresh.rs`, `gitlab_refresh.rs`, `github/mod.rs`, `github/fetch.rs`, `new_assignments/{mod,rules}.rs`, `gitlab_view_state/error.rs`, `lib.rs`, `src/github/{GithubView.tsx,types.ts,api.ts}`, `src/gitlab/useGitlabSeenMark.ts`, `src/App.tsx`, `tests/{hidden_items,new_assignments_crash}.rs`), 14/14 symbols ✓ (`FORGET_AFTER_MS` and `baseline_first_seen` are `pub` and re-exported; `Store::write`/`read`, `StoreHandle::with_store(&mut Store)`, `crate::blocking` + `BackgroundFailed`, `StoreError::{Sql, Damaged, Busy}`, `GithubRepo::key()` (ASCII lowercase, at most 140 chars, so `repo = lower(repo)` and `length <= 255` hold), `RepoResult { repo, outcome }`, `RepoView { repo, issues, hidden, error }`, `GithubError::Truncated`, `gh_issue` / `hidden_record` / `state_with_list` / `prev_with_list` test helpers, `days_off(day, created_at_ms)`, `Flag tone="new"`, `StoreError` TS type), brief↔plan ✓. The only `GithubSnapshot` literals without `..starting()` are the two in `github/mod.rs` the plan names; no TS code builds a `GithubSnapshot` literal; no test hard-codes schema version 15 as the latest (only `hidden_items.rs` asserts v15's own position). Progress↔Phase: 6/6 phases, 27/27 criteria rows, no checkboxes in phase blocks ✓.

Checked and found sound (no finding):

- Seen-until mark semantics mirror GitLab exactly: `shown_until` = `round_ms` of the last round whose seen-set write succeeded, saved with `max()`, `last_mark` filter in `accept_round` for a round that read an older mark, failed write keeps `kept_new` / `shown_until`, list drop clears both (`gitlab_refresh.rs` `accept_round`, `mark_seen`, round tail).
- `refresh_round` wiring is placeable as written: `record_seen` before the gate borrows `&fetched` before it moves into `next_snapshot`; `list_dropped` is computed before `results` moves in the `kept_full` block; `settle_new` runs before `*state.write() = next`. Lock order (gate → kept_config → kept_full → kept_new → shown_until → last_mark → snapshot; store lock never under a `kept_*` lock) holds in `act_on_issue`, `settle_new` and `mark_seen`.
- Hard Rules: v16 appended only; tables hold repo key, numbers, instants; logs carry kinds only; `SeenKey` / `RepoRound` Debug opaque; Rust↔TS mirrors land in the same phase (snapshot field in Phase 3, error type with the command in Phase 4); the hook takes `view` only (no second timer subscription); "Pokaż ukryte" stays `useState`; `cli/`, `bin/`, `hidden_issues/`, `new_assignments/`, `source_refresh.rs`, `src/gitlab/` untouched (Phase 4 check 4.4).
- User decisions honoured: NEW flag only, no toast or event-log entry; v16 is the slice's only migration; AGENTS.md app-data list updated with keys and instants only; first-ever visit is silent (no mark → nothing fresh; per-repo seeding silent).
- Every phase compiles and keeps gates green on its own: new items are `pub` in a `pub mod` (no dead-code lint), `newReferences` is added in Rust and TS in Phase 3, `markGithubSeen` / `GithubSeenError` are exported (no unused-local error).

## Findings

### F1 — Per-repo seeding without a mark can flood a repo with NEW after a race

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 1 §4 `plan_repo` (`insert_first_seen_ms`), decision 8, Phase 2 `record_round`
- **Detail**: A seeding repo stores its keys at `baseline_first_seen(mark, now)`, which is `now` when no mark exists yet. GitLab can afford that because an account's seeding round is its first round, so no older `shown_until` for that account can become the mark. GitHub's mark is global and seeding is per repo: round R(k) at T(k) reads "no mark" and seeds a newly added repo B at T(k); if the user leaves the GitHub view in the window between `record_seen` and the gate (the window the `last_mark` guard exists for), `mark_seen` saves the previous round's `shown_until` T(k-1) < T(k). From the next round on every issue of B has `first_seen = T(k) > T(k-1)` and shows `Nowe` — the "repo added later floods NEW" case decision 8 set out to prevent. With an existing mark the plan is safe (`min(mark, now)` is at or below every later mark), so only a repo that seeds before the very first mark is exposed (the window is short, but a hide command holding the gate widens it).
- **Fix**: Store seeded keys at an instant no mark can be below — `insert_first_seen_ms = if seed { 0 } else { now_ms }` (or `mark.map_or(0, |m| m.min(now_ms))` to keep the S-06 shape); keep `fresh` empty for the seeding round. Update `seeding_round_is_silent_and_stores_at_the_baseline_instant` (no mark → `0`), the crash model in `github_seen_crash.rs` (round-1 rows first = `0`), and add `a_repo_seeded_without_a_mark_is_never_fresh_under_an_older_mark` to `tests/github_seen.rs` (seed alpha at NOW, seed beta at NOW + 1000 with no mark, `mark_view_seen(NOW)`, record both at NOW + 2000 → fresh empty).
  - Strength: Closes the race at the data level with a one-line rule change; `first_seen_ms` is read only by the NEW rule (forget uses `last_seen_ms`), so nothing else moves.
  - Tradeoff: Departs from the S-06 `baseline_first_seen` helper the plan imports (the import can then go); three tests change.
  - Confidence: HIGH — the sequence follows directly from `mark_seen` saving `shown_until` and `record_round` reading the mark before the gate.
  - Blind spot: None significant.
- **Decision**: PENDING

### F2 — Rule unit tests leave `seeded`, `mark` or `now` unstated

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §4, unit tests of `rules.rs`
- **Detail**: `a_stored_issue_keeps_its_first_instant`, `absent_issue_is_forgotten_after_thirty_days_only` and `repeated_numbers_are_fresh_once_in_input_order` do not say `seeded = true`, the `now_ms`, or (for the repeated-numbers test) a mark and empty `stored`. With `seeded = false` the expected `fresh` values are empty, so an implementer working from the plan alone writes red tests or guesses. The integration and refresh-state tests are fully specified; these three are the gap for a single-agent implementer.
- **Fix**: Add the inputs to each test line, e.g. `seeded = true, now = 1000`; repeated numbers: `stored = []`, `mark = Some(0)`, `current = [3, 2, 3]` → `fresh == [(3, 1000), (2, 1000)]`.
- **Decision**: PENDING

### F3 — No refresh-state test for "a key first seen after the saved mark stays NEW"

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 4 §5 unit tests
- **Detail**: GitLab covers the visit-ends-mid-round sequence with `an_item_from_a_round_stored_after_shown_until_stays_new` (`gitlab_refresh.rs`). The GitHub list has `accept_round_drops_keys_the_last_mark_covers` (the drop side) but no test that a key first seen after the saved mark survives the `last_mark` filter, the half that keeps unseen items flagged.
- **Fix**: Add `a_key_first_seen_after_the_saved_mark_stays_new`: `accept_round(100, Some(&[]))`, `mark_seen(|_| Ok(()))` (saves 100), `accept_round(200, Some(&[(alpha#3, 200)]))` → returns `{alpha#3}`.
- **Decision**: PENDING

### F4 — Phase 4 lists no new imports for `github_refresh.rs`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 §1–§3
- **Detail**: The contracts need `std::collections::HashSet`, `std::sync::atomic::{AtomicBool, Ordering}`, `crate::github_seen::{self, GithubSeenError, SeenKey}`, `crate::github::RepoView` and `crate::store::StoreError` (today only `StoreHandle` is imported). The compiler will point them out, but the plan is otherwise exact to the line.
- **Fix**: Add one "Imports" bullet to Phase 4 §1 with that list.
- **Decision**: PENDING

### F5 — Check 6.2 reads the unstaged diff only

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 6, success criterion 6.2
- **Detail**: `git diff --numstat -- AGENTS.md` prints nothing once the file is staged, so the check passes or fails depending on when it runs.
- **Fix**: Use `git diff HEAD --numstat -- AGENTS.md` (shows `1	1` staged or not).
- **Decision**: PENDING
