---
date: 2026-11-28T08:05:00+01:00
researcher: Claude (Sonnet 5.5, research sub-subagent)
git_commit: 76cd680178f4111aebfe07eea076b26768fd5b25
branch: slice/github-new-and-notify
repository: day-navigator
topic: "S-03 github-new-and-notify: NEW flag on GitHub items that appeared since the last visit to the GitHub view (MS-30, FR-005, FR-023)"
tags: [research, github, new-flag, seen-keys, view-mark, migration-v16, source-refresh]
status: complete
last_updated: 2026-11-28
last_updated_by: Claude (Sonnet 5.5, research sub-subagent)
---

# Research: NEW flag for GitHub items (S-03 `github-new-and-notify`)

**Date**: 2026-11-28T08:05:00+01:00
**Researcher**: Claude (Sonnet 5.5, research sub-subagent)
**Git Commit**: 76cd680178f4111aebfe07eea076b26768fd5b25 (develop after M-4 wave 2)
**Branch**: slice/github-new-and-notify
**Repository**: day-navigator

This research was written non-interactively. Every choice beyond the user's decisions is marked **(default chosen by agent)**.

## Research Question

How do the GitLab NEW flag and the GitLab view's seen-until mark work end to end (store, migrations, commands, TS, rendering, mark advance on visit, seeding), how do the seen-assignment keys and their seeding markers work (`new_assignments/`, opaque account key, `:reviews` suffix) and can they serve GitHub, how does the GitHub view get its data and identify items, how is a "visit to a view" detected in `App.tsx`, what must migration v16 contain, and where does the slice overlap with the parallel slice S-07?

## User decisions (recorded verbatim; not re-asked)

- NEW flag only on GitHub items that appeared since the last visit to the GitHub view. NO Windows notifications for GitHub (FR-004 notices stay GitLab-only).
- GitHub seen keys and the GitHub view's visit mark need a migration: this slice takes migration v16 (the wave's only migration; append to `src-tauri/src/store/schema.rs`, never edit a shipped one).
- Update the AGENTS.md app-data list for the new records (no titles, URLs, logins; opaque/minimal keys as for GitLab).
- Respect the AGENTS.md rule "never subscribe to the timer status a second time" and the existing GitHub "Pokaż ukryte" in-memory toggle; the first-ever visit must not mark everything NEW (seed like GitLab does). The plan decides details, marked "default chosen by agent".

## Summary

1. **GitLab NEW is a rule on stored instants, not a diff in the UI.** An item is NEW iff its `first_seen_ms` (stored when its key was first recorded for the account) is strictly after the account's seen-until mark (`fresh_since`, `src-tauri/src/new_assignments/rules.rs:90-99`). No mark means nothing is NEW (`rules.rs:91-93`). The seeding round (the account's first stored round) returns no fresh keys even if a mark exists (`new_assignments/mod.rs:288-294`).
2. **The mark advances on leaving the view, to the instant of the last shown round, never the wall clock.** `mark_seen` saves `shown_until` (the `now_ms` of the last round stored in the snapshot whose seen set was written) with `max()` so it never moves back (`gitlab_refresh.rs:243-267`, `new_assignments/mod.rs:143-157`). The UI calls it when the active view leaves `gitlab` or the window is hidden while `gitlab` is active (`src/gitlab/useGitlabSeenMark.ts:15-40`).
3. **Seen keys are GitLab-shaped and cannot hold GitHub items without a rebuild.** `seen_assignments.kind` is `CHECK IN ('issue','mergeRequest')` and `item_id` is GitLab's global numeric id under an FK to `assignment_accounts` (`store/schema.rs:130-146`); `ItemKey` is `(ItemKind, u64)` (`new_assignments/rules.rs:16-19`). GitHub identity is `(lowercase owner/name repo key, issue number)` (`github/mod.rs:23-30`, `hidden_issues/github_round.rs:27-28`). **Recommendation: GitHub gets its own tables in v16, not a generalisation of the GitLab ones** (reason: no table rebuild, no edit of `new_assignments/` which the S-07 brief lists as hands-off for S-07 and "S-03 may generalise", and the GitLab path keeps its tests unchanged).
4. **v16 shape (proposed, default chosen by agent):** `github_seen_repos(repo TEXT PK, seeded_at_ms)` (per-repo seeding marker), `github_seen_issues(repo, issue_number, first_seen_ms, last_seen_ms)` and `github_view_mark(id=1, seen_until_ms)`. No title, URL, login, reference text or hostname. See "Migration v16".
5. **GitHub has no account key and needs none.** The GitHub config is a list of repos (`cli/mod.rs:177-180`); the `gh` login is fetched only for hidden-issue comment checks (`github/fetch.rs:55-58`, `hidden_issues/github_round.rs:150`). The repo key is already the scope of `hidden_items` (`store/schema.rs:298-327`), so the same key is the scope of the seen keys. Seeding is per repo, so a repo added to settings later seeds silently instead of flagging every issue NEW.
6. **GitHub refresh has no seen-set code today.** `github_refresh.rs` keeps `kept_config`, `kept_full`, `hidden_gate` only (`github_refresh.rs:64-73`). The slice adds `kept_new`, `shown_until`, `last_mark` and a `mark_github_seen` command, mirroring the GitLab ones (`gitlab_refresh.rs:99-107,198-267,419-441`), and must re-apply the flags on every path that shapes the list, including `act_on_issue` (`github_refresh.rs:113-154`), because `partition_github` rebuilds each `GithubIssue` from the fetched one (`hidden_issues/github_round.rs:294-334`).
7. **Visit detection already exists as one hook taking only the view id.** `useGitlabSeenMark(view)` is called in `App.tsx:37` with the view state only; it does not use `WorkTimerProvider` (which wraps the header and views inside `App`'s return, `App.tsx:62-85`). A GitHub twin that takes `view` the same way satisfies "never subscribe to the timer status a second time" by construction.
8. **No new CLI call is needed.** The NEW rule uses only data already fetched (repo key, number). `cli/mod.rs` and `fake_cli.rs` stay untouched (S-07 owns them in this wave).
9. **Main risks:** a first-round race (flags set before the seen set is written), a failed repo or failed round marking absence (must not forget or re-seed), the `GithubIssue` struct literal sites (21 occurrences in 9 files) when `is_new` is added, the `AGENTS.md` and `lib.rs` merge overlap with S-07, and a failure mode where the baseline seeding is lost on a crash (solved by one `Store::write`, see below).

## Detailed Findings

### 1. The GitLab NEW flag, end to end

**Store.** Migration v7 creates `assignment_accounts(account PK, seeded_at_ms)` and `seen_assignments(account, kind, item_id, first_seen_ms, last_seen_ms, PK(account, kind, item_id))` with FK to the accounts table (`store/schema.rs:130-146`). Migration v9 creates `gitlab_view_marks(account PK, seen_until_ms)`, deliberately without an FK, "a mark can exist before the account's first round seeded it" (`store/schema.rs:209-221`). The account key is the SHA-256 hex of host and user (`gitlab/mod.rs:28-36`).

**Round.** `record_round` runs as one `Store::write` per successful round (`new_assignments/mod.rs:170-304`): it reads the seeded flag, the review marker, all rows of the account and the mark (`:178-223`), plans (`plan_round`, `rules.rs:130-155`), inserts the marker when seeding, upserts every current key (insert sets `first_seen_ms`, a conflict updates only `last_seen_ms`, `:243-262`), deletes keys absent for at least 30 days (`FORGET_AFTER_MS`, `rules.rs:10`, `:263-274`), and returns `RoundSeen { new, fresh }`. The seeding round returns an empty `fresh` (`:288-294`).

**Refresh state.** Four fields carry the NEW state on `GitlabRefreshState` (`gitlab_refresh.rs:99-107`):
- `kept_new`: the fresh keys, so every path that shapes the list sets the same flags;
- `shown_until`: `(account, now_ms)` of the last round stored in the snapshot whose seen-set write succeeded;
- `last_mark`: the last mark saved this session; a round that read an older mark drops the fresh keys it already covers (`accept_round`, `gitlab_refresh.rs:198-235`);
- the lock order is documented as gate, `kept_config`, `kept_full`, `kept_filter`, `kept_new`, `shown_until`, `last_mark`, snapshot (`gitlab_refresh.rs:11-17`).

A failed seen-set write passes `fresh = None`: `kept_new` and `shown_until` stay as they were, so leaving the view never marks unshown items as seen (`gitlab_refresh.rs:200-204,226-228`, `record_seen` at `:757-817`).

**Command.** `mark_gitlab_seen` runs `mark_seen` under the gate (`gitlab_refresh.rs:237-267`): reads `shown_until` (no shown round gives `Ok(false)`), saves the mark through `with_store(... mark_view_seen)` (`:419-441`), clears `kept_new`, remembers `last_mark`, clears `is_new` on the snapshot, bumps `seq`; the command then publishes. It is registered at `lib.rs:206`; the TS side is `markGitlabSeen` (`src/gitlab/api.ts:15-18`).

**Rendering.** `item.isNew && <Flag tone="new">Nowe</Flag>` (`src/gitlab/GitlabView.tsx:189`); `Flag` already has `tone="new"` (blue) (`src/components/Flag`, prop comment "new = blue (Nowe, a freshly assigned item)"). Hidden issues are never NEW (`mark_fresh`, `new_assignments/mod.rs:125-139`).

**Visit detection.** `useGitlabSeenMark(view)` marks when the previous view was `gitlab` and the current one is not, using a ref so a StrictMode double effect is idempotent (`useGitlabSeenMark.ts:15-24`; `src/main.tsx:8` uses `React.StrictMode`), and listens for `main-hidden` while `gitlab` is active (`:26-40`). `main-hidden` is emitted from the `CloseRequested` handler (`lib.rs:33-34,188-197`). The call sits in `App.tsx:37`. Quitting from the tray while the view is active saves no mark: the flags show again next time (design note in `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan-brief.md`, "Open Risks").

**Seeding so the first visit marks nothing.** Two independent guards: (a) no mark gives no fresh keys (`rules.rs:91-93`); (b) the seeding round of an account returns no fresh keys (`new_assignments/mod.rs:290-294`). The first mark is written the first time the user leaves the view after a successful round, and from then on only keys first seen later are NEW.

### 2. Seen-assignment keys, seeding markers, `:reviews`

- `ItemKey` is `(ItemKind, u64)` with an opaque `Debug` so ids never reach a log (`rules.rs:12-45`).
- The seeding marker is a row in `assignment_accounts`; it is never pruned so an account whose list was empty for a long time still notifies its next item (`schema.rs:124-129`, module doc `new_assignments/mod.rs:14-16`).
- The review set (S-06) has its own marker row `review_marker_key(account) = "{account}:reviews"` in the same table; "the account key is 64 hex characters, so the suffix cannot collide" and no `seen_assignments` row ever uses it (`new_assignments/mod.rs:85-91`, `:17-23`). `review_baseline` and `baseline_first_seen` take review-only keys in silently with `first_seen_ms = min(mark, now)` so `fresh_since` never flags them (`rules.rs:101-128`).
- **Reuse for GitHub, evaluated:**
  - Reuse the same tables: not possible without a rebuild: `kind` CHECK excludes GitHub, `item_id` is a GitLab global id, and the FK needs an account row. A GitHub row would also need the repo key as part of the primary key. A rebuild of `seen_assignments` would touch the most notification-critical table and force changes in `new_assignments/` (which S-07 must not touch, and which S-03 should avoid editing).
  - A derived marker such as `"{key}:github"` in `assignment_accounts` would work for the seeding flag but not for the keys, so two tables would still be needed.
  - **Chosen: separate GitHub tables and a separate tauri-free module (name proposed: `src-tauri/src/github_seen/`), with its own small rules** (default chosen by agent). The rules to copy are tiny (`fresh_since`, a forget rule, a seeding flag); a generic `fn fresh_since<K>` in `new_assignments/rules.rs` is possible but edits a file the GitLab path depends on. The plan may still prefer a generic, see Open Questions.

### 3. How the GitHub view gets its data

- **Fetch.** `fetch_assigned` runs one read-only `gh issue list` per configured repo; `Unavailable`, `LoggedOut`, `ToolMissing` end the round for the whole source, `Failed`, `ResponseInvalid`, `Truncated` belong to the repo (`github/fetch.rs:19-47`). Result: `Vec<RepoResult { repo, outcome }>`.
- **Identity.** `GithubIssue { repo (as in settings), number, reference = "<lowercase owner/name>#<n>", title, web_url, updated_at, labels, changed, hidden_at_ms }` (`github/mod.rs:22-40`). The scope key is `GithubRepo::key()` (lowercase `owner/name`, `cli/mod.rs:165-169`). `GithubConfig` is only `repos` (`cli/mod.rs:177-180`): no user, no host, so there is no account to hash.
- **Round.** `refresh_round` (`github_refresh.rs:249-330`): `run_fetch` -> `check_github_round` (processes, no lock) -> take `hidden_gate` -> `next_snapshot` (replaces `repos` on success; keeps `prev.repos` on `Unavailable` and error kinds only while the repo list is unchanged, `github/mod.rs:316-380`) -> `commit_github_round` partitions into visible/hidden (`:297-302`) -> update `kept_full`/`kept_config` -> reconcile "To do" -> store snapshot with `next_refresh_at_ms` -> release gate -> publish. The store is touched inside the gate, never under a `kept_*` lock (`github_refresh.rs:12-16`).
- **Skeleton.** `source_refresh.rs` provides `SourceCell` (snapshot, `publish`, `begin_round`, `start_thread`, `end_round`, `log_reconcile`) and `run_loop`; the GitHub module keeps fetch, rules and locks (`source_refresh.rs:31-214`, AGENTS.md "Project Structure"). **S-03 needs no skeleton change.**
- **Hide actions.** `act_on_issue` re-partitions `kept_full` from the store's records and overwrites only `repos`, `hidden_error`, `hidden_check_failed`, `seq` (`github_refresh.rs:113-154`). `partition_github` clones each fetched issue and sets `changed`/`hidden_at_ms` (`hidden_issues/github_round.rs:294-334`), so any new field on `GithubIssue` is reset to its fetched value on each re-partition unless the NEW flags are applied again afterwards (as GitLab does with `mark_fresh` after partition, `gitlab_refresh.rs:299-314`).
- **View.** `GithubView.tsx` mounts only while `view === "github"` (`App.tsx:75`), calls `useGithub()` with `refreshOnMount` true (`useGithub.ts:19-60`) and renders `IssueRow` with `extra` holding the "ukryte" date and the `Zmienione` flag (`GithubView.tsx:53-98`). `TodoView` also calls `useGithub({ refreshOnMount: false })` (`src/todo/TodoView.tsx:144`), so the snapshot type and its TS mirror have two consumers.
- **"Pokaż ukryte".** `const [showHidden, setShowHidden] = useState(false)` inside `GithubView` (`GithubView.tsx:272-273`): in memory, reset on leaving the view because the view unmounts. The slice does not touch it; the NEW rule applies to the visible list only, as GitLab (hidden issues never NEW). A hidden GitHub issue is therefore never NEW (**default chosen by agent**), but its key is still recorded in the seen set so hide/unhide never makes it NEW.

### 4. Visit detection and the timer-status rule

- `App` owns the active view in one `useState` (`App.tsx:36`) and calls `useGitlabSeenMark(view)` (`:37`). `WorkTimerProvider` is rendered by `App` itself (`App.tsx:62`), so a hook called in the body of `App` runs above the provider and cannot call `useWorkTimer()`; the existing hook takes the string `view` only.
- AGENTS.md rule: "The timer state is one `WorkTimerProvider` ... read it through `useWorkTimer()` and never subscribe to the timer status a second time." The seen-mark hook neither reads nor subscribes to timer status, so the rule holds for a twin `useGithubSeenMark(view)` called next to it. Subscribing to `main-hidden` is not a timer subscription; the existing hook already does it (`useGitlabSeenMark.ts:26-40`).
- A GitHub twin differs only in the compared view id (`"github"`), the command name, and the same `main-hidden` listener. Two near-identical hooks are acceptable; a single parameterised hook `useSeenMark(view, "gitlab", markGitlabSeen)` is the alternative (touches an existing hook, S-07 does not use it). **Default chosen by agent:** one shared parameterised hook is optional; the plan decides, with a bias to leaving `useGitlabSeenMark.ts` unchanged.

### 5. Proposed design (for the plan, not decided here)

**Rule (Rust, `tauri`-free, unit-tested):**
- NEW iff `first_seen_ms > mark` for the GitHub view mark; no mark means none; a repo's seeding round yields none.
- Round input: only repos with an `Ok` outcome (`results` where `outcome.is_ok()`); a failed repo is neither recorded nor forgotten (mirrors `commit_github_round`, `github_round.rs:9-11`, "a failed repo's absence is never judged").
- Keys: visible plus hidden issues of Ok repos, so hiding and unhiding never makes an issue NEW; the NEW flag is applied to visible issues only.
- Forget: key absent from an Ok repo for 30 days (`FORGET_AFTER_MS`), as GitLab; rows of repos no longer in settings are pruned or kept (see Open Questions; `apply_items_round` prunes by `active_scopes`, `hidden_issues/items.rs:261-351`).
- Seeding: per repo marker; a repo with no marker row seeds silently in its first successful round (also after the user adds a repo). Alternative one global marker would flag every issue of a newly added repo NEW. **Default chosen by agent: per-repo marker.**

**Round integration.** Between `commit_github_round` and the snapshot store, under the gate, as in `gitlab_refresh.rs:574-626`: record the seen set (one `Store::write`, no process, no lock held except the store lock) before taking the gate, take `fresh` under the gate (`accept_round`), apply flags to `next.repos` after partition, and set `shown_until = now_ms` only when the write succeeded. A failed write leaves `kept_new`/`shown_until` unchanged and logs a kind (never a repo or number).

**`act_on_issue`.** After `partition_github`, re-apply the flags from `kept_new` (as GitLab's hide commands do, `gitlab_refresh.rs:299-314`); otherwise hide/unhide/dismiss would silently clear all NEW flags for the visit.

**Failed or stale round.** `next_snapshot` keeps `prev.repos` (already flagged) for a calm failure only while the config is unchanged (`github/mod.rs:370-380`); `kept_new` must be re-applied when the repos are rebuilt from `kept_full` and cleared with `kept_full` when the list is dropped (`github_refresh.rs:304-311`).

**Command.** `mark_github_seen` (async, `crate::blocking`, `StoreHandle::with_store`, like `mark_gitlab_seen`, `gitlab_refresh.rs:419-441`): under the gate, save `max(mark, shown_until)`, clear `kept_new`, clear `is_new` on the snapshot, `seq + 1`, publish. Registered in `lib.rs` next to `github_refresh::refresh_github` (append only).

**Contract.** `GithubIssue.is_new: bool` (`#[serde(rename_all = "camelCase")]` gives `isNew`), TS `isNew: boolean` in `src/github/types.ts` in the same commit (AGENTS.md "Rust <-> TypeScript Contracts"). `GithubIssue {` matches 21 times in 9 files by `grep` (the definition and the struct literals): `github/mod.rs`, `github_refresh.rs`, `hidden_issues/github_round.rs`, `hidden_issues/rules.rs`, `hidden_issues/tests.rs`, `todo/commands.rs`, `todo/tests.rs`, `tests/todo.rs`, `tests/todo_crash.rs` (the last three build issues for "To do" tests). This is the same mechanical cost S-02 had with `GitlabItem` (plan-brief "13 files"). An alternative with no field churn is a separate `new_references: Vec<String>` on `RepoView` (checked in TS with `includes`); the field on the issue is the GitLab-consistent choice.

**UI.** In `IssueRow.extra`: `{issue.isNew && <Flag tone="new">Nowe</Flag>}` before the `changed` flag (`GithubView.tsx:77-92`). A new hook `useGithubSeenMark(view)` in `src/github/` calls `markGithubSeen()` from `api.ts` (the only file that imports `invoke`), registered in `App.tsx` next to line 37.

### 6. Migration v16

Current migrations end at version 15 (`hidden_items`, `store/schema.rs:298-327`); `validate` requires versions starting at 1 and rising by 1 (`schema.rs:331-340`). Tests derive the latest version from `schema::MIGRATIONS.len()` (`tests/new_assignments.rs:54`, `tests/hidden_items.rs:441`, `tests/events.rs:91` and others), so no hard-coded "15" needs editing; precedent: v14 "one version assert" renumber note (`context/archive/2026-11-27-gitlab-view-memory/plan-brief.md`, "Open Risks").

Proposed SQL (default chosen by agent; names are suggestions):

```sql
CREATE TABLE github_seen_repos (
  repo         TEXT PRIMARY KEY CHECK (length(repo) BETWEEN 1 AND 255),
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

- `repo` is the lowercase `owner/name`, the same key `hidden_items.scope` already stores (AGENTS.md app-data list: "GitHub issues by the lowercase `owner/name` repo key and issue number"). A hash like GitLab's account key is an option (SHA-256 is available, `gitlab/mod.rs:28-36`); the plain key is chosen because the table then reads the same as `hidden_items`, no new work data kind appears, and the AGENTS.md wording "minimal keys" holds. **Open for plan**: hash vs plain.
- The mark is one global row (the GitHub view is one list; the mark does not depend on the repo set). `github_view_mark` mirrors `gitlab_view_state`'s single-row shape (`schema.rs:290-300`). A per-repo mark would make a newly added repo's items fresh relative to the global visit; the per-repo seeding marker covers that case instead.
- No FK from the mark to the seeding table, as v9: a mark may exist before a repo's first stored round.
- Store errors never carry row values or paths (AGENTS.md "App Data Store"); reuse the `NewAssignmentError`-style error or a new `GithubSeenError` with `kind` + `Store` (precedent: `new_assignments/error.rs`).
- Tests (precedents): an integration test with the production migrations and a "v15 -> v16 upgrade" test like `tests/new_assignments.rs:50-120` (positions by `m.sql.contains("CREATE TABLE ...")`), a crash test with `tests/common/crash.rs` (data slices reuse the crash harness, AGENTS.md "App Data Store"), pure-rule unit tests, and snapshot-level tests for `accept_round`/`mark_seen` twins (`gitlab_refresh.rs:1546-1731`). Invented data only (`octo-test/alpha` style, as in `github/mod.rs` tests).

### 7. AGENTS.md update

The Hard Rules app-data list (single long bullet at `AGENTS.md:11`) enumerates records. S-03 adds, in the same style: GitHub seen-issue keys (lowercase `owner/name` repo key, issue number, two instants) and their per-repo seeding markers (the repo key and one instant), and the GitHub view's seen-until mark (one instant). No title, URL, login or comment text. The same bullet is edited by S-07 (hidden-item snapshots widened to GitLab merge requests): textual conflict on one line; resolve by keeping both additions. The frontend convention "The 'Pokaż ukryte' toggle of 'Projekty (GitHub)' is view state in memory only" stays true (no change).

### 8. Precedents and what to carry over

- `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan-brief.md`: decisions for the mark (last visit, per account, v9), end of visit (leave view or hide window), first visit without a mark = nothing NEW, rule in Rust in `record_round`, mark value `shown_until` not the clock (plan review F1), exit detection in `App.tsx` plus `main-hidden`, `tone="new"`. Carried over unchanged for GitHub.
- `context/archive/2026-11-23-new-assignment-notify/plan-brief.md`: the seen set per account, silent seeding via a never-pruned marker, forget after 30 days, session fallback when the store fails (needed for toasts only). **GitHub has no toasts**, so the session fallback (`SessionSeen`) is not carried over: a failed seen-set write simply leaves flags as they are (as `fresh = None`).
- `context/archive/2026-11-27-github-hide-resurface/research.md` and `plan.md`: `hidden_items` v15, `partition_github`, `act_on_issue`, no GitHub account key, scope = repo key.
- `context/archive/2026-11-27-gitlab-review-requests/research.md` and `plan.md`: the `:reviews` marker and `baseline_first_seen` (baseline key stored with `first_seen = min(mark, now)`, and `last_seen = now` so the forget rule does not drop it, plan "Baseline ordering"). **GitHub analogue:** a repo's first successful round is the baseline; keys stored at the seeding round have `first_seen = last_seen = now` and `fresh` is forced empty for that repo in that round.
- `context/archive/2026-11-27-gitlab-view-memory/plan-brief.md`: single-row table shape, load/save in the list-state hook, AGENTS.md data list edit "in this change", renumber risk.

### 9. Roadmap and PRD conventions

- Roadmap S-03: outcome "user can see a NEW flag on GitHub items that appeared since their last visit to the view; GitHub gives no Windows notice (decided 2026-11-27)"; Change ID `github-new-and-notify`; PRD refs MS-30, FR-005, FR-023; prerequisite F-01 (done); risk "A migration (GitHub seen keys and a GitHub view mark); it runs in a later wave than S-01" (`context/foundation/roadmap.md:129-140`). The roadmap's baseline text still says "schema v13" and "9 read-only CliCall variants" (`:80-81`), which is stale (store is at v15 on `develop`); the roadmap is read-only for this slice.
- MS-30 scope anchor: "GitHub seen-assignment keys in today's shape and a NEW flag on items that appeared since the last visit to the view; no Windows notice for GitHub (issue #30 ...)" (`roadmap.md:33`). "In today's shape" is read here as "the same shape as GitLab's seen set" (key, first/last seen, a seeding marker), which the proposed tables satisfy.
- PRD note convention: M-4 adds no FR; each slice may get a one-line PRD note "(MS-NN, roadmap), bez nowego FR", as for MS-35 (`context/foundation/prd.md:242`) and M-3's notes (`:236-240`). FR-023 text is GitLab-specific (`prd.md:179`); the GitHub flag is anchored on MS-30 without changing FR-023. **Default chosen by agent:** at most one PRD sentence after line 242 in the same Polish style; the roadmap says S-06 and S-07 get PRD notes, S-03 is not named in roadmap Open Question 2 (`roadmap.md:207`), so the note is optional.

### 10. Overlap with S-07 (`gitlab-hide-merge-requests`)

| Shared file | S-03 edit | Risk |
| --- | --- | --- |
| `AGENTS.md` (app-data bullet, line 11) | add GitHub seen keys, markers and the view mark | one long line edited by both; resolve by combining |
| `src-tauri/src/lib.rs` | append `github_refresh::mark_github_seen` (and `mod github_seen;` if a module is added) | order of the `generate_handler!` list and `mod` lines; append, do not reorder |
| `src-tauri/src/store/schema.rs` | only S-03 appends v16 | none for S-07 (no migration) |
| `src-tauri/src/cli/mod.rs`, `src-tauri/src/bin/fake_cli.rs` | none | S-07 adds a `glab api` variant and guard test |
| `src-tauri/src/hidden_issues/` | none; **but** `GithubIssue` gains `is_new`, and `hidden_issues/github_round.rs`, `rules.rs`, `tests.rs` construct `GithubIssue` literals | if S-07 also edits those files for GitLab MRs the hunks are in different functions; keep the S-03 edit to adding `is_new: false` |
| `new_assignments/`, GitLab seen keys | none (separate GitHub tables) | none |
| `gitlab_refresh.rs`, `src/gitlab/` | none | S-07 may edit; no S-03 hunk there |
| `src/App.tsx` | one hook call line and one import | S-07 is unlikely to touch it |

### 11. Risks and edge cases to settle in the plan

- **Race: first round vs the visit.** The flags of a round are computed before the gate from the mark read in the same write; a late round whose mark read predates a `mark_github_seen` is filtered by `last_mark` (GitLab `accept_round`, `gitlab_refresh.rs:198-235`). The GitHub twin needs the same guard.
- **Failed repo in an otherwise Ok round.** Its issues are not in the round; they must not be forgotten (absence is not judged) nor flagged; the repo's old rows keep `last_seen_ms` (the 30-day forget counts from `last_seen_ms` only for Ok repos).
- **Truncated repo (`GH_ISSUE_LIMIT`).** A truncated repo is `Err(Truncated)` (`github/fetch.rs:28-33`), so it is a failed repo for this purpose.
- **Repo added to settings.** Its first successful round seeds silently (per-repo marker). Repo removed: rows stay until the plan chooses a prune (30-day or by active repos).
- **gh login changes.** Issues now assigned to another login appear as new keys and are NEW once; `fetch_assigned` uses `--assignee @me` (`GithubView.tsx:423` comment) so the key is the issue, not the login. Accepted (the same limitation GitLab documents for reassignment).
- **Closed and reopened issue.** The issue leaves the list and returns within 30 days: same key, not NEW (same as GitLab "reopens stay quiet"); after 30 days absent it is NEW again.
- **Store unavailable at start.** `StoreHandle::with_store` error leaves flags off and `shown_until` unset, so no mark is written (same as GitLab "a failed first round saves no mark", `gitlab_refresh.rs:1727`). The view must not show an alarm: log a kind only (AGENTS.md: logs never carry repo, number, title).
- **Seeding and a pre-existing mark.** Even if the global mark exists, a repo's seeding round returns no fresh keys, otherwise a repo added after the mark would flag all its issues.
- **`TodoView` also reads the snapshot** (`useGithub({ refreshOnMount: false })`), so `isNew` is simply ignored there; the mark must not be written from `TodoView` (the hook is in `App`, tied to the view id).
- **Quit while on the view** writes no mark; flags return next time (GitLab behaviour, accepted).

## Code References

- `src-tauri/src/new_assignments/rules.rs:90-99` - `fresh_since`: first seen after the mark, no mark means none.
- `src-tauri/src/new_assignments/rules.rs:101-128` - review baseline and `baseline_first_seen`.
- `src-tauri/src/new_assignments/rules.rs:130-155` - `plan_round`: seeding, new keys, 30-day forget.
- `src-tauri/src/new_assignments/mod.rs:143-157` - `mark_view_seen`: never moves the mark back.
- `src-tauri/src/new_assignments/mod.rs:170-304` - `record_round`: one write, markers, upsert, forget, fresh.
- `src-tauri/src/new_assignments/mod.rs:85-91` - `review_marker_key` (`:reviews`).
- `src-tauri/src/gitlab_refresh.rs:99-107,198-267` - `kept_new`, `shown_until`, `last_mark`, `accept_round`, `mark_seen`.
- `src-tauri/src/gitlab_refresh.rs:419-441` - `mark_gitlab_seen` command.
- `src-tauri/src/gitlab_refresh.rs:757-817` - `record_seen` (store failure handling).
- `src-tauri/src/github_refresh.rs:64-154` - `GithubRefreshState`, `act_on_issue`.
- `src-tauri/src/github_refresh.rs:249-330` - `refresh_round`.
- `src-tauri/src/github/mod.rs:22-40,247-290,316-380` - `GithubIssue`, `GithubSnapshot`, `next_snapshot`.
- `src-tauri/src/github/fetch.rs:19-47` - per-repo outcomes and whole-source failures.
- `src-tauri/src/hidden_issues/github_round.rs:294-334` - `partition_github`.
- `src-tauri/src/store/schema.rs:130-146,209-221,290-327,331-340` - v7, v9, v14, v15, `validate`.
- `src-tauri/src/lib.rs:33-34,188-197,206-210` - `main-hidden`, command registration.
- `src/App.tsx:36-37,62-85` - view state, seen-mark hook, provider.
- `src/gitlab/useGitlabSeenMark.ts:15-40` - visit detection hook.
- `src/gitlab/GitlabView.tsx:189` - the NEW flag.
- `src/github/GithubView.tsx:53-98,272-273` - `IssueRow`, in-memory `showHidden`.
- `src/github/types.ts`, `src/github/api.ts`, `src/github/useGithub.ts` - TS mirror, api, hook.
- `context/foundation/roadmap.md:33,129-140` - MS-30 and S-03.

## Architecture Insights

- Domain rules (NEW rule, seeding, forget) belong in a `tauri`-free Rust module; the UI only shows `isNew` and calls `markGithubSeen` on a view change (AGENTS.md "Frontend Conventions").
- The refresh state keeps view-affecting state as separate mutexes under one documented lock order; the GitHub state must extend the documented order (gate, `kept_config`, `kept_full`, `kept_new`, `shown_until`, `last_mark`, snapshot) and never take the store lock while holding a `kept_*` lock (`github_refresh.rs:12-16`).
- GitLab and GitHub differ in identity (account hash plus numeric id vs repo key plus number) but share the temporal rule (`first_seen > mark`); the two sets of tables stay separate and the temporal rule is the part duplicated. The refresh skeleton (F-01) deliberately does not own this.
- Data contracts: a new `is_new` field is a serde change; Rust and TS mirror in one commit; logs carry kinds and counts only.

## Historical Context (from prior changes)

- `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan-brief.md` - mark semantics, `shown_until`, first visit silent, StrictMode-safe exit detection.
- `context/archive/2026-11-23-new-assignment-notify/plan-brief.md` - seen set, silent seeding with a never-pruned marker, 30-day forget, session fallback (toasts only).
- `context/archive/2026-11-27-github-hide-resurface/research.md` - GitHub identity as `(repo key, number)`, `hidden_items` v15, no account key.
- `context/archive/2026-11-27-gitlab-review-requests/research.md` and `plan.md` - derived seeding marker, baseline instants.
- `context/archive/2026-11-27-gitlab-view-memory/plan-brief.md` - single-row table and AGENTS.md data-list precedent; migration renumber risk.

## Related Research

- `context/archive/2026-11-27-github-hide-resurface/research.md`
- `context/archive/2026-11-27-gitlab-review-requests/research.md`
- `context/archive/2026-11-27-gitlab-view-memory/research.md`

## Open Questions

1. **Repo key plain vs hashed** in v16 (default: plain lowercase `owner/name`, as `hidden_items`). Owner: team. Block: no (settled in `/10x-plan`).
2. **Own tables and rules vs a generic `fresh_since`/`plan_round`** shared with GitLab (default: own module, no edit of `new_assignments/`). Owner: team. Block: no.
3. **Prune of rows of repos removed from settings** (30-day forget by `last_seen_ms` only vs also by active repos like `apply_items_round`). Default: forget by 30 days absent from an Ok round; rows of removed repos expire the same way only if a prune also runs on `last_seen_ms` for repos not in the round (needs a rule). Owner: team. Block: no.
4. **`is_new` on `GithubIssue` (21 literals) vs a `RepoView` list of numbers** (default: field on the issue, GitLab-consistent). Owner: team. Block: no.
5. **One shared parameterised seen-mark hook vs a twin hook** (default: twin `useGithubSeenMark`, leave `useGitlabSeenMark.ts` unchanged). Owner: team. Block: no.
6. **Optional PRD one-liner for MS-30** (default: one sentence after `prd.md:242`, no FR change; the roadmap names notes only for S-06 and S-07). Owner: user. Block: no.
7. **Not checked in this research:** whether a Tauri `window.emit` in `CloseRequested` always reaches the webview before it is suspended (GitLab precedent assumes so, safe side: flags show again); the real-GitHub behaviour of a round with one failing repo was inferred from the code and its tests, not run (no `gh` call was made, no app was started).
