<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Shared source refresh skeleton (F-01)

- **Plan**: context/changes/source-refresh-skeleton/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 3 observations

Run non-interactively: triage was not offered; every finding stays `PENDING` for the owner.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

## Evidence

### Behaviour preserved (line-by-line comparison with the merge base)

- **GitLab lock order and gate.** In `refresh_round` only the planned lines changed: the "refreshing" block became `state.cell.begin_round(app)` (`gitlab_refresh.rs:546`), the next-try line became `source_refresh::next_refresh_at_ms(wait)` at the same spot (`:701`), the todo emit/notify became `SOURCE.notify_todo_changed` (`:705`), and the tail logs plus health wake became `SOURCE.end_round` after the toasts (`:713`). The order `next_refresh_at_ms` → store → `drop(gate)` → `publish` → todo notify → toasts → logs/health wake is unchanged, so the elapsed time is still measured after the toasts. No `kept_*`, `shown_until`, `last_mark` or `hidden_gate` access moved.
- **Seq-ordered publication.** `SourceCell::publish_with` locks `published`, re-reads the snapshot, skips `seq <= last`, sets `last` and emits while still holding `published`, the same as the old `publish`. The emit payload is `&S` in both versions, so it serializes the same.
- **`begin_round`** drops the write guard before it publishes, as before.
- **ThreadDown stand-in.** `start_thread` spawns on `id.thread`; on error it logs first, computes the stand-in from the current snapshot, stores it, publishes and wakes health, the same order as before. `start` now looks up `app.state::<…RefreshState>()` before spawning instead of only on failure. This is safe: `lib.rs:170-171` manages both states before `lib.rs:178-179` starts them, and the refresh thread looks the same state up on its first round anyway (see F2).
- **Wake coalescing.** `run_loop` is the old loop verbatim (`recv_timeout(wait)`; `Ok` drains with `try_recv`; `Timeout` loops; `Disconnected` returns). GitLab keeps `resume_after` and `session_seen` as loop locals captured `&mut` by the `FnMut` closure; GitHub's round returns `REFRESH_INTERVAL`, which is what it waited on before. GitHub's round has no early return.
- **Health wake** fires only on `before_status != after_status` (round) and on thread-start failure, as before.
- **"To do" reconcile logging.** `log_reconcile` keeps the three branches (debug on `changed > 0`, debug on `TodoError::Store(_)`, warn otherwise) and the return value.
- **Toasts** stay in `gitlab_refresh.rs`, after the publish and the todo notify, under no lock.

### Log lines

- All 8 log calls in `source_refresh.rs` (outside `mod tests`) pass `target: self.target` / `target: self.id.target`; the counts are 8 and 8.
- The texts equal the old literals: `"{prefix}: emit failed: {e}"`, `"{prefix}: cannot start refresh thread: {e}"`, `"todo: emit failed: {e}"`, `"{prefix}: {from} -> {to}"`, `"{prefix}: round {to} in {} ms"`, `"todo: reconcile changed {changed} rows"`, `"todo: reconcile failed: {kind}"` (debug/warn). `from`/`to` are `String`s passed as `&String`; the output is unchanged.
- `SOURCE.target` is `module_path!()` evaluated in each source module, so it is `day_navigator_lib::gitlab_refresh` / `day_navigator_lib::github_refresh`. `tauri-plugin-log` 2.10.2 prints and filters on `record.target()` (`lib.rs:468, 521`), and the `level_for("day_navigator_lib", Info)` filter (`src-tauri/src/lib.rs:159`) still matches these targets.

### Contracts and scope

- No Tauri command, managed-state public method or TS type changed. `snapshot()`, `wake()`, `write()` and `publish(app, state)` keep their signatures, so `runtime.rs` and `hidden_issues/commands.rs` are untouched.
- No store migration, no `store/schema.rs`, `src/`, `gitlab/`, `github/`, `runtime.rs`, `hidden_issues/`, `todo/` or `tests/` change (scope check empty).
- `lib.rs` differs by exactly `+pub mod source_refresh;`; `AGENTS.md` by exactly the one planned bullet, after the `cli/` bullet.
- Existing test bodies unchanged: the plan's diff check prints `gitlab_refresh tests unchanged` and `github_refresh tests unchanged`, and even the `use` lines in both `mod tests` are identical to the merge base.
- No parent-scope `#[cfg(test)]` item and no `#[allow(...)]` in the three files.
- AGENTS.md compliance: no new `glab`/`gh` call, no secrets, logs carry only the prefix, state kinds and elapsed time; tests use invented `SourceId` values (`test-changed`, `test-refresh`, `test`, `test_target`).

### Skeleton tests

- `run_loop_coalesces_queued_wakes`: without the drain it would run 4 rounds (fails `assert_eq!(…, 2)`); without the `Disconnected` return it hits the cap of 10 and panics. The only sender is dropped inside the round, so the 1 h wait cannot block.
- `run_loop_loops_on_timeout`: a return on `Timeout` gives 1 round (fails); an ignored `Disconnected` hits the cap.
- `publish_with_emits_only_a_newer_seq` covers seq 0, a bump, a repeat and a lower seq.
- `begin_refresh` for both snapshots and the `next_refresh_at_ms` range test are present.
- No test can hang `cargo test`.

### Does the seam ease the later slices (without implementing them)?

- **S-02 (GitHub interval + `nextRefreshAtMs`)**: yes. GitHub's round already returns its `Duration`; S-02 swaps `REFRESH_INTERVAL` for the setting and adds `next.next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait))` before the store, the same line GitLab uses.
- **S-01 (GitHub hiding)**: yes. The cell exposes `write()` and `publish()` separately, so GitHub can copy GitLab's store-under-gate, `drop(gate)`, then publish pattern without fighting the skeleton.
- **S-03 (GitHub NEW flags/notices)**: yes for the loop state (an `FnMut` round capturing `&mut` locals is proven by GitLab). The toast slot between `notify_todo_changed` and `end_round` is a convention only (F3), and `toast_new_assignments` stays GitLab-private, so S-03 decides whether to share it.
- **S-06 (second GitLab MR fetch)**: yes. `run_fetch` stays in the source module, and nothing in the skeleton depends on the fetch shape.

## Findings

### F1 — Thread-start failure and the emit/tail wiring have no automated test

- **Severity**: OBSERVATION
- **Impact**: LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/source_refresh.rs:126 (`start_thread`), :114 (`begin_round`), :92 (`publish`), :163 (`end_round`)
- **Detail**: These paths need an `AppHandle`, and the crate does not enable tauri's `test` feature (`mock_app`), so the plan covers them with Manual rows 3.6-3.11, which are still unchecked. The review compared them line by line with the merge base and found them equivalent; the seq guard they rely on is unit-tested via `publish_with`. This was accepted in the plan ("Integration Tests: None added").
- **Fix**: None now. The owner checks Manual rows 3.6-3.12 in the integrated build before archive.
- **Decision**: ACCEPTED (no change)

### F2 — `start` now resolves the managed state before spawning

- **Severity**: OBSERVATION
- **Impact**: LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/gitlab_refresh.rs:455, src-tauri/src/github_refresh.rs:117
- **Detail**: Before, `app.state::<…RefreshState>()` was called only in the spawn-error branch. Now it is called on every start, because `start_thread` is a method of the cell. If `start` ever ran before `manage`, the app would panic at startup instead of in the refresh thread's first round. Today `lib.rs:170-171` manages both states before `lib.rs:178-179` starts them, so the observable behaviour is unchanged. This is the intended consequence of the planned API, not drift.
- **Fix**: None needed. Keep `manage` before `start` in `lib.rs` (as the existing doc on `start` already requires for `HealthState`).
- **Decision**: ACCEPTED (no change)

### F3 — The source-only tail slot is not written down on `end_round`

- **Severity**: OBSERVATION
- **Impact**: LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src-tauri/src/source_refresh.rs:163
- **Detail**: GitLab runs its toasts between `notify_todo_changed` and `end_round`, so the elapsed debug line includes them. Nothing on `end_round` says it must be the last call of a round, or that source-only steps (S-03's GitHub notices) go before it. A later slice could call `end_round` before its toasts and silently change the elapsed figure and the health-wake timing.
- **Fix**: Optionally add one doc sentence to `end_round`: "Call it last in a round, after any source-only step such as toasts; the elapsed time includes them."
- **Decision**: FIXED in the doc comment of `SourceId::end_round` (source_refresh.rs)

## Success criteria run

| Check | Result |
|---|---|
| `npm run lint`, `npm run format:check`, `npm run build` | pass |
| `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` | pass (0 warnings; 644 unit tests incl. 33 `gitlab_refresh`, 3 `github_refresh`, 6 new `source_refresh`; all integration suites ok) |
| Existing test bodies unchanged (plan diff check) | `gitlab_refresh tests unchanged`, `github_refresh tests unchanged` |
| Skeleton log calls with target | 8 / 8 |
| `lib.rs` numstat | `1 0 src-tauri/src/lib.rs`, line `+pub mod source_refresh;` |
| Scope check (`src/`, `store/`, `gitlab/`, `github/`, `runtime.rs`, `hidden_issues/`, `todo/`, `tests/`) | empty |
| Identity literals | each `1` |
| Copied mechanics left (`recv_timeout`, `thread::Builder`, `published`) outside tests | none |
| `AGENTS.md` numstat | `1 0 AGENTS.md` |
| Changed files | only the planned ones |
| Manual rows 3.6-3.12 | pending (owner, integrated build) |
