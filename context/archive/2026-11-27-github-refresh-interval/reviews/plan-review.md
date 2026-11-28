<!-- PLAN-REVIEW-REPORT -->
# Plan Review: S-02 github-refresh-interval Implementation Plan

- **Plan**: context/changes/github-refresh-interval/plan.md (reviewed at b3a3e00)
- **Mode**: Deep
- **Date**: 2026-11-27
- **Verdict**: SOUND
- **Findings**: 0 critical, 1 warning, 1 observation

Non-interactive run: the triage decisions below were made by the agent ("default chosen by agent"). Both findings were accepted and applied to the plan as minimal edits.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | PASS |
| Plan Completeness | WARNING |

## Grounding

Grounding: 9/9 paths ✓, 12/12 symbols ✓, brief↔plan ✓. Progress↔Phase ✓: one `## Progress`, 3/3 phases, 18/18 criteria, no checkboxes in the phase blocks. `docs/reference/contract-surfaces.md` does not exist, so that check was skipped. No `context/foundation/lessons.md`.

Verified against the worktree code:

- `settings.rs:401-460`. `DEFAULT_REFRESH_MINUTES` and `REFRESH_MINUTES_MAX` are confirmed. `parse_gitlab_refresh_minutes` is hard-wired to `"gitlabRefresh"`, has only one caller (`:432`) and is private. `RawGitlabRefresh` is used only at `:451`. The file-level problems that give `Ok(10)` are confirmed. `load_github_settings` (`:157-175`) reports the file problems as `SettingsInvalid`.
- `github_refresh.rs`. `REFRESH_INTERVAL` is defined at `:39` and used only at `:195`. `refresh_round` (`:163-196`) has no early return or `?`: every path, including `NotConfigured`, `ConfigDirUnresolved` and `SettingsInvalid`, goes through `next_snapshot` and reaches `*state.write() = next` (`:188`). That makes the planned order sound: flags after `next_snapshot`, then `next_refresh_at_ms` just before the store, then return `wait`.
- `ThreadDown`. `start_thread` (`source_refresh.rs:126-142`) runs once at start-up. Its stand-in is built from `starting()` (`github_refresh.rs:121-125`), so with carry-over `nextRefreshAtMs` is `null`, as the plan states.
- `GithubSnapshot` has exactly two full struct literals, both in `github/mod.rs`: `starting()` at `:247` and `next_snapshot` at `:291`. The other four constructions in tests (`github_refresh.rs:240`, `health/probe.rs:302` and `:347`, `todo/commands.rs:244`) use `..starting()` / `..snap()` and still compile. No destructuring pattern exists. There is no TS object literal of `GithubSnapshot`: `useGithub.ts` starts from `null`.
- GitLab model. The plan's citations match: `gitlab_refresh.rs:515-527` (`refresh_interval`), `:537-538`, `:648-649` and `:700-714`. `gitlab/mod.rs:20` imports `DEFAULT_REFRESH_MINUTES`. `retryText` (`GitlabView.tsx:68-81`) is the only user of `formatClock` in that file (`:19`, `:80`). `components/index.ts:34` has the separate `export { formatClock } from "./time";` line.
- `GithubView.tsx`. The calm "GitHub niedostępny" card has no children (`:209-218`). `SourceError` is `:99-132`. `ErrorState.message` is inline `ReactNode` inside a `<p>`.

## Findings

### F1 — `retry` name collides with `useGithub()`'s `retry`

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 — #2 GitHub view
- **Detail**: The plan says to "compute `retry = retryText(...)`" in `GithubView` and to give `SourceError` a prop named `retry`. `GithubView` already declares `retry` as the reload callback: `const { snapshot, loadError, refresh, refreshError, retry } = useGithub();` (`src/github/GithubView.tsx:135`), which the `loadError` `ErrorState` uses as `onRetry={retry}`. A second `const retry` in the same function scope fails `tsc` ("Cannot redeclare block-scoped variable"). An implementer who renames the hook's value instead would churn unrelated code.
- **Fix**: Name the value `retryNote`, computed right after `nowMs` (which is after the early returns, where `snapshot` is non-null), and name the `SourceError` prop `retryNote: string | null`.
- **Recommendation**: ACCEPT. Following the plan literally does not compile, and the fix is a rename only.
- **Decision**: FIXED (applied to plan.md Phase 3 #2; default chosen by agent)

### F2 — `git diff --stat` criteria ignore the plan's own Progress edits

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 criterion 1.4, Phase 2 criterion 2.5, Phase 3 criterion 3.4
- **Detail**: The criteria say the diff shows "no change outside `src-tauri/src/settings.rs`" or "changes only in" a fixed file list. The implementer also ticks the Progress rows in `plan.md`, often in the same commit, so a literal check fails, or an automated implementer starts doubting a green phase.
- **Fix**: Add "(apart from this plan's Progress rows)" to the three criteria. The Progress titles stay unchanged.
- **Recommendation**: ACCEPT. It is a wording-only change that removes a false failure in a mechanical gate.
- **Decision**: FIXED (applied to plan.md; default chosen by agent)

## Notes (no finding)

- **Lean execution.** The private copy of `refresh_interval` in `github_refresh.rs` duplicates GitLab's nine lines. The plan justifies it explicitly: `gitlab_refresh.rs` is edited by S-06. The validation itself is shared through the keyed parser, so it is acceptable.
- **Merge awareness.** S-01 appends fields to `GithubSnapshot` and may gate the store in `refresh_round`. The Merge Points table names both points and the rule to keep `next_refresh_at_ms` right before the store. Nothing more is needed in this plan.
- **Error-state retry text.** It is correct for `LoggedOut`, `SettingsInvalid` and `ConfigDirUnresolved`, because rounds keep running for those kinds. For a file-level settings problem the GitHub refresh loader returns `Ok(10)`, so "co 10 min" matches the real wait. Suppressing it only for `ThreadDown` is right.
