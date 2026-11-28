# Hide and resurface GitHub issues (S-01 `github-hide-resurface`) Implementation Plan

## Overview

In "Projekty (GitHub)" the user can hide a GitHub issue from its row and show it again. A hidden issue comes back on its own, marked "Zmienione", when its state changes (it leaves the open list and comes back) or when someone other than the user comments. The look and the behaviour mirror "Praca (GitLab)". Hide records live in the app data store (migration v15, a source-neutral table that can also hold GitLab merge requests later). The data comes through two new read-only `gh` calls: `gh api user` (the user's login) and `gh issue view <N> -R <repo> --json comments`.

Roadmap M-4, slice S-01, anchor MS-31 (PRD refs FR-003, FR-005; GitHub issue #31). No new FR in the PRD. Planned non-interactively: the owner could not be asked, so every choice the skill would have asked about is marked **default chosen by agent** with a one-line reason. The user's three decisions in `change.md` are binding and are not re-opened.

## Current State Analysis

The research (`context/changes/github-hide-resurface/research.md`, commit f9545be) maps the code in detail. What matters here:

- **GitLab hiding (S-03, migration v6)** is complete: table `hidden_issues` keyed `(host, issue_id)`, store ops in `hidden_issues/mod.rs`, pure rules in `hidden_issues/rules.rs`, a two-half round in `hidden_issues/round.rs` (`check_round` with no lock, `commit_round` in one write), commands in `hidden_issues/commands.rs`, wiring in `gitlab_refresh.rs` (`hidden_gate`, `kept_full`, `act_on_issue`).
- **The rules are GitLab-bound in four places only** (`rules.rs:9-11`): `GitlabItem` (id, `updated_at`, label names), `IssueNote` (author, system, `created_at`), `GitlabUser`, and `ChangeReason` (`gitlab/mod.rs:49-53`). The algorithm needs an item key, an `updated_at` text and a status fingerprint per listed item, and author / counts / `created_at` per comment.
- **GitHub has none of the hiding machinery**: `GithubRefreshState` holds only `cell` and `kept_config` (`github_refresh.rs:54-58`); `GithubIssue`, `RepoView` and `GithubSnapshot` (`github/mod.rs:19-35, 222-256`) have no hidden fields. One `gh issue list --state open` per repo (`cli/mod.rs:186-204`); `Unavailable`/`LoggedOut`/`ToolMissing` end the round, `Failed`/`ResponseInvalid`/`Truncated` stay per repo (`github/fetch.rs:15-46`).
- **The "To do" reconcile reads the snapshot's repos** (`github_refresh.rs:198-226`) and `todo::reconcile_github` collects one map per repo key (`todo/mod.rs:657-673`). Once the snapshot splits visible and hidden issues, the reconcile input must hold both, or a hidden issue in "To do" would be ticked as closed in the source.
- **The CLI guard is an allowlist of exact shapes** (`cli/mod.rs:727-758`) with counts and an exhaustive `ordinal` (`cli/mod.rs:529-568`). `CliCall::ALL` is the parameterless list the ignored real-CLI smoke test runs (`tests/cli_real.rs`).
- **GitLab really shows** a "Pokaż ukryte (n)" toggle and, when it is on, the hidden issues as dimmed rows after the visible ones, each with "ukryte DD.MM" (`GitlabView.tsx:178, 199, 458-467, 516-523`). There is no separate "Ukryte (n)" section.
- **`ItemRow` takes one flag** (`components/ItemRow/ItemRow.tsx:12-13, 76`); a GitHub row now needs "Zmienione", "W Do zrobienia" and "ukryte DD.MM" together.
- No `context/foundation/lessons.md` exists.

## Desired End State

- Each visible GitHub issue row has "Ukryj do zmiany"; each hidden one has "Pokaż na liście"; a marked one has "OK, zauważone". The view head has "Pokaż ukryte (n)"; when it is on, each repo panel shows its hidden issues dimmed after its visible ones, each with "ukryte DD.MM". A returned issue carries the "Zmienione" flag with the reason as a tooltip ("nowy komentarz" or "wróciło na listę").
- A hidden issue resurfaces when someone other than the user comments after the baseline (ghost and missing authors count as someone else), or when it was absent from the open list and is listed again. Labels, edits, reactions and the user's own comments never bring it back.
- Hide records live in the new table `hidden_items` (migration v15), keyed `(source, scope, kind, item_id)`. GitLab issues stay in `hidden_issues`; GitLab code and behaviour are unchanged.
- A failed repo's records are left out of the round; a failed `gh api user` counts the due checks as failed; a store failure shows every issue with a notice. Never an empty list in place of a failed source.
- `AGENTS.md` lists what a GitHub hide record holds.

Verify with the gates and the tests named in each phase, plus the Manual rows (checked later by the user in the running app).

### Key Discoveries:

- `plan_round` marks every record without a listed item as absent (`rules.rs:149-157`), so a repo that failed this round must have its records removed from the round's input, not passed with an empty list.
- `commit_round` counts failed checks only for items still hidden after the write (`round.rs:187-192`); the GitHub round keeps that rule.
- `gh issue view --json comments` returned every comment of a 231-comment issue; `gh issue list --json comments` stops at 100 (research section 3). Only `createdAt` is compared, so edits never resurface an issue.
- `todo::reconcile_github` builds its per-repo map with `collect()` (`todo/mod.rs:665-673`): two entries with the same repo key would overwrite each other, so visible and hidden issues must be joined into one entry per repo.
- `lib.rs` declares every module `pub mod`. New items are `pub` in their (public) module or have a non-test caller in the same phase, so `cargo clippy --all-targets -- -D warnings` never fails on dead code between phases.
- Existing migration tests find their migration by content and compare with `MIGRATIONS.len()` (`tests/gitlab_view_state.rs:95-118`), so appending v15 breaks none of them.

## Team Questions (answered)

### 1. Can `hidden_issues/rules.rs` be shared between GitLab and GitHub without a copy?

**Yes.** One copy of the rules, made generic with two small traits and one neutral comment struct, all in `rules.rs`:

- `trait HideItem` (a listed item): `hide_id() -> u64`, `updated_at() -> &str`, `status_fingerprint() -> Vec<String>`. `GitlabItem` returns its scoped labels (today's `scoped_labels(item.label_names())`); `GithubIssue` returns its number and an empty fingerprint (labels never resurface a GitHub issue: user's decision).
- `trait HideRecord` (a stored record): `item_id`, `fingerprint`, `baseline_updated_at`, `settle_after_ms`, `hidden_at_ms`, `absent_since_ms`, `changed_reason`, with provided `is_hidden()` and `baseline()`. Implemented by today's `HiddenRecord` (unchanged fields: `issue_id`, `scoped_labels`, ...) and by the new `HiddenItemRecord`.
- `struct Comment { author: Option<String>, counts: bool, created_at }`. GitLab maps a note to `counts = !system`, author `Some(username)`; GitHub maps every comment to `counts = true`, author `None` for a ghost or missing login.
- `comment_verdict(&[Comment], baseline, user: Option<&str>)`: a comment counts when `counts`, `created_at > baseline`, and either no user is known, or its author is `None`, or the author differs from the user (ASCII case-insensitive).
- `plan_round`, `needs_comment_check` and `status_changed` become generic over `R: HideRecord, I: HideItem`. `Transition`, `CommentVerdict`, `SETTLE_MS` and `PRUNE_AFTER_MS` are shared as they are; `Transition::*::issue_id` keeps its name and means "the item key within its scope" (GitLab global id, GitHub issue number).
- `ChangeReason` stays in `gitlab/mod.rs` and is re-exported from `hidden_issues` (`pub use crate::gitlab::ChangeReason`); GitHub code imports it from `hidden_issues`. **Default chosen by agent:** moving it would edit `gitlab/mod.rs`, which S-06 may touch.
- The GitLab public API (`list`, `hide`, `unhide`, `dismiss`, `apply_round`, `partition`, `check_round`, `commit_round`) keeps its signatures, and `HiddenRecord` keeps its fields and inherent methods, so `gitlab_refresh.rs`, `gitlab/` and the GitLab integration tests do not change. Only `round.rs` (maps `IssueNote` to `Comment`, passes `config.user.as_ref().map(GitlabUser::as_str)`) and the rule unit tests in `hidden_issues/tests.rs` (new `comment_verdict` argument types) change.

### 2. Storage shape: can it hold GitLab merge requests later (S-07) without a migration?

**Yes, with option A: a new generic table `hidden_items`; GitLab issues stay in `hidden_issues`.**

- Key `(source, scope, kind, item_id)`: `source` in `('gitlab','github')`, `scope` = the lowercase GitHub `owner/name` (or, for S-07, the GitLab host), `kind` in `('issue','mergeRequest')`, `item_id` = GitHub issue number (or, for S-07, the GitLab global MR id). Nullable `project_id`/`iid` (required for `gitlab` rows, forbidden for `github` rows) give S-07 its MR notes call. A JSON `status_fingerprint` holds S-07's scoped labels; it is `[]` for GitHub.
- S-07 then only adds rows with `source='gitlab', kind='mergeRequest'`: no migration of its own.
- **Why not option B (move GitLab rows in v15):** it switches the GitLab SQL in `hidden_issues/mod.rs`, adds an upgrade-with-data test and a crash path for the move, and puts a GitLab regression risk into this slice while S-06 changes the GitLab round in parallel. Option A makes v15 a pure `CREATE TABLE` that cannot fail on existing data and keeps the GitLab code paths byte-for-byte as they are. The cost is two tables with one meaning; a later change may fold `hidden_issues` into `hidden_items` if it ever pays off (S-07 does not need it).
- Option C (a GitHub-only table) is excluded: S-07 would need its own migration.

## What We're NOT Doing

- No GitLab merge-request hiding (S-07), no move of GitLab rows, no change to `gitlab_refresh.rs`, `gitlab/`, `src/gitlab/` or the GitLab hidden-issue behaviour.
- No GitHub refresh interval setting or `nextRefreshAtMs` (S-02), no GitHub NEW flags or seen keys (S-03).
- No `stateReason` or `closedAt` in any call and no change to the `gh issue list` arguments. **Default chosen by agent:** GitLab detects open/closed changes only by an issue leaving and re-joining the open list (`returned`), and the user asked for both sources to behave alike; a close and reopen inside one refresh interval without a comment by someone else is not detected on either source.
- No label, edit, reaction or GitHub Projects "Status" change brings an issue back (user's decision).
- No stored login: `gh api user` is called per round when a check is due, never cached across rounds, never logged, never stored. **Default chosen by agent:** a cache would keep a stale login after `gh auth switch` and turn the user's own comments into false resurfacings; the call costs one process per round only when a check is due.
- No remembered "Pokaż ukryte" toggle for GitHub (in memory only). **Default chosen by agent:** remembering it needs a store record, and S-03 plans a GitHub view-state migration in a later wave.
- No refactor of the GitLab view's row buttons into a shared component. **Default chosen by agent:** it would edit `src/gitlab/GitlabView.tsx`, which S-06 changes; the GitHub view writes its own three buttons with the same icons, titles and labels.
- No CSP change: no new kind of webview resource or connection.
- No new FR in the PRD; `context/foundation/roadmap.md` is not edited by this plan.

## Implementation Approach

Bottom-up in five phases, each committed with all gates green by one implementer:

1. The two `gh` calls, their guard shapes, fake routes and parsers (no caller in the app yet; all items `pub`).
2. Migration v15, the `hidden_items` store ops, the generic rules (GitLab adapted, unchanged behaviour) and the `AGENTS.md` app-data line.
3. The GitHub hidden round as tauri-free functions (check, commit, partition) plus the new snapshot fields and their TS mirror.
4. The wiring: gate and kept full list in `GithubRefreshState`, the round in `refresh_round`, the three commands and their registration.
5. The UI: commands in `api.ts`, actions in `useGithub.ts`, rows and toggle in `GithubView.tsx`, an additive `ItemRow` extension.

Decisions taken as defaults (each also noted where it applies):

- **Command key: `reference`** (`owner/name#N`, lowercase), resolved against the kept full list. **Default chosen by agent:** it is already the stable key in `GithubIssue` and the "To do" key, and one string argument keeps the commands like `add_todo_github_item`.
- **Settle check kept for GitHub (`SETTLE_MS`, one extra check two minutes after a hide).** **Default chosen by agent:** shared rules stay one copy, and whether GitHub delays `updatedAt` is unverified; the cost is one call per hide.
- **Failed `gh api user`: every due check of the round counts as failed** (no resurfacing by comment that round, the "Nie udało się sprawdzić N ukrytych" notice shows). **Default chosen by agent:** treating every author as someone else would resurface issues on the user's own comments; failing safe matches the GitLab stop rule.
- **Ghost or missing author counts as someone else** (`author: None`). **Default chosen by agent:** FR-003 counts every other author, a deleted account is not the user, and a parse failure would block every check of that issue.
- **Minimized comments count** like any other. **Default chosen by agent:** the rule errs towards showing; GitHub's "minimized" is a display choice of the repo, not authorship.
- **`gh issue view` asks only `--json comments`.** **Default chosen by agent:** the rules read nothing else (state comes from the list), so less data crosses the process boundary.
- **`GhUser` joins `CliCall::ALL`** and so the ignored real-CLI smoke test. **Default chosen by agent:** it is parameterless and read-only, and the smoke test is the one place that proves the login call works with the real `gh`.
- **The scope is the lowercase repo key in clear**, not a hash. **Default chosen by agent:** it matches the "To do" GitHub key already stored in clear and the GitLab `host` column, and S-07 keys by host the same way.
- **Toggle scope: one "Pokaż ukryte (n)" for the whole view**, n = hidden issues across all loaded repos; hidden rows appear inside their repo panel after the visible rows. **Default chosen by agent:** GitLab has one toggle per view and puts hidden rows after visible ones.
- **`ItemRow` gets two additive props** (`extra` rendered after `flag`, `dimmed` for the hidden look). **Default chosen by agent:** it keeps every current `ItemRow` caller unchanged and gives S-03's "Nowe" flag a place later.

### How "Ukryte (n)" maps onto what GitLab does

The slice wording says "a 'Ukryte (n)' section with the hide date". GitLab has no separate section: it has the "Pokaż ukryte (n)" toggle, and the hidden issues are dimmed rows (after the visible rows) each showing "ukryte DD.MM". The user's priority is that both sources look and behave alike, so GitHub gets exactly that: "(n)" is the count in the toggle, and the "section" is the set of dimmed hidden rows the toggle reveals in each repo panel. **Default chosen by agent** (the user asked to mirror what GitLab really does).

## Critical Implementation Details

- **Failed repos and failed rounds.** Only repos with an `Ok` outcome this round take part in `plan_round`; records of a failed repo get no transition and no comment call. A whole-source failure (list `Unavailable`, `LoggedOut`, `ToolMissing`) runs no hidden step at all: the snapshot keeps the last partitioned repos and their hidden fields while `next_snapshot` keeps the list, exactly like GitLab.
- **Lock order in `github_refresh.rs`:** `hidden_gate`, then `kept_config`, then `kept_full`, then the snapshot lock (in `SourceCell`). The store lock is taken inside the gate but never while a `kept_*` lock is held. The comment checks (processes) run before the gate; the commit, the snapshot store and the kept list change under it; publication happens after the gate is released. The module doc gets this paragraph, as in `gitlab_refresh.rs:9-16`.
- **"To do" reconcile reads visible and hidden issues** of each loaded repo, joined into one entry per repo key.
- **Logging:** kinds and plain counts only. Never a repo name, reference, number, login, title, comment author or serde error.

## Rules for Implementers (binding for every phase)

- Read `AGENTS.md` first; its Hard Rules are binding: no secrets; `gh`/`glab` read-only and only through a fixed `CliCall` variant covered by the guard test; no work data in the repo, tests, fixtures or logs (invented data only: repos `octo-test/*`, invented logins); never an empty list in place of a failed source; the app writes only its own state.
- Work only in the slice worktree `D:\my-projects\day-navigator\.claude\worktrees\github-hide-resurface` (branch `slice/github-hide-resurface`).
- Run `npm install` once before the first build. If it adds `"license": "MIT"` to `package-lock.json`, revert it (`git checkout -- package-lock.json`) and never commit that change.
- Before every commit run all gates in the worktree: `npm run lint`, `npm run format:check`, `npm run build`, and in `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (allow up to 10 minutes).
- One Conventional Commit per phase (e.g. `feat(github): ...`); the phase's Automated Progress rows are ticked as `references/progress-format.md` describes. Every commit message ends with exactly: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
- Forbidden: `git push`, any merge or rebase, `--no-verify`, `/10x-archive`, editing `context/foundation/roadmap.md`, `npm run tauri dev` or starting the app, touching `%APPDATA%` or other worktrees, new FR numbers in the PRD.
- Manual Progress rows stay unticked; only the user checks them.
- Keep edits to files shared with S-02 and S-06 small and local (see "Parallel Slices").

## Phase 1: Read-only `gh` calls, guard, fake routes and parsers

### Overview

Adds `gh api user` and `gh issue view <N> -R <repo> --json comments` as new `CliCall` variants with fixed arguments, covered by the guard test, with fake-cli routes on invented data and tauri-free parsers and fetch functions. No caller in the app yet.

### Changes Required:

#### 1. CLI calls and guard

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Two new read-only calls: the user's own login and one issue's comments. A new issue-number type keeps the number opaque in `Debug` and never zero.

**Contract**:
- `pub struct GithubIssueNumber` (newtype over a non-zero `u64`, `new(u64) -> Option<Self>`, `get()`, opaque `Debug` like `IssueRef`).
- Variants appended at the end of `CliCall`: `GhUser` and `GhIssueComments(GithubRepo, GithubIssueNumber)`; `tool()` = `Tool::Gh`; `name()` = `"gh:user"` / `"gh:issue-comments"`.
- `args()`: `["api", "user"]` and `["issue", "view", "<N>", "-R", "<repo>", "--json", "comments"]` (number in decimal, repo as written in settings).
- `GhUser` is appended to `CliCall::ALL`; `examples()` appends one `GhIssueComments` with the invented repo `octo-test/alpha` and number 12.
- Test module: `VARIANT_COUNT` 10 → 12, `EXAMPLE_COUNT` 14 → 16, `ordinal` arms `GhUser => 10`, `GhIssueComments(..) => 11`, `seen` array gains `1, 1` at the end, the "ALL must stay parameterless" list gains `GhIssueComments(..)`.
- `check_read_only` gains two exact shapes: `args == ["api","user"]`, and a 7-argument `["issue","view",<guard_number_ok>,"-R",<guard_repo_ok>,"--json","comments"]` (the existing independent validators).
- Near-miss tests rejected by the guard: `api user` plus any extra argument, `api users`, `api user/emails`, `api user -X GET`, `issue view` with `--web`, with another `--json` field list, with `--comments`, with a leading-zero or non-digit number, without `-R`, with the arguments reordered.

#### 2. Parsers

**File**: `src-tauri/src/github/comments.rs` (new, `pub mod comments;` in `github/mod.rs`)

**Intent**: Read only the facts the rules need. The comment body, reactions, URLs and every other field are never deserialized, so no comment text can reach a value, a log or an error.

**Contract**:
- `pub struct GithubLogin` (opaque `Debug`, `as_str()`); `pub fn parse_login(stdout) -> Result<GithubLogin, GithubError>` reads the `login` string of one JSON object; empty, longer than 255 bytes, or containing whitespace/control characters is `ResponseInvalid`.
- `pub struct GithubComment { pub author: Option<String>, pub created_at: DateTime<FixedOffset> }` with a `Debug` that does not print the author; `pub fn parse_comments(stdout) -> Result<Vec<GithubComment>, GithubError>` reads `{"comments":[{"author":{"login":..},"createdAt":..}, ...]}`. A missing or `null` `author`, a missing or `null` `login`, or an empty `login` gives `author: None`; an unreadable `createdAt` or any serde error is `ResponseInvalid` (serde error dropped).

#### 3. Fetch functions

**File**: `src-tauri/src/github/fetch.rs`

**Intent**: Run the two calls; same error mapping as the list call.

**Contract**: `pub fn fetch_login(runner) -> Result<GithubLogin, GithubError>`; `pub fn fetch_issue_comments(runner, &GithubRepo, GithubIssueNumber) -> Result<Vec<GithubComment>, GithubError>`. `CliError` maps through the existing `From<CliError> for GithubError`.

#### 4. Fake CLI routes

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Invented data for every outcome the round must handle.

**Contract**: New `gh-hidden-*` scenarios that route `issue list` like `gh-issues` (repos `octo-test/alpha` and `octo-test/beta`), `api user` → `{"login":"<invented login with a leak marker>","id":1,"type":"User"}`, and `issue view <N> ...` → comments chosen by the scenario: `gh-hidden-none` (no comments), `gh-hidden-other` (one comment by another invented login after the fixture's `updatedAt`), `gh-hidden-own` (only the invented user's own comment, different letter case), `gh-hidden-ghost` (`"author": null` and one with `"login": ""`), `gh-hidden-old` (another author's comment before the baseline), `gh-hidden-bad-json`, `gh-hidden-view-unavailable` (the stderr that the runner classifies as unavailable), `gh-hidden-user-fails` (`api user` exits with a non-zero code), `gh-hidden-closed` (`issue list` of `octo-test/alpha` without one fixture issue), `gh-hidden-beta-fails` (`issue list` of `octo-test/beta` fails). Every comment carries a body with a new marker `LEAK-MARKER-GH-COMMENT-<4 hex>`. Unknown argument shapes keep exiting 98.

#### 5. Tests

**Files**: `src-tauri/src/github/comments.rs` (unit), `src-tauri/tests/github_fetch.rs` (append) or a new `src-tauri/tests/github_comments.rs`

**Intent**: Parsers and fetch functions through a real process, invented data only.

**Contract**: unit tests for `parse_login` and `parse_comments` (ghost/missing/empty author, `null` author, unreadable timestamp, extra fields ignored, empty list); `Debug` of `GithubLogin`, `GithubComment` and `GithubIssueNumber` carries no value; fake-cli tests that every error and `Debug` output is free of the leak markers, that `gh-hidden-view-unavailable` maps to `Unavailable` and `gh-hidden-user-fails` to `Failed`. `tests/cli_runner.rs` keeps passing over the grown `examples()`.

### Success Criteria:

#### Automated Verification:

- Guard test passes with the two new shapes and every near-miss rejected: `cargo test cli::`
- Parser and fake-cli tests pass with no leak marker in any error or `Debug` output: `cargo test --test github_fetch` (or `--test github_comments`)
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- `cargo test --test cli_real -- --ignored --nocapture` on the user's machine prints `gh:user -> data` (logged-in `gh`)

**Implementation Note**: After completing this phase and all automated verification passes, the phase is committed; the Manual row stays unticked for the user. Phase blocks use plain bullets — the corresponding `- [ ]` checkboxes live in the `## Progress` section at the bottom of the plan.

---

## Phase 2: Migration v15, hidden-item store and shared rules

### Overview

Creates the `hidden_items` table and its store operations, makes the rules generic (GitLab adapted with unchanged behaviour), and updates the `AGENTS.md` app-data list.

### Changes Required:

#### 1. Migration v15

**File**: `src-tauri/src/store/schema.rs`

**Intent**: One source-neutral table for hide records of GitHub issues now and GitLab merge requests later (S-07). No titles, URLs, references, comment text or logins.

**Contract**: Appended `Migration { version: 15, .. }` with a comment naming the slice and stating what a row holds. The SQL (exact shape; renumber on merge if develop already has a v15):

```sql
CREATE TABLE hidden_items (
  source              TEXT    NOT NULL CHECK (source IN ('gitlab','github')),
  scope               TEXT    NOT NULL CHECK (length(scope) BETWEEN 1 AND 255),
  kind                TEXT    NOT NULL CHECK (kind IN ('issue','mergeRequest')),
  item_id             INTEGER NOT NULL CHECK (item_id > 0),
  project_id          INTEGER,
  iid                 INTEGER,
  status_fingerprint  TEXT    NOT NULL,
  baseline_updated_at TEXT    NOT NULL,
  settle_after_ms     INTEGER,
  hidden_at_ms        INTEGER NOT NULL,
  absent_since_ms     INTEGER,
  changed_reason      TEXT CHECK (changed_reason IN ('status','comment','returned')),
  changed_at_ms       INTEGER,
  CHECK ((changed_reason IS NULL) = (changed_at_ms IS NULL)),
  CHECK ((project_id IS NULL) = (iid IS NULL)),
  CHECK ((source = 'gitlab') = (project_id IS NOT NULL)),
  PRIMARY KEY (source, scope, kind, item_id)
);
```

#### 2. Generic rules

**File**: `src-tauri/src/hidden_issues/rules.rs`

**Intent**: One copy of the resurfacing rules for both sources (team question 1).

**Contract**: `trait HideItem`, `trait HideRecord`, `struct Comment` and the generic `plan_round` / `needs_comment_check` / `status_changed` / `comment_verdict(&[Comment], baseline, Option<&str>)` as described under "Team Questions". Impls: `HideItem for GitlabItem` (id, `updated_at`, scoped labels), `HideItem for GithubIssue` (number, `updated_at`, `vec![]`), `HideRecord for HiddenRecord`, `HideRecord for HiddenItemRecord`, `From<&IssueNote> for Comment`. `HiddenRecord` keeps its fields and inherent `is_hidden`/`baseline`. Module doc updated: the rules are source-neutral.

**File**: `src-tauri/src/hidden_issues/round.rs`

**Intent**: The GitLab round uses the generic rules; behaviour unchanged.

**Contract**: notes mapped to `Comment`s, user passed as `Option<&str>`; `check_round`/`commit_round` signatures unchanged.

**File**: `src-tauri/src/hidden_issues/mod.rs`

**Intent**: Export the new module and names; GitLab functions untouched.

**Contract**: `pub mod items;`, `pub use crate::gitlab::ChangeReason;`, re-exports of `HideItem`, `HideRecord`, `Comment`; module doc names both tables.

#### 3. Hidden-item store

**File**: `src-tauri/src/hidden_issues/items.rs` (new)

**Intent**: Store ops on `hidden_items`, each one `Store::write`, mirroring the GitLab ops (`hidden_issues/mod.rs:88-271`) with the scope in the key. Errors never carry row values.

**Contract**:
- `pub enum HiddenSource { Gitlab, Github }` (`'gitlab'`/`'github'`), `pub enum HiddenKind { Issue, MergeRequest }` (`'issue'`/`'mergeRequest'`).
- `pub struct HiddenItemRecord { source, scope: String, kind, item_id: u64, project_id: Option<u64>, iid: Option<u64>, status_fingerprint: Vec<String>, baseline_updated_at: String, settle_after_ms, hidden_at_ms, absent_since_ms, changed_reason, changed_at_ms }` with a `Debug` that does not print `scope`.
- `pub struct NewHiddenItem { source, scope, kind, item_id, project_id, iid, status_fingerprint, baseline_updated_at }`; `pub struct HiddenItemKey { source, scope, kind, item_id }`.
- `list_items(store, source, kind) -> Result<Vec<HiddenItemRecord>, HiddenIssueError>` ordered by `scope, item_id`; an unreadable row is `Damaged`.
- `hide_item(store, &NewHiddenItem, now_ms)`: upsert a fresh hidden record (`settle_after_ms = now + SETTLE_MS`, `hidden_at_ms = now`, absent and mark cleared); replaces a marked record.
- `unhide_item(store, &HiddenItemKey)` deletes a hidden record, `dismiss_item` a marked one; otherwise `NotFound`.
- `apply_items_round(store, source, kind, planned: &[(String, Vec<Transition>)], active_scopes: &[String], now_ms) -> Result<Vec<HiddenItemRecord>, HiddenIssueError>`: one write; each transition applied only if its row is still in the state the plan assumed (same guards as `apply_round`); deletes rows of `(source, kind)` whose scope is not in `active_scopes` and that were hidden `PRUNE_AFTER_MS` or more ago; returns every `(source, kind)` row after the write.

#### 4. Tests

**Files**: `src-tauri/src/hidden_issues/tests.rs` (adapted), `src-tauri/tests/hidden_items.rs` (new), `src-tauri/tests/hidden_items_crash.rs` (new)

**Intent**: Prove GitLab behaviour is unchanged and the generic rules and store hold for a GitHub-shaped item, invented data only (repos `octo-test/*`).

**Contract**:
- Existing rule tests keep their cases; only argument types change (`Comment`s, `Option<&str>`).
- New rule tests with a GitHub-shaped item: empty fingerprint never yields `Status`; a ghost author counts; the user's own comment (other letter case) does not; a comment at exactly the baseline does not; no user means any author counts.
- `tests/hidden_items.rs`: hide/list/unhide/dismiss round trip; `NotFound` on the wrong state; guarded transitions skipped after a re-hide; `MarkAbsent` → `Prune` after 30 days; unconfigured-scope prune only after 30 days hidden and only for its `(source, kind)`; GitLab `hidden_issues` rows untouched by every op; the CHECKs refuse a `github` row with `project_id`, a `gitlab` row without it, `item_id = 0`, an empty scope; upgrade test: a store at the version before v15 opens with the full list and keeps its data (content search for `CREATE TABLE hidden_items`, as in `tests/gitlab_view_state.rs:95-118`).
- `tests/hidden_items_crash.rs`: the crash harness (`tests/common/crash.rs`) kills a writer of hides, marks, dismisses and unhides at any moment; a reported op is never lost and no half-applied record exists.

#### 5. AGENTS.md

**File**: `AGENTS.md`

**Intent**: The Hard Rules app-data list states what a GitHub hide record holds, in the same change as the table.

**Contract**: "hidden-issue snapshots" becomes, in substance: "hidden-item snapshots (GitLab issues by host and global id; GitHub issues by the lowercase `owner/name` repo key and issue number; each with a status fingerprint (GitLab scoped labels, empty for GitHub), the baseline `updated_at`, the hide, settle, absent and change instants and the change reason; no title, URL, comment text or login)". The "App Data Store" section is unchanged.

### Success Criteria:

#### Automated Verification:

- Existing GitLab hidden-issue tests pass unchanged in behaviour: `cargo test --test hidden_issues --test hidden_issues_round --test hidden_issues_crash` and `cargo test hidden_issues::`
- New store, upgrade and crash tests pass: `cargo test --test hidden_items --test hidden_items_crash`
- No GitLab call site changed: `git diff --stat $(git merge-base develop HEAD) -- src-tauri/src/gitlab_refresh.rs src-tauri/src/gitlab/ src/gitlab/` is empty (against the merge base, so a develop that moved on does not count)
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 3: GitHub hidden round and snapshot fields

### Overview

The GitHub round as tauri-free functions (check with no lock, commit in one write, partition per repo), the new snapshot fields and their TS mirror. Not wired into the refresh thread yet.

### Changes Required:

#### 1. Snapshot model

**File**: `src-tauri/src/github/mod.rs`

**Intent**: Carry the hidden state to the UI the way `GitlabSnapshot` does.

**Contract**:
- `GithubIssue` gains `changed: Option<ChangeReason>` (serde `changed`) and `hidden_at_ms: Option<i64>` (serde `hiddenAtMs`); the parser sets both to `None`.
- `RepoView` gains `hidden: Vec<GithubIssue>` (the repo's hidden issues, list order); `issues` holds the visible ones (marked ones carry `changed`). A fetched (unpartitioned) view has `hidden: []`.
- `GithubSnapshot` gains `hidden_check_failed: u32` and `hidden_error: Option<String>` (fixed Polish text). `starting()` sets `0`/`None`; `next_snapshot` sets them to `0`/`None` on success (the round sets them after the commit) and carries them over when it keeps the previous repos.
- `repo_view` becomes `pub` (the partition reuses it for a failed repo's error view).
- Struct literals of these types in tests (`github/mod.rs`, `github_refresh.rs`, `health/probe.rs`, `todo/commands.rs`, `todo/tests.rs`, `tests/todo.rs`, `tests/todo_crash.rs`) gain the new fields with empty values; no assertion changes.

#### 2. TS mirror

**File**: `src/github/types.ts`

**Intent**: Same commit as the Rust types (AGENTS.md).

**Contract**: `GithubIssue.changed: ChangeReason | null`, `GithubIssue.hiddenAtMs: number | null`, `RepoView.hidden: GithubIssue[]`, `GithubSnapshot.hiddenCheckFailed: number`, `GithubSnapshot.hiddenError: string | null`. `ChangeReason` is imported from `../gitlab/types` (one mirror per Rust type), not copied. `HiddenIssueError` is not imported here: nothing in `types.ts` uses it, and an unused import fails `npm run build` (`noUnusedLocals`); Phase 5's `api.ts` imports it from `../gitlab/types`. The view still compiles (it reads only `issues`).

#### 3. GitHub round

**File**: `src-tauri/src/hidden_issues/github_round.rs` (new, `pub mod github_round;`)

**Intent**: The GitHub counterpart of `round.rs`, on the shared rules and the `hidden_items` store.

**Contract**:
- `pub struct GithubRoundChecks` (records read before the calls, or the store error; one verdict per `(scope, number)`; `stopped_at: Option<(String, u64)>` with an accessor). `pub fn check_github_round(store: &StoreHandle, runner: &CliRunner, results: &[RepoResult], now_ms: i64, resume_after: Option<(String, u64)>) -> GithubRoundChecks`:
  - reads `list_items(Github, Issue)`; a read failure skips every call;
  - walks the hidden records of repos with an `Ok` outcome whose issue is listed and due (`needs_comment_check`), in `(scope, number)` order rotated after `resume_after`;
  - fetches the login once, before the first due check; on any login failure (`Failed` and `ResponseInvalid` included) no `gh issue view` call starts, every due check is `Err`, and `stopped_at` keeps `resume_after`;
  - an unreadable baseline is `Err` without a call; after the first `Unavailable`/`LoggedOut`/`ToolMissing` from `gh issue view` no further call starts and `stopped_at` is that record;
  - logs only the failed count.
- `pub struct GithubRoundOutcome { pub repos: Vec<RepoView>, pub check_failed: u32, pub store_error: Option<HiddenIssueError> }`. `pub fn commit_github_round(store, config: &GithubConfig, results: &[RepoResult], checks: GithubRoundChecks, now_ms) -> GithubRoundOutcome`:
  - plans per repo with an `Ok` outcome: its records (scope = `repo.key()`), its issues, its verdicts; failed repos contribute nothing;
  - applies everything in one `apply_items_round(Github, Issue, planned, active_scopes = every configured repo key, now_ms)`;
  - partitions from the rows that write returned; counts failed checks only for issues still hidden after the write;
  - on a store read or write failure returns every repo unpartitioned (all issues visible, `changed: None`, `hidden: []`) with `store_error` set, never an empty list.
- `pub fn partition_github(results: &[RepoResult], records: &[HiddenItemRecord]) -> Vec<RepoView>`: one view per result in settings order; a failed repo keeps its error view; for an `Ok` repo, issues without a record are visible, hidden records go to `hidden` with `hidden_at_ms`, marked records stay visible with `changed`; list order kept in both.

#### 4. Tests

**Files**: `src-tauri/src/hidden_issues/github_round.rs` (unit), `src-tauri/tests/github_hidden_round.rs` (new, fake-cli)

**Intent**: Every rule of the round through a real process and a test store, invented data only.

**Contract**: scenarios from Phase 1, rounds driven by calling `check_github_round` + `commit_github_round` as the refresh thread will:
- hide → `gh-hidden-other` after `updatedAt` moved → visible with `changed = comment`;
- `gh-hidden-own` → stays hidden, baseline advances, no further check until `updatedAt` moves again;
- `gh-hidden-ghost` → resurfaces (`comment`);
- `gh-hidden-old` → stays hidden;
- settle check runs once at `SETTLE_MS` and not again;
- `gh-hidden-closed` → absent, then listed again → `changed = returned`; 30 days absent → pruned;
- `gh-hidden-beta-fails` → beta's records untouched (no `absent_since_ms`), alpha still reconciled;
- `gh-hidden-user-fails` → every due check failed, nothing resurfaces by comment, `check_failed` = due count, and the call log (`FAKE_CLI_CALL_LOG`) holds no `issue view` call;
- `gh-hidden-view-unavailable` with two due issues → one call made, `stopped_at` set, next round checks the other first (call log via `FAKE_CLI_CALL_LOG`);
- `gh-hidden-bad-json` → failed check, no leak marker in any error;
- store refused (e.g. a damaged table in a test store) → unpartitioned repos with `store_error`;
- a record of a repo no longer configured → pruned only after 30 days hidden;
- `partition_github` unit tests: order kept, error view kept, `hidden_at_ms` only on hidden items.

### Success Criteria:

#### Automated Verification:

- Round integration tests pass: `cargo test --test github_hidden_round`
- Partition and snapshot unit tests pass: `cargo test github_round` and `cargo test github::`
- No leak marker (title, label, comment, login) in any error, log-facing `Debug` or snapshot error text in these tests
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 4: Refresh wiring and commands

### Overview

`GithubRefreshState` gets the gate and the kept full list, `refresh_round` runs the hidden round, and three commands hide, show and dismiss a GitHub issue by its reference.

### Changes Required:

#### 1. Refresh state and round

**File**: `src-tauri/src/github_refresh.rs`

**Intent**: The GitLab gate pattern (`gitlab_refresh.rs:140-196, 550-704`) for GitHub, kept local to this file.

**Contract**:
- `GithubRefreshState` gains `kept_full: Mutex<Option<Vec<RepoResult>>>` (the unpartitioned results of the last successful round, never serialized; kept and dropped together with `kept_config`) and `hidden_gate: Mutex<()>`. Module doc gains the lock-order paragraph from "Critical Implementation Details".
- `pub(crate) fn act_on_issue(&self, reference: &str, act: impl FnOnce(&GithubRepo, &GithubIssue) -> Result<Vec<HiddenItemRecord>, HiddenIssueError>) -> Result<(), HiddenIssueError>`: under the gate; `ListUnavailable` without a kept config and list; `NotInList` unless an `Ok` repo of the kept list holds an issue with that reference; runs `act`, re-partitions the kept list with `partition_github`, stores in one snapshot write `repos`, `hidden_error = None`, `hidden_check_failed = min(current, hidden total)` and `seq + 1`. The caller publishes.
- `refresh_loop` keeps a `resume_after: Option<(String, u64)>` across rounds.
- `refresh_round`: after `run_fetch`, when there is a config and an `Ok` list, `check_github_round` (no lock; updates `resume_after`); then take the gate; build `next` with `next_snapshot`; on success run `commit_github_round` and set `next.repos`, `next.hidden_check_failed`, `next.hidden_error` (`HiddenIssueError::message_pl` of the store error); update `kept_full` (success: the results; failure without kept config: `None`; failure with kept config: unchanged) and `kept_config`; reconcile "To do"; store the snapshot; drop the gate; publish; then the round tail as today. The returned `Duration` stays `REFRESH_INTERVAL` (S-02 owns it).
- `reconcile_input` returns one `(repo key, visible + hidden issues)` entry per loaded repo of an `Ok` round; the call site passes them to `todo::reconcile_github` unchanged in meaning.

#### 2. Commands

**File**: `src-tauri/src/hidden_issues/commands.rs`

**Intent**: The UI entry, like the GitLab commands; the issue data comes from the kept list, never from the UI.

**Contract**: `#[tauri::command] pub async fn hide_github_issue(app, reference: String) -> Result<(), HiddenIssueError>`, `unhide_github_issue`, `dismiss_github_change`. Each runs on the blocking pool through `crate::blocking`, calls `GithubRefreshState::act_on_issue` with a closure that runs `hide_item` (`NewHiddenItem` from the issue: `Github`, `repo.key()`, `Issue`, number, no `project_id`/`iid`, `status_fingerprint()`, listed `updated_at`) / `unhide_item` / `dismiss_item` and then `list_items(Github, Issue)`, then publishes on success. Store errors logged at warn by kind, refusals at debug by kind. Module doc updated (GitLab by `id`, GitHub by `reference`).

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the commands.

**Contract**: three lines appended right after `hidden_issues::commands::dismiss_gitlab_change` in `generate_handler!`.

#### 3. Tests

**File**: `src-tauri/src/github_refresh.rs` (unit tests)

**Intent**: The state rules without a Tauri app.

**Contract**: `act_on_issue` → `ListUnavailable` with no kept list; `NotInList` for an unknown reference and for an issue of a failed repo; success re-partitions, clears `hidden_error`, caps `hidden_check_failed` and bumps `seq` by one; `reconcile_input` includes hidden issues once per repo key and still returns nothing for a non-`Ok` snapshot.

### Success Criteria:

#### Automated Verification:

- State and reconcile unit tests pass: `cargo test github_refresh::`
- Phase 3 round tests still pass: `cargo test --test github_hidden_round`
- The three commands are registered: `grep -c "_github_issue\|dismiss_github_change" src-tauri/src/lib.rs` prints 3
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

**Implementation Note**: Commit when all automated rows are green. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Phase 5: "Projekty (GitHub)" UI

### Overview

Row actions, the "Pokaż ukryte (n)" toggle, dimmed hidden rows with "ukryte DD.MM", the "Zmienione" flag and the notices, mirroring the GitLab view.

### Changes Required:

#### 1. API and hook

**File**: `src/github/api.ts`

**Intent**: One typed function per command; GitHub-worded error texts.

**Contract**: `hideGithubIssue(reference)`, `unhideGithubIssue(reference)`, `dismissGithubChange(reference)` → `invoke("hide_github_issue" | "unhide_github_issue" | "dismiss_github_change", { reference })`; `isGithubHiddenStale(e)` (true for `NotFound`/`NotInList`; `HiddenIssueError` imported from `../gitlab/types`); `githubHiddenErrorMessage(e)` with the GitLab texts except `ListUnavailable` → "Lista GitHuba nie jest jeszcze dostępna.", `Store` → `storeErrorMessage`.

**File**: `src/github/useGithub.ts`

**Intent**: The GitLab `act` pattern (`useGitlab.ts:92-114`): busy set, action error, refresh on a stale row, no optimistic update (the new snapshot arrives as `github-changed`).

**Contract**: returns additionally `actionError`, `busy: ReadonlySet<string>` (references), `hide`, `unhide`, `dismiss` (each `(reference: string) => void`). Existing callers (`TodoView.tsx`) keep working unchanged.

#### 2. Row component extension

**Files**: `src/components/ItemRow/ItemRow.tsx`, `src/components/ItemRow/ItemRow.css`

**Intent**: Additive props so a row can show several flags and the hidden look; every current caller is unchanged.

**Contract**: `extra?: ReactNode` rendered right after `flag`; `dimmed?: boolean` adds class `is-dimmed` (`opacity` like `.gitlab-row.is-hidden`, no colour literal).

#### 3. View

**Files**: `src/github/GithubView.tsx`, `src/github/GithubView.css`

**Intent**: Mirror the GitLab hidden UI in the GitHub layout.

**Contract**:
- View head: "Pokaż ukryte (n)" (`Button size="sm" variant="quiet" pressed`), disabled when n is 0 and the toggle is off; n = hidden issues across repos without an error; state in `useState`, in memory only.
- `RepoGroup`: visible rows, then (toggle on) the repo's hidden rows with `dimmed`. Count `[n]` stays the visible count. Empty text: "Brak otwartych zgłoszeń przypisanych do Ciebie." only when the repo has neither visible nor hidden issues; with only hidden issues and the toggle off: "Wszystkie przypisane zgłoszenia są ukryte."
- `IssueRow`: `flag` stays "W Do zrobienia"; `extra` holds "ukryte DD.MM" (`formatDay(hiddenAtMs, now)`, or "ukryte" without a time) on hidden rows and the "Zmienione" flag on marked rows (tone `change`, tooltip and screen-reader text from the same texts as GitLab's `CHANGE_LABEL`: "zmiana statusu", "nowy komentarz", "wróciło na listę"; `CHANGE_LABEL` is local to `GitlabView.tsx`, so `GithubView.tsx` keeps its own copy and `GitlabView.tsx` is neither edited nor imported from). Actions in GitLab order: "OK, zauważone" (`check-circle`) when marked, "Pokaż na liście" (`eye`) when hidden, else "Ukryj do zmiany" (`eye-off`); disabled while the reference is busy; aria-labels `OK, zauważone: <title>`, `Pokaż: <title>`, `Ukryj: <title>`. `TodoAddButton` only when the row is visible or already in "To do" (as `GitlabView.tsx:249`); the external link stays.
- "Nic przypisanego" only when no repo has visible or hidden issues.
- Notices (as `GitlabView.tsx:678-688`): `hiddenError` → "Lista pokazuje wszystkie zgłoszenia, bo nie udało się odczytać ukrytych. <text>"; `hiddenCheckFailed > 0` → "Nie udało się sprawdzić N ukrytych — spróbuję przy następnym odświeżeniu"; `actionError` as an error notice.
- The "Zakres" panel gains the rule note: "Ukryte zgłoszenie wraca samo, gdy ktoś inny doda komentarz albo zgłoszenie wróci na listę po zamknięciu i ponownym otwarciu." with a comment that it mirrors `plan_round` in `src-tauri/src/hidden_issues/rules.rs`.
- No literal colours; view CSS stays layout only.

#### 4. "Do zrobienia" flag

**File**: `src/todo/TodoView.tsx`

**Intent**: GitLab entries in "Do zrobienia" show "zmienione" when their issue carries a mark (`TodoView.tsx:235-240, 288-290`); GitHub entries get the same flag from the GitHub snapshot. **Default chosen by agent:** "both sources behave alike" covers the one other place GitLab shows the mark, and the data is already in the snapshot the view holds.

**Contract**: a `changedGithub` set from `githubSnapshot.repos[].issues` with `changed !== null`, keyed like the GitHub entry; an unticked GitHub entry in the set gets the flag `{ text: "zmienione", tone: "change" }`. The picker keeps listing `repos[].issues` only, so hidden GitHub issues drop out of it like hidden GitLab issues.

#### 5. AGENTS.md

**File**: `AGENTS.md`

**Intent**: Record the in-memory toggle next to the GitLab list-state rule.

**Contract**: one sentence in "Frontend Conventions → Navigation": the "Pokaż ukryte" toggle of "Projekty (GitHub)" is view state in memory only.

### Success Criteria:

#### Automated Verification:

- Type check and build pass with the new fields and props: `npm run build`
- No colour literal outside tokens: `git diff $(git merge-base develop HEAD) -- src/github src/components/ItemRow src/todo | grep -E "^\+.*#[0-9a-fA-F]{3,6}\b"` prints nothing
- All AGENTS.md gates green: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- Hiding a GitHub issue removes it from its repo panel; "Pokaż ukryte (1)" shows it dimmed with "ukryte DD.MM" (today's date) after the visible rows
- "Pokaż na liście" brings the hidden issue back unmarked
- A comment by another account on a hidden issue makes it visible after the next refresh with "Zmienione" and the tooltip "nowy komentarz"; "OK, zauważone" clears the mark
- The user's own comment on a hidden issue leaves it hidden
- Closing a hidden issue and reopening it (with at least one refresh in between) brings it back marked "Zmienione" with "wróciło na listę"
- A hidden issue that is in "Do zrobienia" keeps its entry (not ticked as closed) and its row keeps the "Do zrobienia" button
- GitLab hiding ("Praca (GitLab)") works as before
- The built app (`npm run tauri build -- --debug --no-bundle`) shows no CSP violation in the DevTools console, and the app log holds no repo name, login, title or comment text from the hidden checks
- A returned GitHub issue that is in "Do zrobienia" shows "zmienione" there until the mark is dismissed

**Implementation Note**: Commit when all automated rows are green; the Manual rows stay unticked for the user. Phase blocks use plain bullets — the `- [ ]` checkboxes live in `## Progress`.

---

## Parallel Slices: Shared Files and Merge Points

Keep edits to shared files small and local: append enum variants, match arms, struct fields and handler lines at the end of their lists; never reorder or reformat unrelated code.

| File / area | S-01 (this) | S-02 `github-refresh-interval` (#29, no migration) | S-06 `gitlab-review-requests` (#35, no migration) | Expected merge point |
| --- | --- | --- | --- | --- |
| `src-tauri/src/cli/mod.rs` | `GhUser`, `GhIssueComments`, `GithubIssueNumber`, counts, `ordinal` 10/11, `seen` +2, two guard shapes, near-miss tests | — | a new `glab` variant, counts, `ordinal`, `seen`, a guard shape | Textual: `VARIANT_COUNT`, `EXAMPLE_COUNT`, `ordinal`, `seen` and `check_read_only` lines. The second slice to merge renumbers its ordinals and recomputes the counts and the `seen` array |
| `src-tauri/src/bin/fake_cli.rs`, `tests/cli_runner.rs` | `gh-hidden-*` scenarios and routes | — | a `glab` review-request route | New arms in `main`'s scenario `match`; keep both |
| `src-tauri/src/github_refresh.rs` | `kept_full`, `hidden_gate`, `act_on_issue`, check step, gated commit, `resume_after`, `reconcile_input` | interval from the setting instead of `REFRESH_INTERVAL`, `next_refresh_at_ms` before the store | — | `refresh_round` body and `refresh_loop`: S-02 changes the wait and the last lines before the store; S-01 wraps the store in the gate. Resolve by keeping the gate and setting `next_refresh_at_ms` inside it, right before the snapshot store (as GitLab does) |
| `src-tauri/src/github/mod.rs` `GithubSnapshot`, `starting()`, `next_snapshot` | `hidden_check_failed`, `hidden_error`; `GithubIssue.changed/hidden_at_ms`; `RepoView.hidden` | `refresh_minutes`, `next_refresh_at_ms`, maybe `refresh_setting_invalid` | — | Adjacent field lists in the struct, `starting()` and `next_snapshot`: keep both sets |
| Test literals of `GithubSnapshot` (`health/probe.rs`, `todo/commands.rs`, `github_refresh.rs`) | new fields | new fields | — | Keep both sets of fields |
| `src/github/types.ts` | hidden fields | interval fields | — | Adjacent lines: keep both |
| `src/github/GithubView.tsx`, `useGithub.ts`, `api.ts` | toggle, rows, actions, notices | "every N min — next try" in the unavailable/error states | — | View head and the state blocks: keep both; S-02's retry text and S-01's notices are separate elements |
| `src/components/ItemRow/`, `src/todo/TodoView.tsx` | additive `extra`/`dimmed` props; GitHub "zmienione" flag | — | — | None expected |
| `src-tauri/src/settings.rs` + TS mirror | none | `githubRefresh.minutes` | maybe | No S-01 conflict |
| `src-tauri/src/hidden_issues/*` | generic rules, `items.rs`, `github_round.rs`, GitHub commands | — | expected untouched | None expected; GitLab signatures kept stable on purpose |
| `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/gitlab/`, `src/gitlab/` | **not touched** | — | new fetch, review section | None from S-01 |
| `src-tauri/src/lib.rs` `generate_handler!` | 3 GitHub commands after `dismiss_gitlab_change` | — | maybe new commands | Append-only; keep both |
| `src-tauri/src/store/schema.rs` | v15 `hidden_items` (the wave's only migration) | none | none | None in this wave; if develop has a v15 at merge time, renumber and note it in the migration comment |
| `AGENTS.md` | app-data list (Phase 2), GitHub toggle note (Phase 5) | maybe a settings note | maybe a CLI or notices note | Different bullets; keep both |

## Testing Strategy

### Unit Tests:

- Generic rules with GitLab and GitHub items: fingerprint change, comment verdict (own, other, ghost, at baseline, no user), settle check, absent/returned/prune, advance baseline.
- Parsers: login, comments (ghost, missing, empty, null author, bad timestamp, extra fields).
- Guard: two new shapes and every near-miss.
- `partition_github`, `act_on_issue`, `reconcile_input`.

### Integration Tests:

- `tests/github_fetch.rs` (or `github_comments.rs`): the two calls through `fake-cli`.
- `tests/hidden_items.rs`, `tests/hidden_items_crash.rs`: store ops, CHECKs, upgrade, durability.
- `tests/github_hidden_round.rs`: full rounds with `fake-cli` and a test store.
- Existing `tests/hidden_issues*.rs`: GitLab unchanged.

### Manual Testing Steps:

1. In "Projekty (GitHub)", hide an issue of a configured repo; check the toggle and the dimmed row with today's date.
2. From another account, comment on it; refresh; check "Zmienione" with "nowy komentarz"; click "OK, zauważone".
3. Hide it again, comment with your own account, refresh: it stays hidden.
4. Close it, refresh, reopen it, refresh: it comes back with "wróciło na listę".
5. Check a hidden issue that is in "Do zrobienia" stays there.
6. Check GitLab hiding still works; open the built app and check the DevTools console and the app log.

## Performance Considerations

A round adds one `gh issue view` per hidden issue whose `updatedAt` moved or whose settle check is due, plus one `gh api user` when at least one check is due. The calls run on the refresh thread before the gate, sequentially, like the GitLab notes calls; a whole-source failure stops them after the first one.

## Migration Notes

v15 only creates `hidden_items`; existing data is untouched and no GitLab row moves. A database from this build opened by an older build is refused as `NewerSchema` (existing rule). Rolling back the feature needs no data change: the table is simply unused.

## References

- Research: `context/changes/github-hide-resurface/research.md`
- GitLab hiding: `src-tauri/src/hidden_issues/` (`rules.rs:110-192`, `mod.rs:94-271`, `round.rs:80-199`, `commands.rs:20-81`), `src-tauri/src/gitlab_refresh.rs:140-196, 550-704`
- GitHub refresh: `src-tauri/src/github_refresh.rs:54-226`, `src-tauri/src/github/mod.rs:19-330`, `src-tauri/src/github/fetch.rs:15-46`
- CLI guard: `src-tauri/src/cli/mod.rs:328-466, 529-758`
- GitLab hidden UI: `src/gitlab/GitlabView.tsx:62-158, 178-199, 458-523, 678-694`, `src/gitlab/api.ts:25-73`, `src/gitlab/useGitlab.ts:92-114`
- Prior plans: `context/archive/2026-11-23-hide-and-resurface/plan.md`, `context/archive/2026-11-27-source-refresh-skeleton/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Read-only `gh` calls, guard, fake routes and parsers

#### Automated

- [x] 1.1 Guard test passes with the two new shapes and every near-miss rejected — ff8e4f7
- [x] 1.2 Parser and fake-cli tests pass with no leak marker in any error or Debug output — ff8e4f7
- [x] 1.3 All AGENTS.md gates green — ff8e4f7

#### Manual

- [x] 1.4 Real-CLI smoke test prints gh:user -> data on the user's machine — manual 2026-11-28

### Phase 2: Migration v15, hidden-item store and shared rules

#### Automated

- [x] 2.1 Existing GitLab hidden-issue tests pass unchanged in behaviour — 409f337
- [x] 2.2 New store, upgrade and crash tests pass — 409f337
- [x] 2.3 No GitLab call site changed — 409f337
- [x] 2.4 All AGENTS.md gates green — 409f337

### Phase 3: GitHub hidden round and snapshot fields

#### Automated

- [x] 3.1 Round integration tests pass — a6a4de1
- [x] 3.2 Partition and snapshot unit tests pass — a6a4de1
- [x] 3.3 No leak marker in any error, Debug or snapshot error text — a6a4de1
- [x] 3.4 All AGENTS.md gates green — a6a4de1

### Phase 4: Refresh wiring and commands

#### Automated

- [x] 4.1 State and reconcile unit tests pass — 13f0761
- [x] 4.2 Phase 3 round tests still pass — 13f0761
- [x] 4.3 The three commands are registered — 13f0761
- [x] 4.4 All AGENTS.md gates green — 13f0761

### Phase 5: "Projekty (GitHub)" UI

#### Automated

- [x] 5.1 Type check and build pass with the new fields and props — 16f648c
- [x] 5.2 No colour literal outside tokens — 16f648c
- [x] 5.3 All AGENTS.md gates green — 16f648c

#### Manual

- [x] 5.4 Hiding removes the issue; the toggle shows it dimmed with ukryte DD.MM — manual 2026-11-28
- [x] 5.5 Pokaż na liście brings the issue back unmarked — manual 2026-11-28
- [x] 5.6 A comment by another account resurfaces it with Zmienione (nowy komentarz); OK, zauważone clears it — manual 2026-11-28
- [x] 5.7 The user's own comment leaves it hidden — manual 2026-11-28
- [x] 5.8 Close and reopen brings it back with wróciło na listę — manual 2026-11-28
- [x] 5.9 A hidden issue in Do zrobienia keeps its entry and its button — manual 2026-11-28
- [x] 5.10 GitLab hiding works as before — manual 2026-11-28
- [x] 5.11 Built app shows no CSP violation and the log holds no repo, login, title or comment text — manual 2026-11-28
- [x] 5.12 A returned GitHub issue in Do zrobienia shows zmienione until dismissed — manual 2026-11-28
