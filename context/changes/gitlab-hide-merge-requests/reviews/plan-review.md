<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Hide GitLab merge requests (S-07 `gitlab-hide-merge-requests`)

- **Plan**: context/changes/gitlab-hide-merge-requests/plan.md (commit 926785b)
- **Mode**: Deep
- **Date**: 2026-11-28
- **Verdict**: REVISE. The fixes are small and local; with F1 and F2 applied the plan is ready to implement.
- **Findings**: 2 critical, 4 warnings, 2 observations

Run non-interactively for the M-4 wave 3 orchestrator. No triage was done, so every decision is PENDING.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | FAIL |

## Grounding

Grounding: 21/21 paths ✓, 14/14 symbols ✓ (see the list below), brief↔plan ✓, Progress↔Phase ✓ (6/6 phases; 1.1–1.4, 2.1–2.3, 3.1–3.4, 4.1–4.4, 5.1–5.4, 6.1–6.13; no checkboxes inside the phase blocks).

The symbols checked:

- `act_on_issue`, `shape_items`, `reshape_kept`, `filter_shown`, `refresh_round`, `refresh_loop`, `list_hidden`
- `check_round`, `commit_round`, `check_order`, `stops_round`
- `apply_items_round`, `list_items`, `hide_item`/`unhide_item`/`dismiss_item`
- `IssueRef`, `ISSUE_NOTES_*`, `notes_endpoint_allowed`, `VARIANT_COUNT`/`EXAMPLE_COUNT`/`ordinal`/`seen`
- fake-cli `Endpoint::Notes`, `parse_notes_endpoint`, `gitlab_notes_body`, `gitlab_notes_scenario`, `record_notes_call`, MR fixtures 6002 `(102,34)`, 6003 `(102,12)`, 6101 `(104,21)`
- `all_items`, `mark_fresh`, `keys_of`
- the `reconcile_gitlab` present set, `changedGitlab`/`gitlabHidden` in `TodoView.tsx`
- `RowButtons` gate at `GitlabView.tsx:235`
- `hiddenIssueErrorMessage`, `ItemKind` in TS
- both `AGENTS.md` strings to replace (each found once)

## What the plan gets right (verified in code)

- **No migration.** `HiddenItemRecord`, `NewHiddenItem` and `HiddenItemKey` carry `scope`, `project_id`/`iid` and `changed_reason` (`hidden_issues/items.rs:62-110`). `HiddenKind::MergeRequest` exists.
- **Prune of other hosts.** `apply_items_round(store, source, kind, &[(String, Vec<Transition>)], active_scopes, now)` returns every scope's rows. It prunes rows of non-active scopes by `hidden_at_ms` older than 30 days (`items.rs:264-360`), as the plan assumes.
- **Dismissing a hidden row.** `dismiss_item` on a hidden, unmarked row is `NotFound` (`items.rs:225-242`).
- **Generic `check_order`.** `HideRecord::item_id()` exists for both record types (`rules.rs:64, 108, 132`), so the generic `check_order<R: HideRecord>` is feasible. `pub(super)` reaches the sibling `mr_round`.
- **GitLab issue hiding stays unchanged.** The `hidden_issues` table, `HiddenRecord`, `round.rs` logic and the issue command contracts are untouched. The fingerprint stays `scoped_labels` for `ItemKind::Issue`. The issue round keeps its own write.
- **One record per MR.** `split_review_requests` makes `merge_requests`/`review_requests` disjoint by id. The plan feeds both lists to `plan_round` as "listed" (Key Discoveries), so a review-only hidden MR is never marked absent. Moving between sections keeps it hidden, and a test covers that.
- **Read-only guard.** The new variant is a fixed-argument GET. It is not in `ALL`, sits in the exhaustive `ordinal` and the `examples()` counts, and gets a second exact shape in `notes_endpoint_allowed`. The near-miss list is good: it includes `/issues/` and `/merge_requests/` nested in each other, `-X POST` and `--method POST`. `endpoint_allowed` (`cli/mod.rs:754`) is the only consumer. `tests/cli_runner.rs:32` iterates `examples()` against the `data` echo, so the new example is covered automatically.
- **Telling MR and issue notes apart in fake-cli.** The path is used, not the numbers: MR 6003 and the issue share `(102, 12)`, and the plan's Phase 1 test asserts the difference.
- **Logs and data.** The logs carry kinds and counts only, and the tests use invented data only. The custom `Debug` of `HiddenItemRecord` omits `scope`.
- **Edits to files shared with S-03 are minimal.** `cli/mod.rs` appends, `lib.rs` appends three handler lines, `new_assignments/mod.rs` gets two one-line additions (`all_items`, `mark_fresh`), `AGENTS.md` changes only the hidden-item clause, and `schema.rs` is untouched.
- **Lock order** matches `gitlab_refresh.rs:9-16`: `check_mr_round` runs with no lock, and `commit_mr_round` runs under the gate with the store lock inside.
- **The order between phases is sound.** Every new item is `pub` in a `pub mod`, so clippy reports no dead code between phases. Phase 2 fills the new field with empty values everywhere, and Phase 5 switches all re-shapes at once.

## Findings

### F1 — Phase 2 says "no assertion changes" but a serde pin test breaks

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 — §1 Rust model (third bullet, "no assertion changes")
- **Detail**: The test `snapshot_serializes_for_the_webview` (`src-tauri/src/gitlab/mod.rs:2183-2189`) compares the whole serialized `GitlabSnapshot` with a literal string. That string contains `"filteredOut":{"mergeRequests":0,"issues":0,"hiddenIssues":0,"reviewRequests":0}`. Adding `FilteredOut.hidden_merge_requests` changes the JSON, so `cargo test` goes red in Phase 2. The plan explicitly tells the implementer to change no assertion. A weak implementer will either stall or "fix" the struct instead of the expected string. The compiler does not list this one, because it is a string, not a struct literal.
- **Fix**: In Phase 2 §1 (or §4 Tests), add: "Update the expected JSON in `snapshot_serializes_for_the_webview` (`gitlab/mod.rs`): `"reviewRequests":0}` becomes `"reviewRequests":0,"hiddenMergeRequests":0}` (the field order follows the struct). This is the only assertion that changes in this phase."
- **Decision**: PENDING

### F2 — Phase 4's resume test is underspecified, and the obvious fixture choice does not test resuming

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 — §3 Tests, the `gitlab-notes-unavailable` / `gitlab-notes-one-unavailable` bullet
- **Detail**: The bullet merges two tests into one sentence and names no MR ids or expected counts. The checks run in `item_id` order: 6002 < 6003 < 6101. Under `gitlab-notes-one-unavailable` (Phase 1), only MRs outside `MR_NOTES_KEYS` fail, which among the fixtures is 6003. If the implementer copies the issue test with 6002 + 6003, then 6002 is checked first and answers, 6003 fails, and nothing is starved. The test then either does not exercise `resume_after` or fails on an assertion copied from the issue test (`stopped_at == first`). Only 6003 + 6101 reproduce the issue case: 6003 first fails and stops the round, and the next round with `Some(6003)` checks 6101 first.
- **Fix**: Split the bullet in two:
  - (a) `gitlab_notes_unavailable_runs_one_call_and_fails_two_checks`: hide 6002 and 6101, run `gitlab-notes-unavailable`. Expect 1 notes call, `check_failed == 2`, `stopped_at == Some(6002)`, both still hidden.
  - (b) `a_failing_mr_call_does_not_starve_the_checks_after_it`: hide 6003 and 6101, run `gitlab-notes-one-unavailable`.
    - Round 1 (`None`): `stopped_at == Some(6003)`, 1 call, `check_failed == 2`.
    - Round 2 (`Some(6003)`): 3 calls in total, 6101 is visible in `review_requests` with `Comment`, `hidden_merge_requests` ids are `[6003]`, `check_failed == 1`.

  Copy the structure of `a_call_that_keeps_failing_does_not_starve_the_checks_after_it` (`tests/hidden_issues_round.rs:317-348`).
- **Decision**: PENDING

### F3 — What counts as an MR "state change" was settled by the agent although the research marked it "Owner: user"

- **Severity**: ⚠️ WARNING
- **Impact**: 🔬 HIGH — architectural stakes; think carefully before deciding
- **Dimension**: End-State Alignment
- **Location**: What We're NOT Doing (O1, draft/ready); Implementation Approach (kind-aware fingerprint); Desired End State bullet 2
- **Detail**: The user's binding rule is that an MR comes back only after a state change or a comment by someone else. The research (Open Question 1) says O1 vs O2 "can change the slice size. Owner: user". The plan takes three interpretive defaults together:
  - (a) A merge or close of a hidden MR is never shown as "Zmienione"; the MR silently drops out, and with `closedDays` on it appears in "Zamknięte". Merge is the most common MR state change.
  - (b) Scoped labels never bring an MR back, whereas FR-003's "status" for issues is exactly the scoped label (for example `workflow::review` on an MR).
  - (c) A draft/ready toggle is ignored.

  Each choice is defensible and documented. Together they decide whether the slice meets what the user meant by "state change", and a non-interactive plan cannot confirm that.
- **Fix A ⭐ Recommended**: The orchestrator confirms O1 + labels ignored + draft ignored with the user before `/10x-implement` (one question). No plan edit is needed if they agree.
  - Strength: no rework; the plan already isolates the choice in one fingerprint impl and one "NOT doing" bullet.
  - Tradeoff: one round-trip to the user.
  - Confidence: HIGH. The plan is explicit about every alternative.
  - Blind spot: if the user wants O2, Phases 1, 4 and 6 grow (a second `CliCall`, a state field, a row for an MR that is no longer open).
- **Fix B**: Keep O1 but treat scoped labels as the MR status, as for issues: one shared fingerprint impl, which removes the kind-aware branch.
  - Strength: matches FR-003's existing meaning of "status" and needs less code.
  - Tradeoff: label edits by bots or reviewers resurface MRs, which the user may see as noise.
  - Confidence: MED. Unknown how the team labels MRs.
  - Blind spot: whether work MRs carry scoped labels at all.
- **Decision**: PENDING

### F4 — `examples()` host move is ambiguous and the literal reading does not compile

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 — §1, `examples()` bullet
- **Detail**: Today the `GlabIssueNotes` example moves `host` (`cli/mod.rs:447-453`). The plan says to place the MR example right after it and adds "it takes the cloned host, the issue-notes example keeps the move or both clone". Placed after a move, the MR push cannot use `host`. The parenthetical leaves the implementer to guess.
- **Fix**: State it exactly: "change the `GlabIssueNotes` push to `host.clone()`, then push `CliCall::GlabMergeRequestNotes(host, IssueRef { project_id: 77, iid: 34 })` right after it; `seen` becomes `[1, 1, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 2, 1]`; `with_user` stays 5."
- **Decision**: PENDING

### F5 — Phase 5 is large for one weak-model session

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Completeness
- **Location**: Phase 5 — Refresh wiring and commands
- **Detail**: Phase 5 changes the signatures of `shape_items`, `reshape_kept`, `act_on_issue`→`act_on_item` (closure type `Vec<HiddenRecord>` → `GitlabHidden`), `refresh_loop` (second cursor) and `refresh_round`. That includes the `checks` tuple, two commits, the item literal at `gitlab_refresh.rs:608-613`, the `check_failed` sum and the error fallback. It also changes the private `run` helper in `commands.rs`, adds three commands and three `lib.rs` lines, and rewrites the existing `act_on_issue_*` tests (including `act_on_issue_refuses_an_mr_or_an_unknown_id`, `gitlab_refresh.rs:1364`) plus new tests. Every item is specified, but a single red gate in that much surface is hard for a weak implementer to localise. The work splits cleanly with green gates in between:
  - 5a: `GitlabHidden` threaded through `shape_items`/`reshape_kept`/`act_on_item`, the commands and `lib.rs`.
  - 5b: the MR round wired into `refresh_loop`/`refresh_round`.
- **Fix**: Split Phase 5 into 5a and 5b, as above. Each gets its own Progress rows and commit. In 5a the round still builds `hidden_merge_requests: Vec::new()` but reads `list_gitlab` for the failed-round reshape. Also spell out the new `refresh_round` success arm as a short code sketch: `commit_round` → `commit_mr_round` → the `AssignedItems { merge_requests: mr.merge_requests, issues: round.visible_issues, hidden_issues: round.hidden_issues, review_requests: mr.review_requests, hidden_merge_requests: mr.hidden_merge_requests }` literal.
  - Strength: smaller red surface per commit; matches the "one weak implementer per phase" model.
  - Tradeoff: one more commit and Progress block.
  - Confidence: HIGH. Both halves compile on their own (verified against the current signatures).
  - Blind spot: none significant.
- **Decision**: PENDING

### F6 — The Phase 6 hidden-rows rule drops the `showHidden` gate

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 6 — §2 View, `Lists` bullet
- **Detail**: Today `hiddenOfKind = showHidden ? items.hiddenIssues.filter(...) : []` (`GitlabView.tsx:500`). The plan describes the hidden rows as "`[...items.hiddenIssues, ...items.hiddenMergeRequests]` filtered by kind and view" without the `showHidden ?` guard. A literal implementation shows hidden rows with the toggle off.
- **Fix**: Write it as `const hiddenOfKind = showHidden ? [...items.hiddenIssues, ...items.hiddenMergeRequests].filter((i) => matchesKind(i, kind)) : [];` and the rows as `hidden.map((item) => ({ item, hidden: true, reviewTag: item.reviewRequested }))`.
- **Decision**: PENDING

### F7 — "Do review" says "Brak merge requestów do review." while hidden review MRs exist

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 6 — §2 View, `ReviewBlock` "unchanged"
- **Detail**: When every review-only MR is hidden, `ReviewBlock` (`GitlabView.tsx:398-420`) shows the plain empty text, which reads as "nothing to review". The data is right: the hidden MRs are under the toggle. Only the wording may mislead.
- **Fix**: Optionally mention the hidden ones in the empty text when `items.hiddenMergeRequests.some((i) => i.reviewRequested)` (for example "… (ukryte: n)"). Otherwise accept it as is and leave it to the Manual row 6.6.
- **Decision**: PENDING

### F8 — When the VPN drops, the MR checks add one more 20 s timeout to the round

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Implementation Approach, "Two independent rounds"; Phase 5 §1 `refresh_round`
- **Detail**: The plan runs `check_mr_round` even when `check_round` stopped on `Unavailable`. The runner timeout is 20 s (`cli/runner.rs:60`), so a VPN drop right after the list fetch costs up to 40 s before the gate instead of 20 s. The plan states this and accepts it. A `LoggedOut` or `ToolMissing` stop fails fast, so only the timeout case costs time.
- **Fix**: Optional: skip the MR checks (all due MR checks `Err`, `stopped_at` unchanged) when the issue checks stopped. This needs `RoundChecks` to expose whether it stopped, which it already does through `stopped_at()`. Otherwise accept it.
- **Decision**: PENDING
