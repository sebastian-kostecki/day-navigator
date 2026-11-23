# Assigned GitHub Issues ("Projects (GitHub)") Implementation Plan

## Overview

Roadmap slice S-05 (FR-005, FR-015). The user opens a new "Projekty (GitHub)" view and sees the open GitHub issues assigned to them from the repositories listed in `settings.json`. Data comes from the logged-in `gh.exe` on Windows: read-only, one `gh issue list` call per repo, refreshed in the background every 10 minutes. A repo that fails shows its own error line while the other repos still show. An unreachable GitHub is a calm state. A logged-out or missing `gh` is an error in the view and in the tray. The slice also closes the gap S-07 left open (FR-011/FR-012): GitHub issues can be picked into "Do zrobienia", and an entry ticks itself when its issue is no longer open and assigned on GitHub.

## Current State Analysis

- **CLI runner (F-02):** `Tool::Gh` already exists. `program()` returns the bare `"gh"` and `Command::new` resolves it from PATH (`src-tauri/src/cli/mod.rs:17-31`). The only `gh` calls are `GhVersion` and `GhAuthStatus` (`cli/mod.rs:193-203`). Classification already handles gh: exit 4 → `LoggedOut`, network markers and HTTP 5xx → `Unavailable` (`src-tauri/src/cli/classify.rs:43-58`). The runner runs in the temp dir, sets `GH_PROMPT_DISABLED` and `GH_NO_UPDATE_NOTIFIER`, has a 20 s timeout, and maps a spawn `NotFound` to `ToolMissing` (`cli/runner.rs:33-43`, `:115`, `:132-138`). `CliError::message_pl` already has "GitHub niedostępny…" (`cli/mod.rs:319-321`).
- **Read-only guard:** `check_read_only` accepts only `version`/`--version`/`auth status` or the exact five-argument `glab api` shape (`cli/mod.rs:464-491`). `ordinal()` has no `_` arm and `VARIANT_COUNT`/`EXAMPLE_COUNT` are fixed (`cli/mod.rs:342-366`), so adding a variant breaks the build until the guard is updated. `BAD_CHARS` is `&|<>^%"\`;$` (`cli/mod.rs:414`), so `@me` and a comma-separated `--json` list pass.
- **Fake CLI:** `src-tauri/src/bin/fake_cli.rs` is gated behind the `test-support` feature, selected by `FAKE_CLI_SCENARIO` (`:295`) and injected via `CliRunner::with_programs(glab, gh, timeout, env)` (`runner.rs:76-88`). It has `gh-logged-out` and `gh-no-network` (`:348-361`) but no gh data scenario.
- **GitLab twin to mirror:**
  - Settings block loader `load_gitlab_settings` (`src-tauri/src/settings.rs:86-135`): one loader per top-level block, absent or `null` means `None`, inner struct uses `deny_unknown_fields`, errors are value-free, and the file is re-read every round.
  - Parse/fetch/snapshot in `src-tauri/src/gitlab/` (`mod.rs:180-400`, `fetch.rs:12-32`).
  - Refresh thread `src-tauri/src/gitlab_refresh.rs`: snapshot `RwLock`, wake channel, ordered `seq` publish, `refresh_round` at `:235-339`, interval `:38`, reconcile hook `:341-379`.
  - Health mapping `health::gitlab_status` (`src-tauri/src/health/probe.rs:81-99`), `SourceId` (`health/mod.rs:22-42`), wiring in `runtime.rs:107-117,161-164,247-261,290`.
  - UI folder `src/gitlab/` (`GitlabView.tsx`, `api.ts`, `types.ts`, `useGitlab.ts`) and the `VIEWS` list in `src/App.tsx:10-18`.
- **Nothing for GitHub yet:** no settings field, no data call, no health `SourceId`, no view, no "To do" variant (grep of `src/` and `src-tauri/src`).
- **"To do" store:** `todo_items` (migration v3, `src-tauri/src/store/schema.rs:46-80`) restricts `source IN ('own','gitlab')` and has a row-shape CHECK for exactly those two. SQLite cannot alter a CHECK, so GitHub rows need a table rebuild. Nothing references `todo_items` (it only references `own_tasks`), and each migration runs in one transaction (`store/mod.rs:123-135`). The last migration is v6 (`schema.rs:106-123`).
- **"To do" logic:** `TodoEntry` is `Own | Gitlab` (`src-tauri/src/todo/mod.rs:57-76`). `roll_over_in` hard-codes `source = 'gitlab'` (`:119`). `into_item` returns corrupt for an unknown source (`:185-206`). `add_gitlab` checks duplicates and the limit (`:316-357`). `reconcile_gitlab` ticks missing entries with `ticked_by='source'` and refreshes title and URL (`:421-490`). The command `add_todo_gitlab_item` takes the item from the live snapshot, never from UI text (`todo/commands.rs:36-50,102-118`). The TS mirror is in `src/todo/types.ts:5-38`, and `src/todo/TodoView.tsx` holds the GitLab-only picker and "unverified" banner (`:118-127`, `:237-264`).

## Desired End State

- `settings.json` accepts `"github": { "repos": ["owner/name", …] }`. Without it, the view says GitHub is not configured and shows how to add it.
- "Projekty (GitHub)" sits in the nav after "Praca (GitLab)". It lists open issues assigned to the logged-in `gh` account, grouped by repo, with the time of the last successful refresh and a refresh button. Each issue opens in the browser.
- A repo that fails (not found, no access, unexpected output, over 500 assigned issues) shows its own error line inside its group while the other repos still show. The tray shows the GitHub source as an error.
- No network, a GitHub 5xx, or a timeout gives a calm "GitHub niedostępny" with the last list greyed out. The tray stays green, the tooltip adds "(GitHub niedostępny)", and no notice appears.
- A logged-out `gh` (exit 4) or a missing `gh.exe` shows `ErrorState` in the view and a red tray icon with a Windows notice, through the existing health transitions.
- In "Do zrobienia" the user can pick GitHub issues (within the shared limit). A picked issue that disappears from its repo's successfully loaded list (closed, or no longer assigned) ticks itself as closed in the source and does not carry over. Entries from repos that failed, or that are no longer in settings, are never ticked by a refresh.
- Verify with `cargo test` (fake CLI, invented repos), `npm run build`, the manual steps below with the real `gh`, and an existing database upgraded from v6 to v7 with all "To do" rows intact.

### Key Discoveries:

- The read-only guard re-implements validators on purpose, so it does not trust production code. The new shape gets its own repo validator inside the test module (`cli/mod.rs:417-461`).
- Parameterless calls are listed in `CliCall::ALL` for the ignored real smoke test (`cli/mod.rs:207-212`, `tests/cli_real.rs`). The new call is parameterised, so it goes into `examples()` only (`:216-237`).
- GitLab health mapping: `Starting` returns `None`, so the source stays absent from the tray until the first round (`health/probe.rs:81-99`). Unavailable is silent in both directions (`health/mod.rs:137-157`).
- The GitLab refresh round wakes the health loop when its status changes (`gitlab_refresh.rs:334-338`). Tray "check now" wakes the GitLab thread (`runtime.rs:107-117,161-164`). GitHub needs the same two links.
- `useGitlab({ refreshOnMount: false })` is how TodoView reads a source without forcing a refresh (`TodoView.tsx:46`). `useGithub` copies this.
- The parallel change `new-assignment-notify` (S-04) has no plan yet but may also append a migration. Migrations were renumbered on merge before (v5 and v6, `schema.rs:90-91,104-105`).

## What We're NOT Doing

- No writes to GitHub: no closing, commenting, assigning or labelling (PRD Non-Goals). Ticking in "To do" never touches GitHub.
- No pull requests, only issues (FR-005). No unassigned issues, no closed issues, no repos outside the settings list.
- No WSL path for `gh`. `gh.exe` on Windows only (Open Roadmap Question #1). A missing binary is `ToolMissing`.
- No settings UI. `settings.json` stays hand-edited, as for GitLab and "To do".
- No configurable refresh interval: a fixed 10-minute constant. No GitHub notifications.
- No hiding/resurfacing for GitHub issues (S-03 is GitLab-only), and no visibility filter (S-02 is GitLab-only).
- No tracking of the `gh` account identity. A picked entry is reconciled by repo, not by account (see Open Risks in the brief).
- No reuse or renaming of the `gitlab_*` columns in `todo_items`. GitHub gets its own columns, so existing GitLab code paths keep their SQL.
- No keeping a failed repo's previous issues. A failed repo shows its error line only. Whole-source failures (unavailable, logged out, missing tool) keep the last list greyed, as GitLab does.

## Implementation Approach

Mirror the GitLab vertical slice step by step, with three GitHub-specific differences:

1. **Several repos per round.** Repos are fetched sequentially. Whole-source failures (`Unavailable`, `LoggedOut`, `ToolMissing`) stop the round, because every later call would fail the same way. Repo-specific failures (`Failed`, `ResponseInvalid`, too many results) become a per-repo error, and the round continues.
2. **Repo names come from settings.** They enter the argument list only through a validated `GithubRepo` newtype passed with `-R`. GitHub owners cannot start with `-`, so the value can never be read as a flag.
3. **Reconcile scope is per repo.** "To do" reconcile receives only the repos that loaded successfully in this round, so a failed or removed repo never ticks entries.

Phases go backend source → refresh/health → view → "To do". Each phase builds, passes its tests and ships something usable: phase 3 gives the user the view, and phase 4 adds the "To do" integration and the only store migration.

## Critical Implementation Details

- **Migration v7 is a table rebuild.** Create `todo_items_v7` with the widened CHECKs and the new `github_ref`, `github_title` and `github_url` columns. Copy every existing column by explicit name (including `gitlab_account` from v4), drop `todo_items`, rename the new table, then recreate `todo_items_live_own` and `todo_items_live_gitlab` exactly as in v3 and add `todo_items_live_github`. Everything goes in the one migration SQL string, which already runs in a single transaction. Do not edit v3/v4. If S-04 merges a migration first, renumber this one to the next free version.
- **Repo key normalisation.** GitHub owner and repo names are case-insensitive. Compare repos and store `github_ref` in lowercase (`owner/name#N`) so that `Octo/Repo` in settings and `octo/repo` in a stored entry are the same item. Display text can keep the case from settings.
- **Truncation guard.** `--limit 500`. A repo returning exactly 500 issues is a per-repo error ("lista niepełna"), never a list. Otherwise reconcile could tick entries that were just cut off.
- **Work-data rule still applies.** GitHub data is private, not work data, but tests use invented repos (`octo-test/alpha`, …), and logs record only call names and kinds (`gh:assigned-issues`), never repo names or titles, as the GitLab code does.

## Phase 1: GitHub source backend

### Overview

Everything needed to turn the settings repo list into parsed per-repo results through a read-only `gh` call, proven with the fake CLI. No UI, thread or store work yet.

### Changes Required:

#### 1. Repo newtype and CLI call

**File**: `src-tauri/src/cli/mod.rs`

**Intent**: Add a validated repository name and the one GitHub data call, keeping the call path fixed and read-only.

**Contract**:
- `pub struct GithubRepo` with `parse(&str) -> Result<GithubRepo, ()>`:
  - The whole value is `owner/name`.
  - Owner: 1–39 chars of `[A-Za-z0-9-]`, not starting or ending with `-`.
  - Name: 1–100 chars of `[A-Za-z0-9._-]`, not `.` or `..`.
  - Exactly one `/`, no whitespace.
- `GithubRepo` provides `as_str()` and a lowercase `key()`. Its `Debug` is opaque, like `GitlabHost`.
- `CliCall::GhAssignedIssues(GithubRepo)` → `tool() = Gh`, `name() = "gh:assigned-issues"`, `args()` exactly:
  `["issue","list","-R",<repo>,"--assignee","@me","--state","open","--limit","500","--json","number,title,url,updatedAt"]`.
- Expose the limit as a constant (`GH_ISSUE_LIMIT = 500`) used by the fetch's truncation check.
- `examples()` gains this variant with an invented repo. Update `VARIANT_COUNT`, `EXAMPLE_COUNT` and `ordinal()`.

#### 2. Read-only guard

**File**: `src-tauri/src/cli/mod.rs` (test module)

**Intent**: Teach the guard exactly one new allowed shape, so any other `gh` argument list still fails.

**Contract**:
- `check_read_only` accepts a third shape: the 12-argument list above, with position 3 checked by a guard-local repo validator (written out again, not calling `GithubRepo::parse`). Every other position must match the literal.
- Add guard tests:
  - A repo like `-X/foo` or `a/b c` is rejected.
  - A wrong `--state` value or an extra write flag is rejected.
  - A proptest showing that `GithubRepo::parse` never yields a value starting with `-` or containing a `BAD_CHARS` character.

#### 3. GitHub settings block

**File**: `src-tauri/src/settings.rs`

**Intent**: Read the repo list from its own block, so a mistake there never breaks the vault, GitLab or "To do" blocks.

**Contract**:
- `load_github_settings(path) -> Result<Option<GithubConfig>, GithubError>` reads top-level `"github"`.
- Absent, `null` or an empty `repos` gives `None` (not configured).
- Otherwise it parses `RawGithub { repos: Vec<String> }` with `deny_unknown_fields`, validates each entry with `GithubRepo::parse`, drops case-insensitive duplicates keeping the first, and caps the list at 50 repos.
- Any failure, including more than 50 repos, is a value-free `GithubError::SettingsInvalid`.
- `GithubConfig { repos: Vec<GithubRepo> }` lives in `cli/mod.rs` next to `GitlabConfig`.
- Tests mirror the GitLab block tests, including block independence.

#### 4. GitHub domain module

**File**: `src-tauri/src/github/mod.rs` (new), `src-tauri/src/github/fetch.rs` (new), `src-tauri/src/lib.rs` (`mod github;`)

**Intent**: Parse `gh issue list --json` output and run one fetch round over the configured repos with the agreed failure split.

**Contract**:
- `GithubIssue { repo: String, number: u64, reference: String, title: String, web_url: Option<String>, updated_at: String }`:
  - serde camelCase.
  - `reference` is `<repo.key()>#<number>`.
  - `web_url` is kept only when it starts with `https://github.com/<owner>/<name>/issues/` (case-insensitive owner/name).
  - Sorted by `updated_at` newest first, ties broken by number.
- `parse_issues(repo: &GithubRepo, stdout: &str) -> Result<Vec<GithubIssue>, GithubError>`:
  - Input is a single JSON array, not paginated pages.
  - Number 0, a non-RFC 3339 `updatedAt` or any serde error gives `ResponseInvalid`, with the serde error dropped.
- `GithubError` variants: `NotConfigured`, `SettingsInvalid`, `Unavailable`, `LoggedOut`, `ToolMissing`, `Failed { exit_code }`, `ResponseInvalid`, `Truncated`, `ConfigDirUnresolved`, `ThreadDown`.
  - `thiserror` + `Serialize` with `#[serde(tag = "kind", content = "detail")]`.
  - `From<CliError>`, `kind()`, `message_pl()` (Polish, no repo names).
- `RepoResult { repo: GithubRepo, outcome: Result<Vec<GithubIssue>, GithubError> }`.
- `fetch_assigned(runner: &CliRunner, config: &GithubConfig) -> Result<Vec<RepoResult>, GithubError>`:
  - Calls repos in settings order.
  - `Unavailable`, `LoggedOut` or `ToolMissing` from any call returns `Err` immediately (whole source).
  - `Failed`, `ResponseInvalid`, or a result count equal to `GH_ISSUE_LIMIT` (`Truncated`) is recorded on that repo, and the loop continues.

#### 5. Fake CLI gh data

**File**: `src-tauri/src/bin/fake_cli.rs`

**Intent**: Make every per-repo and whole-source state testable without a real account.

**Contract**:
- Route `issue list` calls on the `-R` value:
  - `octo-test/alpha` → 2 invented issues.
  - `octo-test/beta` → `[]`.
  - `octo-test/missing` → exit 1 with a "Could not resolve to a Repository" style stderr.
  - `octo-test/garbled` → invalid JSON.
  - `octo-test/huge` → 500 issues.
  - `octo-test/offline` → exit 1 with a network-error stderr (`dial tcp …`), classified `Unavailable`.
  - `octo-test/logged-out` → exit 4, classified `LoggedOut`.
- These run under a new `gh-issues` scenario. The existing `gh-logged-out` and `gh-no-network` cover whole-source failures on the first call.
- A general call log: when `FAKE_CLI_CALL_LOG` is set, every call (glab and gh) appends its argv as one line to that file. `FAKE_CLI_SCENARIO` is process-wide, so this is the only way to prove a later repo was never called.
- Invented data carries the existing leak markers, so leak tests can assert they never reach logs or errors.

#### 6. Tests

**File**: `src-tauri/tests/github_fetch.rs` (new), unit tests in `github/mod.rs`, `settings.rs`, `cli/mod.rs`, `tests/cli_runner.rs`

**Intent**: Prove parsing, the failure split and the exact argv.

**Contract**:
- `cli_runner.rs` asserts the exact argv for the new example.
- `github_fetch.rs` covers:
  - alpha + beta both OK.
  - alpha OK + missing → per-repo `Failed`.
  - garbled → per-repo `ResponseInvalid`.
  - huge → per-repo `Truncated`.
  - `[alpha, offline, beta]` and `[alpha, logged-out, beta]` → whole-source `Err` (`Unavailable` / `LoggedOut`). The `FAKE_CLI_CALL_LOG` contains alpha and the failing repo, never beta.
  - `gh-logged-out` and `gh-no-network` → whole-source `Err` on the first call.
  - A nonexistent gh path → `ToolMissing`.
- Unit tests cover URL checking, sort order, lowercase reference and `message_pl` containing no repo text.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the new guard, settings, parse and `github_fetch` tests
- `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/`
- `cargo fmt --check` passes in `src-tauri/`

#### Manual Verification:

- `gh issue list -R <one of your repos> --assignee @me --state open --limit 500 --json number,title,url,updatedAt` run by hand on Windows returns a JSON array that `parse_issues` accepts (confirms the real output shape)

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Refresh thread, health and commands

### Overview

A background GitHub round every 10 minutes, a snapshot the UI can read, and the GitHub source in the tray.

### Changes Required:

#### 1. Snapshot model

**File**: `src-tauri/src/github/mod.rs`

**Intent**: Hold what the view shows and decide what survives a failed round.

**Contract**:
- `GithubState` (tagged `state`): `starting | ok | notConfigured | unavailable { message } | error { kind, message }`.
- `RepoView { repo: String, issues: Vec<GithubIssue>, error: Option<RepoErrorView { kind, message }> }`.
- `GithubSnapshot { state, repos: Vec<RepoView>, last_success_ms, checked_at_ms, refreshing, seq }`, serde camelCase.
- `next_snapshot(prev, kept_config, config, outcome, now_ms)`:
  - A successful round (even with per-repo errors) replaces `repos` and sets `last_success_ms`.
  - Unavailable or a whole-source error keeps the previous `repos` and `last_success_ms` while the repo list is unchanged.
  - `NotConfigured`, `SettingsInvalid` or a changed repo list drops them.
- Unit tests mirror the GitLab `next_snapshot` tests.

#### 2. Refresh thread and commands

**File**: `src-tauri/src/github_refresh.rs` (new), `src-tauri/src/lib.rs`

**Intent**: Run a round now and every 10 minutes, coalesce wakes, and push ordered snapshots to the webview.

**Contract**:
- `GithubRefreshState` (snapshot `RwLock`, kept config, wake `Sender`, published seq); `manage(app)` and `start(app, settings_path, rx)` on a thread named `github-refresh`.
- `REFRESH_INTERVAL = 10 min`; event `"github-changed"`.
- Round order: set `refreshing` and publish → `load_github_settings` → `fetch_assigned` → `next_snapshot` → publish → wake health if the mapped health status changed.
- Commands: `github_snapshot() -> Result<GithubSnapshot, String>` and `refresh_github() -> Result<(), String>` (sends a wake only).
- In `lib.rs`: register both commands, `manage` before the health thread starts, `start` after `runtime::start`, mirroring `lib.rs:182-186`.

#### 3. Health source

**File**: `src-tauri/src/health/mod.rs`, `src-tauri/src/health/probe.rs`, `src-tauri/src/runtime.rs`

**Intent**: Show GitHub in the tray with a calm unavailable state and a red state for logged-out, missing tool or repo errors.

**Contract**:
- `SourceId::Github` (key `"github"`, label `"GitHub"`).
- `health::github_status(&GithubSnapshot) -> Option<SourceStatus>` maps:
  - `Starting` → `None`.
  - `unavailable` → `SourceState::Unavailable`.
  - `error` → `Error`.
  - `notConfigured` → the same state GitLab's `notConfigured` maps to.
  - `ok` with any repo `error` → `Error` with the fixed kind `RepoFailed` (never the per-repo kind, so the status and its transition stay stable while the set of failing repos or their kinds change) and a message without repo names: "Nie udało się pobrać N z M repozytoriów GitHub".
  - `ok` without repo errors → OK.
- `short_reason` covers the new kinds: `Truncated` → "lista niepełna", `RepoFailed` → "błąd repozytorium" (reuse existing ones for the rest).
- `runtime.rs`: `probe_round` and `check_round` take a new GitHub status parameter (after GitLab) and read the GitHub snapshot. The GitHub thread is woken from the same three places as GitLab: `check_health_now` (`runtime.rs:112`), the tray "check" item (`:162`) and the watchdog resume (`:347`).
- Tests:
  - A `github_snapshot_maps_to_a_source` test.
  - Unavailable is silent in both directions.
  - Logged-out → OK recovery is a transition.
  - A repo error is red.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the `next_snapshot`, health mapping and transition tests
- `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/`
- `cargo fmt --check` passes in `src-tauri/`

#### Manual Verification:

- With a `github.repos` block in `settings.json`, `npm run tauri dev` shows a "GitHub" entry in the tray status, and the app log shows `gh:assigned-issues` calls without repo names or titles
- Adding a nonexistent repo to the list turns the tray red with a GitHub reason; removing it turns it back to OK on the next round or "check now"

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: "Projekty (GitHub)" view

### Overview

The user-facing view, following the frontend conventions in `AGENTS.md` and the GitLab view.

### Changes Required:

#### 1. Feature folder

**File**: `src/github/types.ts`, `src/github/api.ts`, `src/github/useGithub.ts`, `src/github/GithubView.tsx` (all new)

**Intent**: Read and show the GitHub snapshot with every source state, and never show an empty list in place of a failure.

**Contract**:
- `types.ts` mirrors `GithubSnapshot`, `GithubState`, `RepoView` and `GithubIssue` (tagged unions on `state` / `kind`).
- `api.ts` is the only file that imports `invoke`: `githubSnapshot()`, `refreshGithub()` and `openGithubIssue(url)` (via the opener plugin, as in `src/gitlab/api.ts`).
- `useGithub({ refreshOnMount = true })`:
  - Subscribes to `github-changed`, then loads, and keeps the newer snapshot by `seq`.
  - Asks for one refresh on mount unless the state is starting or a refresh is running.
  - Exposes `loadError`, `refreshError` and `retry`.
- `GithubView` renders:
  - Load error → `ErrorState` with retry.
  - `starting` → "Pobieram…".
  - `notConfigured` → notice with the `"github": { "repos": ["właściciel/nazwa"] }` hint.
  - `unavailable` → calm `role="status"` notice with the last-success time, list greyed.
  - `error` → `ErrorState` (`title="Błąd GitHuba"`, retry = refresh, `lastSuccess`).
  - `ok` → one group per repo in settings order:
    - Repo name.
    - Its issues (`#number` + title, opening the URL), or "Brak przypisanych otwartych zgłoszeń" when empty, or the repo's error line.
  - A refresh button with "Odśwież"/"Odświeżam…" next to the last-success time.

#### 2. Navigation

**File**: `src/App.tsx`, `src/App.css`

**Intent**: Make the view reachable.

**Contract**:
- `View` gains `"github"`, and `VIEWS` gets `{ id: "github", label: "Projekty (GitHub)" }` right after GitLab. Render `<GithubView />`.
- Reuse the existing source-view CSS classes where they fit. Add only what the per-repo grouping needs.

### Success Criteria:

#### Automated Verification:

- `npm run build` finishes with zero errors
- `npm run lint` passes
- `npm run format:check` passes

#### Manual Verification:

- With real repos configured, "Projekty (GitHub)" lists your open assigned issues per repo, and clicking one opens it on github.com
- With the `github` block removed, the view shows the not-configured hint, not an empty list
- With network off (Wi-Fi disabled), refresh shows the calm "GitHub niedostępny" notice with the last-success time and the previous list greyed; the tray stays green
- A nonexistent repo in the list shows an error line in its own group while the other repos still list their issues
- (Optional) With `gh` temporarily off PATH, the view shows `ErrorState` and the tray turns red

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 4: GitHub issues in "Do zrobienia"

### Overview

Pick GitHub issues into "To do", and tick them automatically when they are no longer open and assigned in a repo that loaded successfully.

### Changes Required:

#### 1. Migration v7

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Allow GitHub rows in `todo_items` without touching the shipped migrations.

**Contract**:
- Append `Migration { version: 7, … }`, rebuilding `todo_items` (see Critical Implementation Details).
- New columns: `github_ref TEXT`, `github_title TEXT`, `github_url TEXT`.
- `source IN ('own','gitlab','github')`.
- The row CHECK gains a third branch: `source = 'github'` requires `github_ref` and `github_title` non-null and all `own_task_id`/`gitlab_*` null, with the same `(ticked_at IS NULL) = (ticked_by IS NULL)` rule.
- The own and gitlab branches additionally require all `github_*` to be null.
- New index `todo_items_live_github ON todo_items (github_ref) WHERE github_ref IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL`.
- A comment states the slice and the renumbering rule.

#### 2. "To do" domain

**File**: `src-tauri/src/todo/mod.rs`

**Intent**: Treat GitHub entries like GitLab ones: an own tick, a reference/title snapshot, and source ticking.

**Contract**:
- `TodoEntry::Github { reference: String, title: String, web_url: Option<String>, closed_in_source: bool }` (serde `kind: "github"`).
- `roll_over_in` closes ticked GitHub entries like GitLab ones (`source IN ('gitlab','github')`).
- `Row`/`read_live`/`into_item` read the `github_*` columns and add the `"github"` arm.
- `add_github(store, issue: &GithubIssue, today, now, limit) -> Result<i64, TodoError>`:
  - Rolls over first.
  - Returns `AlreadyOnList` if a live row has the same `github_ref`.
  - Checks the shared limit, then inserts.
- `reconcile_github(store, loaded: &[(String /* repo key */, &[GithubIssue])], today, now) -> Result<usize, TodoError>`:
  - Looks at open live GitHub entries whose `github_ref` repo part is in `loaded`.
  - Ticks (`ticked_by='source'`) those missing from that repo's issues.
  - Refreshes title and URL for those still present.
  - Leaves everything else alone.
- `set_ticked` needs no change: it is generic for non-own rows. Verify.

#### 3. Command and refresh hook

**File**: `src-tauri/src/todo/commands.rs`, `src-tauri/src/todo/mod.rs` (`TodoError`), `src-tauri/src/github_refresh.rs`, `src-tauri/src/lib.rs`

**Intent**: Pick from the live snapshot only, and reconcile after every successful round.

**Contract**:
- `add_todo_github_item(reference: String)` (async, `spawn_blocking` / `on_store` like the GitLab command) looks the issue up in the current `GithubSnapshot` repos by lowercase reference.
- Not found → new `TodoError::NotInGithubList`.
- Register the command in `lib.rs`.
- In the GitHub round, after an `Ok` outcome, call `reconcile_github` with only the repos whose outcome is `Ok`.
- Emit `"todo-changed"` when it changed rows.
- A store error is logged by kind and does not fail the GitHub snapshot (mirror `gitlab_refresh.rs:341-379`).

#### 4. UI

**File**: `src/todo/types.ts`, `src/todo/api.ts`, `src/todo/TodoView.tsx`

**Intent**: Let the user pick GitHub issues and see them on the list next to GitLab ones.

**Contract**:
- The `TodoEntry` union gains `{ kind: "github"; reference; title; webUrl; closedInSource }`. `TodoError` gains `NotInGithubList`, and `addTodoGithubItem(reference)` is added.
- TodoView:
  - Reads `useGithub({ refreshOnMount: false })`.
  - Adds a GitHub section to the picker, with issues from repos without errors and the same status texts as the GitLab section.
  - The `onList` picker key gets a source prefix (`own:…`, `gitlab:<itemKind>:<ref>`, `github:<ref>`), replacing `gitlabKey` (`TodoView.tsx:28-30`, `:112-114`). Today every non-own row is keyed without a source, so a GitHub `owner/repo#N` would collide with a GitLab reference.
  - Renders GitHub entries in every branch that today assumes "non-own = GitLab": the open link (`:132`), the kind badge (`:157-163`, "Zgłoszenie GitHub" instead of reading `itemKind`) and the closed-in-source badge (`:165-166`, "zamknięte w GitHubie"). The union has no `never` check, so a missed branch fails silently rather than at build time.
  - Extends the "unverified" banner so it also shows when an unticked GitHub entry exists and either GitHub is down or not loaded, or that entry's repo (the `reference` part before `#`) has an error in the snapshot or is missing from `snapshot.repos` (removed from settings). Such entries are never ticked by reconcile, so they must not look verified.

#### 5. Tests

**File**: `src-tauri/tests/todo.rs`, `src-tauri/tests/todo_crash.rs`, `src-tauri/tests/store.rs` (or the existing migration test location), unit tests in `todo/mod.rs`

**Intent**: Prove the migration keeps data and the GitHub rules hold.

**Contract**:
- **Existing tests to update:** `tests/todo.rs:71-78` (exact list of `todo_items` objects; add `todo_items_live_github`), `tests/hidden_issues.rs:112` (`user_version == 6`; compare against the v6 prefix or `MIGRATIONS.len()` instead), `tests/todo_crash.rs:168-177` (duplicate check on `gitlab_*`; add the GitHub twin).
- **Migration:** following the precedent `tests/hidden_issues.rs:96-123` (`Store::open(.., &MIGRATIONS[..6], ..)`, insert, reopen with the full list), a database built at v6 with own and GitLab rows (live, ticked, removed, closed, with and without `gitlab_account`) opens at v7 with every row and column value unchanged, and the old unique indexes still reject duplicates.
- **"To do":**
  - Add a GitHub entry, then add it again → `AlreadyOnList`.
  - The limit is shared across all three sources.
  - Reconcile ticks a missing issue only when its repo is in `loaded`.
  - A failed repo and a repo removed from settings never tick.
  - Title/URL refresh works.
  - A source-ticked entry rolls over to closed the next day.
  - A GitLab and a GitHub entry with similar refs (`a/b#1`) coexist.
- **Crash:** reuse `tests/common/crash.rs` for `add_github` and `reconcile_github`.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes in `src-tauri/`, including the v6→v7 migration, "To do" GitHub and crash tests
- `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/`
- `cargo fmt --check` passes in `src-tauri/`
- `npm run build` finishes with zero errors
- `npm run lint` passes
- `npm run format:check` passes

#### Manual Verification:

- Starting the app on your existing database upgrades it (a `backups/` copy exists from before), and today's "Do zrobienia" entries look exactly as before
- A GitHub issue picked in "Do zrobienia" shows with its link; closing that issue on github.com and pressing refresh in the GitHub view ticks it as closed in the source
- With network off, a picked GitHub entry stays unticked and the "unverified" banner appears

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- `GithubRepo::parse`: valid forms, a leading `-`, `.`/`..`, an extra `/`, whitespace, over-length parts, and a proptest that no accepted value starts with `-` or contains a `BAD_CHARS` character.
- Read-only guard: the new shape is accepted, every single-position mutation is rejected, and a write flag anywhere is rejected.
- `parse_issues`: valid, empty, zero number, bad timestamp, foreign URL dropped, sort order, lowercase reference.
- `next_snapshot`: success, per-repo errors, unavailable keeps the list, config change drops it.
- Health mapping and transitions for `SourceId::Github`.
- "To do": GitHub arm of `into_item`, roll-over, add and reconcile rules.

### Integration Tests:

- `tests/github_fetch.rs` with the fake CLI: every per-repo and whole-source state, plus the early stop after a whole-source failure.
- `tests/cli_runner.rs`: exact argv of `GhAssignedIssues`.
- `tests/todo.rs` and `tests/todo_crash.rs`: GitHub entries end to end on a temp store.
- Migration v6→v7 on a populated database.

### Manual Testing Steps:

1. Add `"github": { "repos": ["<your-owner>/<repo>"] }` to `settings.json`, then start `npm run tauri dev`.
2. Open "Projekty (GitHub)". Your open assigned issues are listed and clicking one opens github.com.
3. Add `"octo-nonexistent/none"` to the list. Its group shows an error line, the other groups are unchanged, and the tray is red with a GitHub reason.
4. Disable Wi-Fi and press refresh. The calm notice appears, the list is greyed, the tray stays green and no Windows notice appears.
5. Pick a GitHub issue into "Do zrobienia", close it on github.com, and refresh. It ticks as closed in the source and does not carry over tomorrow.

## Performance Considerations

One `gh issue list` per configured repo every 10 minutes, sequential, each capped by the runner's 20 s timeout. With a handful of repos this is a few short process spawns per interval, well within NFR "Lekkość w tle" (< 1% CPU, < 150 MB RAM idle). A whole-source failure stops after the first call, so being offline costs one call per round. The 50-repo cap bounds the worst case.

## Migration Notes

- Store migration v7 rebuilds `todo_items` in one transaction. On failure the store refuses to open (existing rule: a damaged database is refused, never reset), and the daily backup in `backups/` is the rollback (see AGENTS.md "Restoring").
- `settings.json` needs no migration. The `github` block is optional, and other blocks ignore it (`RawSettings` has no `deny_unknown_fields`).
- If S-04 (`new-assignment-notify`) merges a migration first, renumber this one to the next free version before merging, as was done for v5/v6.

## Implementation Notes

Recorded after the implementation review (`reviews/impl-review.md`). These are deliberate differences between the code and the plan text above:

- `check_round` takes no GitHub parameter. It reads the GitHub snapshot itself, as it already did for GitLab. Only `probe_round` gained the parameter.
- The snapshot command wrapper is `getGithubSnapshot()`, mirroring `getGitlabSnapshot()`.
- `GithubRepo::parse` returns a value-free `RepoError`, not `()`.
- `Truncated` triggers on `>= GH_ISSUE_LIMIT`.
- Migration v7's github row branch also requires `gitlab_account IS NULL` (review F1).
- `reconcile_github` keeps a stored URL when the fresh one was dropped (review F6). Its doc comment states the single-account assumption (review F3).

## References

- Roadmap slice: `context/foundation/roadmap.md` (S-05, Open Roadmap Question #1)
- PRD: `context/foundation/prd.md` (FR-005, FR-011, FR-012, FR-015)
- Archived F-02 plan (deferred S-05 notes): `context/archive/2026-11-23-read-only-cli-runner/plan.md:39-43,53,63`
- Archived S-07 brief (GitHub deferred): `context/archive/2026-11-23-todo-list/plan-brief.md:43,73`
- Similar implementation: `src-tauri/src/gitlab_refresh.rs:235-379`, `src-tauri/src/gitlab/mod.rs:180-400`, `src-tauri/src/settings.rs:86-135`, `src/gitlab/GitlabView.tsx`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: GitHub source backend

#### Automated

- [x] 1.1 `cargo test` passes in `src-tauri/`, including the new guard, settings, parse and `github_fetch` tests — 175dd4c
- [x] 1.2 `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/` — 175dd4c
- [x] 1.3 `cargo fmt --check` passes in `src-tauri/` — 175dd4c

#### Manual

- [ ] 1.4 Real `gh issue list` output on Windows is accepted by `parse_issues`

### Phase 2: Refresh thread, health and commands

#### Automated

- [x] 2.1 `cargo test` passes in `src-tauri/`, including the `next_snapshot`, health mapping and transition tests — fa3a507
- [x] 2.2 `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/` — fa3a507
- [x] 2.3 `cargo fmt --check` passes in `src-tauri/` — fa3a507

#### Manual

- [ ] 2.4 Tray shows a "GitHub" entry and the log shows `gh:assigned-issues` calls without repo names or titles
- [ ] 2.5 A nonexistent repo turns the tray red; removing it returns it to OK

### Phase 3: "Projekty (GitHub)" view

#### Automated

- [x] 3.1 `npm run build` finishes with zero errors — 81a74e9
- [x] 3.2 `npm run lint` passes — 81a74e9
- [x] 3.3 `npm run format:check` passes — 81a74e9

#### Manual

- [ ] 3.4 The view lists open assigned issues per repo and opens them on github.com
- [ ] 3.5 Without the `github` block the view shows the not-configured hint
- [ ] 3.6 With network off the view shows the calm unavailable notice and the tray stays green
- [ ] 3.7 A nonexistent repo shows its own error line while other repos still list issues
- [ ] 3.8 (Optional) With `gh` off PATH the view shows `ErrorState` and the tray turns red

### Phase 4: GitHub issues in "Do zrobienia"

#### Automated

- [x] 4.1 `cargo test` passes in `src-tauri/`, including the v6→v7 migration, "To do" GitHub and crash tests — 1c0f5ff
- [x] 4.2 `cargo clippy --all-targets -- -D warnings` passes in `src-tauri/` — 1c0f5ff
- [x] 4.3 `cargo fmt --check` passes in `src-tauri/` — 1c0f5ff
- [x] 4.4 `npm run build` finishes with zero errors — 1c0f5ff
- [x] 4.5 `npm run lint` passes — 1c0f5ff
- [x] 4.6 `npm run format:check` passes — 1c0f5ff

#### Manual

- [ ] 4.7 The existing database upgrades and today's "Do zrobienia" entries look as before
- [ ] 4.8 A picked GitHub issue closed on github.com ticks itself after refresh
- [ ] 4.9 With network off a picked GitHub entry stays unticked and the "unverified" banner appears
