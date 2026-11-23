<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Timer Reminders Implementation Plan

- **Plan**: context/changes/timer-reminders/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 4 observations

Phase 2 code is fully landed and its automated rows are `[x]`. The manual rows 2.5–2.8 are still `[ ]` and are treated as pending, not as findings.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

### Evidence summary

- Plan files and diff (`84cf20f..HEAD`) line up exactly. All 10 planned code files changed, and only docs changed outside them. There are no TypeScript changes.
- Plan-drift check: every planned contract matches, including the settings shape and ranges, `reminder_facts` in one read, the slot-start keys, the DST anchor, the Polish text byte for byte, the 60 s thread, the toast area prefix, and the source key, label and message. Harmless extras: `APP_NAME` is now `pub(crate)`, there is a new `DEFAULT_LONG_RUNNING_HOURS` constant, and there are additional tests.
- Commands run on 2026-11-23:
  - `cargo fmt --check` passed.
  - `cargo clippy --all-targets -D warnings` passed.
  - `cargo test --lib reminders`: 28 passed.
  - `cargo test --lib settings`: 34 passed.
  - `cargo test --lib health`: 34 passed.
  - `cargo test --test work_time`: 6 passed.
  - `npm run lint`, `npm run format:check` and `npm run build` passed.
  - Full `cargo test`: first run green. Second run had one failure, `hidden_issues_crash` (see F3).

## Findings

### F1 — Reminder thread logs the full error and hides non-store errors

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_reminders.rs:59-62
- **Detail**: The thread logs `log::debug!("reminders: facts unavailable: {e}")`, which prints the `Display` text of `WorkTimeError`.
  - That text includes `StoreError` reason strings: `Sql { reason }`, `Damaged { reason }` and `MigrationFailed { reason }`.
  - This breaks the repo rule "errors are logged by kind only" (`work_time/commands.rs:6-7`, and `store` logs `e.kind()`).
  - Every error is logged at `debug`. Release builds filter the crate at Info (`lib.rs:161`), so the line never appears.
  - `reconcile_todo` (`gitlab_refresh.rs:368-375`) logs store errors at debug and every other error at warn. Here a non-store error such as `InvalidLocalTime` would turn reminders off every round with no trace in the log.
- **Fix**: Turn the private `kind()` in `work_time/commands.rs:19` into `WorkTimeError::kind()`. Then match like `reconcile_todo`: `Err(e @ WorkTimeError::Store(_))` logs `e.kind()` at debug, and any other error logs `e.kind()` at warn.
- **Decision**: FIXED — `WorkTimeError::kind()` moved to `work_time/error.rs`; the reminder thread logs the kind, store errors at debug and others at warn

### F2 — "A panic is only logged" is false; in release a panic exits the whole app

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/work_reminders.rs:5, :32-38 (plan.md:171)
- **Detail**: The plan (line 171) and the module doc both say a panic in the loop is logged and does not stop the app. Neither is true:
  - The code has no `catch_unwind` and no panic hook.
  - The release profile sets `panic = "abort"` (`Cargo.toml:85`), so a panic on this thread ends the whole app and nothing is written to the log file.
  - In dev, the panic goes only to stderr and the thread dies silently.
  - `catch_unwind` would not help in release, because `panic = "abort"` does not unwind.
  - I found no panic that can actually happen in the round: the threshold is capped with `try_from(..).unwrap_or(i64::MAX)`, and chrono is used through `.earliest()` and `_opt`. So the real risk is low and the wrong part is the claim.
- **Fix A ⭐ Recommended**: Correct the module doc and the plan text to say "a panic aborts the app (release `panic = "abort"`); the round has no known panic path".
  - Strength: Matches what actually happens; no code change for a panic nobody can reach.
  - Tradeoff: Leaves a hypothetical future panic unlogged.
  - Confidence: HIGH — the subagent audit and a manual read of `reminders.rs` found no unwrap, expect or unchecked arithmetic.
  - Blind spot: Panics inside chrono or rusqlite internals were not audited.
- **Fix B**: Install an app-wide `std::panic::set_hook` in `lib.rs` that sends the panic (thread name and location, no payload values) to `log::error!` before the abort.
  - Strength: Every thread's panic, including health and GitLab, then reaches the log file. This also makes the plan's claim partly true ("logged", although the app still exits).
  - Tradeoff: An app-wide change beyond this slice's scope; the hook must not echo work data from panic messages.
  - Confidence: MED — standard pattern, but there is no precedent for it in this repo.
  - Blind spot: How `tauri-plugin-log` behaves when it is called from a panic hook just before abort.
- **Decision**: FIXED via Fix A — module doc and plan.md:171 corrected

### F3 — Full `cargo test` is not reliably green (pre-existing `hidden_issues_crash` flake)

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Success Criteria
- **Location**: src-tauri/tests/hidden_issues_crash.rs:142
- **Detail**: The criterion "full suite passes" (Progress 2.2) is not reliably met.
  - `crash_never_loses_or_half_applies_a_hide_record` failed 4 times in 5 on HEAD, for example `round 3 (AfterDelay(500ms)): issue 129 after reported step 129`.
  - On the base commit `84cf20f` it failed 2 times in 5 with the same assertion.
  - This change touches nothing in `store/`, `hidden_issues/`, `tests/common` or that test, so the flake predates it. It is not caused by timer-reminders.
  - It does mean CI can fail at random, and the test signals a possible durability gap: a step was reported done but its hide was not present after the kill.
- **Fix**: Do not block this change on it. Open a separate change (for example `hidden-issues-crash-flake`) to find out whether the writer reports a step before its commit is durable or whether the test's expectation is wrong.
- **Decision**: DEFERRED — pre-existing; queued in follow-ups/review-fixes.md for a separate change

### F4 — Stale docs: restart behaviour and store-open comment

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/work_time/reminders.rs:4-7; src-tauri/src/lib.rs:178-180
- **Detail**: Two comments no longer match the code:
  - The `reminders.rs` module doc says a restart "never produces a backlog or a repeat". The plan (line 42) and the test `an_empty_memory_fires_the_current_slot_once` both say a restart may send the current slot once more.
  - The `lib.rs` comment says the first health round opens the store. Now the reminder thread's first round can win that race (harmless: still off the main thread, as the plan notes at line 173).
- **Fix**: Reword the module doc to "a restart may re-send the current slot once; never a backlog", and change the `lib.rs` comment to "the first health or reminder round opens it off the main thread".
- **Decision**: FIXED — `reminders.rs` module doc and `lib.rs` setup comment reworded

### F5 — Test gaps: new-entry id check not exercised, no ambiguous-anchor case

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/work_time/reminders_tests.rs:65, :115
- **Detail**: Three gaps in the rule tests:
  - The "new entry starts a fresh slot sequence" test is weak. Entry 2's slot (06:00) is later than entry 1's last sent slot (18:00 of the day before), so the test would still pass if the entry-id comparison were removed.
  - The plan's ambiguous-anchor rule (fall-back → earlier instant) has no test.
  - The worked example samples 19:00, 23:00, 23:40 and 00:00 instead of every hour.
- **Fix**: Add a case where the second entry's first slot start is not later than the previous entry's remembered slot. Add a 2026-10-25 `notRunningAfter: "02:30"` case that asserts the earlier instant.
- **Decision**: FIXED — added `a_different_entry_counts_as_new_even_with_an_older_slot` and `dst_fall_back_anchors_at_the_earlier_of_two_instants`

### F6 — A clock that briefly runs ahead silences reminders until real time catches up

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/reminders.rs:114-117, :147-150
- **Detail**: This is the mirror side of "a clock moved backwards never repeats".
  - If the clock jumps forward (for example a wrong time after resume that NTP later corrects), the memory keeps a slot start that lies in the future.
  - Until real time passes that slot, no long-run reminder fires for that entry, and the idle reminder stays silent for the rest of that day.
  - This follows from the plan's own design and is rare on Windows with time sync.
- **Fix**: Accept it and add one line to the plan's "What We're NOT Doing" (or to the module doc) so the trade-off is explicit.
- **Decision**: FIXED (accepted and documented) — module doc and plan What We are NOT Doing

### F7 — Facts tests placed in the durability test file

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/tests/work_time.rs:136-215
- **Detail**: The 3 `reminder_facts` tests were added to the S-08 file whose header reads "survive closing and reopening the real store", and none of them reopen the store. They also put `use` statements inside the test functions, while the rest of the file imports at the top. The plan asked for this file (line 224), so this is a placement nit, not drift.
- **Fix**: Move the imports to the top and widen the file header to cover the `reminder_facts` queries.
- **Decision**: FIXED — imports moved to the top, file header widened
