---
date: 2026-11-27T16:57:40+01:00
researcher: Claude (Opus 5.5, slice orchestrator sub-agent)
git_commit: 00e6a19b6653f988e73b4c7744f9dbb5050a6981
branch: slice/github-hide-resurface
repository: day-navigator
topic: "S-01 github-hide-resurface: hide/show GitHub issues and resurface them marked Zmienione (MS-31, FR-003, FR-005)"
tags: [research, github, hidden-issues, cli, store, migration-v15, source-refresh]
status: complete
last_updated: 2026-11-27
last_updated_by: Claude (Opus 5.5, slice orchestrator sub-agent)
---

# Research: hide and resurface GitHub issues (S-01 `github-hide-resurface`)

**Date**: 2026-11-27T16:57:40+01:00
**Researcher**: Claude (Opus 5.5, slice orchestrator sub-agent)
**Git Commit**: 00e6a19b6653f988e73b4c7744f9dbb5050a6981 (develop after M-4 wave 1)
**Branch**: slice/github-hide-resurface
**Repository**: day-navigator

## Research Question

How does today's GitLab hidden-issue flow work end to end (store, rules, round,
commands, UI), how does the GitHub refresh fetch issues, which read-only `gh`
calls with fixed arguments give an issue's state and its comments with author
logins and timestamps, plus the user's login (`gh api user`), and, for the
plan to settle:

1. Can `src-tauri/src/hidden_issues/rules.rs` be shared by GitLab and GitHub without a copy?
2. Which storage shape (migration v15) can also hold GitLab merge requests later (S-07), with source/kind in the key?
3. Where does this slice meet the parallel slices S-02 `github-refresh-interval` and S-06 `gitlab-review-requests`?

User decisions taken as fixed (from `change.md`): the login comes from a new
`gh api user` `CliCall`, not from a setting; only a state change or a comment by
someone other than the user brings an issue back (no labels, edits,
reactions); the GitHub look and behaviour mirror GitLab's.

## Summary

- **Rules can be shared without a copy, with a small generalisation.** The
  rule functions in `rules.rs` touch exactly four GitLab-specific things: the
  item type `GitlabItem` (`id`, `updated_at`, label names), the note type
  `gitlab::notes::IssueNote`, the user type `cli::GitlabUser`, and
  `gitlab::ChangeReason` (`rules.rs:9-11`). The algorithm (`needs_comment_check`,
  `comment_verdict`, `plan_round`, `rules.rs:110-192`) needs only an item key,
  an `updated_at` text and a "status fingerprint" from each listed item, and an
  author, a "counts as a comment" flag and a `created_at` from each comment. A
  small trait for the listed item plus a source-neutral comment struct and an
  `Option<&str>` user make the same functions serve both sources. The GitLab
  call sites and the integration tests that call `hidden_issues::hide/list`
  with GitLab types (`tests/hidden_issues_round.rs:102,109`) can keep their
  signatures.
- **GitHub fits the existing `u64` record key when the record scope is the repo.**
  GitLab keys records by `(host, issue_id)` (`store/schema.rs:107-122`). For
  GitHub, `(repo key, issue number)` is the same shape: the scope is the
  lowercase `owner/name` (`cli/mod.rs:166-169`) and the id is the issue number
  (`github/mod.rs:23`). The rules then stay on `u64` ids.
- **Storage: a generic `hidden_items` table keyed by `(source, scope, kind,
  item_id)`** holds GitHub issues now and GitLab merge requests later with no
  further migration. Two variants are viable (new table beside `hidden_issues`,
  or new table plus moving the GitLab rows in the same v15); a GitHub-only table
  is ruled out by the S-07 constraint. Options and trade-offs are below.
- **`gh` calls (verified against gh 2.105.0 on this machine):**
  - `gh api user` → JSON object with `login`, `id`, `type` (keys checked; values not recorded).
  - `gh issue view <N> -R <owner/name> --json state,updatedAt,comments` → every comment
    (an issue with 231 comments returned 231), each with `author.login`, `createdAt`,
    `authorAssociation`, `viewerDidAuthor`, `body`, and more.
  - Rejected: `gh issue list --json comments` returns at most 100 comments per issue
    (231-comment issue → 100), so new comments are lost. `gh api repos/.../comments
    --paginate` needs `?`/`&` in an argument (the guard's `BAD_CHARS` refuses `&` outside
    the GitLab endpoint position, `cli/mod.rs:741-753`) and prints one JSON array per page.
- **"State change" on GitHub reduces to leaving and re-joining the list.** The
  list call is `--state open` (`cli/mod.rs:186-204`), so a hidden issue that is
  closed leaves the list (`MarkAbsent`) and comes back on reopen as
  `returned` (`rules.rs:149-174`), the same reasoning GitLab's S-03 used
  for open/closed. Labels are excluded by the user's decision, so the GitHub
  status fingerprint is empty unless the plan adds `stateReason` (open question 1).
- **The GitHub refresh has none of the hiding machinery yet:** no gate, no kept
  full list, no partition, no hidden fields in `GithubSnapshot`
  (`github_refresh.rs:54-58`, `github/mod.rs:232-266`). F-01 left these to S-01 on purpose.
- **Per-repo failures are a new hazard.** `plan_round` marks every record whose
  item is not listed as absent (`rules.rs:149-157`). A GitHub repo that failed
  this round (`Failed`, `ResponseInvalid`, `Truncated`, `github/fetch.rs:15-46`) has
  no list, so its records must be left out of that round, or they would later
  resurface falsely as `returned`.
- **Merge points:** S-02 meets this slice in `github_refresh.rs::refresh_round`,
  `GithubSnapshot` + `src/github/types.ts`, and `GithubView.tsx`. S-06 meets it
  in `cli/mod.rs` (variant enum, `tool/args/name`, `examples`,
  `VARIANT_COUNT`/`EXAMPLE_COUNT`/`ordinal`/seen array, `check_read_only`),
  `bin/fake_cli.rs`, and, only if the plan moves GitLab rows or edits GitLab
  call sites, `hidden_issues/` + `gitlab_refresh.rs`.

## Detailed Findings

### 1. GitLab hidden-issue flow end to end (implemented, S-03 `hide-and-resurface`, migration v6)

**Store record** (`store/schema.rs:101-122`): table `hidden_issues` with
`host, issue_id, project_id, iid, scoped_labels (JSON), baseline_updated_at
(RFC 3339 text), settle_after_ms, hidden_at_ms, absent_since_ms,
changed_reason ('status'|'comment'|'returned'), changed_at_ms`, primary key
`(host, issue_id)`, and a CHECK that `changed_reason` and `changed_at_ms` are
both NULL or both set. A NULL reason is a hidden issue; a reason is a visible
issue with the "Zmienione" mark; no row is a plain issue (`rules.rs:19-39`).
No titles, URLs, references or comment text (`hidden_issues/mod.rs:5-6`).

**Store operations** (`hidden_issues/mod.rs`):
- `list(store, host)` reads the host's rows (`:88-90`); an unreadable row is `Damaged` (`:41-45`, `:75-85`).
- `hide` upserts a fresh snapshot (scoped labels, listed `updated_at` as baseline, `settle_after_ms = now + SETTLE_MS`, `hidden_at_ms = now`) and refuses a merge request with `NotInList` (`:94-131`).
- `unhide` deletes a hidden row, `dismiss` deletes a marked row; the other case is `NotFound` (`:133-166`).
- `apply_round` applies all transitions in one write, each guarded by the row state the plan assumed (`hidden_at_ms`, absent or not), prunes other hosts' rows hidden 30 days or more, and returns the host's rows after the write (`:168-241`).
- `partition(full_issues, records)` splits the full list into visible (with `changed`) and hidden (with `hidden_at_ms`) in list order (`:243-271`).

**Rules** (`hidden_issues/rules.rs`, pure, `now_ms` is a parameter):
- `SETTLE_MS = 120_000` (GitLab's 60 s touch throttle plus margin) and `PRUNE_AFTER_MS = 30 days` (`:13-17`).
- `status_changed`: the sorted, deduplicated `key::value` label set differs from the stored one (`:79-94`).
- `needs_comment_check`: hidden, and the listed `updated_at` is strictly later than the baseline (unreadable counts as moved) or the settle check is due (`:96-115`).
- `comment_verdict`: some note is non-system, `created_at` strictly after the baseline, and its author is not the user (ASCII case-insensitive); with no user any author counts (`:117-135`).
- `plan_round`: per record, not listed → `MarkAbsent` once, `Prune` after 30 days absent; listed and marked → `ClearAbsent` if needed; listed and hidden → reason by precedence status, comment, returned → `Resurface`; else, after a successful "nothing new" check, `AdvanceBaseline` to the listed `updated_at` with a new settle time if it moved (`:137-192`).

**Round** (`hidden_issues/round.rs`):
- `check_round` (no lock held) reads the records, and for each hidden, listed, due issue runs one `glab` notes call. After the first `Unavailable`/`LoggedOut`/`ToolMissing` no further call starts (`stops_round`, `round.rs:53-60`), and the next round resumes after the issue that stopped it (`check_order`, `round.rs:62-70`; `stopped_at`, `:33-41`). A store read failure skips the calls (`round.rs:80-142`).
- `commit_round` plans, applies in one write, partitions from the returned rows, and counts failed checks only for issues still hidden after the write. A store failure returns the list unpartitioned with `store_error` set, never an empty list (`round.rs:144-199`).
- The user comes from settings `gitlab.user` (`cli/mod.rs:88-124`, passed at `round.rs:118` as `config.user.as_ref()`).

**Refresh wiring** (`gitlab_refresh.rs`):
- `GitlabRefreshState` has `kept_config`, `kept_full` (the unpartitioned list, never serialized), `kept_filter`, `kept_new`, `shown_until`, `last_mark` and `hidden_gate` (`:86-110`). The lock order is documented at `:9-16`.
- In `refresh_round` the check runs before the gate (`:550-558`), the commit and the snapshot store run under `hidden_gate` (`:584-660`), and publication happens after the gate is released.
- `act_on_issue(id, act)` takes the gate, finds the issue in `kept_full` (`NotInList`, `ListUnavailable` otherwise), runs the store op, re-shapes the list from the returned rows and stores the snapshot with `seq + 1`; the caller publishes (`:140-196`).

**Commands** (`hidden_issues/commands.rs`): `hide_gitlab_issue`,
`unhide_gitlab_issue`, `dismiss_gitlab_change`, each `async`, taking `id: u64`,
running on the blocking pool through `crate::blocking`, then
`publish` on success; store errors are logged by kind (`:20-81`). Registered in
`lib.rs:211-213`. Error type `HiddenIssueError { Store, NotFound, NotInList,
ListUnavailable }` with `tag = "kind", content = "detail"` and no value-carrying
variant (`hidden_issues/error.rs:10-21`); fixed Polish text in `message_pl`
(`error.rs:58-88`, e.g. "Lista GitLaba nie jest jeszcze dostępna." is GitLab-worded).

**UI** (`src/gitlab/`; read in full by a sub-agent, decisive lines re-checked):
- `api.ts:25-38` wraps the three commands with `{ id }`; `hiddenIssueErrorMessage` (`api.ts:54`) maps the error kinds to Polish text.
- `types.ts:8` `ChangeReason = "status" | "comment" | "returned"`; `GitlabItem.changed` (`types.ts:31`) and `hiddenAtMs` (`types.ts:37`); `HiddenIssueError` mirror at `types.ts:123`.
- `useGitlab.ts:92` `act()` adds the id to `busy`, awaits the command and relies on the `gitlab-changed` event (`useGitlab.ts:42`) for the new snapshot; there is no optimistic update.
- `GitlabView.tsx`: `CHANGE_LABEL` (`:62`) gives the reasons "zmiana statusu", "nowy komentarz", "wróciło na listę"; `RowButtons` (`:109-158`) has the actions "OK, zauważone" (`:129`), "Pokaż na liście" (`:141`), "Ukryj do zmiany" (`:152`); a hidden row is dimmed (`:178`, class `is-hidden`) and shows "ukryte DD.MM" (`:199`, `formatDay`); the toggle "Pokaż ukryte (n)" (`:522`); the notices for `hiddenError` (`:678`), `hiddenCheckFailed` (`:684`) and the "set user" hint (`:691`).
- **Correction to the slice wording:** GitLab has no separate "Ukryte (n)" section. Hidden issues appear as dimmed rows in the same table when the "Pokaż ukryte (n)" toggle is on, and that toggle is persisted in `gitlab_view_state` (migration v14, `store/schema.rs:286-296`). "Mirror GitLab" therefore means a toggle plus dimmed rows with "ukryte DD.MM", unless the plan decides otherwise.

### 2. GitHub refresh today

- One call per configured repo, in settings order: `CliCall::GhAssignedIssues(repo)` → `gh issue list -R <repo> --assignee @me --state open --limit 500 --json number,title,url,updatedAt,labels` (`cli/mod.rs:186-204`, `github/fetch.rs:20-46`).
- `Unavailable`, `LoggedOut` and `ToolMissing` end the round as a whole-source error. `Failed`, `ResponseInvalid` and `Truncated` (500 or more results, `fetch.rs:28`) stay per repo (`fetch.rs:15-19`).
- Parsing: `GithubIssue { repo, number, reference = "<owner/name lowercase>#<n>", title, web_url, updated_at, labels: Vec<String> }`; a number 0 or an unreadable timestamp is `ResponseInvalid` (`github/mod.rs:19-35`, `:168-205`).
- Snapshot: `GithubSnapshot { state, repos: Vec<RepoView{repo, issues, error}>, last_success_ms, checked_at_ms, refreshing, seq }` (`github/mod.rs:204-266`). `next_snapshot` keeps the last repos on a whole-source failure only while the repo list is unchanged (`github/mod.rs:276-330`).
- Refresh: `GithubRefreshState { cell, kept_config }` (`github_refresh.rs:54-58`), a fixed `REFRESH_INTERVAL` of 10 min (`:39`). `refresh_round` fetches, builds `next`, reconciles "To do" with the repos that loaded in an `Ok` round only (`reconcile_input`, `:201-211`), stores and publishes (`:163-196`). Commands: `github_snapshot`, `refresh_github` (`:87-101`; `lib.rs:209-210`).
- UI: `src/github/GithubView.tsx` renders one `Panel` per repo (`RepoGroup`, `:73`) of `ItemRow`s (`IssueRow`, `:34`) with the to-do "+" and an external-link button; `useGithub.ts:29` listens to `github-changed`. `ItemRow` takes a single `flag` with one `flagTone` (`components/ItemRow/ItemRow.tsx:12-13,76`), so showing "Zmienione" next to "W Do zrobienia", or a hide date, needs either an `ItemRow` extension or composing them in the `title` node. The GitLab view uses `DataTable` rows instead.
- F-01 notes for S-01 (`context/archive/2026-11-27-source-refresh-skeleton/plan.md:57`, `research.md:171-175`): GitHub needs "the same gate pattern and a `kept_full`", a check step outside the gate and a commit under it, and publication after the gate. `SourceCell::publish` is `pub(crate)` (`source_refresh.rs:92`) and callable from a command module.

### 3. `gh` calls for hiding (verified with gh 2.105.0; no real output copied)

The checks ran in a temp directory against a public repo (`cli/cli`) and the
user's own `gh api user`, printing only key names, counts and `type`.

| Purpose | Fixed argument list | What it gives | Verdict |
| --- | --- | --- | --- |
| Own login | `api user` | object with `login`, `id`, `type` (`"User"` here) | **use** (user decision); parse `login` only |
| State and comments of one hidden issue | `issue view <N> -R <owner/name> --json state,updatedAt,comments` | `state`, `updatedAt`, `comments[]` with `author.login`, `createdAt`, `authorAssociation`, `viewerDidAuthor`, `body`, `id`, `isMinimized`, `includesCreatedEdit`, `minimizedReason`, `reactionGroups`, `url` | **use**; all comments (231 of 231 on a large issue) |
| Comments inside the list call | `issue list ... --json ...,comments` | at most 100 comments per issue (231-comment issue → 100) | reject: newest comments lost, and the list call grows 500× heavier |
| REST comments | `api repos/<o>/<r>/issues/<N>/comments?per_page=100 --paginate` | `user.login`, `user.type`, `created_at`; ascending; one JSON array per page | reject: `?`/`&` hit the guard's `BAD_CHARS` outside the GitLab endpoint slot (`cli/mod.rs:745-753`); the `gh api` form also accepts `-f`/`-F`/`--method` |

Fields offered by `gh issue view/list --json` in 2.105.0 include `state`,
`stateReason`, `closed`, `closedAt`, `comments`, `updatedAt`, `labels`,
`projectItems`. `gh issue view` also accepted a pull-request number (state
`MERGED`), so a parser must accept a `state` value beyond `OPEN`/`CLOSED` or
treat an unknown one as `ResponseInvalid`.

Facts for the rule:
- Only `createdAt` is compared, so comment edits never resurface an issue (matches the user's decision). Reactions are not comments.
- Bots and automated comments count as "someone else" (FR-003, `prd.md:126-127`); the login compare is case-insensitive like GitLab's (`rules.rs:128`).
- `viewerDidAuthor` would tell "my comment" without `gh api user`. The user chose `gh api user`, so this is recorded as an alternative only.
- A deleted author ("ghost") or a missing `author` must count as someone else, not fail the parse. How gh renders it was not verified (open question 4).
- Whether GitHub throttles `updatedAt` bumps the way GitLab does (the reason for `SETTLE_MS`) is unverified. Keeping one settle check costs one extra call per hide.

Guard and fake-cli consequences (`cli/mod.rs:529-760`, `bin/fake_cli.rs:404-560`):
- Each new variant must be added to `tool()`, `args()`, `name()` and `examples()`; `VARIANT_COUNT` (`:529`), `EXAMPLE_COUNT` (`:531`), `ordinal` (`:543`) and the `seen` array must grow; `ALL` stays parameterless (`gh api user` takes no parameter, so it may join `ALL` and the ignored real smoke test, which the plan should decide).
- `check_read_only` (`:727-759`) is an allowlist of exact shapes that ignores the tool. It needs two new exact shapes, `["api","user"]` and `["issue","view",<digits>,"-R",<repo>,"--json","state,updatedAt,comments"]`, each checked by validators written in the test module (`guard_number_ok` `:624`, `guard_repo_ok` `:696` exist), plus near-miss rejections (extra arg, `-X`, another endpoint after `api`, a leading-zero or non-digit number).
- The fake `gh` scenario routes only `issue list` and exits 98 for anything else (`bin/fake_cli.rs:455-461`); `api user` and `issue view` need routes with invented logins and comments carrying leak markers.

### 4. Sharing the rules (team question 1)

What is GitLab-bound in `rules.rs`:
- `HiddenRecord` fields `issue_id`, `project_id`, `iid`, `scoped_labels` (`:22-34`). `project_id`/`iid` are only used by `round.rs` to build the notes call (`round.rs:113-117`), not by the rules.
- `status_changed(record, &GitlabItem)` reads `item.label_names()` (`:92-94`).
- `updated_moved`/`needs_comment_check`/`plan_round` read `item.id` and `item.updated_at` (`:98-115`, `:149`, `:186`).
- `comment_verdict(&[IssueNote], baseline, Option<&GitlabUser>)` reads `n.system`, `n.created_at`, `n.author` (`:120-135`).
- `ChangeReason` lives in `gitlab/mod.rs:49-53` and is the serialized `changed` value.

A generalisation that keeps one copy:
- A trait for a listed item, e.g. `fn hide_id(&self) -> u64; fn updated_at(&self) -> &str; fn status_fingerprint(&self) -> Vec<String>`, implemented by `GitlabItem` (scoped labels) and `GithubIssue` (empty, or `stateReason` if the plan adds it).
- One comment struct `{ author: String, counts: bool, created_at }`: GitLab maps `counts = !system`, GitHub maps `counts = true`.
- `user: Option<&str>` instead of `Option<&GitlabUser>`.
- `ChangeReason` moves to `hidden_issues` (or stays and is re-exported) so GitHub does not depend on `gitlab`.
- The rename of `scoped_labels` to a neutral fingerprint is internal; the existing rule tests construct `HiddenRecord` in 2 places in `hidden_issues/tests.rs`, 1 in `tests/hidden_issues_round.rs` and 2 in `gitlab_refresh.rs` tests (counted by grep for `HiddenRecord {`).
- `round.rs::check_round/commit_round` take `GitlabConfig`, `AssignedItems` and call `fetch_issue_notes`. GitHub needs its own thin round (repo loop, `gh issue view`, `gh api user`), or a generic round over a "fetch comments" closure. The pure parts are `check_order`, `stops_round` (per error type) and the failed-count rule.

Conclusion: yes, without a copy. The cost is a trait plus two small types, and the GitLab public API (`hide`, `list`, `check_round`, `commit_round`) keeps its signatures if GitLab-typed wrappers stay.

### 5. Storage shape (team question 2)

Constraints: append migration v15 only (`store/schema.rs:298-310`, contiguous
versions); one write per change; no titles, URLs or comment text; the GitLab
MR id space is separate from the issue id space, so `kind` must be in the key
(`context/archive/2026-11-23-new-assignment-notify/plan.md:12`); S-07 must need
no migration.

**Shape:** `hidden_items(source TEXT CHECK IN ('gitlab','github'), scope TEXT
NOT NULL, kind TEXT CHECK IN ('issue','mergeRequest'), item_id INTEGER CHECK
(item_id > 0), project_id INTEGER NULL, iid INTEGER NULL, status_fingerprint
TEXT NOT NULL, baseline_updated_at, settle_after_ms, hidden_at_ms,
absent_since_ms, changed_reason, changed_at_ms, same reason CHECK, PRIMARY KEY
(source, scope, kind, item_id))`.
- GitHub issue: `source='github'`, `scope` = lowercase `owner/name`, `kind='issue'`, `item_id` = number, `project_id`/`iid` NULL.
- GitLab MR (S-07): `source='gitlab'`, `scope` = host, `kind='mergeRequest'`, `item_id` = global MR id, `project_id`/`iid` for the MR notes call.
- A CHECK can tie `project_id`/`iid` to `source='gitlab'`.

| Option | v15 contents | GitLab code touched | S-07 | Risk |
| --- | --- | --- | --- | --- |
| A. New generic table, GitLab issues stay in `hidden_issues` | `CREATE TABLE hidden_items` | none (rules generalised only) | uses `hidden_items` for MRs, no migration | two tables with one meaning; GitLab issues and MRs in different tables |
| B. New generic table and move GitLab rows | `CREATE` + `INSERT ... SELECT 'gitlab', host, 'issue', issue_id, ... FROM hidden_issues` + `DROP TABLE hidden_issues` in one transaction | `hidden_issues/mod.rs` SQL switches table; crash and upgrade tests for the move | no migration | GitLab regression risk in this slice; more merge surface with S-06; upgrade test needed (template: content-search pattern in `tests/new_assignments.rs:47-75`) |
| C. GitHub-only table | `CREATE TABLE github_hidden_issues` | none | needs its own migration | **excluded** by the S-07 constraint |

Account: GitLab rows are keyed by host, not by account (`schema.rs:121`);
mirroring that, GitHub rows carry no account and survive a `gh auth switch`.
A repo removed from settings is never listed again, so its rows need the same
treatment as GitLab's other-host rows (prune after 30 days hidden,
`hidden_issues/mod.rs:235-238`), or they stay forever.

Existing migration tests do not hard-code the last version (`tests/events.rs:180-201`,
`tests/gitlab_view_state.rs:101-115`, `tests/new_assignments.rs:50-58` use
content search plus `MIGRATIONS.len()`), so appending v15 breaks none of them.

### 6. GitHub round design facts the plan needs

- **Failed repos:** records of a repo whose result is `Err` this round must be excluded before `plan_round`, or they get `MarkAbsent` and later a false `returned` (`rules.rs:149-157`, `github/fetch.rs:15-19`). Same rule as `reconcile_input` (`github_refresh.rs:198-211`).
- **Whole-source failure:** `check_round`-style stop after the first `Unavailable`/`LoggedOut`/`ToolMissing` from `gh issue view` or `gh api user`; resume rotation as in GitLab (`round.rs:53-70`).
- **Login:** `gh api user` is needed only when at least one comment check is due, so it can run lazily once per round. If it fails, the plan must choose: count the due checks as failed (safe: no false resurface on the user's own comment), or treat every author as someone else (GitLab's no-user behaviour, `rules.rs:117-119`). Never store the login in clear; nothing in the rule needs it stored.
- **Kept list and gate:** commands need `kept_full` (the unpartitioned per-repo lists) and a `hidden_gate` in `GithubRefreshState`; the round stores under the gate and publishes after it, like `gitlab_refresh.rs:584-660`.
- **Snapshot and TS mirror:** GitHub needs per-issue `changed` and `hiddenAtMs`, a hidden list per repo (or one list), `hiddenCheckFailed`, `hiddenError`, and a GitHub-worded `ListUnavailable` text ("Lista GitHuba ..."). `HiddenIssueError::message_pl` is GitLab-worded (`error.rs:84`).
- **Command keys:** GitHub has no global numeric id in `GithubIssue`; the commands can take `reference` (`owner/name#N`, already the "To do" key) or `(repo, number)`, resolved against the kept list (never data from the UI, as `commands.rs:4` requires).
- **"To do" interplay:** GitLab renders the "+" for a hidden row only if it is already in "To do" (`GitlabView.tsx:249`); `todo::reconcile_github` gets the full loaded list (`github_refresh.rs:216-226`), so a hidden issue is not ticked as closed.

### 7. Merge points with the parallel slices

| File / area | S-01 (this) | S-02 `github-refresh-interval` | S-06 `gitlab-review-requests` |
| --- | --- | --- | --- |
| `src-tauri/src/github_refresh.rs` `refresh_round`, `GithubRefreshState` | check step, gate, `kept_full`, partition | replaces `REFRESH_INTERVAL` with the setting, sets `nextRefreshAtMs` | — |
| `src-tauri/src/github/mod.rs` `GithubSnapshot`, `next_snapshot`, `src/github/types.ts` | hidden fields | `refreshMinutes`/`nextRefreshAtMs`-style fields | — |
| `src/github/GithubView.tsx`, `useGithub.ts`, `api.ts` | row actions, toggle, flags, notices | "every N min — next try" in unavailable/error states | — |
| `src-tauri/src/settings.rs` + TS mirror | none expected | `githubRefresh.minutes` | maybe |
| `src-tauri/src/cli/mod.rs` | `GhUser`, `GhIssueView` variants, counts, guard shapes | — | new `glab api` variant, counts, guard shape |
| `src-tauri/src/bin/fake_cli.rs`, `tests/cli_runner.rs` | gh `api user` / `issue view` routes | — | glab review-request route |
| `src-tauri/src/hidden_issues/*`, `gitlab_refresh.rs` | rules generalised; under option B the GitLab SQL too | — | `gitlab_refresh.rs` gains a fetch; `hidden_issues` likely untouched |
| `src-tauri/src/lib.rs` `generate_handler!` | 3 GitHub commands | — | maybe |
| `store/schema.rs` | v15 (the wave's only migration) | none | none |
| `AGENTS.md` app-data list | GitHub hidden snapshots | — | — |

The `cli/mod.rs` conflicts with S-06 are textual (both bump `VARIANT_COUNT`,
`EXAMPLE_COUNT`, the `seen` array and add `ordinal` arms); whichever merges
second re-adds its numbers. Keeping the GitLab signatures of `hidden_issues`
stable keeps S-06's `gitlab_refresh.rs` edits apart from this slice.

## Code References

- `src-tauri/src/hidden_issues/rules.rs:9-11` - GitLab imports that bind the rules
- `src-tauri/src/hidden_issues/rules.rs:110-192` - check trigger, comment verdict, round plan
- `src-tauri/src/hidden_issues/mod.rs:94-131` - `hide` upsert with settle time
- `src-tauri/src/hidden_issues/mod.rs:173-241` - `apply_round`, guarded transitions, other-host prune
- `src-tauri/src/hidden_issues/round.rs:80-142` - `check_round` (no lock, stop and resume)
- `src-tauri/src/hidden_issues/round.rs:144-199` - `commit_round` (one write, unpartitioned on store error)
- `src-tauri/src/hidden_issues/commands.rs:20-81` - async commands via `blocking` + `act_on_issue`
- `src-tauri/src/hidden_issues/error.rs:10-21` - `HiddenIssueError`
- `src-tauri/src/gitlab_refresh.rs:9-16,86-110,140-196,550-660` - lock order, state, `act_on_issue`, gated commit
- `src-tauri/src/github_refresh.rs:54-58,163-196,198-226` - GitHub state, round, reconcile input
- `src-tauri/src/github/mod.rs:19-35,168-205,204-330` - `GithubIssue`, parser, snapshot
- `src-tauri/src/github/fetch.rs:15-46` - whole-source vs per-repo failures
- `src-tauri/src/cli/mod.rs:136-204,328-345,406-462` - `GithubRepo`, gh list args, `CliCall`
- `src-tauri/src/cli/mod.rs:529-760` - counts, `ordinal`, write flags, `check_read_only`
- `src-tauri/src/bin/fake_cli.rs:455-528,559` - gh fake routes (issue list only)
- `src-tauri/src/store/schema.rs:101-122,286-310` - v6 `hidden_issues`, v14, `validate`
- `src/gitlab/GitlabView.tsx:62,109-158,178,199,522,678-691` - GitLab hidden UI
- `src/gitlab/api.ts:25-54`, `src/gitlab/types.ts:8,31,37,123` - commands and TS mirrors
- `src/github/GithubView.tsx:34,73`, `src/github/types.ts:3-40` - GitHub view and types
- `src/components/ItemRow/ItemRow.tsx:12-13,76` - single-flag limit

## Architecture Insights

- "Thin skeleton, thick source": `source_refresh.rs` gives helpers (cell,
  publish, round tail); gating and partitioning stay in the source
  (`context/archive/2026-11-27-source-refresh-skeleton/plan.md:68-101`).
- Hidden state is computed in Rust only; the UI sends ids/references and shows
  `changed`/`hiddenAtMs` (AGENTS.md "Domain rules ... live in Rust").
- Records never hold display data; titles come from the live list. Logs carry
  kinds and counts only (`round.rs:1-9`, `github_refresh.rs:12-13`).
- Every CLI shape is an exact allowlist entry with an independently written
  validator in the guard test.

## Historical Context (from prior changes)

- `context/archive/2026-11-23-hide-and-resurface/plan.md` - S-03 design: no
  open/closed comparison (list is `state=opened`; plan-review F4), notes only
  for hidden listed issues with a moved `updated_at`, user from settings with
  "no new identity call" (supported for GitLab; superseded for GitHub by the
  2026-11-27 decision in `roadmap.md:112`), MRs excluded ("Only issues get
  Ukryj"; S-07 now reopens this).
- `context/archive/2026-11-23-github-assigned-issues/plan.md` - `gh.exe` on
  Windows (no WSL), one `issue list` per repo, `Truncated` at 500, per-repo vs
  whole-source failure split, "no hiding for GitHub issues (S-03 is GitLab-only)".
- `context/archive/2026-11-24-github-issue-labels/plan.md:43-47` - `labels`
  appended to the `--json` list; the arg tail is written in four places.
- `context/archive/2026-11-23-read-only-cli-runner/plan.md` - closed enum,
  guard allowlist, fake-cli conventions (`FAKE_CLI_SCENARIO`, call log,
  leak markers).
- `context/archive/2026-11-27-source-refresh-skeleton/` - S-01 needs its own
  gate and `kept_full` (plan.md:57, research.md:171-175).
- Migration history: v1-v14 (`store/schema.rs`), v14 = `gitlab_view_state`
  (S-05). Renumbering on merge is routine and recorded in the migration comment.

## Related Research

- `context/archive/2026-11-23-hide-and-resurface/research.md`
- `context/archive/2026-11-23-github-assigned-issues/research.md`
- `context/archive/2026-11-27-source-refresh-skeleton/research.md`

## Open Questions

1. **What is a GitHub "state change" while the issue stays on the open list?**
   Default (GitLab precedent): only leaving and re-joining the list, marked
   `returned`. Option: add `stateReason` to the list call's `--json` (a fifth
   copy of the arg tail to update) so a close and reopen between two rounds
   (`stateReason` becomes `REOPENED`) marks `status`. The `REOPENED` value
   comes from GitHub's documented enum and was not observed here. GitHub
   Projects "Status" (`projectItems`) is excluded by the user's rule unless
   the user says otherwise.
2. **Storage option A or B** (section 5). A keeps GitLab untouched in this
   wave; B leaves one table for every hidden item.
3. **Behaviour when `gh api user` fails** while checks are due: count them as
   failed, or treat all authors as others. Also whether to cache the login for
   the session.
4. **Ghost / missing authors in `gh issue view` comments:** parse shape not
   verified; the parser should treat a missing or empty `author.login` as
   someone else, not `ResponseInvalid`.
5. **GitHub view layout:** a "Pokaż ukryte (n)" toggle per repo panel or one for
   the view, in memory only (remembering it would need a store record, which
   is outside v15 unless the plan adds it), and how `ItemRow` shows two flags
   plus "ukryte DD.MM".
6. **Command key:** `reference` string vs `(repo, number)`; either is resolved
   against `kept_full`.
7. **Settle check:** keep `SETTLE_MS` for GitHub (one extra call per hide) or
   drop it; GitHub's `updatedAt` throttling is unverified.
8. **`GhUser` in `CliCall::ALL`:** joining it puts `gh api user` into the
   ignored real-CLI smoke test (`tests/cli_real.rs`); fine for read-only, but a
   plan choice.
