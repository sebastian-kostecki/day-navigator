<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Own Tasks Implementation Plan

- **Plan**: context/changes/own-tasks/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

Automated checks run during the review (all pass):

- `cargo fmt --check`: OK.
- `cargo clippy --all-targets -- -D warnings`: OK.
- `cargo test`: 116 unit tests plus every integration and crash suite, 0 failed.
- `npm run build`, `npm run lint`, `npm run format:check`: OK.
- Whole-word grep for `TasksView`, `TaskSettings`, `list_work_tasks` and `mod tasks` in `src/` and `src-tauri/src/`: nothing found. The literal substring grep matches only `OwnTasksView`.

All Manual Progress items are ticked.

## Findings

### F1 — Failed reload after a mutation is swallowed silently

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src/own-tasks/useOwnTasks.ts:29-33, :55-56
- **Detail**: When the list is already loaded, a failed reload keeps the old list and drops the error. `mutate` awaits `reload()` and then returns `null`, so the UI treats the mutation as fully successful:
  - The add form clears, but the new task is missing from the now-stale list. The user may re-add it and create a duplicate.
  - The edit form closes, and the row still shows the old values.
  - A toggled checkbox snaps back to its old state.

  No message is shown, and the loaded view has no retry control. The old `TasksView` on `main` switched to the error state in this case, and `useHealth.ts` shows a separate check error next to the stale data. AGENTS.md says "surface the failure."

  Related: a `NotFound` error tells the user to "odśwież listę", but `mutate` skips the reload on error and the loaded view has no refresh button. This is unlikely while no delete command exists.

  The trigger is narrow: a local store read fails right after a write succeeded, for example `Busy` during the health round's backup.
- **Fix A ⭐ Recommended**: Expose `reloadError` from the hook and show it above the loaded list with `lastSuccessAt` and a "Spróbuj ponownie" action. When a reload fails, `mutate` returns a message such as "Zapisano, ale nie udało się odświeżyć listy." It also reloads after a `NotFound` error.
  - Strength: Mirrors `useHealth.ts` (stale data plus a visible error) and satisfies the "surface the failure" rule without throwing away a readable list.
  - Tradeoff: Adds one piece of hook state and one banner in the view, and the mutation result needs a third outcome (saved, but stale).
  - Confidence: HIGH — the pattern already exists in `src/health/`.
  - Blind spot: Haven't checked whether main's `ErrorState` `lastSuccess` prop suits an inline banner or only full-screen errors.
- **Fix B**: Merge the `OwnTask` that every mutation already returns into the local list, re-sorted on the client. A reload failure then only affects freshness.
  - Strength: The user sees their change immediately, even when the reload fails.
  - Tradeoff: Ordering is a Rust domain rule (AGENTS.md), so the client would duplicate `sort_for_display`. A failed reload still needs surfacing.
  - Confidence: MED — it breaks the "UI only displays results" convention.
  - Blind spot: Drift between the TS and Rust sort orders.
- **Decision**: FIXED via Fix A. Variant: a save followed by a failed reload still counts as saved, so the form never invites a duplicate. The stale list gets an ErrorState banner with the last-refresh time and a retry, and NotFound triggers a reload.

### F2 — Branch is behind main: 3 conflicts and an unplanned app shell

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src/App.tsx, src-tauri/src/lib.rs, src-tauri/src/settings.rs
- **Detail**: `main` gained 5 commits from `gitlab-assigned-list`. `git merge-tree` reports content conflicts in `src/App.tsx`, `src-tauri/src/lib.rs` and `src-tauri/src/settings.rs`:
  - `src/App.tsx`: main now has a view shell (`type View = "gitlab" | "tasks" | "health"`, with nav buttons) whose `"tasks"` entry mounts the deleted `TasksView` as "Zadania (vault)".
  - `src-tauri/src/lib.rs`: adjacent handler-list lines.
  - `src-tauri/src/settings.rs`: a new top-level `RawGitlab` block. `RawSpace` is not touched.

  The plan's "Merge order" section only anticipated S-08. Its fallback ("mount as the `"tasks"` entry, labelled Moje zadania") still applies, though, and no migration conflict exists because main's `MIGRATIONS` is still empty. Main's `ErrorState` also gained an optional `lastSuccess` prop. `OwnTasksView` currently folds the time into `message`, which still compiles.
- **Fix**: Merge `main` into the branch. In App.tsx, mount `<OwnTasksView />` for the `"tasks"` entry with the label "Moje zadania". Optionally switch to `ErrorState`'s `lastSuccess` prop. Then re-run the full check list and a quick manual pass.
  - Strength: Follows the plan's own fallback for a shell that already exists. The other two conflicts are mechanical.
  - Tradeoff: One more manual UI pass after the merge.
  - Confidence: HIGH — conflicts inspected; no schema collision.
  - Blind spot: S-08 (`work-timer`) is still unmerged and will need the v1/v2 renumbering later.
- **Decision**: FIXED. Merged main (7cf94ea). OwnTasksView is the "tasks" entry of the view shell, labelled "Moje zadania". The error state uses the ErrorState lastSuccess prop. Dropped the stale check_plain import from main settings.rs.

### F3 — AGENTS.md still names the removed `blocking` helper

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: AGENTS.md:40
- **Detail**: "call `with_store` inside `spawn_blocking` (the `blocking` helper in `lib.rs`)". This change deleted `blocking` and introduced `on_store` (`src-tauri/src/lib.rs:42-61`), which is now the canonical way to run store commands. Future agents will look for a helper that doesn't exist.
- **Fix**: Replace "the `blocking` helper" with "the `on_store` helper" on AGENTS.md:40.
- **Decision**: FIXED. AGENTS.md:40 now names on_store.

### F4 — `required` on the title input bypasses the Polish message

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/own-tasks/OwnTaskForm.tsx:73
- **Detail**: The native `required` bubble blocks an empty title in the WebView's locale, so "Tytuł jest wymagany." never appears for an empty field. It still appears for a whitespace-only title. The plan says Rust is the only limit.
- **Fix**: Drop `required` so an empty title reaches Rust and shows `TitleEmpty` inline.
- **Decision**: FIXED. Removed required from the title input.

### F5 — `lib.rs` imports `Space` through the vault re-export

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/lib.rs:17
- **Detail**: `use crate::vault::{Space, VaultError, VaultRegistry};` feeds the new `own_task_default_space` (:93). The plan moved `Space` to `crate::space` so that own tasks don't depend on vault code, and `own_tasks.rs` and the tests already use `crate::space::Space`.
- **Fix**: Import `crate::space::Space` in lib.rs and drop it from the vault import.
- **Decision**: FIXED. lib.rs imports crate::space::Space.

### F6 — The note preview has no way to expand

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/App.css:259
- **Detail**: The plan's "collapsed preview" is implemented as `-webkit-line-clamp: 3` with no expand control, so a note longer than 3 lines is readable only through "Edytuj". Line breaks are kept, as planned.
- **Fix**: Add a per-row "więcej / mniej" toggle that removes the clamp, or record that read-via-edit is intended.
- **Decision**: FIXED. NotePreview shows "Pokaż całą notatkę" / "Zwiń notatkę" only when the clamped note overflows (ResizeObserver).

### F7 — Two empty messages at once with zero tasks and "Pokaż zrobione" on

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/own-tasks/OwnTasksView.tsx:92-93, :204
- **Detail**: With no tasks at all, "Nie masz jeszcze żadnych zadań." shows, and turning on "Pokaż zrobione" adds "Brak zrobionych zadań." below it. The plan does not cover this combination.
- **Fix**: Hide the "Zrobione" section (or its empty message) when `tasks.length === 0`.
- **Decision**: FIXED. The Zrobione section is hidden when there are no tasks at all.

### F8 — Every "Edytuj" button has the same accessible name

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/own-tasks/OwnTasksView.tsx:143
- **Detail**: The row checkbox's `aria-label` includes the task title, but the edit button is a bare "Edytuj" on every row.
- **Fix**: Add ``aria-label={`Edytuj: ${task.title}`}`` to the edit button.
- **Decision**: FIXED. The edit button has aria-label "Edytuj: <title>".

### F9 — Minor backend nits: sort clones, `corrupt()` label, no `due` CHECK

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/own_tasks.rs:171-190, :194-196; src-tauri/src/store/schema.rs
- **Detail**:
  - `sort_for_display` clones `due` and `created_at` inside the key closure on every comparison. This is negligible at personal-list scale.
  - `corrupt()` reports a bad stored `space` or `priority` as `rusqlite::Error::InvalidQuery`. That is safe, because no value leaks, but it names the wrong cause.
  - The migration has no `CHECK` on the `due` format. Only Rust validates it.
- **Fix**:
  - Use `as_deref()` / `as_str()` in the sort key.
  - Add a comment on `corrupt()`.
  - Leave `due` to a future migration if it is ever wanted. Never edit v1.
- **Decision**: FIXED. Borrowed sort key (display_key), comment on corrupt(). The due CHECK is deferred to a future migration; v1 stays frozen.
