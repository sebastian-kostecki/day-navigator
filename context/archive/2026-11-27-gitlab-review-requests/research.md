---
date: 2026-11-27T16:55:17+01:00
researcher: Claude (Opus 5.5)
git_commit: 00e6a19b6653f988e73b4c7744f9dbb5050a6981
branch: slice/gitlab-review-requests
repository: day-navigator
topic: "S-06: GitLab merge requests awaiting my review in a \"Do review\" section"
tags: [research, gitlab, cli, new-assignments, todo, gitlab-view, s-06, ms-35]
status: complete
last_updated: 2026-11-27
last_updated_by: Claude (Opus 5.5)
---

# Research: GitLab merge requests awaiting my review ("Do review")

**Date**: 2026-11-27T16:55:17+01:00
**Researcher**: Claude (Opus 5.5)
**Git Commit**: 00e6a19 (develop tip, M-4 wave 1 merged)
**Branch**: slice/gitlab-review-requests
**Repository**: day-navigator

Line numbers are from this commit. A `~` marks a line read from a partial listing.

## Research Question

How should S-06 (MS-35; FR-001, FR-002, FR-004) be built on today's code?

- Fetch the merge requests where the user is a reviewer through a new read-only `glab api` `CliCall`.
- Show them in a "Do review" section of "Praca (GitLab)", under the FR-002 settings filter, with "+" to "Do zrobienia".
- Give a Windows notice for a new review request by the FR-004 mechanism and threshold, bypassing the filter.
- Reuse the `seen_assignments` keys with kind `mergeRequest`, with no migration.
- An MR that is both assigned and awaiting review gets at most one notice and is not shown twice in a confusing way.
- Without VPN it behaves like the rest of the GitLab source.

Specific sub-questions:

- Which endpoint to use.
- How the username or id is known today.
- The dedupe and notice key.
- Whether hidden-issue snapshots touch MRs.
- The merge points with S-01 and S-02.

## Summary

1. **Endpoint.** Mirror the existing assigned-MR call with two fixed shapes, one per `gitlab.user` setting:
   - **No user configured:** `merge_requests?scope=reviews_for_me&state=opened&with_labels_details=true&per_page=100`.
   - **User configured:** `merge_requests?scope=all&state=opened&reviewer_username=<login>&with_labels_details=true&per_page=100`.

   The app never learns the logged-in username or id. There is no `glab api user` call. The only username is the optional `gitlab.user` setting (`src-tauri/src/cli/mod.rs:121`, `:214-216`). So `reviewer_username` and `reviewer_id` cannot be used without that setting. `scope=reviews_for_me` is the GitLab-documented scope for "merge requests where the current user is a reviewer" (GitLab REST docs, List merge requests).

   Risk: the work GitLab's version is unknown. If it does not support `reviews_for_me`, the call fails.
2. **Identity and dedupe.** The identity used everywhere is `ItemKey{kind, id}` with GitLab's global id (`src-tauri/src/new_assignments/rules.rs:16`). An MR that is both assigned and to review has the same key in both lists. If the review list goes into the same `keys_of` input, three things follow:
   - `plan_round` and `items_for` dedupe it (`rules.rs:107-111`, `new_assignments/mod.rs:70-89`). That gives **one notice, one seen row and one NEW state**, with no migration.
   - Moving between "assigned" and "review only" never re-notifies.
   - A key is forgotten only after 30 days absent from both lists (`rules.rs:10`, `:115-119`).
3. **One real gap: first round after release.** An account that is already seeded has no baseline for review MRs. The first successful round would notify for every currently open review MR (one summary notice if there are more than 3) and flag them all NEW.

   This can be avoided without a migration with a second seeding marker in the existing `assignment_accounts` table, under a derived opaque account key (see "Baseline" below). The plan must choose between that marker and accepting a one-time burst.
4. **Where review MRs live in the snapshot.** Recommended: a new `AssignedItems.review_requests: Vec<GitlabItem>` holding **review-only** MRs.
   - An MR that is also assigned stays only in `merge_requests`, so it is one row (assigned wins), optionally with a "do review" marker.
   - Every consumer that walks `AssignedItems` must be extended by name. These are listed under "Files to touch". The two silent-failure ones are the "To do" reconcile and `find_gitlab_item`.
5. **Failure policy.** Recommended: make the review call the third call of `fetch_assigned`, so its failure fails the round ("no partial list", `src-tauri/src/gitlab/fetch.rs:16-18`).
   - The calm "unavailable" state and S-04's kept, dimmed list then cover review MRs with no new code. `next_snapshot` keeps `prev.items`, which is the whole `AssignedItems` (`src-tauri/src/gitlab/mod.rs:551`).
   - A per-section error, like `ClosedSection.error`, would let the "To do" reconcile tick review-only entries as "closed in source" on a round where only the review call failed. The FR-004 seen set would also run on an incomplete list.
6. **Hidden-issue snapshots** are issue-only. `hide()` refuses non-issues, the table has no kind column, and MRs pass through the hide round untouched (details below). Nothing in S-06 touches `hidden_issues`.
7. **Merge points.**
   - **With S-01:** `src-tauri/src/cli/mod.rs` (enum, `examples()`, `VARIANT_COUNT`, `EXAMPLE_COUNT`, `ordinal()`, the expected-counts array, the guard's `check_read_only` and `endpoint_allowed`), `src-tauri/src/bin/fake_cli.rs` (`endpoint()` and scenario arms), and maybe `tests/cli_runner.rs`. These are textual conflicts resolved by combining both sides and adding up the counts.
   - **With S-02:** none expected. S-06 needs no change in `settings.rs`, `github_refresh.rs`, `src/github/` or `source_refresh.rs`.
   - **`lib.rs`:** probably untouched by S-06, since no new command is needed.
   - **`store/schema.rs`:** untouched, since there is no migration.
   - **AGENTS.md:** at most a wording touch in the app-data list (the derived marker key), next to S-01's edit of the same list.

## Detailed Findings

### CLI: today's GitLab calls and the guard

- **Variants.** `CliCall` (`src-tauri/src/cli/mod.rs:328`) has 10 variants. The GitLab MR calls are `GlabAssignedMergeRequests(GitlabHost, Option<GitlabUser>)` and `GlabRecentlyClosedMergeRequests(..)`.
- **Argument shape.** Every GitLab call is `["api", <endpoint>, "--paginate", "--hostname", <host>]`. It is a GET with the query baked into the endpoint and no `-f` or `-F` (`cli/mod.rs:218-~240`).
- **Endpoints.**
  - Fixed form: `ASSIGNED_MRS_ENDPOINT` (`cli/mod.rs:210-211`).
  - User form: `MRS_USER_PREFIX` + login + `USER_SUFFIX` (`cli/mod.rs:215-216`).
  - The login is a validated `GitlabUser`, `[A-Za-z0-9._-]`, 1-255 characters (per the CLI worker, `cli/mod.rs:~88-111`).
- **Guard test** (`mod tests`).
  - `check_read_only` (`cli/mod.rs:727`) accepts `api` calls only when `endpoint_allowed` (`:673`) passes.
  - `endpoint_allowed` accepts:
    - `ALLOWED_ENDPOINTS` (`:610`), the two fixed assigned endpoints;
    - a strict user form: exactly one `?`, exactly four `&`, none of `#%:/+`, and `<ISSUES_USER_PREFIX|MRS_USER_PREFIX><login><USER_SUFFIX>`;
    - the notes and closed shapes.
  - It re-implements the validators instead of trusting the production parsers.
- **What a new variant touches.**
  - The enum, `tool()`, `args()`, `name()` (log key, for example `glab:review-mrs`), and `examples()` (`:360`).
  - `VARIANT_COUNT` = 10 → 11 (`:529`).
  - `EXAMPLE_COUNT` = 14 → 16 (`:531`), one with a user and one without.
  - The exhaustive `ordinal()` (`:543`), the expected-counts array in `examples_cover_every_variant_with_and_without_a_user` (`:559`), and the with-user count.
  - The fixed endpoint in `ALLOWED_ENDPOINTS`, and a third user prefix accepted by `endpoint_allowed`. The new prefix, `merge_requests?scope=all&state=opened&reviewer_username=`, keeps the same `?`/`&` counts.
  - `CliCall::ALL` (`:349`) holds only parameterless calls. The new parameterised variant does not go there, so `tests/cli_real.rs` is unaffected.
  - `tests/cli_runner.rs` echo-checks every `examples()` entry under the `data` scenario, so the new variant is covered once it is in `examples()` (per the CLI worker).
- **Error classification** (`src-tauri/src/cli/classify.rs:44-57`) is shared.
  - Network markers and HTTP 5xx give `Unavailable` (calm).
  - 401 and login hints give `LoggedOut`.
  - Anything else gives `Failed`. An HTTP 400 from an unsupported `scope` value would land in `Failed`, and the GitLab state would show `Error`.
  - `From<CliError> for GitlabError` is at `gitlab/mod.rs:~151`.
- **Username or id.** I found no `glab api user` call. The grep over `src-tauri/src/` hit only fake-cli and classify test strings (CLI worker). `GitlabConfig{host, user: Option<GitlabUser>}` is at `cli/mod.rs:121`.
- **GitLab API facts (docs.gitlab.com/api/merge_requests, read 2026-11-27, first 100k characters).**
  - `scope` takes `created_by_me` (the default), `assigned_to_me`, `reviews_for_me` or `all`. `reviews_for_me` means the current user is a reviewer. The inspected part gave no version note.
  - `reviewer_username` and `reviewer_id` are mutually exclusive.
  - The response has a `reviewers[]` array.
- **Why `scope=all&reviewer_username=` for the user form.** It mirrors the assignee user form, which exists for a `glab` logged in as a bot whose own lists are empty. `reviews_for_me` would list the bot's reviews.

### fake-cli

- **How it is driven.** `FAKE_CLI_SCENARIO` selects behaviour (`src-tauri/src/bin/fake_cli.rs:555`).
  - Bodies are inline invented `json!` values; there are no fixture files.
  - `endpoint()` (`:66`) matches argv by prefix in this order: closed MRs (`state=all`), closed issues, any `merge_requests?` → `MergeRequests`, issues, notes.
- **What S-06 changes there.** A review endpoint would be served the assigned-MR body today. S-06 needs:
  - an `Endpoint::ReviewMergeRequests` arm placed before the generic MR arm, matching `scope=reviews_for_me` or `reviewer_username=`;
  - a body in `gitlab_assigned_body` (`:231`) with invented review-only MRs and one MR shared with the assigned body, to test dedupe;
  - failure scenarios modelled on `gitlab-mrs-ok-issues-no-network` (`:~595`).
- **Tests.** `tests/gitlab_fetch.rs` is the place for fetch tests. One precedent is `second_call_failure_is_unavailable_with_no_partial_result` (`:147`).

### Fetch, filter, snapshot and refresh round

- **`fetch_assigned`** (`src-tauri/src/gitlab/fetch.rs:19-39`) runs the MRs and then the issues, and the first failure ends it. `run_fetch` (`src-tauri/src/gitlab_refresh.rs:495`) calls it, and `refresh_round` (`:530`) runs the rest.
- **Parsing.** `GitlabItem` (`gitlab/mod.rs:67-100`) has `kind`, `id` (global), `reference`, `title`, `web_url`, `updated_at`, `labels`, `draft`, `changed`, `milestone`, `is_new` and `hidden_at_ms`. `project_id` and `iid` are `#[serde(skip)]`.
  - `RawItem` (`:280`) parses no `reviewers` or `author`.
  - Review MRs parse with the existing `parse_merge_requests`, and no new field is needed.
- **FR-002 filter.** It runs in Rust: `filter_shown` (`gitlab_refresh.rs:277`) filters `merge_requests`, `issues` and `hidden_issues` separately and counts what it removes into `FilteredOut` (`gitlab/mod.rs:445`).
  - Dimensions are project (from the reference), labels and milestone, with include/exclude per dimension and AND between them.
  - State is fixed by the query (`opened`), not by the filter.
  - The full list stays unfiltered (`gitlab_refresh.rs:603`), and the seen set and notices use it.
  - A review list must be added to `filter_shown`, to `FilteredOut`, and to `reshape_kept` (`:323`), which re-filters the kept full list after a failed round.
- **Snapshot.** `GitlabSnapshot` is at `gitlab/mod.rs:476`, and `next_snapshot` (`:551`) keeps `prev.items` on a failed round when the config is unchanged (S-04).
  - A `review_requests` field inside `AssignedItems` is kept and dimmed with no new code.
- **Skeleton.** F-01's `source_refresh.rs` leaves the fetch and rules to the source. A second or third fetch in GitLab's own `run_fetch` needs no skeleton change. The F-01 research and impl-review say this explicitly for S-06 (UI and history worker, `context/archive/2026-11-27-source-refresh-skeleton/`).
- **Locks.** The lock order in a round is unchanged; the review fetch runs with no lock held, like the others.

### FR-004 notices and the seen set

- **Key.** `ItemKey{kind: ItemKind, id: u64}` (`src-tauri/src/new_assignments/rules.rs:16`).
  - In the database the kind is `"mergeRequest"` (`new_assignments/mod.rs:36-49`).
  - The table is `seen_assignments(account, kind, item_id, first_seen_ms, last_seen_ms)`, with primary key (account, kind, item_id), CHECK `kind IN ('issue','mergeRequest')` and `item_id > 0` (store worker, `src-tauri/src/store/schema.rs:~131-143`, migration 7).
  - **Notice key for a review MR:** (`account_key(cfg)`, `mergeRequest`, global id), the same key as the assigned form. So an MR in both lists is one key.
- **Input.** `keys_of` and `all_items` walk `merge_requests`, `issues` and `hidden_issues` (`new_assignments/mod.rs:51-68`). The review list must be chained in, with the same treatment in `items_for` (`:70-89`).
- **New, first run and pruning.**
  - `plan_round` (`rules.rs:101-126`) gives "new" only when the account is `seeded`.
  - The first round inserts the `assignment_accounts` marker and seeds silently (`new_assignments/mod.rs:176-181`). That round also gives no NEW flags (`:218-222`).
  - A key is forgotten after 30 days absent (`FORGET_AFTER_MS`, `rules.rs:10`).
  - `record_round` keeps `first_seen_ms` on upsert.
- **Text and threshold.** `notices_for` is at `new_assignments/notice.rs:38`, with `MAX_NOTICE_LINES = 3` (`src-tauri/src/health/mod.rs:20`).
  - With 1-3 items, the title comes from `item.kind` only: "nowe zgłoszenie" or "nowy MR" (`notice.rs:45`).
  - With more than 3, one summary is titled "nowe przypisania (N)" (`notice.rs:58`).
  - A review request would be titled "nowy MR" unless `notices_for` learns the role. The role needs a set of review-only keys, or a flag on the item, passed alongside.
- **Sending.** `toast_new_assignments` (`gitlab_refresh.rs:814`) runs after publish, outside the locks.
  - `record_seen` (`:752`) writes the round in one `store.write` and logs one `NewEvent::Assigned` per new key in the same `with_store`.
  - `SessionSeen::resolve` is the fallback when the store fails (`new_assignments/mod.rs:254-286`).
- **Baseline (decision for the plan).**
  - Without extra code, the first round after release on an already-seeded account notifies every open review MR and flags them NEW.
  - Option (a), no migration: store a "review set seeded" marker as one more `assignment_accounts` row under a derived opaque key, for example SHA-256 of the account key plus a fixed suffix.
    - On a round where that marker is missing, the review-only keys not already stored are upserted silently: not new, not fresh, and the marker is inserted in the same write.
    - Later rounds work as today.
    - It needs `record_round` to receive the review-only keys separately, or a set of them.
    - A brand-new account seeds both at once.
  - Option (b): accept a one-time burst. It is at most one summary notice when there are more than 3.
  - Recommendation: (a). The burst would also arrive mislabelled as "nowe przypisania".
- **Event log.**
  - `NewEvent::Assigned` writes kind `assigned`, shown as "Nowe przypisanie" (`src/todo/EventLogPanel.tsx:21-24`).
  - The `events.kind` CHECK is closed (store worker: `schema.rs:~230`, rebuilt in migration 13). A "review" event kind would need a migration, which is out.
  - Choices: log review requests as `assigned` (the dashboard says "Nowe przypisanie"), or do not log them.
  - Recommendation: log them as `assigned`, so every notice has its dashboard line.

### "Do zrobienia" (To do)

- **"+" command.** It is `add_todo_gitlab_item(item_kind, reference)`.
  - It looks the item up with `find_gitlab_item` (`src-tauri/src/todo/commands.rs:33-46`), only in `merge_requests` or `issues` of the shown snapshot.
  - Today a review-only MR would fail with `NotInGitlabList`, so it must also search `review_requests`.
  - Stored: kind, reference, title, URL and account. The duplicate check is (kind, reference) (`src-tauri/src/todo/mod.rs:345`).
  - The UI key is `gitlab:<kind>:<reference>` (`src/todo/keys.ts:6`), so one To-do entry serves both forms of the same MR.
- **Auto-tick.** `reconcile_gitlab` (`todo/mod.rs:569`) builds `present` from `merge_requests`, `issues` and `hidden_issues` by (kind, reference), and ticks absent entries as `ClosedInSource`.
  - If `review_requests` is not chained in, a To-do entry picked from "Do review" is ticked on the next round. That is the most dangerous silent bug for this slice.
  - Once it is chained in: a review MR that is merged or closed leaves the `state=opened` list and auto-ticks, which is wanted.
  - A review MR the user is removed from as reviewer, but that is still open, also auto-ticks as "closed in source". That matches today's behaviour for an assigned MR that gets unassigned.
- **UI consumer.** `TodoView.tsx:409` builds `gitlabItems` from `mergeRequests` and `issues`. The review list must be added to it.

### Hidden issues

- `hide()` refuses `kind != Issue` (`src-tauri/src/hidden_issues/mod.rs:~100-102`).
- The `hidden_issues` table has an `issue_id` and no kind (migration 6).
- `commit_round` takes only `full.issues`, and MRs pass through unchanged (`gitlab_refresh.rs:~605`).
- So hidden-issue snapshots do not touch MRs, and S-06 needs no change there.
- S-07 will add hiding for both MR lists, so `review_requests` as plain `GitlabItem`s leaves room for it.

### UI: "Praca (GitLab)"

- **Mirror.** TS types are at `src/gitlab/types.ts`: `GitlabItem` (`:17-38`), `AssignedItems` (`:40-46`), `FilteredOut` (`:58-62`) and `GitlabSnapshot` (`:90-120`). Add `reviewRequests` to `AssignedItems` and `FilteredOut` in the same commit as the Rust change.
- **Layout.** `Lists` (`src/gitlab/GitlabView.tsx:433`) renders, in order:
  - the filter bar;
  - the toolbar (project, label and milestone `MultiSelect`s, the `Segmented` type switch from `KIND_OPTIONS` at `:83`, and "Pokaż ukryte (n)");
  - one untitled main `Panel` holding assigned MRs, issues and hidden issues;
  - `ClosedBlock` (`:346`).
- **Section pattern.** `ClosedBlock` is the titled-section pattern to copy: `Panel title meta footer`, a `Notice` on error, `ViewEmpty`, an empty message, and a `GitlabTable` filtered by `matchesKind` and `matchesView`.
  - Suggested: a "Do review" `Panel` between the main panel and "Zamknięte".
  - Show it for kind "Wszystko" and "MR-y", and hide it for "Zgłoszenia". `matchesKind` already gives this, because review items are `mergeRequest`s.
  - It needs its own empty text, for example "Brak merge requestów do review.", plus a filter-aware variant and `footerNote(filteredOut.reviewRequests, …)`.
- **Rows.** `GitlabRow` works unchanged.
  - Hide buttons render only for `kind === "issue"`, so MR rows get only "+" (`TodoAddButton`).
  - The NEW `Flag` comes from `isNew`, so `mark_fresh` (`new_assignments/mod.rs:93-100`) must also mark `review_requests`.
- **View filter.** `viewOptions` (`src/gitlab/viewFilter.ts:64`) is built from the lists in `Lists`. Add the review items, or their projects and labels are missing from the view filter.
- **Header.** The summary "N widoczne · M ukryte" counts `mergeRequests + issues`. The plan decides whether review MRs are counted, or shown as a separate "· K do review" (my suggestion).
- **Offline.** Everything inside `.gitlab-lists` is dimmed when the state is not ok (`GitlabView.tsx:483`, S-04), so the new panel inherits it.
- **No hiding toggle.** None is needed for the section in this slice. A remembered collapse state would need a `gitlab_view_state` column and a migration, so keep any collapse in memory only or skip it.

## Code References

- `src-tauri/src/cli/mod.rs:208-216` - assigned MR fixed endpoint and user prefix (pattern for the review call).
- `src-tauri/src/cli/mod.rs:328`, `:349`, `:360`, `:529-565`, `:610`, `:673`, `:727` - enum, `ALL`, `examples()`, the guard test counts and the endpoint guard.
- `src-tauri/src/cli/classify.rs:44-57` - error classes (calm unavailable vs logged out vs failed).
- `src-tauri/src/bin/fake_cli.rs:66`, `:231`, `:555` - endpoint matcher, bodies, scenarios.
- `src-tauri/src/gitlab/fetch.rs:16-39` - `fetch_assigned`, no partial list.
- `src-tauri/src/gitlab/mod.rs:28-36` - `account_key` (opaque SHA-256 of host and user).
- `src-tauri/src/gitlab/mod.rs:111`, `:445`, `:476`, `:551` - `AssignedItems`, `FilteredOut`, `GitlabSnapshot`, `next_snapshot`.
- `src-tauri/src/gitlab_refresh.rs:277`, `:323`, `:495`, `:530`, `:603`, `:752`, `:814` - `filter_shown`, `reshape_kept`, `run_fetch`, `refresh_round`, the unfiltered full list, `record_seen`, the toasts.
- `src-tauri/src/new_assignments/mod.rs:51-100`, `:126-228` - `keys_of`, `items_for`, `mark_fresh`, `record_round`.
- `src-tauri/src/new_assignments/rules.rs:10`, `:16`, `:101-126` - forget window, `ItemKey`, `plan_round`.
- `src-tauri/src/new_assignments/notice.rs:38-58` - notice titles and the summary.
- `src-tauri/src/todo/commands.rs:33-46` - `find_gitlab_item`.
- `src-tauri/src/todo/mod.rs:345`, `:569-600` - `add_gitlab`, `reconcile_gitlab`'s `present` map.
- `src/gitlab/GitlabView.tsx:83`, `:346`, `:433`, `:483` - type switch, `ClosedBlock`, `Lists`, stale dimming.
- `src/gitlab/types.ts:17-120` - TS mirrors.
- `src/gitlab/viewFilter.ts:64` - `viewOptions`.
- `src/todo/TodoView.tsx:409` - the To-do picker's GitLab items.
- `src/todo/EventLogPanel.tsx:21-24` - "Nowe przypisanie" label.

## Architecture Insights

- **One GitLab source, one round.** Review MRs are a third fetch in GitLab's own round, not a new source. A new `SourceCell` would mean a new health entry, event and TS view, for no gain.
- **`AssignedItems` is walked by name in many places.** These are: `all_items`/`keys_of`, `items_for`, `mark_fresh`, `filter_shown`, `reshape_kept`, `find_gitlab_item`, `reconcile_gitlab`, `TodoView` `gitlabItems`, `viewOptions` in `Lists`, and the header counts.
  - A new Vec is explicit, but each of these must list it.
  - Putting review-only MRs into `merge_requests` with a flag would need fewer edits, but it would silently change "assigned" semantics: header counts, "Brak przypisanych merge requestów", the toast title, and `FilteredOut.merge_requests`.
  - Recommendation: the separate Vec, plus one Rust unit test per consumer (or a shared iterator helper) so a missed consumer shows up.
- **Dedupe in Rust at fetch time.** Drop from `review_requests` every MR whose id is in `merge_requests`, so the UI never sees one MR twice. Optionally set a `review_requested` flag on the assigned row. This is a domain rule, so it lives in Rust with unit tests (AGENTS.md: "Domain rules … live in Rust").
- **Notice role.** To keep "nowy MR do review" distinct from "nowy MR", pass the review-only key set into `notices_for`. Suggested single title: "… — MR do review". The summary title "nowe przypisania (N)" can stay, or become neutral. The plan decides the wording.

## Historical Context (from prior changes)

- `context/archive/2026-11-23-gitlab-assigned-list/plan.md:44` and `context/archive/2026-11-23-new-assignment-notify/plan.md:41` excluded reviewer MRs as a PRD Non-Goal. This slice lifts that.
- The `new-assignment-notify` plan-brief is the source of the threshold (separate notices up to 3, one combined above) and of the session fallback. Both still hold (`notice.rs`, `new_assignments/mod.rs:254`).
- `context/archive/2026-11-23-gitlab-visibility-filter/plan-brief.md:23`: the filter applies to MRs and issues alike, and notices bypass it. That still holds (`gitlab_refresh.rs:603`).
- `context/archive/2026-11-27-source-refresh-skeleton/` (F-01): "thin skeleton, thick source". The research and impl-review name S-06's extra `glab api` fetch as belonging in GitLab's `run_fetch`, and dedupe as a `new_assignments` rule. Both still hold.
- `context/archive/2026-11-27-gitlab-offline-last-list/` (S-04): failed rounds keep the items, and the UI dims `.gitlab-lists`. It records a risk that later slices must keep "failed round keeps items". The recommended all-or-nothing fetch keeps that.
- `context/archive/2026-11-27-gitlab-view-memory/` (S-05): only `kind` and `showHidden` are persisted (migration 14). The view filter stays in memory.

## Related Research

- `context/archive/2026-11-27-source-refresh-skeleton/research.md`
- `context/archive/2026-11-23-new-assignment-notify/` (plan and plan-brief)

## Merge points with wave-2 siblings (from the roadmap and the code layout; their branches were not inspected)

- **S-01 `github-hide-resurface`.**
  - It adds a `gh api user` `CliCall`. It is probably parameterless, so it goes into `CliCall::ALL` and `cli_real.rs`.
  - That touches the same lines in `cli/mod.rs` as S-06: the enum, `tool()`, `args()`, `name()`, `examples()`, `VARIANT_COUNT`, `EXAMPLE_COUNT`, `ordinal()`, the seen-counts array, and `check_read_only` for a `gh api` shape.
  - Both slices also touch `fake_cli.rs` (`endpoint()` and scenario arms) and maybe `tests/cli_runner.rs`.
  - S-01 adds GitHub hide commands to `lib.rs` `generate_handler!`, a migration to `store/schema.rs`, and an app-data line to AGENTS.md.
  - S-06 expects no `lib.rs` or `schema.rs` change. AGENTS.md changes at most in wording.
  - `src/todo/TodoView.tsx` may be touched by both, near the GitLab and GitHub item lists.
- **S-02 `github-refresh-interval`.** It touches `settings.rs`, `github_refresh.rs`, `src/github/` and possibly `source_refresh.rs`. S-06 needs none of these, so no overlap is expected.
- **Resolution.** The conflicts are additive. When merging, add the counts together (for example `VARIANT_COUNT` 10 + 1 + 1) and keep both arms, then re-run `cargo test` so the exhaustive `ordinal()` and the counts check catch a wrong sum.

## Suggested PRD note (for the plan or implementation; no FR numbers change)

- In `context/foundation/prd.md:227` ("Poza v1"), drop "MR-y, w których użytkownik jest reviewerem".
- After the autostart note (`prd.md:240`), add one line, for example: "MR-y, w których użytkownik jest reviewerem, są dostarczane w M-4 jako MS-35 (roadmapa): sekcja „Do review” w „Praca (GitLab)” pod filtrem FR-002, powiadomienia mechanizmem i progiem FR-004; bez nowego FR."

## Open Questions (answered here as defaults; the plan may revise)

1. **No user configured.** Use `scope=reviews_for_me`.
   - Unresolved fact: whether the work GitLab's version supports it.
   - If it does not, the user form (`gitlab.user`) is the fallback, or a later `glab api user` call.
   - It needs one manual check on the real instance. Without that check, an unsupported scope would turn the whole GitLab source into "Error" under the all-or-nothing policy.
2. **Both assigned and awaiting review.** The MR appears once, in the assigned list (optionally with a "do review" flag). The review section lists review-only MRs. There is one key and one notice.
3. **First round after release.** Default: a silent baseline through a derived marker in `assignment_accounts` (option (a)).
4. **Event log.** Default: log new review requests with the existing `assigned` kind.
5. **Notice title.** Default: "MR do review" for single notices. The summary keeps its threshold and wording.
6. **Approved MRs.** An MR the user already approved stays in "Do review" while it is open. The REST list has no reviewer-state filter in the inspected part of the docs. Accepted for this slice.
7. **"Zamknięte" section.** It stays assigned-only; recently closed review MRs are not listed.
