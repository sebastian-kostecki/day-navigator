# Assigned GitLab issues and MRs ("Praca (GitLab)") Implementation Plan

## Overview

Roadmap slice S-01, the north star. The user opens the "Praca (GitLab)" view and sees the open merge requests and issues assigned to them in the work GitLab. Without VPN the view says "GitLab niedostępny" calmly and shows the last list from this session, marked stale, with the time of the last successful refresh. A logged-out `glab` is an error in the tray. The app also gets navigation between views and a fixed header (the work-time counter lands there in S-08).

## Current State Analysis

- F-02 is done: `src-tauri/src/cli/` runs `glab`/`gh` without a shell, with a 20 s limit, and classifies every failure as `Unavailable`, `LoggedOut`, `ToolMissing` or `Failed` (`cli/mod.rs:86-95`, `cli/classify.rs:43-58`). `CliCall` has only four parameterless calls (`version`, `auth status`), it is `Copy`, and it is listed in `CliCall::ALL` (`cli/mod.rs:41-55`). The runner runs the child in the system temp dir (`cli/runner.rs:115`).
- Outside a git checkout, `glab api` defaults to **gitlab.com** (`glab api --help`, `--hostname`: "Defaults to gitlab.com, or the authenticated host in the current Git directory"). On this Windows machine `glab` has exactly one configured host, gitlab.com, and is not logged in there (checked 2026-11-23 with hostnames masked). The app therefore needs the work hostname explicitly, and the user has to log `glab` in to the work GitLab on Windows once.
- `glab api <endpoint> --paginate` with the default `--output json` prints all pages as **one JSON array** (`glab api --help`). A GET with a query string in the endpoint needs no `-f`/`-F` fields, so it stays a GET.
- The read-only guard test forbids `&` and requires a `version` / `auth status` shape (`cli/mod.rs:169-197`). A query-string endpoint contains `&`, `?` and `=`, so the guard has to learn a third, exact shape.
- Source health knows `Ok`, `NotConfigured` and `Error` (`health/mod.rs:55-61`). Any `Error` turns the tray red and toasts (`health/mod.rs:121-141`, `:206-216`). FR-015 needs a calm state that does neither. `transitions()` already ignores non-`Error` states, so a new `Unavailable` state is silent by construction.
- The health check loop rebuilds the whole source list every 30 s in `probe_round` (`runtime.rs:239-250`). The watchdog detects resume from sleep and wakes the check loop (`runtime.rs:330-335`).
- `settings.json` has a top-level `spaces` object, and the top level does not deny unknown fields (`settings.rs:36-40`). A `gitlab` key is ignored today.
- `load_settings` is all-or-nothing (`settings.rs:104-122`), and its single result feeds the health probe (`probe.rs:17-22`), `VaultState` (`lib.rs:179`) and the task commands (`lib.rs:61`). An invalid GitLab host inside that parse would take the vaults down with it, so the `gitlab` block gets its own loader.
- `App.tsx` stacks the vault `TasksView` and `HealthPanel` with no navigation (`src/App.tsx:5-12`). `ErrorState` has no slot for the time of the last successful refresh (`src/components/ErrorState.tsx`).
- `opener:default` is already granted (`capabilities/default.json`), and it allows opening `https://*` URLs (tauri-plugin-opener `permissions/allow-default-urls.toml`). `@tauri-apps/plugin-opener` is already an npm dependency.
- `serde_json` error messages can quote values from the input (e.g. `invalid type: string "…"`), so a parse error must never be formatted into a message, error value or log.

## Desired End State

- `settings.json` accepts `"gitlab": { "host": "<hostname>" }`. The host is validated as a bare hostname and kept only in the user's app config dir.
- A background thread fetches the assigned open MRs and issues at startup, every 5 minutes, after a resume from sleep and on demand. It keeps the last successful result in memory only and publishes a snapshot to the webview (`gitlab-changed`) and to source health.
- The "Praca (GitLab)" view is the default view. It shows two sections, "Merge requesty" then "Zgłoszenia", each newest-updated first, with a count. A title opens the item in the default browser. Without VPN a calm notice replaces the alarm, and the last list stays visible, greyed, with "lista z HH:MM". After a restart without VPN it says "brak udanego odświeżenia od uruchomienia aplikacji".
- The tray shows GitLab as a source: green with the tooltip "Nawigator dnia — OK (GitLab niedostępny)" without VPN and no toast; red with a toast when `glab` is logged out, missing or fails.
- A header with navigation ("Praca (GitLab)", "Zadania (vault)", "Stan źródeł") stays at the top of the window.

Verify with `cargo test` (settings, guard, parsing, no-leak, snapshot rule, health rules, fake-CLI fetch), the full pre-commit gate from `AGENTS.md`, and a manual run with the VPN on, off and with `glab` logged out.

### Key Discoveries:

- Pure rules stay free of `tauri` (`health/mod.rs:1-2`, `cli/mod.rs:1-6`). The side effects (threads, tray, emit) live outside, as in `runtime.rs`.
- Background-thread pattern to copy: named thread, `mpsc` wake channel coalescing bursts, `publish` by `seq` and `emit` (`runtime.rs:174-191`, `:211-224`, `:252-271`).
- The webview pattern to copy: subscribe first, then load, order by `seq` (`src/health/useHealth.ts:17-56`).
- Error enum pattern: `thiserror::Error` + `Serialize`, `#[serde(tag = "kind", content = "detail")]` (`vault/error.rs:11-13`, `cli/mod.rs:84-95`).
- Async commands that touch blocking work go through `spawn_blocking` (`lib.rs:76-87`). The refresh thread runs blocking work directly, so its commands only read state or send a wake.
- Test-only API behind `#[cfg(any(test, feature = "test-support"))]`; the `fake-cli` bin with `FAKE_CLI_SCENARIO` (`src/bin/fake_cli.rs`, `tests/cli_runner.rs:12-20`).

## What We're NOT Doing

- No visibility filter (project, labels, milestone, state). That is S-02. S-01 shows open items only.
- No hiding or resurfacing items (S-03), no notifications for new assignments (S-04). The refresh loop is built so S-04 can reuse it, but it does not diff or notify.
- No MRs where the user is only a reviewer (PRD Non-Goals).
- No writes to GitLab. The only new calls are two `glab api` GETs.
- No persistence of the list or of the last-refresh time. Work data stays in memory and is gone after a restart.
- No port, scheme or path in `gitlab.host`. A GitLab on a non-standard port is out of scope.
- No settings UI. The user edits `settings.json` by hand, as for the vault paths.
- No change to the vault health sources or to the "no vault configured" error. That belongs to S-06 and roadmap Open Question #4.
- No partial lists: if one of the two calls fails, the round fails as a whole.
- No `gh` / GitHub work (S-05).

## Implementation Approach

The host is the only value from outside that reaches a `glab` argument, so it enters through a validated newtype inside a `CliCall` variant. The endpoints stay fixed constants. The guard test checks the exact shape `api <allowed endpoint> --paginate --hostname <valid host>`, and every other read-only rule stays as it is.

GitLab logic is split like health. `gitlab/` is pure: response parsing into a small item model, sorting, the browser-URL check, and the snapshot rule that keeps the last list across failures. It is unit-tested on invented JSON. A `fetch` function runs the two calls through `CliRunner` and is integration-tested with `fake-cli`. A tauri-side refresh module owns the thread, the managed state, the commands and the event, and hands a source status to the health loop.

Source health gets one calm state, `Unavailable`. It never makes `overall` an error, never produces a transition and adds a note to the green tooltip.

The UI gets one piece of view state in `App.tsx` and a `src/gitlab/` feature folder that follows the health pattern.

## Critical Implementation Details

**No work data in errors or logs.** A response that does not parse maps to a fixed `ResponseInvalid` with no detail. Never format the `serde_json::Error`, the response, an item, the host or a URL into an error value, a log line or a toast. The refresh thread logs only state kinds and elapsed time.

**Health and refresh threads do not block each other.** `probe_round` only reads the last GitLab snapshot (a cheap clone under a short lock). It never runs `glab`. When a refresh round changes the GitLab source status, the refresh thread sends `HealthState.wake` so the tray updates without waiting up to 30 s. Do not hold the GitLab snapshot lock across `emit` or tray calls (the locking rule at `runtime.rs:5-9`).

**Items belong to one host.** The kept list is dropped when the round sees no configured host, invalid settings, or a host different from the one the list came from. Otherwise a list from an old host would show as "stale" under the new one.

## Phase 1: GitLab backend contract

### Overview

Pure and process-level backend: the setting, the two calls and their guard, the item model and parsing, the snapshot rule, the fetch function, and fake-CLI scenarios. No thread, no command and no UI yet.

### Changes Required:

#### 1. GitLab host setting

**File**: `src-tauri/src/settings.rs`

**Intent**: Read an optional `gitlab.host` from `settings.json` so the app knows which GitLab to ask, without the hostname ever being in the repo. The `gitlab` block is parsed **separately** from the vault settings, so a GitLab typo never breaks the vaults and a vault typo never hides the GitLab list.

**Contract**:
- `Settings`, `load_settings` and `parse_settings` stay unchanged. `RawSettings` already ignores unknown top-level keys, so `load_settings` keeps tolerating the `gitlab` key; add a test that a file with a valid or an invalid `gitlab` block still loads its vault settings unchanged.
- New `pub fn load_gitlab_settings(path: &Path) -> Result<Option<GitlabHost>, GitlabError>` in `settings.rs`. It reads the file (BOM stripped, as `load_settings` does), checks `version` like `parse_settings`, and reads only the top-level `gitlab` key. A missing file, an absent `gitlab` or `null` → `Ok(None)` (not configured).
- Raw shape: a `gitlab` object with `#[serde(deny_unknown_fields)]` and one required field `host: String`.
- Every failure (unreadable file, invalid UTF-8, malformed JSON, bad `version`, `gitlab` of the wrong type, unknown field, invalid host) → `GitlabError::SettingsInvalid` with **no detail**. Never format the `serde_json::Error` or the value: `"gitlab": "gitlab.firma.example"` would otherwise be quoted by serde (`invalid type: string "…"`). `GitlabError::message_pl` gives a fixed text such as `Nieprawidłowy blok "gitlab" w settings.json — "host" musi być samą nazwą hosta, np. gitlab.example.com`.
- Unit tests (invented hosts only): a valid block parses; missing file, absent or `null` gives `None`; an unknown field, `gitlab` as a string, empty host, scheme (`https://…`), port, path, whitespace, leading `-`, and a single label without a dot are refused; for each refusal, the error's `Debug`, `Display`, JSON and `message_pl` do not contain the rejected value.

#### 2. Host newtype and the two GitLab calls

**File**: `src-tauri/src/cli/mod.rs`, `src-tauri/src/cli/runner.rs`

**Intent**: Add the two read-only `glab api` calls with the host as a validated parameter, keeping `CliCall` the single reviewed list of everything the app may run.

**Contract**:
- `pub struct GitlabHost(String)` with `GitlabHost::parse(&str) -> Result<GitlabHost, HostError>` and `as_str()`. Rules: 1–253 characters; dot-separated labels of 1–63 characters from `[a-z0-9-]`, each not starting or ending with `-`; at least two labels. The value is stored lowercased. The error carries no value. `Debug` prints `GitlabHost(..)`, so a host never reaches a log through `{:?}`.
- `CliCall` gains `GlabAssignedIssues(GitlabHost)` and `GlabAssignedMergeRequests(GitlabHost)`. It drops `Copy` (keeps `Clone`, `Debug`, `Eq`). `tool`, `args` and `name` take `&self`; `args` returns `Vec<&str>`.
- Fixed endpoint constants: `issues?scope=assigned_to_me&state=opened&per_page=100` and `merge_requests?scope=assigned_to_me&state=opened&per_page=100`. Arguments are exactly `["api", <endpoint>, "--paginate", "--hostname", <host>]`. Log names are `glab:assigned-issues` / `glab:assigned-mrs`.
- `CliCall::ALL` is replaced by a parameterless list for the existing calls (used by the real-CLI smoke test) plus a test-only `examples()` that builds every variant, the GitLab ones with `gitlab.example.invalid`.
- `CliRunner::run` takes `&CliCall`.
- Guard test update: a third allowed shape is exactly `api`, an endpoint from the fixed allow-list, `--paginate`, `--hostname`, a value that passes `GitlabHost::parse`. The forbidden-character check applies to every argument except the allow-listed endpoint. The write-flag check applies to all. The exhaustiveness test maps each variant (payload ignored) to an ordinal and checks that `examples()` covers each exactly once.

#### 3. GitLab item model, parsing and snapshot rule

**File**: `src-tauri/src/gitlab/mod.rs` (new), `src-tauri/src/lib.rs` (`pub mod gitlab;`)

**Intent**: Turn `glab api` output into the small, display-ready model the view needs, and hold the rule for what the view shows after each round. Pure code, no `tauri`, no processes.

**Contract**:
- `ItemKind { Issue, MergeRequest }`. `GitlabItem { kind, reference, title, web_url: Option<String>, updated_at, labels: Vec<String>, draft: bool }` (serde camelCase). `reference` comes from `references.full` (`group/project#12`, `group/project!34`). `updated_at` is kept as the RFC 3339 text from GitLab. `draft` comes from the MR `draft` field (default `false`; always `false` for issues).
- `AssignedItems { merge_requests: Vec<GitlabItem>, issues: Vec<GitlabItem> }`. Each list is sorted by `updated_at` descending (parsed as a timestamp), then by `reference` for ties.
- `parse_issues(json, host)` / `parse_merge_requests(json, host)` → `Result<Vec<GitlabItem>, GitlabError>`. Only the fields above are read; extra fields are ignored. A missing required field, wrong type, bad timestamp or non-array top level → `GitlabError::ResponseInvalid`.
- `web_url` is kept only if it starts with `https://<host>/` (case-insensitive host) and has no whitespace or control characters; otherwise `None`, so the row shows no link.
- `GitlabError` (`thiserror` + `Serialize`, `tag = "kind", content = "detail"`): `NotConfigured`, `SettingsInvalid`, `Unavailable`, `LoggedOut`, `ToolMissing`, `Failed { exit_code: Option<i32> }`, `ResponseInvalid`. `From<CliError>`. It has `kind()` and `message_pl()` with fixed Polish texts that never include stderr, the host or response content (reuse `CliError::message_pl` for the CLI variants).
- `GitlabSnapshot` (serde camelCase, sent to the webview): `state` (tagged `state`: `starting` | `ok` | `notConfigured` | `unavailable { message }` | `error { kind, message }`), `items: Option<AssignedItems>` (last successful list in this session), `lastSuccessMs: Option<u64>`, `checkedAtMs: Option<u64>`, `refreshing: bool`, `seq: u64`. The host is not part of it.
- The pure snapshot rule (signature up to the implementer) takes the previous snapshot, the host the kept items came from, this round's host (or none) and this round's outcome. It returns the next snapshot. Success replaces the items and sets `lastSuccessMs`. `Unavailable` and the error kinds keep the items and `lastSuccessMs` while the host is unchanged. `NotConfigured`, `SettingsInvalid` or a changed host drop them. `seq` always increases.
- Unit tests on invented JSON (`gitlab.example.invalid`, invented projects and titles): issue and MR parsing, `references.full`, draft, labels, sorting with ties, the URL check (other host, `http://`, `https://<host>.evil.invalid/`, look-alike prefix), empty arrays, and each invalid shape → `ResponseInvalid`. A no-leak test: for a malformed response containing a marker string, the error's `Debug`, `Display`, JSON and `message_pl` do not contain it. Snapshot-rule table: first success, failure after success keeps items, `Unavailable` keeps items, not configured drops them, host change drops them, `seq` increases.

#### 4. Fetch over the runner

**File**: `src-tauri/src/gitlab/fetch.rs` (new)

**Intent**: Run the two calls for one host and return either both lists or the first failure. It is the only GitLab code that touches processes, and it is blocking like the runner.

**Contract**: `pub fn fetch_assigned(runner: &CliRunner, host: &GitlabHost) -> Result<AssignedItems, GitlabError>`. It runs merge requests first, then issues; the first failure ends the round and the second call is not started. Output is parsed with the functions above. Nothing is logged here beyond what the runner logs.

#### 5. Fake CLI scenarios and integration tests

**File**: `src-tauri/src/bin/fake_cli.rs`, `src-tauri/tests/cli_runner.rs`, `src-tauri/tests/cli_real.rs`, `src-tauri/tests/gitlab_fetch.rs` (new)

**Intent**: Prove the exact arguments and every fetch outcome through a real process on invented data.

**Contract**:
- New scenarios, chosen by the endpoint argument: `gitlab-assigned` (invented MR and issue arrays, including a draft MR, labels, one foreign-host `web_url`), `gitlab-empty` (`[]` for both), `gitlab-bad-json` (a truncated array containing a marker title), `gitlab-mrs-ok-issues-no-network` (MRs succeed, issues fail with the no-network stderr).
- `cli_runner.rs`: the argv echo test iterates `CliCall::examples()`, so the GitLab calls are checked for exact arguments (`api`, endpoint, `--paginate`, `--hostname`, `gitlab.example.invalid`). The other tests use `&call`.
- `cli_real.rs`: iterates the parameterless list only (no real host in tests).
- `gitlab_fetch.rs`: `gitlab-assigned` → sorted lists with the foreign URL dropped; `gitlab-empty` → two empty lists; `gitlab-bad-json` → `ResponseInvalid` and no marker in any error representation; `glab-no-network` → `Unavailable`; `glab-logged-out` → `LoggedOut`; `gitlab-mrs-ok-issues-no-network` → `Unavailable`, no partial result.

### Success Criteria:

#### Automated Verification:

- Unit tests pass: `cargo test --lib` (in `src-tauri/`)
- Integration tests pass: `cargo test --test cli_runner --test gitlab_fetch`
- Format and lint are clean: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
- Full Rust suite passes: `cargo test`

#### Manual Verification:

- Reviewer confirms the two new argument lists are GET-only and the guard test would reject a write flag, a non-allow-listed endpoint or a host with a scheme

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Background refresh, source health and tray

### Overview

The refresh thread and its managed state, the two commands and the event, and the GitLab source in health with the calm `Unavailable` state. After this phase the tray reacts to GitLab, while the UI still shows only the health panel change.

### Changes Required:

#### 1. Calm `Unavailable` state and the GitLab source

**File**: `src-tauri/src/health/mod.rs`, `src-tauri/src/health/probe.rs`

**Intent**: Let a source be "unavailable" without being an error, as FR-015 requires, and register GitLab as a source.

**Contract**:
- `SourceId::Gitlab`, key `gitlab`, label `GitLab`.
- `SourceState::Unavailable { message: String }` serialises as `{"state":"unavailable","message":…}`. It is not an error: `overall` ignores it; `transitions` treats it like `NotConfigured` (entering it, leaving it to `Ok`, and `Error → Unavailable` are silent).
- `tooltip`: for `Overall::Ok` with any unavailable source, the text is `Nawigator dnia — OK (<label> niedostępny)` for one source, listing labels for more, within the existing 127-unit cap. Other states are unchanged.
- `short_reason` gains `LoggedOut` → `wylogowano`, `ToolMissing` → `brak programu`, `Failed` → `błąd wywołania`, `ResponseInvalid` → `nieczytelna odpowiedź`.
- `probe.rs`: `gitlab_status(&GitlabSnapshot) -> Option<SourceStatus>` maps `starting` → `None` (absent), `ok` → `Ok`, `notConfigured` → `NotConfigured`, `unavailable` → `Unavailable`, `error` → `Error { kind, message }`. Messages come from `GitlabError::message_pl` only.
- Unit tests: serialisation of `Unavailable`; `overall` stays `Ok`; no transition into or out of `Unavailable` (including from `Error`); `Error(LoggedOut) → Ok` is a recovery; tooltip texts; the snapshot→source mapping.

#### 2. Refresh thread, state, commands and event

**File**: `src-tauri/src/gitlab_refresh.rs` (new), `src-tauri/src/lib.rs`, `src-tauri/src/runtime.rs`

**Intent**: Own the GitLab side effects: one background thread that refreshes on start, every 5 minutes, on resume and on demand, and publishes the snapshot to the webview and to source health.

**Contract**:
- Managed `GitlabState`: the current `GitlabSnapshot` behind a lock, the host of the kept items, a wake `Sender<()>`, and a `published` seq guard like `HealthState`.
- Wiring order in `setup`: `gitlab_refresh::manage(app)` registers `GitlabState` and returns the wake receiver (the `build_tray` split), and it runs **before** `runtime::start`. The health thread starts inside `runtime::start` and reads `GitlabState` in its first round, and `app.state::<GitlabState>()` panics on unmanaged state. The thread itself starts after `runtime::start`, because it sends `HealthState.wake`.
- Thread `gitlab-refresh`. Each round: re-read the host with `load_gitlab_settings` (so a host change needs no restart; vault settings errors do not affect it); set `refreshing: true` and emit; run `fetch_assigned` with `CliRunner::system()` when a host is configured; apply the snapshot rule; store; emit `gitlab-changed`; if the derived source status (`gitlab_status`) changed, send `HealthState.wake`. Then wait on the wake channel with a 5-minute timeout, draining extra wakes (coalescing), like `check_loop`.
- Commands (async, registered in `generate_handler!`): `gitlab_snapshot() -> Result<GitlabSnapshot, String>` returns the stored snapshot; `refresh_gitlab() -> Result<(), String>` sends a wake and fails only if the thread is gone (`"Wątek odświeżania GitLaba nie działa"`).
- `runtime.rs`: `probe_round` gains a `gitlab: Option<SourceStatus>` parameter and appends it after the store status. `check_round` computes it with `gitlab_status(&app.state::<GitlabState>().snapshot())` (a clone under a short lock) and passes it in, so `probe_round` stays free of managed state. The watchdog's resume branch also wakes the GitLab thread.
- Logging: one `info` line when the state kind changes (`gitlab: <from> -> <to>`), one `debug` line per round with the kind and elapsed ms. Never the host, items, counts tied to names, or URLs.
- No new capability is needed (app commands need no permission; listening is already allowed).

#### 3. Health TS mirror

**File**: `src/health/types.ts`, `src/health/HealthPanel.tsx`

**Intent**: Keep the TypeScript mirror in step with the new Rust state, in the same commit.

**Contract**: `SourceState` gains `{ state: "unavailable"; message: string }`. `HealthPanel` shows it as `niedostępny` with its message, styled calm (not the error colour).

### Success Criteria:

#### Automated Verification:

- Unit tests pass: `cargo test --lib -- health gitlab`
- Full pre-commit gate passes: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- With VPN off and `gitlab.host` set, the tray stays green, its tooltip reads "Nawigator dnia — OK (GitLab niedostępny)", and no toast appears
- After `glab auth logout --hostname <work-host>` and "Sprawdź teraz", the tray turns red with a "GitLab: wylogowano" toast; logging back in brings a recovery toast and a green icon
- No console window flashes when the app runs `glab` (the check F-02 deferred to this slice)
- The app log contains GitLab state kinds only: no hostname, title, project path or URL

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Navigation, header and the "Praca (GitLab)" view

### Overview

The app gets one piece of view state and a fixed header with navigation. The GitLab feature folder renders every state of the snapshot.

### Changes Required:

#### 1. Navigation and fixed header

**File**: `src/App.tsx`, `src/App.css`

**Intent**: Replace the stacked views with a fixed header and one active view, per the frontend conventions in `AGENTS.md`.

**Contract**: `App` owns `view: "gitlab" | "tasks" | "health"` in one `useState`, default `"gitlab"`. A sticky header holds the navigation buttons "Praca (GitLab)", "Zadania (vault)" and "Stan źródeł" (the active one marked with `aria-current="page"`). A comment marks the spot where S-08 puts the work-time counter. No router.

#### 2. ErrorState with last successful refresh

**File**: `src/components/ErrorState.tsx`

**Intent**: Let every source view show the time of the last successful refresh with its error, as `AGENTS.md` requires.

**Contract**: An optional `lastSuccess?: string` prop, rendered as a line under the message when present. Existing callers are unchanged.

#### 3. GitLab feature folder

**File**: `src/gitlab/types.ts`, `src/gitlab/api.ts`, `src/gitlab/useGitlab.ts`, `src/gitlab/GitlabView.tsx` (all new)

**Intent**: Show the snapshot in every state without ever turning a failure into an empty list.

**Contract**:
- `types.ts`: mirrors `GitlabSnapshot`, `GitlabItem`, `AssignedItems` and the tagged `state`.
- `api.ts`: the only file with `invoke` (`getGitlabSnapshot`, `refreshGitlab`) and the opener call (`openGitlabItem(url)` via `openUrl` from `@tauri-apps/plugin-opener`).
- `useGitlab.ts`: subscribe to `gitlab-changed`, then load, keep the newer snapshot by `seq` (the `useHealth` pattern). It exposes `snapshot`, `loadError`, `refresh` and `refreshError`. On mount it requests one refresh only when the loaded snapshot is neither `starting` nor `refreshing`. At app start the thread's first round already runs, so this avoids a second back-to-back round. Switching back to the view still refreshes.
- `GitlabView.tsx` states:
  - Load failed → `ErrorState` with retry.
  - `starting` with no items → "Pobieram przypisane z GitLaba…".
  - `notConfigured` → an explicit message: GitLab is not configured; add `"gitlab": { "host": "…" }` to `settings.json`.
  - `unavailable` → a calm notice (not `role="alert"`, not the error colour): "GitLab niedostępny (brak VPN lub sieci) — lista z HH:MM" or, with no success this session, "— brak udanego odświeżenia od uruchomienia aplikacji". The kept list follows, greyed.
  - `error` → `ErrorState` (title "Błąd GitLaba", the snapshot message, retry = refresh, `lastSuccess`) with the kept list greyed below.
  - `ok` → two sections, "Merge requesty (N)" then "Zgłoszenia (N)". An empty section says "Brak przypisanych merge requestów." / "Brak przypisanych zgłoszeń.".
  - A row shows the reference, the title (a link-styled button calling `openGitlabItem` when `webUrl` is present, plain text otherwise), a "Draft" badge for draft MRs, labels, and the update time (`pl-PL`).
  - A footer with "Odśwież" (disabled and labelled "Odświeżam…" while `refreshing`) and "Ostatnie udane odświeżenie: HH:MM".

### Success Criteria:

#### Automated Verification:

- Frontend gate passes: `npm run lint`, `npm run format:check`, `npm run build`
- Full Rust suite still passes: `cargo test`

#### Manual Verification:

- With VPN on, the app opens on "Praca (GitLab)" and lists the assigned open MRs and issues in two sections, newest updated first, matching the GitLab web UI
- Clicking a title opens the item in the default browser
- Turning the VPN off and clicking "Odśwież" shows the calm notice with "lista z HH:MM" and the greyed list, with no alarm
- Restarting the app without VPN shows "brak udanego odświeżenia od uruchomienia aplikacji" and no list
- With `glab` logged out, the view shows the "Błąd GitLaba" error with the last successful time
- Navigation switches between all three views and the header stays visible while scrolling

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Settings: `load_gitlab_settings` on its own (host validation, every refusal without the value, `gitlab` as a string), and `load_settings` still loading vault settings with a valid or invalid `gitlab` block present.
- CLI: the read-only guard with the third shape, exhaustiveness over `examples()`, `GitlabHost` validation and its opaque `Debug`.
- GitLab: parsing, sorting, the URL check, `ResponseInvalid` for every bad shape, no-leak of response content, and the snapshot-rule table.
- Health: `Unavailable` serialisation, silence in `transitions`, `overall`, tooltip texts, new `short_reason` entries, the snapshot→source mapping.

### Integration Tests:

- `cli_runner.rs`: exact argv for every `CliCall::examples()` entry through the real fake process.
- `gitlab_fetch.rs`: data, empty, bad JSON (no leak), no network, logged out, and a failure on the second call with no partial result.

### Manual Testing Steps:

1. One-time setup: with VPN on, run `glab auth login --hostname <work-host>` on Windows, then add `"gitlab": { "host": "<work-host>" }` to `settings.json` in the app config dir.
2. Start `npm run tauri dev` and compare the lists with the GitLab web UI; open one item from the list.
3. Turn the VPN off, click "Odśwież": calm notice, greyed list, green tray with the tooltip note, no toast.
4. Restart the app without VPN: no list, "brak udanego odświeżenia od uruchomienia aplikacji".
5. Log `glab` out of the work host and click "Sprawdź teraz" in the tray: red icon and a toast; log back in: recovery.
6. Check the app log for any hostname, title or URL (there must be none) and watch for a console window flash during refreshes.

## Performance Considerations

At idle the refresh thread sleeps on its channel. Every 5 minutes it starts at most two short `glab` processes; without VPN the first call usually fails fast (DNS) and the second is skipped, and the 20 s runner limit bounds the worst case. The list lives in memory only and is small (tens of items). `--paginate` with `per_page=100` keeps page count low; the runner's 16 MiB stdout cap bounds memory.

## Migration Notes

None for data. `settings.json` gains an optional `gitlab` block; existing files without it keep working and show GitLab as not configured.

## References

- Roadmap item: `context/foundation/roadmap.md` (S-01: Assigned GitLab issues and MRs in "Work (GitLab)")
- PRD: `context/foundation/prd.md` — FR-001, FR-015, FR-016, US-01, NFR "Nigdy pusta lista zamiast błędu", NFR "Dane pracy i vaulty", NFR "Lekkość w tle"
- F-02 plan and runner: `context/archive/2026-11-23-read-only-cli-runner/plan.md`; `src-tauri/src/cli/`
- Health rules and loop: `src-tauri/src/health/mod.rs:55-141`, `src-tauri/src/runtime.rs:211-335`
- Webview subscription pattern: `src/health/useHealth.ts:17-56`
- Settings loader: `src-tauri/src/settings.rs:36-122`
- Opener default URLs: tauri-plugin-opener `permissions/allow-default-urls.toml`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: GitLab backend contract

#### Automated

- [x] 1.1 Unit tests pass: `cargo test --lib` — f305e16
- [x] 1.2 Integration tests pass: `cargo test --test cli_runner --test gitlab_fetch` — f305e16
- [x] 1.3 Format and lint are clean: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` — f305e16
- [x] 1.4 Full Rust suite passes: `cargo test` — f305e16

#### Manual

- [ ] 1.5 Reviewer confirms the two new argument lists are GET-only and the guard test would reject a write flag, a non-allow-listed endpoint or a host with a scheme

### Phase 2: Background refresh, source health and tray

#### Automated

- [x] 2.1 Unit tests pass: `cargo test --lib -- health gitlab` — 5bef725
- [x] 2.2 Full pre-commit gate passes — 5bef725

#### Manual

- [ ] 2.3 With VPN off and `gitlab.host` set, the tray stays green with the "GitLab niedostępny" tooltip and no toast
- [ ] 2.4 A logged-out `glab` turns the tray red with a toast; logging back in recovers
- [ ] 2.5 No console window flashes when the app runs `glab`
- [ ] 2.6 The app log contains GitLab state kinds only: no hostname, title, project path or URL

### Phase 3: Navigation, header and the "Praca (GitLab)" view

#### Automated

- [x] 3.1 Frontend gate passes: `npm run lint`, `npm run format:check`, `npm run build` — 12ecb4a
- [x] 3.2 Full Rust suite still passes: `cargo test` — 12ecb4a

#### Manual

- [ ] 3.3 With VPN on, the view lists assigned open MRs and issues in two sections, newest first, matching GitLab
- [ ] 3.4 Clicking a title opens the item in the default browser
- [ ] 3.5 VPN off + "Odśwież" shows the calm notice with "lista z HH:MM" and the greyed list
- [ ] 3.6 Restart without VPN shows "brak udanego odświeżenia od uruchomienia aplikacji" and no list
- [ ] 3.7 A logged-out `glab` shows "Błąd GitLaba" with the last successful time
- [ ] 3.8 Navigation switches between all three views and the header stays visible
