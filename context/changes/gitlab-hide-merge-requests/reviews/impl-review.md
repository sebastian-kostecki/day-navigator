<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Hide GitLab merge requests (S-07 `gitlab-hide-merge-requests`)

- **Plan**: context/changes/gitlab-hide-merge-requests/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4, 5, 6, 7
- **Date**: 2026-11-28
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 2 warnings, 6 observations

Reviewed commits 72c028c..797faf0 (base 0788def, `git diff 0788def..HEAD`, 31 files). The review ran non-interactively, so no triage was done. Every Decision is PENDING.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Checks that passed

These are the checks the orchestrator asked to be made hard. They are not findings.

- **Read-only guard**: it is still at least as strict as before.
  - `check_read_only` still requires exactly 5 arguments: `api`, the endpoint, `--paginate`, `--hostname` and the host. There is no room for a method or field flag, so the call is a GET.
  - `notes_endpoint_allowed` (`src-tauri/src/cli/mod.rs:743`) accepts exactly two shapes: `projects/<D>/issues/<D>` and `projects/<D>/merge_requests/<D>`, each followed by the fixed suffix. `<D>` must be digits with no leading zero, 1 to 20 long.
  - Mixed paths are rejected in both directions: `issues/1/merge_requests/34` and `merge_requests/34/issues/2`.
  - The removed `merge_requests` near-miss lines in the issue test and its proptest were the only cases that the second shape now makes valid on purpose.
  - The MR call's arguments, name, tool and opaque `Debug` are pinned exactly (`mr_notes_call_has_exact_arguments_name_and_tool`). The call is not in `CliCall::ALL`.
  - The counts are consistent: `VARIANT_COUNT` 14, `EXAMPLE_COUNT` 19, `ordinal` 13, and the `seen` array has one more entry.
  - The guard is not tied to a variant: an issue-notes call would pass the guard with an MR path. That holds only because the arguments are built from constants and pinned per variant. This was already true before the change.
- **GitLab issue hiding is unchanged**:
  - The issue fingerprint is still the scoped labels, through an exhaustive `match self.kind` (`rules.rs:86-95`).
  - `round.rs` only made `stops_round` and `check_order` `pub(super)`, and `check_order` generic over `HideRecord`.
  - The issue commands keep their names, `id: u64` and the log names `hide`/`unhide`/`dismiss`.
  - The issue error texts in `src/gitlab/api.ts` are unchanged word for word. `tests/hidden_issues*.rs` pass unchanged.
- **One record per global MR id, kind checked**:
  - `split_review_requests` keeps the two lists disjoint.
  - The MR records are read with `kind = mergeRequest` and filtered to the host.
  - `act_on_item` matches `i.id == id && i.kind == kind` and picks the list by kind.
  - The UI busy key is `busyKey(kind, id)`.
  - `a_hidden_merge_request_and_a_hidden_issue_with_one_id_do_not_touch_each_other` covers the case where an issue and an MR have the same id.
- **"Zmienione" rule**:
  - The MR fingerprint is empty.
  - `comment_verdict` counts only non-system notes by someone other than `gitlab.user` that are newer than the baseline.
  - Approvals, pushes, labels and draft toggles are system notes or move only `updated_at`. They lead to a notes check and a baseline advance, never to a resurface.
  - Dropping off the list and being listed again gives `Returned` (O1).
  - Tests: `a_system_note_stays_hidden`, `a_label_change_does_not_bring_a_merge_request_back`, `drop_off_marks_absent_then_return_resurfaces_as_returned`.
- **Phase 6 wiring** (`gitlab_refresh.rs:589-660`):
  - `check_mr_round` runs right after `check_round` with no lock held.
  - `commit_mr_round` runs under `hidden_gate` right after `commit_round`, in one `apply_items_round` write.
  - The `mr_resume_after` cursor and the `mr_skipped` flag are kept in `refresh_loop`.
  - `mr_calls_allowed` never skips the MR calls two rounds in a row.
  - `check_failed` adds both halves.
  - On a store failure, `store_error` sets `hiddenError`, and the MR lists are returned as listed, never empty.
- **UI**:
  - Hidden rows render only under `showHidden ?`.
  - The TS mirrors (`hiddenMergeRequests` on `AssignedItems` and `FilteredOut`) match serde camelCase. `hidden_merge_requests` is the last field of `FilteredOut`, and the JSON pin is updated.
  - Only the `api.ts` files import `invoke`.
  - No colour literal was added.
- **Hard rules**:
  - The two new log lines carry a count and an error kind only (`mr_round.rs:142,168`). The command logs carry static names and the kind.
  - The tests use invented data only (`gitlab.example.invalid`, `acme/*`, `invented.user`).
  - There is no migration.
  - The `AGENTS.md` word diff shows only the three planned insertions in the hidden-item snapshot clause.
  - The PRD gets one paragraph (MS-33) and no FR change.
  - `lib.rs` gets exactly three lines after `dismiss_gitlab_change`.
- **Gates on HEAD 797faf0**: all green.
  - `npm run lint`, `npm run format:check` and `npm run build` pass.
  - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.
  - `cargo test`: 955 passed, 0 failed, 35 result lines.
  - The Manual rows (1.4, 7.5-7.13) are unticked and pending for the user, as required.

## Findings

### F1 — Phase 5 state tests promised by the plan are missing

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/gitlab_refresh.rs:1485-1556
- **Detail**: Phase 5 § 3 requires four checks that are not there:
  - (a) `act_on_item_matches_id_and_kind` accepting an MR from `merge_requests` and one from `review_requests`. The test only checks refusals.
  - (b) Hiding an MR that sits in `merge_requests`. Only a review-only MR is hidden, in `act_on_item_partitions_both_kinds_from_one_result`.
  - (c) The cap (`hidden_total`) counting hidden MRs, including filtered-out ones. The only cap tests use issue records.
  - (d) `reshape_kept` with MR records partitioning both kinds. Every call passes `issues_only(..)` or `GitlabHidden::default()`.

  Point (d) matters most. "Every path that re-partitions must partition both kinds" is the plan's own critical detail, and no test pins it for the failed-round path. The integration round tests also leave two edges untested: an `apply_items_round` write failure after a good read, and a hidden review-only MR that drops off and returns through `review_requests`.
- **Fix**: Add the four unit tests named above to `gitlab_refresh.rs`, plus a write-failure case and a review-list `Returned` case to `tests/hidden_mr_round.rs`.
- **Decision**: PENDING

### F2 — The MR guard shape has fewer near-miss tests than the issue shape

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/mod.rs:1243-1287
- **Detail**: The guard code is strict (see "Checks that passed"). The tests for the new shape do not match the issue shape, though:
  - There is no `near_misses_of_a_valid_notes_endpoint_fail`-style proptest with junk for `merge_requests`. The new proptest is positive only.
  - There is no per-character digit test like `notes_guard_accepts_only_digits_in_the_ids`.
  - These near-misses are missing: `:id`/`:iid` placeholders, a leading `/projects/`, `groups/`, a trailing space, 21-digit ids, `order_by=updated_at`, `per_page=50`, and the missing `--paginate` case (`check_read_only(["api", ok, "--hostname", host])`).

  The orchestrator asked for near-misses of the new MR shape. The planned list is there, but the guard is the read-only safety net, so the MR shape should be pinned as tightly as the issue one.
- **Fix**: Mirror the issue junk proptest, the digit test and the missing-`--paginate` assertion for `merge_requests`, in the existing test module.
- **Decision**: PENDING

### F3 — `list_gitlab` couples the issue and MR halves on the re-shape paths

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/merge_requests.rs:36; src-tauri/src/hidden_issues/commands.rs:43-55; src-tauri/src/gitlab_refresh.rs:710-716
- **Detail**: `list_gitlab` lets the first error win, so a failed MR read now also drops the issue records.
  - **Command path**: `op` commits its own write before `list_gitlab` reads. If only the MR read fails, the UI reports that a hide or unhide failed although it was stored, and the snapshot stays stale until the next round. Before this change, only the issue read could cause that.
  - **Failed-round path**: `kept_records` becomes `None`, so `reshape_kept` keeps the old shown list without this round's filter, and no notice explains it. The list is never emptied, so the hard rule holds.

  Both reads hit the same store connection, so in practice their failures are correlated. The risk is low.
- **Fix A ⭐ Recommended**: Accept it and add one doc line on `list_gitlab` saying that both halves fail together on purpose (one store, one notice).
  - Strength: no code change. It matches the "one notice per failure" default in the plan.
  - Tradeoff: in the rare case where only the MR read fails, a stored action is reported as failed.
  - Confidence: MED — failures of two reads on one SQLite connection in one closure are almost always shared.
  - Blind spot: a decode error on one MR row (a bad `changed_reason`) would fail only the MR half.
- **Fix B**: Return the records from the same `store.write` as `op`, or let `reshape_kept`/`act_on_item` take each half's `Result` separately.
  - Strength: a stored action is never reported as failed, and issue re-shaping survives a bad MR row.
  - Tradeoff: more code in shared paths, and it changes the issue command flow, which the slice keeps unchanged.
  - Confidence: MED.
  - Blind spot: the interplay with the GitHub `act_on_issue` pattern.
- **Decision**: PENDING

### F4 — "Ukryte do review: n" also counts assigned MRs that never show in "Do review"

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/gitlab/GitlabView.tsx:608
- **Detail**: `hiddenReviewCount` is `hiddenMergeRequests.filter((i) => i.reviewRequested).length`. That set includes hidden MRs that are both assigned and awaiting review. Those MRs live in the main table, never in the "Do review" panel (`split_review_requests` removes them from `review_requests`). So an empty panel can say "Ukryte do review: 1" when turning on "Pokaż ukryte" would add nothing to it. The code follows the plan ("assigned or not"), so this is a small flaw in the plan.
- **Fix**: Count only the hidden review-only MRs, for example `hiddenMergeRequests.filter((i) => i.reviewRequested && !<was assigned>)`. That needs a flag from Rust, or the count of `partition.hidden` items that came from `review_requests`. Otherwise, confirm the wording with the user.
- **Decision**: PENDING

### F5 — `MrRoundChecks` derives a `Debug` that shows global MR ids

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/hidden_issues/mr_round.rs:29
- **Detail**: `#[derive(Debug)]` prints the `checks` map, whose keys are global MR ids. Nothing logs it today, and the issue `RoundChecks` does the same. `GithubRoundChecks`, the newer precedent, has an opaque `Debug`. Ids are not titles, but the module rule is "never an id" in logs.
- **Fix**: Give it a manual opaque `Debug`, like `GithubRoundChecks`.
- **Decision**: PENDING

### F6 — `partition_merge_requests` relies on the two lists being disjoint

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/hidden_issues/merge_requests.rs:108-135
- **Detail**: If one id were ever in both `merge_requests` and `review_requests`, it would appear twice in `hidden` and be counted twice in "Pokaż ukryte (n)". That would break the user decision that such an MR is one item. Today `split_review_requests` guarantees the lists are disjoint, but that guarantee lives in another module and nothing here checks it.
- **Fix**: Skip ids already placed from `merge_requests` when walking `review_requests` (a small `HashSet`), or add a `debug_assert!` and a doc line naming the invariant.
- **Decision**: PENDING

### F7 — The MR footnote sentence is shown only when hidden MRs exist

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/gitlab/GitlabView.tsx:592-596
- **Detail**: The plan says the footnote "adds" the MR sentence with no condition. The code renders it only when `hiddenMergeRequests.length + filteredOut.hiddenMergeRequests > 0`. The change is harmless and arguably better, but it is not documented.
- **Fix**: Note it in the plan as an accepted deviation, or drop the condition.
- **Decision**: PENDING

### F8 — `hide_gitlab_issue` now passes `super::hide` directly (cosmetic extra)

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/hidden_issues/commands.rs:71; src-tauri/src/gitlab_refresh.rs:1180
- **Detail**: The closure was replaced by the function path. The behaviour is the same. A redundant `hidden_merge_requests: Vec::new()` also sits next to `..AssignedItems::default()` in a test. Both changes are harmless and not in the plan.
- **Fix**: None needed. Optionally drop the redundant test field.
- **Decision**: PENDING

## Merge risks vs S-03 (`github-new-and-notify`)

- `AGENTS.md` Hard Rules app-data line: both slices edit the same long line, so a textual conflict is certain. Keep S-07's three insertions word for word: "and merge requests", "issue " and "GitLab merge requests and ".
- `src-tauri/src/cli/mod.rs`: if S-03 adds a `CliCall`, the second slice to merge must renumber `VARIANT_COUNT`, `EXAMPLE_COUNT`, `ordinal` and the `seen` array. S-07 takes ordinal 13.
- `src-tauri/src/new_assignments/mod.rs`: S-07 adds one `.chain(&items.hidden_merge_requests)` in `all_items` and one loop in `mark_fresh`. If S-03 generalises or moves these functions, re-apply both, or a hidden MR becomes NEW or loses its seen key.
- `src-tauri/src/lib.rs`: three lines after `dismiss_gitlab_change`. S-03 appends elsewhere.
- `context/foundation/prd.md`: the MS-33 paragraph after the MS-35 line. An MS-30 line may be added at the same place; keep both.
- `src/todo/TodoView.tsx`: `changedGitlab` (about :238) and `gitlabHidden` (about :431). This conflicts only if S-03 edits the same lines.
- `src-tauri/src/bin/fake_cli.rs`: the notes routes (`MrNotes`, `MR_NOTES_KEYS`). This conflicts only if S-03 adds GitHub scenarios next to them.
