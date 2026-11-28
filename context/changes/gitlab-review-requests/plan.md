# S-06 "Do review": GitLab merge requests awaiting my review — Implementation Plan

## Overview

Slice S-06, change id `gitlab-review-requests`, GitHub issue #35. It is anchored to roadmap item **MS-35**, with PRD refs FR-001, FR-002 and FR-004. There is no new FR.

"Praca (GitLab)" gets a **"Do review"** section. It lists the open merge requests where the user is a reviewer:

- under the same settings filter (FR-002) as the rest;
- with "+" to "Do zrobienia" on each row;
- with a Windows notice for a new review request, using the FR-004 mechanism and threshold and bypassing the filter;
- with no migration: the existing `seen_assignments` keys with kind `mergeRequest` are reused;
- offline, it behaves like the rest of the GitLab source.

The data comes from one new read-only `glab api` call.

Research (source-backed, read in full): `context/changes/gitlab-review-requests/research.md`.

This plan was written non-interactively. Every choice beyond the user's decisions is marked **(default chosen by agent)**.

## Current State Analysis

- **GitLab round.** The GitLab round fetches two lists in `fetch_assigned` (`src-tauri/src/gitlab/fetch.rs:19-39`): assigned MRs, then assigned issues. The first failure ends the round and leaves no partial list.
- **Failed rounds.** S-04 keeps the whole `AssignedItems` on a failed round (`gitlab/mod.rs` `next_snapshot`), and the UI dims `.gitlab-lists`.
- **`AssignedItems`** (`gitlab/mod.rs:111`) has `merge_requests`, `issues` and `hidden_issues`. Several places walk it by field name:
  - `all_items`/`keys_of`/`items_for`/`mark_fresh` (`new_assignments/mod.rs:51-100`);
  - `filter_shown` and `shape_items` (`gitlab_refresh.rs:277-312`), and the round's shown list (`gitlab_refresh.rs:604`);
  - `find_gitlab_item` (`todo/commands.rs:33-46`) and `reconcile_gitlab` (`todo/mod.rs:569-584`);
  - in the UI, `Lists` and the header summary (`src/gitlab/GitlabView.tsx:451-471`, `:606`), and the To-do picker (`src/todo/TodoView.tsx:409-417`).
- **CLI calls.** `CliCall` (`cli/mod.rs:328`) has 10 variants. The guard test pins `VARIANT_COUNT = 10`, `EXAMPLE_COUNT = 14`, the exhaustive `ordinal()`, the seen-counts array `[1,1,1,1,2,2,1,1,2,2]` and `with_user == 4`. It also re-implements the endpoint validators (`ALLOWED_ENDPOINTS`, `endpoint_allowed`).
- **Identity.** The app never learns the logged-in username. The only login is the optional `gitlab.user` setting.
- **FR-004.** The FR-004 seen set is `seen_assignments(account, kind, item_id, first_seen_ms, last_seen_ms)`. Each account has a seeding marker in `assignment_accounts(account, seeded_at_ms)`, and `seen_assignments.account` references it.
  - `record_round` (`new_assignments/mod.rs:126-228`) plans with `plan_round` (`rules.rs:101`) in one `Store::write`.
  - An account's first round seeds silently.
  - `notices_for` (`notice.rs:38`) titles single notices by kind. More than `MAX_NOTICE_LINES` (3) new items give one combined notice.
- **Hidden issues** are issue-only, so S-06 does not touch them.

## Desired End State

- **The call.** `CliCall::GlabReviewMergeRequests(GitlabHost, Option<GitlabUser>)` exists. It is read-only, has fixed arguments and is covered by the guard test. Its two shapes are:
  - `merge_requests?scope=reviews_for_me&state=opened&with_labels_details=true&per_page=100` without a user;
  - `merge_requests?scope=all&state=opened&reviewer_username=<login>&with_labels_details=true&per_page=100` with one.
- **The fetch.** `fetch_assigned` runs it as the third call. A failure fails the whole round, so the round is calm "unavailable" without VPN and S-04's kept, dimmed list covers the section.
- **The data model.**
  - `AssignedItems.review_requests` holds the **review-only** open MRs.
  - An MR that is both assigned and awaiting review stays only in `merge_requests`, flagged `review_requested = true`.
  - Every review item also carries `review_requested = true`.
- **Filter and NEW flags.** The settings filter applies to `review_requests`, which is counted in `FilteredOut.review_requests`. NEW flags (`is_new`) are set on review items too.
- **Notices.**
  - A new review-only MR gives one FR-004 notice with the title `Day Navigator — nowy MR do review`, using the same threshold and the same combined notice.
  - It is logged in today's event log as `assigned`.
  - The settings filter never suppresses it.
  - An MR in both lists is one key, so it gives one notice at most.
- **First round after release.** It is silent for review MRs that are already open: no notice and no NEW flag. This works through a derived marker row in `assignment_accounts`, with no schema change.
- **"Do zrobienia".**
  - "+" works on "Do review" rows.
  - The reconcile counts `review_requests` as present, so such an entry is ticked only when the MR leaves the open lists.
- **UI.**
  - "Praca (GitLab)" shows a titled "Do review" panel between the main list and "Zamknięte". It is hidden when the type switch is "Zgłoszenia".
  - Assigned MRs that also await review show a "Do review" flag.
  - The header adds "· K do review".
- **PRD.** The PRD no longer lists reviewer MRs under "Poza v1" and carries a one-line MS-35 note. No FR number is added or changed.

**Verify.** All gates are green after each phase. The Manual rows at the end are for the user after integration.

### Key Discoveries:

- `fetch.rs:16-18`: the "no partial list" policy. Adding the review call as the third `?` call keeps it.
- `next_snapshot` keeps `prev.items` (the whole `AssignedItems`) on a failed round. So the new field is kept and dimmed offline with no new code. Only `shape_items` and `reshape_kept` must copy it (`gitlab_refresh.rs:298-312`).
- `ItemKey{kind, id}` uses GitLab's global MR id, so the same MR has one key in both lists. `plan_round` and `items_for` already dedupe by key (`rules.rs:101-126`, `new_assignments/mod.rs:70-89`).
- `fresh_since` (`rules.rs:88`) is "fresh iff `first_seen_ms > mark`". A baseline key stored with `first_seen_ms <= mark` is therefore never fresh, with no special case.
- `assignment_accounts` has no FK pointing at it from a marker row. Only `record_round` reads it (`new_assignments/mod.rs:134`, `:178`). `seen_assignments.account REFERENCES assignment_accounts(account)` (`store/schema.rs:137`), so the real account row must still exist before its keys are upserted.
- `reconcile_gitlab` ticks every open GitLab To-do entry absent from `present` (`todo/mod.rs:577-584`). Leaving out `review_requests` is the most dangerous silent bug in this slice.
- `GitlabRow` renders hide buttons only for `kind === "issue"` (`GitlabView.tsx:288`), so MR rows get only "+" (`TodoAddButton`). No hiding is added.
- fake-cli's `endpoint()` (`bin/fake_cli.rs:66`) matches any `merge_requests?` as assigned MRs. The review arm must come before that generic arm and after the closed arms.
- There are 38 `GitlabItem { … }` literals and about 15 `AssignedItems { … }` literals in `src/` and `tests/`. Adding a field touches them all mechanically. `AssignedItems` derives `Default`; `GitlabItem` does not.

## What We're NOT Doing

- **No hiding of review MRs.** It comes with S-07 (hiding GitLab merge requests) in a later wave. There is no hide button, no `hidden_*` table change and no "Pokaż ukryte" count change.
- **No migration and no `store/schema.rs` change.** S-01 owns this wave's migration (v15).
- **No new `events.kind`.** Review requests log as `assigned`, shown as "Nowe przypisanie".
- **No "recently closed" review MRs.** "Zamknięte" stays assigned-only.
- **No reviewer-state filtering.** An MR the user already approved stays in "Do review" while it is open.
- **No `glab api user` call** to learn the username. No `scope=reviews_for_me` fallback logic: an unsupported scope surfaces as the source error. That is a Manual check. Under the all-or-nothing fetch it would stop the whole GitLab source (assigned lists kept and dimmed under "Error"); the workaround is setting `gitlab.user`, which switches every call to its `*_username` form.
- **No remembered collapse state** for the section. That would need a migration.
- **No new Tauri command, no `lib.rs` change, no settings or `settings.rs` change, no change to `github_refresh.rs`, `src/github/` or `source_refresh.rs`.**
- **No AGENTS.md edit (default chosen by agent).** The derived review marker is one more opaque `assignment_accounts` seeding-marker row, the same kind of record that exists today. S-01 edits the app-data list in this wave, and S-06 stays off it to avoid a conflict.
- **No roadmap edit.** `context/foundation/roadmap.md` is left to the orchestrator, so the "planning" status sync is skipped here.

## Implementation Approach

The slice adds one GitLab source with one round and a third fetch. It is not a new `SourceCell`.

1. **Phase 1:** add the call and its test fixtures, with nothing using it yet.
2. **Phase 2:** extend the model, the fetch, the Rust dedupe rule, the filter and the kept list, with the TS mirror in the same commit.
3. **Phase 3:** extend the FR-004 seen set, the notices and the silent baseline.
4. **Phase 4:** extend the "Do zrobienia" lookup, the reconcile and the picker.
5. **Phase 5:** add the UI section and the PRD note.

Domain rules live in Rust with unit tests: the dedupe, the baseline, the notice title and the reconcile presence. The UI only displays.

**Shared files and expected merge points (wave 2).**

| File | Other slice | What S-06 changes | Resolution |
|---|---|---|---|
| `src-tauri/src/cli/mod.rs` | S-01 (new `gh api` `CliCall` variants and guard) | Appends one variant and its arms in `tool()`, `args()` and `name()`; two `examples()` entries; consts; `VARIANT_COUNT` 10→11, `EXAMPLE_COUNT` 14→16, `ordinal()` arm `=> 10`, seen array plus `2`, `with_user` 4→5, one `ALLOWED_ENDPOINTS` entry, one prefix in `endpoint_allowed` | Textual and additive. Keep both sides, sum the counts and renumber `ordinal()` so each variant is unique. `cargo test` catches a wrong sum. |
| `src-tauri/src/bin/fake_cli.rs` | S-01 (GitHub endpoints and scenarios) | New `Endpoint::ReviewMergeRequests` and its matcher arm, a body arm in `gitlab_assigned_body`, two new scenarios | Additive. Keep both. |
| `src/todo/TodoView.tsx` | S-01 (possibly, near the GitHub picker items) | `gitlabItems` and `gitlabFilteredOut` expressions (two lines) | Keep both. |
| `src/gitlab/GitlabView.tsx` | S-02 (moves the local `retryText` to `src/components/time.ts`; the `../components` import list swaps `formatClock` for `retryText`) | `ReviewBlock`, `reviewTag` on `TableRow`/`GitlabRow`, the `viewOptions` input, the header summary | Keep both. Leave the import list as S-02 leaves it (never re-add an unused `formatClock`); add only what `ReviewBlock` needs. `npm run build` catches an unused import. |
| `context/foundation/prd.md` | (unknown) | Non-Goals line "Poza v1" and one note after the autostart note | Keep both notes. |
| `src-tauri/src/gitlab_refresh.rs`, `gitlab/*`, `new_assignments/*`, `todo/*` | none expected | S-06 only | — |
| `lib.rs`, `store/schema.rs`, `settings.rs`, `github_refresh.rs`, `src/github/`, AGENTS.md | S-01, S-02 | **untouched by S-06** | — |

S-06 keeps its edits to shared files small and local. It appends variants and arms, and does not reorder or reformat unrelated code.

## Critical Implementation Details

- **Field churn.** Adding `GitlabItem.review_requested` and `AssignedItems.review_requests` breaks every struct literal: about 38 `GitlabItem` and about 15 `AssignedItems` literals in `src/` and `tests/`.
  - In the `GitlabItem` literals, add `review_requested: false`.
  - In the `AssignedItems` literals, add `review_requests: Vec::new()` (or `vec![]`).
  - The parser (`gitlab/mod.rs:346`) sets `review_requested: false`.
  - `FilteredOut` has no `..Default::default()` in its literals: add `review_requests: 0` in the five test literals (`gitlab/mod.rs:1907`, `gitlab_refresh.rs:956`, `:1012`, `:1090`, `:1396`).
  - `snapshot_serializes_for_the_webview` (`gitlab/mod.rs:2079`) pins the exact JSON: its `filteredOut` object gains `"reviewRequests":0` (the only intended assertion change in existing tests).
  - Do not reformat surrounding code.
- **Order of calls.** The order is MRs, issues, reviews. The existing `gitlab-mrs-ok-issues-no-network` scenario keeps failing at issues, before the review call. Every existing `gitlab-assigned` test must stay green, because the review body is separate and the assigned lists are unchanged.
- **Baseline ordering inside `record_round`.** The rule needs these facts from the same transaction: the main seeded flag, the review marker flag, the stored rows and the mark. Then:
  - Insert the main marker if `plan.seed`.
  - Insert the review marker when it is missing.
  - Upsert, using the baseline `first_seen_ms` for the baseline keys. Their `last_seen_ms` stays `now_ms`: the upsert today binds one value (`?4`) to both columns, so it gets a separate first-seen parameter. A `last_seen_ms` set to an old mark would let the forget rule drop the key after one absent round and re-notify it.
  - Compute `fresh` from the per-key `first_seen` actually written.
  - Remove the baseline keys from `new`.

  All of this is one `store.write`, so a crash leaves both markers and rows, or neither.

## Phase 1: Read-only review-MR call, guard and fake-cli

### Overview

Add the new `CliCall` with fixed arguments and cover it with the read-only guard test. Teach fake-cli to serve an invented review list and two failure scenarios. Nothing in production calls it yet.

### Changes Required:

#### 1. The call

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Add a GET-only call that lists the open MRs where the account (or the configured `gitlab.user`) is a reviewer. It mirrors `GlabAssignedMergeRequests`.

**Contract**:

- **Constants:**
  - `pub const REVIEW_MRS_ENDPOINT: &str = "merge_requests?scope=reviews_for_me&state=opened&with_labels_details=true&per_page=100";`
  - `pub const REVIEW_MRS_USER_PREFIX: &str = "merge_requests?scope=all&state=opened&reviewer_username=";`
  - Reuse `USER_SUFFIX`.
- **Variant:** `GlabReviewMergeRequests(GitlabHost, Option<GitlabUser>)` is appended after `GlabRecentlyClosedMergeRequests`, with a doc comment "GET of the open merge requests awaiting the user's review (read-only)".
- **Arms:**
  - `tool()` → `Tool::Glab`.
  - `args()` → `assigned_args(REVIEW_MRS_ENDPOINT, REVIEW_MRS_USER_PREFIX, host, user)`.
  - `name()` → `"glab:review-mrs"`.
- **`examples()`:** two entries, without and with the invented user.
- **`CliCall::ALL`:** not added, because the call is parameterised.
- **Guard test (`mod tests`):**
  - `VARIANT_COUNT = 11` and `EXAMPLE_COUNT = 16`.
  - `ordinal()` arm `CliCall::GlabReviewMergeRequests(..) => 10`.
  - Expected seen array `[1, 1, 1, 1, 2, 2, 1, 1, 2, 2, 2]`.
  - `with_user` matches `GlabReviewMergeRequests(_, Some(_))` and asserts 5.
  - The "ALL must stay parameterless" list includes the new variant.
  - `ALLOWED_ENDPOINTS` gains `REVIEW_MRS_ENDPOINT`.
  - `endpoint_allowed` accepts `REVIEW_MRS_USER_PREFIX` in its prefix list. The shape check stays: one `?`, four `&`, the same forbidden characters.
- **Guard case:** add one negative example, `reviewer_username=` with an extra `&state=closed`, which must be refused. This is the same style as the existing negative cases.

#### 2. fake-cli

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Serve an invented review list, so fetch tests can check parsing, the dedupe against the assigned list and the failure classes.

**Contract**:

- **`Endpoint::ReviewMergeRequests`.** `endpoint()` returns it for `merge_requests?` arguments containing `scope=reviews_for_me` or `&reviewer_username=`. The check comes after the two closed arms and before the generic `merge_requests?` arm.
- **Review body.** `gitlab_assigned_body` gets a `ReviewMergeRequests` body with invented data on `gitlab.example.invalid`:
  - two review-only MRs, for example id 6101 `acme/gadgets!21` "Review gadget cache" with label `backend`, and id 6102 `acme/sprockets!5` "Tune sprocket timeouts" with a milestone;
  - one MR identical to the assigned id 6002 `acme/widgets!34`, to test the dedupe.
- **New scenarios:**
  - `gitlab-reviews-no-network`: MRs and issues answer with the `gitlab-assigned` bodies, and the review call prints `GLAB_NO_NETWORK` and exits 1.
  - `gitlab-reviews-bad-scope`: MRs and issues answer, and the review call prints an invented `glab: 400 Bad Request (HTTP 400)` line and exits 1.
- **`gitlab-empty`** already answers `[]` for any endpoint, so it needs no change.

#### 3. Runner coverage

**File**: `src-tauri/tests/cli_runner.rs` (only if needed)

**Intent**: The `data` echo test iterates `CliCall::examples()`, so the new variant is covered automatically. Touch this file only if a count or variant list there is explicit.

**Contract**: none beyond existing tests passing.

### Success Criteria:

#### Automated Verification:

- Guard test passes with the new counts and refuses the negative review endpoint: `cargo test --lib cli::`
- Runner echo test covers the new examples: `cargo test --test cli_runner`
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean
- Full suite green: `cargo test`

---

## Phase 2: Review list in the round: fetch, dedupe, filter, kept list, TS mirror

### Overview

The round fetches the review list as its third call. A Rust rule splits it into review-only items and flags the assigned MRs that also await review. The FR-002 filter and the kept/re-shaped list carry it. The TS mirror gets the new fields in the same commit.

### Changes Required:

#### 1. Model

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Give the review section its own list and mark the reviewer role on items.

**Contract**:

- **`GitlabItem.review_requested: bool`** (serde `reviewRequested`): true on every review item and on an assigned MR that also awaits the user's review. The parser gives `false`.
- **`AssignedItems.review_requests: Vec<GitlabItem>`** (serde `reviewRequests`), documented as:
  - open MRs awaiting the user's review and not assigned to them;
  - empty in a list built before the review call;
  - never hidden.
- **`FilteredOut.review_requests: u32`.**
- **Dedupe rule.** New pure function `pub fn split_review_requests(merge_requests: &mut [GitlabItem], reviews: Vec<GitlabItem>) -> Vec<GitlabItem>`:
  - It sets `review_requested = true` on every assigned MR whose `id` appears in `reviews`.
  - It returns the reviews whose id is not an assigned MR id, each with `review_requested = true`.
  - The first occurrence wins on a repeated id, and the input order is kept.
- **Unit tests:** no overlap, full overlap, partial overlap, a repeated id within the reviews, and the flag being set on both sides.
- **Snapshot test:** extend the existing `next_snapshot` tests with one asserting that a failed round with an unchanged config keeps `items.review_requests`.

#### 2. Fetch

**File**: `src-tauri/src/gitlab/fetch.rs`

**Intent**: Run the review call third. Its failure fails the round, so there is never a partial list (the comment at `:16-18` is updated to say "three calls").

**Contract**:

- `fetch_assigned` runs `CliCall::GlabReviewMergeRequests(host, config.user)` after the issues and parses it with `parse_merge_requests`.
- It first runs `let review_requests = split_review_requests(&mut merge_requests, reviews);` (`merge_requests` becomes `let mut`), then builds `AssignedItems { merge_requests, issues, review_requests, hidden_issues: Vec::new() }`. Calling the split inside the literal would borrow `merge_requests` after it moved.

#### 3. Round, filter, kept list

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Carry the review list through the shown list, the FR-002 filter and S-04's re-shape of the kept list.

**Contract**:

- `filter_shown` applies `filter.apply(&shown.review_requests)` and fills `FilteredOut.review_requests`.
- `shape_items` and the successful-round shown list (`:604`) copy `full.review_requests`.
- The full list (`round_full`) stays unfiltered.
- **Unit tests:**
  - `filter_shown` removes and counts review items like MRs.
  - `shape_items` and `reshape_kept` keep the review items of a kept full list.

#### 4. TS mirror (same commit)

**File**: `src/gitlab/types.ts`

**Intent**: Mirror the new serde fields.

**Contract**:

- `GitlabItem.reviewRequested: boolean`.
- `AssignedItems.reviewRequests: GitlabItem[]`.
- `FilteredOut.reviewRequests: number`.
- Doc comments follow the existing style. No UI behaviour changes in this phase.

#### 5. Mechanical literal updates

**Files**: every `GitlabItem { … }` and `AssignedItems { … }` literal in `src-tauri/src/**` and `src-tauri/tests/**`

**Intent**: Compile with the new fields. There is no behaviour change in the tests.

**Contract**: `review_requested: false` and `review_requests: Vec::new()` (or `vec![]`).

#### 6. Fetch integration tests

**File**: `src-tauri/tests/gitlab_fetch.rs`

**Intent**: Prove the third call, the dedupe and the failure policy through fake-cli.

**Contract**:

- **New tests:**
  - Under `gitlab-assigned`, `review_requests` holds exactly `acme/gadgets!21` and `acme/sprockets!5`, all flagged, and `merge_requests` still has its existing items with `acme/widgets!34` flagged `review_requested`.
  - With a user, both endpoint forms are served and parsed.
  - `gitlab-reviews-no-network` gives `Err(GitlabError::Unavailable)` with no partial result.
  - `gitlab-reviews-bad-scope` gives the same error variant as the existing unknown-failure case: not `Unavailable` and not `LoggedOut`.
  - With a user, failures of the review call never show the user or the host. Extend the existing leak test.
- Existing tests stay unchanged in their assertions.

### Success Criteria:

#### Automated Verification:

- Dedupe unit tests pass: `cargo test --lib gitlab::`
- Round, filter and kept-list unit tests pass: `cargo test --lib gitlab_refresh::`
- Fetch integration tests pass, including the two new failure scenarios: `cargo test --test gitlab_fetch`
- TS mirror type-checks: `npm run build`
- `npm run lint`, `npm run format:check`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` are clean
- Full suite green: `cargo test`

---

## Phase 3: FR-004 notices, NEW flags and the silent first-round baseline

### Overview

Review items join the FR-004 input:

- The seen set gets one key per MR, whichever list it is in.
- New review-only MRs notify with their own title.
- Review items get NEW flags.
- The first successful round after release takes the already-open review MRs in silently, through a derived marker row with no schema change **(default chosen by agent: research option (a))**.

### Changes Required:

#### 1. Inputs and flags

**File**: `src-tauri/src/new_assignments/mod.rs`

**Intent**: Make every FR-004 consumer see the review list.

**Contract**:

- `all_items` chains `review_requests`, so `keys_of`, `items_for` and the event-log input all include it.
- `mark_fresh` sets `is_new` on `review_requests` too.
- New `pub fn review_only_keys(items: &AssignedItems) -> Vec<ItemKey>`: the keys of `review_requests` under the same id filter as `keys_of`.

#### 2. Baseline rule

**File**: `src-tauri/src/new_assignments/rules.rs`

**Intent**: Decide, without a store, which review keys the review set's first round takes in silently, and the `first_seen_ms` they get, so that they are never fresh.

**Contract**:

- `pub fn review_baseline(seeded: bool, reviews_seeded: bool, stored: &[SeenRow], review_only: &[ItemKey]) -> Vec<ItemKey>`:
  - It returns the keys of `review_only` that are not in `stored`, once each, in input order.
  - It returns them only when `seeded && !reviews_seeded`, and returns empty otherwise.
  - A brand-new account's seeding round is already silent.
- `pub fn baseline_first_seen(mark: Option<i64>, now_ms: i64) -> i64`:
  - With a mark, it returns `min(mark, now_ms)`, so `fresh_since` (`first_seen > mark`) never flags a baseline key.
  - Without a mark, it returns `now_ms`. Without a mark nothing is fresh, and the next mark is a round shown at or after this one.
- **Unit tests:**
  - not seeded gives empty;
  - seeded and reviews seeded gives empty;
  - seeded and not reviews seeded gives only the unknown review keys;
  - a stored key is never in the baseline;
  - a repeated key appears once;
  - `baseline_first_seen` with no mark, a mark before now and a mark after now.

#### 3. Recording a round

**File**: `src-tauri/src/new_assignments/mod.rs`

**Intent**: Apply the baseline inside the round's single write. The review set gets its own opaque seeding marker in the existing `assignment_accounts` table.

**Contract**:

- New `pub fn review_marker_key(account: &str) -> String`, which returns `format!("{account}:reviews")` **(default chosen by agent)**.
  - The account key is 64 lowercase hex characters, so the `:` suffix can never collide with a real account key. It stays opaque and carries no host or user.
  - No `seen_assignments` row ever uses it.
- `record_round(store, account, current, review_only, now_ms)` gains `review_only: &[ItemKey]`. `current` still holds every key, including the review-only ones.
- In the one write:
  1. Read `seeded` and `reviews_seeded` (the marker row for `review_marker_key(account)`), the rows and the mark.
  2. Run `plan_round`, then `review_baseline`.
  3. Insert the main marker when `plan.seed`, and insert the review marker whenever `reviews_seeded` is false (also on the seeding round).
  4. Upsert every current key. Baseline keys are inserted with `first_seen_ms = baseline_first_seen(mark, now_ms)` and `last_seen_ms = now_ms` (separate parameters; every other key binds `now_ms` to both, as today); the conflict rule is unchanged. A unit or integration test asserts a baseline row's `last_seen_ms == now_ms`.
  5. Compute `seen` from the `first_seen` actually written.
  6. Return `new = plan.new` minus the baseline keys.
- **Doc comment.** The module and function doc comments say this:
  - the marker row is a "review set seeded" flag;
  - like the account marker, it is never pruned;
  - a key moving between the assigned list and the review list is the same key, so it never notifies again.

#### 4. Caller

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Pass the review-only keys of the full, unfiltered list.

**Contract**:

- `record_seen` calls `record_round(s, account, &keys_of(full), &review_only_keys(full), now_ms)`.
- `SessionSeen::resolve` stays as is. Its fallback is per key, and its first session round is silent.

#### 5. Notice title

**File**: `src-tauri/src/new_assignments/notice.rs`

**Intent**: A single notice says when the new item is a review request. The threshold and the combined notice stay the same.

**Contract**:

- In the separate-notice branch, `ItemKind::MergeRequest` with `item.review_requested` is titled `format!("{APP_NAME} — nowy MR do review")` **(default chosen by agent)**. An assigned MR that also awaits review was flagged in phase 2, so it gets this title too.
- An MR without the flag keeps "nowy MR", and an issue keeps "nowe zgłoszenie".
- The combined title stays `nowe przypisania (N)` **(default chosen by agent: unchanged wording, fewer test changes; being added as a reviewer counts as an assignment here)**.
- **Unit tests:** a review MR's title, the mixed combined notice keeping the existing title and lines, and the existing tests unchanged.

#### 6. Event log

**File**: none (behaviour via `items_for`)

**Intent**: New review requests are logged as `NewEvent::Assigned`, shown on the dashboard as "Nowe przypisanie" **(default chosen by agent; a distinct kind would need a migration)**.

**Contract**: covered by an integration test (below) asserting one `assigned` event per new review key.

#### 7. Tests

**Files**: `src-tauri/src/new_assignments/tests.rs`, `src-tauri/tests/new_assignments.rs`, `src-tauri/tests/new_assignments_crash.rs`

**Intent**: Prove the baseline and the one-key rule against a real store with invented ids.

**Contract**:

- **Mechanical:** existing `record_round` call sites pass `&[]` for `review_only`. The crash harness keeps passing `&[]`, with assertions unchanged.
- **New integration tests (`tests/new_assignments.rs`):**
  - An account seeded before this slice, with no review marker, records a round with review-only keys. The result has no `new` and no `fresh`, the review marker now exists, and the rows were inserted. In the next round, a new review-only key is `new` and fresh after the mark.
  - A brand-new account seeds both markers silently.
  - An id present as an assigned MR and later only as a review MR, or the reverse, is never `new` again.
  - A key in `current` and in `review_only` gives one row.
  - A review-only key filtered out of the shown list still notifies. `keys_of` runs on the full list: a unit test in `new_assignments/tests.rs`.
- **New unit tests (`new_assignments/tests.rs`):** `keys_of`, `items_for` and `mark_fresh` include `review_requests`, and `review_only_keys` returns only review keys.

### Success Criteria:

#### Automated Verification:

- Baseline rule and notice unit tests pass: `cargo test --lib new_assignments::`
- Store-backed baseline and one-key tests pass: `cargo test --test new_assignments`
- Crash harness still green with the new signature: `cargo test --test new_assignments_crash`
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean
- Full suite green: `cargo test`

---

## Phase 4: "Do zrobienia": pick from "Do review" and reconcile safely

### Overview

"+" on a review row adds a GitLab To-do entry. The reconcile treats review MRs as present, so an entry picked from "Do review" is ticked "closed in source" only when the MR leaves both open lists. The To-do picker lists review MRs too.

### Changes Required:

#### 1. Lookup

**File**: `src-tauri/src/todo/commands.rs`

**Intent**: `add_todo_gitlab_item` must find a review-only MR in the shown snapshot instead of failing with `NotInGitlabList`.

**Contract**:

- `find_gitlab_item` searches `merge_requests` and then `review_requests` for `ItemKind::MergeRequest`. Issues are unchanged.
- The stored entry (kind, reference, title, URL, account) and the duplicate check are unchanged.
- **Unit test** in the existing `#[cfg(test)]` block: a review-only MR is found, and an unknown reference is still refused.

#### 2. Reconcile

**File**: `src-tauri/src/todo/mod.rs`

**Intent**: Prevent the silent wrong tick.

**Contract**:

- `reconcile_gitlab` builds `present` from `merge_requests`, `review_requests`, `issues` and `hidden_issues`.
- Its doc comment adds: "a merge request awaiting the user's review counts as present".
- **Unit tests (`todo/tests.rs`):**
  - An entry whose MR is only in `review_requests` is not ticked, and its title and URL are refreshed from it.
  - The same entry is ticked once the MR is in neither list. This is the existing "closed in source" behaviour, which also covers being removed as reviewer.

#### 3. Picker

**File**: `src/todo/TodoView.tsx`

**Intent**: The To-do picker offers review MRs like the other GitLab items, and its filter note counts them.

**Contract**:

- `gitlabItems` adds `...snapshot.items.reviewRequests`.
- `gitlabFilteredOut` adds `snapshot.filteredOut.reviewRequests`.
- Two expressions change, with no other edits.

### Success Criteria:

#### Automated Verification:

- To-do unit tests pass, including the review-only presence and tick cases: `cargo test --lib todo::`
- To-do integration tests stay green: `cargo test --test todo`
- `npm run lint`, `npm run format:check`, `npm run build` are clean
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean
- Full suite green: `cargo test`

---

## Phase 5: "Do review" section in "Praca (GitLab)" and the PRD note

### Overview

Show the review list as its own titled panel. Flag assigned MRs that also await review, and count review MRs in the header. Record in the PRD that MS-35 lifts the "reviewer MRs outside v1" Non-Goal, with no FR change.

### Changes Required:

#### 1. Section

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: Render a "Do review" `Panel` between `.gitlab-main` and `ClosedBlock`, inside `.gitlab-lists`, so S-04's stale dimming applies. It follows the `ClosedBlock` pattern.

**Contract**:

- New `ReviewBlock({ items, filteredOut, filterActive, selection, clear, now, actions })`. `Lists` renders it only when `kind !== "issue"`.
- **Rows:** `items.filter(matchesView)` sorted by `byUpdated`.
- **Panel:**
  - `title="Do review"`;
  - `meta={rows.length}`;
  - `footer={footerNote(filteredOut, before - after)}`.
- **Body:**
  - `ViewEmpty` when the view filter emptied it;
  - otherwise, when empty, `<p className="dn-empty">`:
    - "Brak merge requestów do review." **(default chosen by agent)**;
    - "Brak merge requestów do review pasujących do filtra." when `filterActive` and `filteredOut > 0`;
  - otherwise `GitlabTable label="Do review z GitLaba"` with `actions`, so each row has "+".
- MR rows get no hide button, which is today's `GitlabRow` rule. No CSS beyond layout; no literal colours.

#### 2. Row flag for "assigned and awaiting review"

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: An MR both assigned and awaiting review shows once, in the assigned list, with a "Do review" flag. It never also appears in the section, because Rust dedupes it **(default chosen by agent)**.

**Contract**:

- `TableRow` gains `reviewTag?: boolean`.
- `Lists` sets `reviewTag: item.reviewRequested` on the main list's rows.
- `GitlabRow` renders `<Flag tone="info">Do review</Flag>` when `row.reviewTag`, next to the other flags. The section's rows do not set it, so the flag is not repeated.

#### 3. View filter and header

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: The view filter options include the review items' projects, labels and milestones. The header shows the review count.

**Contract**:

- `viewOptions([...items.mergeRequests, ...items.reviewRequests, ...items.issues, ...items.hiddenIssues, ...closed])`.
- The header `summary` appends ` · ${n} do review` when `items.reviewRequests.length > 0` **(default chosen by agent)**. "widoczne" keeps counting MRs and issues.

#### 4. PRD note

**File**: `context/foundation/prd.md`

**Intent**: Lift the Non-Goal by the user's decision, with no new or changed FR numbers.

**Contract**:

- In `## Non-Goals` → "Poza v1 (z rundy Sokratesa)", drop "MR-y, w których użytkownik jest reviewerem;". The line keeps "usuwanie zadań (FR-009 usunięty)".
- After the autostart note, add one line: "MR-y, w których użytkownik jest reviewerem, są dostarczane w M-4 jako MS-35 (roadmapa): sekcja „Do review” w „Praca (GitLab)” pod filtrem FR-002, „+” do „Do zrobienia”, powiadomienia mechanizmem i progiem FR-004; bez nowego FR."
- The historical Socrates quote (`prd.md:123`) is left as it is.

### Success Criteria:

#### Automated Verification:

- UI type-checks and builds: `npm run build`
- `npm run lint` and `npm run format:check` are clean
- PRD has no reviewer Non-Goal and has the MS-35 note: `grep -n "MS-35" context/foundation/prd.md` finds the note, and the "Poza v1" line no longer contains "reviewerem"
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` stay green

#### Manual Verification:

These are run by the user after integration on a built app with the work GitLab. Back up the database before the first dev run.

1. **`reviews_for_me` on the real instance.** With no `gitlab.user`, the "Do review" section lists the user's open review MRs. The work GitLab supports `scope=reviews_for_me` and the GitLab state stays OK, with no "Error". This check must pass before the release PR to `main`; if the state turns "Error", set `gitlab.user` as the workaround and report it.
2. **With `gitlab.user` set,** the section lists that user's review MRs (`reviewer_username`).
3. **The section:**
   - It sits between the main list and "Zamknięte".
   - It is hidden for "Zgłoszenia" and shown for "Wszystko" and "MR-y".
   - Its rows have "+" and no hide button.
   - The settings filter (FR-002) removes matching review MRs and the footer shows "Ukryte filtrem: N".
   - The view filter works on it.
   - The header shows "· K do review".
4. **"+" adds a review MR to "Do zrobienia".** The entry is not ticked on the next refresh. It is ticked "closed in source" after the MR is merged or closed.
5. **A new review request** gives a Windows notice titled "Day Navigator — nowy MR do review":
   - It arrives even when the settings filter hides that MR.
   - More than 3 new items at once give one combined notice.
   - The dashboard event log shows "Nowe przypisanie" for it.
6. **First run after upgrade on an existing database:** no notice burst and no "Nowe" flags for review MRs that were already open.
7. **An MR assigned to the user and awaiting their review** shows once, in the main list with a "Do review" flag, not in the section. It gives at most one notice.
8. **Without VPN:**
   - the source is calm "unavailable";
   - the last list, including "Do review", stays visible and dimmed;
   - after VPN returns, the list refreshes with no repeated notices.
9. **The "Do zrobienia" picker** lists review MRs next to the assigned items, adds one, and its filter note counts review MRs the settings filter hid.
10. **A new review MR's row** in "Do review" shows "Nowe" until the user leaves "Praca (GitLab)", like a new assigned item.

---

## Testing Strategy

### Unit Tests:

- `cli::tests`: counts, the exhaustive ordinal, both endpoint shapes accepted, and the malformed reviewer shape refused.
- `gitlab::` `split_review_requests`: overlap cases, the flag set on both sides, a repeated id. Also `next_snapshot` keeping `review_requests`.
- `gitlab_refresh::`: `filter_shown` count, `shape_items` and `reshape_kept` copying review items.
- `new_assignments::rules`: `review_baseline` and `baseline_first_seen` cases.
- `new_assignments::notice`: the review title, the combined notice unchanged.
- `new_assignments::tests`: `keys_of`, `items_for`, `mark_fresh` and `review_only_keys` include and isolate review items.
- `todo::`: lookup of a review-only MR, the reconcile presence and the tick when it is absent from both lists.

### Integration Tests:

- `tests/gitlab_fetch.rs` (fake-cli):
  - review parsing and dedupe;
  - the user form;
  - `gitlab-reviews-no-network` gives `Unavailable` with no partial list;
  - `gitlab-reviews-bad-scope` gives a non-calm error;
  - nothing leaks.
- `tests/new_assignments.rs` (real store, invented ids):
  - the silent baseline on a pre-slice seeded account;
  - the next round notifies;
  - a new account seeds both markers;
  - no re-notify when moving between lists;
  - an `assigned` event per new review key, through `items_for` and `events::record` in the existing event test style (`tests/events.rs`) if `record_seen` is not reachable from tests.
- `tests/new_assignments_crash.rs`: unchanged assertions with the new signature. The baseline is atomic because it is inside the existing single write **(default chosen by agent: no new crash case; no new table or write path)**.

### Manual Testing Steps:

See the Phase 5 Manual Verification. These rows stay unticked until the user tests after integration.

## Performance Considerations

There is one extra paginated GET per GitLab round, on the existing interval. The review list is small, usually under 100 items. Dedupe is linear with a `HashSet` of ids. No new locks: the call runs in `run_fetch` with no lock held.

## Migration Notes

There is no schema migration. The only new persisted rows are:

- one extra `assignment_accounts` row per account, under `"<account-key>:reviews"`;
- the review MRs' `seen_assignments` rows, kind `mergeRequest`, in the existing table.

Rollback to an older build is safe. Old code never reads the marker row, and the extra seen rows are pruned after 30 days absent, as today.

## References

- Research: `context/changes/gitlab-review-requests/research.md`
- CLI pattern: `src-tauri/src/cli/mod.rs:206-216`, `:328-470`, `:529-700`
- No-partial fetch: `src-tauri/src/gitlab/fetch.rs:16-39`
- Seen set and seeding: `src-tauri/src/new_assignments/mod.rs:51-228`, `rules.rs:88-126`, `notice.rs:38-61`
- Round, filter and kept list: `src-tauri/src/gitlab_refresh.rs:274-345`, `:530-715`, `:752-812`
- To-do: `src-tauri/src/todo/commands.rs:33-46`, `src-tauri/src/todo/mod.rs:560-600`
- UI: `src/gitlab/GitlabView.tsx:221-330` (row and table), `:346-390` (`ClosedBlock` pattern), `:433-545` (`Lists`), `:604-609` (header)
- Prior slices: `context/archive/2026-11-23-new-assignment-notify/`, `context/archive/2026-11-27-gitlab-offline-last-list/`, `context/archive/2026-11-27-source-refresh-skeleton/`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Read-only review-MR call, guard and fake-cli

#### Automated

- [x] 1.1 Guard test passes with the new counts and refuses the negative review endpoint
- [x] 1.2 Runner echo test covers the new examples
- [x] 1.3 cargo fmt --check and cargo clippy --all-targets -- -D warnings are clean
- [x] 1.4 Full suite green: cargo test

### Phase 2: Review list in the round: fetch, dedupe, filter, kept list, TS mirror

#### Automated

- [x] 2.1 Dedupe unit tests pass
- [x] 2.2 Round, filter and kept-list unit tests pass
- [x] 2.3 Fetch integration tests pass, including the two new failure scenarios
- [x] 2.4 TS mirror type-checks: npm run build
- [x] 2.5 npm run lint, npm run format:check, cargo fmt --check, cargo clippy are clean
- [x] 2.6 Full suite green: cargo test

### Phase 3: FR-004 notices, NEW flags and the silent first-round baseline

#### Automated

- [x] 3.1 Baseline rule and notice unit tests pass
- [x] 3.2 Store-backed baseline and one-key tests pass
- [x] 3.3 Crash harness still green with the new signature
- [x] 3.4 cargo fmt --check and cargo clippy --all-targets -- -D warnings are clean
- [x] 3.5 Full suite green: cargo test

### Phase 4: "Do zrobienia": pick from "Do review" and reconcile safely

#### Automated

- [ ] 4.1 To-do unit tests pass, including the review-only presence and tick cases
- [ ] 4.2 To-do integration tests stay green
- [ ] 4.3 npm run lint, npm run format:check, npm run build are clean
- [ ] 4.4 cargo fmt --check and cargo clippy --all-targets -- -D warnings are clean
- [ ] 4.5 Full suite green: cargo test

### Phase 5: "Do review" section in "Praca (GitLab)" and the PRD note

#### Automated

- [ ] 5.1 UI type-checks and builds: npm run build
- [ ] 5.2 npm run lint and npm run format:check are clean
- [ ] 5.3 PRD has no reviewer Non-Goal and has the MS-35 note
- [ ] 5.4 cargo fmt --check, cargo clippy, cargo test stay green

#### Manual

- [ ] 5.5 reviews_for_me is supported on the real instance and the section lists review MRs
- [ ] 5.6 With gitlab.user set, the section lists that user's review MRs
- [ ] 5.7 Section placement, type switch, "+" without hide, settings filter footer, view filter, header count
- [ ] 5.8 "+" adds a review MR to Do zrobienia; not ticked on refresh; ticked after merge or close
- [ ] 5.9 New review request gives "nowy MR do review" notice despite the filter; threshold; event log line
- [ ] 5.10 First run after upgrade: no notice burst and no Nowe flags for already-open review MRs
- [ ] 5.11 MR both assigned and to review shows once with a Do review flag and gives at most one notice
- [ ] 5.12 Without VPN: calm unavailable, Do review stays visible and dimmed, no repeated notices after
- [ ] 5.13 Do zrobienia picker lists and adds review MRs; its filter note counts them
- [ ] 5.14 A new review MR row shows Nowe until the view is left
