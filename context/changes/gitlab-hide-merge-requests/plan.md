# Hide GitLab merge requests (S-07 `gitlab-hide-merge-requests`) Implementation Plan

## Overview

In "Praca (GitLab)" the user can hide a merge request from its row, both an assigned MR and an MR in the "Do review" section (S-06), and show it again. A hidden MR comes back on its own, marked "Zmienione", when someone other than the user comments or when it leaves the open lists and is listed again (its state changed: closed or merged and reopened). Approvals, commits, pipelines, labels and a draft/ready toggle never bring it back. Hide records are rows of the existing table `hidden_items` (v15) with `source='gitlab'`, `kind='mergeRequest'`, `scope` = the GitLab host, `item_id` = the global MR id and `project_id`/`iid` set: **no migration**. The comment data comes through one new read-only call, `glab api projects/<id>/merge_requests/<iid>/notes?...`.

Roadmap M-4, wave 3, slice S-07, anchor MS-33 (PRD ref FR-003; GitHub issue #33). No new FR. Planned non-interactively: the skill's questions were not asked; the user's decisions in the brief are binding and every other choice is marked **default chosen by agent** with a one-line reason.

Complexity: MEDIUM-HIGH (one new CLI call, a second hidden round beside the issue round, a new snapshot list with its TS mirror, refresh wiring and UI). No question budget was spent: the research, the user's decisions and the orchestrator's guidance settle every decision.

## Current State Analysis

The research (`context/changes/gitlab-hide-merge-requests/research.md`, commit 0ea6c0c) maps the code; line numbers below are from the worktree at that commit. What matters here:

- **Storage and rules are ready.** `hidden_items` (v15, `src-tauri/src/store/schema.rs:306-327`) accepts gitlab merge-request rows and requires `project_id`/`iid` for gitlab rows. Store ops `list_items`, `hide_item`, `unhide_item`, `dismiss_item`, `apply_items_round` (`src-tauri/src/hidden_issues/items.rs:171-275`) take `HiddenSource`/`HiddenKind`; `HiddenSource::Gitlab` and `HiddenKind::MergeRequest` exist (`items.rs:14-55`). The rules `plan_round`, `comment_verdict`, `needs_comment_check`, `status_changed` are generic over `HideRecord`/`HideItem` (`hidden_issues/rules.rs:48-80, 228-333`), and `HideRecord for HiddenItemRecord` exists (`rules.rs:128-150`).
- **One rules gap:** `impl HideItem for GitlabItem` returns the scoped labels (`rules.rs:82-92`), right for issues, wrong for MRs (labels must not bring an MR back). `GitlabItem` has a `kind: ItemKind` field (`gitlab/mod.rs:70`), so the impl can branch on it.
- **GitLab issue hiding** uses its own table `hidden_issues` (`hidden_issues/mod.rs`), the round `check_round`/`commit_round` (`hidden_issues/round.rs:83-206`), the commands `hide_gitlab_issue`/`unhide_gitlab_issue`/`dismiss_gitlab_change` (`hidden_issues/commands.rs:27-90`) and the wiring in `gitlab_refresh.rs` (`act_on_issue` `:147-196`, `filter_shown` `:277-296`, `shape_items` `:301-316`, `reshape_kept` `:327-352`, `refresh_round` `:534-715`). Every path that re-shapes the list reads only the issue records: hiding an issue today re-partitions issues only.
- **MR lists:** `AssignedItems` holds `merge_requests`, `issues`, `hidden_issues`, `review_requests` (`gitlab/mod.rs:139-149`). `split_review_requests` (`gitlab/mod.rs:108-128`) makes `merge_requests` and `review_requests` disjoint by id, so one global id keys one hide record whatever the section. Both lists are `state=opened` only (`cli/mod.rs:208-221`).
- **Notes call:** `CliCall::GlabIssueNotes(GitlabHost, IssueRef)` (`cli/mod.rs:349-364, 384`), `fetch_issue_notes` (`gitlab/fetch.rs:119-126`), `parse_notes` (`gitlab/notes.rs`, reads only `author.username`, `system`, `created_at`). The guard test (`cli/mod.rs:596-720`) has `VARIANT_COUNT = 13`, `EXAMPLE_COUNT = 18`, an exhaustive `ordinal` (last ordinal 12), the `seen` array and `notes_endpoint_allowed`, which splits only on `/issues/` (`cli/mod.rs:710-717`).
- **fake-cli:** `Endpoint::Notes{project_id,iid}` and `parse_notes_endpoint` know only `/issues/` (`bin/fake_cli.rs:45-71`); notes are served by `gitlab_notes_body` and `gitlab_notes_scenario` (`:343-433`) for the issue `(102, 12)` only. The assigned MR fixture `6003` is also `(102, 12)` (`:268-276`), so an MR notes route must be told apart by its path, never by the numbers alone.
- **UI:** `RowButtons` render only on issue rows (`src/gitlab/GitlabView.tsx:235`); the "Do review" panel (`ReviewBlock`, `:381-420`) has no hide buttons; the toolbar toggle "Pokaż ukryte (n)" counts `hiddenIssues` only (`:513, :555-562`); `useGitlab.ts` keys `busy` by the bare id (`:24, :92-115`). `TodoView.tsx` collects "zmienione" from `items.issues` only (`:237-240`) and shows the GitLab hidden count from `hiddenIssues` (`:427`).
- **Other enumerations of `AssignedItems`:** `new_assignments/mod.rs:62-69` (`all_items`) and `:126-140` (`mark_fresh`); `todo/mod.rs:579-586` (`reconcile_gitlab` "present" set). `find_gitlab_item` (`todo/commands.rs:32-48`) pools MRs from `merge_requests` and `review_requests` only, so a hidden MR is not pickable, as a hidden issue is not.
- No `context/foundation/lessons.md` exists.

## Desired End State

- Every MR row in the main table and in "Do review" has "Ukryj do zmiany"; a hidden MR has "Pokaż na liście"; a marked MR has "OK, zauważone". Hidden MRs (assigned and review-only) appear as dimmed rows after the visible rows of the main table when "Pokaż ukryte (n)" is on, with "ukryte DD.MM" and the "Do review" tag when they await the user's review; n counts hidden issues and hidden MRs. A returned MR sits in its own section with the "Zmienione" flag and the reason as a tooltip.
- A hidden MR resurfaces when a non-system note by someone other than the user (or any author when `gitlab.user` is not set) is newer than the baseline (`Comment`), or when it was absent from both open lists and is listed again (`Returned`). Its status fingerprint is empty: labels, draft/ready, approvals, pushes and pipelines never bring it back (they only move `updated_at`, which triggers a notes check and then a baseline advance).
- An MR both assigned and in "Do review" has one record and is hidden from both places; an MR that moves between the two sections while hidden stays hidden.
- GitLab issue hiding behaves exactly as before (table, rules, notices, texts, tests).
- A failed MR store read or write shows every MR unpartitioned with the existing "hidden state" notice; never an empty list.
- `AGENTS.md` app-data list names GitLab merge requests in the hidden-item snapshots; the PRD has one line for MS-33.

Verify with the gates and the tests named in each phase, plus the Manual rows (checked later by the user).

### Key Discoveries:

- `plan_round` marks every record without a listed item as absent (`rules.rs:281-300`): the MR round must pass `merge_requests` and `review_requests` together as the listed items, or a review-only hidden MR would be marked absent.
- Issue ids and MR ids are separate id spaces and can collide (research section 1): every lookup by id checks the kind too (`act_on_issue` already does, `gitlab_refresh.rs:162-166`), and the UI `busy` key must carry the kind.
- `apply_items_round` returns every `(source, kind)` row of all scopes (`items.rs:264-300`): the MR round keeps only rows whose `scope` equals the configured host before partitioning; `active_scopes = [host]` lets rows of an old host be pruned after 30 days hidden.
- The full list kept in `kept_full` and passed to the "To do" reconcile is unpartitioned (`gitlab_refresh.rs:640-704`), so a hidden MR stays in `merge_requests`/`review_requests` there and never ticks its "To do" entry; the reconcile chain change is defensive only.
- `lib.rs` declares every module `pub mod`: new items are `pub` (or have a non-test caller in the same phase), so `cargo clippy --all-targets -- -D warnings` never fails on dead code between phases.
- Adding a field to `AssignedItems` breaks about 32 struct literals (`gitlab/fetch.rs`, `gitlab/mod.rs`, `gitlab_refresh.rs`, `new_assignments/tests.rs`, `todo/commands.rs`, `todo/tests.rs`, `tests/hidden_issues_round.rs`, `tests/new_assignments.rs`, `tests/todo.rs`) and `FilteredOut` 7 (`gitlab/mod.rs`, `gitlab_refresh.rs`); the compiler lists each one.

## What We're NOT Doing

- **No migration** and no change to `hidden_issues` (table, SQL, `HiddenRecord`, `ChangeReason`, `SETTLE_MS`, `PRUNE_AFTER_MS`).
- **No second per-MR call to read a merged/closed state (option O1, default chosen by agent):** a hidden MR that is merged or closed leaves the open lists, is marked absent and pruned after 30 days; it comes back as "Zmienione" ("wróciło na listę") only if it is reopened and listed again, the same rule as GitLab and GitHub issues. With `closedDays` on, a merged hidden MR shows in "Zamknięte". Showing a merge or close as "Zmienione" (option O2: a second read-only `glab api projects/<id>/merge_requests/<iid>` call for absent hidden MRs) is a possible follow-up the user may ask for.
- **Draft/ready is not a state change (default chosen by agent):** the user's rule names only state and comments, and GitLab writes draft toggles as system notes, which never count.
- No hiding in the "Zamknięte" section; no change to the recently-closed call.
- No `glab api user` call to learn the user's login (out of scope; would be another new call): without `gitlab.user` every author counts, as for issues, and the existing notice covers it.
- No change to the NEW flags, seen keys or toasts beyond keeping a hidden MR never NEW (`mark_fresh`); `new_assignments` is otherwise untouched (S-03 may generalise it).
- No change to `find_gitlab_item`: a hidden MR cannot be added to "To do" (no "+" on a hidden row unless it is already on the list), same as hidden issues (default chosen by agent, orchestrator guidance 4).
- No CSP change, no new setting, no edit of `context/foundation/roadmap.md`, no new FR number.

## Implementation Approach

Bottom-up in six phases, each committed with all gates green by one implementer:

1. The `glab` MR-notes call, its guard shape, fake-cli routes and `fetch_merge_request_notes` (no caller yet).
2. The snapshot model: `AssignedItems.hidden_merge_requests`, `FilteredOut.hidden_merge_requests`, the TS mirror, the literal fallout, and the places that enumerate the lists (`filter_shown`, `all_items`, `mark_fresh`, `reconcile_gitlab`).
3. The MR hiding core: the kind-aware fingerprint, `hidden_issues/merge_requests.rs` (records of one host, both kinds' records read together, the MR partition, the store-item builders).
4. The MR round `hidden_issues/mr_round.rs` (`check_mr_round` with no lock, `commit_mr_round` in one write) and its fake-cli integration tests.
5. Refresh wiring and commands: `act_on_item` replaces `act_on_issue`, every re-shape reads both kinds' records, the round runs both checks and commits, three new commands.
6. UI, PRD line and `AGENTS.md`.

Decisions taken as defaults (each also noted where it applies):

- **Kind-aware `HideItem for GitlabItem`** (issue: scoped labels; MR: empty). **Default chosen by agent:** one impl, no newtype; issues keep the exact same fingerprint, and an MR's labels never bring it back (user's rule).
- **Reuse `IssueRef`** for the MR notes call (project id + iid). **Default chosen by agent:** same two opaque numbers with the same no-zero and opaque-`Debug` rules; a new type would add code and guard surface for no safety gain. Its doc gains "also a merge request's project and iid".
- **Two independent rounds, two writes** (issues in `hidden_issues`, MRs in `hidden_items`), each one write, both under the same `hidden_gate`. **Default chosen by agent:** the tables are independent and the existing functions each own one write (GitHub precedent); a failure in one leaves the other consistent and is shown by the same notice. Separate `resume_after` cursors; the MR checks run even when the issue checks stopped (the list fetch just succeeded, so a VPN drop costs at most one more timeout).
- **One record per global MR id; partition both MR lists by the same records; hidden MRs in one list `hidden_merge_requests`** shown in the main table under the one "Pokaż ukryte (n)" toggle. **Default chosen by agent (orchestrator guidance 3):** the main table already mixes kinds under the "Typ" switch and owns the toggle; one list needs one field, one filter count and one place in the UI; the "Do review" panel then lists visible review MRs only, and a hidden review MR keeps its "Do review" tag.
- **Separate commands by kind** (`hide_gitlab_merge_request`, `unhide_gitlab_merge_request`, `dismiss_gitlab_merge_request_change`, each by global `id`). **Default chosen by agent:** mirrors the issue and GitHub commands and keeps the issue commands' contract unchanged.
- **UI error texts for MRs** come from an optional `kind` argument of `hiddenIssueErrorMessage`; issue texts stay word for word. **Default chosen by agent:** issue behaviour unchanged, MR texts correct.
- **Notice and footnote wording** widen to cover MRs ("ukrytych pozycji"; a second footnote sentence for MRs). **Default chosen by agent:** one notice per failure, as today, with words that are true for both kinds.

## Critical Implementation Details

- **Lock order is unchanged** (`gitlab_refresh.rs:9-16`): `check_mr_round` runs `glab` with no lock, right after `check_round`; `commit_mr_round` runs under `hidden_gate` right after `commit_round`; the store lock is taken inside the gate but never while a `kept_*` lock is held. Every re-shape (command, failed round) reads issue and MR records in one `with_store` call through `list_gitlab`, before any `kept_*` lock.
- **Every path that re-partitions must partition both kinds.** After this slice a path that re-shapes with issue records only would show hidden MRs as visible. `shape_items`, `reshape_kept`, `act_on_item` and the round all take a `GitlabHidden { issues, merge_requests }`.
- **Logging:** kinds and plain counts only. Never a host, reference, id, project id, iid, login, title or note author.

## Rules for Implementers (binding for every phase)

- Read `AGENTS.md` first; its Hard Rules are binding: no secrets; `glab` read-only and only through a fixed `CliCall` variant covered by the guard test; no work data in the repo, tests, fixtures or logs (invented data only: host `gitlab.example.invalid`, projects `acme/*`, logins `invented.user`/`other.invented`); never an empty list in place of a failed source; the app writes only its own state.
- Work only in `D:\my-projects\day-navigator\.claude\worktrees\gitlab-hide-merge-requests` (branch `slice/gitlab-hide-merge-requests`). Never touch the main checkout, the S-03 worktree or `context/changes/settings-button/`.
- Run `npm install` once. If it adds `"license": "MIT"` to `package-lock.json`, revert it (`git checkout -- package-lock.json`) and never commit that change.
- Before every commit run all gates: `npm run lint`, `npm run format:check`, `npm run build`, and in `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (allow up to 10 minutes). Never commit on red.
- One Conventional Commit per phase (e.g. `feat(gitlab): ...`); tick the phase's Automated Progress rows as `progress-format.md` describes. Every commit message ends with exactly: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
- Forbidden: `git push`, any merge or rebase, `--no-verify`, `/10x-archive`, editing `context/foundation/roadmap.md`, `npm run tauri dev` or starting the app, touching `%APPDATA%`/`%LOCALAPPDATA%` app folders or the real registry, a migration, new FR numbers.
- Manual Progress rows stay unticked; only the user checks them.
- Keep edits to the files shared with S-03 small and append-only (see "Parallel Slice S-03").

## Phase 1: Read-only MR notes call, guard, fake routes and fetch

### Overview

Adds `glab api projects/<id>/merge_requests/<iid>/notes?sort=desc&order_by=created_at&per_page=100 --paginate --hostname <host>` as a new `CliCall` variant with fixed arguments, covered by the read-only guard test, with fake-cli routes on invented data and a fetch function. No caller in the app yet.

### Changes Required:

#### 1. CLI call and guard

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: One new read-only call, built like the issue notes call; the guard accepts exactly one more endpoint shape.

**Contract**:
- Constant `pub const MR_NOTES_MIDDLE: &str = "/merge_requests/";` next to `ISSUE_NOTES_*` (`:349-351`); the prefix and suffix are shared. A private `mr_notes_args(host, &IssueRef)` builds `["api", "projects/<project_id>/merge_requests/<iid>/notes?sort=desc&order_by=created_at&per_page=100", "--paginate", "--hostname", <host>]`.
- `IssueRef` doc gains: "also a merge request's project id and iid (the MR notes call)".
- Variant appended at the end of `CliCall`: `/// GET of one merge request's notes, newest first (read-only).` `GlabMergeRequestNotes(GitlabHost, IssueRef)`; `tool()` = `Tool::Glab`; `name()` = `"glab:mr-notes"`; `args()` = `mr_notes_args`. Not in `CliCall::ALL`.
- `examples()` appends one `GlabMergeRequestNotes(host, IssueRef { project_id: 77, iid: 34 })` (place it right after the `GlabIssueNotes` example; it takes the cloned host, the issue-notes example keeps the move or both clone).
- Test module: `VARIANT_COUNT` 13 → 14, `EXAMPLE_COUNT` 18 → 19, `ordinal` arm `CliCall::GlabMergeRequestNotes(..) => 13`, `seen` array gains `, 1` at the end, the "ALL must stay parameterless" `matches!` list gains `| CliCall::GlabMergeRequestNotes(..)`.
- `notes_endpoint_allowed`: after stripping `projects/` and the suffix, accept iff splitting on `/issues/` gives two `guard_number_ok` parts, **or** splitting on `/merge_requests/` gives two `guard_number_ok` parts. Doc comment names both shapes.
- New tests (beside `notes_call_has_exact_arguments_name_and_tool` and `guard_accepts_the_notes_shape_and_rejects_near_misses`): exact args, name, tool and opaque `Debug` (no `77`, no host) for the MR call; the guard accepts `projects/77/merge_requests/34<SUFFIX>` and rejects: `merge_requests/0`, `merge_requests/034`, `projects/077/merge_requests/34`, `projects//merge_requests/34`, `projects/77/merge_request/34` (singular), `projects/77/merge_requests/34/extra`, `projects/77/merge_requests/34/notes/5?...`, `projects/77/merge_requests/34/issues/2`, `projects/77/issues/1/merge_requests/34`, `sort=asc` instead of `desc`, an extra `&x=1`, a missing `per_page`, and the right endpoint followed by `-X POST` or `--method POST`. A proptest like the issue one: every `(project_id, iid)` in `1..=u64::MAX` passes the guard through `CliCall::args()`.

#### 2. Fetch function

**File**: `src-tauri/src/gitlab/fetch.rs`

**Intent**: Run the call and parse it with the unchanged issue-notes parser (same fields, research section 4).

**Contract**: `pub fn fetch_merge_request_notes(runner: &CliRunner, host: &GitlabHost, mr: IssueRef) -> Result<Vec<IssueNote>, GitlabError>`, a sibling of `fetch_issue_notes` (`:119-126`): `runner.run(&CliCall::GlabMergeRequestNotes(host.clone(), mr))?` then `parse_notes`.

#### 3. Fake CLI routes

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Serve MR notes for the existing notes scenarios, told apart from issue notes by the path.

**Contract**:
- `Endpoint` gains `MrNotes { project_id: u64, iid: u64 }` (`projects/<p>/merge_requests/<i>/notes?...`). `parse_notes_endpoint` returns `Notes` for `/issues/` and `MrNotes` for `/merge_requests/`.
- Constants `MR_NOTES_KEYS: [(u64, u64); 2] = [(102, 34), (104, 21)]` with a doc: MR 6002 `acme/widgets!34` (assigned and awaiting review) and MR 6101 `acme/gadgets!21` (review-only) have notes; every other MR, including 6003 `(102, 12)`, gets `[]`.
- `gitlab_notes_body` and `gitlab_notes_scenario` treat `MrNotes` exactly like `Notes`, with "mine" = the pair is in `MR_NOTES_KEYS` (instead of `(NOTES_PROJECT_ID, NOTES_IID)`): same notes per scenario (`-other`, `-own`, `-system`, `-none`, `two-pages`), `gitlab-notes-unavailable` fails every MR notes call, `gitlab-notes-one-unavailable` fails every MR not in `MR_NOTES_KEYS`, `gitlab-notes-bad-json` serves the truncated body. Each MR notes call is recorded by `record_notes_call` like an issue notes call. Unknown argument shapes keep exiting 98.

#### 4. Tests

**File**: `src-tauri/tests/gitlab_fetch.rs` (append)

**Intent**: The fetch through a real process, invented data only.

**Contract**: with `gitlab-notes-other`: MR `(102, 34)` returns two notes with the authors and order of the issue case; MR `(102, 12)` returns no notes (the issue `(102, 12)` still returns its notes: the route is told apart by path); `gitlab-notes-unavailable` → `GitlabError::Unavailable`; `gitlab-notes-bad-json` → `ResponseInvalid` with no `BAD_NOTES_MARKER` in the error's `Debug`/`Display`. `tests/cli_runner.rs` keeps passing over the grown `examples()`.

### Success Criteria:

#### Automated Verification:

- Guard tests pass with the MR shape accepted and every near-miss rejected: `cargo test cli::`
- MR notes fetch tests pass through fake-cli: `cargo test --test gitlab_fetch`
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- Once, against the real work GitLab, the user runs in a terminal `glab api "projects/<id>/merge_requests/<iid>/notes?sort=desc&order_by=created_at&per_page=100" --paginate --hostname <host>` for one MR with a comment and checks that each note has `author.username`, `system` and `created_at` (RFC 3339); nothing from the output is saved, pasted or committed

**Implementation Note**: Commit when all automated rows are green; the Manual row stays unticked for the user. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 2: Snapshot model and list enumerations

### Overview

The snapshot gets a hidden-MR list and its filter count, with the TS mirror in the same commit. Every place that enumerates `AssignedItems` learns about the new list. Nothing fills it yet (always empty), so behaviour is unchanged.

### Changes Required:

#### 1. Rust model

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Carry hidden MRs to the UI the way `hidden_issues` carries hidden issues.

**Contract**:
- `AssignedItems` gains, after `review_requests`, `/// Merge requests the user has hidden, assigned or awaiting review (S-07). Empty in a fetched list; the round fills it when it partitions both MR lists.` `pub hidden_merge_requests: Vec<GitlabItem>` (serde `hiddenMergeRequests`). The `review_requests` doc "never hidden" becomes "the visible ones after the round partitions them (S-07)". The `GitlabItem.hidden_at_ms` doc becomes "Only on `hidden_issues` and `hidden_merge_requests` items".
- `FilteredOut` gains `pub hidden_merge_requests: u32` after `review_requests`.
- Every `AssignedItems { .. }` and `FilteredOut { .. }` literal in `src/` and `tests/` gains the new field with an empty value (`Vec::new()`/`vec![]`/`0`); no assertion changes. The compiler lists them (see Key Discoveries).

#### 2. Enumerations

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: The visibility filter applies to the new list and counts what it removed.

**Contract**: `filter_shown` (`:277-296`) applies `filter.apply` to `shown.hidden_merge_requests` and fills `FilteredOut.hidden_merge_requests`. `shape_items` and the round build `AssignedItems` with `hidden_merge_requests: Vec::new()` in this phase (Phase 5 fills it).

**File**: `src-tauri/src/new_assignments/mod.rs`

**Intent**: A hidden MR keeps its seen key and is never NEW. Touch only these two functions (S-03 shares this module).

**Contract**: `all_items` (`:62-69`) chains `&items.hidden_merge_requests` after `&items.review_requests`; `mark_fresh` (`:126-140`) sets `is_new = false` on every `hidden_merge_requests` item (like `hidden_issues`); its doc says "hidden issues and hidden merge requests are never new".

**File**: `src-tauri/src/todo/mod.rs`

**Intent**: A hidden MR counts as present, so hiding never ticks its "To do" entry (defensive: the reconcile input is the unpartitioned full list).

**Contract**: the `present` map in `reconcile_gitlab` (`:579-586`) chains `items.hidden_merge_requests.iter()`; the doc comment above the function says "An issue or merge request the user hid ... counts as present".

#### 3. TS mirror

**File**: `src/gitlab/types.ts`

**Intent**: Same commit as the Rust types (AGENTS.md).

**Contract**: `AssignedItems.hiddenMergeRequests: GitlabItem[]` (doc: "Merge requests the user has hidden, assigned or awaiting review."), `FilteredOut.hiddenMergeRequests: number`; the `reviewRequests` doc says "visible ones"; the `hiddenAtMs` doc says "only on hidden issues and merge requests"; `GitlabSnapshot.hiddenCheckFailed` doc says "Hidden issues and merge requests whose comment check failed in the last round." The view compiles unchanged.

#### 4. Tests

**Files**: `src-tauri/src/gitlab_refresh.rs` (unit), `src-tauri/src/new_assignments/tests.rs`, `src-tauri/src/todo/tests.rs`

**Intent**: The new list is filtered, counted, never NEW, keyed and present.

**Contract**: `filter_shown` drops a non-matching hidden MR and counts it in `hidden_merge_requests`; `mark_fresh` clears `is_new` on a hidden MR whose key is in `fresh`; `keys_of` includes a hidden MR's key; `reconcile_gitlab` leaves the "To do" entry of an MR that is only in `hidden_merge_requests` unticked.

### Success Criteria:

#### Automated Verification:

- New and existing unit tests pass: `cargo test gitlab_refresh::`, `cargo test new_assignments::`, `cargo test todo::`
- Type check and build pass with the new TS fields: `npm run build`
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 3: MR hiding core (fingerprint, records, partition)

### Overview

The MR-specific pieces the round, the commands and the refresh wiring share, as tauri-free functions with unit tests. No caller in the app yet (all `pub`).

### Changes Required:

#### 1. Kind-aware fingerprint

**File**: `src-tauri/src/hidden_issues/rules.rs`

**Intent**: Labels bring an issue back, never an MR.

**Contract**: `impl HideItem for GitlabItem::status_fingerprint` returns `scoped_labels(self.label_names())` for `ItemKind::Issue` and `Vec::new()` for `ItemKind::MergeRequest` (exhaustive `match`, no `_` arm), with a doc line. `hide_id`/`updated_at` unchanged.

#### 2. MR records, both kinds' records, partition, store items

**File**: `src-tauri/src/hidden_issues/merge_requests.rs` (new; `pub mod merge_requests;` in `hidden_issues/mod.rs`, re-export the public items)

**Intent**: One place for what an MR hide record is and how both GitLab kinds' records travel together.

**Contract**:
- `#[derive(Debug, Clone, Default, PartialEq, Eq)] pub struct GitlabHidden { pub issues: Vec<HiddenRecord>, pub merge_requests: Vec<HiddenItemRecord> }` — the GitLab hide records of one host, both kinds.
- `pub fn list_mr_records(store: &Store, host: &GitlabHost) -> Result<Vec<HiddenItemRecord>, HiddenIssueError>`: `list_items(store, HiddenSource::Gitlab, HiddenKind::MergeRequest)` keeping only `scope == host.as_str()` (order by `item_id`, as stored).
- `pub fn list_gitlab(store: &Store, host: &GitlabHost) -> Result<GitlabHidden, HiddenIssueError>`: `list(store, host)` and `list_mr_records(store, host)`; the first error wins.
- `pub fn new_hidden_mr(host: &GitlabHost, item: &GitlabItem) -> NewHiddenItem`: `source: Gitlab`, `scope: host.as_str().to_string()`, `kind: MergeRequest`, `item_id: item.id`, `project_id: Some(item.project_id)`, `iid: Some(item.iid)`, `status_fingerprint: item.status_fingerprint()` (empty for an MR), `baseline_updated_at: item.updated_at.clone()`.
- `pub fn mr_key(host: &GitlabHost, id: u64) -> HiddenItemKey` (Gitlab, host, MergeRequest, id).
- `pub struct MrPartition { pub merge_requests: Vec<GitlabItem>, pub review_requests: Vec<GitlabItem>, pub hidden: Vec<GitlabItem> }` and `pub fn partition_merge_requests(merge_requests: &[GitlabItem], review_requests: &[GitlabItem], records: &[HiddenItemRecord]) -> MrPartition`: per item of each list, matched by `item_id == item.id` (records are MR records of one host): no record → visible with `changed: None, hidden_at_ms: None`; hidden record → `hidden` with `hidden_at_ms: Some(r.hidden_at_ms)`; marked record → visible with `changed: r.changed_reason`, `hidden_at_ms: None`. `hidden` holds the hidden assigned MRs first, then the hidden review-only MRs, each in list order; every other field (including `review_requested`) is kept.

#### 3. Tests

**Files**: `src-tauri/src/hidden_issues/tests.rs` (unit), `src-tauri/tests/hidden_items.rs` (append)

**Intent**: The rules of this phase, invented data only.

**Contract**:
- `status_fingerprint`: an issue with `a::b` keeps `["a::b"]`; an MR with the same labels gives `[]`; `status_changed` is false for an MR record whose labels changed.
- `partition_merge_requests`: order kept in all three lists; a hidden review-only MR lands in `hidden` with `review_requested` kept; a marked assigned MR stays in `merge_requests` with `changed`; `hidden_at_ms` only on hidden items; an MR whose id equals a record of another id is untouched.
- Store (`tests/hidden_items.rs`, test store): `hide_item(new_hidden_mr(..))` then `list_mr_records` returns it with `project_id`/`iid` set and an empty fingerprint; a row of another host and a GitHub row are not returned; `list_gitlab` returns a hidden issue (`hidden_issues`) and a hidden MR together; `unhide_item(mr_key)` removes it; `dismiss_item` on a hidden (unmarked) MR is `NotFound`; hiding an MR changes no `hidden_issues` row.

### Success Criteria:

#### Automated Verification:

- Fingerprint and partition unit tests pass: `cargo test hidden_issues::`
- Store tests pass: `cargo test --test hidden_items`
- Existing GitLab issue hiding tests pass unchanged: `cargo test --test hidden_issues --test hidden_issues_round --test hidden_issues_crash`
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 4: The MR round

### Overview

The GitLab MR counterpart of `round.rs`, on the shared rules and the `hidden_items` store: a check half with no lock and a commit half in one write. Not wired into the refresh thread yet.

### Changes Required:

#### 1. Shared helpers

**File**: `src-tauri/src/hidden_issues/round.rs`

**Intent**: Reuse the stop rule and the resume order instead of copying them; issue behaviour unchanged.

**Contract**: `stops_round` becomes `pub(super)`; `check_order` becomes `pub(super) fn check_order<R: HideRecord>(records: &[R], resume_after: Option<u64>) -> Vec<&R>` (partition by `item_id() <= cursor`, same order as today).

#### 2. MR round

**File**: `src-tauri/src/hidden_issues/mr_round.rs` (new; `pub mod mr_round;` in `hidden_issues/mod.rs`, re-export the public items)

**Intent**: Every rule of the issue round, for MRs, over both MR lists.

**Contract**:
- `#[derive(Debug)] pub struct MrRoundChecks { records: Result<Vec<HiddenItemRecord>, HiddenIssueError>, checks: HashMap<u64, Result<CommentVerdict, ()>>, stopped_at: Option<u64> }` with `pub fn stopped_at(&self) -> Option<u64>`.
- `pub fn check_mr_round(store: &StoreHandle, runner: &CliRunner, config: &GitlabConfig, full: &AssignedItems, now_ms: i64, resume_after: Option<u64>) -> MrRoundChecks`: reads `list_mr_records(s, &config.host)` (a failure skips every call); for each record in `check_order(.., resume_after)` finds the listed MR by id in `full.merge_requests` then `full.review_requests` (kind `MergeRequest`); skips it when not listed, when `status_changed` or when not `needs_comment_check`; an unreadable baseline is `Err` without a call; otherwise `fetch_merge_request_notes(runner, &config.host, IssueRef { project_id: item.project_id, iid: item.iid })` → `Comment::from` each note → `comment_verdict(.., baseline, config.user.as_ref().map(GitlabUser::as_str))`; after the first `stops_round` error no further call starts, `stopped_at` is that record and every remaining due check is `Err`. Logs only `gitlab: hidden mr comment checks failed: {n}` at debug.
- `#[derive(Debug)] pub struct MrRoundOutcome { pub merge_requests: Vec<GitlabItem>, pub review_requests: Vec<GitlabItem>, pub hidden_merge_requests: Vec<GitlabItem>, pub check_failed: u32, pub store_error: Option<HiddenIssueError> }`.
- `pub fn commit_mr_round(store: &StoreHandle, host: &GitlabHost, full: &AssignedItems, checks: MrRoundChecks, now_ms: i64) -> MrRoundOutcome`: listed = `full.merge_requests` followed by `full.review_requests`; `plan_round(&records, &listed, &verdicts, now_ms)`; one `apply_items_round(store, Gitlab, MergeRequest, &[(host.as_str().to_string(), transitions)], &[host.as_str().to_string()], now_ms)`; keeps the returned rows with `scope == host.as_str()`; `partition_merge_requests`; `check_failed` counts only failed verdicts whose record is still hidden after the write. On a store read or write failure: both MR lists as listed with `changed: None` and `hidden_at_ms: None`, `hidden_merge_requests: []`, `check_failed: 0`, `store_error` set, and `log::warn!("gitlab: hidden mr state unavailable: {}", e.kind())`. Never an empty list in place of the listed MRs.

#### 3. Tests

**File**: `src-tauri/tests/hidden_mr_round.rs` (new, modelled on `tests/hidden_issues_round.rs`)

**Intent**: Every rule of the MR round through `fake-cli` and a test store, invented data only.

**Contract**: fixtures from fake-cli: MR 6002 `(102, 34)` assigned and awaiting review (in `merge_requests` with `review_requested`), MR 6101 `(104, 21)` review-only, MR 6003 `(102, 12)` assigned without notes. MRs are hidden through `hide_item(new_hidden_mr(..))` with an `updated_at` older than the listed one, so a check is due. Rounds call `check_mr_round` + `commit_mr_round` as the refresh thread will. Cases:
- `gitlab-notes-other`: hidden 6101 (review-only) → visible in `review_requests` with `changed = Comment`; hidden 6002 → visible in `merge_requests` with `changed = Comment`.
- `gitlab-notes-own` → stays hidden, baseline = listed `updated_at`, settle check set; the next round makes no notes call.
- `gitlab-notes-system` (a system note by someone else, e.g. an approval or a draft toggle) → stays hidden.
- Labels changed since hiding (edit the listed MR's labels in the test) with `gitlab-notes-none` → stays hidden (no `Status` for MRs).
- Drop-off: remove 6101 from `review_requests` → `absent_since_ms` set, not resurfaced; listed again → `changed = Returned`; absent for `PRUNE_AFTER_MS` → record deleted.
- Moving sections: hidden 6101 listed in `merge_requests` instead of `review_requests` → still hidden, no absent mark.
- `gitlab-notes-unavailable` with two due MRs → one notes call, `check_failed = 2`, `stopped_at` set; the next round with `resume_after` checks the other MR first (`gitlab-notes-one-unavailable` and the notes call log, as `a_call_that_keeps_failing_does_not_starve_the_checks_after_it`).
- `gitlab-notes-bad-json` → failed check, no `BAD_NOTES_MARKER` in any `Debug` of the outcome.
- Without a configured user, `gitlab-notes-own` resurfaces (`Comment`).
- An unreadable baseline → failed check, no call.
- A store failure (`StoreHandle::new(None)`) → no call, both lists as listed, `hidden_merge_requests` empty, `store_error` set.
- A row of another host (hidden 31+ days ago) is pruned; a row of another host hidden recently is kept and never shown.
- A hidden MR and a hidden issue with the same global id: hiding the MR never hides the issue and the round never touches the `hidden_issues` row.

### Success Criteria:

#### Automated Verification:

- MR round integration tests pass: `cargo test --test hidden_mr_round`
- GitLab issue round tests pass unchanged: `cargo test --test hidden_issues_round`
- No leak marker (title, note body) in any error or `Debug` output in these tests
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 5: Refresh wiring and commands

### Overview

The GitLab refresh state partitions both kinds everywhere, the round runs the MR checks and commit next to the issue ones, and three commands hide, show and dismiss an MR by its global id.

### Changes Required:

#### 1. Refresh state and shaping

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Every path that shapes the list partitions issues and MRs from one `GitlabHidden`; one entry point for actions on either kind.

**Contract**:
- `shape_items(full, hidden: &GitlabHidden, fresh, filter)`: issues via `partition(&full.issues, &hidden.issues)`, MRs via `partition_merge_requests(&full.merge_requests, &full.review_requests, &hidden.merge_requests)`; fills `merge_requests`, `review_requests` and `hidden_merge_requests` from the MR partition; then `mark_fresh` and `filter_shown` as today.
- `reshape_kept(.., records: Option<&GitlabHidden>, ..)`; in `refresh_round` the kept records are read with `list_gitlab(s, &cfg.host)` instead of `list_hidden`.
- `act_on_issue` is replaced by `pub(crate) fn act_on_item(&self, kind: ItemKind, id: u64, act: impl FnOnce(&GitlabHost, &GitlabItem) -> Result<GitlabHidden, HiddenIssueError>) -> Result<(), HiddenIssueError>`: same gate, locks and snapshot write as today; the item is found by id **and** kind: `Issue` in `full.issues`, `MergeRequest` in `full.merge_requests` then `full.review_requests`; `NotInList` otherwise (before `act` runs); the cap is `hidden_total = shown.hidden_issues.len() + filtered_out.hidden_issues + shown.hidden_merge_requests.len() + filtered_out.hidden_merge_requests`. Doc comment names both kinds.
- `refresh_loop` keeps a second cursor `mr_resume_after: Option<u64>`, passed to `refresh_round` as a new `&mut Option<u64>` parameter.
- `refresh_round`: right after `check_round`, `check_mr_round(&store, runner, cfg, full, now_ms() as i64, *mr_resume_after)` (no lock; updates `mr_resume_after`); `checks` becomes `Option<(RoundChecks, MrRoundChecks)>`. Under the gate, after `commit_round`, `commit_mr_round(&store, &cfg.host, &full, mr_checks, now_ms() as i64)`; the items take `merge_requests`, `review_requests`, `hidden_merge_requests` from the MR outcome; `check_failed` = both counts summed; `hidden_error` = the issue store error, else the MR store error (`message_pl`). Everything else in the round is unchanged (filter, NEW flags, closed section, "To do" reconcile on the full list, interval).
- The module doc's `hidden_gate` paragraph says "hidden-issue and hidden-MR state".

#### 2. Commands

**File**: `src-tauri/src/hidden_issues/commands.rs`

**Intent**: The UI entry for MRs, like the issue commands; the MR data comes from the kept list, never from the UI. The issue commands keep their names, arguments and behaviour, and now re-shape with both kinds' records.

**Contract**:
- The GitLab helper `run` gains a `kind: ItemKind` parameter, calls `act_on_item(kind, id, ..)`, and its closure ends with `list_gitlab(s, host)` instead of `super::list(s, host)`. The three issue commands pass `ItemKind::Issue`.
- `#[tauri::command] pub async fn hide_gitlab_merge_request(app: AppHandle, id: u64) -> Result<(), HiddenIssueError>`: `hide_item(store, &new_hidden_mr(host, item), now)`; `unhide_gitlab_merge_request`: `unhide_item(store, &mr_key(host, item.id))`; `dismiss_gitlab_merge_request_change`: `dismiss_item(store, &mr_key(host, item.id))`. Each passes `ItemKind::MergeRequest`; log names `hide-mr`, `unhide-mr`, `dismiss-mr` (store errors at warn by kind, refusals at debug by kind). Module doc mentions GitLab merge requests by `id`.

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the commands.

**Contract**: three lines appended right after `hidden_issues::commands::dismiss_gitlab_change` in `generate_handler!` (append-only; S-03 may append its own lines elsewhere).

#### 3. Tests

**File**: `src-tauri/src/gitlab_refresh.rs` (unit tests)

**Intent**: The state rules without a Tauri app; the existing tests keep their meaning.

**Contract**:
- Existing `act_on_issue_*` tests move to `act_on_item(ItemKind::Issue, ..)` with a local helper that wraps issue records as `GitlabHidden { issues, merge_requests: vec![] }`; assertions unchanged.
- `act_on_issue_refuses_an_mr_or_an_unknown_id` becomes `act_on_item_matches_id_and_kind`: an issue id asked as `MergeRequest` and an MR id asked as `Issue` are `NotInList`; an MR of `review_requests` and one of `merge_requests` are accepted; an unknown id is `NotInList`.
- New: hiding an MR re-partitions it into `hidden_merge_requests` (from either section) while a hidden issue stays hidden; an MR marked `Comment` stays in its section with `changed`; the cap counts hidden MRs (including filtered-out ones); `reshape_kept` with both kinds' records partitions both; `reshape_kept` with `None` records leaves the shown list as it was.

### Success Criteria:

#### Automated Verification:

- State unit tests pass: `cargo test gitlab_refresh::`
- Round tests still pass: `cargo test --test hidden_mr_round --test hidden_issues_round`
- The three MR commands are registered: `grep -c "_gitlab_merge_request" src-tauri/src/lib.rs` prints 3
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 6: "Praca (GitLab)" UI, "Do zrobienia", PRD and AGENTS.md

### Overview

Row actions on every MR row (main table and "Do review"), hidden MRs under the one toggle, kind-aware busy keys and error texts, the "Do zrobienia" flag and count, the PRD line and the `AGENTS.md` app-data clause.

### Changes Required:

#### 1. API and hook

**File**: `src/gitlab/api.ts`

**Intent**: One typed function per command; MR-worded error texts without changing the issue texts.

**Contract**: `hideGitlabMergeRequest(id)`, `unhideGitlabMergeRequest(id)`, `dismissGitlabMergeRequestChange(id)` → `invoke("hide_gitlab_merge_request" | "unhide_gitlab_merge_request" | "dismiss_gitlab_merge_request_change", { id })`. `hiddenIssueErrorMessage(e, kind: ItemKind = "issue")`: for `"mergeRequest"` `NotFound` → "Merge request nie jest już ukryty — odśwież listę.", `NotInList` → "Merge requesta nie ma na liście — odśwież listę."; every other text (and every issue text) unchanged.

**File**: `src/gitlab/useGitlab.ts`

**Intent**: One busy set for both kinds without id collisions.

**Contract**: `export function busyKey(kind: ItemKind, id: number): string` (`${kind}:${id}`); `busy: ReadonlySet<string>`; `act(kind, id, run)`; `hide`, `unhide`, `dismiss` become `(kind: ItemKind, id: number) => void` and call the issue or MR API function by kind; the error text uses `hiddenIssueErrorMessage(e, kind)`. `TodoView.tsx` (the other caller) does not use these and keeps compiling.

#### 2. View

**Files**: `src/gitlab/GitlabView.tsx` (and `GitlabView.css` only if the layout needs it; no literal colours)

**Intent**: Hiding works the same on issues and MRs, in both sections.

**Contract**:
- `RowActions`: `busy: ReadonlySet<string>`, `hide`/`unhide`/`dismiss: (kind: ItemKind, id: number) => void`. `RowButtons` uses `busyKey(item.kind, item.id)` and passes `item.kind`.
- `GitlabRow` renders `RowButtons` whenever `actions` is given (drop the `item.kind === "issue"` condition, `:235`); the prop comment changes accordingly. The "Do zrobienia" button rule stays (`todoFound || !hidden`).
- `Lists`: hidden rows = `[...items.hiddenIssues, ...items.hiddenMergeRequests]` filtered by kind and view, each `{ item, hidden: true, reviewTag: item.reviewRequested }`; `hiddenCount = items.hiddenIssues.length + items.hiddenMergeRequests.length`; `outByFilter` adds `showHidden && kind !== "issue" ? filteredOut.hiddenMergeRequests : 0`; `viewOptions` input includes `items.hiddenMergeRequests`.
- `ReviewBlock` is unchanged apart from the buttons it now shows (it lists `items.reviewRequests`, the visible review MRs).
- `GitlabView`: `hasHidden` adds `items.hiddenMergeRequests.length + snapshot.filteredOut.hiddenMergeRequests`; the summary's hidden count adds `items.hiddenMergeRequests.length`.
- Texts: the `hiddenError` notice → "Lista pokazuje wszystkie pozycje, bo nie udało się odczytać ukrytych."; the `user` hint → "... aby Twoje własne komentarze nie przywracały ukrytych pozycji."; the footnote keeps its issue sentence and adds "Ukryty merge request wraca, gdy {ktoś inny doda komentarz | pojawi się nowy komentarz} albo wróci na listę po zniknięciu z niej (np. po zamknięciu i ponownym otwarciu)." with the comment that it mirrors `plan_round` and the MR fingerprint in `src-tauri/src/hidden_issues/rules.rs`.

#### 3. "Do zrobienia"

**File**: `src/todo/TodoView.tsx`

**Intent**: A returned MR in "Do zrobienia" shows "zmienione" like a returned issue; the hidden count in the picker counts hidden MRs.

**Contract**: `changedGitlab` iterates `items.mergeRequests`, `items.reviewRequests` and `items.issues` (`:237-240`); `gitlabHidden` = `hiddenIssues.length + hiddenMergeRequests.length` (`:427`). The picker list stays `mergeRequests + reviewRequests + issues` (hidden MRs drop out like hidden issues).

#### 4. PRD line

**File**: `context/foundation/prd.md`

**Intent**: The roadmap's one-line note for MS-33; no FR changed.

**Contract**: one paragraph after the MS-35 line (`:242`): "Ukrywanie MR-ów (przypisanych i z „Do review”) jest dostarczane w M-4 jako MS-33 (roadmapa): ukryty MR wraca jako „Zmienione” po komentarzu kogoś innego albo po powrocie na listę (zmiana stanu), regułą FR-003; zatwierdzenia, commity i pipeline'y go nie przywracają; bez nowego FR."

#### 5. AGENTS.md

**File**: `AGENTS.md`

**Intent**: The app-data list names GitLab merge requests in the hidden-item snapshots. Touch only that clause of the Hard Rules line (S-03 edits other clauses of the same line).

**Contract**: in "hidden-item snapshots (GitLab issues by host and global id; GitHub issues ...; each with a status fingerprint (GitLab scoped labels, empty for GitHub), ...", change "GitLab issues by host and global id" to "GitLab issues and merge requests by host and global id" and "(GitLab scoped labels, empty for GitHub)" to "(GitLab issue scoped labels, empty for GitLab merge requests and GitHub)". The rest of the clause ("no title, URL, comment text or login") stays.

### Success Criteria:

#### Automated Verification:

- Type check and build pass: `npm run build`
- No colour literal added: `git diff $(git merge-base develop HEAD) -- src/gitlab src/todo | grep -E "^\+.*#[0-9a-fA-F]{3,6}\b"` prints nothing
- AGENTS.md names GitLab merge requests in the hidden-item snapshots: `grep -c "GitLab issues and merge requests by host and global id" AGENTS.md` prints 1
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- Hiding an assigned MR removes it from the main table; "Pokaż ukryte (n)" shows it dimmed with "ukryte DD.MM" (today) after the visible rows; "Pokaż na liście" brings it back unmarked
- Hiding a review-only MR from "Do review" removes it from that panel; with the toggle on it shows dimmed in the main table with the "Do review" tag
- An MR that is both assigned and awaiting review, once hidden, is gone from both places and counts once in "Pokaż ukryte (n)"
- A comment by another account on a hidden MR brings it back after the next refresh in its own section with "Zmienione" and the tooltip "nowy komentarz"; "OK, zauważone" clears the mark
- The user's own comment (with `gitlab.user` set), an approval, a new commit, a pipeline run, a label change and a draft/ready toggle each leave a hidden MR hidden
- Closing a hidden MR and reopening it (at least one refresh in between) brings it back marked "Zmienione" with "wróciło na listę"
- A hidden MR that is in "Do zrobienia" keeps its entry (not ticked) and its row keeps the "Do zrobienia" button; a returned MR there shows "zmienione"
- GitLab issue hiding works as before (hide, comment by someone else, label status change, dismiss)
- The built app (`npm run tauri build -- --debug --no-bundle`) shows no CSP violation in the DevTools console, and the app log holds no host, reference, title, login or note text from the hidden checks

**Implementation Note**: Commit when all automated rows are green; the Manual rows stay unticked for the user. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Parallel Slice S-03: Shared Files and Merge Points

Keep edits to shared files small and local: append enum variants, match arms, struct fields and handler lines at the end of their lists; never reorder or reformat unrelated code.

| File / area | S-07 (this) | S-03 `github-new-and-notify` (#30, migration v16) | Expected merge point |
| --- | --- | --- | --- |
| `src-tauri/src/cli/mod.rs` | `GlabMergeRequestNotes`, `MR_NOTES_MIDDLE`, counts 14/19, `ordinal` 13, `seen` +1, `notes_endpoint_allowed` second shape, tests | should add no call | Only if S-03 adds a call: `VARIANT_COUNT`, `EXAMPLE_COUNT`, `ordinal`, `seen`; the second slice to merge renumbers and recomputes |
| `src-tauri/src/bin/fake_cli.rs` | `Endpoint::MrNotes`, `MR_NOTES_KEYS`, MR branch in the notes scenarios | maybe GitHub scenarios | Different functions and scenario arms; keep both |
| `src-tauri/src/lib.rs` `generate_handler!` | 3 MR commands after `dismiss_gitlab_change` | maybe GitHub seen-mark command(s) | Append-only; keep both |
| `AGENTS.md` Hard Rules app-data line | hidden-item snapshot clause only | GitHub seen keys and the GitHub view's seen-until mark (other clauses of the same line) | Same long line: textual conflict likely; keep both edits word for word |
| `context/foundation/prd.md` | one line after the MS-35 line | maybe one line for MS-30 at the same place | Adjacent paragraphs; keep both |
| `src-tauri/src/new_assignments/mod.rs` | `all_items` +1 chain, `mark_fresh` +1 loop | may generalise GitLab/GitHub seen keys | If S-03 moves these functions, re-apply the two additions in their new place |
| `src-tauri/src/store/schema.rs` | none | v16 | None |
| `src-tauri/src/hidden_issues/*` | `rules.rs` fingerprint, `round.rs` helpers, new `merge_requests.rs`, `mr_round.rs`, commands | should not touch | None expected |
| `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/gitlab/`, `src/gitlab/`, `src/todo/TodoView.tsx` | this slice | `TodoView.tsx` maybe (GitHub NEW) | `TodoView.tsx` only if S-03 edits the same lines; keep both |

## Testing Strategy

### Unit Tests:

- Guard: the MR notes shape accepted, every near-miss rejected, proptest over ids.
- Fingerprint: issue labels kept, MR labels ignored.
- `partition_merge_requests`, `filter_shown`, `mark_fresh`, `keys_of`, `reconcile_gitlab` with hidden MRs.
- `act_on_item` (kind check, both sections, cap), `shape_items`/`reshape_kept` with both kinds.

### Integration Tests:

- `tests/gitlab_fetch.rs`: MR notes through fake-cli, told apart from issue notes by path.
- `tests/hidden_items.rs`: MR rows per host, both kinds read together, no `hidden_issues` change.
- `tests/hidden_mr_round.rs`: full MR rounds with fake-cli and a test store.
- Existing `tests/hidden_issues*.rs` and `tests/hidden_items_crash.rs`: unchanged and green.

### Manual Testing Steps:

1. Check the MR notes response shape once with the real GitLab (Phase 1 manual row).
2. In "Praca (GitLab)", hide an assigned MR and a review-only MR; check the toggle, the dimmed rows and the "Do review" tag.
3. From another account, comment on each; refresh; check "Zmienione" with "nowy komentarz" and dismiss.
4. Hide again; approve, push, run a pipeline, toggle draft, comment with your own account; refresh: still hidden.
5. Close and reopen a hidden MR across refreshes: "wróciło na listę".
6. Check "Do zrobienia" with a hidden and a returned MR; check GitLab issue hiding; check the built app's console and log.

## Performance Considerations

A round adds one `glab api .../merge_requests/<iid>/notes` per hidden MR whose `updated_at` moved past its baseline or whose settle check is due. MRs move `updated_at` on pushes, pipelines and approvals, so hidden MRs with CI activity cost more calls than hidden issues; each call advances the baseline, so a quiet MR costs nothing. Calls run sequentially before the gate and stop after the first unavailable/logged-out/missing-tool failure.

## Migration Notes

None: MR records are new rows in `hidden_items` (v15). A database written by this build opens in an older build with the rows ignored (the older build lists only GitHub issue rows). Rolling back needs no data change.

## References

- Research: `context/changes/gitlab-hide-merge-requests/research.md`
- Precedents: `context/archive/2026-11-27-github-hide-resurface/plan.md` (generic rules, `hidden_items`, "Storage shape"), `context/archive/2026-11-27-gitlab-review-requests/plan.md` ("Do review"), `context/archive/2026-11-23-hide-and-resurface/plan.md` (GitLab issue hiding)
- Rules and store: `src-tauri/src/hidden_issues/rules.rs:48-333`, `items.rs:14-275`, `round.rs:52-206`, `github_round.rs:109-330`, `commands.rs:27-90`
- Refresh: `src-tauri/src/gitlab_refresh.rs:86-352, 534-715`
- CLI guard: `src-tauri/src/cli/mod.rs:299, 345-395, 596-720, 1028-1080`; fake: `src-tauri/src/bin/fake_cli.rs:40-71, 243-310, 343-433`
- UI: `src/gitlab/GitlabView.tsx:80-260, 381-420, 452-600, 690-745`, `src/gitlab/useGitlab.ts:92-115`, `src/gitlab/api.ts:25-73`, `src/todo/TodoView.tsx:237-240, 418-433`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Read-only MR notes call, guard, fake routes and fetch

#### Automated

- [ ] 1.1 Guard tests pass with the MR shape accepted and every near-miss rejected
- [ ] 1.2 MR notes fetch tests pass through fake-cli
- [ ] 1.3 All AGENTS.md gates green

#### Manual

- [ ] 1.4 MR notes response shape checked once against the real GitLab, nothing saved

### Phase 2: Snapshot model and list enumerations

#### Automated

- [ ] 2.1 New and existing unit tests pass
- [ ] 2.2 Type check and build pass with the new TS fields
- [ ] 2.3 All AGENTS.md gates green

### Phase 3: MR hiding core (fingerprint, records, partition)

#### Automated

- [ ] 3.1 Fingerprint and partition unit tests pass
- [ ] 3.2 Store tests pass
- [ ] 3.3 Existing GitLab issue hiding tests pass unchanged
- [ ] 3.4 All AGENTS.md gates green

### Phase 4: The MR round

#### Automated

- [ ] 4.1 MR round integration tests pass
- [ ] 4.2 GitLab issue round tests pass unchanged
- [ ] 4.3 No leak marker in any error or Debug output
- [ ] 4.4 All AGENTS.md gates green

### Phase 5: Refresh wiring and commands

#### Automated

- [ ] 5.1 State unit tests pass
- [ ] 5.2 Round tests still pass
- [ ] 5.3 The three MR commands are registered
- [ ] 5.4 All AGENTS.md gates green

### Phase 6: "Praca (GitLab)" UI, "Do zrobienia", PRD and AGENTS.md

#### Automated

- [ ] 6.1 Type check and build pass
- [ ] 6.2 No colour literal added
- [ ] 6.3 AGENTS.md names GitLab merge requests in the hidden-item snapshots
- [ ] 6.4 All AGENTS.md gates green

#### Manual

- [ ] 6.5 Hiding an assigned MR removes it; the toggle shows it dimmed with ukryte DD.MM; Pokaż na liście brings it back
- [ ] 6.6 Hiding a review-only MR moves it from Do review to the hidden rows with the Do review tag
- [ ] 6.7 An MR both assigned and awaiting review is hidden from both places and counted once
- [ ] 6.8 A comment by another account resurfaces a hidden MR with Zmienione (nowy komentarz); OK, zauważone clears it
- [ ] 6.9 Own comment, approval, commit, pipeline, label change and draft toggle leave a hidden MR hidden
- [ ] 6.10 Close and reopen brings a hidden MR back with wróciło na listę
- [ ] 6.11 A hidden MR in Do zrobienia keeps its entry; a returned one shows zmienione
- [ ] 6.12 GitLab issue hiding works as before
- [ ] 6.13 Built app shows no CSP violation and the log holds no host, reference, title, login or note text
