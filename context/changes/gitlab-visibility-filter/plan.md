# GitLab Visibility Filter Implementation Plan

## Overview

A new top-level `gitlabFilter` block in the hand-written `settings.json` decides which assigned GitLab items the "Praca (GitLab)" list shows. It can filter by project, labels and milestone, and it can add an opt-in section of recently closed items (roadmap S-02, PRD FR-002). The filter narrows only what the list shows. "To do" reconciliation, hidden-issue tracking and new-assignment notifications keep working on the full open list (PRD FR-002 Socrates note, roadmap S-02 risk).

## Current State Analysis

- Settings are only edited by hand. Each top-level block has its own loader and is re-read every round, so a change needs no restart (`src-tauri/src/settings.rs:101`, `:389`). If `gitlabRefresh` is invalid, the app falls back to the default and shows a notice (`settings.rs:389-427`, `src/gitlab/GitlabView.tsx:247`). No settings screen exists.
- The two assigned calls fetch open items only (`state=opened`, `src-tauri/src/cli/mod.rs:208-215`). Three features depend on that, because each treats "missing from a successful list" as closed or unassigned:
  - "To do" reconciliation (`todo/mod.rs:493-508`, which reads `merge_requests`, `issues` and `hidden_issues`);
  - hidden-issue tracking (`hidden_issues/round.rs`);
  - the seen set for notifications (`new_assignments/mod.rs:62`).
- One round of `refresh_round` (`src-tauri/src/gitlab_refresh.rs:264-380`) runs in this order:
  1. fetch the list;
  2. run the hidden-issue checks;
  3. `commit_round` partitions the issues into visible and hidden;
  4. `next_snapshot`;
  5. `kept_full` stores the unpartitioned list;
  6. `reconcile_todo(next)` reads `next.items`, which today means all items, partitioned (`gitlab_refresh.rs:385-390`, `:442`);
  7. publish;
  8. notifications run on `round_full`, the unpartitioned list (`gitlab_refresh.rs:365`).
- `act_on_issue` (hide/unhide/dismiss) re-partitions `kept_full` and writes `snap.items` (`gitlab_refresh.rs:97-135`).
- The "To do" picker lists `snapshot.items.mergeRequests + issues` (`src/todo/TodoView.tsx:305`). `add_todo_gitlab_item` accepts only an item found in `snapshot.items` (`src-tauri/src/todo/commands.rs:124`).
- `RawItem` does not read `milestone` or `state` yet (`src-tauri/src/gitlab/mod.rs:185-198`). The project path appears only inside `references.full`: `group/sub/project#12` for an issue, `…!34` for an MR.
- The read-only guard test spells out every allowed endpoint shape. It bans `:`, `/`, `+`, `#` and `%` in the parameterised shape and counts `&` exactly (`cli/mod.rs:493-560`). `VARIANT_COUNT`/`EXAMPLE_COUNT` and the exhaustive `ordinal` break the build when a variant is added (`cli/mod.rs:430-491`).

## Desired End State

- With a `gitlabFilter` block, the view shows only matching MRs, issues and hidden issues. Each section shows a line "Ukryte filtrem: N" when the filter removed items. An all-filtered section says the filter is on; it never reads as "nothing assigned".
- With `closedDays: N`, a "Zamknięte (ostatnie N dni)" section lists assigned issues closed and MRs closed or merged in the last N days, with the same filter applied. If that fetch fails, the section shows its own error and the open list is unaffected.
- An invalid block shows the full list and a notice, never an error state and never a red tray.
- An item the filter removes still notifies when it is newly assigned. It still ticks itself off in "To do" only when it really closes, and hidden-issue tracking still runs on it.
- Verify with `cargo test` (filter rules, loader, round wiring, guard) and a manual run against the real GitLab.

### Key Discoveries:

- `reconcile_input` reads `next.items` (`gitlab_refresh.rs:385`). Once the snapshot carries a filtered list, a filtered-out item would look "missing" and its "To do" entry would tick itself off. Reconciliation must switch to the full round list.
- `act_on_issue` writes `snap.items` directly (`gitlab_refresh.rs:127`). Without re-filtering there, a hide click would put filtered-out items back on screen until the next round.
- The guard's parameterised shape bans `:` and `+`. A date-only `updated_after=YYYY-MM-DD` fits an explicit new guard shape without loosening the existing ones.
- GitLab MRs have the states `opened`, `closed`, `locked` and `merged`. One `state=all` call for MRs (keeping only `closed` and `merged`) costs one call instead of two. Issues only have `opened` and `closed`, so `state=closed` is enough.

## What We're NOT Doing

- No settings screen. The filter is edited in `settings.json`, like every other block.
- No filter on notifications, hidden-issue tracking or "To do" reconciliation (FR-002 Socrates note, FR-004).
- No filter on the GitHub view (S-05 is out of FR-002).
- No rows for filtered-out items: only counts (user decision). No peek section.
- No unbounded closed history: `closedDays` is capped at 30.
- Closed items get no hide/unhide/dismiss actions, cannot be picked into "To do" and never trigger a notification or a reconciliation.
- Picking into "To do" stays limited to what the list shows (`snapshot.items`). A filtered-out item cannot be picked; an existing "To do" entry for it stays and is reconciled as before.
- No tray or health change: an invalid filter and a failed closed fetch are view notices, not source errors.
- No milestone or state badge on rows.
- No regex or wildcard matching.

## Implementation Approach

The filter is a pure Rust rule in a new `gitlab/filter.rs`, loaded from its own top-level block, so a typo there never touches the `gitlab` block. It is applied once, at the end of the round, to the already partitioned list. The unfiltered list stays the input for every other consumer (`kept_full`, `round_full`). The round keeps the filter next to `kept_full` so `act_on_issue` can re-apply it. The closed section is a separate, optional fetch after a successful open fetch. Its result goes only into a new snapshot field, never into `AssignedItems`, so `keys_of`, `reconcile_gitlab` and `partition` cannot see it.

Rules settled during planning:
- **Shape and combination.** Each dimension (`projects`, `labels`, `milestones`) has optional `include` and `exclude` lists. An item matches `include` when it matches ANY entry, and is removed when it matches ANY `exclude` entry. Dimensions combine with AND. An empty or absent list means no constraint.
- **Projects.** An entry matches the item's project path exactly or as a group prefix on a `/` boundary: `acme/backend` matches `acme/backend#7` and `acme/backend/api#4`, but not `acme/backend-old#1`. Matching is case-insensitive.
- **Labels and milestones.** Exact, case-insensitive match on the label text and the milestone title. An item without a milestone fails a non-empty milestone `include` and is never removed by `exclude`. The same goes for an item whose project path cannot be read.
- **Scope.** The filter applies to MRs and issues alike, and to the hidden and closed sections.
- **Closed items.** "Closed" means the issue state `closed`, and the MR states `closed` or `merged`. Off by default.
- **Invalid block.** Show everything, plus a notice. Closed stays off.

## Critical Implementation Details

**State sequencing.** In `refresh_round` the filter applies after `commit_round` and before `next_snapshot`. `full_list`/`round_full` (unfiltered) feeds `kept_full`, the "To do" reconciliation and the notifications. Only the filtered list reaches `snapshot.items`. Keep the filter in a new `kept_filter` slot, written under `hidden_gate` together with `kept_full`, so `act_on_issue` re-filters with the same rule the round used. The lock order becomes gate → `kept_config` → `kept_full` → `kept_filter` → snapshot.

**Privacy.** Filter values (project paths, labels, milestone titles) are work data. They never reach a log line, an error value or a message. Loader errors carry no detail (serde errors can quote values), matching the existing loaders. Tests use invented values only.

## Phase 1: Filter rule and settings block

### Overview

The pure building blocks: the filter type and its matching rule, milestone parsing, and the fail-open loader. No wiring yet.

### Changes Required:

#### 1. Milestone on items

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Read the milestone title from both API lists so the filter can match it. It is not shown in the UI.

**Contract**: `RawItem` gains `#[serde(default)] milestone: Option<RawMilestone { title: String }>`. `GitlabItem` gains `#[serde(skip)] pub milestone: Option<String>`. The webview JSON is unchanged, and `snapshot_serializes_for_the_webview` stays green. Every struct literal of `GitlabItem` in tests across modules gets `milestone: None`. A milestone with the wrong type is `ResponseInvalid`, like the other fields.

#### 2. Filter rule

**File**: `src-tauri/src/gitlab/filter.rs` (new), `pub mod filter;` in `gitlab/mod.rs`

**Intent**: One place that decides whether an item is shown, and that splits a list into shown items plus a count of removed ones.

**Contract**:
- `pub struct DimensionRule { include: Vec<String>, exclude: Vec<String> }`, holding lowercased entries.
- `pub struct GitlabFilter { projects, labels, milestones: DimensionRule, closed_days: Option<u32> }` with `Default` (no constraint, closed off) and `is_active()` (true when any list is non-empty).
- `pub fn project_path(item: &GitlabItem) -> Option<&str>` returns the part of `reference` before the last `#` (issue) or `!` (MR), or `None` when the separator is missing or the path is empty.
- `pub fn keeps(&self, item: &GitlabItem) -> bool` implements the rules in Implementation Approach.
- `pub fn apply(&self, items: &[GitlabItem]) -> (Vec<GitlabItem>, u32)` keeps the input order and returns the kept items and the removed count.
- `Debug` for both types is opaque (`GitlabFilter(..)`), so values never reach a log.

#### 3. Settings loader

**File**: `src-tauri/src/settings.rs`

**Intent**: Read the top-level `gitlabFilter` block on its own and fail open.

**Contract**:
- `pub fn load_gitlab_filter(path: &Path) -> Result<GitlabFilter, FilterSettingInvalid>`, where `FilterSettingInvalid` is a unit struct like `RefreshSettingInvalid`.
- A file-level problem (missing, unreadable, bad UTF-8 or JSON, wrong `version`) is `Ok(GitlabFilter::default())`, because the `gitlab` block already reports the file. So is an absent or `null` block.
- `Err` covers a block that is not an object, any unknown key at any level (`deny_unknown_fields`), and a dimension that is not an object.
- `Err` also covers a list that is not an array of strings, an entry that is empty after trimming, longer than 255 characters or contains a control character, and more than 100 entries in one list.
- `Err` covers a project entry with an empty segment, which includes a leading or trailing `/`, and a `closedDays` that is not an integer 1..=30, `null` or absent.
- Entries are trimmed and lowercased on load.
- Shape, documented in the doc comment:

```json
"gitlabFilter": {
  "projects":   { "include": ["acme/backend"], "exclude": ["acme/backend/legacy"] },
  "labels":     { "exclude": ["blocked"] },
  "milestones": { "include": ["Sprint 42"] },
  "closedDays": 7
}
```

### Success Criteria:

#### Automated Verification:

- Filter unit tests pass (`cargo test gitlab::filter`):
  - include ANY, exclude ANY, AND across dimensions;
  - group prefix on a `/` boundary (`acme/backend` vs `acme/backend/api`, `acme/backend-old`);
  - case-insensitive matching;
  - no milestone vs include and exclude;
  - unreadable project path;
  - MR vs issue separator;
  - `apply` keeps order and counts what it removed.
- Loader tests pass (`cargo test settings`):
  - absent, null and file-level problems give the default;
  - every invalid shape is `Err` and no error text contains the value;
  - `closedDays` bounds 0/1/30/31;
  - entry limits;
  - a broken `gitlabFilter` leaves the `gitlab`, `todo` and `gitlabRefresh` loaders green.
- Milestone parsing test: present, `null`, absent, wrong type.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human before proceeding to the next phase.

---

## Phase 2: Apply the filter in the round

### Overview

The round and the hide commands publish a filtered list with counts and flags. Every other consumer keeps the full list.

### Changes Required:

#### 1. Snapshot fields

**File**: `src-tauri/src/gitlab/mod.rs`

**Intent**: Tell the view whether a filter is on, whether it was invalid, and how many items each section lost.

**Contract**:
- `GitlabSnapshot` gains `filter_active: bool`, `filter_invalid: bool` and `filtered_out: FilteredOut { merge_requests: u32, issues: u32, hidden_issues: u32 }` (camelCase serde, `Default`).
- `next_snapshot` keeps these with the items on rounds that keep the items, and resets them when the items are dropped, just like the hidden-issue fields.
- `filter_invalid` is a round flag like `refresh_setting_invalid`: the round sets it whatever the outcome.
- Update the webview serialization test.

#### 2. Round wiring

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Load the filter each round, apply it to the partitioned list, and keep the unfiltered list as the input for every other consumer.

**Contract**:
- At round start, call `load_gitlab_filter`. On `Err`, use `GitlabFilter::default()` with `filter_invalid = true`.
- On success, apply the filter to `merge_requests`, `issues` and `hidden_issues` after `commit_round`, then fill the counts.
- `GitlabRefreshState` gains `kept_filter: Mutex<GitlabFilter>`, written under the gate whenever `kept_full` is replaced.
- `reconcile_todo` takes the round's full list (`round_full`) and the snapshot's account. Change `reconcile_input` so that it returns `(full, account)` only for an `Ok` snapshot with an account and a full list from this round.
- Notifications are unchanged: they still run on `round_full`.
- On a failed round that keeps the items (`next_kept` is `Some`), re-derive `snapshot.items` from `kept_full` with this round's filter. Use the same partition-and-filter helper as `act_on_issue`, with the hidden records as stored. Then an offline filter edit takes effect, and a `filter_invalid` notice never sits over a list that is still filtered by the old rule. Store `kept_filter` on such rounds too.

#### 3. Hide/unhide/dismiss re-filter

**File**: `src-tauri/src/gitlab_refresh.rs` (`act_on_issue`)

**Intent**: Keep an action from bringing filtered-out items back on screen.

**Contract**: After `partition`, apply `kept_filter` to `merge_requests`, the visible issues and the hidden issues. Store the counts and `filter_active` with them. `filter_invalid` is left as the last round set it. `hidden_check_failed` is still capped by the hidden count before filtering, since the checks ran on all of them.

### Success Criteria:

#### Automated Verification:

- `reconcile_input` tests: only `Ok` with an account and a full list. The full list, not the filtered one, is what it returns.
- A test with a filtered-out issue shows that `reconcile_gitlab` (fed the round input) does not tick its "To do" entry.
- `act_on_issue` tests:
  - an active filter is re-applied to visible and hidden issues and to MRs, and the counts are stored;
  - a default filter leaves today's behaviour unchanged (existing tests stay green).
- `next_snapshot` tests: filter fields kept with kept items, reset when items drop; `filter_invalid` carried like `refresh_setting_invalid`. A failed round that keeps the items re-filters `kept_full` with that round's filter (default after an invalid block).
- `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human before proceeding to the next phase.

---

## Phase 3: Recently closed fetch

### Overview

When `closedDays` is set, two extra read-only calls list the assigned items closed in the window. The result is a separate, filtered snapshot section that cannot fail the open list.

### Changes Required:

#### 1. CLI calls and guard

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Two new fixed-shape, read-only calls that carry a date the guard can check.

**Contract**:
- `pub struct UpdatedAfter` is built only from a `chrono::NaiveDate` and renders `YYYY-MM-DD`.
- New variants: `GlabRecentlyClosedIssues(GitlabHost, Option<GitlabUser>, UpdatedAfter)` and `GlabRecentlyClosedMergeRequests(GitlabHost, Option<GitlabUser>, UpdatedAfter)`.
- Issue endpoints: `issues?scope=assigned_to_me&state=closed&updated_after=<D>&per_page=100` without a user, and `issues?scope=all&state=closed&assignee_username=<login>&updated_after=<D>&per_page=100` with one.
- MR endpoints: the same shapes with `merge_requests` and `state=all`.
- Arguments: `api <endpoint> --paginate --hostname <host>`.
- Names: `glab:closed-issues` and `glab:closed-mrs`.
- Extend `examples()` (with and without a user), `ordinal`, `VARIANT_COUNT` and `EXAMPLE_COUNT`.
- The guard gets its own written-out shape for these endpoints, with the date checked by a guard-local `YYYY-MM-DD` digit check. The existing shapes do not get looser.

#### 2. Parse and fetch

**File**: `src-tauri/src/gitlab/mod.rs`, `src-tauri/src/gitlab/fetch.rs`

**Intent**: Turn the two responses into one closed list, newest first.

**Contract**:
- `RawItem` gains `#[serde(default)] state: Option<String>`, `#[serde(default)] closed_at: Option<String>` and `#[serde(default)] merged_at: Option<String>`, read only by the closed parser. They are optional, so the open fixtures without them (`fake_cli.rs`, `raw_json` in `gitlab/mod.rs`) keep parsing.
- `updated_after` is only a server-side pre-filter: an item closed long ago but commented on yesterday also matches it. The parser decides with the closing time.
- `pub fn parse_recently_closed(json, host, kind, cutoff: DateTime<FixedOffset>)` keeps an issue only in state `closed` with `closed_at >= cutoff`. It keeps an MR only in state `merged` with `merged_at >= cutoff`, or in state `closed` with `closed_at >= cutoff`. A missing `state` or closing time, or one that does not parse, drops the item; it is not a `ResponseInvalid`. A closing time that is present but has the wrong type is `ResponseInvalid`, like the other fields.
- `pub fn fetch_recently_closed(runner, config, after: UpdatedAfter, cutoff) -> Result<Vec<GitlabItem>, GitlabError>` runs MRs, then issues. The first failure ends it. It merges both kinds with the newest closing time first, ties broken by kind, then reference. The cutoff is local midnight `closedDays` days ago. `UpdatedAfter` is the day before the cutoff date, so the UTC reading of a date-only value never cuts an item off.

#### 3. Snapshot section and round

**File**: `src-tauri/src/gitlab/mod.rs`, `src-tauri/src/gitlab_refresh.rs`

**Intent**: Show the closed items, or an error for them, without touching the open list or its consumers.

**Contract**:
- `GitlabSnapshot` gains `closed: Option<ClosedSection { days: u32, items: Vec<GitlabItem>, filtered_out: u32, error: Option<String> }>`. It is `None` when `closedDays` is off or the filter is invalid.
- The round runs the closed fetch only after a successful open fetch, outside every lock, with the cutoff and the `UpdatedAfter` date from §2. It applies the filter to the result.
- On a closed failure, `items` is empty and `error` holds `GitlabError::message_pl()`.
- `next_snapshot` keeps `closed` with kept items and drops it with dropped items.
- `act_on_issue` leaves `closed` unchanged.
- Closed items never enter `AssignedItems`, `kept_full`, reconciliation or notifications.

#### 4. Fake CLI

**File**: `src-tauri/src/bin/fake_cli.rs`, `src-tauri/tests/gitlab_fetch.rs`

**Intent**: Test the closed calls end to end with invented data.

**Contract**:
- `endpoint()` (`fake_cli.rs:59-65`) routes on the `merge_requests?` / `issues?` prefix. Add `Endpoint` variants for the closed calls and match `state=closed` / `state=all` before the prefix checks, or they get the open fixtures.
- Scenarios serve the closed endpoints with invented items:
  - a merged MR;
  - a closed MR;
  - an `opened` MR that must be dropped;
  - a closed issue;
  - an issue closed before the cutoff but updated after it, which must be dropped.
- A failing scenario covers the closed calls.
- Update the exact-args assertions in `tests/cli_runner.rs` if they enumerate variants.

### Success Criteria:

#### Automated Verification:

- Guard tests pass: both new variants with and without a user are read-only. The guard refuses a date with `:`, a non-digit, an extra `&` or a missing parameter. `examples` covers every variant.
- Closed parse tests:
  - `opened` and `locked` MRs dropped; merged and closed kept;
  - an issue that is not `closed` dropped;
  - closed before the cutoff but updated after it, dropped;
  - a missing `state` or closing time dropped, not an error;
  - open fixtures without `state` still parse;
  - sort by closing time;
  - no response content leaks into errors.
- Fake-CLI integration tests: `fetch_recently_closed` returns the expected invented items, and its failure classifies like the open calls.
- Snapshot tests: `closed` kept or dropped with the items; serialization includes `closed`.
- `cargo test`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.

#### Manual Verification:

- Against the real GitLab with `closedDays` set, the closed calls succeed, which confirms that GitLab accepts a date-only `updated_after`, and recently closed and merged items appear.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human before proceeding to the next phase.

---

## Phase 4: UI

### Overview

The view shows the filtered counts, the invalid notice and the closed section. The TS mirrors match the new Rust types.

### Changes Required:

#### 1. TS mirrors

**File**: `src/gitlab/types.ts`

**Intent**: Mirror the new snapshot fields in the same change as the Rust types (AGENTS.md contract rule).

**Contract**: `GitlabSnapshot` gains `filterActive: boolean`, `filterInvalid: boolean`, `filteredOut: { mergeRequests: number; issues: number; hiddenIssues: number }` and `closed: ClosedSection | null`, where `ClosedSection = { days: number; items: GitlabItem[]; filteredOut: number; error: string | null }`.

#### 2. View

**File**: `src/gitlab/GitlabView.tsx`

**Intent**: Never let an all-filtered section look like "nothing assigned"; flag an invalid filter; show recent closures.

**Contract**:
- Sections show "Ukryte filtrem: N" when N > 0. The hidden section is rendered when it has rows or a filtered count.
- When the filter is active and a section is empty, the text reads "Brak … pasujących do filtra".
- When `filterInvalid` is set, a `gitlab-notice` reads "Nieprawidłowy blok gitlabFilter w settings.json — pokazuję wszystkie przypisane".
- When `closed` is set, a "Zamknięte (ostatnie N dni)" section lists rows without actions. It shows its error as a `gitlab-notice` instead of rows when `error` is set, its own "Ukryte filtrem" count, and "Brak zamkniętych w ostatnich N dniach." when empty.
- The stale styling applies to the closed section as well.

#### 3. "To do" picker

**File**: `src/todo/TodoView.tsx`

**Intent**: The picker reads the filtered `snapshot.items`, so an all-filtered list must not read as "nothing assigned" there either.

**Contract**: When `filterActive` is set and the GitLab list is empty, the picker's empty text (today "Brak przypisanych w GitLabie.", `TodoView.tsx:459`) reads "Brak pasujących do filtra (ukryte filtrem: N)". N is the sum of `filteredOut.mergeRequests` and `filteredOut.issues`.

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` and `npm run build` pass.
- `cargo test` passes (the serialization tests pin the field names the TS mirror uses).

#### Manual Verification:

- With no `gitlabFilter`, the view looks exactly as before.
- With a project include filter, only matching items show, each section shows the right "Ukryte filtrem" count, and the "To do" picker offers only shown items.
- With a filter that removes everything, the sections and the "To do" picker say "pasujących do filtra" with counts, not "Brak przypisanych".
- A typo in `gitlabFilter` shows the full list and the notice; the tray stays green.
- Hiding an issue with a filter active keeps the filtered-out items off screen.
- With `closedDays: 7`, the "Zamknięte" section shows recent closures. Without VPN it keeps the last list as stale, like the open sections.
- An item already on "To do" whose project is then filtered out stays on "To do" and is not ticked off.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Filter rule: every combination of include and exclude per dimension, the AND across dimensions, prefix boundaries, case, a missing milestone or project path.
- Loader: fail-open cases, every invalid shape with value-free errors, limits, independence from other blocks.
- Round helpers: `reconcile_input` returns the full list; `act_on_issue` re-filters; `next_snapshot` keep/drop for the filter and closed fields.
- Closed parser: state selection and order.

### Integration Tests:

- `tests/gitlab_fetch.rs` with the fake CLI: closed calls success and failure classification.
- `tests/todo.rs`: an entry whose item is filtered out from the view but present in the full list stays unticked after reconcile.

### Manual Testing Steps:

1. Start with no `gitlabFilter` and confirm the view is unchanged.
2. Add `projects.include` for one group and press "Odśwież". Only that group shows, and the counts are right.
3. Add `labels.exclude` for a label on a shown issue. It disappears and the issue count goes up by one.
4. Introduce a typo (`"projcts"`). The full list and the notice show, and the tray stays green.
5. Set `closedDays: 7`. The closed section appears with merged and closed items.
6. Disconnect VPN. The open and closed sections stay with the stale look, and the "unavailable" notice shows.

## Performance Considerations

Filtering is linear over at most a few hundred items per round. The closed calls run only when `closedDays` is set, are bounded by a ≤ 30-day window, and add at most two `glab` calls per round, which keeps the "Lekkość w tle" NFR.

## Migration Notes

None. The block is optional and absent means today's behaviour. No store schema change.

## References

- Roadmap: `context/foundation/roadmap.md` (S-02)
- PRD: `context/foundation/prd.md` (FR-002, FR-004, NFR "Nigdy pusta lista zamiast błędu")
- Prior slices: `context/archive/2026-11-23-gitlab-assigned-list/plan.md`, `context/archive/2026-11-23-hide-and-resurface/plan.md` (filter must apply after the partition), `context/archive/2026-11-23-todo-list/plan.md` (reconciliation must stay on the unfiltered set)
- Patterns: `src-tauri/src/settings.rs:389` (fail-open block loader), `src-tauri/src/gitlab_refresh.rs:264` (round), `src-tauri/src/cli/mod.rs:217` (assigned call args), `src-tauri/src/cli/mod.rs:530` (endpoint guard)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Filter rule and settings block

#### Automated

- [x] 1.1 Filter unit tests pass (include/exclude, AND, group prefix, case, missing milestone/path, MR vs issue, apply order and count) — fd449e1
- [x] 1.2 Loader tests pass (fail-open, invalid shapes value-free, closedDays bounds, limits, independence from other blocks) — fd449e1
- [x] 1.3 Milestone parsing test: present, null, absent, wrong type — fd449e1
- [x] 1.4 cargo fmt --check and cargo clippy --all-targets -- -D warnings pass — fd449e1

### Phase 2: Apply the filter in the round

#### Automated

- [ ] 2.1 reconcile_input tests: only Ok with account and full list; returns the full list
- [ ] 2.2 Filtered-out issue does not tick its "To do" entry
- [ ] 2.3 act_on_issue re-applies the filter and stores counts; default filter keeps today's behaviour
- [ ] 2.4 next_snapshot keeps/resets filter fields with the items; filter_invalid carried as a round flag; a kept round re-filters kept_full
- [ ] 2.5 cargo test, cargo fmt --check and cargo clippy pass

### Phase 3: Recently closed fetch

#### Automated

- [ ] 3.1 Guard tests: new variants read-only with and without a user; bad dates and shapes refused; examples cover every variant
- [ ] 3.2 Closed parse tests: state selection, closing time vs cutoff, optional state, order, no content leaks
- [ ] 3.3 Fake-CLI integration tests for closed fetch success and failure classification
- [ ] 3.4 Snapshot tests: closed kept/dropped with items; serialization includes closed
- [ ] 3.5 cargo test, cargo fmt --check and cargo clippy pass

#### Manual

- [ ] 3.6 Real GitLab accepts date-only updated_after; recent closed and merged items appear

### Phase 4: UI

#### Automated

- [ ] 4.1 npm run lint, npm run format:check and npm run build pass
- [ ] 4.2 cargo test passes

#### Manual

- [ ] 4.3 No gitlabFilter: view unchanged
- [ ] 4.4 Project include filter: only matching items, correct counts, picker offers only shown items
- [ ] 4.5 All-filtered sections and the "To do" picker say "pasujących do filtra" with counts
- [ ] 4.6 Typo in gitlabFilter: full list and notice, tray green
- [ ] 4.7 Hiding with an active filter keeps filtered-out items off screen
- [ ] 4.8 closedDays 7: Zamknięte section shows recent closures; stale without VPN
- [ ] 4.9 "To do" entry of a filtered-out item stays and is not ticked off
