<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Guarded Vault Access Implementation Plan

- **Plan**: context/changes/safe-vault-access/plan.md
- **Mode**: Deep
- **Date**: 2026-11-20
- **Verdict**: REVISE (before fixes) → SOUND (after fixes)
- **Findings**: 1 critical, 2 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | FAIL |
| Plan Completeness | WARNING |

## Grounding

6/6 paths ✓ (`.gitattributes` is new, as planned), 6/6 symbols and line refs ✓, brief↔plan ✓, Progress↔Phase ✓.

Checked by experiment on this machine:
- `cargo test` (unit and integration) runs on a scratch copy of this Tauri crate.
- `ReplaceFileW` keeps the target's attributes and creation time.
- `ReplaceFileW` fails with 32 while the replacement file's handle is open.

## Findings

### F1 — ReplaceFileW failure handling is incomplete

- **Severity**: ❌ CRITICAL
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 3 §1 replace_atomic; Critical Implementation Details
- **Detail**: The plan handles only error 1176. Three other cases break it:
  - Error 1177. The original is renamed to an unknown internal name and the target path is empty. Generic temp clean-up then deletes the only copy of the new content.
  - An open temp-file handle makes every ReplaceFileW call fail with 32, which the plan reports as `Busy`.
  - A read-only target (error 5) is retried and reported as `Busy`.
- **Fix**: Pass an explicit backup name. Close the temp handle before the call. Recover from 1176 and 1177 without deleting the temp or backup file. Retry only on 32/33. Return `ReadOnly` for a read-only target. Put the replace call behind an injectable function so tests can force these branches.
  - Strength: Every failure branch keeps a known copy of both the original and the new content.
  - Tradeoff: One more file name, and the backup is deleted after success.
  - Confidence: HIGH — the error semantics come from the Microsoft docs and were tested on this machine.
  - Blind spot: 1177 is reachable only through the test seam.
- **Decision**: FIXED (applied recommendation)

### F2 — `trash` contradicts the path guard's own rules

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 §2 trash
- **Detail**:
  - `VaultPath` and `resolve()` reject `.trash`, so the destination can't be named or guarded through them, and the return type can't hold it.
  - `.trash` is never containment-checked, so it could be a junction pointing outside the vault.
  - Obsidian numbers collisions from 2.
- **Fix**: Add an internal `trash_dest()` with containment checks that skips the hidden rule for that one segment. Return the file name as a `String`. Number collisions from ` 2`.
- **Decision**: FIXED

### F3 — Span rules are too loose for real Obsidian notes

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §1, §3
- **Detail**:
  - A line-start `#tag` could be treated as an H1 heading.
  - Indentation rules for headings are not specified.
  - Blank lines and comments inside a field block are not specified.
  - Property 2 cannot hold for fields that aren't in canonical form.
- **Fix**: Add the CommonMark ATX rule and a `#tag` example test. Define the field-block end rule. Limit property 2 to sections and fields in canonical form.
- **Decision**: FIXED

### F4 — Orphaned temp files end up in the vault's git history

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §1
- **Detail**: After a crash, `.dn-*.tmp` files holding the full note content remain in the folder, and obsidian-git commits them every 3 h.
- **Fix**: Before each write, delete that note's `.dn-*.tmp` and `.dn-*.bak` files older than 1 minute.
- **Decision**: FIXED

### F5 — Keep `tauri` out of the vault and settings modules

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §5
- **Detail**: Test binaries don't get Tauri's Common-Controls manifest (tauri#13419; fixed only in tauri-build 2.8). A test that reaches Tauri symbols crashes with `0xc0000139`.
- **Fix**: `vault/*` and `settings.rs` don't import `tauri`; the wiring stays in `lib.rs`.
- **Decision**: FIXED

### F6 — Scalar values are written verbatim, so they can break YAML

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2 replace_field
- **Detail**: A value that needs YAML quoting breaks Obsidian's Properties block.
- **Fix**: Return `InvalidValue` for scalars that would need quoting.
- **Decision**: FIXED
