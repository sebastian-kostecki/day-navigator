# "To do" List — Plan Brief

> Full plan: `context/changes/todo-list/plan.md`

## What & Why

Roadmap slice S-07 (FR-011, FR-012, US-01 second half) completes the morning flow. In a "Do zrobienia" view the user picks what they will work on today from their own tasks and their assigned GitLab issues and MRs. A limit (default 5) keeps it from becoming another full list. Unfinished items carry over to tomorrow, and items closed in GitLab tick themselves.

## Starting Point

Own tasks already exist in the SQLite store, with `done_at` as their single done state (S-06). The assigned GitLab list exists, but only in memory and only as "assigned and open" items (S-01). There is no "To do" table, no limit setting and no view. `App.tsx` opens on "Praca (GitLab)".

## Desired End State

"Do zrobienia" is the first tab and opens at start. It shows today's open items in pick order, with an "od DD.MM" badge on carried-over ones, and a "Zrobione dziś" part below. An "Dodaj z list" picker adds GitLab items and open own tasks until the open count reaches the limit. Ticking an own task is the same state as "done" in "Moje zadania". After a successful GitLab refresh, entries no longer assigned and open tick themselves ("zamknięte w GitLabie"). No VPN never changes anything.

## Key Decisions Made

| Decision          | Choice                                                                  | Why (1 sentence)                                                                 |
| ----------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| What the limit counts | Open (unticked) items on today's list; it gates adding only         | Keeps the focus cap but lets the user pull the next item after finishing one.     |
| Closed detection  | Missing from a **successful** assigned-and-open refresh → auto-tick     | No new `glab` call; never ticks on no-VPN or error; unassigned also counts as done. |
| Picking UX        | "Dodaj z list" picker inside the "Do zrobienia" view                    | Matches PRD flow step 5; source views stay unchanged; one place shows the limit.  |
| Ticked items      | Stay visible (struck through, untickable) until the end of the local day | Mis-clicks are easy to undo and the day's progress stays visible.                 |
| Placement         | First nav tab and the default view                                      | US-01's morning starts with yesterday's carry-over in front of the user.          |
| Order             | Pick order (oldest first)                                               | Stable and predictable; carried-over items lead naturally.                        |
| Carry-over marker | "od DD.MM" badge with the pick date                                     | Makes items that keep sliding visible.                                            |
| Own-task tick     | Reuses `own_tasks.done_at` inside the To-do transaction (`set_done_in`) | FR-008: one state, no second "done" column.                                       |
| Roll-over         | Run first in every To-do write and in the own-task done/reopen command (local date); entries ticked before today get `closed_at` | No scheduler; a task reopened later never comes back by itself, even before the list is read. |
| Limit setting     | `settings.json` `"todo": { "limit": 1–20 }`, default 5; invalid → error shown, adding refused | Follows the per-block loader; a failure is surfaced, never silently defaulted. |
| GitLab pick source | The backend looks the item up in the live snapshot by kind + reference | The store never takes title or URL text from the UI; the link stays host-checked. |

## Scope

**In scope:**
- Migration v3 `todo_items` and a tauri-free `todo` module (pick, tick, untick, remove, list with roll-over, limit, reconcile)
- `own_tasks::set_done_in` and the `todo.limit` settings loader
- A reconcile hook in the GitLab refresh round and a `todo-changed` event
- Five async commands and the `src/todo/` view with a picker, badges and loading/error/empty states; the view first in nav
- Unit, integration and crash tests on invented data

**Out of scope:**
- GitHub items (S-05 is blocked), writing to GitLab, per-item GitLab state queries, auto-untick
- Manual reorder, a settings UI, trimming when the limit is lowered, history of past days
- A frontend test runner

## Architecture / Approach

The work is bottom-up, following S-06 and S-08:
- `todo_items` holds one row per pick. An own entry takes its ticked state from `own_tasks.done_at`. A GitLab entry stores its own tick and a reference/title snapshot.
- Visibility on day D: not removed and not closed, and either open or ticked on D.
- `refresh_round` reconciles GitLab entries only when the new snapshot is `Ok`, before publishing.
- Thin async commands read the limit from `settings.json` per call and return the fresh list.
- The React feature only displays results and sends actions. The picker combines `useGitlab()` and `listOwnTasks()`.

## Phases at a Glance

| Phase                                 | What it delivers                                                             | Key risk                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| 1. To-do domain and store records     | Migration v3, the `todo` module, `set_done_in`, `todo.limit`, unit/integration/crash tests | Visibility and roll-over edge cases (reopen after close, day boundary) |
| 2. GitLab reconciliation              | Auto-tick and title refresh after successful rounds; `todo-changed`          | Ticking on a failed or empty round; a store error leaking into GitLab status |
| 3. Commands and "Do zrobienia" view   | 5 commands, `src/todo/`, picker, badges, states; default tab                 | UI drift from the Rust shape; a list left open over midnight              |

**Prerequisites:** S-01 gitlab-assigned-list and S-06 own-tasks (both done, on `main`).
**Estimated effort:** ~2–3 sessions across 3 phases.

## Open Risks & Assumptions

- Being unassigned from a GitLab item ticks it like a close (accepted with the "gone from list" decision).
- Changing the GitLab `host` or `user` in settings makes the old host's open entries tick on the next successful round.
- S-02 (visibility filter) must keep reconciliation on the unfiltered assigned-and-open set, or filtered-out items would tick themselves.
- `tauri dev` uses the installed app's `day-navigator.db`; manual passes back it up and clear it first (AGENTS.md procedure).
- GitHub items need a new `TodoEntry` variant and source once S-05 lands; the schema's `source` CHECK will need a new migration then.

## Success Criteria (Summary)

- Each morning the user sees yesterday's unfinished items, adds today's picks up to the limit, and ticks them off during the day.
- Own tasks show the same done state in both views; GitLab items closed in the source tick themselves, and no VPN never changes the list.
- The list survives restart and crash, and every failure is shown instead of an empty list.
