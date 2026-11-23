# Assigned GitHub Issues ("Projects (GitHub)") — Plan Brief

> Full plan: `context/changes/github-assigned-issues/plan.md`

## What & Why

Roadmap slice S-05 (FR-005, FR-015): a "Projekty (GitHub)" view with the open GitHub issues assigned to the user from the repositories listed in `settings.json`, read through the logged-in `gh.exe`. It also completes FR-011/FR-012 for GitHub, which the "To do" slice (S-07) deferred to here: GitHub issues can be picked into "Do zrobienia" and tick themselves when closed.

## Starting Point

The read-only CLI runner (F-02) already knows `gh`: logged-out on exit 4, calm "unavailable" on network errors, and a fake CLI for tests. The GitLab slice (S-01, S-07) is a complete pattern to copy: settings block, parse/fetch, refresh thread, tray source, view, "To do" reconcile. Nothing GitHub-specific exists yet, and `todo_items` only allows own and GitLab rows.

## Desired End State

The user adds `"github": { "repos": ["owner/name", …] }` to `settings.json`. A new nav entry lists their open assigned issues per repo and refreshes every 10 minutes and on demand. A bad repo shows its own error line. Offline is a calm notice. A logged-out or missing `gh` is red in the view and the tray. GitHub issues can be picked into "Do zrobienia" and tick themselves once they are no longer open and assigned.

## Key Decisions Made

| Decision                | Choice                                                     | Why (1 sentence)                                                                                             |
| ----------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Scope                   | View **and** "To do" integration (pick + auto-tick)        | Closes the must-have FR-011/FR-012 gap S-07 explicitly deferred to S-05.                                     |
| Fetch strategy          | One `gh issue list -R <repo> --assignee @me --state open` per repo | Exact live data on the core API, and a failure is attributable to one repo; search lags and has a 30/min limit. |
| One repo fails          | Other repos still show + an error line for that repo; tray red | One stale repo name must not hide every issue, and the failure stays visible, never an empty list.             |
| Whole-source failures   | Logged-out / missing `gh` stop the round and show an error; offline is calm | Every later call would fail the same way; offline matches FR-015's calm GitLab treatment.                 |
| GitHub unreachable      | Calm "GitHub niedostępny", tray green, tooltip note, no toast | Reuses the existing Unavailable path; offline evenings must not raise alarms.                               |
| Refresh                 | Background thread every 10 min + view open, refresh button, tray "check now" | "To do" auto-tick works without opening the view; matches the resolved 10-min default.                   |
| Repo names in argv      | Validated `GithubRepo` newtype passed with `-R`; guard learns one exact shape | Settings values must never act as flags (AGENTS.md hard rule, F-02 plan).                                   |
| "To do" storage         | Migration v7 rebuilds `todo_items` with separate `github_*` columns | SQLite cannot alter CHECKs; separate columns leave GitLab SQL untouched and avoid ref collisions.             |
| Reconcile scope         | Only repos that loaded successfully this round              | A failed or removed repo says nothing about its issues, so it must never tick entries.                       |
| Truncation              | `--limit 500`; hitting it is a per-repo error              | A cut-off list would make reconcile tick open issues.                                                        |

## Scope

**In scope:**
- `github.repos` settings block, `GithubRepo` newtype, `CliCall::GhAssignedIssues`, guard shape, fake-CLI scenarios
- GitHub parse/fetch, snapshot, 10-min refresh thread, `github_snapshot` / `refresh_github` commands
- `SourceId::Github` in the tray
- `src/github/` view and nav entry "Projekty (GitHub)"
- Migration v7, `TodoEntry::Github`, `add_todo_github_item`, `reconcile_github`, TodoView picker and banner

**Out of scope:**
- Any write to GitHub; pull requests; closed or unassigned issues; WSL `gh`
- A settings UI, a configurable interval, GitHub notifications, hiding or filtering GitHub issues
- Tracking the `gh` account identity for reconcile

## Architecture / Approach

`settings.json` → `load_github_settings` → `fetch_assigned` (one read-only `gh issue list` per repo through `CliRunner`) → `next_snapshot` → `GithubSnapshot` published as `github-changed` → `GithubView` / TodoView. The same round maps the snapshot to a tray source and, after a successful round, calls `reconcile_github` with only the repos that loaded. It is a straight twin of `gitlab_refresh.rs`, plus a per-repo result layer.

## Phases at a Glance

| Phase                               | What it delivers                                                 | Key risk                                                        |
| ----------------------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------- |
| 1. GitHub source backend            | Settings, newtype, read-only call, parse, per-repo fetch, tests  | Guard shape too loose or real `gh` JSON differing from the fake |
| 2. Refresh thread, health, commands | 10-min round, snapshot, GitHub in the tray                       | Wrong calm/red mapping (offline must stay silent)               |
| 3. "Projekty (GitHub)" view         | User-visible view with every source state                        | Empty list shown where a repo or the source failed              |
| 4. GitHub issues in "Do zrobienia"  | Migration v7, pick + auto-tick, TodoView integration             | Table rebuild losing or altering existing "To do" rows          |

**Prerequisites:** F-02 (done); `gh.exe` installed and logged in on Windows (confirmed 2026-11-23); local `main` (98 commits ahead of `origin/main`) as the base.
**Estimated effort:** ~3–4 sessions across 4 phases; phase 4 is the largest.

## Open Risks & Assumptions

- **Migration numbering:** S-04 (`new-assignment-notify`) may also add a migration. Whichever merges second renumbers its migration (precedent: v5, v6).
- **Account switch:** reconcile is per repo, not per `gh` account. Switching the logged-in `gh` account makes previously picked issues look unassigned, so they tick as closed in the source. This is accepted as rare.
- **Reassignment counts as closed:** an issue reassigned to someone else disappears from the list and ticks itself. That matches GitLab's behaviour.
- **Real output shape:** assumes `gh issue list --json number,title,url,updatedAt` returns one JSON array (checked against `gh` 2.105 help; manual step 1.4 confirms against the real output).

## Success Criteria (Summary)

- With repos configured, "Projekty (GitHub)" shows the user's open assigned issues per repo, and every failure (bad repo, offline, logged out, missing `gh`) is visible as the agreed calm or red state, never an empty list.
- GitHub issues picked into "Do zrobienia" tick themselves after being closed on GitHub, and existing "To do" data survives the v7 upgrade unchanged.
- All AGENTS.md pre-commit checks pass (`cargo test`, clippy, fmt, `npm run build`, lint, format).
