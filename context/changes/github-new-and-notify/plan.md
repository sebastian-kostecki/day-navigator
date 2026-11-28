# GitHub NEW flag since the last visit (S-03 `github-new-and-notify`) Implementation Plan

## Overview

The "Projekty (GitHub)" view gets the same blue **Nowe** flag the GitLab view has: an issue is NEW when it appeared on the list since the user's last visit to the GitHub view. The rule lives in Rust (a new tauri-free module `github_seen`, store migration v16); the UI only shows the flag and reports the end of a visit. There are no Windows notifications for GitHub (FR-004 notices stay GitLab-only). Slice S-03 of M-4, GitHub issue #30, anchor MS-30.

This plan was written non-interactively by the plan sub-subagent. Every choice beyond the user's decisions is marked **(default chosen by agent)**.

User decisions (binding, verbatim):

- NEW flag only on GitHub items that appeared since the last visit to the GitHub view. NO Windows notifications for GitHub (FR-004 notices stay GitLab-only).
- GitHub seen keys and the GitHub view's visit mark need a migration: this slice takes migration v16 (the wave's only migration; append to src-tauri/src/store/schema.rs, never edit a shipped one).
- Update the AGENTS.md app-data list for the new records (no titles, URLs, logins; opaque/minimal keys as for GitLab).
- Respect the AGENTS.md rule "never subscribe to the timer status a second time" and the existing GitHub "Pokaż ukryte" in-memory toggle; the first-ever visit must not mark everything NEW (seed like GitLab does) — plan decides details, marked "default chosen by agent".

## Current State Analysis

- GitLab NEW is a rule on stored instants: an item is NEW iff its `first_seen_ms` is strictly after the account's seen-until mark; no mark means nothing is NEW; the account's seeding round returns nothing fresh (`src-tauri/src/new_assignments/rules.rs` `fresh_since`, `src-tauri/src/new_assignments/mod.rs` `record_round`).
- The mark moves on the end of a visit to the `now_ms` of the last round whose seen set was written (`shown_until`), never the clock, with `max()` so it never moves back (`src-tauri/src/gitlab_refresh.rs` `accept_round`, `mark_seen`, `mark_gitlab_seen`; `new_assignments::mark_view_seen`).
- The UI reports the end of a visit with `useGitlabSeenMark(view)` (`src/gitlab/useGitlabSeenMark.ts`), called in `src/App.tsx` with the view id only: it does not touch the work timer.
- The GitLab seen tables (`assignment_accounts`, `seen_assignments`, `gitlab_view_marks`; migrations v7 and v9) are GitLab-shaped: `kind` CHECK, GitLab global ids, FK to an account row. GitHub identity is `(lowercase owner/name, issue number)` and GitHub has no account key (`src-tauri/src/cli/mod.rs` `GithubConfig { repos }`, `GithubRepo::key()`).
- `src-tauri/src/github_refresh.rs` has no seen-set code: state is `cell`, `kept_config`, `kept_full`, `hidden_gate`. `refresh_round` fetches, runs the comment checks (no lock), then under `hidden_gate` builds `next` with `github::next_snapshot`, partitions with `commit_github_round`, updates `kept_full` and `kept_config`, reconciles "To do", stores the snapshot, releases the gate and publishes. `act_on_issue` re-partitions `kept_full` for the hide/unhide/dismiss commands.
- `GithubIssue` struct literals appear 21 times in 9 files, `RepoView` literals in `hidden_issues/github_round.rs`, `health/probe.rs` and `todo/commands.rs`; every `GithubSnapshot` literal outside `github/mod.rs` uses `..GithubSnapshot::starting()`.
- The store is at migration v15 (`hidden_items`). Tests derive the latest version from `schema::MIGRATIONS.len()`; foreign keys are on (`PRAGMA foreign_keys=ON`, `src-tauri/src/store/mod.rs`).

## Desired End State

- Migration v16 adds `github_seen_repos`, `github_seen_issues` and `github_view_mark`; they hold only the lowercase repo key, issue numbers and instants.
- Every successful GitHub round records the issues (visible and hidden) of each repo that loaded, in one `Store::write`; a repo's first stored round seeds silently; an issue absent for 30 days is forgotten; rows of repos no longer in settings are dropped.
- The snapshot carries `newReferences`: the references of visible issues first seen after the GitHub view's mark. The GitHub view shows `Nowe` on those rows.
- Leaving the GitHub view (or hiding the window while it is active) saves the mark of the last shown round and clears the flags. The first-ever visit shows nothing NEW.
- No Windows notification, no event-log entry, no new CLI call.
- AGENTS.md lists the new records.

Verify: all gates green; `cargo test --test github_seen`, `cargo test --test github_seen_crash`, `cargo test --lib github_seen`, `cargo test --lib github_refresh`, `cargo test --lib github::tests` pass; the Manual rows below are checked by the user in a built app.

### Key Discoveries:

- `record_round` in `src-tauri/src/new_assignments/mod.rs` is the pattern for one-write seen-set rounds (read marker, rows and mark; plan; insert marker; upsert; forget; return fresh).
- `baseline_first_seen(mark, now) = min(mark, now)` (`new_assignments/rules.rs`) is how S-06 stores a silent baseline so a later round never flags it. GitHub's per-repo seeding needs a stronger baseline, because the GitHub mark is global and seeding is per repo: a repo can seed before the first mark exists and a mark older than that round's `now` can still be saved afterwards (plan review F1). Seeded GitHub keys are therefore stored at `first_seen_ms = 0` (decision 8), and `baseline_first_seen` is not used.
- `accept_round` / `mark_seen` in `src-tauri/src/gitlab_refresh.rs` (and their tests `mark_seen_without_shown_until_saves_nothing`, `mark_seen_saves_shown_until_clears_flags_and_bumps_seq`, `a_failed_seen_set_write_keeps_the_flags_and_shown_until`) are the patterns to mirror.
- `partition_github` (`src-tauri/src/hidden_issues/github_round.rs`) rebuilds every `GithubIssue` and `RepoView`; a flag carried on the snapshot instead of on the issue or repo view needs no edit there.
- `tests/hidden_items.rs` `v14_database_upgrades_to_the_hidden_items_table_and_keeps_its_data` and `tests/new_assignments_crash.rs` are the migration-test and crash-test patterns.

## What We're NOT Doing

- No Windows notification (toast) for GitHub and no "Nowe" entry in today's event log (user decision; FR-004 stays GitLab-only).
- No edit in `src-tauri/src/new_assignments/`, `src-tauri/src/hidden_issues/`, `src-tauri/src/cli/`, `src-tauri/src/bin/fake_cli.rs`, `src-tauri/src/source_refresh.rs`, `src/gitlab/` (S-07 owns those files in this wave, or they need no change). `new_assignments` items are only imported.
- No `is_new` field on `GithubIssue` and no field on `RepoView` **(default chosen by agent)**: the flags travel on `GithubSnapshot.new_references`, so the 21 `GithubIssue` literals and the `RepoView` literal in `hidden_issues/github_round.rs` stay untouched.
- No change to the "Pokaż ukryte" toggle (stays `useState` in `GithubView`, in memory only) and no change to `WorkTimerProvider` / `useWorkTimer`.
- No session fallback (`SessionSeen`) for a failing store: it exists for GitLab toasts only **(default chosen by agent)**.
- No PRD note and no roadmap edit **(default chosen by agent)**: the roadmap names PRD notes for S-06 and S-07 only, and the wave rules forbid editing `context/foundation/roadmap.md` (the roadmap status sync of `/10x-plan` is skipped for that reason).
- No change to the "Zakres" side-panel text of the GitHub view.

## Implementation Approach

A tauri-free module `src-tauri/src/github_seen/` holds the rules (`rules.rs`), the error (`error.rs`) and the store operations (`mod.rs`), with its own tables in migration v16 (research Open Question 2: own module, no generic shared with GitLab; `FORGET_AFTER_MS` is imported from `new_assignments`, never edited) **(default chosen by agent)**. `github_refresh.rs` mirrors GitLab's `kept_new` / `shown_until` / `last_mark` state, records the seen set before the gate, accepts the fresh keys under it, and writes `next.new_references` on every path that shapes the list. A new command `mark_github_seen` saves the mark. The UI adds a twin hook `useGithubSeenMark(view)` next to the GitLab one in `App.tsx` and renders `Nowe` from `snapshot.newReferences`.

Order: schema and rules → store operations → snapshot contract → refresh state and command → UI → AGENTS.md. Each phase compiles, passes every gate and is committed on its own.

Resolved decisions (research Open Questions and the slice's open details):

1. **Repo key plain vs hashed** → plain lowercase `owner/name` (`GithubRepo::key()`), the same value `hidden_items.scope` already stores; no new kind of work data **(default chosen by agent)**.
2. **Own tables and rules vs a generic shared with GitLab** → own module `github_seen` and own tables; import `new_assignments::FORGET_AFTER_MS` without editing that module **(default chosen by agent)**.
3. **Prune of repos removed from settings** → in every successful round, the rows and the seeding marker of every stored repo not in this round's settings are deleted; re-adding a repo later seeds silently again. Within a loaded repo, an issue absent for 30 days or more (`FORGET_AFTER_MS`, by `last_seen_ms`) is deleted. A configured repo that failed this round is not recorded, not forgotten and not pruned **(default chosen by agent)**.
4. **Where the flag travels** → `GithubSnapshot.new_references: Vec<String>` (TS `newReferences: string[]`), the references of visible NEW issues in list order; this deviates from the research default (`is_new` on `GithubIssue`) to keep `hidden_issues/` and the 21 literals untouched, as the orchestrator preferred **(default chosen by agent)**.
5. **Seen-mark hook** → a twin `src/github/useGithubSeenMark.ts`; `src/gitlab/useGitlabSeenMark.ts` stays unchanged **(default chosen by agent)**.
6. **PRD one-liner for MS-30** → none (see "What We're NOT Doing") **(default chosen by agent)**.
7. **`main-hidden` emit reliability on close** → accepted as for GitLab: if the event is lost, no mark is saved and the flags simply show again next time (the safe side) **(default chosen by agent)**.
8. **Seeding** → per repo: a repo with no row in `github_seen_repos` seeds silently in its first successful round, also after the user adds it to settings. Seeded keys are stored with `first_seen_ms = 0` and `last_seen_ms = now`, so no later round flags them, whatever mark is saved later. Two cases need this: the global mark can predate a repo added later, and a repo can seed in a round that read no mark while a visit ending between `record_seen` and the gate saves the previous round's `shown_until`, which is older than that round's `now` (plan review F1). `0` is at or below every possible mark, so the strict `first_seen_ms > mark` rule never holds for a seeded key. `mark.map_or(0, |m| m.min(now))` would be equally safe (a saved mark only moves forward with `max()`), but it adds a branch for no gain and an import of `baseline_first_seen`, so the constant `0` is used. `first_seen_ms` is read only by the NEW rule; the 30-day forget uses `last_seen_ms` only, so a `0` row is forgotten 30 days after it was last seen, like any other **(default chosen by agent)**.
9. **First-ever visit** → no mark means nothing is fresh; the first mark is written when the user first leaves the GitHub view after a successful round; only issues first seen after that are NEW (as GitLab) **(default chosen by agent)**.
10. **The mark** → one global row (`github_view_mark`, `id = 1`), not per repo; its value is `shown_until` (the `now_ms` of the last round stored in the snapshot whose seen-set write succeeded), never the wall clock, saved with `max()` **(default chosen by agent)**.
11. **Hidden issues** → recorded in the seen set (so hiding and unhiding never makes an issue NEW) but never flagged while hidden; an issue unhidden during the same visit shows `Nowe` again if it is still fresh **(default chosen by agent)**.
12. **End of a visit** → the active view leaves `"github"`, or `main-hidden` fires while `"github"` is active; quitting from the tray saves no mark. The hook takes only `view` and never reads timer state, so "never subscribe to the timer status a second time" holds **(default chosen by agent)**.
13. **Store failure** → a failed seen-set write leaves `kept_new` and `shown_until` unchanged (leaving the view never marks unshown items as seen); the first non-busy failure of a session logs a warning by kind, later ones and `Busy` log at debug; no UI notice (the flag is a convenience, not a source) **(default chosen by agent)**.
14. **Error type** → a new `GithubSeenError` (`Store`, `Background`; same shape as `GitlabViewStateError`) with a TS mirror in `src/github/types.ts` added in the phase that registers the command **(default chosen by agent)**.
15. **`lib.rs` placement** → `pub mod github_seen;` inserted right after `pub mod github_refresh;`; `github_refresh::mark_github_seen,` inserted right after `github_refresh::refresh_github,` in `generate_handler!`; nothing reordered **(default chosen by agent)**.
16. **Logs** → kinds only: `github: seen set failed: {kind}` and `github: mark seen failed: {kind}`; never a repo, number, reference or title **(default chosen by agent)**.

## Critical Implementation Details

- **State sequencing.** In `refresh_round` the seen set is written before `hidden_gate` is taken (it takes the store lock; no `kept_*` lock may be held then), `accept_round` runs right after the gate is taken, and `next.new_references` is written after the `kept_full` block, before `*state.write() = next`. `results` is moved inside the `kept_full` block, so compute `list_dropped` before it.
- **Lock order** becomes: `hidden_gate`, `kept_config`, `kept_full`, `kept_new`, `shown_until`, `last_mark`, snapshot. `mark_seen` clones `shown_until` and releases it before calling `save` (which takes the store lock).
- **Seeding.** Storing a seeded repo's keys at `now` would flag them all in a later round whenever the global mark is older than that `now` (a mark that predates the repo, or a mark saved by a visit that ended mid-round before the first mark existed); store them at `first_seen_ms = 0` instead (decision 8).

## Phase 1: Migration v16, rules and error

### Overview

Adds the three tables, the pure NEW rules for one repo and the module's error type, with unit and migration tests. Nothing calls them yet.

### Changes Required:

#### 1. Migration v16

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append one `Migration { version: 16, .. }` after the v15 entry (never edit v1–v15), with a comment in the style of the v14/v15 comments: "v16 (slice S-03 `github-new-and-notify`): seen GitHub issues for the NEW flag of the GitHub view, keyed by the lowercase `owner/name` repo key and issue number, a per-repo seeding marker and the view's one seen-until mark. Only keys and instants: no title, URL, reference, label or login."

**Contract**: exactly this SQL (one `sql` string; the store runs it in the migration's transaction):

```sql
CREATE TABLE github_seen_repos (
  repo         TEXT    PRIMARY KEY CHECK (length(repo) BETWEEN 1 AND 255 AND repo = lower(repo)),
  seeded_at_ms INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE github_seen_issues (
  repo          TEXT    NOT NULL REFERENCES github_seen_repos(repo),
  issue_number  INTEGER NOT NULL CHECK (issue_number > 0),
  first_seen_ms INTEGER NOT NULL,
  last_seen_ms  INTEGER NOT NULL,
  PRIMARY KEY (repo, issue_number)
) WITHOUT ROWID;
CREATE TABLE github_view_mark (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  seen_until_ms INTEGER NOT NULL
);
```

#### 2. Module skeleton

**File**: `src-tauri/src/github_seen/mod.rs` (new)

**Intent**: Module doc: "Seen GitHub issues (S-03): the durable seen set behind the NEW flag of the GitHub view. No `tauri` import and no clock: `now_ms` is a parameter. A round is one `Store::write`. Rows hold the lowercase repo key, issue numbers and instants only. No toast, no event log." Declare `pub mod error; pub mod rules;` and re-export `pub use error::GithubSeenError;` and `pub use rules::{RepoPlan, SeenKey, StoredIssue, plan_repo};`.

**Contract**: module path `crate::github_seen`.

**File**: `src-tauri/src/lib.rs`

**Intent**: Insert `pub mod github_seen;` on the line right after `pub mod github_refresh;`. Nothing else in `lib.rs` changes in this phase.

#### 3. Error

**File**: `src-tauri/src/github_seen/error.rs` (new)

**Intent**: Copy the shape of `src-tauri/src/gitlab_view_state/error.rs`: an enum that never carries a row value.

**Contract**: `#[derive(Debug, Clone, thiserror::Error, Serialize)] #[serde(tag = "kind", content = "detail")] pub enum GithubSeenError { #[error("app data store error: {0}")] Store(StoreError), #[error("background task failed")] Background }`; `impl From<crate::BackgroundFailed>` (→ `Background`), `From<StoreError>` (→ `Store`), `From<rusqlite::Error>` (→ `Store(StoreError::from(e))`); `pub fn kind(&self) -> &'static str` (`Store(e) => e.kind()`, `Background => "Background"`).

#### 4. Pure rules

**File**: `src-tauri/src/github_seen/rules.rs` (new)

**Intent**: The NEW rule of one repo for one round: seeding, insert instant, 30-day forget and fresh keys. No store, clock or process.

**Contract**:

- `pub use crate::new_assignments::FORGET_AFTER_MS;` (the only import from `new_assignments`).
- `#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)] pub struct SeenKey { pub repo: String, pub number: u64 }` — `repo` is the lowercase `owner/name` key. `impl SeenKey { pub fn new(repo: impl Into<String>, number: u64) -> Self; pub fn reference(&self) -> String /* format!("{}#{}", repo, number), the same text as GithubIssue::reference */ }`. `impl std::fmt::Debug for SeenKey` writes exactly `SeenKey(..)` (keys never reach a log).
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct StoredIssue { pub number: u64, pub first_seen_ms: i64, pub last_seen_ms: i64 }`.
- `#[derive(Debug, Clone, PartialEq, Eq)] pub struct RepoPlan { pub seed: bool, pub insert_first_seen_ms: i64, pub forget: Vec<u64>, pub fresh: Vec<(u64, i64)> }`.
- `pub fn plan_repo(seeded: bool, stored: &[StoredIssue], current: &[u64], mark: Option<i64>, now_ms: i64) -> RepoPlan`:
  - `seed = !seeded`;
  - `insert_first_seen_ms = if seed { 0 } else { now_ms }` (decision 8: `0` is at or below every mark, so a seeded key is never fresh);
  - `forget` = numbers of `stored` not in `current` with `now_ms - last_seen_ms >= FORGET_AFTER_MS`, in `stored` order (by `last_seen_ms` only, never `first_seen_ms`, so seeded rows stored at `0` are forgotten like any other);
  - `fresh` = empty when `seed` or `mark` is `None`; otherwise, for each number of `current` in input order, once per number: its first instant is the stored `first_seen_ms` if stored, else `insert_first_seen_ms`; include `(number, first)` iff `first > mark`.

Unit tests in `#[cfg(test)] mod tests` of `rules.rs` (invented data only):

- `seeding_round_is_silent_and_stores_at_instant_zero` — `seeded = false`, `stored = []`, `current = [1, 2]`, `now = 1000`, for each of mark `Some(500)`, `None` and `Some(2000)` (after now) → `seed`, `insert_first_seen_ms == 0`, `fresh` empty, `forget` empty.
- `without_a_mark_nothing_is_fresh` — `seeded = true`, `stored = []`, `current = [1, 2]`, mark `None`, `now = 1000` → `seed == false`, `fresh` empty, `insert_first_seen_ms == 1000`.
- `first_seen_after_the_mark_is_fresh_and_at_the_mark_is_not` — `seeded = true`, mark `Some(500)`, `now = 1000`, stored #1 `{ first 500, last 900 }` and #2 `{ first 501, last 900 }`, `current = [1, 2, 3]` → `fresh == [(2, 501), (3, 1000)]`, `forget` empty.
- `a_stored_issue_keeps_its_first_instant` — `seeded = true`, mark `Some(50)`, `now = 1000`, stored #1 `{ first 100, last 900 }`, `current = [1]` → `fresh == [(1, 100)]` (not `(1, 1000)`), `forget` empty.
- `absent_issue_is_forgotten_after_thirty_days_only` — `seeded = true`, mark `None`, `now = 1_800_000_000_000`, stored #1 `{ first 0, last now - FORGET_AFTER_MS }`, #2 `{ first 0, last now - FORGET_AFTER_MS + 1 }`, #3 `{ first 0, last now - 2 * FORGET_AFTER_MS }`, `current = [3]` → `forget == [1]` (#2 is one millisecond short; #3 is present, so never forgotten; a `first_seen_ms` of `0` plays no part), `fresh` empty.
- `repeated_numbers_are_fresh_once_in_input_order` — `seeded = true`, `stored = []`, mark `Some(0)`, `now = 1000`, `current = [3, 2, 3]` → `fresh == [(3, 1000), (2, 1000)]`.
- `seen_key_debug_is_opaque_and_reference_is_key_hash_number` — `format!("{:?}", SeenKey::new("octo-test/alpha", 7)) == "SeenKey(..)"`, `reference() == "octo-test/alpha#7"`.

#### 5. Migration tests

**File**: `src-tauri/tests/github_seen.rs` (new)

**Intent**: Store-level tests with the production migrations, in the style of `tests/hidden_items.rs` (`mod common; use common::test_store_dir;`, a `today()` helper, a `scalar` helper). Invented data only (`octo-test/alpha`).

**Contract** — tests:

- `v15_database_upgrades_to_the_github_seen_tables_and_keeps_its_data` — `pos = MIGRATIONS.iter().position(|m| m.sql.contains("CREATE TABLE github_seen_repos"))`; assert `MIGRATIONS[pos].version == 16`; open `&MIGRATIONS[..pos]`, assert `PRAGMA user_version == 15`, insert a `days_off` row (`INSERT INTO days_off(day, created_at_ms) VALUES ('2026-11-12', 1000)`); reopen with all migrations; `user_version == MIGRATIONS.len()`; `days_off` count 1; the three new tables are empty.
- `github_seen_tables_refuse_invalid_rows` — each of these inserts fails with `StoreError::Sql { .. }`: repo `''`; repo `'Octo-Test/Alpha'` (not lowercase); issue with `issue_number = 0`; issue whose repo has no `github_seen_repos` row (FK); mark with `id = 2`. A valid repo row, issue row and mark row are accepted.
- `github_seen_tables_hold_only_keys_and_instants` — `PRAGMA table_info` column names are exactly `[repo, seeded_at_ms]`, `[repo, issue_number, first_seen_ms, last_seen_ms]` and `[id, seen_until_ms]`.

### Success Criteria:

#### Automated Verification:

- Rule unit tests pass: `cargo test --lib github_seen` (in `src-tauri/`)
- Migration tests pass: `cargo test --test github_seen` (in `src-tauri/`)
- Schema validation still passes: `cargo test --lib store::schema`
- Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

---

## Phase 2: Seen-set store operations

### Overview

Adds the one-write round, the mark save and the round input builder, with integration and crash tests. Still not wired into the refresh.

### Changes Required:

#### 1. Store operations

**File**: `src-tauri/src/github_seen/mod.rs`

**Intent**: The durable side of the rule: record one successful round, save the mark, read the mark, and turn fetched results into round input.

**Contract**:

- `#[derive(Clone, PartialEq, Eq)] pub struct RepoRound { pub repo: String, pub numbers: Vec<u64> }` — `repo` is the lowercase key. `Debug` is implemented by hand and writes exactly `RepoRound(..)` (no repo in logs); do not derive it.
- `pub fn round_input(results: &[crate::github::fetch::RepoResult]) -> Vec<RepoRound>` — one entry per result whose `outcome` is `Ok`, in input order, `repo = result.repo.key()`, `numbers` = the issue numbers in list order, skipping any number outside `1..=i64::MAX` (cannot be stored). A failed repo (`Err`, `Truncated` included) is left out.
- `pub fn record_round(store: &mut Store, loaded: &[RepoRound], active: &[String], now_ms: i64) -> Result<Vec<(SeenKey, i64)>, GithubSeenError>` — one `store.write::<_, GithubSeenError>(|tx| ..)`:
  1. `mark: Option<i64>` = `SELECT seen_until_ms FROM github_view_mark WHERE id = 1` (`OptionalExtension::optional`).
  2. `seeded: HashSet<String>` = `SELECT repo FROM github_seen_repos`.
  3. For each `RepoRound` in `loaded`: read `SELECT issue_number, first_seen_ms, last_seen_ms FROM github_seen_issues WHERE repo = ?1` into `StoredIssue`s (a number `<= 0` → `Err(GithubSeenError::Store(StoreError::Damaged { reason: "github seen record is unreadable".into() }))`); `plan = plan_repo(seeded.contains(&repo), &stored, &numbers, mark, now_ms)`; if `plan.seed` insert `INSERT INTO github_seen_repos (repo, seeded_at_ms) VALUES (?1, ?2)` (before the issue rows: FK); upsert each number with `INSERT INTO github_seen_issues (repo, issue_number, first_seen_ms, last_seen_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT (repo, issue_number) DO UPDATE SET last_seen_ms = excluded.last_seen_ms` (`?3 = plan.insert_first_seen_ms`, `?4 = now_ms`); delete each `plan.forget` number (`DELETE FROM github_seen_issues WHERE repo = ?1 AND issue_number = ?2`); append `plan.fresh` as `(SeenKey::new(repo, n), first)`.
  4. Prune: for each repo in `seeded` that is not in `active`, `DELETE FROM github_seen_issues WHERE repo = ?1` then `DELETE FROM github_seen_repos WHERE repo = ?1`.
  5. Return the fresh keys (repos in `loaded` order, numbers in list order).
- `pub fn mark_view_seen(store: &mut Store, seen_until_ms: i64) -> Result<(), GithubSeenError>` — `INSERT INTO github_view_mark (id, seen_until_ms) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET seen_until_ms = max(seen_until_ms, excluded.seen_until_ms)` in one write. Never moves back.
- `pub fn view_mark(store: &Store) -> Result<Option<i64>, GithubSeenError>` — `store.read` of the mark (used by tests).
- Extend the re-exports: `pub use rules::FORGET_AFTER_MS;`.

Unit test in `#[cfg(test)] mod tests` of `mod.rs`:

- `round_input_takes_only_ok_repos_by_lowercase_key` — results for `Octo-Test/Alpha` (Ok, issues 5 and 3), `octo-test/beta` (`Err(GithubError::Truncated)`), `octo-test/gamma` (Ok, empty) → `[RepoRound { repo: "octo-test/alpha", numbers: [5, 3] }, RepoRound { repo: "octo-test/gamma", numbers: [] }]`. Build `GithubIssue` literals in the test (all nine fields, as in `github_refresh.rs` `gh_issue`).

#### 2. Integration tests

**File**: `src-tauri/tests/github_seen.rs` (append)

**Intent**: The rule end to end against a real store (`Store::open(&dir, schema::MIGRATIONS, today())`). Constants: `NOW: i64 = 1_800_000_000_000`, `DAY: i64 = 86_400_000`, repos `"octo-test/alpha"`, `"octo-test/beta"`. A helper `fn round(repo: &str, numbers: &[u64]) -> RepoRound`.

**Contract** — tests:

- `first_round_seeds_silently_even_with_a_mark` — `mark_view_seen(NOW - DAY)`; record alpha [1, 2] at NOW → fresh empty; the marker row exists; both issue rows have `first_seen_ms = 0` and `last_seen_ms = NOW`; next round alpha [1, 2] at NOW + 1000 → fresh empty (baseline stored at `0`).
- `a_repo_seeded_without_a_mark_is_never_fresh_under_an_older_mark` — no mark; record alpha [1] at NOW (seeds alpha); record alpha [1] and beta [5, 6] at NOW + 1000 with both active (seeds beta, still no mark) → fresh empty; `mark_view_seen(NOW)` (a mark older than beta's seeding round, as saved by a visit that ended between `record_seen` and the gate); record alpha [1] and beta [5, 6] at NOW + 2000 → fresh empty (beta's rows are at `0`, not `NOW + 1000`).
- `no_mark_means_nothing_is_fresh` — seed alpha [1] at NOW; record alpha [1, 2] at NOW + 1000 → fresh empty.
- `an_issue_first_seen_after_the_mark_is_fresh_until_the_mark_passes_it` — seed alpha [1] at NOW; `mark_view_seen(NOW)`; record alpha [1, 2] at NOW + 1000 → fresh `[(alpha#2, NOW + 1000)]`; again at NOW + 2000 → same (first instant kept); `mark_view_seen(NOW + 2000)`; again at NOW + 3000 → empty.
- `a_repo_added_later_seeds_silently_under_an_existing_mark` — seed alpha, `mark_view_seen(NOW)`; record alpha and beta [7] at NOW + 1000 (beta new) → fresh empty; record again at NOW + 2000 → still empty.
- `a_failed_repo_is_neither_recorded_nor_forgotten` — seed alpha [1] and beta [4] at NOW; record only alpha (beta not in `loaded`, but in `active`) at NOW + 31 * DAY → beta's row 4 still exists with its old `last_seen_ms`.
- `an_absent_issue_is_forgotten_after_30_days_and_is_fresh_again` — seed alpha [1, 2] at NOW, `mark_view_seen(NOW)`; record alpha [1] at NOW + 31 * DAY → row 2 deleted; record alpha [1, 2] at NOW + 32 * DAY → fresh `[(alpha#2, NOW + 32 * DAY)]`.
- `an_issue_back_within_30_days_keeps_its_first_instant` — seed alpha [1, 2] at NOW, mark NOW; alpha [1] at NOW + 10 * DAY; alpha [1, 2] at NOW + 20 * DAY → fresh empty.
- `a_repo_removed_from_settings_is_pruned_and_seeds_silently_when_added_again` — seed alpha and beta; record alpha only with `active = [alpha]` → no `github_seen_repos` or `github_seen_issues` row for beta; mark NOW + 1000; record alpha and beta [9] with both active at NOW + 2000 → fresh empty (beta seeded again).
- `the_mark_never_moves_back` — `mark_view_seen(500)`, `mark_view_seen(400)` → `view_mark == Some(500)`; `mark_view_seen(600)` → `Some(600)`.
- `the_seen_set_survives_a_reopen` — seed and add issue via one `Store`, drop it, then a `StoreHandle::new(Some(dir)).with_store(today(), ..)` round sees the stored first instants (fresh as expected).

#### 3. Crash test

**File**: `src-tauri/tests/github_seen_crash.rs` (new)

**Intent**: Killing the writer mid-round never leaves a partial round or a marker without its rows. Copy the structure of `tests/new_assignments_crash.rs` exactly, with one repo `"octo-test/crash"`, no mark, `STEP_MS = 11 days`, `WIDTH = 4`, round `n` = issue numbers `n..n + WIDTH`, `model_after(m)` computing `number -> (first, last)` with the 30-day forget (by `last` only), where the numbers of round 1 (the seeding round, no mark) have `first = 0` and a number first seen in round `k > 1` has `first = now_of(k)`, the writer resuming after `MAX(last_seen_ms)` of `github_seen_issues`, and the parent asserting: `quick_check == "ok"`, marker present iff `committed > 0`, `committed ∈ {last, last + 1}` when `last > 0`, `committed` never goes back, and the rows equal `model_after(committed)`; plus `previous > 4` at the end.

**Contract** — tests `writer_process` (`#[ignore = "writer process for crash_never_leaves_a_partial_round"]`) and `crash_never_leaves_a_partial_round`; the writer calls `record_round(&mut store, &[RepoRound { repo, numbers }], &[repo], now_of(n))`.

### Success Criteria:

#### Automated Verification:

- Store operation unit test passes: `cargo test --lib github_seen`
- Integration tests pass: `cargo test --test github_seen`
- Crash test passes: `cargo test --test github_seen_crash`
- Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Seeding-race test passes: `cargo test --test github_seen a_repo_seeded_without_a_mark_is_never_fresh_under_an_older_mark`

---

**Deviation:** the reopen test (`the_seen_set_survives_a_reopen`) opens the store with `Store::open` instead of `StoreHandle::with_store`; it exercises the same store file and needs no handle layer. Unit tests live inline in `github_seen/mod.rs` (`mod tests`), not in a separate file.

---

## Phase 3: Snapshot contract `newReferences`

### Overview

The snapshot gains the list of NEW references and its TS mirror; a pure helper computes it from a shaped list. Still always empty at runtime.

### Changes Required:

#### 1. Snapshot field

**File**: `src-tauri/src/github/mod.rs`

**Intent**: Add the NEW flags to `GithubSnapshot` so the view can mark rows without a field on `GithubIssue` or `RepoView`.

**Contract**: new field `pub new_references: Vec<String>` (doc: "References of visible issues that are new since the last visit to the GitHub view, in list order; never a hidden issue. Set by the refresh state on every path that shapes the list."), placed after `next_refresh_at_ms`. `GithubSnapshot::starting()` sets `Vec::new()`. In `next_snapshot` the `next` literal sets `new_references: Vec::new()`; the `keeps` branch also sets `next.new_references = prev.new_references.clone()`. Serde gives `newReferences`.

Tests in `github/mod.rs` `mod tests`:

- `new_references_reset_on_success_and_carry_over_when_the_list_is_kept` — from `prev_with_list()` with `prev.new_references = vec!["octo-test/alpha#1".into()]`: an `Ok` round → empty; `Err(Unavailable)` with the same config → carried; `Err(NotConfigured)` → empty.
- `snapshot_serializes_new_references_in_camel_case` — `serde_json::to_value(GithubSnapshot::starting())["newReferences"]` is `[]`.

Also extend `starting_snapshot_is_empty` with `assert!(s.new_references.is_empty());`.

#### 2. Visible NEW references

**File**: `src-tauri/src/github_seen/mod.rs`

**Intent**: One pure helper every shaping path uses, so hidden issues and failed repos are never flagged.

**Contract**: `pub fn visible_new(repos: &[crate::github::RepoView], fresh: &HashSet<SeenKey>) -> Vec<String>` — the `reference` of each issue in `repo.issues` (visible only, never `repo.hidden`) of each repo with `error.is_none()`, in list order, when `SeenKey::reference()` of some key in `fresh` equals it.

Unit test: `visible_new_lists_fresh_visible_issues_of_loaded_repos_in_list_order` — alpha visible [#2, #1], hidden [#3]; beta with an error; fresh = {alpha#1, alpha#2, alpha#3, beta#1} → `["octo-test/alpha#2", "octo-test/alpha#1"]`.

#### 3. TS mirror

**File**: `src/github/types.ts`

**Intent**: Mirror the new field in the same commit.

**Contract**: in `GithubSnapshot` add `/** References of visible issues new since the last visit to this view, in list order. */ newReferences: string[];` after `nextRefreshAtMs`.

### Success Criteria:

#### Automated Verification:

- Snapshot tests pass: `cargo test --lib github::tests`
- Helper test passes: `cargo test --lib github_seen`
- Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

---

## Phase 4: Refresh state, round wiring and `mark_github_seen`

### Overview

The GitHub refresh records the seen set each successful round, keeps the flags on every path that shapes the list, and a new command ends a visit.

### Changes Required:

#### 1. Refresh state

**File**: `src-tauri/src/github_refresh.rs`

**Intent**: Mirror GitLab's NEW state (`gitlab_refresh.rs` `kept_new`, `shown_until`, `last_mark`, `accept_round`, `mark_seen`) without an account (GitHub has one global mark).

**Contract**:

- Imports (added to the existing `use` block; `StoreHandle`, `GithubConfig`, `GithubRepo`, `RepoResult`, `AppHandle`, `Manager`, `Local` and `now_ms` are already imported): `use std::collections::HashSet;`, `use std::sync::atomic::{AtomicBool, Ordering};`, `use crate::github_seen::{self, GithubSeenError, SeenKey};`, `RepoView` added to the existing `use crate::github::{..}` list, and `StoreError` added to `use crate::store::StoreHandle;` (→ `use crate::store::{StoreError, StoreHandle};`). Keep `rustfmt` order; nothing else in the block changes.
- Module doc: replace the lock-order sentence with "Lock order: `hidden_gate`, then `kept_config`, then `kept_full`, then `kept_new`, then `shown_until`, then `last_mark`, then the snapshot lock (in `SourceCell`). The store lock is taken inside the gate but never while a `kept_*`, `shown_until` or `last_mark` lock is held." and add a paragraph "NEW flags (S-03): a round records its seen set before the gate and brings the fresh keys under it; `kept_new` holds them so every path that shapes the list writes the same `new_references`. `shown_until` is the `now_ms` of the last round stored in the snapshot whose seen set was written: `mark_seen` saves exactly that, never the clock. A failed write leaves both as they were. `last_mark` is the last mark saved this session; a round that read an older mark drops the keys it covers. No toast and no event-log entry for GitHub."
- New fields on `GithubRefreshState`: `kept_new: Mutex<HashSet<SeenKey>>`, `shown_until: Mutex<Option<i64>>`, `last_mark: Mutex<Option<i64>>` (doc comments as in GitLab, without the account); initialised empty/`None` in `new()`.
- `fn accept_round(&self, now_ms: i64, fresh: Option<&[(SeenKey, i64)]>) -> HashSet<SeenKey>` — under the caller's gate; locks `kept_new`, `shown_until`, `last_mark` in that order. `fresh == None` (write failed): return `kept_new.clone()`, change nothing. Otherwise `kept_new` = keys of `fresh` whose `first_seen_ms > last_mark` (all of them when `last_mark` is `None`), `shown_until = Some(now_ms)`, return the new `kept_new`.
- `fn settle_new(&self, list_dropped: bool, repos: &[RepoView]) -> Vec<String>` — locks `kept_new` then `shown_until`; when `list_dropped`, clears `kept_new` and sets `shown_until = None`; returns `github_seen::visible_new(repos, &kept_new)`.
- `pub(crate) fn mark_seen(&self, save: impl FnOnce(i64) -> Result<(), GithubSeenError>) -> Result<bool, GithubSeenError>` — takes `hidden_gate`; clones `shown_until` and releases it; `None` → `Ok(false)`; `save(until)?` (an error changes nothing); then clears `kept_new`, sets `last_mark = Some(until)`, clears `snap.new_references`, `snap.seq += 1`, returns `Ok(true)`. The caller publishes.
- `act_on_issue`: after `partition_github`, clone `kept_new` (lock and release), compute `github_seen::visible_new(&repos, &fresh)` before moving `repos` into the snapshot, and set `snap.new_references` to it in the same snapshot write. Update its doc: the fields that change are `repos`, `new_references`, `hidden_error`, `hidden_check_failed` and `seq`.

#### 2. Round wiring

**File**: `src-tauri/src/github_refresh.rs`

**Intent**: Record the seen set once per successful round and carry the flags into the stored snapshot.

**Contract**:

- `static GITHUB_SEEN_WARNED: AtomicBool = AtomicBool::new(false);`
- `fn record_seen(app: &AppHandle, cfg: &GithubConfig, results: &[RepoResult], now_ms: i64) -> Option<Vec<(SeenKey, i64)>>` — `loaded = github_seen::round_input(results)`, `active = cfg.repos.iter().map(GithubRepo::key).collect::<Vec<_>>()`, `app.state::<StoreHandle>().with_store(Local::now().date_naive(), |s| github_seen::record_round(s, &loaded, &active, now_ms))`; `Ok(fresh) → Some(fresh)`; on `Err(e)`: `Busy` → `log::debug!("github: seen set failed: {}", e.kind())`; otherwise the first time per session (`GITHUB_SEEN_WARNED.swap(true, Ordering::Relaxed)` was false) `log::warn!` with the same text, later `log::debug!`; return `None`. Holds no lock of `GithubRefreshState`.
- In `refresh_round`, after the `checks` block and before `let gate = ...`: `let round_ms = now_ms() as i64; let seen = match (&config, &fetched) { (Some(cfg), Ok(results)) => Some(record_seen(app, cfg, results, round_ms)), _ => None };`
- Right after the gate is taken: `if let Some(fresh) = &seen { state.accept_round(round_ms, fresh.as_deref()); }`
- Before the `kept_full` block: `let list_dropped = results.is_none() && next_kept.is_none();`. After the `kept_full` block (before `next.refresh_setting_invalid = ...`): `next.new_references = state.settle_new(list_dropped, &next.repos);`

#### 3. Command and registration

**File**: `src-tauri/src/github_refresh.rs`

**Intent**: End a visit from the UI: save the mark of the last shown round and publish the cleared flags. Writes the app store only; never calls `gh`.

**Contract**: `#[tauri::command] pub async fn mark_github_seen(app: AppHandle) -> Result<(), GithubSeenError>` — same body shape as `mark_gitlab_seen` (`crate::blocking`, `state.mark_seen(|until| store.with_store(today, |s| github_seen::mark_view_seen(s, until)))`); `Ok(true)` → `publish(&app, &app.state::<GithubRefreshState>())`; `Err(Store(Busy))` → `log::debug!("github: mark seen failed: {}", e.kind())`; other errors → `log::warn!` with the same text; returns `result.map(|_| ())`.

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the command.

**Contract**: insert `github_refresh::mark_github_seen,` on the line right after `github_refresh::refresh_github,` in `generate_handler!`. No other line moves.

#### 4. TS api and error mirror

**File**: `src/github/api.ts`

**Contract**: `/** Ends a visit to the GitHub view: the "new" flags go out and the seen mark moves. A failure rejects with `GithubSeenError`; Rust has already logged it. */ export function markGithubSeen(): Promise<void> { return invoke<void>("mark_github_seen"); }` placed after `refreshGithub`.

**File**: `src/github/types.ts`

**Contract**: `/** Mirrors github_seen::GithubSeenError. */ export type GithubSeenError = { kind: "Store"; detail: StoreError } | { kind: "Background" };` with `import type { StoreError } from "../components/storeError";`.

#### 5. Unit tests

**File**: `src-tauri/src/github_refresh.rs` (`mod tests`, reuse `state_with_list`, `gh_issue`, `hidden_record`)

**Contract** — tests:

- `accept_round_keeps_fresh_keys_and_records_shown_until` — fresh `[(alpha#1, 500), (alpha#2, 900)]`, no `last_mark` → returns both; `kept_new` has both; `shown_until == Some(now)`.
- `accept_round_drops_keys_the_last_mark_covers` — `last_mark = Some(600)` → only alpha#2.
- `a_key_first_seen_after_the_saved_mark_stays_new` — the other half of the visit-ends-mid-round guard (GitLab: `an_item_from_a_round_stored_after_shown_until_stays_new`): `accept_round(100, Some(&[]))`; `mark_seen(|_| Ok(()))` → `Ok(true)` and `last_mark == Some(100)`; `accept_round(200, Some(&[(alpha#3, 200)]))` returns `{alpha#3}`, `kept_new == {alpha#3}` and `shown_until == Some(200)`.
- `a_failed_seen_set_write_keeps_the_flags_and_shown_until` — after a successful accept, `accept_round(later, None)` returns the previous set and leaves `shown_until` at the earlier instant.
- `settle_new_reapplies_kept_flags_and_clears_them_when_the_list_is_dropped` — with `kept_new = {alpha#1}` and the list of `state_with_list()`, `settle_new(false, &repos) == ["octo-test/alpha#1"]`; `settle_new(true, &[])` is empty and `shown_until` and `kept_new` are cleared.
- `mark_seen_without_shown_until_saves_nothing` — `save` is never called, returns `Ok(false)`, snapshot unchanged.
- `mark_seen_saves_shown_until_clears_flags_and_bumps_seq` — `shown_until = Some(1234)`, snapshot `new_references = ["octo-test/alpha#1"]` → `save` gets 1234, `Ok(true)`, `new_references` empty, `kept_new` empty, `last_mark == Some(1234)`, `seq + 1`.
- `a_failed_mark_save_changes_nothing` — `save` returns `Err(GithubSeenError::Store(StoreError::Busy))` → error returned; `kept_new`, `last_mark`, `new_references`, `seq` unchanged.
- `act_on_issue_keeps_new_flags_of_visible_issues_only` — `kept_new = {alpha#1, alpha#2}`; hide #2 → `new_references == ["octo-test/alpha#1"]`; show again (`act` returns `vec![]`) → `["octo-test/alpha#1", "octo-test/alpha#2"]`.

### Success Criteria:

#### Automated Verification:

- Refresh-state tests pass: `cargo test --lib github_refresh`
- Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- No new CLI call: `git diff --stat develop -- src-tauri/src/cli src-tauri/src/bin src-tauri/src/hidden_issues src-tauri/src/new_assignments` is empty
- Saved-mark test passes: `cargo test --lib github_refresh a_key_first_seen_after_the_saved_mark_stays_new`

---

## Phase 5: UI — `Nowe` flag and end-of-visit hook

### Overview

The GitHub view renders `Nowe` on NEW rows and the app reports the end of a GitHub visit.

### Changes Required:

#### 1. End-of-visit hook

**File**: `src/github/useGithubSeenMark.ts` (new)

**Intent**: A twin of `src/gitlab/useGitlabSeenMark.ts` for `"github"`, calling `markGithubSeen()` from `./api` (errors swallowed: Rust logs them; the only effect of a failure is that the flags show again). It takes the view id only and never reads the work timer.

**Contract**: `export function useGithubSeenMark(view: string): void` — a `useRef(view)` previous-view ref; an effect on `[view]` that calls the mark when the previous view was `"github"` and the current is not; an effect on `[view]` that, while `view === "github"`, listens for `"main-hidden"` and calls the mark, with the same `cancelled`/`unlisten` cleanup as the GitLab hook. Doc comment as in the GitLab hook, with "GitHub".

#### 2. App wiring

**File**: `src/App.tsx`

**Intent**: Report the end of a GitHub visit next to the GitLab one.

**Contract**: add `import { useGithubSeenMark } from "./github/useGithubSeenMark";` (after the `GithubView` import) and `useGithubSeenMark(view);` on the line right after `useGitlabSeenMark(view);`. Nothing else in `App.tsx` changes.

#### 3. Flag rendering

**File**: `src/github/GithubView.tsx`

**Intent**: Show the blue `Nowe` flag (as `src/gitlab/GitlabView.tsx` does with `<Flag tone="new">Nowe</Flag>`) on the visible rows whose reference is in `snapshot.newReferences`.

**Contract**:

- `IssueRow` gets `isNew = false` (prop `isNew?: boolean`); inside `extra`, render `{isNew && <Flag tone="new">Nowe</Flag>}` before the `changed` flag span.
- `RepoGroup` gets a prop `newRefs: ReadonlySet<string>` and passes `isNew={newRefs.has(issue.reference)}` to the visible `repo.issues` rows only; hidden rows get no `isNew`.
- `GithubView` computes `const newRefs = new Set(snapshot.newReferences);` after the snapshot destructuring and passes it to every `RepoGroup`. The `showHidden` state stays as it is.

### Success Criteria:

#### Automated Verification:

- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- In a debug build (`npm run tauri build -- --debug --no-bundle`, own data dir backed up first), the first visit to "GitHub" shows no `Nowe` flag
- After leaving the GitHub view and assigning yourself a new issue in a configured test repo, the next round shows `Nowe` on that issue only; leaving the view and returning clears it
- Hiding the window (close button) while on the GitHub view also clears the flag on the next visit
- A NEW issue moved under "Pokaż ukryte" shows no flag there; unhiding it during the same visit shows `Nowe` again
- No Windows notification appears for a new GitHub issue, and the DevTools console shows no CSP violation and no "IPC custom protocol failed"
- The work-time counter in the header keeps running unchanged while switching between GitLab and GitHub

---

## Phase 6: AGENTS.md app-data list

### Overview

Documents the new records in the Hard Rules app-data list (user decision), as a minimal insertion.

### Changes Required:

#### 1. App-data list

**File**: `AGENTS.md`

**Intent**: List the GitHub seen keys, their seeding markers and the GitHub view's mark, in the same style as the GitLab entries; no title, URL or login.

**Contract**: in the single Hard Rules bullet that lists the app data dir records, insert right after the text `the GitLab view's seen-until mark (opaque account key, one instant), ` the text `GitHub seen-issue keys (lowercase `owner/name` repo key, issue number and two instants) with their per-repo seeding markers (the repo key and one instant), the GitHub view's seen-until mark (one instant), `. No other line of AGENTS.md changes.

### Success Criteria:

#### Automated Verification:

- The inserted phrase is present once: `grep -c "GitHub seen-issue keys" AGENTS.md` prints `1`
- AGENTS.md diff touches one line only: before the commit, `git diff HEAD --numstat -- AGENTS.md` shows `1	1` (staged or not)
- All gates pass: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

---

## Testing Strategy

### Unit Tests:

- `github_seen::rules`: seeding at instant `0`, no mark, strict `>` at the mark, kept first instants, 30-day forget boundary, duplicate numbers, opaque `Debug`.
- `github_seen`: `round_input` (only `Ok` repos, lowercase keys), `visible_new` (visible issues of loaded repos only, list order).
- `github::tests`: `new_references` reset/carry and serialization.
- `github_refresh::tests`: `accept_round` (including a key first seen after the saved mark), `settle_new`, `mark_seen` (with and without `shown_until`, failing save), `act_on_issue` flags.

### Integration Tests:

- `tests/github_seen.rs`: v15 → v16 upgrade, CHECK/FK refusals, column lists, seeding under a mark, new repo under a mark, repo seeded without a mark then an older mark, failed repo, forget and return, prune and re-add, mark never moves back, reopen.
- `tests/github_seen_crash.rs`: kill harness, whole rounds only.

### Manual Testing Steps:

1. Back up the own data dir, build `npm run tauri build -- --debug --no-bundle`, start the built exe.
2. Open "GitHub": no `Nowe`. Switch to another view.
3. Assign yourself an invented test issue in a configured test repo; press "Odśwież" in the GitHub view: only that issue shows `Nowe`.
4. Leave and return: the flag is gone. Repeat with the close button instead of a view switch.
5. Hide a `Nowe` issue, open "Pokaż ukryte": no flag on the hidden row; unhide: the flag is back.
6. Check that no Windows notification came and DevTools shows no CSP or IPC protocol errors.

## Performance Considerations

One extra `Store::write` per GitHub round (at most 50 repos × 500 issues, the existing caps): a few hundred small upserts in one transaction, the same order of cost as the GitLab seen set. The snapshot grows by a list of references of NEW issues only.

## Migration Notes

- v16 is append-only and creates new empty tables; no existing data moves. The first round after the upgrade seeds every configured repo silently.
- Merge risks with S-07 (`gitlab-hide-merge-requests`): `AGENTS.md` Hard Rules app-data bullet (one long line, both slices insert; keep both insertions); `src-tauri/src/lib.rs` (`pub mod github_seen;` after `pub mod github_refresh;`, `github_refresh::mark_github_seen,` after `github_refresh::refresh_github,`; S-07 adds its own lines elsewhere in the handler list); `src-tauri/src/store/schema.rs` (only S-03 appends v16). S-03 does not edit `cli/`, `bin/fake_cli.rs`, `hidden_issues/` or `new_assignments/`.
- Rollback: the change is additive; a database already at v16 is refused by an older build as `NewerSchema` (the store's rule), so restoring an older build needs a v15 backup from `backups/`.

## References

- Research: `context/changes/github-new-and-notify/research.md`
- GitLab NEW flag precedent: `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan-brief.md`
- Seen set and seeding precedent: `context/archive/2026-11-23-new-assignment-notify/plan-brief.md`
- GitHub hiding precedent: `context/archive/2026-11-27-github-hide-resurface/`
- Review-set baseline precedent: `context/archive/2026-11-27-gitlab-review-requests/`
- Code: `src-tauri/src/new_assignments/mod.rs` (`record_round`, `mark_view_seen`), `src-tauri/src/new_assignments/rules.rs` (`fresh_since`, `baseline_first_seen`), `src-tauri/src/gitlab_refresh.rs` (`accept_round`, `mark_seen`, `mark_gitlab_seen`, `record_seen`), `src-tauri/src/github_refresh.rs` (`refresh_round`, `act_on_issue`), `src-tauri/src/github/mod.rs` (`GithubSnapshot`, `next_snapshot`), `src/gitlab/useGitlabSeenMark.ts`, `src/App.tsx`, `src/github/GithubView.tsx`
- Test patterns: `src-tauri/tests/hidden_items.rs`, `src-tauri/tests/new_assignments.rs`, `src-tauri/tests/new_assignments_crash.rs`, `src-tauri/tests/common/crash.rs`
- Roadmap: `context/foundation/roadmap.md` (S-03, MS-30; read-only for this slice)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Migration v16, rules and error

#### Automated

- [x] 1.1 Rule unit tests pass: `cargo test --lib github_seen` (in `src-tauri/`)
- [x] 1.2 Migration tests pass: `cargo test --test github_seen` (in `src-tauri/`)
- [x] 1.3 Schema validation still passes: `cargo test --lib store::schema`
- [x] 1.4 Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [x] 1.5 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

### Phase 2: Seen-set store operations

#### Automated

- [x] 2.1 Store operation unit test passes: `cargo test --lib github_seen`
- [x] 2.2 Integration tests pass: `cargo test --test github_seen`
- [x] 2.3 Crash test passes: `cargo test --test github_seen_crash`
- [x] 2.4 Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [x] 2.5 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [x] 2.6 Seeding-race test passes: `cargo test --test github_seen a_repo_seeded_without_a_mark_is_never_fresh_under_an_older_mark`

### Phase 3: Snapshot contract `newReferences`

#### Automated

- [x] 3.1 Snapshot tests pass: `cargo test --lib github::tests`
- [x] 3.2 Helper test passes: `cargo test --lib github_seen`
- [x] 3.3 Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [x] 3.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

### Phase 4: Refresh state, round wiring and `mark_github_seen`

#### Automated

- [x] 4.1 Refresh-state tests pass: `cargo test --lib github_refresh`
- [x] 4.2 Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- [x] 4.3 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [x] 4.4 No new CLI call: `git diff --stat develop -- src-tauri/src/cli src-tauri/src/bin src-tauri/src/hidden_issues src-tauri/src/new_assignments` is empty
- [x] 4.5 Saved-mark test passes: `cargo test --lib github_refresh a_key_first_seen_after_the_saved_mark_stays_new`

### Phase 5: UI — `Nowe` flag and end-of-visit hook

#### Automated

- [x] 5.1 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`
- [x] 5.2 Full Rust gates pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual

- [ ] 5.3 In a debug build (`npm run tauri build -- --debug --no-bundle`, own data dir backed up first), the first visit to "GitHub" shows no `Nowe` flag
- [ ] 5.4 After leaving the GitHub view and assigning yourself a new issue in a configured test repo, the next round shows `Nowe` on that issue only; leaving the view and returning clears it
- [ ] 5.5 Hiding the window (close button) while on the GitHub view also clears the flag on the next visit
- [ ] 5.6 A NEW issue moved under "Pokaż ukryte" shows no flag there; unhiding it during the same visit shows `Nowe` again
- [ ] 5.7 No Windows notification appears for a new GitHub issue, and the DevTools console shows no CSP violation and no "IPC custom protocol failed"
- [ ] 5.8 The work-time counter in the header keeps running unchanged while switching between GitLab and GitHub

### Phase 6: AGENTS.md app-data list

#### Automated

- [ ] 6.1 The inserted phrase is present once: `grep -c "GitHub seen-issue keys" AGENTS.md` prints `1`
- [ ] 6.2 AGENTS.md diff touches one line only: before the commit, `git diff HEAD --numstat -- AGENTS.md` shows `1	1` (staged or not)
- [ ] 6.3 All gates pass: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
