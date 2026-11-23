# Hide and Resurface GitLab Issues — Implementation Plan

## Overview

Roadmap slice S-03 (PRD FR-003, US-02). In "Praca (GitLab)" the user can hide an issue. The app keeps a small snapshot of it in the store. Each successful refresh round compares the snapshot with the current issue. The issue comes back, marked as changed, when one of three things happens:
- a scoped label changes;
- someone other than the user posts a real comment (bots count);
- it drops off the assigned list and later returns (this also covers close and reopen, because the list holds only open issues).

The mark stays until the user dismisses it. Hidden issues are listed in a collapsed "Ukryte (N)" section with a "Pokaż" (unhide) button.

## Current State Analysis

The GitLab list (S-01) is fetched by the `gitlab-refresh` thread every 5 min and on demand (`src-tauri/src/gitlab_refresh.rs:102-209`). Each round runs two `glab api` GETs (MRs, then issues; all-or-nothing: `src-tauri/src/gitlab/fetch.rs:11-30`). The round produces an in-memory `GitlabSnapshot` (`src-tauri/src/gitlab/mod.rs:221-301`) and publishes it as the `gitlab-changed` event. Nothing is persisted and no round is compared with the previous one.

The parser keeps only title, URL, `updated_at`, labels, `references.full` and draft (`gitlab/mod.rs:143-153`). `id`, `iid`, `project_id` and `state` are dropped, so the only key is `reference`, which changes when a project moves. Both list endpoints use `state=opened` (`src-tauri/src/cli/mod.rs:126-135`): a closed issue simply leaves the list.

Nothing reads comments, and nothing knows the user's identity except the optional `gitlab.user` login (`cli/mod.rs:88-124`, `settings.rs:86-134`). That login exists because a group/bot token can be the `glab` login (archived S-01 plan, Addendum A).

The store (F-01) has migrations v1 (`work_entries`) and v2 (`own_tasks`) and no table for hidden issues (`src-tauri/src/store/schema.rs:9-42`). AGENTS.md allows hidden-issue snapshots in the app data dir and names "hidden-issue reappearance" as a Rust-side, unit-tested domain rule.

The CLI read-only guard (`cli/mod.rs:291-794`) accepts only exact argument shapes. Its dynamic endpoint check rejects `/` and `:` (`cli/mod.rs:379-395`), so a per-issue notes endpoint needs a new, explicitly guarded shape.

The UI (`src/gitlab/GitlabView.tsx`) has no per-row actions. `useGitlab.ts` subscribes to `gitlab-changed` and keeps the snapshot with the highest `seq`.

## Desired End State

On the "Praca (GitLab)" view:
- **Hiding:** every issue row has "Ukryj". A hidden issue leaves the "Zgłoszenia" section and appears in a collapsed "Ukryte (N)" section with "Pokaż". Hidden state survives restarts.
- **What resurfaces:** a later successful round brings the issue back into "Zgłoszenia", marked with a reason ("zmiana statusu", "nowy komentarz" or "wróciło na listę"), in these cases:
  - its set of scoped labels (`key::value`) changed ("zmiana statusu");
  - a non-system comment by anyone other than `gitlab.user` appeared after the snapshot;
  - it dropped off the list and came back.
- **Closing:** both list calls use `state=opened` and GitLab issues are only `opened` or `closed`, so a closed issue drops off the list instead of changing state on it. A reopen within 30 days brings it back as "wróciło na listę". The open/closed state is not stored or compared.
- **What does not resurface:** the user's own comments (when `gitlab.user` is set), GitLab system notes and non-scoped label changes.
- **The mark:** "OK" on the mark dismisses it. "Ukryj" on a marked issue hides it again with a fresh snapshot.
- **Failure states:**
  - A failed comment check leaves the issue hidden and shows "Nie udało się sprawdzić N ukrytych". It is re-checked next round.
  - Without `gitlab.user` any new comment resurfaces, and the view says so.
  - An unreadable store shows the whole list plus a notice; it never shows an empty list.
- **Clean-up:** records of issues absent from the list for 30 days are removed.

Verify with the Phase success criteria: `cargo test` (rules, fake-cli round, store, crash), the CLI guard tests, `npm run build`, and the manual check against the real GitLab in Phase 4.

### Key Discoveries:

- `next_snapshot` keeps the previous items on `Unavailable` and error rounds while host and user are unchanged (`gitlab/mod.rs:254-301`). Reconciliation must run only on a successful round, and absence is judged only from a successful full list.
- The refresh thread is a plain background thread. It may call `StoreHandle::with_store` directly (AGENTS.md forbids only *sync commands* from doing so: the health round holds the store lock during reopen/backup).
- The guard is an allowlist of exact shapes with an independent login check (`guard_login_ok`, `cli/mod.rs:369-375`). The new notes shape gets its own independent digits-only check in the same style, plus the per-character test and proptest pattern (`cli/mod.rs:575-596`).
- `fake-cli` picks a response by endpoint prefix (`src/bin/fake_cli.rs:31-41`); anything else exits 98. The notes endpoint needs a new arm. Canned JSON is inline with `gitlab.example.invalid` data.
- Record-module pattern to copy: `src-tauri/src/work_time/` (`mod.rs` ops in one `store.write` each, `rules.rs` pure, `error.rs` value-free, `tests.rs`). Crash harness: `tests/common/crash.rs` as used by `tests/own_tasks_crash.rs`.
- UI action pattern to copy: `src/own-tasks/useOwnTasks.ts` (`mutate`, busy set, inline `notice` with `role="alert"`). `storeErrorMessage` (`src/own-tasks/api.ts:36-57`) maps store errors to Polish.
- `glab api` expands placeholders like `:id` from the working directory's repo. Numeric path segments, with `:` still forbidden, avoid that.
- `glab api --paginate` never merges pages: with stdout piped it writes each page's array in turn, so the output is `[...][...]` (glab `internal/commands/api/api.go`, its tests pin `[{"id":1}][{"id":2}]`). Today's `serde_json::from_str::<Vec<_>>` (`gitlab/mod.rs:174`) fails on a second page. That is a latent S-01 bug for more than 100 assigned items, and a likely one for notes: system notes count toward the 100.
- GitLab throttles the issue touch (`ThrottledTouch`, 1 minute): a comment posted less than 60 s after the issue's last `updated_at` bump leaves `updated_at` unchanged. Scoped-label changes do bump `updated_at` (subject to the same throttle), but they are not notes. A `updated_at`-only gate can therefore miss a quick reply.

## What We're NOT Doing

- Hiding merge requests. Only issues get "Ukryj". MRs always show (FR-003 speaks of issues).
- Treating GitLab system notes ("changed milestone", "mentioned in !12", label/assignee activity) as comments.
- Resurfacing on non-scoped label changes, title/description edits, milestone or assignee changes.
- Calling `glab api user` or any other new identity call. "The user" is `gitlab.user`; without it every new comment resurfaces.
- Fetching hidden issues that dropped off the list (no per-item state query). Their return is detected when they reappear.
- Applying the S-02 visibility filter (not built). When S-02 lands, its filter applies after this partition and never to the reconciliation input.
- Notifying about resurfaced issues (S-04 is separate).
- Storing titles, URLs, references or comment text. The store row holds ids, scoped labels and timestamps only.
- Writing to GitLab in any form.

## Implementation Approach

The comparison is a domain rule in Rust (new `hidden_issues` module: pure rules plus store ops). The refresh round drives it after a successful fetch, in this order:
1. Read the hide records for the host (`check_round`, no lock).
2. Decide which hidden issues need a comment check: only those still listed whose `updated_at` moved past the stored baseline, or whose settling check is due (see Throttled touch).
3. Run those notes calls with no lock held, stopping at the first unavailable/logged-out/missing-tool failure.
4. Under `hidden_gate`: apply every resulting transition in one store `write`, re-read the rows in that write and partition the full issue list from them into visible (with marks) and hidden (`commit_round`); then build and store the snapshot.
5. Release the gate and publish.

Hide, unhide and dismiss are async commands. They change one row, then re-partition the in-memory full list and publish, without a GitLab round.

The key is GitLab's global issue `id` plus the host. The comment baseline is the issue's `updated_at` as last seen: a comment is new when its `created_at` is later than the baseline.

## Critical Implementation Details

**State sequencing.** A round reads records before its notes calls, and the user can hide, unhide or dismiss meanwhile. Transitions are therefore re-validated inside the single write against each row as it is then; a row that was removed or re-hidden in the meantime is skipped. Commands and the round's write-partition-store step share one mutex. It is never held across a `glab` call or an `emit`. A late round can therefore never publish a partition older than a command's.

**Baseline advance.** When a check finds only the user's own comments and system notes after the baseline, advance the baseline to the issue's *listed* `updated_at` from this round, not to the newest note time. A comment that lands between the list fetch and the notes fetch is then still newer than the baseline next round.

**Throttled touch.** Because GitLab may not bump `updated_at` for a comment within 60 s of the previous bump, every new baseline gets one settling check. `hide` sets `settle_after_ms = now_ms + 120_000` (local clock: 60 s throttle plus a skew margin). An `AdvanceBaseline` whose listed `updated_at` moved sets it the same way. A check that ran only because settling was due clears it (`settle_after_ms = NULL`), so it runs once. Cost: at most one extra notes call per hide or baseline advance.

**Precedence.** If several reasons apply in one round, the stored reason is `status`, then `comment`, then `returned`. Only one reason is kept.

## Phase 1: Issue identity and the read-only notes call

### Overview

Parse the fields the comparison needs, and add one guarded read-only `glab api` call that lists an issue's notes, with fake-cli scenarios and a value-free parser.

### Changes Required:

#### 1. Item identity

**File**: `src-tauri/src/gitlab/mod.rs` (including the test helpers `item`/`items` at `gitlab/mod.rs:596-609`)

**Intent**: Keep GitLab's global `id`, `project_id` and `iid` from each listed item, so hide records have a stable key and the notes URL can be built. Read multi-page `--paginate` output correctly.

**Contract**:
- `RawItem` gains `id: u64`, `iid: u64` and `project_id: u64`. A missing field is `ResponseInvalid`, as today. `state` is not parsed (see Desired End State, "Closing").
- `GitlabItem` gains `id: u64` (serialized as `id`). It also gains `project_id` and `iid`, which are Rust-only and not serialized; the UI needs only `id`.
- New shared helper `parse_pages<T: DeserializeOwned>(json) -> Result<Vec<T>, GitlabError>`: reads a stream of JSON arrays with `serde_json::Deserializer::from_str(json).into_iter::<Vec<T>>()` and flattens them. Empty or whitespace-only output and any serde error are `ResponseInvalid` with no content. `parse_items` and the notes parser both use it.
- Sorting and `checked_url` are unchanged. MRs parse the same fields.

#### 2. Notes call

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Add the only new process call: a GET of one issue's notes, newest first.

**Contract**:
- New `IssueRef { project_id: u64, iid: u64 }` (built only from parsed JSON numbers, opaque `Debug`).
- New `CliCall::GlabIssueNotes(GitlabHost, IssueRef)`, with log name `glab:issue-notes` and args `["api", "projects/<project_id>/issues/<iid>/notes?sort=desc&order_by=created_at&per_page=100", "--paginate", "--hostname", <host>]`.
- Added to `examples()` but not to `ALL`. `VARIANT_COUNT`, `EXAMPLE_COUNT`, `ordinal()`, the seen array and `gitlab_calls_have_exact_arguments_and_names` are updated.
- Guard: `endpoint_allowed` accepts exactly `projects/<D>/issues/<D>/notes?sort=desc&order_by=created_at&per_page=100`, where `<D>` is checked by an independent test-side helper: ASCII digits only, 1–20 chars, no leading zero.
- Guard: every other endpoint containing `/` is still rejected. The `WRITE_FLAGS` and `BAD_CHARS` rules are unchanged.
- Guard tests: a per-character test and a proptest over `u64` pairs check that every `IssueRef` passes the guard and that near-miss shapes (an extra segment, `:id`, `merge_requests`, a different query) fail.

#### 3. Notes parsing and fetch

**File**: `src-tauri/src/gitlab/notes.rs` (new), `src-tauri/src/gitlab/fetch.rs`

**Intent**: Turn the notes JSON into the three facts the rule needs and nothing else.

**Contract**:
- `IssueNote { author: String, system: bool, created_at: DateTime<FixedOffset> }`, parsed from `author.username`, `system` and `created_at`.
- `body` is never deserialized.
- Parsed through `parse_pages`, so a multi-page (`[...][...]`) response yields all notes.
- A serde or timestamp error becomes `GitlabError::ResponseInvalid` with no content.
- `fetch::fetch_issue_notes(runner, &GitlabHost, IssueRef) -> Result<Vec<IssueNote>, GitlabError>`.

#### 4. Fake CLI

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Make every notes outcome testable with invented data.

**Contract**:
- The `gitlab-assigned` issue/MR JSON gains `id`, `iid` and `project_id`.
- A new endpoint arm for `projects/…/notes?…`.
- Scenarios cover: a comment by another user, the configured user's comments only, a system note only, no notes, notes unavailable (no network) while the list succeeds, invalid notes JSON with a leak marker, and two concatenated pages (`[...][...]`) for both the notes call and the issues list.
- Each scenario returns notes for a fixed project/iid; any other issue gets an empty array.

### Success Criteria:

#### Automated Verification:

- CLI guard and argument tests pass, including the new shape, per-character and proptest cases: `cd src-tauri && cargo test cli::`
- Fetch and runner integration tests pass, including notes scenarios and argv echo: `cd src-tauri && cargo test --test gitlab_fetch --test cli_runner`
- Notes parsing rejects bad JSON without leaking content (leak-marker test): `cd src-tauri && cargo test gitlab::`
- Lint and format are clean: `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings`

**Implementation Note**: After automated verification passes, pause for confirmation before Phase 2.

---

## Phase 2: Hidden-issue store and resurfacing rules

### Overview

Persist hide records (migration v3). Implement the comparison as pure, unit-tested rules, and the store ops that apply them in one write.

### Changes Required:

#### 1. Migration v3

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append a table for hide records. Shipped migrations stay untouched.

**Contract**: `Migration { version: 3, … }` with a comment naming S-03.

```sql
CREATE TABLE hidden_issues (
  host               TEXT    NOT NULL,
  issue_id           INTEGER NOT NULL,
  project_id         INTEGER NOT NULL,
  iid                INTEGER NOT NULL,
  scoped_labels      TEXT    NOT NULL,  -- JSON array of `key::value` labels, sorted
  baseline_updated_at TEXT   NOT NULL,  -- RFC 3339 as GitLab sent it
  settle_after_ms    INTEGER,           -- one settling comment check due (Throttled touch)
  hidden_at_ms       INTEGER NOT NULL,
  absent_since_ms    INTEGER,
  changed_reason     TEXT CHECK (changed_reason IN ('status','comment','returned')),
  changed_at_ms      INTEGER,
  CHECK ((changed_reason IS NULL) = (changed_at_ms IS NULL)),
  PRIMARY KEY (host, issue_id)
);
```

A row with `changed_reason IS NULL` is hidden. A row with a reason is a visible issue carrying a mark. No row means a plain visible issue.

#### 2. Pure rules

**File**: `src-tauri/src/hidden_issues/rules.rs` (new)

**Intent**: Every decision of FR-003/US-02 as functions with no store, clock or process.

**Contract**:
- `scoped_labels(&[String]) -> Vec<String>`: labels containing `::`, sorted and deduplicated.
- `status_changed(&HiddenRecord, &GitlabItem) -> bool`: the scoped label set differs.
- `needs_comment_check(&HiddenRecord, &GitlabItem, now_ms) -> bool`: the issue is hidden (no reason), and either the listed `updated_at` is strictly later than the baseline (compared as parsed instants) or `settle_after_ms` is set and `now_ms >= settle_after_ms`.
- `comment_verdict(&[IssueNote], baseline, Option<&GitlabUser>) -> CommentVerdict { NewFromOthers, NothingNew }`. A note counts when it is non-system, its `created_at` is strictly later than the baseline, and its author is not the user (ASCII case-insensitive). When the user is `None`, any author counts.
- `plan_round(records, listed_issues, checks: map issue_id → Result<CommentVerdict, ()>, now_ms) -> Vec<Transition>`, with:
  - `Transition::Resurface { issue_id, reason }`;
  - `AdvanceBaseline { issue_id, updated_at, settle_after_ms: Option<u64> }` (checked, nothing new): `Some(now_ms + 120_000)` when the listed `updated_at` moved past the baseline, `None` when the check ran only because settling was due;
  - `MarkAbsent { issue_id }` (first round missing);
  - `Prune { issue_id }` (absent ≥ 30 days, i.e. `now_ms - absent_since_ms >= 30 * 86_400_000`);
  - `ClearAbsent` only together with `Resurface { reason: returned }` on reappearance.
  - A failed check (`Err`) yields no transition for that issue.
  - Reason precedence: status, then comment, then returned.
- Marked rows (reason set) get no new reason. They take part only in `MarkAbsent` and `Prune`, and in `ClearAbsent` without a resurface.

#### 3. Store ops and errors

**File**: `src-tauri/src/hidden_issues/mod.rs`, `error.rs`, `tests.rs` (new); `src-tauri/src/lib.rs` (`mod hidden_issues;`)

**Intent**: The only code that reads and writes `hidden_issues`, following `work_time/`.

**Contract**:
- `list(store, host) -> Vec<HiddenRecord>`.
- `hide(store, host, &GitlabItem, now_ms)` upserts a fresh hidden row with `settle_after_ms = now_ms + 120_000`; this replaces a marked row.
- `unhide(store, host, issue_id)` deletes a hidden row; otherwise `NotFound`.
- `dismiss(store, host, issue_id)` deletes a marked row; otherwise `NotFound`.
- `apply_round(store, host, &[Transition], now_ms) -> Vec<HiddenRecord>`: one `store.write`. It re-reads each target row and skips transitions whose row is gone or no longer in the state the plan assumed. In the same write it deletes rows of any other host whose `hidden_at_ms` is 30 days or more before `now_ms` (their absence is never judged), then returns the host's rows as they are after the write.
- `partition(full_issues, records) -> (visible: Vec<GitlabItem> with marks, hidden: Vec<GitlabItem>)`. Both keep the list's sort order. The round always partitions from the rows `apply_round` returned, never from the records read before the notes calls.
- `HiddenIssueError { Store(StoreError), NotFound, NotInList, ListUnavailable }`, with `thiserror` + `Serialize` and `tag = "kind", content = "detail"`. No variant carries ids, labels or the host.

### Success Criteria:

#### Automated Verification:

- Rule unit tests pass. They cover:
  - an equal-timestamp comment (not new);
  - a case-insensitive own comment;
  - `None` user counts any author;
  - a system note is ignored;
  - a non-scoped label change is ignored;
  - scoped label add/remove/swap resurfaces;
  - a settling check is due at exactly `settle_after_ms` (one ms before: not due) and runs even when `updated_at` did not move (a throttled reply is caught);
  - a settling-only check that finds nothing new clears `settle_after_ms`, while a check triggered by a moved `updated_at` sets it again;
  - precedence;
  - absent → returned;
  - the 30-day boundary (one ms before: kept; exactly 30 days: pruned);
  - a failed check gives no transition.
  
  Run: `cd src-tauri && cargo test hidden_issues::`
- Store integration tests pass:
  - v3 applies on a fresh store and on a v2 store with data;
  - hide/unhide/dismiss round trip;
  - hide over a marked row resets it;
  - data survives close and reopen;
  - `apply_round` skips stale transitions and returns the rows after the write;
  - `apply_round` prunes another host's rows hidden 30 days or more ago and keeps younger ones;
  - CHECK constraints back the rules.
  
  Run: `cd src-tauri && cargo test --test hidden_issues`
- Crash test never loses or half-applies a hide record: `cd src-tauri && cargo test --test hidden_issues_crash`
- Errors never contain submitted values (leak test in `hidden_issues::tests`): `cd src-tauri && cargo test hidden_issues::tests`
- Lint and format are clean: `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings`

**Implementation Note**: After automated verification passes, pause for confirmation before Phase 3.

---

## Phase 3: Wiring into the refresh round and commands

### Overview

Run reconciliation in each successful round, extend the snapshot with hidden issues and marks, and add the three user commands with their TS mirrors.

### Changes Required:

#### 1. Round reconciliation

**File**: `src-tauri/src/hidden_issues/round.rs` (new), `src-tauri/src/gitlab_refresh.rs` (including its `snap` test helper at `:230` and the `kept_config` doc comment, which no longer holds once commands read it), `src-tauri/src/health/probe.rs` (the `GitlabSnapshot` test helper at `:289`)

**Intent**: After a successful fetch, compare hidden issues and publish the partitioned list. Failed comment checks never block the list.

**Contract**:
- Two `tauri`-free halves, integration-testable with fake-cli and a temp store:
  - `check_round(store_handle, runner, &GitlabConfig, full: &AssignedItems, now_ms) -> RoundChecks`: reads the records and runs the due notes calls with no lock held. After the first `Unavailable`, `LoggedOut` or `ToolMissing` it starts no further notes call; every remaining due issue counts as a failed check. A store read failure is carried in `RoundChecks` and skips the notes calls.
  - `commit_round(store_handle, host, full: &AssignedItems, RoundChecks, now_ms) -> RoundOutcome { visible_issues, hidden_issues, check_failed: u32, store_error: Option<HiddenIssueError> }`: plans the transitions, runs `apply_round` and partitions from the rows it returns.
- `refresh_round` calls `check_round` with no lock, then takes `hidden_gate` and holds it across `commit_round`, `next_snapshot`, the `kept_config`/full-list update and `*state.write()`. It releases the gate, then publishes.
- `GitlabRefreshState` gains:
  - the last full (unpartitioned) `AssignedItems`, kept with `kept_config` and never serialized;
  - a `hidden_gate: Mutex<()>`, never held across `glab` or `emit`.
- Unsuccessful rounds keep today's `next_snapshot` behaviour, with no reconciliation and no absence marking.
- If the store cannot be read or written, the list shows unpartitioned, with `hiddenError` set.
- Log lines carry kinds and plain counts only.

#### 2. Snapshot shape

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Carry what the view needs to render hidden state.

**Contract**:
- `GitlabItem` gains `changed: Option<ChangeReason>` (`status` | `comment` | `returned`, camelCase).
- `AssignedItems` gains `hidden_issues: Vec<GitlabItem>`.
- `GitlabSnapshot` gains:
  - `hidden_check_failed: u32`;
  - `own_comments_known: bool` (true when `gitlab.user` is set);
  - `hidden_error: Option<String>` (fixed Polish text from `storeErrorMessage`-equivalent kinds, no values).
- `next_snapshot` keeps these with the kept items on unavailable or error rounds.

#### 3. Commands

**File**: `src-tauri/src/hidden_issues/commands.rs` (new), `src-tauri/src/lib.rs` (`generate_handler!`)

**Intent**: Let the UI hide, unhide and dismiss by issue `id`. The issue data for the snapshot comes from the in-memory full list, never from the UI.

**Contract**:
- `hide_gitlab_issue(id: u64)`, `unhide_gitlab_issue(id: u64)` and `dismiss_gitlab_change(id: u64)` are async and run via `crate::blocking`.
- Each takes `hidden_gate`, does one store op, re-partitions the kept full list, stores the snapshot with `seq + 1`, releases the gate and publishes.
- Errors are `HiddenIssueError`:
  - `NotInList` when the id is not an issue in the kept list;
  - `ListUnavailable` when there is no kept list or config.
- No new capability permissions are needed (custom commands).

#### 4. TS mirrors

**File**: `src/gitlab/types.ts`, `src/gitlab/api.ts`

**Intent**: Mirror the Rust changes in the same commit.

**Contract**:
- `GitlabItem.id: number` and `changed: ChangeReason | null`.
- `AssignedItems.hiddenIssues`.
- Snapshot `hiddenCheckFailed`, `ownCommentsKnown` and `hiddenError`.
- `HiddenIssueError` as a tagged union on `kind`.
- `api.ts`: `hideGitlabIssue(id)`, `unhideGitlabIssue(id)` and `dismissGitlabChange(id)`, plus a Polish message mapper.

### Success Criteria:

#### Automated Verification:

- Round integration tests with fake-cli and a temp store pass:
  - an other's comment resurfaces with `comment`;
  - an own comment advances the baseline and stays hidden;
  - a system note stays hidden;
  - notes unavailable gives `check_failed = 1`, the issue stays hidden and the list is still returned;
  - with two due issues and notes unavailable, only one notes call runs and `check_failed = 2`;
  - two concatenated notes pages are read in full (a qualifying comment on the second page resurfaces);
  - a scoped-label change resurfaces without a notes call;
  - drop-off marks absent and return resurfaces with `returned`;
  - with no user configured, any comment resurfaces;
  - a store failure gives the unpartitioned list plus `store_error`.
  
  Run: `cd src-tauri && cargo test --test hidden_issues_round`
- `next_snapshot` and partition unit tests pass, including that hidden fields are kept on unavailable rounds: `cd src-tauri && cargo test gitlab::`
- All Rust tests, lint and format pass: `cd src-tauri && cargo test && cargo fmt --check && cargo clippy --all-targets -- -D warnings`
- Frontend type check and build pass with the new mirrors: `npm run build`

**Implementation Note**: After automated verification passes, pause for confirmation before Phase 4.

---

## Phase 4: UI in "Praca (GitLab)"

### Overview

Row actions, the "changed" mark, the hidden section and the notices.

### Changes Required:

#### 1. View

**File**: `src/gitlab/GitlabView.tsx`, `src/gitlab/useGitlab.ts`, the existing stylesheet

**Intent**: Display results and send actions; no rule logic in the UI.

**Contract**:
- **Issue rows** get "Ukryj" (`aria-label` includes the title), disabled while that row's action runs.
- **Marked issue rows** show a badge "Zmienione: zmiana statusu / nowy komentarz / wróciło na listę" and an "OK" button that dismisses the mark.
- **Hidden section:** below "Zgłoszenia" (omitted when there are no hidden issues), a collapsed `<details>` with the summary "Ukryte (N)" lists the hidden issues with "Pokaż".
- **Notices:**
  - `hiddenCheckFailed > 0`: "Nie udało się sprawdzić N ukrytych — spróbuję przy następnym odświeżeniu".
  - `!ownCommentsKnown` while hidden issues exist: a hint to set `"user"` in the `gitlab` block of settings.json so own comments don't resurface issues.
  - `hiddenError`: a notice that the list shows all issues because hidden state could not be read.
- **Action errors** show inline with `role="alert"` (own-tasks pattern). The new snapshot arrives through the existing `gitlab-changed` subscription.
- **MR rows** are unchanged.

### Success Criteria:

#### Automated Verification:

- Frontend checks pass: `npm run lint && npm run format:check && npm run build`
- The full gate passes before commit: `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

#### Manual Verification:

- With VPN and `gitlab.user` set, "Ukryj" moves an issue into "Ukryte (N)". It stays hidden after an app restart.
- Commenting on the hidden issue yourself keeps it hidden after "Odśwież".
- A comment by someone else or a bot, or a scoped-label move (e.g. `workflow::…`), brings it back with the right reason. "OK" removes the mark.
- "Pokaż" in "Ukryte (N)" brings a hidden issue back unmarked.
- With VPN off, the view stays calm, hidden issues stay hidden and nothing resurfaces.
- MR rows have no "Ukryj". No title, host or label appears in the app log after these steps.

**Implementation Note**: After automated verification passes, pause for manual confirmation from the human that the manual testing was successful.

---

## Testing Strategy

### Unit Tests:

- Rules in `hidden_issues/rules.rs`: every boundary listed in Phase 2 (timestamps strictly later, case-insensitive author, `None` user, system notes, scoped vs plain labels, precedence, 30-day boundary, failed check).
- Guard: the exact notes shape, digit check independent of `IssueRef`, near-miss rejections, proptest over `u64` pairs.
- Parsing: new item fields, notes without `body`, invalid JSON never leaks content.
- Error JSON shape and a leak test for `HiddenIssueError`.

### Integration Tests:

- `tests/hidden_issues.rs`: migration, ops, reopen durability, stale-transition skip.
- `tests/hidden_issues_crash.rs`: crash harness over hide/dismiss writes.
- `tests/hidden_issues_round.rs`: `check_round` + `commit_round` with fake-cli scenarios and a temp store.
- Existing `tests/gitlab_fetch.rs` / `tests/cli_runner.rs` extended for the new fields and call.

### Manual Testing Steps:

1. Set `gitlab.user`, open "Praca (GitLab)" on VPN, hide one issue, restart the app: it is still in "Ukryte (N)".
2. Comment on it yourself in GitLab, press "Odśwież": still hidden.
3. Ask a colleague or bot to comment (or move its `workflow::` label), press "Odśwież": it returns marked; press "OK".
4. Hide another issue, press "Pokaż": it returns unmarked.
5. Disconnect VPN, press "Odśwież": calm "unavailable" state, hidden issues still hidden.
6. Check the app log for the absence of titles, references, labels and the host.

## Performance Considerations

Each round makes at most one notes call per hidden issue whose `updated_at` moved or whose single settling check is due, and none when nothing changed. A round stops issuing notes calls at the first unavailable/logged-out/missing-tool failure, so a VPN drop costs one 20 s timeout, not one per issue. That keeps the background-lightness NFR. Notes are paginated, newest first; typical issues fit in one page, and multi-page output is read in full.

## Migration Notes

Opening the store applies v3 automatically. An older app build refuses the v3 database as `NewerSchema` (existing F-01 behaviour); restore from `backups/` if downgrading is ever needed. No existing data changes.

## References

- Roadmap slice: `context/foundation/roadmap.md` (S-03)
- PRD: `context/foundation/prd.md` (FR-003, US-02)
- S-01 plan and Addendum A: `context/archive/2026-11-23-gitlab-assigned-list/plan.md`
- F-01 store plan: `context/archive/2026-11-23-app-data-store/plan.md`
- Record-module pattern: `src-tauri/src/work_time/`
- Guard pattern: `src-tauri/src/cli/mod.rs:369-396`, `:575-596`
- UI action pattern: `src/own-tasks/useOwnTasks.ts`, `src/own-tasks/OwnTasksView.tsx`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Issue identity and the read-only notes call

#### Automated

- [x] 1.1 CLI guard and argument tests pass, including the new shape, per-character and proptest cases — 276380a
- [x] 1.2 Fetch and runner integration tests pass, including notes scenarios and argv echo — 276380a
- [x] 1.3 Notes parsing rejects bad JSON without leaking content — 276380a
- [x] 1.4 Lint and format are clean — 276380a

### Phase 2: Hidden-issue store and resurfacing rules

#### Automated

- [x] 2.1 Rule unit tests pass
- [x] 2.2 Store integration tests pass
- [x] 2.3 Crash test never loses or half-applies a hide record
- [x] 2.4 Errors never contain submitted values
- [x] 2.5 Lint and format are clean

### Phase 3: Wiring into the refresh round and commands

#### Automated

- [ ] 3.1 Round integration tests with fake-cli and a temp store pass
- [ ] 3.2 next_snapshot and partition unit tests pass
- [ ] 3.3 All Rust tests, lint and format pass
- [ ] 3.4 Frontend type check and build pass with the new mirrors

### Phase 4: UI in "Praca (GitLab)"

#### Automated

- [ ] 4.1 Frontend checks pass
- [ ] 4.2 The full gate passes before commit

#### Manual

- [ ] 4.3 Hide moves an issue into "Ukryte (N)" and survives a restart
- [ ] 4.4 Own comment keeps the issue hidden
- [ ] 4.5 Another's comment or a scoped-label move resurfaces it with the right reason; OK removes the mark
- [ ] 4.6 Pokaż brings a hidden issue back unmarked
- [ ] 4.7 With VPN off the view stays calm and hidden issues stay hidden
- [ ] 4.8 MR rows have no Ukryj and the log carries no work data
