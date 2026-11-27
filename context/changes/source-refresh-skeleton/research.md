---
date: 2026-11-27T11:57:17+01:00
researcher: Claude (10x-new-research, non-interactive agent run)
git_commit: 2b6ac0861b564e2e096afc50f39e343f1fa41d11
branch: slice/source-refresh-skeleton
repository: day-navigator
topic: "F-01 shared source refresh skeleton for GitLab and GitHub (MS-28, issue #28)"
tags: [research, codebase, gitlab_refresh, github_refresh, runtime, refresh-skeleton]
status: complete
last_updated: 2026-11-27
last_updated_by: Claude (10x-new-research, non-interactive agent run)
---

# Research: F-01 shared source refresh skeleton

**Date**: 2026-11-27T11:57:17+01:00
**Researcher**: Claude (non-interactive agent run; the owner could not be asked)
**Git Commit**: 2b6ac0861b564e2e096afc50f39e343f1fa41d11
**Branch**: slice/source-refresh-skeleton
**Repository**: day-navigator

All paths are relative to the worktree root. Line numbers refer to the commit above.

## Research Question

Roadmap M-4, F-01 (anchor MS-28, GitHub issue #28): GitLab and GitHub should refresh through one shared skeleton: a thread with a `ThreadDown` stand-in, the interval, wake after sleep, "Check now", snapshot publication, `lastSuccessMs` and `nextRefreshAtMs`. The source-specific fetch and rules stay in `src-tauri/src/gitlab_refresh.rs` and `src-tauri/src/github_refresh.rs`. Behaviour, Tauri commands and TS types do not change, and existing test bodies stay as they are (tests may only move or change imports).

The research maps these points:

- the duplicated parts;
- the mutexes in GitLab's `refresh_round`;
- wake after sleep;
- "Check now";
- snapshot and event publication;
- "To do" reconciliation;
- the test → private-item dependencies;
- what S-01, S-02, S-03 and S-06 need from the skeleton.

## Summary

- **Duplication:** the two modules repeat the same eleven building blocks almost line for line (table below). They differ only in the snapshot/config/error types, the event name, the log prefix and the extra GitLab steps. GitHub's whole non-test part (`github_refresh.rs:1-278`) is either a copy or source glue. GitLab's `refresh_round` (`gitlab_refresh.rs:535-737`, about 200 lines) wraps the same frame (refreshing flip → fetch → `next_snapshot` → reconcile → store → publish → todo emit → logs → health wake) around source-only steps: hidden checks, the closed section, the seen set, the gate, NEW flags and the kept-list reshape.
- **Three items the roadmap lists sit outside the refresh modules today:**
  - Wake after sleep is detected in the `runtime.rs` watchdog (`runtime.rs:509-516`), which calls each state's public `wake()`.
  - "Check now" is that same `wake()`, called from three places: the `refresh_gitlab`/`refresh_github` commands, the tray "check" menu item (`runtime.rs:261-264`) and `check_health_now` (`runtime.rs:127-139`).
  - `last_success_ms` is set by the tauri-free rules function `next_snapshot` (`gitlab/mod.rs:551-635`, `github/mod.rs:284-330`).
- **`nextRefreshAtMs` and the interval:** today only GitLab has them. `GitlabSnapshot` has `refresh_minutes` and `next_refresh_at_ms` (`gitlab/mod.rs:500-502`), and the round sets `next_refresh_at_ms` at `gitlab_refresh.rs:711`. GitHub uses a constant `REFRESH_INTERVAL` of 10 min (`github_refresh.rs:31`), and `GithubSnapshot` has no such fields (`github/mod.rs:234-242`). Adding them would change the TS contract, so they belong to S-02, not F-01.
- **Mutexes:** `refresh_round` locks six distinct `Mutex`es directly: `hidden_gate`, `kept_config`, `kept_full`, `kept_filter`, `kept_new`, `shown_until` (`gitlab_refresh.rs:594, 641-645, 671-674, 703`). It reaches two more indirectly, `last_mark` via `accept_round` (`:203`) and `published` via `publish` (`:881`), plus the snapshot `RwLock`. `GitlabRefreshState` holds 8 `Mutex`es and 1 `RwLock` in total (`gitlab_refresh.rs:70-96`).
- **Test constraint:** no integration test in `src-tauri/tests/` references either module (grep: zero hits). The 36 unit tests (33 in `gitlab_refresh.rs`, 3 in `github_refresh.rs`) use `use super::*` and need these names in each module's scope:
  - the private constructor `new() -> (Self, mpsc::Receiver<()>)`, `snapshot()`, `wake()`, the private `write()` guard;
  - GitLab's private fields `kept_config`, `kept_full`, `kept_filter`, `kept_new`, `shown_until`, `last_mark` as `Mutex<…>`;
  - the free functions `state_label`, `reconcile_input`, `notify_input`, `filter_shown`, `shape_items`, `reshape_kept`, `load_filter`, `refresh_interval`.

  No test reads the fields `snapshot`, `published`, `wake` or `hidden_gate` directly, so those fields can move into a shared cell.
- **Recommended seam (Option A, "thin skeleton, thick source"):** a new module (e.g. `src-tauri/src/source_refresh.rs`) with three parts:
  - a generic `SourceCell<S>` that owns the snapshot `RwLock`, the wake `Sender`, the `published` seq guard and `publish`;
  - a small `RefreshSnapshot` trait (`seq`, `begin_refresh`);
  - free helpers: thread spawn with a `ThreadDown` fallback, the coalescing `recv_timeout` loop driven by a round that returns its own `Duration`, the "refreshing" flip, the round tail (todo emit, events notify, transition logs, health wake), the `next_refresh_at_ms` instant, and the todo-reconcile result logging.

  Each source state keeps its type name, its private extra mutexes and thin inherent wrappers, so test bodies, `lib.rs`, `runtime.rs`, `hidden_issues/commands.rs` and `todo/commands.rs` compile unchanged. The options are compared in "Architecture Insights".

## Detailed Findings

### 1. Duplication map (GitHub ↔ GitLab)

| # | Building block | GitHub | GitLab | Difference |
|---|---|---|---|---|
| 1 | Managed state: snapshot `RwLock`, `kept_config`, `wake: mpsc::Sender<()>`, `published: Mutex<u64>` | `github_refresh.rs:35-55` | `gitlab_refresh.rs:70-114` | GitLab adds `kept_full`, `kept_filter`, `kept_new`, `shown_until`, `last_mark`, `hidden_gate` |
| 2 | `snapshot()` clone, `wake()`, private `read()`/`write()` with poison recovery | `github_refresh.rs:57-76` | `gitlab_refresh.rs:116-126, 258-264` | identical apart from the type |
| 3 | `*_snapshot` and `refresh_*` commands (wake → Polish error text) | `github_refresh.rs:78-92` | `gitlab_refresh.rs:391-405` | text "GitHuba"/"GitLaba"; GitLab adds `mark_gitlab_seen` (`:407-431`) |
| 4 | `manage(app) -> Receiver<()>` | `github_refresh.rs:94-101` | `gitlab_refresh.rs:433-440` | identical |
| 5 | `start`: named thread, on spawn error store `next_snapshot(.., Err(ThreadDown))`, publish, wake health | `github_refresh.rs:103-124` | `gitlab_refresh.rs:442-463` | thread name, log prefix, error type |
| 6 | `refresh_loop`: `recv_timeout`, drain extra wakes, return on `Disconnected` | `github_refresh.rs:126-136` | `gitlab_refresh.rs:465-485` | GitHub waits the constant; GitLab waits the `Duration` its round returns and carries loop-local `resume_after` and `SessionSeen` |
| 7 | `state_label` (value-free log label) | `github_refresh.rs:138-147` | `gitlab_refresh.rs:487-496` | same 5 arms on two distinct but same-shaped enums (`GithubState` `github/mod.rs:206-212`, `GitlabState` `gitlab/mod.rs:434-440`) |
| 8 | `run_fetch`: no path → `ConfigDirUnresolved`; load settings; `None` → `NotConfigured`; else fetch | `github_refresh.rs:149-166` | `gitlab_refresh.rs:498-518` | loader, fetch fn, types |
| 9 | Round frame: `before` + `before_status`; `refreshing = true; seq += 1`; publish; fetch; `kept_config` → `next_snapshot`; store; publish; todo emit + `events::commands::notify`; `from -> to` info log; debug elapsed log; health wake on status change | `github_refresh.rs:168-217` | `gitlab_refresh.rs:535-737` | GitLab inserts hidden checks (`:560-569`), closed section (`:571-578`), seen set (`:580-590`), gate (`:594`, released `:713`), commit/partition (`:608-635`), round flags (`:650-660`), kept reshape (`:661-702`), `next_refresh_at_ms` (`:711`), toasts (`:721-724`) |
| 10 | `reconcile_input` + `reconcile_todo` (store call, `Ok(changed)` / `Store` → debug / other → warn) | `github_refresh.rs:219-264` | `gitlab_refresh.rs:739-751, 847-876` | input shape and `todo::reconcile_github` vs `reconcile_gitlab`; the result-logging `match` is identical |
| 11 | `publish`: lock `published`, re-read snapshot, skip `seq <= last`, emit | `github_refresh.rs:266-278` | `gitlab_refresh.rs:878-890` | event name and log prefix |

Source-only GitLab parts that stay in `gitlab_refresh.rs`:

- `act_on_issue`, `accept_round`, `mark_seen` (`:128-256`);
- `filter_shown`, `shape_items`, `reshape_kept`, `load_filter`, `fetch_closed_section` (`:267-389`);
- `refresh_interval` (`:520-532`), `notify_input` (`:753-760`), `SeenOutcome`/`record_seen`/`toast_new_assignments` (`:762-845`), `SEEN_SET_WARNED` (`:66`).

### 2. Mutexes and ordering in GitLab `refresh_round`

- The lock order is documented in the module header (`gitlab_refresh.rs:9-15`): gate → `kept_config` → `kept_full` → `kept_filter` → `kept_new` → `shown_until` → `last_mark` → snapshot. The store lock may be taken inside the gate, but never while a `kept_*`, `shown_until` or `last_mark` lock is held.
- On this path, the round runs these steps in order:
  1. Interval and filter read before any lock (`:542-544`).
  2. The "refreshing" write and publish (`:551-556`).
  3. `glab` fetch, hidden checks, the closed fetch and the seen-set store write, all with no lock held (`:558-590`).
  4. Gate taken (`:594`).
  5. `accept_round`, which locks `kept_new`, `shown_until` and `last_mark` (`:195-223`).
  6. `commit_round` store write (`:610`).
  7. `kept_config` clone (`:641-645`), then `next_snapshot`.
  8. The hidden-record read under the gate but before any `kept_*` lock (`:663-669`).
  9. A block holding `kept_full`, `kept_filter`, `kept_new` and `shown_until` together (`:670-702`).
  10. A `kept_config` write (`:703`).
  11. `reconcile_todo`, a store write still under the gate (`:709`).
  12. `next_refresh_at_ms` (`:711`), then the snapshot write (`:712`).
  13. `drop(gate)` (`:713`), then publish, todo emit, events notify and toasts outside every lock (`:714-724`).
- Commands that share the gate:
  - `act_on_issue` (`:135-184`), used by `hidden_issues/commands.rs:33`, which then calls `publish` (`:47`);
  - `mark_seen` (`:231-256`), used by `mark_gitlab_seen` (`:410-431`).
- Implication: the skeleton cannot own the "store `next`, then publish" step as one call for GitLab, because GitLab stores under the gate and publishes after `drop(gate)`. The skeleton offers `publish` as a separate call, and the source decides when to call it.

### 3. Wake after sleep and "Check now"

- Neither refresh module detects sleep. The watchdog thread (`runtime.rs:499-516`) compares the gap between 15 s ticks (`WATCHDOG_INTERVAL`, `runtime.rs:39`) with `RESUME_GAP` of 30 s (`runtime.rs:42`) through `health::is_resume` (`health/mod.rs:320-322`). On a resume it wakes health, `GitlabRefreshState::wake()` and `GithubRefreshState::wake()` (`runtime.rs:513-515`).
- "Check now" exists in three paths, all ending in `wake()`:
  - The view button "Sprawdź teraz" (`src/gitlab/GitlabView.tsx:649`, `src/github/GithubView.tsx:214`) calls `refreshGitlab`/`refreshGithub` (`src/gitlab/api.ts:11`, `src/github/api.ts:10`), which reach the `refresh_gitlab`/`refresh_github` commands (`gitlab_refresh.rs:400-405`, `github_refresh.rs:87-92`). The hooks also call them on mount (`src/gitlab/useGitlab.ts:62`, `src/github/useGithub.ts:47`).
  - The tray "check" item wakes both sources and health (`runtime.rs:261-265`).
  - `check_health_now` takes both states as `tauri::State` and wakes them (`runtime.rs:127-139`).
- Coalescing: `refresh_loop` drains queued wakes after any wake (`gitlab_refresh.rs:480`, `github_refresh.rs:131`). The unit test `state_holds_a_starting_snapshot_and_wakes_coalesce` checks the channel behaviour for both sources (`gitlab_refresh.rs:1277`, `github_refresh.rs:350-360`).
- Skeleton consequence: wake after sleep and "Check now" are shared already, through each state's `wake()`. The skeleton only has to keep `wake()` on each state, so `runtime.rs` compiles unchanged. A `wake_sources(app)` helper could replace the three pairs of calls in `runtime.rs`, but M-4 adds no third source, so it is optional (see Open Questions).

### 4. Snapshot publication, `lastSuccessMs`, `nextRefreshAtMs`

- `publish` emits the whole snapshot on `gitlab-changed`/`github-changed` (`gitlab_refresh.rs:63`, `github_refresh.rs:30`) only when `seq` grew. Listeners: `src/gitlab/useGitlab.ts:42`, `src/github/useGithub.ts:29`. `hidden_issues/commands.rs:17,47` and `mark_gitlab_seen` (`gitlab_refresh.rs:423`) publish from commands too.
- `seq` increases in three places: in `next_snapshot` (`gitlab/mod.rs:558`, `github/mod.rs:297`), in the "refreshing" flip (`gitlab_refresh.rs:554`, `github_refresh.rs:178`), and in the GitLab command paths (`gitlab_refresh.rs:182, 254`).
- `last_success_ms` and `checked_at_ms` are set in `next_snapshot`: a success sets both, and an error keeps `last_success_ms` only while the config is unchanged (`gitlab/mod.rs:579-635`, `github/mod.rs:299-329`). These are rules with their own tests (e.g. `gitlab/mod.rs:1490`, `github/mod.rs:753`). Moving them into a tauri-side skeleton would break the "rules stay free of tauri" split (`gitlab_refresh.rs:2-3`, `github_refresh.rs:2-3`).
- `next_refresh_at_ms`:
  - GitLab sets it to `now_ms() + wait` right before the snapshot store (`gitlab_refresh.rs:710-711`).
  - `next_snapshot` carries it over from `prev`, together with `refresh_minutes` and `refresh_setting_invalid` (`gitlab/mod.rs:569-575`).
  - The "refreshing" snapshot keeps the old value.
  - Frontend: `src/gitlab/types.ts` (`nextRefreshAtMs`), used in `src/gitlab/GitlabView.tsx:70-80`.
  - `src/github/types.ts:40-49` `GithubSnapshot` has `state, repos, lastSuccessMs, checkedAtMs, refreshing, seq` and nothing more.
- Interval: GitLab reads `gitlabRefresh.minutes` every round through `refresh_interval` (`gitlab_refresh.rs:520-532`; `settings.rs:401` `DEFAULT_REFRESH_MINUTES = 10`, `settings.rs:422` `load_gitlab_refresh_minutes`), and the round returns its `Duration` to the loop (`:542-543, 736`). GitHub uses `REFRESH_INTERVAL` (`github_refresh.rs:31, 130`).

### 5. "To do" reconciliation and event publication

- GitHub reconciles on `reconcile_input`: only `Ok` rounds, only repos without their own error, keyed lowercase (`github_refresh.rs:219-232`). Then `todo::reconcile_github` (`:243-245`).
- GitLab reconciles only on an `Ok` round with this round's unfiltered full list and an account (`gitlab_refresh.rs:739-751`). Then `todo::reconcile_gitlab` (`:855-857`), still under the gate (`:709`).
- The result handling is identical in both: `changed > 0` → debug log and `true`; `TodoError::Store` → debug; anything else → warn (`github_refresh.rs:246-263`, `gitlab_refresh.rs:858-875`).
- After the publish, both emit `todo::CHANGED_EVENT` and call `events::commands::notify` when something changed (`github_refresh.rs:200-205`, `gitlab_refresh.rs:715-720`). `ClosedInSource` events are written inside `todo::reconcile_*` (context/archive/2026-11-24-event-log/plan.md:11-13).
- GitLab also writes `NewEvent::Assigned` in `record_seen` and calls `notify` on its own (`gitlab_refresh.rs:793-812`).

### 6. Tests: what moving code must keep reachable

- Integration tests: zero references to `gitlab_refresh` or `github_refresh` in `src-tauri/tests/**` (grep over `*.rs`). `lib.rs:7,9` exposes both as `pub mod`, with no `pub use` of their items.
- `github_refresh.rs` tests (`:280-361`, 3 tests) need these names in scope through `use super::*`:
  - `state_label` (`:293`) and `reconcile_input` (`:324`);
  - `GithubRefreshState::new()` returning `(Self, mpsc::Receiver<()>)`, plus `.snapshot()` and `.wake()` returning `Result<(), mpsc::SendError<()>>` (`:350-360`);
  - the `github` module alias, used as `github::RepoView` and `github::RepoErrorView` (`:312-321`).
- `gitlab_refresh.rs` tests (`:892-1727`, 33 tests) need these names in scope:
  - The private free functions `state_label`, `reconcile_input`, `notify_input`, `filter_shown`, `shape_items`, `reshape_kept`, `load_filter` and `refresh_interval`. `refresh_interval` is tested at `:1503-1512` with `&None` and a temp settings file.
  - `GitlabRefreshState::new()`, `snapshot()`, `wake()` and the private `write()`. Tests use `state.write().seq = …` and `state.write().closed = …`, so `write()` must return a guard that derefs mutably to `GitlabSnapshot`.
  - `accept_round` (private), plus `act_on_issue` and `mark_seen` (`pub(crate)`).
  - The private fields `kept_config` (`:1330, 1401`), `kept_full` (`:1334, 1405, 1539`), `kept_filter` (`:1418`), `kept_new` (`:1643, 1677, 1724`), `shown_until` (`:1662, 1705, 1725`) and `last_mark` (`:1622, 1645, 1678`). Tests call `.lock().unwrap()` on them, so each must stay a direct `Mutex<…>` field of `GitlabRefreshState` with its current inner type.
  - Names that come from the parent's `use` lines: `HashSet`, `GitlabFilter`, `GitlabConfig`, `GitlabHost`, `GitlabItem`, `AssignedItems`, `ClosedItem`, `ClosedSection`, `FilteredOut`, `ItemKind`, `GitlabSnapshot`, `HiddenRecord`, `HiddenIssueError`, `ItemKey`, `NewAssignmentError`, `StoreError` and `gitlab`. If a refactor stops using one of them in the parent, the parent still needs that import (for example under `#[cfg(test)]`), or the tests stop compiling.
- No test reads the `snapshot`, `published`, `wake` or `hidden_gate` fields directly (grep of field accesses in `gitlab_refresh.rs:892-1727`). Those fields can move into a shared cell, provided the inherent methods stay.
- No test calls `run_fetch`, `refresh_round`, `refresh_loop`, `start`, `publish`, `record_seen`, `toast_new_assignments`, `reconcile_todo` or `fetch_closed_section` (grep over both test modules). These have no unit test, so F-01's behaviour check for them rests on review and the manual run, not on the unit suite.
- Other consumers that must keep compiling:
  - `lib.rs:169-178` (`manage`/`start`) and `lib.rs:202-208` (the commands);
  - `runtime.rs:29-30, 129-134, 262-263, 428-429, 514-515`;
  - `hidden_issues/commands.rs:17, 33, 47` (`GitlabRefreshState`, `publish`);
  - `todo/commands.rs:18, 20, 143, 163` (`.snapshot()`).

### 7. What later slices need from the skeleton

- **S-02, github-refresh-interval** (roadmap.md:117-126; issue #29):
  - GitHub's round must return its own `Duration`, as GitLab's does, so S-02 only swaps the constant for a `githubRefresh.minutes` read.
  - It needs a shared `next_refresh_at_ms(wait)` helper and a hook to set it right before the snapshot store.
  - S-02 itself adds `refresh_minutes`, `next_refresh_at_ms` and `refresh_setting_invalid` to `GithubSnapshot` and the TS mirror, with the carry-over in `github::next_snapshot`.
  - A generic interval reader over a settings block name would let S-02 reuse GitLab's range and default (`settings.rs:401-455`). That reader is in `settings.rs`, so it is S-02's choice, not F-01's.
- **S-03, github-new-and-notify** (roadmap.md:129-139; issue #30):
  - It needs space for loop-local state (GitLab's `SessionSeen`, `gitlab_refresh.rs:470`), so the loop helper must accept a round closure that captures `&mut` state.
  - It needs a post-publish slot for toasts, after the gate and the publish (`gitlab_refresh.rs:721-724`).
  - It needs a source-owned gate plus `kept_new`/`shown_until`/`last_mark`-style mutexes next to the shared cell.
  - Its view mark and seen keys are store work (a migration), not skeleton work.
- **S-01, github-hide-resurface** (roadmap.md:103-114; issue #31):
  - It needs the same gate pattern and a `kept_full` for GitHub.
  - Commands must store a snapshot with `seq + 1` and then publish, so the shared `publish` stays `pub(crate)` and callable from a command module.
  - It needs a check step without a lock before the gate, and a commit under it.
  - The hiding rules (`hidden_issues/rules.rs`) are outside the skeleton.
- **S-06, gitlab-review-requests** (roadmap.md:166-176; issue #35):
  - The second `glab api` fetch belongs in GitLab's own `run_fetch`/round, so the skeleton must not fix "one fetch per round" or one outcome type.
  - Dedupe of an MR that is both assigned and to review is a `new_assignments` rule.
  - S-06 needs nothing beyond F-01 keeping GitLab's round body source-owned.

## Code References

- `src-tauri/src/github_refresh.rs:35-136` - GitHub state, commands, `manage`, `start` (ThreadDown), loop
- `src-tauri/src/github_refresh.rs:168-217` - GitHub round frame
- `src-tauri/src/github_refresh.rs:266-278` - GitHub `publish`
- `src-tauri/src/gitlab_refresh.rs:9-28` - lock order and logging rules
- `src-tauri/src/gitlab_refresh.rs:70-265` - GitLab state, gate-guarded methods
- `src-tauri/src/gitlab_refresh.rs:442-485` - GitLab `start` (ThreadDown) and loop with loop-local state
- `src-tauri/src/gitlab_refresh.rs:535-737` - GitLab `refresh_round`
- `src-tauri/src/gitlab_refresh.rs:847-890` - GitLab `reconcile_todo`, `publish`
- `src-tauri/src/gitlab/mod.rs:476-635` - `GitlabSnapshot`, `starting`, `next_snapshot`
- `src-tauri/src/github/mod.rs:234-330` - `GithubSnapshot`, `starting`, `next_snapshot`
- `src-tauri/src/runtime.rs:127-139, 261-265, 499-516` - "Check now" paths and resume wake
- `src-tauri/src/lib.rs:169-178, 202-208` - wiring and commands
- `src-tauri/src/hidden_issues/commands.rs:17-47` - command-side publish
- `src-tauri/src/settings.rs:401-455` - `DEFAULT_REFRESH_MINUTES`, `load_gitlab_refresh_minutes`

## Architecture Insights

### Seam options

**Option A: thin skeleton, thick source (recommended).** New module, e.g. `src-tauri/src/source_refresh.rs`, declared once in `lib.rs`.

- `SourceCell<S>` holds `snapshot: RwLock<S>`, `wake: mpsc::Sender<()>` and `published: Mutex<u64>`. It provides:
  - `new(starting) -> (Self, Receiver<()>)`, `snapshot()`, `read()`, `write()`, `wake()`;
  - `publish(app, event, log_tag)`: same body as today, with the event name and log prefix passed in.
- `trait RefreshSnapshot: Clone + Serialize` with `seq()` and `begin_refresh()` (sets `refreshing = true; seq += 1`). A `state_kind()` that returns the log label would let the bodies of both `state_label`s become one shared function. The free functions named `state_label` must stay in each module for the tests.
- `spawn(app, name, log_tag, body, on_fail)`: the `std::thread::Builder` call. `on_fail` returns the ThreadDown snapshot. The skeleton stores it, publishes it and wakes health.
- `run_loop(wake_rx, round: impl FnMut() -> Duration)`: the coalescing `recv_timeout` loop. GitHub's round returns `REFRESH_INTERVAL` for now.
- Round helpers:
  - `begin_round` (refreshing flip + publish);
  - `finish_round` (todo emit + `events::commands::notify` when changed, the `from -> to` info and elapsed debug logs, and a health wake on status change). The source runs its toasts between publish and `finish_round`, or `finish_round` takes a closure. The current GitLab order is publish → todo emit → notify → toasts → logs → health wake (`gitlab_refresh.rs:714-735`);
  - `next_refresh_at(wait) -> u64`;
  - `log_reconcile(result) -> bool` for the shared todo-result match.
- Each source state keeps its name and becomes `{ cell: SourceCell<…>, kept_config, <extras> }`. It keeps inherent `new`, `snapshot`, `wake`, `write` and `read` that delegate, and a module-level `pub(crate) fn publish(app, state)` that delegates. Tests, `runtime.rs`, `hidden_issues/commands.rs` and `todo/commands.rs` stay untouched.
- Pros: every test body and every external call site compiles as it is. GitLab's gate stays fully source-owned. Each later slice adds code only in its own module.
- Cons: thin delegating wrappers remain. The skeleton does not own the full round, so a third source would still write its own round body (acceptable: M-4 has no third source).

**Option B: trait-driven round (`trait RefreshSource { type Snapshot; type Config; type Fetched; fn run_fetch…; fn next_snapshot…; fn after_store… }`, with the skeleton owning `refresh_round`).**

- Pros: the most deduplication, and the skeleton owns the full round.
- Cons: GitLab needs at least four hooks inside its round (checks before the gate, the closed section, the seen set, the gate itself across the commit/store, the kept reshape). Moving the gate into a hook risks the documented lock order, and the roadmap names this as F-01's main risk (`roadmap.md:98`). Private helpers would move behind trait methods, and the tests need them by their current free-function names. Rejected as F-01's default; it could be revisited once S-01 and S-03 show what the GitHub round looks like.

**Option C: `macro_rules!` generating the shared items per source.**

- Pros: byte-identical behaviour.
- Cons: harder to read and review, worse for clippy and the IDE, and it hides the lock order. Rejected.

### Constraints the plan must keep

- The snapshot lock is never held across `emit`, a tray call or a process (`gitlab_refresh.rs:5-7`). The gate is never held across `glab` or `emit` (`:11-12`). Publish happens after `drop(gate)` (`:713-714`).
- Log text and prefixes ("gitlab: …", "github: …", "todo: …") stay as they are. They are part of the observed behaviour, and the logging rule allows state kinds and elapsed time only (`gitlab_refresh.rs:27-28`, `github_refresh.rs:10-11`).
- The Polish texts of the command errors (`gitlab_refresh.rs:404`, `github_refresh.rs:91`) and the thread names `gitlab-refresh`/`github-refresh` (`:448`, `:109`) stay as they are.
- No change to `GitlabSnapshot`/`GithubSnapshot` fields, serde, event names, command names or signatures. `src/**/types.ts` is untouched.
- No store migration. `lib.rs` gets at most one `mod` line.

### Shared-file touches (parallel slices)

- F-01 touches `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/github_refresh.rs`, a new skeleton module and one `mod` line in `src-tauri/src/lib.rs` (near `lib.rs:7-9`). Optionally it touches `runtime.rs`, only for a `wake_sources` helper, and an AGENTS.md "Project Structure" line.
- S-05 (`gitlab-view-memory`) also edits `lib.rs` (new commands in `invoke_handler`, `lib.rs:198+`), `AGENTS.md` and `store/schema.rs` (v14). Expect a small textual merge in `lib.rs` (different hunks: the `mod` list vs `invoke_handler`) and in `AGENTS.md` if F-01 adds a line there. F-01 must not touch `schema.rs`.
- S-04 (`gitlab-offline-last-list`) and S-05 edit `src/gitlab/GitlabView.tsx`, and S-05 edits `GitlabListViewProvider.tsx`. F-01 touches no frontend file.

## Historical Context (from prior changes)

- `context/archive/2026-11-23-github-assigned-issues/plan.md:55` - the GitHub slice chose to "mirror the GitLab vertical slice step by step". That is the origin of the copy, and no archive item records it as deliberate debt. Verdict: supported, and still the current shape.
- `context/archive/2026-11-23-github-assigned-issues/plan.md:47, 232` - fixed 10-minute GitHub interval, no notices. Verdict: supported (`github_refresh.rs:31`); S-02 and S-03 supersede it.
- `context/archive/2026-11-23-github-assigned-issues/reviews/plan-review.md:56-65` - the resume wake was missing in the GitHub plan and was added. Verdict: supported (`runtime.rs:515`).
- `context/archive/2026-11-23-gitlab-assigned-list/plan.md:34, 190, 349` - named thread, coalescing wake channel, ordered `seq` publish, ThreadDown on spawn failure. Verdict: supported (`gitlab_refresh.rs:445-485, 878-890`). The interval it names (5 min fixed) is superseded by `gitlabRefresh.minutes`.
- `context/archive/2026-11-20-source-health-status/plan.md:54` - `std::thread` + `recv_timeout`, no tokio, so "check now" wakes instantly. Verdict: supported.
- `context/archive/2026-11-23-new-assignment-notify/plan.md:55, 273-303` - interval read every round, range 1-14, default 10, invalid value → default plus a snapshot flag. Verdict: supported (`gitlab_refresh.rs:520-543, 658-659`).
- `context/archive/2026-11-24-gitlab-filter-bar-retry/plan.md:26, 43, 66` - `nextRefreshAtMs` = end of round + interval, set right before the store, carried over in `next_snapshot`. Verdict: supported (`gitlab_refresh.rs:711`, `gitlab/mod.rs:571`).
- `context/archive/2026-11-23-hide-and-resurface/plan.md:75-76, 291-294` - check with no lock, then commit under `hidden_gate`, never across `glab`/`emit`. Verdict: supported (`gitlab_refresh.rs:560-569, 594-713`).
- `context/archive/2026-11-24-event-log/plan.md:11-13, 46` - `ClosedInSource` events are written in `todo::reconcile_*`; new-assignment events are GitLab only. Verdict: supported. Its line numbers into the refresh modules are stale; the function names are not.

## Related Research

- `context/archive/2026-11-23-github-assigned-issues/` and `context/archive/2026-11-23-gitlab-assigned-list/` (plans and reviews of the two copies).

## Defaults chosen by agent

The owner could not be asked in this run. Each item below is the safest option, and `/10x-plan` may revisit it.

1. **`lastSuccessMs` stays in the rules `next_snapshot` functions** (`gitlab/mod.rs`, `github/mod.rs`) instead of moving into the skeleton. Moving it would touch tested, tauri-free rules and their test bodies. The skeleton "covers" it only in the sense that every round goes through `next_snapshot`. Default chosen by agent.
2. **GitHub gets no `nextRefreshAtMs`/`refreshMinutes` field in F-01.** Adding them would change `GithubSnapshot` and its TS mirror, which F-01 forbids. F-01 provides the hook (round returns `Duration`, `next_refresh_at(wait)` helper), and S-02 adds the fields. Default chosen by agent.
3. **Wake after sleep and "Check now" stay where they are** (`runtime.rs` watchdog, tray and `check_health_now` calling each state's `wake()`). F-01 keeps `wake()` on both states and does not change `runtime.rs`, unless the plan opts into an optional `wake_sources` helper. Default chosen by agent: no `runtime.rs` change.
4. **Option A (thin skeleton, thick source) is the recommended seam.** The GitLab gate, kept-list mutexes and source rules stay in `gitlab_refresh.rs`, with no trait-owned round. Default chosen by agent.
5. **Log lines, thread names, event names and Polish error texts stay byte-identical.** Default chosen by agent.

## Open Questions

- Should `SourceCell::publish` take the event name per call, or should the cell keep it as a field? Either way the behaviour stays the same; this is a plan-level choice.
- Should GitHub's round already return `Duration` (= `REFRESH_INTERVAL`) in F-01, so S-02 only changes the value? Recommended yes, with no behaviour change.
- Whether to add one line to AGENTS.md "Project Structure" for the new skeleton module. This would merge with S-05's AGENTS.md edit. The plan decides; if added, keep it in "Project Structure", away from the Hard Rules lines S-05 changes.
- No unit test covers `refresh_round`, `refresh_loop`, `start` or `publish` today (section 6). The plan should name how "no behaviour change" is checked for them: new skeleton-level unit tests (e.g. a `run_loop` coalescing test with a fake round, and a `publish` seq-guard test through a pure helper) plus a manual run of both views with and without VPN.
