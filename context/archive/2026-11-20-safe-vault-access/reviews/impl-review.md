<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Guarded Vault Access Implementation Plan

- **Plan**: context/changes/safe-vault-access/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-20
- **Verdict**: REJECTED
- **Findings**: 1 critical, 6 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | WARNING |
| Safety & Quality | FAIL |
| Architecture | WARNING |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

Automated criteria all passed on re-run (2026-11-20):
- `cargo build` passed.
- `cargo test` passed: 54 unit/property, 15 vault_guard and 29 vault_write tests, with 1 ignored (the smoke test).
- The filtered runs passed: `cargo test settings` (6), `cargo test proptests` (5) and `cargo test -- vault::markdown vault::edit` (47).
- `npm run build` passed.
- `cargo audit` exited 0. It reports only 6 transitive warnings in Tauri's Linux GTK stack (chrono, proc-macro-error, zbus_names, zvariant, zvariant_derive, glib) and none in the newly added crates.

Manual items 1.6, 1.7, 2.4 and 3.6–3.8 are unticked, so they are pending and none was ticked without evidence.

## Findings

### F1 — Lock retry replaces the note without re-checking it

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/fsops.rs:124-141
- **Detail**: The step-2 `precheck()` (re-read and compare with `expected_current`) runs once, at lines 102-119. On a sharing or lock violation (32/33), the loop sleeps 30–240 ms and calls `replace()` again without re-reading the target. A sharing violation usually means Obsidian is writing that note at that moment. Once the lock clears, the retry overwrites Obsidian's edit with bytes computed from the older content. This silently reverts a change anywhere in the note, which breaks the hard rule that the rest of the file stays byte-identical. It also stretches the plan's accepted "few milliseconds" race (plan.md:66, 100) to about 450 ms, in exactly the case where a concurrent write is most likely.
- **Fix**: Before each retry, re-run `precheck()`. On a mismatch, delete the temp file and return `ReplaceError::Changed`, so `Vault::apply` recomputes from fresh bytes. Add a test where the injected replace returns 32 once and modifies the target before the retry.
- **Decision**: FIXED — fsops.rs re-runs the re-read check before every retry; a changed note returns `Changed`, a still-locked read waits again. Test `note_changed_while_locked_is_changed_and_not_overwritten`.

### F2 — The 1176 recovery assumes a file state that ReplaceFileW does not produce

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/fsops.rs:142-151 (plan.md:92; test tests/vault_write.rs:417-429)
- **Detail**: The code follows the plan: for `ERROR_UNABLE_TO_MOVE_REPLACEMENT` (1176) it assumes the original sits under the backup name. Microsoft's ReplaceFileW documentation says that when a backup name is given, 1176 leaves *both files under their original names*. So the original is still at the target and no backup exists. The branch as written then has three problems:
  - (a) It moves the temp file onto the target with `MOVEFILE_REPLACE_EXISTING` without re-checking the target.
  - (b) It loses the creation time, ACL and alternate data streams that ReplaceFileW was meant to keep.
  - (c) If that move fails, `ReplaceIncomplete` names a backup that does not exist.

  The test fakes the plan's assumed state (`fs::rename(t, backup)`), so the documented state is never exercised. This is a flaw in the plan that the code carried over. The doc text was quoted from memory by the reviewer, so confirm it against the docs page before fixing.
- **Fix**: In the 1176 branch, branch on whether the backup file exists.
  - If it does not (the documented state): the target is intact, so treat it like a retryable failure. Run the precheck again, delete the temp, and return `Io`, or retry.
  - If it does (the plan's state): keep the current recovery.

  Also fix the plan text at plan.md:92 and add a test for the documented state.
  - Strength: Handles both states, so it is correct whichever reading of the docs is right. It never overwrites the target without a check.
  - Tradeoff: One more filesystem probe on a rare error path, and one more test.
  - Confidence: MED — the documented semantics come from memory and should be confirmed on learn.microsoft.com.
  - Blind spot: How ReplaceFileW actually behaves under this error can't be reproduced without fault injection at the OS level.
- **Decision**: FIXED — docs confirmed (learn.microsoft.com ReplaceFileW: 1176 with a backup name means "the replaced and replacement files retain their original file names"). With no backup file, 1176 is retried after a re-check, then `Io` (note unchanged); the moved-aside state is kept as a defensive branch. plan.md:92 corrected. Tests: documented-state retry, persistent 1176, defensive branch.

### F3 — Unguarded write helpers are part of the crate's public API

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture
- **Location**: src-tauri/src/vault/mod.rs:4 (`pub mod fsops`), mod.rs:303-308 (`#[doc(hidden)] pub fn apply`), fsops.rs:69, 79, 231, 254
- **Detail**: The plan does not ask for either of these:
  - `replace_atomic`, `replace_atomic_with`, `create_new` and `move_noclobber` are public and take raw absolute paths with no vault guard.
  - `Vault::apply` is public and accepts any rewrite of the whole file.

  The plan's design depends on "exactly one place where bytes reach a vault", reached only through the guarded, span-scoped operations. A future `#[tauri::command]` in lib.rs could call `fsops::replace_atomic(user_path, …)` or `apply(...)` and skip both the folder guard and the span guarantee. They are public only because the integration tests use them (vault_write.rs:10, 206, 231).
- **Fix A ⭐ Recommended**: Make `fsops` and `apply` `pub(crate)`, and re-export test hooks only under a `test-support` cargo feature that the integration tests enable through a self dev-dependency.
  - Strength: The integration tests stay where they are. Production code can't reach the unguarded functions.
  - Tradeoff: Adds a feature flag and a self-referencing dev-dependency to Cargo.toml.
  - Confidence: HIGH — a common Rust pattern for test-only API.
  - Blind spot: Some editor setups need the feature enabled before they show the test code.
- **Fix B**: Make them `pub(crate)` and move the fsops failure-branch tests (1176/1177, Busy, read-only, leftovers) into `#[cfg(test)]` unit tests inside `fsops.rs`.
  - Strength: The API surface is smallest and needs no feature flags.
  - Tradeoff: About 10 tests have to move, and they can no longer use `tests/common`.
  - Confidence: MED — the `apply`-based conflict tests (vault_write.rs:206, 231) would need another seam as well.
  - Blind spot: I haven't checked how many vault_write tests depend on `apply`.
- **Decision**: FIXED via Fix A — `fsops` is private and `Vault::apply` is private; the `test-support` feature (self dev-dependency) exposes `fsops` and `apply_for_tests` to `cargo test` only. Verified: `cargo build` compiles without the feature, `cargo test` with it.

### F4 — Field edits can produce invalid or different YAML

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/markdown.rs:190-217 (test at :782-787); src-tauri/src/vault/edit.rs:33-59
- **Detail**: Two separate problems.
  - **A list split by a blank or comment line is only partly replaced.** Following the plan, a field block stops at a blank line or a `#` comment. So for `A:\n  - 1\n\n  - 2\nB: x` the span covers only `A:\n  - 1\n`, and the test at markdown.rs:782 locks this in (`"  - lost"`).
    - Replacing with `List(["z"])` gives YAML `[z, 2]`, so the old item survives.
    - Replacing with `Scalar("z")` gives `A: z\n\n  - 2`, which is invalid YAML and breaks every property in the note in Obsidian.
  - **`check_plain` accepts some values that change the YAML.** It accepts a bare `-` and a bare `?`, which are invalid or nested-list YAML. It also accepts a leading `,`, which is a flow indicator.
- **Fix**: In `find_field`, if an indented or `- ` line follows a blank or comment line before the next top-level key or the end of the frontmatter, return a typed refusal. Use an existing variant with a clear reason or a new `UnsupportedLayout`. Also extend `check_plain` to reject a bare `-` or `?` and a leading `,`. Update plan.md:306 to match.
  - Strength: Fails closed. The note is never left with broken Properties, and normal Obsidian frontmatter is unaffected.
  - Tradeoff: Notes with this unusual layout can't be edited until the user tidies them.
  - Confidence: HIGH — the behaviour is pinned by an existing test, and the proposed rule is local.
  - Blind spot: Whether YAML keywords such as `No` or `true`, or numbers, should be quoted or refused is still open. They are written as-is today, so their type changes.
- **Decision**: FIXED — `find_field` returns new `FieldLayoutUnsupported { key }` when list lines follow blank/comment lines; `check_plain` also refuses bare `-`/`?` and a leading `,`. plan.md:309/330 updated. YAML keyword/number quoting left open (not in the fix).

### F5 — Paths over 260 characters are treated as outside the vault and silently left out of lists

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/mod.rs:186-188, 261-267, 485-506
- **Detail**: `dunce::canonicalize` keeps the `\\?\` prefix when a path is longer than 260 characters (dunce 1.0.5 `is_safe_to_strip_unc`). The root canonicalises to `D:\…`, but a deep note canonicalises to `\\?\D:\…`. `relative_remainder` compares component strings (`\\?\d:` against `d:`), finds no match, and the guard returns `OutsideVault`.
  - `read_note` then gives a misleading error.
  - `list()` treats `OutsideVault` as "skip", so the note disappears without any error. That breaks the rule against showing an empty list in place of a failed source.
  - Deep folders with long Polish titles can realistically pass 260 characters.
- **Fix**: In `component_key`, normalise `Prefix::VerbatimDisk(d)` to `Disk(d)` and `VerbatimUNC` to `UNC`. Add a test with a path longer than 260 characters.
- **Decision**: FIXED — `component_key` treats VerbatimDisk/VerbatimUNC as Disk/UNC. Test `notes_beyond_260_characters_are_readable_and_listed` (fails on the old code).

### F6 — New section text can change the rest of the note's structure

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/edit.rs:136-158 (`replace_section`), 160-189 (`append_to_section`)
- **Detail**: `new_body` and `lines` are only EOL-normalised. Bytes outside the span stay identical, but their meaning can change:
  - A body that contains `## Log` adds a section, so later edits to `## Log` return `SectionAmbiguous`.
  - A single unclosed ```` ``` ```` turns every later heading into code, both in Obsidian and in `headings()`, so later sections can no longer be found.

  The plan did not ask for this check, so this is a gap in the plan.
- **Fix**: Reject inserted text (`InvalidValue`) if it has an ATX heading, outside fences, at the target's level or higher, or if it leaves a fence open. Deeper subheadings stay allowed.
  - Strength: Every edit stays scoped to its own section, both in meaning and in bytes. The heading and fence scanner already exists in markdown.rs.
  - Tradeoff: Some legitimate inputs are refused, for example pasted text that contains a same-level heading. The calling slice has to show that error.
  - Confidence: HIGH — the existing scanner can be reused on the inserted text.
  - Blind spot: No slice exists yet, so it is unknown whether any of them will need to insert same-level headings.
- **Decision**: FIXED — new `markdown::check_section_text` refuses same-or-higher headings outside fences and unclosed fences in `replace_section`/`append_to_section`; deeper subheadings allowed. Unit test added; plan updated.

### F7 — The smoke test's safety marker is inside the fixture itself

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/tests/vault_write.rs:611-618; src-tauri/tests/fixtures/vault/.dn-test-vault
- **Detail**: The guard only checks `root.join(".dn-test-vault").is_file()`. With `DN_SMOKE_VAULT=src-tauri/tests/fixtures/vault` it passes, and the test would then edit fixture notes, create `Dziennik/2027`, and move `Notes/duplicate-headings.md` to the trash. That breaks "tests never touch the fixture itself" and the byte-exact fixture.
- **Fix**: Also refuse to run when the canonicalised `DN_SMOKE_VAULT` is the canonicalised `fixture_dir()` or lies inside it.
- **Decision**: FIXED — smoke test also refuses when DN_SMOKE_VAULT is the fixture or inside it. Verified: running it against the fixture refuses and leaves the fixture untouched.

### F8 — Excluded entries can fail open

- **Severity**: 💡 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/path.rs (`starts_with_ci`), src-tauri/src/vault/mod.rs:169-171, 194, src-tauri/src/settings.rs
- **Detail**: Excluded entries are matched by lowercase string comparison with no Unicode normalisation. A folder whose name is stored on disk in NFD form (for example `Dostępy` synced from macOS or through git) doesn't match an NFC entry typed into settings.json. That folder can then be read and listed. Separately, an excluded entry that matches nothing, such as a typo, is accepted silently and protects nothing.
- **Fix**: NFC-normalise both sides before lowercasing (the `unicode-normalization` crate). When `vault()` builds a Vault, report excluded entries that don't resolve to an existing path.
  - Strength: Closes a silent way for exclusion to fail open, which the Access Control rule says must never happen.
  - Tradeoff: Adds a dependency. Warning about missing entries needs a non-fatal way to report it, which belongs to F-02.
  - Confidence: MED — NFD names on NTFS are an edge case for a vault created on Windows.
  - Blind spot: I haven't checked whether the user's vaults ever pass through macOS or a sync tool that writes NFD names.
- **Decision**: FIXED — exclusion matching NFC-normalises segments (`unicode-normalization` 0.1.26, no advisories); new `Vault::unmatched_exclusions()` for F-02 to surface. Tests for NFD folders and unmatched entries (NFD test fails without the fold).

### F9 — Leftover clean-up can hard-delete the only copy of an original note

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/vault/fsops.rs:93-96, 209-228
- **Detail**: When the 1177 recovery fails, the original survives only as `.x.md.dn-*.bak` and the error is `ReplaceIncomplete`. If the note is later recreated at the same path, the next edit runs `remove_leftovers`, which deletes that `.bak` once it is more than 60 seconds old. That is a hard delete, which the never-hard-delete rule forbids. It takes two failures in a row, so it is unlikely.
- **Fix**: Have `remove_leftovers` delete only `.tmp` leftovers. Move `.bak` leftovers to `.trash`, or leave them in place, and adjust the plan step 0 and its test.
- **Decision**: FIXED — `remove_leftovers` deletes only `.tmp`; `.bak` files are never deleted. Test and plan step 0 updated.

### F10 — Small drift and hygiene issues

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: several (listed below)
- **Detail**: Each item is small:
  - (a) settings.rs:61-64 puts the absolute settings path into `Io.path`, which breaks the plan's invariant that paths are vault-relative.
  - (b) `FieldView.value_text` holds the whole raw block (`key:` line plus list lines), not the value (mod.rs:133). It is documented in the code, but the name suggests the parsed value.
  - (c) About 8 vault_write tests don't assert that no `.dn-*` leftovers remain, which plan.md:454 asks of "any test". Examples are the CRLF/BOM replace, field replace, trash and creation-time tests.
  - (d) roadmap.md still shows F-01 as `in-progress` while change.md says `implemented`.
  - (e) The code is not rustfmt-clean (mod.rs:292-293, 432; fsops.rs:190-191).
  - (f) The I/O error mapping and the `BOM` constant are each duplicated in 3–4 places.
  - (g) Between `resolve()` and the write or move in `create_note` and `trash`, a folder could be swapped for a junction. This race is not written down as an accepted risk next to the Obsidian race.
- **Fix**: In one hygiene pass:
  - Make `Io.path` the file name only.
  - Rename `value_text` to `raw_block`.
  - Add `assert_no_leftovers` to the remaining tests.
  - Sync the roadmap status.
  - Run `cargo fmt`.
  - Add one sentence to Critical Implementation Details on the junction-swap race.
- **Decision**: FIXED (partly) — (a) settings `Io.path` is now the file name only; (b) `value_text` renamed to `raw_block` (code and plan); (c) `assert_no_leftovers` added to 8 more tests; (e) `cargo fmt` applied; (g) junction-swap race written into Critical Implementation Details. Not changed: (d) the roadmap keeps `in-progress`, which is accurate until the manual checks pass and the change is archived; (f) the duplicated helpers were not in the fix.
