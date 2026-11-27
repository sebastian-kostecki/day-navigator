<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Shared source refresh skeleton (F-01) Implementation Plan

- **Plan**: context/changes/source-refresh-skeleton/plan.md
- **Mode**: Deep
- **Date**: 2026-11-27
- **Verdict**: REVISE
- **Findings**: 1 critical, 3 warnings, 3 observations

The design is sound: thin skeleton, thick source, GitHub first. Every risky claim checked against the code holds. All findings except F6 are about the verification commands in the Success Criteria. Three of them give a wrong result on correct code, and one rule in the plan breaks the test-body diff check if followed. An unattended implementer cannot tick those Progress rows without improvising, so fix them before `/10x-implement`.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | PASS |
| Plan Completeness | FAIL |

## Grounding

Grounding: 6/6 paths ✓ (`gitlab_refresh.rs`, `github_refresh.rs`, `runtime.rs`, `lib.rs`, `gitlab/mod.rs`, `github/mod.rs`; `source_refresh.rs` correctly absent), 12/12 symbols ✓, brief↔plan ✓. The test counts are right: 33 tests in `gitlab_refresh.rs` and 3 in `github_refresh.rs`.

Claims verified against the code:

- **GitLab gate and lock order are kept.**
  - The snapshot lock stays last in the documented order (`gitlab_refresh.rs:9-15`).
  - `next_refresh_at_ms` → store → `drop(gate)` → publish is at `:711-714`. The tail order (todo emit/notify → toasts → info/debug logs → health wake) is at `:715-735`.
  - `begin_round` (flip under a write guard, guard released, then publish) matches `:551-556`.
  - `start_thread` (log, compute from snapshot, store, publish, health wake) matches `:445-462` and `github_refresh.rs:106-124`.
  - The one difference: `app.state::<…RefreshState>()` is now resolved before the spawn, not only in the error branch. This is harmless, because `manage` always runs before `start` (`lib.rs:169-178`).
- **Test reachability.**
  - Tests use only methods (`new`, `snapshot`, `wake`, `write`, `accept_round`, …), the `kept_*`/`shown_until`/`last_mark` fields, and free functions that stay in place.
  - The only parent `use` names the test modules need are `HashSet` and `now_ms` in GitLab. Both stay used by non-test code.
  - No test reads `snapshot`, `wake`, `published` or `hidden_gate` directly, and no test builds a state with a struct literal (`kept_state` uses `GitlabRefreshState::new()`, `:1328-1329`).
  - So the test bodies can stay unchanged.
- **Log target.**
  - `tauri-plugin-log` 2.10.2 prints `record.target()` (`lib.rs:517-523`).
  - `log` 0.4.35 (`Cargo.lock`) accepts `target: $target:expr`, so `SOURCE.target` / `self.id.target` work.
  - `module_path!()` in the source's const expands to the source module path.
  - `level_for("day_navigator_lib", Info)` (`src-tauri/src/lib.rs:158`) still matches by prefix.
  - The approach keeps log lines byte-identical.
- **Dead code.**
  - In Phase 1 every skeleton item has a non-test caller in GitHub (`write` through the store in `refresh_round`, `thread` through `start_thread`, and so on).
  - Holding `next_refresh_at_ms` back to Phase 2 is correct.
- **Visibility.** `crate::events::commands::notify` is `pub fn`, `crate::todo::CHANGED_EVENT` is `pub const`, and `crate::runtime::HealthState::wake` is `pub`. All three are reachable from `crate::source_refresh`.
- **Blast radius.**
  - Outside callers use only `snapshot()`, `wake()`, `publish(app, &state)`, `manage`, `start` and the commands: `hidden_issues/commands.rs:17,47`, `runtime.rs:133-134,262-263,428-429,514-515`, `todo/commands.rs:143,163`. All of these keep their signatures.
  - The frontend listens on `"gitlab-changed"` / `"github-changed"` (`src/gitlab/useGitlab.ts:42`, `src/github/useGithub.ts:29`), and those strings are kept.
- **Pattern check.** `runtime.rs` `HealthState` (`:47-77`, `publish` `:347-357`, loop `:400-414`) has the same cell shape. Its `publish` is tied to the tray and health-only fields, so it cannot reuse `SourceCell` in this change. It could adopt it later. This is not a finding.
- **Diff check.** The test-body diff command from Phase 1, run now against `develop`, prints `gitlab_refresh tests unchanged` and `github_refresh tests unchanged`.
- **Later slices.**
  - S-02 gets a round that returns its `Duration` plus the `next_refresh_at_ms` helper.
  - S-03 gets `run_loop` with an `FnMut` round that can capture `&mut` loop locals, plus a post-publish slot before `end_round`.
  - S-01 can store under its own gate and then call `publish`, which is `pub(crate)` and seq-guarded.
  - S-06 keeps `run_fetch` and the round body source-owned.
  - All four are genuinely eased.
- **Phase sizing.** Each phase fits one Sonnet implementer and ends green, once the checks below are fixed.

## Findings

### F1 — Identity-string check cannot pass on correct code

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2, Success Criteria (Progress row 2.6)
- **Detail**:
  - The criterion says each identity string appears "exactly once in non-test code under `src-tauri/src/`", checked with `grep -rn --include=*.rs '<string>' src-tauri/src`.
  - The Polish texts already appear twice each today: `"Wątek odświeżania GitLaba nie działa"` at `gitlab/mod.rs:205` (the `ThreadDown` message) and `gitlab_refresh.rs:404` (the command error), and `"Wątek odświeżania GitHuba nie działa"` at `github/mod.rs:130` and `github_refresh.rs:91`.
  - Unquoted, the event and thread names also match the module docs (`gitlab_refresh.rs:1-2,442`, `github_refresh.rs:1-2,103`).
  - The grep does not exclude test modules either. Row 2.6 can never be ticked honestly.
- **Fix**: Scope the check to the quoted literals in the three refresh files only. For example, `grep -rnF '"gitlab-changed"' src-tauri/src/gitlab_refresh.rs src-tauri/src/github_refresh.rs src-tauri/src/source_refresh.rs` (same for the other five strings), expecting exactly one line each. For the two Polish texts, also state that `gitlab/mod.rs:205` and `github/mod.rs:130` keep their own copy, unchanged.
- **Decision**: PENDING

### F2 — Log-target count also counts the `SourceId.target` field

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 and Phase 2, Success Criteria (Progress rows 1.3 and 2.3)
- **Detail**:
  - The check compares `grep -cE 'log::[a-z]+!\('` with `grep -c 'target:'` on the non-test part of `source_refresh.rs`.
  - The contract itself puts `pub(crate) target: &'static str` in that same part, and any doc comment that mentions `target:` matches too. So on correct code the second count is higher.
  - A dry run on a two-line sample (one struct field, one log call) gives 2 against 1.
- **Fix**: Count only log-call targets, e.g. `grep -cE 'target: *(self\.)?(id\.)?target'`. Better still, require that no line in the non-test part matches `log::[a-z]+!\(` without a `target:` on the same or the next line. Alternatively, keep both counts and state the expected offset: target count = log-call count + 1.
- **Decision**: PENDING

### F3 — AGENTS.md one-line check drops the bullet it should show

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3, Success Criteria (Progress row 3.3)
- **Detail**:
  - `git diff … -- AGENTS.md | grep -E '^[+-][^+-]'` is meant to skip the `+++`/`---` headers. It also skips every added Markdown bullet, because the diff line is `+- …` and its second character is `-`.
  - The new bullet would print nothing, so the check cannot show "exactly one added line". A dry run confirms grep exits 1 on such a line.
  - The `lib.rs` check (`+pub mod …`) is not affected.
- **Fix**: Use `git diff --numstat $(git merge-base HEAD develop) -- AGENTS.md` and expect `1	0	AGENTS.md`. The same `--numstat` form (`1	0	src-tauri/src/lib.rs`) is a sturdier check for rows 1.4/2.4 too.
- **Decision**: PENDING

### F4 — The `#[cfg(test)] use` option breaks the test-body diff check

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Critical Implementation Details — "The rule for test imports"; Success Criteria rows 1.2/2.2/3.2
- **Detail**:
  - The rule allows re-adding a lost name "with a `#[cfg(test)]` `use` in the parent".
  - The diff check starts its range at the first column-0 `#[cfg(test)]` line (`sed -n '/^#\[cfg(test)\]/,$p'`). A parent-level `#[cfg(test)]` before the test module would pull in extra parent lines, or at least one extra `#[cfg(test)]` line. The check would then report a diff on unchanged test bodies.
  - The code check shows no test needs such a rescue (only `HashSet` and `now_ms`, both still used). The option is a trap and helps nothing.
- **Fix**: Drop the parent `#[cfg(test)] use` option and allow only a `use` line inside `mod tests`, which the check already ignores. Alternatively, anchor the sed on `^mod tests` in both the base file and the new file.
- **Decision**: PENDING

### F5 — Phase 2 does not list the leftovers clippy will flag

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §2 (and Phase 1 §3)
- **Detail**:
  - Phase 1 says "Remove `read()` if it is unused" for GitHub. Phase 2 says nothing for GitLab, whose private `read()` (`gitlab_refresh.rs:258`) is used only by `snapshot()` (`:119`). After delegation it becomes `dead_code`, and `clippy -D warnings` fails.
  - In both modules `mpsc` (except `mpsc::Receiver`/`SendError` in signatures), `RecvTimeoutError`, `RwLock`, `RwLockReadGuard`, `Emitter` and `HealthState` become unused. `RwLockWriteGuard`, `Instant`, `Duration` and `now_ms` stay.
  - Clippy will point all of this out, but naming it saves an implementer round trip.
- **Fix**: In Phase 2 §2, add "remove the private `read()` and the imports left unused (`RecvTimeoutError`, `RwLock`, `RwLockReadGuard`, `Emitter`, `HealthState`); keep `RwLockWriteGuard`, `Instant`, `now_ms`, `HashSet`". Add the same list in Phase 1 §3 for GitHub.
- **Decision**: PENDING

### F6 — Plan narrows the roadmap's F-01 outcome without saying so where S-02 will look

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: What We're NOT Doing; Defaults 1–3
- **Detail**:
  - The roadmap outcome (`roadmap.md` F-01) lists interval, wake after sleep, "Check now", `lastSuccessMs` and `nextRefreshAtMs` as moving through the skeleton. S-02's risk note says it is "small once F-01 exposes the interval and `nextRefreshAtMs` for every source".
  - The plan deliberately delivers less, and reasonably so (it stays within the item constraints):
    - `lastSuccessMs` stays in the rules;
    - wake and "Check now" stay in `runtime.rs`;
    - GitHub gets only a `Duration` return and a helper, with no snapshot fields.
  - The defaults record this, but S-02's planner reads the roadmap and `change.md`, whose Notes still state the full outcome.
- **Fix**: At archive time, note in `change.md` that F-01 delivers the `Duration` return and the `next_refresh_at_ms` helper only, and that S-02 adds the `GithubSnapshot` fields and their carry-over. Do not edit the roadmap; that is the orchestrator's job.
- **Decision**: PENDING

### F7 — `run_loop` tests can hang instead of failing on a broken loop

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §2 (skeleton unit tests)
- **Detail**:
  - If `run_loop` ignores `Disconnected` and keeps looping, `recv_timeout` on a closed channel returns at once on every call.
  - The 1 h round then spins forever, and `cargo test` hangs instead of reporting a failure.
  - The tests themselves are designed correctly: 3 queued wakes give exactly 2 rounds when the drain works and 4 when it does not.
- **Fix**: Let the test round panic once it has been called more than a small cap (for example 10 times), so a broken loop fails fast.
- **Decision**: PENDING
