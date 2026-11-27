# Shared source refresh skeleton (F-01) Implementation Plan

## Overview

GitLab and GitHub refresh through one shared skeleton in a new module, `src-tauri/src/source_refresh.rs`. The skeleton owns the parts both sources repeat today:

- the snapshot cell (lock, wake channel, ordered publication);
- the thread start with a `ThreadDown` stand-in;
- the coalescing wait loop, driven by a round that returns its own interval;
- the "refreshing" flip;
- the round tail (the "To do" event, transition and elapsed logs, the health wake);
- the "To do" reconcile result logging;
- the `nextRefreshAtMs` instant.

The fetch and the source rules stay in `gitlab_refresh.rs` and `github_refresh.rs`. Behaviour, Tauri commands, TS types, log lines, thread names, event names and Polish texts do not change, and no existing test body changes.

Roadmap M-4, F-01, anchor MS-28 (GitHub issue #28). Planned non-interactively: the owner could not be asked, so every choice the skill would have asked about is recorded below as "default chosen by agent".

## Current State Analysis

The research (`context/changes/source-refresh-skeleton/research.md`) maps the copy in detail. The parts that matter for this plan:

- **The copy.** `github_refresh.rs:35-278` copies eleven building blocks of `gitlab_refresh.rs` almost line for line (research §1 table). They differ in the snapshot/config/error types, the event name, the log prefix, the thread name and GitLab's extra round steps.
- **GitLab's round.** `refresh_round` (`gitlab_refresh.rs:535-737`) wraps the shared frame around source-only steps: hidden checks, the closed section, the seen set, the `hidden_gate`, NEW flags and the kept reshape. It stores the snapshot under the gate (`:712`), drops the gate (`:713`) and only then publishes (`:714`). The skeleton therefore cannot own "store + publish" as one call for GitLab.
- **Outside the refresh modules.** Wake after sleep, "Check now" and the tray "check" item all call each state's public `wake()` (`runtime.rs:127-139, 261-265, 509-516`). `last_success_ms` is set by the tauri-free rule `next_snapshot` (`gitlab/mod.rs:551-635`, `github/mod.rs:284-330`).
- **Interval and next try.** Only GitLab has an interval read every round and a `next_refresh_at_ms` (`gitlab_refresh.rs:520-543, 711`). GitHub waits a constant `REFRESH_INTERVAL` of 10 min (`github_refresh.rs:31, 130`), and `GithubSnapshot` has no such fields.
- **Test dependencies.** The tests reach state internals and private functions through `use super::*` (research §6):
  - the private `new()`, plus `snapshot()`, `wake()` and the private `write()`;
  - GitLab's `kept_*`, `shown_until` and `last_mark` as direct `Mutex` fields;
  - the free functions `state_label`, `reconcile_input`, `notify_input`, `filter_shown`, `shape_items`, `reshape_kept`, `load_filter` and `refresh_interval`;
  - `now_ms` and the parent's `use` names.

  No test reads the `snapshot`, `wake`, `published` or `hidden_gate` fields directly. No integration test in `src-tauri/tests/` references either module.
- **No unit test** covers `start`, `refresh_loop`, `refresh_round` or `publish` (research §6). F-01 adds skeleton-level tests for the parts it takes over.

## Desired End State

- `src-tauri/src/source_refresh.rs` exists and holds `SourceId`, `RefreshSnapshot`, `SourceCell<S>`, `run_loop` and `next_refresh_at_ms`, with its own unit tests.
- `GithubRefreshState` and `GitlabRefreshState` each hold a `cell: SourceCell<…>` in place of their `snapshot`/`wake`/`published` fields. Each keeps its name, its source-only fields, and thin inherent `new`/`snapshot`/`wake`/`write` wrappers. Each module keeps its `pub(crate) fn publish(app, state)`.
- GitHub's round returns its `Duration` (still `REFRESH_INTERVAL`), as GitLab's does. S-02 then only swaps the value and calls `next_refresh_at_ms`.
- `lib.rs` differs from `develop` by exactly one line, `pub mod source_refresh;`. No other file outside the two refresh modules, the new module and one `AGENTS.md` bullet changes.
- The app behaves exactly as before. The built app shows the same views, tray and notices. Its log file shows the same lines with the same `[day_navigator_lib::gitlab_refresh]` / `[day_navigator_lib::github_refresh]` targets.

Verify with the gates, the test-body diff check, the log-target check and the scope checks in each phase, plus the Manual rows (checked later by the user in the integrated build).

### Key Discoveries:

- **The log target is part of every log line.** The log plugin's default format writes `[time][LEVEL][target] message`, and the target is `module_path!()` of the calling module (`tauri-plugin-log` 2.10 `lib.rs:517-523`; the builder in `src-tauri/src/lib.rs:151-159` keeps the default format). A `log::info!` moved into `source_refresh.rs` would print `[day_navigator_lib::source_refresh]` instead of `[day_navigator_lib::gitlab_refresh]`. Every log call in the skeleton must therefore pass `target: id.target`, where each source sets `target: module_path!()`.
- **`publish` emits while it holds the `published` lock** (`gitlab_refresh.rs:880-890`, `github_refresh.rs:268-278`). Keeping `last` locked across `emit` is what keeps events ordered by `seq`. The shared `publish` must keep this.
- **GitLab sets `next_refresh_at_ms` right before the snapshot store, under the gate** (`gitlab_refresh.rs:710-712`), and `next_snapshot` carries it over (`gitlab/mod.rs:569-575`). The helper only computes the instant; GitLab still decides where to call it.
- **The GitLab tail order** is publish → "To do" emit → `events::commands::notify` → toasts → `from -> to` info → elapsed debug → health wake (`gitlab_refresh.rs:714-735`). GitHub has the same order without toasts (`github_refresh.rs:199-216`). The elapsed time is measured after the toasts.
- **`lib.rs` declares every module `pub mod`** (`lib.rs:1-22`). With `pub mod source_refresh;` and `pub(crate)` items, an unused skeleton item is a `dead_code` warning, and `cargo clippy -- -D warnings` fails on it. So each phase adds only skeleton items that have a non-test caller in that phase.

## What We're NOT Doing

- No `refreshMinutes`, `nextRefreshAtMs` or `refreshSettingInvalid` on `GithubSnapshot` and no `githubRefresh.minutes` setting (S-02).
- No GitHub NEW flags, seen keys, notices or loop-local session state (S-03), no GitHub hiding or gate (S-01), and no second GitLab MR fetch (S-06).
- No trait-owned round (research Option B) and no macro (Option C).
- `last_success_ms`/`checked_at_ms` do not move out of the tauri-free `next_snapshot` rules (`gitlab/mod.rs`, `github/mod.rs`).
- No change to `runtime.rs`: no `wake_sources` helper, and wake after sleep, the tray "check" item and `check_health_now` keep calling each state's `wake()`.
- `state_label` and `run_fetch` are not shared: they stay per source module, `state_label` because the tests call it by name and `run_fetch` because it is the source-specific fetch S-06 extends.
- No store migration, no `store/schema.rs` edit, no frontend or `src/**/types.ts` edit, no PRD FR, no `context/foundation/roadmap.md` edit.

## Implementation Approach

Option A from the research: a thin skeleton and a thick source. The skeleton is a cell plus helpers the source calls in its own round. It is not a framework that calls the source. GitHub moves first, because its round is the shared frame with nothing else in it, so the skeleton's API is proven on the simple case. GitLab moves second, and its gate, lock order and source steps stay where they are, line for line. A last small phase adds the `AGENTS.md` pointer and runs the final checks over the whole change.

### Skeleton contract (Phase 1 defines it; Phase 2 and S-01/S-02/S-03/S-06 rely on it)

```rust
// src-tauri/src/source_refresh.rs
#[derive(Clone, Copy)]
pub(crate) struct SourceId {
    pub(crate) event: &'static str,  // "gitlab-changed" / "github-changed"
    pub(crate) thread: &'static str, // "gitlab-refresh" / "github-refresh"
    pub(crate) prefix: &'static str, // "gitlab" / "github" (log text prefix)
    pub(crate) target: &'static str, // module_path!() of the source module (log target)
}

pub(crate) trait RefreshSnapshot: Clone + Serialize + Send + Sync + 'static {
    fn seq(&self) -> u64;
    fn begin_refresh(&mut self); // refreshing = true; seq += 1
}

pub(crate) struct SourceCell<S> { /* snapshot: RwLock<S>, wake: Sender<()>, published: Mutex<u64>, id: SourceId */ }

impl<S: RefreshSnapshot> SourceCell<S> {
    pub(crate) fn new(starting: S, id: SourceId) -> (Self, mpsc::Receiver<()>);
    pub(crate) fn snapshot(&self) -> S;                                // clone under a short read lock
    pub(crate) fn wake(&self) -> Result<(), mpsc::SendError<()>>;
    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, S>;             // poison-recovering
    pub(crate) fn publish(&self, app: &AppHandle);                     // = publish_with(emit on id.event)
    fn publish_with(&self, emit: impl FnOnce(&S));                     // seq guard; emit runs under `published`
    pub(crate) fn begin_round(&self, app: &AppHandle);                 // begin_refresh under write, then publish
    pub(crate) fn start_thread(&self, app: &AppHandle,
        body: impl FnOnce() + Send + 'static,
        thread_down: impl FnOnce(&S) -> S);                            // spawn id.thread; on error: log, store, publish, wake health
}

impl SourceId {
    pub(crate) fn notify_todo_changed(&self, app: &AppHandle, changed: bool); // todo::CHANGED_EVENT + events::commands::notify
    pub(crate) fn end_round(&self, app: &AppHandle, from: &str, to: &str,
        started: Instant, status_changed: bool);                      // info, debug elapsed, health wake
    pub(crate) fn log_reconcile(&self, result: Result<usize, todo::TodoError>) -> bool;
}

pub(crate) fn run_loop(wake_rx: mpsc::Receiver<()>, round: impl FnMut() -> Duration);
pub(crate) fn next_refresh_at_ms(wait: Duration) -> u64;               // Phase 2 (first caller: GitLab)
```

Each source module declares one `const SOURCE: SourceId = SourceId { event: …, thread: …, prefix: …, target: module_path!() };`, which replaces its `EVENT` const. The private health wake (`app.state::<HealthState>().wake.send(())`, result ignored) is a private skeleton function used by `start_thread` and `end_round`.

## Critical Implementation Details

- **Byte-identical log output.** Every `log::…!` call in `source_refresh.rs` (outside its test module) passes `target: self.target` / `id.target`. Its text is built as `"{}: …", prefix` so the output equals today's literal. Cover all of these: `"{prefix}: cannot start refresh thread: {e}"` (error), `"{prefix}: emit failed: {e}"` (warn), `"todo: emit failed: {e}"` (warn), `"{prefix}: {from} -> {to}"` (info, only when `from != to`), `"{prefix}: round {to} in {} ms"` (debug), `"todo: reconcile changed {changed} rows"` (debug, only when `changed > 0`), `"todo: reconcile failed: {kind}"` (debug for `TodoError::Store(_)`, warn otherwise).
- **Ordering that must survive.**
  - `publish_with` holds `published` across the re-read and the emit, and skips when `seq <= last`.
  - GitLab keeps `next_refresh_at_ms` → store → `drop(gate)` → publish → `notify_todo_changed` → toasts → `end_round`, in that order.
  - `begin_round` releases the write guard before it publishes.
  - `start_thread` logs the error first, then computes the stand-in from the current snapshot, stores it, publishes it and wakes health.
- **The rule for test imports.** Test bodies may not change. If a refactor stops using a name in a module's parent scope that its tests need, re-add it only as a `use` line inside `mod tests` (the diff check ignores `use` lines). Never add a `#[cfg(test)]` item (a `use` or anything else) to the parent scope of `gitlab_refresh.rs`, `github_refresh.rs` or `source_refresh.rs`: the only `#[cfg(test)]` in each file is the one directly above `mod tests`. Today no test needs such a rescue: the only parent names the tests use are `HashSet` and `now_ms` in GitLab, and both stay used by non-test code. Do not add `#[allow(dead_code)]` or `#[allow(unused_imports)]` to the skeleton or to the source modules.

## Phase 1: Skeleton module and GitHub on it

### Overview

Adds `source_refresh.rs` with every contract item except `next_refresh_at_ms`, plus skeleton unit tests, and moves `github_refresh.rs` onto it. After this phase GitHub uses the skeleton and GitLab is untouched.

### Changes Required:

#### 1. Skeleton module

**File**: `src-tauri/src/source_refresh.rs` (new)

**Intent**: Hold the shared refresh mechanics once, so each source keeps only its fetch and rules. Add a module doc that says what the skeleton owns and that it is tauri-side (the rules stay in `gitlab/` and `github/`). The doc also restates the locking rule: the snapshot lock is held only for a clone or a store, and `published` is held across the emit.

**Contract**: The items in "Skeleton contract" above, without `next_refresh_at_ms`. `publish` re-reads `snapshot()` under `published`, skips when `seq <= last`, sets `last = seq` and emits on `id.event`. An emit error logs `"{prefix}: emit failed: {e}"` at warn. `run_loop` runs `round()`, then waits on `wake_rx.recv_timeout(wait)`: `Ok` drains the queued wakes with `try_recv`, `Timeout` loops, `Disconnected` returns. The rules under "Critical Implementation Details" apply.

#### 2. Skeleton unit tests

**File**: `src-tauri/src/source_refresh.rs` (`#[cfg(test)] mod tests`)

**Intent**: Give a unit test to the behaviour that had none before (research §6). These tests are new, so they do not touch any existing test body.

**Contract**: Tests with invented data only. A test that needs a `SourceId` uses invented values (for example `"test-changed"`, `"test-refresh"`, `"test"`), never the real event or thread names, so the identity-literal check in Phase 2 stays at one line per literal.
- `run_loop` coalesces. Queue 3 wakes and use a round that returns 1 h and drops the sender on its 2nd call: exactly 2 rounds run and the call returns.
- `run_loop` loops on timeout. A round that returns 1 ms and drops the sender on its 3rd call: exactly 3 rounds run.
- Both `run_loop` tests count the round's calls and panic once the count exceeds a cap of 10 (for example `assert!(calls <= 10, "run_loop did not stop")` at the top of the round). A loop that ignores `Disconnected` or never drains then fails fast instead of hanging `cargo test`.
- `publish_with` keeps the seq guard: a fake `RefreshSnapshot` with `seq` 0 emits nothing; after a bump it emits once with the new seq; a repeat with no bump or a lower seq emits nothing.
- `begin_refresh` on `GithubSnapshot::starting()` sets `refreshing` and `seq == 1`. Phase 2 adds the same test for `GitlabSnapshot`.

#### 3. GitHub on the skeleton

**File**: `src-tauri/src/github_refresh.rs`

**Intent**: Replace the copied mechanics with skeleton calls. GitHub's behaviour, its command signatures and its private test-facing names stay as they are.

**Contract**:
- `const SOURCE: SourceId` with `"github-changed"`, `"github-refresh"`, `"github"`, `module_path!()` replaces `EVENT`. `REFRESH_INTERVAL` stays.
- `impl RefreshSnapshot for GithubSnapshot` lives in this file.
- `GithubRefreshState { cell: SourceCell<GithubSnapshot>, kept_config: Mutex<Option<GithubConfig>> }`. The private `new() -> (Self, mpsc::Receiver<()>)`, `pub fn snapshot()`, `pub fn wake()` and private `write()` delegate to the cell. Remove `read()` (see the leftovers below).
- `github_snapshot`, `refresh_github` (with the text `"Wątek odświeżania GitHuba nie działa"`) and `manage` are unchanged.
- `start` calls `cell.start_thread`, whose stand-in closure is `github::next_snapshot(prev, None, None, Err(GithubError::ThreadDown), now_ms()).0`.
- `refresh_loop` calls `source_refresh::run_loop(wake_rx, || refresh_round(…))`.
- `refresh_round` returns `Duration` (`REFRESH_INTERVAL`) and uses `begin_round`, then the unchanged fetch/`next_snapshot`/`kept_config` steps, then the store, `publish`, `SOURCE.notify_todo_changed` and `SOURCE.end_round`.
- `reconcile_todo` keeps its store call and returns `SOURCE.log_reconcile(result)`.
- `pub(crate) fn publish(app, state)` keeps its signature and delegates to `state.cell.publish(app)`.
- `state_label`, `run_fetch` and `reconcile_input` are unchanged. Update the module doc to point at `source_refresh.rs`.
- Leftovers clippy would flag (`-D warnings`), removed in this phase: the private `read()` (its only caller, `snapshot()`, now delegates to the cell) and the imports left unused, `RecvTimeoutError`, `RwLock`, `RwLockReadGuard`, `Emitter` and `HealthState`. Keep `mpsc` (for `mpsc::Receiver` / `mpsc::SendError` in signatures), `Mutex`, `RwLockWriteGuard`, `Duration`, `Instant`, `Manager` and `now_ms`.

#### 4. Module declaration

**File**: `src-tauri/src/lib.rs`

**Intent**: Declare the new module. This is the only `lib.rs` edit in the whole change, which keeps the merge with S-05 trivial.

**Contract**: One added line, `pub mod source_refresh;`, between `pub mod settings_file;` and `pub mod space;`.

### Success Criteria:

#### Automated Verification:

- All gates pass in the worktree (`npm install` first only if `node_modules` is missing; revert `package-lock.json` if it changes): `npm run lint`, `npm run format:check`, `npm run build`, and in `src-tauri/` `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Existing test bodies are unchanged (Git Bash, worktree root; prints `gitlab_refresh tests unchanged` and `github_refresh tests unchanged` and no diff). The range starts at the column-0 `mod tests` line in both the base file and the new file, so nothing in the parent scope can enter it: `B=$(git merge-base HEAD develop); for f in gitlab_refresh github_refresh; do diff <(git show $B:src-tauri/src/$f.rs | sed -n '/^mod tests/,$p' | grep -vE '^\s*use ') <(sed -n '/^mod tests/,$p' src-tauri/src/$f.rs | grep -vE '^\s*use ') && echo "$f tests unchanged"; done`
- Every skeleton log call carries a target (Git Bash, worktree root; the two counts are equal). The second count matches only log-call targets (`target: self.target` in `impl SourceId`, `target: self.id.target` in `impl SourceCell`), not the `target: &'static str` field of `SourceId`: `sed '/^mod tests/,$d' src-tauri/src/source_refresh.rs | grep -cE 'log::[a-z]+!\('` and `sed '/^mod tests/,$d' src-tauri/src/source_refresh.rs | grep -cE 'target: *(self\.)?(id\.)?target\b'`
- `lib.rs` differs from the merge base by exactly `+pub mod source_refresh;` (Git Bash, worktree root): `B=$(git merge-base HEAD develop); git diff --numstat $B -- src-tauri/src/lib.rs` prints `1	0	src-tauri/src/lib.rs` (tab-separated), and `git diff -U0 $B -- src-tauri/src/lib.rs | grep -x '+pub mod source_refresh;'` prints that one line
- Nothing outside the scope changed (empty output): `git diff --stat $(git merge-base HEAD develop) -- src/ src-tauri/src/store/ src-tauri/src/gitlab/ src-tauri/src/github/ src-tauri/src/runtime.rs src-tauri/src/hidden_issues/ src-tauri/src/todo/ src-tauri/tests/`
- The new skeleton tests (`run_loop` coalescing, `run_loop` timeout, `publish_with` seq guard, GitHub `begin_refresh`) exist and pass in `cargo test`
- Committed: `refactor(source-refresh): add shared refresh skeleton and move GitHub onto it`, message ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

**Implementation Note**: Run every gate before the commit, and never commit red. Forbidden: `git push`, merges, `--no-verify`, `npm run tauri dev` (port 1420 is shared with other worktrees), any `glab`/`gh` call that is not read-only, real work data in tests.

---

## Phase 2: GitLab on the skeleton

### Overview

Moves `gitlab_refresh.rs` onto the same cell and helpers and adds `next_refresh_at_ms`, whose first caller is GitLab. The gate, the lock order, the kept mutexes and every source step stay where and as they are.

### Changes Required:

#### 1. Next-refresh helper

**File**: `src-tauri/src/source_refresh.rs`

**Intent**: Compute the "next try" instant once, so S-02 reuses it for GitHub.

**Contract**: `pub(crate) fn next_refresh_at_ms(wait: Duration) -> u64` = `now_ms().saturating_add(wait.as_millis() as u64)`. Add a unit test that the result lies between `now_ms() + wait` measured before and after the call. Add the `begin_refresh` test for `GitlabSnapshot::starting()`.

#### 2. GitLab on the skeleton

**File**: `src-tauri/src/gitlab_refresh.rs`

**Intent**: Replace the copied mechanics (cell fields, `start`, loop, the "refreshing" flip, the tail, reconcile logging, `publish`) with skeleton calls. Every GitLab-only step and lock stays in place.

**Contract**:
- `const SOURCE: SourceId` with `"gitlab-changed"`, `"gitlab-refresh"`, `"gitlab"`, `module_path!()` replaces `EVENT`.
- `impl RefreshSnapshot for GitlabSnapshot` lives in this file.
- `GitlabRefreshState` replaces its `snapshot`, `wake` and `published` fields with `cell: SourceCell<GitlabSnapshot>`. `kept_config`, `kept_full`, `kept_filter`, `kept_new`, `shown_until`, `last_mark` and `hidden_gate` stay direct fields with today's types (the tests lock them).
- `new`, `snapshot`, `wake` and `write` delegate to the cell. `act_on_issue`, `accept_round` and `mark_seen` are unchanged.
- `start` uses `cell.start_thread` with the `GitlabError::ThreadDown` stand-in. `refresh_loop` keeps `resume_after` and `session_seen` as loop locals and passes `source_refresh::run_loop` a closure that captures them `&mut`.
- In `refresh_round`, only these lines change: the "refreshing" block + publish becomes `state.cell.begin_round(app)`; `next.next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait))` stays at the same spot; the todo emit/notify becomes `SOURCE.notify_todo_changed`; the logs and the health wake at the end become `SOURCE.end_round(…)` after the toasts. The round still returns `wait`.
- `reconcile_todo` returns `SOURCE.log_reconcile(result)`.
- `pub(crate) fn publish(app, state)` keeps its signature (used by `hidden_issues/commands.rs:17, 47` and `mark_gitlab_seen`) and delegates.
- `gitlab_snapshot`, `refresh_gitlab` (`"Wątek odświeżania GitLaba nie działa"`), `mark_gitlab_seen`, `manage` and all source functions are unchanged.
- In the module doc, the lock-order paragraph keeps its order and says that the snapshot lock now lives in the cell.
- Leftovers clippy would flag (`-D warnings`), removed in this phase: the private `read()` (`gitlab_refresh.rs:258`, used only by `snapshot()`, which now delegates to the cell) and the imports left unused, `RecvTimeoutError`, `RwLock`, `RwLockReadGuard`, `Emitter` and `HealthState`. Keep `mpsc` (for `mpsc::Receiver` / `mpsc::SendError` in signatures), `Mutex`, `RwLockWriteGuard`, `Duration`, `Instant`, `Manager`, `now_ms`, `toast` and `HashSet` (`now_ms` and `HashSet` are also the parent names the tests use).

### Success Criteria:

#### Automated Verification:

- All gates pass in the worktree: `npm run lint`, `npm run format:check`, `npm run build`, and in `src-tauri/` `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Existing test bodies are unchanged (same command as in Phase 1; prints `… tests unchanged` twice)
- Every skeleton log call carries a target (same two counts as in Phase 1, equal)
- `lib.rs` still differs from the merge base by exactly `+pub mod source_refresh;` (same command as in Phase 1)
- Nothing outside the scope changed (same `git diff --stat` command as in Phase 1, empty output)
- Identity strings each appear exactly once as a quoted literal in the three refresh files (Git Bash, worktree root; prints `1 "<literal>"` for all six): `for s in '"gitlab-changed"' '"github-changed"' '"gitlab-refresh"' '"github-refresh"' '"Wątek odświeżania GitLaba nie działa"' '"Wątek odświeżania GitHuba nie działa"'; do echo "$(grep -nF "$s" src-tauri/src/gitlab_refresh.rs src-tauri/src/github_refresh.rs src-tauri/src/source_refresh.rs | wc -l) $s"; done`. The expected lines are the two `SOURCE` consts (event and thread names) and the `refresh_gitlab` / `refresh_github` command errors. The `ThreadDown` messages in `gitlab/mod.rs:205` and `github/mod.rs:130` keep their own copy of the Polish texts, unchanged (the scope check above already shows `gitlab/` and `github/` untouched).
- No copied mechanics remain: `grep -nE 'recv_timeout|thread::Builder|published' src-tauri/src/gitlab_refresh.rs src-tauri/src/github_refresh.rs` returns no line outside the test modules
- Committed: `refactor(source-refresh): move GitLab refresh onto the shared skeleton`, message ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

**Implementation Note**: This is the risky phase (roadmap: "GitLab's refresh round holds six mutexes"). Change only the lines named above. Do not reorder any lock, store write or publish. Run every gate before the commit. The same forbidden list as in Phase 1 applies.

---

## Phase 3: Agent pointer and final checks

### Overview

Adds one `AGENTS.md` pointer, so later slices (S-01, S-02, S-03, S-06) build on the skeleton instead of copying it, and runs the final checks over the whole change.

### Changes Required:

#### 1. Project Structure pointer

**File**: `AGENTS.md`

**Intent**: Tell the next agent where refresh mechanics live and what stays in the source. This is a separate hunk from S-05's `AGENTS.md` edits.

**Contract**: One new bullet in "## Project Structure", directly after the `src-tauri/src/cli/` bullet and touching no other line. It says that `src-tauri/src/source_refresh.rs` is the shared refresh skeleton (snapshot cell, thread with the `ThreadDown` stand-in, wait loop, publication, round tail) and that a source module (`gitlab_refresh.rs`, `github_refresh.rs`) keeps its fetch, rules and locks and calls the skeleton instead of copying it. It also says that skeleton log calls pass the source's `module_path!()` as the target.

### Success Criteria:

#### Automated Verification:

- All gates pass in the worktree: `npm run lint`, `npm run format:check`, `npm run build`, and in `src-tauri/` `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Existing test bodies are unchanged (same command as in Phase 1; prints `… tests unchanged` twice)
- `AGENTS.md` differs from the merge base by exactly one added line (Git Bash, worktree root): `B=$(git merge-base HEAD develop); git diff --numstat $B -- AGENTS.md` prints `1	0	AGENTS.md` (tab-separated), and `git diff -U0 $B -- AGENTS.md | grep -E '^\+- .*src-tauri/src/source_refresh\.rs'` prints the new bullet
- The whole change touches only `AGENTS.md`, `context/changes/source-refresh-skeleton/**`, `src-tauri/src/lib.rs`, `src-tauri/src/source_refresh.rs`, `src-tauri/src/gitlab_refresh.rs` and `src-tauri/src/github_refresh.rs`: `git diff --name-only $(git merge-base HEAD develop)`
- Committed: `docs(source-refresh-skeleton): point AGENTS.md at the shared refresh skeleton`, message ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

#### Manual Verification:

- In a built app (`npm run tauri build -- --debug --no-bundle`; quit the installed "Nawigator dnia" first, because single-instance hands a second launch to it), "Praca (GitLab)" loads its list, and "Sprawdź teraz" shows the refreshing state and then the list as before
- Without VPN, "Praca (GitLab)" shows the calm unavailable card with the same "Odświeżam co N min — następna próba za M min (HH:MM)." text as before, and the minutes count down to the next round
- In the built app, "Projekty (GitHub)" loads its repos, and "Sprawdź teraz" refreshes them as before
- The tray "check" item and the tray status icon behave as before for both sources, and after the PC wakes from sleep both views refresh on their own
- Hiding and showing a GitLab issue, the NEW flag and a new-assignment notice behave as before
- The app log shows the refresh lines with the same targets and texts as before, e.g. `[day_navigator_lib::gitlab_refresh] gitlab: starting -> ok` and `[day_navigator_lib::github_refresh] github: starting -> ok`, and no `[day_navigator_lib::source_refresh]` line
- The DevTools console of the built app shows no CSP violation and no "IPC custom protocol failed"

**Implementation Note**: Leave the Manual rows unchecked. The user checks them later in the integrated build. The same forbidden list as in Phase 1 applies.

---

## Testing Strategy

### Unit Tests:

- All existing tests, unchanged: 33 in `gitlab_refresh.rs`, 3 in `github_refresh.rs`, plus the rules tests in `gitlab/mod.rs` and `github/mod.rs` that cover `last_success_ms` and the carry-over of `next_refresh_at_ms`. A behaviour change in a moved path shows up as a failing test.
- New tests in `source_refresh.rs`:
  - `run_loop` coalescing and `run_loop` timeout, each with a round-call cap of 10 that panics;
  - the `publish_with` seq guard;
  - `begin_refresh` for both snapshots;
  - the `next_refresh_at_ms` range.

### Integration Tests:

- None added. No integration test references the refresh modules, and the thread, `emit` and tray paths need a running Tauri app, so they are covered by the Manual rows.

### Manual Testing Steps:

1. Quit the installed app, build with `npm run tauri build -- --debug --no-bundle` and run the debug exe from `src-tauri/target/debug/`. F-01 has no migration, so the app data store is not changed, but backing up `day-navigator.db` first is still cheap.
2. Open "Praca (GitLab)" and "Projekty (GitHub)", click "Sprawdź teraz" in each, and use the tray "check" item.
3. Disconnect the VPN, then check the GitLab unavailable card and its next-try text.
4. Open the app log (app log dir) and compare the refresh lines and their targets with a log from 0.11.0.

## Performance Considerations

None. The same locks, the same clones and the same number of emits; generic code is monomorphised per source.

## Migration Notes

None. No store schema change, no settings change and no TS contract change. A rollback is a revert of the phase commits.

## Hand-off to later slices

F-01 delivers less than the roadmap's F-01 outcome lists, on purpose (Defaults 1–3). Later slices read this, not the roadmap wording:

- **GitHub interval and next try.** F-01 gives GitHub only a round that returns its `Duration` (still `REFRESH_INTERVAL`) and the shared `next_refresh_at_ms(wait)` helper. S-02 adds the `GithubSnapshot` fields (`refreshMinutes`, `nextRefreshAtMs`, `refreshSettingInvalid`), their carry-over in `github::next_snapshot`, the setting and the TS type itself.
- **`lastSuccessMs`** stays in the tauri-free rules `next_snapshot` (`gitlab/mod.rs`, `github/mod.rs`); the skeleton does not set it.
- **Wake after sleep and "Check now"** stay in `runtime.rs`, through each state's `wake()`; the skeleton only owns the wake channel and the coalescing loop.

`change.md` Notes carry the same scope line. `context/foundation/roadmap.md` is not edited (the orchestrator owns it).

## Shared files / merge notes

Parallel slices in other worktrees, merged into `develop` later:

- **`src-tauri/src/lib.rs`**: F-01 adds one line, `pub mod source_refresh;`, in the `mod` list (near `lib.rs:15-16`). S-05 (`gitlab-view-memory`) adds new commands in `invoke_handler` (`lib.rs:198+`). These are different hunks, so the textual merge is trivial. Keep both.
- **`AGENTS.md`**: F-01 adds one bullet in "Project Structure", after the `cli/` bullet. S-05 edits `AGENTS.md` too (its app-data and view-state lines in "Hard Rules"/"Frontend Conventions"). A conflict is unlikely; if one appears, keep both edits.
- **`src-tauri/src/gitlab_refresh.rs`**: F-01's main file. S-04 and S-05 are not expected to touch it (S-04 is UI only, and S-05's new commands go through `lib.rs`). If S-05 does add code here, merge it onto the cell form: its code keeps using `state.snapshot()`, `state.write()`, `publish(&app, &state)` and the `kept_*` fields, which all still exist with the same signatures.
- **`src-tauri/src/store/schema.rs`**: not touched by F-01. S-05 adds migration v14 there.
- **`src/gitlab/GitlabView.tsx`, `src/gitlab/GitlabListViewProvider.tsx`**: not touched by F-01 (S-04, S-05).
- **`context/foundation/roadmap.md`**: not touched by F-01. The orchestrator updates the status.

## Defaults chosen by agent

The owner could not be asked. Each item is the safest option and is marked "default chosen by agent".

1. `lastSuccessMs` stays in the rules `next_snapshot` functions. Default chosen by agent (carried over from research).
2. GitHub gets no `nextRefreshAtMs`/`refreshMinutes` fields in F-01. Its round returns `Duration` and `next_refresh_at_ms` exists for S-02. Default chosen by agent (carried over from research).
3. Wake after sleep and "Check now" stay in `runtime.rs` through each state's `wake()`, with no `wake_sources` helper and no `runtime.rs` change. Default chosen by agent (carried over from research).
4. Option A (thin skeleton, thick source) is the seam. GitLab's gate, kept mutexes and rules stay in `gitlab_refresh.rs`. Default chosen by agent (carried over from research).
5. Log lines, thread names, event names and Polish texts stay byte-identical. That includes the log target: skeleton log calls pass the source's `module_path!()`. Default chosen by agent (carried over from research and extended with the log-target finding).
6. The cell keeps the event name, thread name, log prefix and log target as one `SourceId` const per source, so `publish` takes no per-call event. Default chosen by agent (resolves the research open question).
7. GitHub's round already returns `Duration` (= `REFRESH_INTERVAL`) in F-01. Default chosen by agent (resolves the research open question).
8. One bullet is added to `AGENTS.md` "Project Structure", away from the lines S-05 changes. Default chosen by agent (resolves the research open question).
9. "No behaviour change" for the untested paths is checked by new skeleton unit tests plus the Manual rows in the built app. Default chosen by agent (resolves the research open question).
10. `state_label` and `run_fetch` stay per source and are not shared. Default chosen by agent.
11. GitHub moves first (Phase 1), so the skeleton API is proven on the simple round before GitLab's gated round. Default chosen by agent.
12. Complexity MEDIUM with 0 questions: research and the fixed orchestrator decisions settle every solution choice, and the remaining choices are recorded here. Default chosen by agent.
13. The `run_loop` tests cap the round at 10 calls and panic above it. Default chosen by agent (plan review F7).
14. Skeleton tests use invented `SourceId` values, never the real event or thread names. Default chosen by agent (keeps the identity-literal check of plan review F1 at one line each).
15. The test-body diff check and the log-target count start their range at `^mod tests`, and no `#[cfg(test)]` item is allowed in a parent scope. Default chosen by agent (plan review F4: both fixes, not one).
16. The scope line for S-02 goes into `change.md` Notes now, during planning, not at archive time. Default chosen by agent (plan review F6; S-02's planner may read `change.md` before F-01 is archived).

## Revision after plan review

Revised after `reviews/plan-review.md` (verdict REVISE, 2026-11-27). Every new check command was run against the current tree, or against a temporary edit that was reverted:

- F1: row 2.6 now counts quoted literals with `grep -nF` in `gitlab_refresh.rs`, `github_refresh.rs` and `source_refresh.rs` only, one line each, and states that `gitlab/mod.rs:205` / `github/mod.rs:130` keep their own copy. Row 2.6 title adjusted to match.
- F2: rows 1.3/2.3 count only log-call targets (`target: *(self\.)?(id\.)?target\b`), so the `SourceId.target` field no longer counts.
- F3: rows 1.4/2.4/3.3 use `git diff --numstat` (expect `1	0	src-tauri/src/lib.rs` / `1	0	AGENTS.md`) plus a `-U0` content grep.
- F4: the test-import rule allows only `use` lines inside `mod tests` and forbids parent-scope `#[cfg(test)]` items; the diff check is anchored on `^mod tests`.
- F5: Phase 1 §3 and Phase 2 §2 name the leftovers to remove and the names to keep.
- F6: new "Hand-off to later slices" section, and the same line in `change.md`.
- F7: the `run_loop` tests cap the round at 10 calls and panic above it.

Phases, contracts and Manual rows are otherwise unchanged.

## References

- Related research: `context/changes/source-refresh-skeleton/research.md`
- Roadmap item: `context/foundation/roadmap.md` (F-01, MS-28), GitHub issue #28
- Copy being removed: `src-tauri/src/github_refresh.rs:35-278`, `src-tauri/src/gitlab_refresh.rs:70-126, 391-890`
- Lock order: `src-tauri/src/gitlab_refresh.rs:9-15`; round tail order: `src-tauri/src/gitlab_refresh.rs:710-736`
- Log format with target: `tauri-plugin-log` 2.10 `src/lib.rs:517-523`; builder `src-tauri/src/lib.rs:151-159`
- Prior slices: `context/archive/2026-11-23-github-assigned-issues/plan.md`, `context/archive/2026-11-23-gitlab-assigned-list/plan.md`, `context/archive/2026-11-24-gitlab-filter-bar-retry/plan.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Skeleton module and GitHub on it

#### Automated

- [x] 1.1 All gates pass in the worktree — 8564f5e
- [x] 1.2 Existing test bodies are unchanged — 8564f5e
- [x] 1.3 Every skeleton log call carries a target — 8564f5e
- [x] 1.4 `lib.rs` differs from the merge base by exactly `+pub mod source_refresh;` — 8564f5e
- [x] 1.5 Nothing outside the scope changed — 8564f5e
- [x] 1.6 The new skeleton tests exist and pass — 8564f5e
- [x] 1.7 Committed: `refactor(source-refresh): add shared refresh skeleton and move GitHub onto it` — 8564f5e

### Phase 2: GitLab on the skeleton

#### Automated

- [ ] 2.1 All gates pass in the worktree
- [ ] 2.2 Existing test bodies are unchanged
- [ ] 2.3 Every skeleton log call carries a target
- [ ] 2.4 `lib.rs` still differs from the merge base by exactly `+pub mod source_refresh;`
- [ ] 2.5 Nothing outside the scope changed
- [ ] 2.6 Identity strings each appear exactly once in the refresh files
- [ ] 2.7 No copied mechanics remain in the source modules
- [ ] 2.8 Committed: `refactor(source-refresh): move GitLab refresh onto the shared skeleton`

### Phase 3: Agent pointer and final checks

#### Automated

- [ ] 3.1 All gates pass in the worktree
- [ ] 3.2 Existing test bodies are unchanged
- [ ] 3.3 `AGENTS.md` differs from the merge base by exactly one added line
- [ ] 3.4 The whole change touches only the planned files
- [ ] 3.5 Committed: `docs(source-refresh-skeleton): point AGENTS.md at the shared refresh skeleton`

#### Manual

- [ ] 3.6 Built app: "Praca (GitLab)" loads and "Sprawdź teraz" refreshes as before
- [ ] 3.7 Without VPN: GitLab calm unavailable card with the same next-try text
- [ ] 3.8 Built app: "Projekty (GitHub)" loads and "Sprawdź teraz" refreshes as before
- [ ] 3.9 Tray "check", tray status and wake after sleep behave as before for both sources
- [ ] 3.10 GitLab hide/show, NEW flag and new-assignment notice behave as before
- [ ] 3.11 App log shows the same refresh lines and targets, no `source_refresh` target
- [ ] 3.12 Built app DevTools console: no CSP violation, no "IPC custom protocol failed"
