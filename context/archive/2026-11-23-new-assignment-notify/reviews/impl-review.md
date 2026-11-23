<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: New Assignment Notifications Implementation Plan

- **Plan**: context/changes/new-assignment-notify/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-23
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 7 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

Only one finding (F1) is at WARNING severity. The other dimension warnings come from observations alone. Nothing in "What We're NOT Doing" was violated. The one unplanned code edit, `tests/hidden_issues.rs:112`, replaces a hard-coded `user_version == 6` with `MIGRATIONS.len()`. Migration v7 required it.

## Automated checks (run 2026-11-23)

| Command | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` (full) | pass: 321 lib tests plus every integration binary |
| `cargo test new_assignments` | pass: 23 unit + 1 integration (see F4) |
| `cargo test gitlab_refresh` | pass: 10 |
| `cargo test settings` | pass: 33 + 3 + 1 |
| `cargo test gitlab::` | pass: 35 |
| `cargo test --test new_assignments --test new_assignments_crash` | pass: 6 + 1 |
| `npm run lint`, `npm run format:check`, `npm run build` | pass |

The manual rows 1.4, 2.4–2.8, 3.5 and 3.6 are ticked. Commit 83bf4fe records the user's confirmation. They are runtime checks, so the diff holds no evidence for them, which is expected.

## Findings

### F1 — Toast items come from item iteration, not from the deduplicated new keys

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/gitlab_refresh.rs:417-423
- **Detail**:
  - `notify_new_assignments` builds the toast list by filtering `merge_requests → issues → hidden_issues` against `new_keys` (`Vec::contains`).
  - `plan_round` deduplicates keys, but this mapping does not. `fetch_assigned` and the parsers do not deduplicate either.
  - A `--paginate` list ordered by `updated_at` can return an item twice when it changes mid-fetch. That item then gives two toasts, or two lines and an inflated N.
  - The order is also always MRs first, then issues. The plan says "the list's order (newest `updated_at` first, as fetched)", so this is a small drift when both kinds are new in one round.
- **Fix**: Iterate `new_keys` (already deduplicated) and look up the first item per key, then sort the picked items by `updated_at` descending.
- **Decision**: FIXED — `new_assignments::items_for` picks one item per new key and sorts newest `updated_at` first (parsed RFC 3339); `notify_new_assignments` uses it. Tests: `a_repeated_item_gives_one_toast_item`, `items_are_newest_first_across_kinds`, `a_key_without_an_item_is_skipped`.

### F2 — Seen-set write failures are silent and one bad id rolls back the whole round

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:411, src-tauri/src/new_assignments/mod.rs:112,125
- **Detail**:
  - Every `record_round` error is logged at `debug` only.
  - The health round checks only that the store opens, so a persistent write failure gives no visible signal. Examples: a CHECK violation, or the new `Damaged` "unreadable row" (`mod.rs:81-85`).
  - In that state the feature runs on session memory for the whole run, and items notify again after each restart.
  - `key.id as i64` wraps for ids above `i64::MAX`, and an id of 0 passes parsing. Either value fails `CHECK (item_id > 0)` and rolls back the whole round, not just that item.
  - GitLab does not return such ids, and `hidden_issues` uses the same cast. The debug level mirrors `reconcile_todo`.
- **Fix**: Log at `warn` once per session for error kinds other than open/busy, and use `i64::try_from(key.id)` to skip an out-of-range key instead of failing the round.
- **Decision**: FIXED — first non-busy seen-set failure of a session logs at `warn` (kind only), later ones at `debug` (`SEEN_SET_WARNED`); `keys_of` skips ids outside `1..=i64::MAX`. Correction: id 0 never reached here (`gitlab/mod.rs` parse rejects it); only > i64::MAX was possible. Test: `ids_that_cannot_be_stored_are_not_keyed`.

### F3 — Combined toast lines cap the title but not the reference

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/new_assignments/notice.rs:28
- **Detail**:
  - `line()` cleans the reference with `usize::MAX`. A long `group/subgroup/project!123` reference plus a 40-char title can wrap.
  - Windows shows only a few body lines, so wrapping can push `i N−3 więcej` out of view. The 40-char title cap was meant to prevent exactly that (plan-review F4).
  - No manual check exercised a combined toast (4+ new items in one round).
- **Fix**: Cap the reference in combined lines (e.g. 60 chars with `…`) and add a unit case, or check one combined toast by hand.
- **Decision**: FIXED — combined-toast lines cut the reference at 60 chars with `…` (`COMBINED_REFERENCE_CHARS`); separate toasts keep it whole. Test: `combined_line_reference_is_cut_at_60_chars`.

### F4 — `cargo test new_assignments` does not run the store or crash tests

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/new-assignment-notify/plan.md:148 (Progress 1.1)
- **Detail**:
  - Criterion 1.1 says "Rule, store and crash tests pass: `cargo test new_assignments`". The name filter matches only 1 of the 6 tests in `tests/new_assignments.rs` and none in `tests/new_assignments_crash.rs`.
  - The full `cargo test` (1.2) does run them, and they pass, so nothing is unverified. The criterion's command just proves less than its label says.
- **Fix**: Next time, write the targeted command as `cargo test new_assignments` plus `cargo test --test new_assignments --test new_assignments_crash`. This could also be recorded as a lesson for plan criteria.
- **Decision**: ACCEPTED — the plan is closed and its Progress titles are not renamed; criterion 1.2 (full `cargo test`) runs these tests. Candidate lesson for future plan criteria (`/10x-lesson`).

### F5 — Crash test exercises only append-one-id rounds

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/tests/new_assignments_crash.rs:64-132
- **Detail**:
  - Each writer round adds exactly one new id at the tail, and the checks are "marker ⇔ rows" and a gap-free id run.
  - These checks would not catch a torn `last_seen_ms` upsert or a torn forget delete. The writer never advances the clock past 30 days.
  - Atomicity comes from `Store::write` (one IMMEDIATE transaction), so the real risk is low. The test still proves less than the plan's "whole of a round or none of it".
- **Fix**: Let the writer drop and add several ids per round, step `now_ms` past `FORGET_AFTER_MS` sometimes, and assert that the reopened rows equal some reported round's full set.
- **Decision**: FIXED — the writer now varies the set per round (sliding issues + an MR sharing an id on even rounds) with rounds 11 days apart, so inserts, refreshes and forgets of both kinds all happen; the parent asserts the reopened rows equal a model of a whole round exactly (first/last instants included). Deliberate break (forget step disabled) fails the test.

### F6 — v7 migration comment still says "renumber at merge"

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/store/schema.rs:129-130
- **Detail**: The v7 comment reads "(Planned as v7; renumber at merge if another slice lands first.)", but v7 is now on `main`. The v5 and v6 comments record what happened. Slices still in flight renumber their own migration, not this one.
- **Fix**: Change the comment to "(Shipped as v7.)". This edits the comment only; the SQL stays byte-identical.
- **Decision**: FIXED — comment now reads "(Shipped as v7.)"; migration SQL unchanged.

### F7 — View notice hard-codes the Rust range and default

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/GitlabView.tsx:249
- **Detail**: The notice text "dozwolone 1–14" / "co 10 min" duplicates `REFRESH_MINUTES_MAX` and `DEFAULT_REFRESH_MINUTES` (`settings.rs:189,192`). This text is what the plan asked for, but the two copies can drift if the constants change.
- **Fix**: Add a one-line comment above the notice pointing at the `settings.rs` constants.
- **Decision**: FIXED — JSX comment above the notice points at `REFRESH_MINUTES_MAX` / `DEFAULT_REFRESH_MINUTES` in `settings.rs`.

### F8 — Delivery edge cases accepted by design are not written down

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/new_assignments/mod.rs:1, :166-171
- **Detail**: All three edge cases are consistent with the plan's "no replay" stance and are not defects:
  - The seen set is committed before the toast shows. A toast OS error, or a quit between the two, loses that notification: delivery is at most once.
  - Suppose the fallback toasts X, then X leaves the list before the store recovers. `toasted_unstored` is cleared on recovery, so X's later return toasts again.
  - Rows and the marker of an abandoned account key (after a host or user change) are never pruned. The volume is tiny.
- **Fix**: Add these three points to the `new_assignments` module header so a later slice doesn't "fix" them by accident.
- **Decision**: FIXED — the three accepted edge cases are listed in the `new_assignments` module header.
