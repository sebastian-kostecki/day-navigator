<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: GitHub NEW flag since the last visit (S-03 `github-new-and-notify`)

- **Plan**: context/changes/github-new-and-notify/plan.md
- **Scope**: Full plan (`git diff 76cd680..HEAD`, commits 33e8b6a, f9e2d75, f49ea3d, b71ab23, 926211a, 84ba309)
- **Reviewed phases**: 1, 2, 3, 4, 5, 6
- **Date**: 2026-11-28
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 3 observations

Run non-interactively (no triage). Phase 5 Manual rows 5.3–5.8 are still open and stay unticked; the user checks them in a built app.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING (F1) |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

### Evidence summary

- **Plan adherence**: every "Changes Required" item is present and matches its contract: migration v16 SQL is verbatim and appended after v15 (v1–v15 untouched); `github_seen::{error, rules, mod}` match the signatures, the opaque `Debug` of `SeenKey` and `RepoRound`, seeding at `first_seen_ms = 0`, the 30-day forget by `last_seen_ms` only, prune of repos no longer in settings, `max()` on the mark; `GithubSnapshot.new_references` and its TS mirror `newReferences`; `kept_new` / `shown_until` / `last_mark`, `accept_round`, `settle_new`, `mark_seen`, `record_seen`, `mark_github_seen`; `useGithubSeenMark`, the `Nowe` flag on visible rows only; the one AGENTS.md insertion. The two documented deviations (reopen test via `Store::open`, inline unit tests) are harmless.
- **Scope**: no edit in `src-tauri/src/cli/`, `src-tauri/src/bin/fake_cli.rs`, `src-tauri/src/hidden_issues/`, `src-tauri/src/new_assignments/`, `src-tauri/src/source_refresh.rs`, `src/gitlab/` (`git diff --stat 76cd680..HEAD` on those paths is empty). `lib.rs` has exactly two inserted lines (`pub mod github_seen;`, `github_refresh::mark_github_seen,`), nothing reordered. No toast, no event-log entry, no new `CliCall`.
- **Race / seeding logic** (compared with `gitlab_refresh.rs` `accept_round` / `mark_seen` and `new_assignments::record_round`): the visit-ends-mid-round guard is the GitLab one (the round reads the DB mark before the gate, `accept_round` drops keys with `first_seen_ms <= last_mark`); per-repo seeding stores keys at `0`, so a mark saved later, older than the seeding round's `now`, never flags them (tested by `a_repo_seeded_without_a_mark_is_never_fresh_under_an_older_mark`); no mark means nothing is fresh (first-ever visit is silent); a failed seen-set write leaves `kept_new` and `shown_until` unchanged; a dropped list clears both.
- **Locks and store**: lock order `hidden_gate` → `kept_config` → `kept_full` → `kept_new` → `shown_until` → `last_mark` → snapshot holds in `accept_round`, `settle_new`, `mark_seen` and `act_on_issue`; `mark_seen` copies `shown_until` and releases it before `save` takes the store lock; `record_seen` runs before the gate with no state lock held. The seen set is one `Store::write` per round; `mark_github_seen` is `async` and calls `with_store` inside `crate::blocking`. Store errors carry no row values (`Damaged` uses a static reason).
- **Logs**: only `github: seen set failed: {kind}` and `github: mark seen failed: {kind}` (Busy at debug, first non-busy seen-set failure at warn). No repo, number, reference or title.
- **Contracts**: `GithubSeenError` (`tag = "kind"`, `content = "detail"`) mirrored as a tagged union in `src/github/types.ts`; `newReferences: string[]` mirrors `Vec<String>`. Both changed in the same commit as their Rust types.
- **Frontend rules**: `useGithubSeenMark(view)` takes the view id only (no second timer subscription); "Pokaż ukryte" stays `useState` in memory; no CSS added (reuses `<Flag tone="new">`).
- **Tests**: invented data only (`octo-test/*`, "Invented N"); assertions check exact rows, instants and fresh lists, not just "no panic"; the crash test reuses `tests/common/crash.rs` and models seeding at `0`.
- **Success criteria**: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass on HEAD 84ba309 (including `--test github_seen`, `--test github_seen_crash`, `--lib github_seen`, `--lib github_refresh`, `--lib github::tests`). `grep -c "GitHub seen-issue keys" AGENTS.md` = 1.

## Findings

### F1 — A repo that fails in the shown round lets the mark cover its unshown NEW issues

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality (correctness; plan gap)
- **Location**: src-tauri/src/github_refresh.rs:200 (`accept_round`, `*shown_until = Some(now_ms)` at :212); src-tauri/src/github_seen/mod.rs:35 (`round_input` leaves failed repos out)
- **Detail**: GitHub has a case GitLab does not: a round can succeed overall while one repo fails (`Err`, `Truncated` included). Then `round_input` leaves that repo out, `accept_round` replaces `kept_new` with the fresh keys of the loaded repos only, and still sets `shown_until = round_ms`. Scenario: issue `octo-test/beta#9` first appears in round N (stored `first_seen_ms = N`, NEW). Before the user opens the view, round N+1 runs with `beta` failing (the view shows the repo error, no list). The user opens the GitHub view, sees the error, leaves: `mark_seen` saves `N+1`, which covers `first_seen_ms = N`. When `beta` loads again, `#9` is not fresh and is never flagged, although it was never shown. This breaks the plan's own rule (decision 13, "leaving the view never marks unshown items as seen") and the user decision ("NEW flag on items that appeared since the last visit"). An `Ok` round where every repo fails is the extreme case (all fresh keys dropped, `shown_until` still moves). The plan's tests do not cover a partial-failure round in `github_refresh` (`accept_round` only sees whole-success inputs).
- **Fix A ⭐ Recommended**: Remember the active-but-not-loaded repo keys of the shown round next to `shown_until` (e.g. `shown_until: Option<(i64, Vec<String>)>`), pass them to the mark save, and in the same `mark_view_seen` write, before moving the mark, set `first_seen_ms = until + 1` for rows of those repos with `old_mark < first_seen_ms <= until` (skip when there is no old mark: nothing was fresh then). Those issues stay fresh after the mark and flag once the repo loads again. Add a `github_refresh` unit test and a `tests/github_seen.rs` test for a failed-repo round followed by a mark.
  - Strength: keeps the single global mark and migration v16 as they are; works from the stored rows, so it also covers the first round of a session (empty `kept_new`); fixes the invariant exactly where it breaks.
  - Tradeoff: `shown_until` and `mark_view_seen` grow a parameter and a second statement; a moderate edit across two modules plus tests.
  - Confidence: MED — seeded rows (`0`) are never in the range, and rows of loaded repos are untouched.
  - Blind spot: not prototyped; a repo that fails for 30+ days has its rows forgotten only when it loads again (forget runs for loaded repos), which this fix does not change.
- **Fix B**: Accept and document it: add a plan addendum (and a line in the module doc of `github_refresh.rs`) that a NEW issue of a repo failing at the end of a visit may lose its flag, plus a follow-up issue.
  - Strength: no code change before merge; the flag is a convenience (plan decision 13) and partial repo failures are uncommon.
  - Tradeoff: the documented rule stays violated; a later fix that needs per-repo marks would need migration v17 instead of editing the still-unshipped v16.
  - Confidence: HIGH — documentation only.
  - Blind spot: how often a single repo fails in practice (deleted repo, lost access) is unknown; a permanently failing repo makes the loss certain for its issues.
- **Decision**: PENDING

### F2 — The `new_references` carry-over in `next_snapshot` is always overwritten

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/github/mod.rs:372
- **Detail**: `next_snapshot` copies `prev.new_references` in the kept branch (as the plan asked), but `refresh_round` sets `next.new_references = state.settle_new(..)` on every path right after (github_refresh.rs:450), so the copy never reaches the stored snapshot. Two places own the field; the test `new_references_reset_on_success_and_carry_over_when_the_list_is_kept` pins behaviour production does not use.
- **Fix**: Keep it but add a one-line doc note that `refresh_round` overwrites the field via `settle_new` (or drop the carry-over and its test assertion).
- **Decision**: PENDING

### F3 — `record_round` fails the whole round if `loaded` repeats a repo key

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/github_seen/mod.rs:121 and :139
- **Detail**: `seeded` is read once before the loop, so a repo key that appeared twice in `loaded` would try `INSERT INTO github_seen_repos` twice and hit the primary key, failing every round (logged once at warn, then debug; flags silently off). Today `parse_github_settings` dedupes repos by `key()` (settings.rs:189–195), so it cannot happen; the guard lives in another module only.
- **Fix**: Insert the marker with `INSERT OR IGNORE` (or add the key to a local `seeded` set after inserting) so `record_round` is safe on its own.
- **Decision**: PENDING

### F4 — `plan_repo` looks up stored issues linearly per current number

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality (performance)
- **Location**: src-tauri/src/github_seen/rules.rs:84
- **Detail**: `stored.iter().find(..)` inside the loop over `current` is O(n·m): at the caps (500 issues, 50 repos) about 12.5 million comparisons per round under the store write lock. Harmless at today's sizes, but the store lock is shared with the health round.
- **Fix**: Build a `HashMap<u64, i64>` of `number -> first_seen_ms` once before the loop.
- **Decision**: PENDING

## Merge risks with S-07 (`gitlab-hide-merge-requests`)

Checked against `slice/gitlab-hide-merge-requests` at e9f920c (phases 1–3 so far).

1. **AGENTS.md Hard Rules app-data bullet** — certain textual conflict: both slices edit the same single long line (S-03 inserts after "the GitLab view's seen-until mark (opaque account key, one instant), "; S-07 will widen the hidden-item snapshots text near the start of the same line). Resolve by keeping both insertions.
2. **src-tauri/src/lib.rs `generate_handler!`** — low risk: S-03 inserts after `github_refresh::refresh_github,`; S-07 inserts three lines after `hidden_issues::commands::dismiss_gitlab_change,`. Three unchanged lines separate the hunks, so git should merge cleanly; check the order after the merge.
3. **src-tauri/src/new_assignments/** — no conflict: S-03 only imports `new_assignments::FORGET_AFTER_MS`; S-07 edits `all_items` / `mark_fresh` in `new_assignments/mod.rs` and leaves `FORGET_AFTER_MS` alone.
4. **src-tauri/src/store/schema.rs** — no conflict: only S-03 appends v16; S-07 adds no migration. Tests that derive the latest version from `MIGRATIONS.len()` stay valid; S-07's `tests/hidden_items.rs` changes assert no fixed version number.
5. **src-tauri/src/cli/, bin/fake_cli.rs, hidden_issues/, gitlab_refresh.rs, src/gitlab/** — S-07 only; S-03 does not touch them. `src/github/types.ts` imports `ChangeReason` from `src/gitlab/types.ts`, which S-07 edits but does not change `ChangeReason`.
6. **Combined build** — after merging both into develop, run all gates once: S-07 adds a field to `AssignedItems` (GitLab), S-03 a field to `GithubSnapshot` (GitHub); neither adds a literal of the other's type, so no compile break is expected.

## Fixes applied

- **F1 (Fix A)**: `GithubRefreshState.shown_unloaded` remembers the repos configured but not loaded in the shown round (set with `shown_until`, cleared on a dropped list; a failed seen-set write keeps it). `mark_view_seen(store, until, unloaded)` moves rows of those repos with `old_mark < first_seen_ms <= until` to `until + 1` in the same write, before the mark moves (skipped without an old mark). Tests: `a_mark_saved_while_a_repo_failed_keeps_that_repos_fresh_issues_new`, `a_failed_repo_changes_nothing_without_an_old_mark` (tests/github_seen.rs), `a_round_with_a_failed_repo_hands_its_key_to_the_mark_save_and_a_drop_clears_it`, `unloaded_repos_lists_active_repos_missing_from_the_round_once` (lib). Plan deviations recorded in Phases 2 and 4.
- **F2**: kept the carry-over in `next_snapshot` and documented that `refresh_round` overwrites it via `settle_new` (the existing test pins the pure function's contract).
- **F3**: the seeding marker is inserted with `INSERT OR IGNORE`; test `a_repo_key_repeated_in_one_round_does_not_fail_the_round`.
- **F4**: `plan_repo` builds a `HashMap<u64, i64>` of stored first instants once; behaviour unchanged, existing tests green.

## Re-check (fix commit ff155f9)

- **Date**: 2026-11-28
- **Scope**: `git diff 1657414..ff155f9`
- **Verdict**: OK (no blocking finding)

- **F1**: correct. `shown_unloaded` is set only together with `shown_until` (`accept_round` with `Some(fresh)`), kept with it on a failed seen-set write and cleared with it on a dropped list (`settle_new(true, ..)`). Lock order holds everywhere it is taken: `accept_round` takes `kept_new`, `shown_until`, `shown_unloaded`, `last_mark`; `settle_new` takes `kept_new`, `shown_until`, `shown_unloaded`; `mark_seen` copies `shown_until` and `shown_unloaded` under `hidden_gate`, releasing each guard at the end of its statement, before `save` takes the store lock. `refresh_round` computes `unloaded` purely (no store call) under the gate. The row update and the mark upsert share one `Store::write`, and the update runs first. No new log line, and errors keep going through `GithubSeenError::Store` (no row values or paths). The `last_mark` guard still works. A round that read the old mark before the save drops the failed repo's keys (`first <= last_mark`). The moved rows (`until + 1`) are fresh again in the next round. Edge cases: without an old mark nothing moves (consistent with the silent first visit). Seeded rows (`0`) are never in `(old, until]`. A repo removed from settings is not in `unloaded` (only `cfg.repos`), and a stale key updates zero rows. A repo that fails across several marks is bumped past each mark until it loads. A mark that does not move forward gives an empty range. A repo that loaded earlier in the visit and failed in the last round keeps its flags for the next visit (the safe side). Unhide during a visit does not touch `shown_until` / `shown_unloaded`. The tests use invented data and take the failing path: `a_mark_saved_while_a_repo_failed_keeps_that_repos_fresh_issues_new` would fail without the fix, because `beta#9` would stay at `NOW + 1000 <= NOW + 2000`. The unit test tells "kept on a failed write" apart from an overwrite.
- **F2**: comment only. It is accurate, because the `ThreadDown` path in `start` passes `kept = None` and never reaches the carry-over branch.
- **F3**: `INSERT OR IGNORE`. The new test would hit the primary key without it.
- **F4**: the `HashMap` is equivalent because `(repo, issue_number)` is the primary key, so stored numbers are unique.
- **Scope**: `git diff --stat 1657414..ff155f9` is empty for `src-tauri/src/cli/`, `src-tauri/src/bin/fake_cli.rs`, `src-tauri/src/hidden_issues/`, `src-tauri/src/new_assignments/`, `src-tauri/src/source_refresh.rs`, `src/gitlab/`, `src-tauri/src/store/schema.rs` and `AGENTS.md`.
- **NIT (not fixed)**: the module doc of `github_refresh.rs` (lock-order paragraph, lines 12–15) still lists `kept_new`, `shown_until`, `last_mark` without `shown_unloaded`, while the field doc and the plan deviation name it. Docs only.
- **Gates** (ff155f9): `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` (726 lib tests plus all integration suites) pass.
