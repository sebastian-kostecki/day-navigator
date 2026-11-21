<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: CI and Release Pipeline

- **Plan**: context/changes/ci-release-pipeline/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-21
- **Verdict**: APPROVED
- **Findings**: 0 critical 2 warnings 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | WARNING |

## Automated checks run in this review

- `npm run build` — pass
- `cargo build --locked` / `cargo test --locked` (src-tauri) — pass (56 + 18 + integration tests ok)
- `bash scripts/test-pre-push.sh` — 12 assertions passed
- `git ls-files -s .githooks/pre-push` — `100755`
- `git grep -n "secrets\." .github/` — only `secrets.GITHUB_TOKEN` (2 uses)
- Four-job node check — ok
- Not re-run: `npm run tauri build` (1.4; the CI release job built the NSIS installer) and the `gh` rows 4.1–4.3 (`gh` is not installed in this environment)

## Findings

### F1 — Hook skips added lines that start with `++`

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: .githooks/pre-push:69
- **Detail**: Added lines are extracted with `grep '^+' | grep -v '^+++'`. A new line whose content starts with `++` (for example `++ <term>`) appears in the diff as `+++ <term>` and is dropped as if it were the file header, so a denylist term on that line passes the guard. Accidental rather than malicious, but the guard is the only defence against work data leaving the machine. The self-test has no case for it.
- **Fix**: Drop only real diff headers, e.g. `grep -v -E '^\+\+\+ (/dev/null|[ab]/)'`, and add a self-test assertion with an added line starting with `++`.
- **Decision**: FIXED — hook drops only real diff headers; self-test assertion added (13 assertions pass)

### F2 — Manual Progress rows still open while the change is closed out

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/ci-release-pipeline/plan.md:413-455
- **Detail**: Rows 1.5, 1.6, 2.4, 2.5, 3.5, 3.6, 4.4, 4.5 and 4.6 are `[ ]`, yet `change.md` is `implemented` and the epilogue commit closed the plan. Some are likely already proven (2.4: the first push went through the hook, which blocks without a denylist; 4.5/4.6: later pushes exist), but nothing records it. Nothing marked `[x]` lacks evidence.
- **Fix**: Run the manual checks (or confirm from run history) and tick the rows with a note, or record explicitly that they are deferred before `/10x-archive`.
- **Decision**: DEFERRED — needs hands-on checks (installer, SmartScreen, non-main push) that can't be verified from this session; tick before `/10x-archive`

### F3 — Release job uses floating action tags and an unchecked binary

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: .github/workflows/ci.yml:37,165
- **Detail**: `tauri-apps/tauri-action@v1` runs with `contents: write` and `taiki-e/install-action@v2` installs a tool, both on moving major tags. gitleaks is downloaded without checksum verification. The plan chose major tags, and the repo is private and solo, so the risk is low.
- **Fix**: Optionally pin `tauri-action` (the only write-scoped third-party action) to a commit SHA; otherwise accept.
- **Decision**: ACCEPTED — plan deliberately uses major tags; private solo repo

### F4 — Roadmap F-03 status set to in-progress (unplanned, harmless)

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Scope Discipline
- **Location**: context/foundation/roadmap.md:44,126
- **Detail**: The plan asked only for the outcome wording. The commit also changed F-03 status `ready` → `in-progress` and left the table row misaligned. It matches the roadmap convention and `/10x-archive` flips it to `done`.
- **Fix**: None needed; archive will set `done`.
- **Decision**: ACCEPTED — harmless; archive sets `done`
