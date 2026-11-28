<!-- PLAN-REVIEW-REPORT -->
# Plan Review: S-06 "Do review": GitLab merge requests awaiting my review

- **Plan**: `context/changes/gitlab-review-requests/plan.md`
- **Mode**: Deep
- **Date**: 2026-11-27
- **Verdict**: REVISE (targeted fixes, all applied; SOUND after fixes)
- **Findings**: 0 critical, 4 warnings, 1 observation

Run non-interactively. Every finding was triaged by the reviewing agent against the user's binding decisions (in `change.md`). None of them contradicts those decisions.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

The plan's paths all exist: `cli/mod.rs`, `bin/fake_cli.rs`, `gitlab/mod.rs`, `gitlab/fetch.rs`, `gitlab_refresh.rs`, `new_assignments/{mod,rules,notice}.rs`, `todo/{commands,mod}.rs`, `src/gitlab/{GitlabView.tsx,types.ts}`, `src/todo/TodoView.tsx` and `context/foundation/prd.md`.

The symbols it relies on also exist:

- `fetch_assigned`, `filter_shown`, `shape_items`, `reshape_kept`;
- `keys_of`, `items_for`, `mark_fresh`, `record_round`, `plan_round`, `fresh_since`, `notices_for`;
- `find_gitlab_item`, `reconcile_gitlab`;
- `VARIANT_COUNT=10`, `EXAMPLE_COUNT=14`, the seen array, `with_user==4`, `ALLOWED_ENDPOINTS`, `endpoint_allowed`;
- `Flag tone="info"`.

There is no plan-brief and no `lessons.md`. Progress and the phases are consistent: there are 5 phases, and the rows match the success criteria.

## What the review verified (no finding)

- **The derived marker `"<account-key>:reviews"` is within the rules.**
  - **Schema.** `assignment_accounts` is `account TEXT PRIMARY KEY, seeded_at_ms INTEGER NOT NULL`, with no CHECK (`store/schema.rs:132-135`). No foreign key points at a marker unless a `seen_assignments` row uses it, and the plan forbids that. So no migration is needed.
  - **AGENTS.md app-data list.** The row has the same shape as today's per-account seeding marker, which belongs to the "seen-assignment keys" records. The key stays opaque: `account_key` is 64 lowercase hex characters from SHA-256 (`gitlab/mod.rs:28-36`), plus a fixed suffix, with no host or user.
  - **Other readers.** Only `record_round` reads the table (`new_assignments/mod.rs:134`, `:178`). The crash test reads only the main marker.
- **The failure policy holds the hard rule.** The review call is the third `?` in `fetch_assigned`, so any failure gives `Err` with no partial list. On a failed round:
  - S-04's `next_snapshot` keeps the whole `AssignedItems`;
  - `reconcile_input` and `notify_input` run only on `GitlabState::Ok` (`gitlab_refresh.rs:721-737`).

  So a failed review call can never tick a To-do entry or seed the seen set from a partial list. "Never an empty list in place of a failed source" holds.
- **The To-do reconcile gives no wrong ticks.**
  - `present` in `reconcile_gitlab` (`todo/mod.rs:577-583`) gains `review_requests`.
  - The input is the unfiltered full list (`reconcile_input`).
  - The Rust dedupe means no key is in two lists.
  - When a reviewer is removed from a still-open MR, the entry is ticked "closed in source". This matches today's behaviour when the user is unassigned, and the plan documents it.
- **Phases compile and pass their gates alone.**
  - Phase 1: the variant and the fake-cli arms are self-contained.
  - Phase 2: the model, fetch, filter and TS fields have no TS literal sites.
  - Phase 3: the `record_round` signature change touches about 30 test call sites, all mechanical.
  - Phases 4 and 5 are independent.

  No phase needs a later phase in order to compile. Between phases 2 and 4 the review list is not yet reconciled. That causes no harm, because "+" cannot reach review MRs before phase 4.
- **The baseline logic is correct.**
  - `fresh_since` is "first_seen > mark" (`rules.rs:88-98`), so `first_seen = min(mark, now)` is never fresh.
  - `plan.new` minus the baseline keys removes the burst.
  - A brand-new account already seeds silently.
- **fake-cli.**
  - Every scenario that reaches a third call either falls through to `gitlab_assigned_body` (`gitlab-assigned`, the notes scenarios, `gitlab-closed-issues-no-network`) or fails earlier. So the existing fetch tests stay green once the review body arm exists.
  - `with == without` (`tests/gitlab_fetch.rs:173`) still holds.

## Findings

### F1 — Merge point with S-02 in `GitlabView.tsx` missing

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Implementation Approach, the "Shared files and expected merge points" table
- **Detail**: The table names no overlap with S-02, and `src/gitlab/GitlabView.tsx` is missing from it. But S-02's plan (`slice/github-refresh-interval`, its plan.md, around lines 184-191 and 272) moves the local `retryText` out of `GitlabView.tsx` into `src/components/time.ts`. It also swaps `formatClock` for `retryText` in the same file's `../components` import list. S-06 phase 5 edits `GitlabView.tsx` heavily.
- **Fix**: Add a table row for `src/gitlab/GitlabView.tsx`: S-02's import change, S-06's `ReviewBlock`, `reviewTag`, `viewOptions` and header edits, and the resolution "keep both; never re-add an unused `formatClock`".
- **Decision**: FIXED (row added)

### F2 — Mechanical-update list misses `FilteredOut` literals, the pinned snapshot JSON and the borrow order

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Critical Implementation Details ("Field churn"); Phase 2 §2 Fetch
- **Detail**:
  - The plan lists only the `GitlabItem` and `AssignedItems` literals.
  - `FilteredOut` has five full literals in tests that break with a new field: `gitlab/mod.rs:1907`, `gitlab_refresh.rs:956`, `:1012`, `:1090` and `:1396`.
  - `snapshot_serializes_for_the_webview` (`gitlab/mod.rs:2079`) pins the exact JSON, including `"filteredOut":{"mergeRequests":0,"issues":0,"hiddenIssues":0}`. So the plan's claim that "existing tests stay unchanged in their assertions" is not quite true there.
  - The fetch contract writes `AssignedItems { merge_requests, …, review_requests: split_review_requests(&mut merge_requests, reviews), … }`. That borrows `merge_requests` after the struct literal has moved it, so it does not compile.
- **Fix**: Name the five `FilteredOut` literals and the JSON assertion. Have the fetch compute `let review_requests = split_review_requests(&mut merge_requests, reviews);` before the literal.
- **Decision**: FIXED

### F3 — The baseline upsert could set `last_seen_ms` to an old mark

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details (baseline ordering); Phase 3 §3 step 4
- **Detail**: Today's upsert binds one parameter to both columns: `VALUES (?1, ?2, ?3, ?4, ?4)` (`new_assignments/mod.rs:184-188`). The plan says baseline keys are "inserted with `baseline_first_seen(mark, now_ms)`". If that value also goes into `last_seen_ms`, a baseline key whose mark is more than 30 days old is forgotten on its first absent round (`plan_round`'s forget rule, `rules.rs:115-119`). It then re-notifies when it returns.
- **Fix**: Specify `first_seen_ms = baseline_first_seen(mark, now_ms)` and `last_seen_ms = now_ms`, with separate parameters, and add a test asserting a baseline row's `last_seen_ms == now_ms`.
- **Decision**: FIXED

### F4 — Manual rows do not cover the To-do picker or the NEW flag

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 5 Manual Verification; Progress 5.5-5.12
- **Detail**:
  - Phase 4 changes the "Do zrobienia" picker (`TodoView.tsx` `gitlabItems` and `gitlabFilteredOut`). The UI has no test runner, so only a manual check can see it, and no Manual row covers it.
  - The NEW flag on a review row (`mark_fresh` and the `isNew` flag) also has no manual check. Row 5.9 checks only the toast.
- **Fix**: Add Manual items 9 and 10 and Progress rows `5.13` and `5.14`. Existing rows keep their indices and titles.
- **Decision**: FIXED

### F5 — An unsupported `reviews_for_me` stops the whole GitLab source

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: What We're NOT Doing (no fallback); Manual 5.5
- **Detail**:
  - The all-or-nothing fetch is the right call: it gives no partial list and no wrong ticks.
  - But if the work instance answers `scope=reviews_for_me` with HTTP 400, `classify` maps it to `Failed` (`cli/classify.rs:44-57`). The whole GitLab source then shows "Error", and the assigned lists stop refreshing. They stay kept and dimmed.
  - The plan accepts this as a Manual check. It does not say that the check gates the release, or what the workaround is.
- **Fix**:
  - State that Manual 5.5 must pass before the release PR to `main`.
  - State the workaround: set `gitlab.user`, which switches every call to its `*_username` form, including `reviewer_username`.
- **Decision**: FIXED (note added to the "no fallback" bullet and to Manual item 1)

## Triage summary

- **Fixed**: F1, F2, F3, F4 and F5 (5).
- **Skipped, accepted or dismissed**: none.
- **Verdict after fixes**: SOUND.

Not changed, by the caller's instruction: `change.md` keeps `status: planned`. The skill would set `plan_reviewed`, but this phase commits only plan.md and this review file.
