<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: S-02 github-refresh-interval

- **Plan**: context/changes/github-refresh-interval/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 3 observations
- **Diff reviewed**: `git diff 00e6a19..HEAD` (code commits d9ac21f, 50c8b78, 7abae12)
- **Mode**: non-interactive (run by the wave orchestrator). The review was done in one context instead of two subagents because the diff is small (9 code files) (default chosen by agent). Triage is left to a follow-up agent: each finding carries a recommendation (fix / accept).

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING (F1) |
| Architecture | PASS (F4 observation, a recorded plan decision) |
| Pattern Consistency | PASS |
| Success Criteria | PASS (F3 observation) |

## Checks requested by the orchestrator

| Check | Result |
|---|---|
| Next-try text worded like GitLab's | PASS. `retryText` moved verbatim to `src/components/time.ts:21-34`; both views call the same function, so the four outputs are byte-for-byte the same. |
| Invalid-setting notice worded like GitLab's | PASS. `src/github/GithubView.tsx:236-242` matches `src/gitlab/GitlabView.tsx:651-656` except the key name (`githubRefresh.minutes`), with the same mirror comment. Same position (after the error state, before `TodoPicksNotices`), shown whatever the source state. |
| Invalid `githubRefresh` reported like invalid `gitlabRefresh` | PASS. Same parser (`settings.rs:425-470`, `parse_refresh_minutes(text, key)`), same `(DEFAULT, true)` mapping (`github_refresh.rs:162-172`), same round flag, same warn notice. Neither source reports it in the tray or "Źródła" (as planned). |
| Missing key keeps 10 min | PASS. Missing file, missing key, `null`, `{}`, `{"minutes":null}` all give `Ok(10)` (test `github_refresh_minutes_defaults_and_file_level_problems_give_ten`); `None` settings path gives `(10, false)`. |
| Rust type and TS mirror match (camelCase) | PASS. `GithubSnapshot` has `#[serde(rename_all = "camelCase")]`; `refreshSettingInvalid: boolean`, `refreshMinutes: number`, `nextRefreshAtMs: number \| null` in `src/github/types.ts:49-54`; same commit 50c8b78; pinned by `snapshot_serializes_the_interval_fields`. |
| No migration | PASS. No change in `src-tauri/src/store/`, `lib.rs`, `cli/`, `AGENTS.md`, `gitlab_refresh.rs`. |
| settings.json only read | PASS. The new loader uses only `fs::read`; no write path added. |
| GitLab files changed minimally | PASS. `src/gitlab/GitlabView.tsx`: `formatClock` → `retryText` in the import list and deletion of the local function (17 lines, no behaviour change). No Rust GitLab file touched. |
| CSS / token rules | PASS. No CSS, no new resource, no CSP change. |
| Commits per phase touch only planned files | PASS. d9ac21f: `settings.rs` (+ plan Progress); 50c8b78: `github/mod.rs`, `github_refresh.rs`, `src/github/types.ts`; 7abae12: `time.ts`, `index.ts`, `GitlabView.tsx`, `GithubView.tsx`, `README.md` (+ plan Progress). |
| Invented data only | PASS. Tests use `gitlab.example.invalid`, `octo-test/*`, numeric values. |

## Deliberate-break check (Phase 2 tests, skipped by the Phase 2 implementer)

Each break was applied alone in the worktree, `cargo test --lib github` run, then reverted with `git checkout` (tree clean afterwards).

| Break | Caught by |
|---|---|
| B1: `next_snapshot` stops carrying `refresh_minutes` (uses the default) | `github::tests::snapshot_carries_the_refresh_interval_fields_across_rounds` FAILED |
| B2: `refresh_interval` calls `load_gitlab_refresh_minutes` | `github_refresh::tests::refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes` FAILED |
| B3: `next_refresh_at_ms` serialised as `nextRefreshAt` | `github::tests::snapshot_serializes_the_interval_fields` FAILED |
| B4: `starting()` uses `refresh_minutes: 0` | `starting_snapshot_is_empty` and `snapshot_serializes_the_interval_fields` FAILED |
| B5: `refresh_round` drops `next.refresh_minutes = minutes;` | nothing fails (see F3) |

Phase 1 extra: `load_github_refresh_minutes` delegating with `"gitlabRefresh"` makes 7 tests fail (6 settings tests plus the `refresh_interval` test). The Phase 1 and Phase 2 tests are real, not tautological; only the round wiring itself is untested.

## Automated success criteria (re-run at review time)

| Command | Result |
|---|---|
| `npm run lint` | pass |
| `npm run format:check` | pass |
| `npm run build` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass (lib 659 passed; all integration suites ok) |
| `grep -n REFRESH_INTERVAL src-tauri/src/github_refresh.rs` | no match |
| `grep -rn "function retryText" src` | one match, `src/components/time.ts:22` |

Manual rows 3.5-3.9 are unticked, as required (the user tests them after integration).

## Findings

### F1 — GitHub error message and next-try text run together without a sentence break

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality (UI text)
- **Location**: src/github/GithubView.tsx:132
- **Detail**: The generic error variant renders `` `${state.message} ${retryNote}` ``. No `GithubError::message_pl` or `CliError::message_pl` text ends with a period (`src-tauri/src/github/mod.rs:100-133`, `src-tauri/src/cli/mod.rs:504-520`), so the user reads e.g. "Nie znaleziono programu gh Odświeżam co 10 min — następna próba za 9 min (14:05)." The `LoggedOut` variant is fine (its text ends with "odśwież."). The plan prescribed the plain `" "` join, so this is a plan flaw carried into code. The retry text itself is unchanged and matches GitLab's.
- **Fix**: Join with a period: `` `${state.message}. ${retryNote}` `` (only the generic variant; leave `LoggedOut` as is). Check manual row 3.8 against the new text.
- **Recommendation**: fix (one line, no GitLab file touched).
- **Decision**: FIXED in d5229a5 (period added unless the message already ends with terminal punctuation; LoggedOut variant untouched)

### F2 — Merge risk: S-01's GitHub hide/show path must keep the three round flags

- **Severity**: 💬 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture (merge with parallel slices)
- **Location**: src-tauri/src/github/mod.rs:243-250, src-tauri/src/github_refresh.rs:196-204
- **Detail**: Only `refresh_round` sets `refresh_setting_invalid`, `refresh_minutes` and `next_refresh_at_ms`; `next_snapshot` carries them over. S-01 (github-hide-resurface, same base 00e6a19) adds hide/show commands that write the GitHub snapshot outside a round and appends fields to the same struct and literals. If its write path rebuilds the snapshot with `..GithubSnapshot::starting()` instead of clone-and-modify, a hide would silently reset the interval to 10 and the next try to `null` (the view would say "Odświeżam co 10 min." with no time) until the next round. GitLab pins this with `act_on_issue_keeps_the_refresh_setting_flag` (`gitlab_refresh.rs:1437-1453`); GitHub has no such path yet, so no test can exist on this branch. Other expected conflicts (all textual): struct tail and both literal tails in `github/mod.rs` (keep both field sets); `github_refresh.rs` round body (keep `next.next_refresh_at_ms = …` immediately before `*state.write() = next`, inside any gate S-01 adds); `src/github/types.ts` tail; the `../components` import list in `GithubView.tsx` (S-01) and `GitlabView.tsx` (S-06: do not re-add `formatClock` unless S-06 uses it, or `noUnusedLocals` fails the build); adjacent `README.md` table rows.
- **Fix**: At wave integration, make S-01's GitHub hide/show write clone-and-modify the stored snapshot and add a GitHub twin of `act_on_issue_keeps_the_refresh_setting_flag`.
  - Strength: Mirrors the GitLab precedent and pins the carry-over with a test.
  - Tradeoff: Work lands in the integration branch, not in this slice.
  - Confidence: MED — S-01's code was not read (other worktree, out of bounds for this run).
  - Blind spot: S-01's actual write path.
- **Recommendation**: accept as is on this branch; fix at integration (the code it concerns does not exist here).
- **Decision**: ACCEPT (integration-time action: S-01 hide/show must clone-and-modify the snapshot, plus a GitHub twin of the GitLab carry-over test)

### F3 — The round wiring of the three fields has no test

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria (test coverage)
- **Location**: src-tauri/src/github_refresh.rs:174-213
- **Detail**: Deliberate break B5 (deleting `next.refresh_minutes = minutes;`) passes all tests; the same holds for the `refresh_setting_invalid` and `next_refresh_at_ms` lines. `refresh_round` needs an `AppHandle`, and the plan says so ("Integration Tests: None new"). GitLab's `refresh_round` has the same gap. Manual rows 3.6 and 3.7 cover it in the app.
- **Fix**: None now; extracting a pure "apply round flags" helper would be churn for three assignments.
- **Recommendation**: accept as is (planned, consistent with GitLab, covered by manual checks).
- **Decision**: ACCEPT (covered by manual rows 3.6/3.7)

### F4 — `refresh_interval` mapping duplicated in the two source modules

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src-tauri/src/github_refresh.rs:162-172 (twin of src-tauri/src/gitlab_refresh.rs:515-527)
- **Detail**: A 10-line `(minutes, invalid)` mapping exists in both modules. The validation is not duplicated (one parser in `settings.rs`), and the copy keeps `gitlab_refresh.rs` untouched for S-06, as the plan records ("default chosen by agent").
- **Fix**: After the wave, optionally move the mapping into `settings.rs` or `source_refresh.rs` taking the loader as an argument.
- **Recommendation**: accept as is (recorded plan decision; avoids a conflict with S-06).
- **Decision**: ACCEPT (recorded plan decision)
