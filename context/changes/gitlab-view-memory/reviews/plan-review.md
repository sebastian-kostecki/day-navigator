<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Remembered GitLab View State Implementation Plan

- **Plan**: context/changes/gitlab-view-memory/plan.md
- **Mode**: Deep (claims verified inline against the code, no sub-agent)
- **Date**: 2026-11-27
- **Verdict**: SOUND
- **Findings**: 0 critical, 1 warning, 3 observations

Run non-interactively: the triage question was answered with "Save report only" (the safest option); every decision stayed PENDING for the agent that revises the plan. All four were resolved and applied to `plan.md` / `plan-brief.md` on 2026-11-27 (see each Decision line).

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | PASS |

## Grounding

Grounding: 9/9 paths ✓ (`useGitlabListView.ts`, `GitlabListViewProvider.tsx`, `GitlabView.tsx`, `api.ts`, `types.ts`, `lib.rs`, `store/schema.rs`, `tests/events.rs`, `tests/common/crash.rs`), 8/8 symbols ✓ (`crate::blocking` is `pub(crate)` at `lib.rs:38`, `on_store` private at `lib.rs:54`, `StoreHandle::with_store(today, f)` at `store/mod.rs:242`, `Store::read(&self)` / `Store::write(&mut self)` at `store/mod.rs:142,153`, `NewAssignmentError` shape, `mark_gitlab_seen` logging, `events.rs:184,200` asserts, `KindFilter` imported only by `GitlabView.tsx:33`), brief↔plan ✓, Progress↔Phase ✓ (8 + 13 rows, titles match, no checkboxes in phase blocks).

Confirmed along the way (no finding):

- v14 is the next free version (`schema.rs` ends at v13; `validate` requires contiguity). Only `tests/events.rs:184` and `:200` pin 13; every other upgrade test uses `MIGRATIONS.len()` or a fixed prefix.
- `with_store` takes a blocking `Mutex`, so a mount load during the first health round waits instead of failing; `Busy` only comes from SQLite's 5 s `busy_timeout`.
- A zero-byte `day-navigator.db` is refused as `Damaged` (`store/mod.rs:61-65`), so Manual row 2.12 really exercises a refused store.
- The crash design fits the harness: `run_writer_until` drains lines printed before the kill, so "`state_of(last)` or `state_of(last + 1)`" (and the `last == 0` case) is the exact bound.
- `crate::blocking` + `with_store` matches `mark_gitlab_seen`; the `lib.rs` diff stays three lines away from the `gitlab_refresh::*` / `github_refresh::*` lines F-01 may touch; `GitlabView.tsx` really needs no edit (the `KindFilter` re-export keeps `:33` compiling, `setShowHidden((s) => !s)` keeps its updater signature).
- The failure policy (defaults, no UI hint, Rust logs the kind) does not break "never render an empty list for a failed source": this record is a preference, not a source, and the list itself still comes from the GitLab snapshot with its own error state.

## Findings

### F1 — The "latest" ref is not pinned, so the first save after the load can write the pre-load value

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details ("State sequencing"); Phase 2 §1 (persist effect)
- **Detail**: The plan promises that a click before the load finishes never writes the default for the field the load then restores, by reading the value "from a latest ref at send time". It never says where that ref is updated. (a) If it is updated in the persist effect (the natural reading), the sequence is: user picks "MR-y" before the load → a save is queued on `load.then(...)` → the load callback calls `setView(merged)` (which only schedules a render) → the very next microtask runs the queued save, which reads the old ref `{ mergeRequest, showHidden: false }` and overwrites a stored `showHidden: true`. A second save after the re-render repairs it, so the end state is right, but the stated guarantee does not hold and a quit in between loses the stored value. (b) If the ref is assigned during render (`latest.current = view` in the hook body), `eslint-plugin-react-hooks` 7.1 (`reactHooks.configs.flat.recommended` in `eslint.config.js`) reports `react-hooks/refs`, and `npm run lint` (gates 2.1/2.4) fails. Related edge: `Segmented` calls `onChange` for the option that is already selected (`src/components/Segmented/Segmented.tsx:36`), so clicking "Wszystko" before the load would mark `kind` as user-changed and silently drop the remembered type.
- **Fix A ⭐ Recommended**: Drive saves from the setters and drop the persist effect.
  - Approach: keep `latest` (a ref) as the value the next save sends. `setKind(k)` / `setShowHidden(update)` compute `next` from `latest.current` (`update(latest.current.showHidden)`, computed once, outside any `setState` updater); if `next` equals `latest.current` do nothing; otherwise mark the field as user-changed, assign `latest.current = next`, call `setView(next)` and queue `chain.current = chain.current.then(() => setGitlabViewState(latest.current)).catch(() => {})`. The load callback (skipped when `cancelled`) builds `merged` from `latest.current`, the loaded value and the user-changed flags, assigns `latest.current = merged`, then `setView(merged)`. Wrap both setters in `useCallback(…, [])`.
  - Strength: no window where the ref lags the state; no effect reacting to state ("you might not need an effect"); no ref access during render, so the React 19 hooks lint stays green; same-value clicks neither save nor block the restore.
  - Tradeoff: the hook text changes from the plan's description, so Critical Implementation Details and Phase 2 §1 need rewording.
  - Confidence: HIGH — event handlers are not double-invoked under StrictMode, and only the mount effect remains (already guarded by `cancelled`).
  - Blind spot: None significant.
- **Fix B**: Keep the persist effect, but pin the ref writes.
  - Approach: state in the plan that `latest.current` is assigned in the setters and in the load callback (with the merged value, before `setView`), never during render; the persist effect only queues the save.
  - Strength: smallest edit to the plan text.
  - Tradeoff: two places write the ref and an effect still mirrors state into IPC; same-value clicks still need an explicit "unchanged" check.
  - Confidence: MED — correct if the implementer follows both pins exactly.
  - Blind spot: Effect ordering against the mount effect must still be checked under StrictMode.
- **Decision**: RESOLVED — Fix A applied (plan revision, 2026-11-27): no persist effect; stable setters compute `next` from `latest.current`, return on a same-value call, otherwise mark the field, assign `latest.current = next`, `setView(next)` and chain the save; the load callback assigns the merged value to `latest` before `setView`; the chain starts from the load promise; refs written only in the setters and the mount effect, never during render; the StrictMode double mount is reasoned through in Critical Implementation Details.

### F2 — Setters must stay referentially stable

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §1
- **Detail**: Today `setKind` / `setShowHidden` are `useState` setters (stable) and sit in the `useMemo` dependency list (`useGitlabListView.ts:44-47`). The new wrapped setters are not stable unless the plan says so; then the context value changes on every provider render and `react-hooks/exhaustive-deps` may ask for extra deps.
- **Fix**: Add "both setters are `useCallback(…, [])` and read only refs" to the Phase 2 §1 contract (covered by Fix A of F1).
- **Decision**: RESOLVED — applied with F1 Fix A: both setters are `useCallback(…, [])` and read only refs (Phase 2 §1, Critical Implementation Details).

### F3 — One error text for two different invalid values, and the parse refusal is untested

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §2 (`load`), Implementation Approach decision 9
- **Detail**: `load` returns `Sql { reason: "unknown view kind" }` also for a `show_hidden` other than 0/1, which is misleading in the log. The CHECKs make both cases impossible to reach through SQL, so the integration tests never exercise this branch, and the unit test only covers serde, not the DB-text parse.
- **Fix**: Use one fixed, value-free reason such as `"invalid view state row"` for both cases, and add the parse pair (`as_db` / parse of `"epic"` and of `2`) to the `#[cfg(test)]` unit test.
- **Decision**: RESOLVED — applied: one fixed reason `"invalid view state row"` via a private `from_row` parse; the unit test covers the `as_db` / `from_db` round-trip and `from_row("epic", 0)` / `from_row("all", 2)` (Phase 1 §2, decision 9, Testing Strategy).

### F4 — The untouched-file gates diff against a fixed commit

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 Automated 1.7, 1.8; Phase 2 Automated 2.5; Critical Implementation Details ("Migration numbering")
- **Detail**: The plan itself foresees taking `develop` into the slice if another migration lands first. After such a merge, `git diff 2b6ac08 -- src/gitlab/GitlabView.tsx src-tauri/src/gitlab_refresh.rs …` also shows F-01's and S-04's changes, so 1.7, 1.8 and 2.5 fail although this slice touched nothing.
- **Fix**: Diff against `$(git merge-base HEAD develop)` instead of `2b6ac08` in 1.7, 1.8 and 2.5 (same result today, still right after a `develop` merge).
- **Decision**: RESOLVED — applied: gates 1.7, 1.8 and 2.5 and their Progress rows diff against `$(git merge-base HEAD develop)`; the Migration numbering note says so.
