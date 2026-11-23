# Own Tasks — Plan Brief

> Full plan: `context/changes/own-tasks/plan.md`

## What & Why

Roadmap slice S-06 (FR-006, FR-007, FR-008, FR-010) gives the user their own tasks inside the app, one of the three sources the narrowed MVP is built on. In the "Moje zadania" view the user can:
- add a task in the Work or Private space;
- edit its title and a short plain note;
- set an optional due date and priority;
- mark it done or reopen it.

The PRD drops the vault-based task screen (`[removed] S-05`), because the MVP no longer touches Obsidian vaults. This change removes it.

## Starting Point

The SQLite app store (F-01) is done but holds no records yet: the migration list is empty and no command uses it. The app still shows the old "Zadania" screen, which reads and writes `Tasks/` in the work vault through `tasks.rs` and four commands. Navigation (S-01) and the work-time counter (S-08) are not planned yet.

## Desired End State

"Moje zadania" replaces the old screen. Tasks live in an `own_tasks` table (migration v1) and survive restarts and crashes. Open tasks are ordered by due date, then priority, then creation. Done tasks sit behind "Pokaż zrobione". A Wszystkie / Praca / Prywatne filter narrows the list. New tasks default to Private until S-08 wires the counter. The vault task code is gone, and an old `settings.json` with a `tasks` block still loads.

## Key Decisions Made

| Decision           | Choice                                                         | Why (1 sentence)                                                                 |
| ------------------ | -------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Priority scale     | Fixed 3 levels: Wysoki / Średni / Niski (optional)             | Typed and sortable, nothing to configure, and never required (FR-010).            |
| Limits             | Title 1–200 chars, single line; note ≤ 2000 chars, plain, line breaks kept | Makes "short note" testable and keeps it from becoming a second Obsidian.   |
| List order         | Open: due asc (none last), priority high→low (none last), created; done hidden behind a toggle | Near-due work surfaces first; tasks can't be deleted, so done tasks must not clutter the list. |
| Reopen             | Done is a toggle; `done_at` cleared on reopen                  | Fixes mis-clicks and lets S-07 untick (FR-008 "and vice versa").                 |
| Spaces in the view | One list, filter tabs (default Wszystkie), space badge per row | Everything in one place, quick narrowing, space editable per task.               |
| Removal depth      | Screen + `tasks.rs` + commands + task settings; vault layer and fixtures stay | No dead feature code without pre-empting roadmap Open Question #4.               |
| Legacy settings    | `tasks` block accepted and ignored (`IgnoredAny`)              | `deny_unknown_fields` would otherwise turn existing settings red.                |
| Default space seam | Rust `default_space(counter_running)`; command passes `false` until S-08 | Domain rule lives in Rust; S-08 changes one argument.                        |
| Space enum         | Moved to `src-tauri/src/space.rs`, re-exported from `vault`    | One enum for both features; the vault API is unchanged.                          |
| Navigation         | Swap `TasksView` → `OwnTasksView` in the stacked `App.tsx`     | S-01 owns navigation; no invented view switcher.                                 |

## Scope

**In scope:**
- `own_tasks` migration v1 and a Rust domain module (validation, ordering, default space, store operations)
- Five async store commands through a generic `spawn_blocking` store helper
- A `src/own-tasks/` feature folder (view, form, hook, API, types) with loading, error (`ErrorState` + last refresh) and empty states
- Removal of the vault task screen, its backend, tests and task settings
- Unit, integration and crash tests on invented data

**Out of scope:**
- Deleting tasks (FR-009 removed), the "Do zrobienia" list (S-07), reading the counter (S-08), navigation/header (S-01)
- Overdue highlighting, search, manual ordering, Markdown
- Configurable priorities, importing vault tasks, removing the vault layer

## Architecture / Approach

The work runs bottom-up and follows the existing patterns:
- a plain Rust module `own_tasks.rs` with no `tauri` import, time passed in, and typed serde errors that never carry user content;
- one appended migration;
- thin async commands that call `StoreHandle::with_store` inside `spawn_blocking`;
- a React feature folder where `api.ts` is the only `invoke` importer.

Ordering and the default-space rule live in Rust; the UI only filters by space and done state. S-07 will reference `own_tasks(id)` and reuse `done_at` as the single done state.

## Phases at a Glance

| Phase                                  | What it delivers                                                       | Key risk                                                              |
| -------------------------------------- | ---------------------------------------------------------------------- | --------------------------------------------------------------------- |
| 1. Own-task domain and store records   | `space.rs`, `own_tasks.rs`, migration v1, unit/integration/crash tests | Migration v1 collides with S-08's; user content leaking into errors   |
| 2. Commands and "Moje zadania" view    | 5 commands, `src/own-tasks/`, view swapped into `App.tsx`              | `App.tsx` merge conflicts with S-01/S-08; store lock contention       |
| 3. Remove the vault task screen        | Old code, tests and task settings gone; legacy settings still load     | `deny_unknown_fields` break; orphaned `pub(crate)` helpers fail clippy |

**Prerequisites:** F-01 app-data-store (done, on `main`).
**Estimated effort:** ~2–3 sessions across 3 phases.

## Open Risks & Assumptions

- S-08 also adds a migration. Whichever merges second renumbers its own migration (safe before any release).
- S-01 and S-08 will reshape `App.tsx`. This slice keeps its `App.tsx` change to a one-line swap to limit conflicts.
- The default space is always Private until S-08 replaces `false` in `own_task_default_space`.
- Browser `maxLength` counts UTF-16 units while Rust counts chars. Rust is authoritative and returns a Polish message.

## Success Criteria (Summary)

- The user adds, edits, completes and reopens own tasks in Work or Private, and they survive a restart or crash.
- The list orders near-due and high-priority work first, hides done tasks until asked, and filters by space.
- The old vault "Zadania" screen is gone, and existing settings files still load green.
