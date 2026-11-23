<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Assigned GitHub Issues ("Projects (GitHub)")

- **Plan**: context/changes/github-assigned-issues/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 4 warnings, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding
20/20 paths ✓, 8/8 symbols ✓, brief↔plan ✓, Progress↔Phase ✓

Verified against code:
- The v7 rebuild is feasible: one IMMEDIATE transaction per migration, no views/triggers, nothing references `todo_items`, no `legacy_alter_table`, and indexes are recreated after the drop.
- Timeout → `Unavailable` and stdout overflow (16 MiB) → `Failed`.
- `set_ticked` is generic.
- The `lib.rs` startup order matches the plan.

## Findings

### F1 — Fake CLI can't show that later repos were skipped

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 — §5 Fake CLI gh data, §6 Tests
- **Detail**:
  - `FAKE_CLI_SCENARIO` applies to the whole process (`fake_cli.rs:295`), so `gh-logged-out` fails the first call too, and the alpha-OK-then-failure case never happens.
  - The only call log (`FAKE_CLI_NOTES_LOG`, `fake_cli.rs:210-223`) records GitLab notes only, so "not called" can't be checked.
- **Fix**: Add `octo-test/offline` / `octo-test/logged-out` routes to `gh-issues`, plus a generic `FAKE_CLI_CALL_LOG`. Test `[alpha, offline, beta]` and assert beta is never called.
- **Decision**: FIXED

### F2 — TodoView keys and branches unlisted for "github"

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 — §4 UI
- **Detail**:
  - `onList` keys every non-own row as `${itemKind}:${reference}` with no source (`TodoView.tsx:28-30,112-114`). This either fails to compile or collides with GitLab `owner/repo#N`.
  - The link (`:132`), the kind badge (`:157-163`) and the closed badge (`:165-166`) are GitLab-only.
  - There is no `never` check, so a missed branch fails silently.
- **Fix**: List the four places in Phase 4 §4. Use a source-prefixed picker key and GitHub badge texts.
- **Decision**: FIXED

### F3 — Health status for repo errors: kind and one wake missing

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 — §3 Health source
- **Detail**:
  - The aggregated "repo error" status had no defined `kind`/`message`. `short_reason` maps by kind, and a per-repo kind could change between rounds.
  - The watchdog resume wake (`runtime.rs:347`) was missing; only `:112` and `:162` were listed.
- **Fix**: Fixed kind `RepoFailed` ("błąd repozytorium"), a message without repo names, a short reason for `Truncated`, and all three wake points.
- **Decision**: FIXED

### F4 — "Unverified" banner ignores per-repo failures

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 4 — §4 UI (banner)
- **Detail**: Entries from failed or removed repos are never ticked. With the source `ok`, they would still show as ordinary open items, with no hint that nothing checks them.
- **Fix**: Show the banner also when an unticked GitHub entry's repo has an error in the snapshot or is missing from `snapshot.repos`.
- **Decision**: FIXED

### F5 — Existing tests that v7 will break are not listed

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 — §5 Tests
- **Detail**: These tests break under v7:
  - `tests/todo.rs:71-78` (exact index list)
  - `tests/hidden_issues.rs:112` (`user_version == 6`)
  - `tests/todo_crash.rs:168-177` (gitlab duplicate check)

  `tests/hidden_issues.rs:96-123` is a ready pattern for the v6→v7 test.
- **Fix**: Add these three files to §5 and point the migration test at that pattern.
- **Decision**: FIXED
