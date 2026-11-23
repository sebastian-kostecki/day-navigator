<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Own Tasks Implementation Plan

- **Plan**: context/changes/own-tasks/plan.md
- **Mode**: Deep (codebase verification run inline)
- **Date**: 2026-11-23
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 4 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding
15/15 paths ✓, 10/10 symbols ✓ (with_store/write/read generics, StoreError variants, Space, RawSpace deny_unknown_fields, blocking, test_store_dir, run_writer_until, App.css :119/:206, TasksView :31-46), brief↔plan ✓ (both shared the stale S-08 claim, F1)

Verified OK: `on_store` fits `with_store`'s `E: From<StoreError>`. `IgnoredAny` with an underscore field keeps `deny_unknown_fields` without dead-code warnings. A non-empty `MIGRATIONS` breaks no existing store test (they use their own test migrations). Only `App.tsx` imports from `src/tasks/`. No other code reads `settings.tasks`.

## Findings

### F1 — S-08 already has a plan that collides with this one

- **Severity**: ⚠️ WARNING
- **Impact**: 🔬 HIGH — architectural stakes; think carefully before deciding
- **Dimension**: Architectural Fitness
- **Location**: Current State Analysis; Phase 2 §3; Phase 3 §1; Migration Notes
- **Detail**: The plan claimed S-08 had no plan. `work-timer/plan.md` (status `plan_reviewed`):
  - also takes migration v1 (`work_entries`);
  - rebuilds `App.tsx` into `type View = "tasks" | "work-time" | "health"`, where "tasks" is the old `TasksView` and the default view;
  - mirrors the `TasksView.tsx:82-170` form, so its form may reuse the `.new-task` / `.task-*` CSS that Phase 3 deletes;
  - already exposes `work_time::is_running(&Store)` for S-06.

  The "one-line swap" only holds if own-tasks merges first.
- **Fix A ⭐ Recommended**: Update Current State and add an "if S-08 lands first" branch: migration v2, mount in S-08's "tasks" entry, read `is_running`, grep for reused CSS before deleting it.
  - Strength: Keeps both slices in parallel; each step is concrete.
  - Tradeoff: The plan carries two merge-order variants.
  - Confidence: HIGH — every coupling point was read in S-08's plan.
  - Blind spot: S-08's actual class names aren't written yet.
- **Fix B**: Run this slice after S-08 merges.
  - Strength: One variant and no dead seam.
  - Tradeoff: Serializes the two slices.
  - Confidence: MED — depends on S-08's timeline.
  - Blind spot: S-08's implementation timeline.
- **Decision**: FIXED (Fix A) — new "Merge order with S-08" in Critical Implementation Details; Current State, Phase 2 §1/§3, Phase 3 §1, What We're NOT Doing, Migration Notes and the brief updated.

### F2 — Dev runs migrate the real, shared database

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details; Migration Notes; manual checks 2.4–2.10, 3.4–3.5
- **Detail**: Every worktree and the installed release use the identifier `com.sebas.day-navigator`, so they share one `day-navigator.db`. Consequences:
  - after a dev run, the installed release refuses the file as `NewerSchema`;
  - after a work-timer dev run, this build skips its v1, so `own_tasks` is missing;
  - after renumbering, a database holding `own_tasks` at v1 fails v2 → `MigrationFailed`, refused.
- **Fix A ⭐ Recommended**: Document a back-up-and-clear procedure around manual passes and renumbering.
  - Strength: In scope, zero code, uses the AGENTS.md procedure.
  - Tradeoff: Manual discipline.
  - Confidence: HIGH — follows from `store/mod.rs:93-99` and the shared identifier.
  - Blind spot: None significant.
- **Fix B**: A separate dev identifier.
  - Strength: Removes the problem for all future slices.
  - Tradeoff: Project-wide; a dev settings.json must be maintained; better as its own change.
  - Confidence: MED.
  - Blind spot: Single-instance plugin behaviour.
- **Decision**: FIXED (Fix A) — "Dev runs share the real database" in Critical Implementation Details, the Phase 2 Implementation Note, Migration Notes and the brief. Fix B is worth its own change once S-08 stores real work-time data.

### F3 — maxLength makes check 2.9 impossible

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: End-State Alignment
- **Location**: Phase 2 §2 (OwnTaskForm) vs manual check 2.9
- **Detail**: `maxLength=200/2000` makes the browser silently cut a pasted over-long value, which Rust then accepts. So 2.9 ("refused with the Polish message, text kept") cannot pass. `maxLength` also counts UTF-16 units, so the browser blocks emoji titles that Rust allows.
- **Fix A ⭐ Recommended**: Drop maxLength; Rust is the only limit; optional live counter.
  - Strength: 2.9 becomes testable; matches "Rust is authoritative".
  - Tradeoff: The user can type past the limit before the refusal.
  - Confidence: HIGH.
  - Blind spot: None significant.
- **Fix B**: Keep maxLength and rewrite 2.9.
  - Strength: No error UX needed.
  - Tradeoff: Silent truncation; the emoji undercount stays.
  - Confidence: HIGH.
  - Blind spot: None significant.
- **Decision**: FIXED (Fix A)

### F4 — Orphaned-helper guidance misses frontmatter_span

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Critical Implementation Details; Phase 3 §2
- **Detail**: `markdown::frontmatter_span` (`markdown.rs:200`) is pub(crate), with tasks.rs as its only caller and no tests. After the removal it is dead code under `-D warnings`, and neither "narrow" nor "keep with tests" fixes that. `from_hex`, `replace_fields` and `create_note_in_existing_folder` are `pub` and never warn.
- **Fix**: Per-item actions: `check_plain` → private, `BOM` → private, `frontmatter_span` → delete, the others unchanged.
- **Decision**: FIXED

### F5 — Tests hard-code migration version 1

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §4
- **Detail**: "reports user_version = 1" breaks on the planned renumbering.
- **Fix**: Assert against `schema::MIGRATIONS.last().unwrap().version`.
- **Decision**: FIXED

### F6 — Empty-state messages undefined at the edges

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2 (OwnTasksView states)
- **Detail**: Some cases had no defined message: Wszystkie with only done tasks, and an empty "Pokaż zrobione" section.
- **Fix**: Three explicit messages: no tasks at all; no open tasks after the filter (space wording only for Praca/Prywatne); no done tasks.
- **Decision**: FIXED
