<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: GitLab — flaga NOWE i data ukrycia

- **Plan**: context/changes/gitlab-new-and-hidden-date/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

Automated checks: `npm run lint`, `format:check` and `build` passed. `rg` found no literal colours. `cargo fmt --check` and `clippy -D warnings` passed. `cargo test` ran 551 tests with 0 failures.

Manual rows 3.4–3.10: the user checked them by hand in the dev preview and they are ticked in Progress. The tick is not committed yet.

## Findings

### F1 — Błąd zapisu zbioru widzianych gasi flagi i przesuwa `shown_until`

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:580-585 (`accept_round`), :765 (`record_seen`)
- **Detail**: When `record_round` fails (e.g. `Busy`), `fresh` comes back empty. `accept_round` then replaces `kept_new` with an empty set and still sets `shown_until = round_ms`. Two effects:
  - (a) The flags already shown disappear for that round. If the user leaves the view at that point, those items count as seen even though they were not shown as NEW.
  - (b) If an account's first round fails this way and the user leaves, the mark gets saved (R1). Round R2 then seeds, and R3 marks the whole list NEW, because every `first_seen = R2` is later than R1.

  Both effects break the plan's promise that a failure always errs towards showing a flag again.
- **Fix**: When the seen-set write fails, `accept_round` changes nothing: `kept_new` and `shown_until` stay as they were, and the round shows the flags from `kept_new`. This could be done with an `Option<&[(ItemKey, i64)]>` argument, where `None` means "fresh unknown".
  - Strength: One condition covers both (a) and (b). With no `shown_until`, `mark_seen` saves no mark, so (b) cannot occur.
  - Tradeoff: During a store failure, items from that round get no NEW flag; they get it in the next successful round, because their `first_seen` is not written yet.
  - Confidence: HIGH. The path is short and traced by both reviewers.
  - Blind spot: The 3-round scenario has no integration test; it needs a unit test on `accept_round`.
- **Decision**: FIXED (Fix now). `accept_round` takes `Option<&[(ItemKey, i64)]>`; on `None` it keeps `kept_new` and `shown_until`. `record_seen` returns `fresh: None` when the seen-set write fails. Two new unit tests cover this: `a_failed_seen_set_write_keeps_the_flags_and_shown_until` and `a_failed_first_round_saves_no_mark`.

### F2 — Pozycje usunięte filtrem widoczności liczą się jako widziane

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/gitlab_refresh.rs:291, :604, :614
- **Detail**: `mark_fresh` runs before `filter_shown`, and `shown_until` covers the whole round. So when the user leaves, an item hidden by the visibility filter counts as seen. After widening the filter it appears without NOWE. The plan review accepted this (tradeoff in F1 of plan-review.md), but the plan has no line saying so.
- **Fix**: Add one line to "What We're NOT Doing": "NOWE does not survive the visibility filter: an item removed by the filter counts as seen when you leave the view."
- **Decision**: FIXED (Fix now). The line was added to `What We're NOT Doing` in plan.md.

### F3 — `NewAssignmentError`: nazwa wariantu i opis modułu odbiegają od sąsiadów

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/new_assignments/error.rs:1, :17
- **Detail**: The new variant is `BackgroundFailed` (`kind()` = "BackgroundFailed"), while `TodoError` and `WorkTimeError` use `Background`. The module doc still says "serialized for logs", but the error is now also a command result sent to the webview.
- **Fix**: Rename the variant to `Background` and update the doc to "serialized for logs and the UI".
- **Decision**: FIXED (Fix now). The variant is now `Background` (`kind()` = "Background") and the module doc says "serialized for logs and the UI".

### F4 — Brak testu „zapomniany klucz znowu świeży” i luźny helper `mark_of`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/tests/new_assignments.rs (`an_absent_item_is_forgotten_after_thirty_days_and_notifies_on_return`, `mark_of`)
- **Detail**: The plan's Testing Strategy lists the case "a key forgotten after 30 days is fresh again when it returns", but the test checks only `back.new`, not `fresh`. The code is correct (the row is deleted, so `first_seen` falls back to `now_ms`). Also, `mark_of` uses `.ok()`, which turns any SQL error into "no mark".
- **Fix**: Set a mark before the return and assert that the returned key is in `back.fresh`. Change `mark_of` to use `.optional()`.
- **Decision**: FIXED (Fix now). A new test, `a_forgotten_key_is_fresh_again_on_return`, covers the case, and `mark_of` now uses `.optional().unwrap()`, so an SQL error fails the test.
