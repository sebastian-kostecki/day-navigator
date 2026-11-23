<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Assigned GitLab issues and MRs ("Praca (GitLab)")

- **Plan**: context/changes/gitlab-assigned-list/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 6 observations

Commits reviewed: f305e16 (p1), 5bef725 (p2), 12ecb4a (p3), b06ac80 (epilogue), e52ba30 (unplanned `gitlab.user`).
Automated checks re-run on HEAD: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (159 unit tests plus all integration suites), `npm run lint`, `npm run format:check`, `npm run build`. All pass.

## Triage

All findings resolved on 2026-11-23 by applying each recommended fix (user: "fix yourself by recommendation"). Fixed: F1–F4, F6–F9 (8). Accepted: F5 (1). The gate was run again after the fixes: lint, format:check, build, fmt, clippy and cargo test (162 unit tests plus integration) all pass.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | WARNING |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

## Findings

### F1 — Unplanned `gitlab.user` filter left the plan out of date

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Scope Discipline
- **Location**: src-tauri/src/cli/mod.rs:83-165, src-tauri/src/settings.rs:108-118, src-tauri/src/gitlab/mod.rs:236-290
- **Detail**: Commit e52ba30 came after the epilogue. It adds an optional `"user"` to the `gitlab` block (`GitlabUser` newtype, `GitlabConfig { host, user }`). With a user set, the two calls switch to `scope=all&assignee_username=<login>`. Several plan contracts no longer describe the code:
  - `load_gitlab_settings` returns `GitlabConfig`, not `GitlabHost`.
  - The `CliCall` variants carry `(GitlabHost, Option<GitlabUser>)`.
  - `args()` returns `Vec<String>`.
  - `examples()` has 8 entries, and the exhaustiveness test now expects counts `[1,1,1,1,2,2]`.
  - `fetch_assigned` takes `&GitlabConfig`.
  - The snapshot rule keys kept items on (host, user).

  Neither the plan, the brief, nor the Desired End State mentions any of this.

  The change itself is sound: read-only, opaque `Debug`, no-leak tests, and the guard allows only the fixed form. It does change behaviour: with a bot or group token, the list shows only items visible to that token, which can differ from "everything assigned to me".
- **Fix**: Add an addendum to plan.md. It documents the `user` field, the `scope=all&assignee_username` form, the new types and signatures, the (host, user) keying, and the visibility caveat. It also updates Desired End State line 23.
  - Strength: Keeps the plan as the source of truth for S-02/S-03/S-04, which build on these types. The code is already tested and safe.
  - Tradeoff: The plan changes after it was closed.
  - Confidence: HIGH — every drift item has a file:line in the code.
  - Blind spot: Not checked whether the roadmap or PRD should record the bot-token visibility limit as an open question.
- **Decision**: FIXED — Addendum A in plan.md documents gitlab.user, the new types and signatures, (host, user) keying and the visibility caveat; Desired End State updated.

### F2 — Tray "Sprawdź teraz" does not refresh GitLab

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/runtime.rs:155-157, src-tauri/src/runtime.rs:106-111
- **Detail**: The tray "check" menu item and `check_health_now` wake only `HealthState`, and the health round only reads the cached GitLab snapshot. A logged-out `glab` therefore reaches the tray only after the next 5-minute round, an "Odśwież", or a remount of the GitLab view. Manual check 2.4 (and testing step 5, "Sprawdź teraz" → red) is ticked, but the code cannot do that through "Sprawdź teraz". The plan's contract never asked for this wake, so the gap is in the plan itself.
- **Fix**: In the tray `"check"` handler and in `check_health_now`, also call `app.state::<GitlabState>().wake()`, as the watchdog resume branch already does (runtime.rs:340).
- **Decision**: FIXED — tray "check" and check_health_now also wake the GitLab thread (runtime.rs).

### F3 — Read-only guard validates the login with the production validator

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/mod.rs:368-377, src-tauri/src/cli/mod.rs:399-403
- **Detail**: `endpoint_allowed` accepts the user form when `GitlabUser::parse(login).is_ok()`. The endpoint argument is also exempt from the `BAD_CHARS` check. If `GitlabUser::parse` ever got wider, the guard would accept the change silently; it is not an independent check. The bad-login test lists do not cover `#`, `:` (glab expands `:id`/`:user` placeholders in endpoints) or `+` (decoded as a space). The current code is safe, because the login charset is `[A-Za-z0-9._-]`.
- **Fix**: In the guard, check the login against a byte allow-list written out in the test, independent of `GitlabUser::parse`. Assert the endpoint has exactly one `?`, the expected number of `&`, and none of `# % : / +` or whitespace. Add `#`, `:` and `+` to the bad-login cases.
- **Decision**: FIXED — guard checks the login with its own allow-list plus endpoint shape (one ?, three &, no # % : / + whitespace/control); bad-login lists extended; deterministic ASCII sweep + proptest. A deliberately loosened validator (allowing +) fails all three tests.

### F4 — Failed refresh-thread spawn leaves an endless spinner and a green tray

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:102-110, src/gitlab/GitlabView.tsx:100
- **Detail**: When `start()` cannot spawn the thread, it only logs. The snapshot stays `Starting` forever:
  - The view shows "Pobieram przypisane z GitLaba…" indefinitely.
  - `gitlab_status` maps `Starting` to `None`, so the tray says OK.
  - `useGitlab` never calls refresh while the state is `starting`.

  This goes against the "surface the failure" rule. It is unlikely, and the release profile uses `panic = "abort"`, so a mid-round panic ends the whole app rather than leaving `refreshing: true` stuck.
- **Fix**: On spawn failure, store and publish an `Error` snapshot (e.g. a fixed "Wątek odświeżania GitLaba nie działa"), so the view and the tray show it.
- **Decision**: FIXED — spawn failure stores and publishes an error/ThreadDown snapshot and wakes health (new GitlabError::ThreadDown, short_reason entry).

### F5 — A paginated fetch that times out shows as calm "no VPN"

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/runner.rs:158-160
- **Detail**: When a call runs past the 20 s runner timeout, it becomes `CliError::Unavailable`, and the view shows the calm "GitLab niedostępny (brak VPN lub sieci)". That case gets no tray error and no toast. A list that is always too slow (many pages, slow server) would never surface as a fault. This behaviour comes from F-02; `--paginate` makes it more reachable.
- **Fix**: Accept it and document it in the plan's Performance Considerations, or give timeouts their own kind in a later slice.
- **Decision**: ACCEPTED — documented as a known limit in plan.md Performance Considerations.

### F6 — Unresolved config dir reported as an invalid "gitlab" block

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:144-146
- **Detail**: When `settings_path` is `None` (the app config dir cannot be resolved), the round returns `GitlabError::SettingsInvalid`. Its `message_pl` blames the `"gitlab"` block in settings.json, which is not the cause.
- **Fix**: Use a distinct variant or message for "config dir unresolved", reusing the health wording.
- **Decision**: FIXED — new GitlabError::ConfigDirUnresolved with its own message and short_reason.

### F7 — Two types named `GitlabState`; stale `kept_host`

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/gitlab/mod.rs:202, src-tauri/src/gitlab_refresh.rs:31, src-tauri/src/gitlab_refresh.rs:34
- **Detail**: `gitlab::GitlabState` (the snapshot state enum) and `gitlab_refresh::GitlabState` (the managed Tauri state) share a name. `runtime.rs` and `health/probe.rs` import different ones, and tests alias one of them. Since e52ba30, `kept_host` holds a whole `GitlabConfig`.
- **Fix**: Rename the managed struct (e.g. `GitlabRefreshState`) and `kept_host` → `kept_config`.
- **Decision**: FIXED — managed struct renamed GitlabRefreshState; kept_host → kept_config.

### F8 — Last-success time shown twice in the error state

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/GitlabView.tsx:141, src/gitlab/GitlabView.tsx:158-162
- **Detail**: In the `error` state, `ErrorState` renders `lastSuccess`, and the footer also renders "Ostatnie udane odświeżenie: HH:MM".
- **Fix**: Hide the footer line when `state.state === "error"`.
- **Decision**: FIXED — footer last-success line hidden in the error state.

### F9 — `opener:default` is broader than the capability description says

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/capabilities/default.json:4-9
- **Detail**: This existing grant was not changed here, but this slice is its first real user. `opener:default` allows any http, https, mailto or tel URL and reveal-in-dir. The description still says the window only listens to app events. The Rust-side `checked_url` (`gitlab/mod.rs:145-155`) is the only thing limiting links to `https://<host>/`.
- **Fix**: Narrow the grant to `opener:allow-open-url` scoped to `https://*` and update the description.
- **Decision**: FIXED — capability grants only opener:allow-open-url scoped to https://*; description updated.
