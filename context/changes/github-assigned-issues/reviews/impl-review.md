<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Assigned GitHub Issues ("Projects (GitHub)")

- **Plan**: context/changes/github-assigned-issues/plan.md
- **Scope**: Full plan (all automated rows done; every Manual row still pending)
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION (all findings triaged on 2026-11-23: 6 fixed, 1 accepted)
- **Findings**: 0 critical, 3 warnings, 4 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | WARNING |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

Success criteria evidence (re-run during this review):

- Rust:
  - `cargo fmt --check` exit 0.
  - `cargo clippy --all-targets -- -D warnings` exit 0.
  - `cargo test` exit 0: 331 unit tests and all integration suites, 0 failed.
- Frontend: `npm run lint`, `npm run format:check` and `npm run build` all exit 0.
- Manual rows 1.4, 2.4–2.5, 3.4–3.8 and 4.7–4.9 are unticked, so they are pending, not falsely checked.

## Findings

### F1 — Migration v7 github CHECK does not require `gitlab_account IS NULL`

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/store/schema.rs:163-167
- **Detail**:
  - The plan says the github branch requires "all `own_task_id`/`gitlab_*` null".
  - The branch checks `gitlab_kind`, `gitlab_ref`, `gitlab_title` and `gitlab_url` but not `gitlab_account`, so a `source='github'` row with a stray `gitlab_account` passes the CHECK.
  - No app path writes it today.
  - v7 is not on `main` yet, so it is not a shipped migration and can still be edited. After release, closing the gap needs another table rebuild.
  - The `own` branch has the same gap, but it dates from v4.
- **Fix**: Add `AND gitlab_account IS NULL` to the github branch, and add a negative case to the CHECK test in `tests/todo.rs` (~:287).
- **Decision**: FIXED — added `AND gitlab_account IS NULL` to the github branch (schema.rs) and a negative case in `table_check_refuses_inconsistent_github_rows`.

### F2 — GitHub picker says "Brak przypisanych w GitHubie." when repos failed

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/todo/TodoView.tsx:493-503
- **Detail**:
  - `githubIssues` keeps only repos without an error, and `githubOk` is `state === "ok"`.
  - When every configured repo fails in an otherwise Ok round (Failed, ResponseInvalid or Truncated), the picker shows "Brak przypisanych w GitHubie." next to "Część repozytoriów GitHub nie załadowała się…".
  - So it states "nothing assigned" about repos that never loaded, and "Część" is wrong when all of them failed. This brushes the hard rule "never render an empty list in place of a failed source".
  - The failure is still visible through the notice and the red tray.
- **Fix**: Render the empty message only when `githubOk && !githubRepoErrors`. When every repo has an error, change the notice to "Żadne repozytorium GitHub nie załadowało się".
- **Decision**: FIXED — `githubRepoErrors` is now none/some/all; "Brak przypisanych w GitHubie." only shows with no repo errors, "some" shows "Brak przypisanych w repozytoriach, które się załadowały.", "all" shows "Żadne repozytorium GitHub nie załadowało się — nie ma czego dodać."

### F3 — Switching the `gh` account source-ticks GitHub "To do" entries

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture
- **Location**: src-tauri/src/todo/mod.rs:555-627
- **Detail**:
  - `--assignee @me` resolves to whichever account `gh` is using.
  - After `gh auth switch` or a re-login as another user, every open GitHub entry in a loaded repo that is not assigned to the new account gets `ticked_by='source'` ("zamknięte w GitHubie").
  - The GitLab twin avoids this with `gitlab_account` (the v4 S-07 review fix).
  - The plan accepted the risk on purpose ("No tracking of the `gh` account identity", Open Risks in the brief). However, the code does not say so, and the `reconcile_github` doc comment implies a missing issue means it is closed or unassigned.
- **Fix A ⭐ Recommended**: Keep the single-account assumption and state it in the `reconcile_github` doc comment, naming account switching as a known false tick.
  - Strength: Matches the planned scope. A one-comment change, and the next reader sees the limit.
  - Tradeoff: The false tick stays possible, though it only happens on a rare, deliberate user action.
  - Confidence: HIGH — the plan states this exclusion explicitly.
  - Blind spot: How often the user switches `gh` accounts.
- **Fix B**: Record an opaque account key on GitHub rows and reconcile per account, as GitLab does.
  - Strength: Removes the false tick and gives the same guarantee as GitLab.
  - Tradeoff: It needs a new read-only `CliCall` (for example `gh api user`, plus guard updates), a `github_account` column in v7, and wiring through the round. That is a scope increase the plan rejected.
  - Confidence: MED — the GitLab precedent shows the shape, but the gh identity call is new.
  - Blind spot: Whether `gh auth status --json` output on the installed gh version is stable enough to parse.
- **Decision**: FIXED via Fix A — single-account assumption stated in the `reconcile_github` doc comment.

### F4 — Valid fake-CLI GitHub data carries no leak marker

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/bin/fake_cli.rs:309-322
- **Detail**:
  - The plan says invented data carries the existing leak markers, so leak tests can assert they never reach logs or errors.
  - Only the `garbled` output carries `BAD_JSON_MARKER` (`:340`). The parsed alpha titles and repo names carry no marker.
  - So nothing proves that a successfully parsed title never reaches a log line or an error message.
- **Fix**: Put a leak marker in one alpha title, and assert in `tests/github_fetch.rs` that it is absent from the captured log and from every `GithubError` / `message_pl` string.
- **Decision**: FIXED — `octo-test/alpha#3` title carries `LEAK-MARKER-GH-TITLE-5a8e`; new test `a_loaded_title_reaches_the_issue_list_and_nothing_else` asserts it is in the issue list and absent from state, health status and repo error text.

### F5 — One timing-out repo makes the whole source unavailable

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/github/fetch.rs:26-38
- **Detail**:
  - The runner classifies its 20 s timeout as `Unavailable`, and the plan treats `Unavailable` as whole-source, so the round stops.
  - A single repo that always times out keeps GitHub "niedostępny" (calm). Repos after it in the settings order then never refresh; they keep showing the last good list greyed.
  - This follows the plan's design, so it is a plan-level tradeoff, not drift.
- **Fix**: No change now. If it shows up in practice, treat a timeout on a non-first repo as a per-repo error.
- **Decision**: ACCEPTED — the plan designs timeouts as whole-source Unavailable; revisit only if a slow repo shows up in practice.

### F6 — Reconcile can blank a stored "To do" URL

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/todo/mod.rs:615-619
- **Detail**:
  - When a present issue's `web_url` is now `None`, `reconcile_github` writes `github_url = NULL` over a previously validated URL. That happens when `checked_url` drops the URL, for example after a repo rename makes gh return the new owner/name.
  - The entry silently loses its link.
  - This is a suspicion: it depends on how gh reports a renamed repo.
- **Fix**: Update with `github_url = COALESCE(?2, github_url)` (the title still refreshes).
- **Decision**: FIXED — `reconcile_github` keeps the stored URL when the fresh one is None; new test `github_reconcile_keeps_the_stored_url_when_the_fresh_one_was_dropped`.

### F7 — Plan text lags the code in minor places

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: context/changes/github-assigned-issues/plan.md (Phase 2 §3, Phase 3 §1); context/foundation/roadmap.md
- **Detail**:
  - The plan says `check_round` takes a GitHub parameter. The code reads the snapshot inside `check_round` (runtime.rs:299), which is how GitLab already works.
  - The plan names `githubSnapshot()`. The code uses `getGithubSnapshot()`, mirroring `getGitlabSnapshot`.
  - Harmless extras:
    - `RepoError` instead of `()`;
    - `>=` for the truncation check;
    - the extra picker notice.
  - Roadmap S-05 is still "in-progress".
- **Fix**: Leave the code as is. Add a short "Implementation notes" line to the plan, and flip S-05 when the change is archived.
- **Decision**: FIXED — "Implementation Notes" section added to plan.md; roadmap S-05 flips at archive.
