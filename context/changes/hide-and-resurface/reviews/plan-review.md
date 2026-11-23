<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Hide and Resurface GitLab Issues

- **Plan**: context/changes/hide-and-resurface/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 2 critical, 3 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | FAIL |
| Plan Completeness | WARNING |

## Grounding
16/16 paths ✓, 9/9 symbols ✓, brief↔plan ✓, Progress↔Phase ✓

External behaviour was checked against primary sources:
- glab `main` @ `74ee3ce4` (v1.131.0): `internal/commands/api/api.go`, `pagination.go`.
- GitLab `master` @ `099035fe`: `app/models/note.rb`, `concerns/throttled_touch.rb`, `issue.rb`, `services/issuable/callbacks/labels.rb`, `lib/api/notes.rb`.

## Findings

### F1 — `glab api --paginate` prints pages as concatenated arrays

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §3 (notes parsing), Phase 1 §4 (fake CLI)
- **Detail**: glab never merges pages. With stdout piped it io.Copy's each page in turn, giving `[...][...]` (glab `api.go:509-592`; its tests pin `[{"id":1}][{"id":2}]`). The plan parsed notes with `serde_json::from_str::<Vec<_>>`, as `gitlab/mod.rs:174` does. Any issue with more than 100 notes (system notes count) would fail parsing, then count as "check failed" in every round and never resurface. The same latent bug already exists in the S-01 list parser.
- **Fix**: Add a shared `parse_pages` helper that reads a stream of arrays and flattens it, used by `parse_items` and the notes parser. Add a fake-cli scenario with two concatenated pages.
- **Decision**: FIXED

### F2 — GitLab skips the `updated_at` bump for a comment within 60 s of the last bump

- **Severity**: ❌ CRITICAL
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2 `needs_comment_check`; Critical Details "Baseline advance"
- **Detail**: `Issue` includes `ThrottledTouch` (`issue.rb:15`), and `touch` does nothing if `updated_at` is less than 1 minute old. Two cases miss a comment:
  - A reply within 60 s of the hide baseline.
  - A comment within 60 s of an advanced baseline.
  In both, `updated_at` doesn't move, so the `updated_at`-only gate never checks, possibly forever.
- **Fix A ⭐ Recommended**: Add one "settling" check after each new baseline (`settle_after_ms` = hide or advance time + 2 min, local clock; cleared by a settling-only check).
  - Strength: One extra notes call per hide/advance; closes both cases.
  - Tradeoff: One more column and rule.
  - Confidence: HIGH — the throttle window is fixed at 1 minute in source.
  - Blind spot: The server's TOUCH_INTERVAL may differ on older GitLab versions.
- **Fix B**: Drop the `updated_at` gate and check every hidden issue each round.
  - Strength: Simplest rule.
  - Tradeoff: N calls every 5 minutes.
  - Confidence: HIGH
  - Blind spot: The expected number of hidden issues is unknown.
- **Decision**: FIXED (Fix A)

### F3 — The lock can't span the round's commit and snapshot store as specified

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architectural Fitness
- **Location**: Phase 3 §1 (`reconcile_round`, `hidden_gate`)
- **Detail**: `reconcile_round` did the notes calls, the write and the partition, while the snapshot store happens later in `refresh_round` (`gitlab_refresh.rs:188-195`). The gate couldn't cover the write through the snapshot store without also covering the `glab` calls. That leaves open the race the plan claimed to prevent, a late round overwriting a command's newer partition. The plan also didn't say the partition must use rows re-read inside the write.
- **Fix**: Split into `check_round` (no lock) and `commit_round` (apply, then re-read and partition), with `apply_round` returning the post-write rows. `refresh_round` holds the gate across `commit_round`, `next_snapshot`, the kept-list update and the snapshot store.
- **Decision**: FIXED

### F4 — The open/closed state comparison can never fire

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Desired End State; Phase 1 §1; Phase 2 §1–2
- **Detail**: Both list calls use `state=opened` (`cli/mod.rs:128-134`), and GitLab issues are only ever `opened` or `closed` (`issuable.rb:321-323`). A stored state and a listed state are always "opened", so the `state` column, that half of the rule and its test cover an unreachable path.
- **Fix**: Drop `state` everywhere; "status" means the scoped-label set. Closing drops the issue off the list; a reopen within 30 days returns it as "wróciło na listę".
- **Decision**: FIXED

### F5 — Comment checks run one by one with no cap and delay the list

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Implementation Approach steps 2–3; Performance Considerations
- **Detail**: The runner timeout is 20 s per call (`cli/runner.rs:60`). A VPN drop after the list fetch costs N × 20 s while the view shows "refreshing", and the new list waits until all checks finish.
- **Fix**: `check_round` stops after the first Unavailable, LoggedOut or ToolMissing and counts the rest in `check_failed`. Add a round test for it.
- **Decision**: FIXED

### F6 — Files the change touches that the plan doesn't list

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §1, Phase 3 §1–2
- **Detail**: These need edits but weren't in the plan:
  - The `GitlabSnapshot` test helpers at `gitlab_refresh.rs:230` and `health/probe.rs:289`.
  - The item helpers at `gitlab/mod.rs:596-609`.
  - The `kept_config` doc comment ("Only the refresh thread touches it"), which becomes false once commands read it.
- **Fix**: Add these to the Phase 1 and Phase 3 file lists.
- **Decision**: FIXED

### F7 — Records for a previous host are never pruned

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §3 (`list(store, host)`, Prune)
- **Detail**: Absence and pruning were judged only for the current host, so rows from a previous host would stay forever.
- **Fix**: `apply_round` also deletes rows of other hosts whose `hidden_at_ms` is 30 days or more old. Add a store test for it.
- **Decision**: FIXED
