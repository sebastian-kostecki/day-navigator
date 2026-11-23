# Review follow-ups: timer-reminders

From `reviews/impl-review.md` (2026-11-23).

## F3 — `hidden_issues_crash` flakes (pre-existing, outside this change)

- **Test**: `src-tauri/tests/hidden_issues_crash.rs:142`, `crash_never_loses_or_half_applies_a_hide_record`.
- **Symptom**: `round N (AfterDelay(..)): issue K after reported step K`. The writer reported step K, but after the kill the hide of issue K is not in the store.
- **Rate**: 4 of 5 runs on `21fccc3` and 2 of 5 on the base `84cf20f`, on the same Windows machine. Timer-reminders touches none of `store/`, `hidden_issues/`, `tests/common` or this test.
- **Why it matters**: CI (`cargo test`) can fail at random. The assertion also points to a possible durability gap in the hide write path: a step reported before its commit is durable, or a test expectation that is too strict.
- **Next step**: open a separate change, for example `/10x-new hidden-issues-crash-flake`, and diagnose it against the crash harness in `tests/common/crash.rs` and the hide write in `src-tauri/src/hidden_issues/`.
