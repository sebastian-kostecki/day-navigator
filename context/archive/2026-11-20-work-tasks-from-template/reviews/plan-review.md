<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Work Tasks from Template Implementation Plan

- **Plan**: context/changes/work-tasks-from-template/plan.md
- **Mode**: Deep
- **Date**: 2026-11-20
- **Verdict**: SOUND
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
10/10 paths ✓, 8/8 symbols ✓ (`RawSpace` deny_unknown_fields, `Vault::apply`, `replace_field`, `check_plain`, `create_note`, `SpanHash`, `VaultError` not yet `Clone` as the plan says, root-listing assertion), brief↔plan ✓, Progress↔Phases ✓. Cited line numbers drift by ±2 (harmless). `chrono` 0.4.45 is already in `Cargo.lock` transitively; `build.rs` has no app-manifest command allowlist, so new commands need no capability change.

## Findings

### F1 — done_hash vs. the fresh note's Done field is unresolved

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §1 — set_status
- **Detail**: `set_status` decides on the `Done` edit from the fresh note, but the hash comes from the client. Two cases were undefined. In the first, `done_hash: None` while the fresh note has `Done`, so the edit has no hash to check. In the second, `done_hash: Some` while the field is gone, so `FieldNotFound` is shown as a generic error, not a conflict.
- **Fix**: Treat a mismatch between `done_hash` being present and the fresh note having `Done` as `Vault(Conflict)`. Add two integration tests.
- **Decision**: FIXED

### F2 — Per-note errors outside the four listed ones can sink the whole list

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §1 — TaskRow.problem / list_tasks
- **Detail**: `FieldAmbiguous`, `FieldLayoutUnsupported`, and the `Io`/`Busy` errors from `read_note` were not mapped to `problem`. With `?`, one hand-written note would turn the whole view into an error. `problem: Option<String>` also did not say which language the string is in, or what shape it has.
- **Fix**: Make every per-note error a `problem: Option<VaultError>`, mapped to Polish via `taskErrorMessage`. Only folder-level errors fail the list. Add a duplicate-Status fixture.
- **Decision**: FIXED

### F3 — A template inside the tasks folder shows up as a task

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §1 — list_tasks
- **Detail**: With `template: "Tasks/_szablon.md"`, the template would be listed and its status could be edited from the app. Every later task would then inherit that status.
- **Fix**: `list_tasks` skips the entry at `cfg.template`. Add one test.
- **Decision**: FIXED

### F4 — FieldValue::Empty is outside the existing property tests

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 1 §4 — Tests
- **Detail**: The `field_value()` strategy in `edit.rs` generates only Scalar and List, so clearing `Done` was covered by example tests alone.
- **Fix**: Add `Just(FieldValue::Empty)` to `field_value()`. The `replace_fields` proptest draws from the same strategy.
- **Decision**: FIXED

### F5 — Unstated webview boundary details and a missing frontmatter helper

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 §1–2; Phase 2 §1 — render_template
- **Detail**: Three details were unstated:
  - Tauri 2 takes command arguments in camelCase, while serde fields default to snake_case.
  - `TaskError::Vault` serialises nested (`detail.kind === "Conflict"`).
  - `markdown::frontmatter` is private, but the `{{title}}`-in-frontmatter check needs it.
- **Fix**: Use camelCase invoke keys plus `rename_all = "camelCase"`, a nested `TaskError` TS union with `isConflict`, and a new `pub(crate) frontmatter_span`.
- **Decision**: FIXED
