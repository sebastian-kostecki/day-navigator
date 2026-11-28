<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Hide and resurface GitHub issues (S-01 `github-hide-resurface`)

- **Plan**: context/changes/github-hide-resurface/plan.md
- **Scope**: Full plan (`git diff 00e6a19..HEAD`, commits ff8e4f7, 409f337, a6a4de1, 13f0761, 16f648c)
- **Reviewed phases**: 1, 2, 3, 4, 5
- **Date**: 2026-11-28
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 7 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

## Summary of evidence

- **Gates (run in the worktree on 2026-11-28):** all green.
  - `npm run lint`, `npm run format:check` and `npm run build`.
  - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
  - `cargo test`: every suite passed, 0 failed; the lib suite ran 675 tests.
- **Phase 2 row 2.3:** `git diff --stat 00e6a19..HEAD -- src-tauri/src/gitlab_refresh.rs src-tauri/src/gitlab/ src/gitlab/` is empty.
- **Phase 4 row 4.3:** `grep -c "_github_issue\|dismiss_github_change" src-tauri/src/lib.rs` prints 3.
- **Phase 5 row 5.2:** the colour-literal grep over `src/github`, `src/components/ItemRow` and `src/todo` prints nothing.
- **`invoke`:** only `src/github/api.ts` and `src/todo/api.ts` import it.
- **Migration v15** (`src-tauri/src/store/schema.rs:298-327`):
  - It is appended only, and its SQL matches the plan character for character.
  - The key is `(source, scope, kind, item_id)`, with `kind IN ('issue','mergeRequest')`. GitLab merge requests (S-07) therefore fit without a new migration.
  - No merge-request hiding is implemented.
- **CLI:**
  - `GhUser` and `GhIssueComments` are fixed `CliCall` variants. `check_read_only` allows two exact shapes: `["api","user"]`, and the 7-argument `issue view <N> -R <repo> --json comments`.
  - Every near-miss listed in the plan is rejected by a test, plus more.
  - The `reference` from the UI is only a lookup key into `kept_full`. It never becomes an argument.
- **Logging:** every new log call carries a kind, a static name or a count.
  - `GithubLogin`, `GithubIssueNumber`, `GithubComment`, `Comment` and `HiddenItemRecord` have hand-written `Debug` that hides their values.
  - Serde errors are dropped, and the comment body is never deserialized.
- **Never an empty list in place of a failed source.** Each failure path behaves as follows:

  | Failure | What the view gets |
  |---|---|
  | Store read or write fails | Unpartitioned repos with `hidden_error` set |
  | One repo fails | That repo keeps its error view; its records get no transition |
  | Login fails | Every due check counts as failed and the notice shows; no `issue view` call runs |
  | Whole source fails | No hidden step runs; `next_snapshot` keeps the previous repos and hidden fields |

- **Lock order** (`src-tauri/src/github_refresh.rs:12-16`, `114-155` and `254-306`):
  - The comment checks run before the gate.
  - The gate, `kept_config` and `kept_full` are taken in that order. The `kept_*` guards are statement temporaries, so no `kept_*` lock is held while the store runs, in either `act` or `commit_github_round`/`reconcile_todo`.
  - The snapshot is stored under the gate. `drop(gate)` comes before `publish`.
- **Merge requirement with S-02 holds.**
  - `act_on_issue` changes the current snapshot in place and never calls `starting()`:

    ```rust
    let mut snap = self.write();
    snap.repos = repos;
    snap.hidden_check_failed = snap.hidden_check_failed.min(hidden_total as u32);
    snap.hidden_error = None;
    snap.seq += 1;
    ```

  - A unit test (`act_on_issue_repartitions_and_changes_only_its_own_fields`) checks that the other fields survive.
  - `refresh_round` likewise only overwrites `repos` and the two hidden fields on the `next` value that `next_snapshot` builds. S-02's `refresh_minutes`, `next_refresh_at_ms` and `refresh_setting_invalid` will therefore pass through, as long as `next_snapshot` and `starting()` set them.
- **Store commands:** `hide_github_issue`, `unhide_github_issue` and `dismiss_github_change` are `async`. They run through `crate::blocking`, do one store op plus `list_items` inside a single `with_store`, and are registered right after `dismiss_gitlab_change`.
- **User decisions are honoured:**
  - `comment_verdict` counts a comment only when it is a real comment, newer than the baseline, and by someone other than the user. The login comparison ignores ASCII letter case.
  - The GitHub fingerprint is empty, so labels never resurface an issue.
  - The login comes from `gh api user` each round and is never stored.
  - The UI mirrors GitLab: the "Pokaż ukryte (n)" toggle, dimmed rows with "ukryte DD.MM", the "Zmienione" flag with its tooltip, and the three row actions with the same icons and aria-labels.
- **Tests:** no case listed in the plan is missing.
  - The round tests cover all 13 scenarios through `fake-cli` and `FAKE_CLI_CALL_LOG`.
  - The store tests cover the round trip, the CHECKs, the v14 → v15 upgrade and the crash harness.
  - The state tests cover `act_on_issue` and `reconcile_input`.
  - Data is invented only (`octo-test/*`, invented logins, `LEAK-MARKER-*`).
- **Manual rows** 1.4 and 5.4–5.12 are unticked. They are pending for the user, which is correct.

## Findings

### F1 — Round types derive `Debug` that prints repo keys and issue numbers

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/github_round.rs:33 (`GithubRoundChecks`), :52 (`GithubRoundOutcome`)
- **Detail**:
  - `GithubRoundChecks` is `#[derive(Debug)]` over `HashMap<(String, u64), _>` and `stopped_at: Option<(String, u64)>`. Its `Debug` output therefore prints the lowercase `owner/name` and the issue number.
  - This is weaker than GitLab: `RoundChecks` there is keyed by an opaque `u64` (`round.rs:27-30`).
  - `GithubRoundOutcome` derives `Debug` over `Vec<RepoView>`, so it prints titles and URLs. GitLab's `RoundOutcome` does the same.
  - Nothing logs either type today, so this is latent. But a single future `debug!("{checks:?}")` would put work data into the app log. The opaque `Debug` on `GithubRepo` exists to prevent exactly that.
- **Fix**: Replace `#[derive(Debug)]` on `GithubRoundChecks` with a hand-written `Debug` that prints counts only. The counts are: records ok/err, checks ok/failed, and `stopped_at.is_some()`. Then add a test that `format!("{checks:?}")` contains no `octo-test`. `GithubRoundOutcome` may stay as it is, for parity with GitLab.
- **Decision**: PENDING

### F2 — No leak test for `HiddenItemRecord` Debug and item-op errors

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/items.rs:79; src-tauri/tests/hidden_items.rs
- **Detail**: The hand-written `Debug` of `HiddenItemRecord` correctly omits `scope` and `item_id`, but no test pins it. There is also no GitHub counterpart of `errors_never_contain_submitted_values` (`hidden_issues/tests.rs`) for `unhide_item` / `dismiss_item` `NotFound` with an invented scope marker.
- **Fix**: Add one test to `tests/hidden_items.rs`. It should check that `{:?}` of a record, and `{e}`, `{e:?}` and the JSON of the item-op errors, are free of the scope and number markers.
- **Decision**: PENDING

### F3 — Comment checks per round are uncapped

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/github_round.rs:138-183
- **Detail**:
  - Each due hidden issue costs one sequential `gh issue view … --json comments`, plus one `gh api user` per round. `gh issue view` returns every comment of the issue (there is no `since`).
  - If many hidden issues are touched at once (a bulk relabel, a bot), that means N processes in one round, with no cap.
  - The design matches the plan's Performance section and GitLab. The halt after the first `Unavailable`/`LoggedOut`/`ToolMissing` limits the damage.
- **Fix**: Leave for now. If a slow round is ever observed, cap the checks per round (for example 20) and carry the rest over through the existing `resume_after` cursor.
  - Strength: the cursor already exists and is tested, so a cap is a few lines.
  - Tradeoff: resurfacing is delayed by one interval or more when more issues are due than the cap allows.
  - Confidence: MED — real lists are small today, so whether this matters is unmeasured.
  - Blind spot: no timing data for `gh issue view` on large issues.
- **Decision**: PENDING

### F4 — `partition_github` scans all records for every issue

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/github_round.rs:295-297; src-tauri/src/github_refresh.rs:122-127
- **Detail**: `records.iter().find(|r| r.scope == scope && r.item_id == issue.number)` runs once per listed issue, across the records of every repo, so the cost is O(issues × records). It runs every round and on every hide, show or dismiss. `act_on_issue` also clones the whole `kept_full` on each click. This is fine at today's sizes.
- **Fix**: Leave; if it ever matters, build a `HashMap<(&str, u64), &HiddenItemRecord>` once per partition.
- **Decision**: PENDING

### F5 — `ListUnavailable` Display text names GitLab

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/hidden_issues/error.rs:19 (and `message_pl`, :84)
- **Detail**:
  - `#[error("there is no GitLab list to act on yet")]` and `message_pl` → "Lista GitLaba nie jest jeszcze dostępna." are now also used for GitHub's `act_on_issue`.
  - The UI maps the error by `kind` (`githubHiddenErrorMessage` has the GitHub text), and `hidden_error` only ever carries store errors, so no wrong text reaches the user. Only the Rust Display and `message_pl` texts are wrong.
- **Fix**: Make the Display text source-neutral ("there is no list to act on yet"). Leave `message_pl` alone, or note in a comment that it is GitLab-only.
- **Decision**: PENDING

### F6 — `github/mod.rs` imports `ChangeReason` from `crate::gitlab`

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/github/mod.rs:17
- **Detail**: The plan says GitHub code imports `ChangeReason` from `hidden_issues`, through the re-export `pub use crate::gitlab::ChangeReason`. The code has `use crate::gitlab::ChangeReason;` instead. It is the same type and behaves the same, but it adds a `github` → `gitlab` dependency that the re-export was meant to avoid.
- **Fix**: `use crate::hidden_issues::ChangeReason;`.
- **Decision**: PENDING

### F7 — fake-cli `issue view` route accepts any argument tail

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/bin/fake_cli.rs:613
- **Detail**: The arm `(Some("issue"), Some("view"))` serves comments whatever arguments follow. Only a wrong subcommand pair reaches exit 98. The guard test keeps the real shape exact, so only test fidelity suffers: a wrong argument list built by `args()` would still get data from the fake.
- **Fix**: Match the full `issue view <N> -R <repo> --json comments` shape in the fake and exit 98 otherwise.
- **Decision**: PENDING

### F8 — Unused tempdir in the store-failure round test

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/tests/github_hidden_round.rs:349
- **Detail**: `let tmp = tempfile::tempdir().unwrap();` is only dropped at the end. The test uses `StoreHandle::new(None)`, so the tempdir serves no purpose.
- **Fix**: Remove the `tmp` lines.
- **Decision**: PENDING

## Fixes applied

Findings 1, 2, 5, 6, 7, 8 fixed in the follow-up fix commit. Findings 3 and 4 left deliberately (only if timing data shows a need).
