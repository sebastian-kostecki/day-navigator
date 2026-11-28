<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: S-06 "Do review": GitLab merge requests awaiting my review

- **Plan**: context/changes/gitlab-review-requests/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4, 5
- **Date**: 2026-11-28
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 6 observations
- **Diff**: 37f7ba8..807b06b (721b794 p1, 557da2a p2, 8fc0730 + 765f9d1 p3, df2be1e p4, 807b06b p5)
- **Mode**: non-interactive. The reviewer triaged each finding (fix / accept / skip) with a one-line reason. No code was changed by this review.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | WARNING |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Summary

The implementation matches the plan in almost every item:

- **CLI and guard test.** The `CliCall::GlabReviewMergeRequests` variant is read-only, has fixed arguments, uses `assigned_args` and is covered by the guard. The counts are 11/16, ordinal 10, `with_user` 5, and the negative `reviewer_username=` case is refused.
- **Login and fixtures.** `GitlabUser::parse` lets only `[A-Za-z0-9._-]` into `reviewer_username=`. fake-cli uses invented data on `gitlab.example.invalid` and has the two new scenarios.
- **Data model.** `split_review_requests` dedupes by id and flags both sides, with unit tests. The fetch makes the review call third, with `?`, so there is no partial list. The filter, the kept list and the reshaped list carry `review_requests`. The TS mirror matches the camelCase serde fields.
- **`record_round`.** The baseline runs inside one `store.write`:
  - the account marker is inserted before the `:reviews` marker and before the seen rows, so the foreign key holds;
  - `first_seen` and `last_seen` are separate parameters, and `last_seen = now`;
  - baseline keys are dropped from `new`.
- **Notices and To-do.** The notice title, the To-do lookup and reconcile (`present` includes `review_requests`), the picker, `ReviewBlock`, the row flag, the header count and the PRD note all follow the plan.
- **Not in the diff:** `lib.rs`, `store/`, `settings.rs`, `github_refresh.rs`, `src/github/`, `source_refresh.rs`, AGENTS.md and `roadmap.md`.
- **Shared files.** The edits to shared files are small and additive. The reflows come from rustfmt or Prettier. `retryText` and the `../components` imports in `GitlabView.tsx` are untouched, so S-02's move will apply cleanly.

**AGENTS.md Hard Rules check.**

| Rule | Result |
|---|---|
| No secrets | Clean |
| `glab` read-only through a fixed `CliCall` covered by the guard test | Clean |
| No work data in tests or fixtures | Clean |
| Logs | Clean: errors are by kind; the new label is `glab:review-mrs` |
| Never an empty list in place of a failed source | Clean: the third call fails the round; `next_snapshot` keeps the list, dimmed |
| App writes only its own state | See F3. The `{account}:reviews` row holds no work data, but the AGENTS.md list does not describe it |

**Process note (not a code finding).** Commit 8fc0730 (phase 3) has no `Co-Authored-By` trailer. History must not be rewritten, so this is recorded only.

**Not changed by this review.** `change.md` was deliberately left at `status: planned`. The orchestrator's brief allows this commit to contain only the review file. The orchestrator should set `status: impl_reviewed` together with the fixes.

## Success criteria run (2026-11-28, worktree at 807b06b)

| Command | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (no warnings) |
| `cargo test` | pass: 850 passed, 0 failed |
| `npm run lint` | pass |
| `npm run format:check` | pass |
| `npm run build` | pass |
| `grep -n "MS-35" context/foundation/prd.md` and the "Poza v1" line without "reviewerem" | pass (`prd.md:227`, `:242`) |

Manual rows 5.5 to 5.14 are unticked and pending. They are for the user after integration. 5.5 is a hard gate before the release PR to `main`; see F2.

## Findings

### F1 — Planned event-log test for review keys is missing

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/tests/new_assignments.rs:515 (end of the review tests block)
- **Detail**: Phase 3 §6 and the Testing Strategy (plan.md:416, :633) require an integration test asserting one `assigned` event per new review key. No such test exists, and `tests/events.rs` is untouched. The behaviour holds by construction: `all_items` chains `review_requests` (`new_assignments/mod.rs:68`), and `record_seen` logs `items_for(full, &round.new)` (`gitlab_refresh.rs:776-785`). Nothing pins it, though, and nothing pins that a baseline round logs zero events (that depends on `round.new` having the baseline keys removed).
- **Fix**: Add one real-store test to `src-tauri/tests/new_assignments.rs`, following the existing `seed_pre_release` and `first_review_round_after_release_is_silent_and_seeds_the_review_marker` tests and the `events::record` usage in `src-tauri/tests/events.rs`. Its steps:
  1. Seed a pre-slice account.
  2. Build an `AssignedItems` with one invented review-only MR in `review_requests` (`review_requested: true`). Run `record_round(store, account, &keys_of(&items), &review_only_keys(&items), now)` and assert that `round.new` is empty (the baseline).
  3. Map `items_for(&items, &round.new)` to `NewEvent::Assigned { item_kind, reference, title }` and record it with `events::record` (it must store 0 events).
  4. Add a second invented review-only MR and run the next round. Assert that `round.new` holds exactly that key, then record its events and assert that `events::list` for today returns exactly one `assigned` event with that reference.
  Invented data only.
- **Decision**: FIX — the plan names this test explicitly, and it is cheap and pins the event-log contract.

### F2 — Unsupported `scope=reviews_for_me` stops the whole GitLab source

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab/fetch.rs:34-38
- **Detail**: The review call uses `?`. A GitLab instance that rejects `scope=reviews_for_me` (HTTP 400) therefore fails every round. The whole source then stays in "Error": the assigned lists are kept and dimmed, there are no FR-004 notices and no To-do reconcile. The recently-closed section does it the other way: its failure stays inside the section (`gitlab_refresh.rs:565-571`). The plan accepted this risk on purpose (plan.md:85). All-or-nothing also protects the reconcile from a wrong tick, and setting `gitlab.user` works around it.
- **Fix A ⭐ Recommended**: Keep all-or-nothing and treat Manual row 5.5 as a hard gate before the release PR to `main`.
  - Strength: It matches the plan and the "no partial list" policy (`fetch.rs:16-18`). The reconcile can never tick a review-picked entry on a partial round.
  - Tradeoff: An older GitLab breaks the whole source until `gitlab.user` is set.
  - Confidence: HIGH — the trade-off is documented in the plan, and the workaround exists.
  - Blind spot: The work instance's GitLab version is not verified yet.
- **Fix B**: Contain a review-call failure inside the section, like `fetch_closed_section`, and skip both the To-do reconcile and the review-key notices on such a round.
  - Strength: The assigned lists stay healthy on an older GitLab.
  - Tradeoff: New partial-round state, more code paths in reconcile and FR-004, and a plan change.
  - Confidence: MED — it touches several rules in this slice.
  - Blind spot: How the UI should show a section-only error.
- **Decision**: ACCEPTED (Fix A) — the plan already decided this. Manual check 5.5 must pass before the PR to `main`. No code change.

### F3 — `{account}:reviews` marker row is not described by the AGENTS.md app-data list

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture
- **Location**: src-tauri/src/new_assignments/mod.rs:89 (`review_marker_key`); AGENTS.md:11
- **Detail**: Phase 3 stores a second row per account in `assignment_accounts`, keyed `"<64-hex account key>:reviews"`, holding one instant.
  - **What fits.** It is opaque: no host, no user and no work data. It is the same kind of seeding marker as the existing account row. Today no other code reads, counts, prunes or iterates `assignment_accounts`; a grep finds only `new_assignments/mod.rs`, `schema.rs` and tests. It cannot collide with a real 64-hex key.
  - **What does not.** AGENTS.md:11 lists only "seen-assignment keys (opaque account key, kind, id)" and names no seeding markers at all. The column's v7 description in `store/schema.rs:124-129` says it holds "the opaque key from `gitlab::account_key`". Any future code that treats `assignment_accounts` rows as accounts would see a phantom account; examples are a forget-account feature, an account count, or a new foreign key from `todo_items.gitlab_account`.
  - **Verdict.** It fits AGENTS.md in spirit, not by the letter.
- **Fix**: After S-01 has merged into the integration branch (S-01 edits this same AGENTS.md line), extend the app-data list in AGENTS.md:11. Change "seen-assignment keys (opaque account key, kind, id)" to "seen-assignment keys (opaque account key, kind, id) and their seeding markers (the opaque account key, or that key with a `:reviews` suffix for the review set, and one instant)".
  - Strength: It makes the Hard Rule describe what is actually stored. Doing it at integration avoids a textual conflict with S-01.
  - Tradeoff: The rule text is wrong until integration.
  - Confidence: HIGH — a one-phrase doc edit.
  - Blind spot: A cleaner model (a nullable `reviews_seeded_at_ms` column) needs a migration. S-01 owns this wave's migration, so it is left for later if it is ever needed.
- **Decision**: FIX (at integration, after S-01 lands) — a doc edit to AGENTS.md:11 only. Do not edit `store/schema.rs` (a shipped migration, and S-01 territory).

### F4 — "Do review" flag uses the `info` tone, against the Flag tone doc

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/GitlabView.tsx:213; src/components/Flag/Flag.tsx:5-6
- **Detail**: `<Flag tone="info">Do review</Flag>` is rendered next to "Nowe". `Flag.css:13-16` styles `info` and `new` the same blue. `Flag.tsx`'s doc says `info` means "closed in the source". The text still tells the two flags apart, but the documented meaning of the tone no longer holds.
- **Fix**: Keep the tone and update the `FlagProps.tone` doc comment in `src/components/Flag/Flag.tsx:5-6` to read "info = blue (closed in the source; Do review, an assigned MR that also awaits the user's review)". Comment only; no CSS change.
- **Decision**: FIX — a one-line comment that keeps the component contract honest. Changing the colour would need a design-system decision.

### F5 — Phase 3 doc comment in `fetch.rs` not re-wrapped

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/gitlab/fetch.rs:16-18
- **Detail**: The updated comment "Merge requests, then issues, then the review list…" left line 17 at 94 characters against 77 and 68 around it. rustfmt does not wrap comments.
- **Fix**: Re-wrap the three comment lines to the surrounding width (≤ 80 columns) without changing the wording.
- **Decision**: FIX — cosmetic, one comment block.

### F6 — Progress rows of phases 1, 2, 4 and 5 lack commit SHAs

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: context/changes/gitlab-review-requests/plan.md:672-715
- **Detail**: The Progress convention (plan.md:666) says to append ` — <commit sha>` when a step lands. Only the phase 3 rows carry one (8fc0730).
- **Fix**: Append the SHA to every ticked Automated row and leave the titles unchanged:
  - ` — 721b794` to rows 1.1–1.4;
  - ` — 557da2a` to rows 2.1–2.6;
  - ` — df2be1e` to rows 4.1–4.5;
  - ` — 807b06b` to rows 5.1–5.4.
- **Decision**: FIX — trivial bookkeeping that `/10x-status` and archive rely on.

### F7 — No direct test that a filtered-out review key still notifies

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/new_assignments/tests.rs:523
- **Detail**: Phase 3 §7 asks for a unit test that a review-only key filtered out of the shown list still notifies. `review_items_join_keys_items_and_flags` only shows that `keys_of` includes review keys; no filter is involved. The behaviour holds because `record_seen` passes the full, unfiltered list (`gitlab_refresh.rs:764-765`), the same path the assigned items already rely on.
- **Fix**: none.
- **Decision**: SKIP — `record_seen` needs an `AppHandle`, so a filter test here would only restate that `keys_of` reads its argument. F1's new test plus Manual row 5.9 cover the behaviour.

### F8 — Assigned MR that also awaits review is titled "nowy MR do review"

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/new_assignments/notice.rs:45-47
- **Detail**: A new MR in the assigned list that is also flagged `review_requested` gets the review title, although assignment is arguably the stronger role.
- **Fix**: none.
- **Decision**: ACCEPTED — an explicit plan default (plan.md:405). It is harmless: one notice either way.

### F9 — Combined-notice test and fake-cli scenario placement (minor)

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/new_assignments/tests.rs:546; src-tauri/src/bin/fake_cli.rs:649-675
- **Detail**: There are two minor points:
  - `mixed_review_and_plain_items_keep_the_combined_notice` asserts only the count and the title, not the lines. `line()` does not depend on `review_requested`, and the existing combined tests pin the lines.
  - The two new fake-cli scenarios sit between `gitlab-mrs-ok-issues-no-network` and `gitlab-closed-issues-no-network`, not at the end. A textual conflict with S-01 is possible but trivial, because both sides are additive.
- **Fix**: none.
- **Decision**: SKIP — no behaviour risk. A merge conflict, if any, is resolved by keeping both sides.
