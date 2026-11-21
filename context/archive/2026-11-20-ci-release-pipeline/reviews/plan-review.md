<!-- PLAN-REVIEW-REPORT -->
# Plan Review: CI and Release Pipeline Implementation Plan

- **Plan**: context/changes/ci-release-pipeline/plan.md
- **Mode**: Deep
- **Date**: 2026-11-20
- **Verdict**: REVISE (after fixes: SOUND)
- **Findings**: 2 critical, 3 warnings, 4 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | FAIL |
| Plan Completeness | WARNING |

## Grounding
7/7 paths ✓, 6/6 symbols ✓, brief↔plan ✓, Progress↔Phases ✓. External claims checked against source code: Tauri 2 `version` fallback to Cargo.toml ✓, `tauri-action@v1` exists (v1.0.0, node24) ✓, `__VERSION__` resolved from Cargo ✓, `--bundles nsis` uploads only `-setup.exe` ✓, gitleaks needs no license on a personal account ✓, `main` concurrency queues without cancelling ✓.

## Findings

### F1 — Denylist saved with CRLF/BOM makes the hook fail open

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 — Pre-push hook contract; criterion 2.3
- **Detail**: A denylist saved on Windows with CRLF line endings gives patterns ending in `\r` that never match. A BOM hides the first term. A comments-only file has zero terms. In all three cases the push passes silently. The self-test uses an LF temp file and cannot catch this, and criterion 2.3 has the same flaw.
- **Fix**: The hook normalizes the denylist (BOM, `\r`, whitespace) and blocks when no term remains. The self-test covers CRLF+BOM and comments-only files. Criterion 2.3 normalizes too.
- **Decision**: FIXED

### F2 — gitleaks-action breaks on the first push (root commit, Node 20)

- **Severity**: ❌ CRITICAL
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 3 — `checks` job; Phase 4 criteria 4.2/4.3
- **Detail**: On push the action scans `--first-parent --no-merges <commits[0]>^..<head>`. On the first push, `<root>^` does not exist, the job fails (gitleaks-action#199), and `release` never runs. Merge commits are skipped. v2 declares node20, which GitHub removed on 2026-09-23.
- **Fix A ⭐ Recommended**: gitleaks CLI over the full history (`gitleaks git --redact`) instead of the action.
  - Strength: Scans the root commit and merges; seconds on a small repo; no Node runtime.
  - Tradeoff: The binary is pinned by hand.
  - Confidence: HIGH — the failure is in the action's range logic.
  - Blind spot: Whether taiki-e/install-action ships gitleaks is unverified.
- **Fix B**: `@v3` plus a newer `GITLEAKS_VERSION`, accepting the scan holes.
  - Strength: Smallest diff.
  - Tradeoff: The root commit and merges go unscanned.
  - Confidence: MED.
  - Blind spot: v3 has not been tried on this repo.
- **Decision**: FIXED (Fix A)

### F3 — Version check fails open into a silent overwrite

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 — `version` job
- **Detail**: `gh release view` exits 1 both for "release not found" and for 5xx/network/401. Checking the exit code alone sets `publish=true` on an API error, and tauri-action then overwrites the existing release's name, body and asset.
- **Fix**: `publish=true` only when stderr contains `release not found`. Any other failure fails the job.
- **Decision**: FIXED

### F4 — A failed upload leaves a published release with no installer

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 3 — `release` / `version` jobs
- **Detail**: tauri-action creates the non-draft release before uploading, with no retry and no rollback. After a failed upload, every later run sees the release, skips it, and stays green.
- **Fix A ⭐ Recommended**: `version` publishes when the release is missing OR has no asset whose label ends in `-setup.exe`.
  - Strength: Self-healing; tauri-action reuses the existing release.
  - Tradeoff: One extra jq expression.
  - Confidence: HIGH.
  - Blind spot: A partly uploaded asset is not detected.
- **Fix B**: Manual recovery written in AGENTS.md, plus `retryAttempts: 2`.
  - Strength: No logic in the workflow.
  - Tradeoff: The failure stays silent.
  - Confidence: HIGH.
  - Blind spot: Relies on a human noticing.
- **Decision**: FIXED (Fix A)

### F5 — Asset name criterion will fail: GitHub renames spaces

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Desired End State; criterion 4.3
- **Detail**: The asset `name` becomes `Nawigator.dnia_…`; the original name with the space survives only in `label` (tauri-action `utils.ts`, `upload-release-assets.ts`).
- **Fix**: 4.3 checks `.assets[0].label`; Desired End State names both forms.
- **Decision**: FIXED

### F6 — Relative core.hooksPath runs no hook when the file is absent

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 manual 2.4; Phase 4 §1
- **Detail**: On a branch or worktree that predates `.githooks/pre-push`, git silently skips the hook.
- **Fix**: A pre-flight `test -x .githooks/pre-push` before `gh repo create`; a note in AGENTS.md and Critical Implementation Details.
- **Decision**: FIXED

### F7 — "Denylist line <n>" needs a per-term loop; diacritics fold by locale

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 — hook contract
- **Detail**: `grep -F -f` does not report which pattern matched, and line numbers shift after comments are stripped. `-i` folds Ł/ł only in a UTF-8 locale.
- **Fix**: A loop over the original numbered lines, `LC_ALL=C.UTF-8`, and line-number and diacritic cases in the self-test.
- **Decision**: FIXED

### F8 — Foundation docs still promise "every merge publishes"

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: What We're NOT Doing; tech-stack.md:24; roadmap F-03
- **Detail**: The plan narrows publishing to version bumps but leaves the foundation docs unchanged.
- **Fix**: Phase 3 §3 updates tech-stack.md and the roadmap outcome.
- **Decision**: FIXED

### F9 — Self-test on the CI runner has no git identity

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §2 / Phase 3 `checks`
- **Detail**: ubuntu-latest has no user.name/user.email, and the default branch name varies.
- **Fix**: The script passes `-c user.name/-c user.email` and `git init -b main`.
- **Decision**: FIXED
