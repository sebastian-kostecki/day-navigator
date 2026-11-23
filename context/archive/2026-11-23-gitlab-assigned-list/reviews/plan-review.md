<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Assigned GitLab issues and MRs ("Praca (GitLab)")

- **Plan**: context/changes/gitlab-assigned-list/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 2 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS (1 observation) |
| Architectural Fitness | WARNING |
| Blind Spots | PASS (1 observation) |
| Plan Completeness | WARNING |

## Grounding
18/18 paths ✓, 8/8 symbols ✓ (CliCall::ALL, HealthState.wake, probe_round, SourceState, load_settings, CREATE_NO_WINDOW, classify 401 markers, glab --paginate → single array), line refs ✓, brief↔plan ✓, Progress↔Phase ✓

## Findings

### F1 — GitLab host tied to the all-or-nothing settings parse

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architectural Fitness
- **Location**: Phase 1 §1, Phase 1 §3, Phase 2 §1
- **Detail**: `parse_settings` is all-or-nothing (settings.rs:104-122), and its single result feeds `health::probe` (probe.rs:17-22), VaultState (lib.rs:179) and `task_cfg` (lib.rs:61). An invalid host returned as `VaultError::SettingsInvalid` would break the vaults and the "Zadania" view. A vault-side mistake would drop the GitLab list. One broken file would give two red sources and two toast lines.
- **Fix A ⭐ Recommended**: Parse the `gitlab` block in its own `load_gitlab_settings`; `Settings` and `load_settings` stay unchanged.
  - Strength: Errors stay in their own source; no duplicate toast; vault contract untouched.
  - Tradeoff: settings.json is read twice per round; the version check is duplicated.
  - Confidence: HIGH — the coupling is visible in probe.rs:17-22 and lib.rs:61/179.
  - Blind spot: Malformed JSON still shows up in both sources (arguably correct).
- **Fix B**: Keep the single parse, make GitLab silent on SettingsInvalid, and document the coupling.
  - Strength: Smallest change.
  - Tradeoff: A GitLab typo still breaks the tasks view.
  - Confidence: MED — a policy choice.
  - Blind spot: None significant.
- **Decision**: FIXED (Fix A)

### F2 — Order of GitlabState vs. the health thread is not specified

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §2
- **Detail**: `app.state::<GitlabState>()` panics when the state is not managed. The health thread starts in `runtime::start` and runs its first round right away. The plan said only "next to runtime::start", and it did not say how `probe_round` receives the GitLab status.
- **Fix**: Manage `GitlabState` before `runtime::start` (the `build_tray` split); `probe_round` takes `Option<SourceStatus>`, computed in `check_round`.
- **Decision**: FIXED

### F3 — "No echo of the host" is not true for a wrong-type `gitlab` value

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §1
- **Detail**: `"gitlab": "<host>"` (a string instead of an object) would go through `bad settings shape: {e}` (settings.rs:113), and serde_json quotes the value there, so the hostname reaches the panel.
- **Fix**: Map every error of the GitLab loader to `GitlabError::SettingsInvalid` with no detail, and add a "gitlab as a string" case to the no-echo test (comes with F1 Fix A).
- **Decision**: FIXED

### F4 — Two rounds back to back at startup

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Lean Execution
- **Location**: Phase 3 §3 (useGitlab)
- **Detail**: The default view's on-mount wake arrives during the thread's first round, so a second round runs right after it: 4 `glab` processes instead of 2.
- **Fix**: Refresh on mount only when the snapshot is neither `starting` nor `refreshing`.
- **Decision**: FIXED
