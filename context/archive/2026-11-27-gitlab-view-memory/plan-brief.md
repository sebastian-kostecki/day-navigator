# Remembered GitLab View State — Plan Brief

> Full plan: `context/changes/gitlab-view-memory/plan.md`
> Research: `context/changes/gitlab-view-memory/research.md`

## What & Why

After quitting and restarting the app, "Praca (GitLab)" should show the same type choice ("Wszystko" / "Zgłoszenia" / "MR-y") and the same "Pokaż ukryte" state (the "Ukryte" expansion). Today both reset on every restart, so the user has to set them again. The view filter (project/label/milestone values) keeps resetting because those values are work data. Roadmap M-4, S-05, anchor MS-34 (issue #34).

## Starting Point

Both values are plain `useState` in `src/gitlab/useGitlabListView.ts` behind `GitlabListViewProvider`, which mounts at app start and already survives tab switches and window close; only a process restart loses them. The app data store is at v13 and has a close precedent: the GitLab view's seen-until mark (v9).

## Desired End State

A one-row table (`gitlab_view_state`, migration v14) holds the type and the hidden toggle. The provider loads it once at start and saves it on every click; consumers (`GitlabView.tsx`) see the same contract as today. A failed load or save never blocks the view: it starts with the defaults and Rust logs the error kind.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| What is remembered | `kind` (all 3 values) and `showHidden`; no filter values | Filter values are work data | User decision |
| Where | App data store, migration v14 | `settings.json` is hand-written and read-only for the app | User decision / Research |
| AGENTS.md | App-data list + "in memory only" sentence updated in this change | The record must be listed where app data is enumerated | User decision |
| Table shape | One fixed row (`id = 1` CHECK, `kind` CHECK, `show_hidden` 0/1), not account-keyed | Typed like the rest of the schema; no dependency on `GitlabRefreshState` (F-01) | Plan (default chosen by agent) |
| Defaults | In Rust: no row loads `{ all, false }` | One source of truth for the first start and the upgrade | Plan (default chosen by agent) |
| Write policy | Saved from the (stable) setters on every value-changing click, no persist effect; one command with both fields, chained after the load, one at a time, latest value from a ref kept current by the setters and the load | Rare clicks; nothing lost on a hard kill; no stale overwrite; lint-clean (no ref access in render) | Plan (default chosen by agent; review F1/F2) |
| Load timing | Once at provider mount; apply a field only if the user has not changed it; a same-value click is not a change; StrictMode-safe | Never overwrite a choice made while loading; re-clicking the selected option keeps the restore | Plan (default chosen by agent) |
| Invalid stored row | One fixed value-free reason `"invalid view state row"`; parse unit-tested | CHECKs make it unreachable through SQL | Plan (default chosen by agent; review F3) |
| Failures | No UI hint; TS swallows, Rust logs kind only | Convenience record; store health is already in the tray | Plan (default chosen by agent) |
| Placement | New module `gitlab_view_state`, `crate::blocking` + `with_store`, own tagged error + TS mirror | Keeps out of `gitlab_refresh.rs` (F-01) and limits `lib.rs` to 3 lines | Plan (default chosen by agent) |
| Tests | Integration (defaults, round-trip, one row, DB refusal, v13→v14 upgrade), serde and DB-text-parse unit test, crash test | Data slices reuse the crash harness (AGENTS.md) | Plan (default chosen by agent) |

## Scope

**In scope:**
- Migration v14 and the `gitlab_view_state` Rust module with two async commands
- TS mirror and `api.ts` functions; load/save logic in `useGitlabListView.ts`
- Fix of the hard-coded `13` asserts in `src-tauri/tests/events.rs`
- AGENTS.md: app-data list and Frontend Conventions sentence

**Out of scope:**
- Persisting filter values or any other view state
- Account-keyed or key/value storage
- Any change to `GitlabView.tsx`, `gitlab_refresh.rs`, `github_refresh.rs`, the PRD or the roadmap

## Architecture / Approach

`GitlabListViewProvider` → `useGitlabListViewSource` → `getGitlabViewState()` / `setGitlabViewState(view)` in `src/gitlab/api.ts` → Tauri commands `gitlab_view_state` / `set_gitlab_view_state` (`src-tauri/src/gitlab_view_state/commands.rs`) → `crate::blocking` → `StoreHandle::with_store` → `load` (one read) / `save` (one upsert in one `write`) on table `gitlab_view_state`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Store record, Rust commands and TS mirror | v14, module, commands, TS types + api, tests, events.rs fix, AGENTS.md data list | Migration number collides if another slice lands one first |
| 2. Load and save in the list-state hook | Remembered type and hidden toggle across restarts; AGENTS.md Frontend sentence | Load/save ordering in the hook (stale overwrite, StrictMode) |

**Prerequisites:** worktree on `slice/gitlab-view-memory`; `npm install` once (revert `package-lock.json` changes).
**Estimated effort:** ~1 session across 2 phases.

## Open Risks & Assumptions

- Assumes no other migration lands in this wave; otherwise merge `develop`, renumber v14 and its one version assert (the untouched-file gates diff against `git merge-base HEAD develop`, so they survive that merge).
- One global row: with several GitLab accounts over time, they share the choice (accepted).
- Manual checks need a built app (`npm run tauri build -- --debug --no-bundle`); they are left for the user in an integrated build.

## Success Criteria (Summary)

- "MR-y" + "Pokaż ukryte" survive "Zakończ" and restart; filter values do not.
- A refused or missing store never blocks the GitLab view; logs carry error kinds only.
- All gates green; `GitlabView.tsx` unchanged.
