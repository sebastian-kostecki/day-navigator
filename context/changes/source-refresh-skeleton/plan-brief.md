# Shared source refresh skeleton (F-01) — Plan Brief

> Full plan: `context/changes/source-refresh-skeleton/plan.md`
> Research: `context/changes/source-refresh-skeleton/research.md`

## What & Why

`github_refresh.rs` copies most of `gitlab_refresh.rs`: the managed snapshot cell, the thread start with a `ThreadDown` stand-in, the wake loop, the "refreshing" flip, the round tail, the "To do" reconcile logging and the event publication. F-01 (MS-28, issue #28) moves these into one skeleton, `src-tauri/src/source_refresh.rs`. S-01, S-02, S-03 and S-06 then add GitHub behaviours and a GitLab fetch on that skeleton instead of a second copy. Nothing the user sees changes.

## Starting Point

Two refresh modules repeat eleven building blocks almost line for line (research §1). GitLab's 200-line round adds a hidden-issue gate and six mutexes. GitHub waits a fixed 10 minutes and has no `nextRefreshAtMs`. Wake after sleep and "Check now" already go through each state's `wake()` from `runtime.rs`.

## Desired End State

A thin skeleton (`SourceId`, `RefreshSnapshot`, `SourceCell<S>`, `run_loop`, `next_refresh_at_ms`) holds the shared mechanics. Both source modules keep their state type names, fetch, rules, locks and test-facing names, and call the skeleton. GitHub's round returns its own `Duration`. Tests, commands, TS types, log lines (targets included), thread names, event names and Polish texts are unchanged.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Seam | Option A: thin skeleton, thick source | GitLab's gate and lock order stay source-owned, so the roadmap's main risk is avoided | Research |
| `lastSuccessMs` | Stays in the tauri-free `next_snapshot` rules | Moving it would touch tested rules and their test bodies | Research |
| GitHub `nextRefreshAtMs` | Not added; round returns `Duration`, helper exists | A new field would change the TS contract, which belongs to S-02 | Research |
| Wake after sleep / "Check now" | Unchanged in `runtime.rs` via `wake()` | Already shared; no `runtime.rs` edit needed | Research |
| Source identity | One `SourceId` const per source (event, thread, prefix, log target) | All byte-identical strings sit in one reviewable place | Plan |
| Log target | Skeleton logs pass the source's `module_path!()` | The log plugin prints the target, so moved log calls would otherwise change every line | Plan |
| `state_label`, `run_fetch` | Stay per source | Tests call `state_label` by name; `run_fetch` is the source fetch S-06 extends | Plan |
| Order of work | GitHub first, then GitLab | Proves the skeleton API on the plain round before the gated one | Plan |
| Proof of unchanged tests | Diff of `#[cfg(test)]` modules against the merge base, ignoring `use` lines | Mechanical, repeatable in every phase | Plan |
| AGENTS.md | One "Project Structure" bullet | Steers later slices onto the skeleton, away from S-05's lines | Plan |

All Plan rows are "default chosen by agent" (non-interactive run).

## Scope

**In scope:**
- New `src-tauri/src/source_refresh.rs` with unit tests.
- `github_refresh.rs` and `gitlab_refresh.rs` moved onto it.
- One `pub mod` line in `lib.rs`.
- One `AGENTS.md` bullet.

**Out of scope:**
- GitHub interval and next try (S-02), NEW flags and notices (S-03), hiding (S-01), the review-request fetch (S-06).
- A trait-owned round or a macro.
- `runtime.rs` changes, migrations, frontend and TS types, and the roadmap.

## Architecture / Approach

Each source state becomes `{ cell: SourceCell<Snapshot>, <source-only fields> }` with thin `new`/`snapshot`/`wake`/`write` wrappers and its old `publish(app, state)` function. The source round calls the skeleton in this order:

1. `cell.begin_round(app)`;
2. its own fetch and rules;
3. the store (GitLab: under its gate, with `next_refresh_at_ms(wait)`);
4. `publish`;
5. `SOURCE.notify_todo_changed`;
6. its own post-publish steps (GitLab toasts);
7. `SOURCE.end_round`.

`run_loop` waits whatever `Duration` the round returns. `cell.start_thread` covers a failed spawn with the source's `ThreadDown` snapshot.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Skeleton module and GitHub on it | `source_refresh.rs` + its tests; GitHub uses it | A skeleton API that does not fit GitLab; a dead-code clippy failure |
| 2. GitLab on the skeleton | GitLab uses the cell and helpers; `next_refresh_at_ms` added | Reordering a lock, store or publish in the gated round |
| 3. Agent pointer and final checks | `AGENTS.md` bullet; whole-change scope check; Manual rows | Merge overlap with S-05 in `AGENTS.md` |

**Prerequisites:** branch `slice/source-refresh-skeleton` at `develop` + research commit; `node_modules` present.
**Estimated effort:** ~3 short implementer sessions (one per phase).

## Open Risks & Assumptions

- `refresh_round`, `start` and `publish` had no unit tests. New skeleton tests cover the loop and the seq guard, and the thread, emit and tray paths rest on review and the Manual rows.
- S-05 might add code to `gitlab_refresh.rs`. Its code merges onto the cell form through the unchanged wrappers.
- The log-target finding assumes the default log format, which the builder in `lib.rs:151-159` keeps.

## Success Criteria (Summary)

- All gates are green after every phase, and the `#[cfg(test)]` modules of both refresh modules match `develop` apart from `use` lines.
- The built app behaves and logs exactly as 0.11.0 for GitLab and GitHub (Manual rows, checked later by the user).
- S-02 can give GitHub an interval and a next try by changing only `github_refresh.rs`, `github/mod.rs`, settings and the TS mirror.
