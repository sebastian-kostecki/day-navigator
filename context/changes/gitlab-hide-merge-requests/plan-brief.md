# Hide GitLab merge requests (S-07) — Plan Brief

> Full plan: `context/changes/gitlab-hide-merge-requests/plan.md`
> Research: `context/changes/gitlab-hide-merge-requests/research.md`

## What & Why

In "Praca (GitLab)" the user can hide an issue until it changes, but not a merge request, so MRs they are waiting on (their own or ones in "Do review") clutter the list. This slice (M-4 S-07, MS-33, issue #33) lets them hide any MR and get it back marked "Zmienione" when someone else comments or the MR returns to the list after a state change, the FR-003 rule. Approvals, commits and pipelines never bring it back.

## Starting Point

GitLab issue hiding works (table `hidden_issues`, round in `hidden_issues/round.rs`). S-01 made the rules generic and added the table `hidden_items` (v15), already shaped for GitLab MR rows (host scope, global id, required `project_id`/`iid`). S-06 added the "Do review" list, disjoint from the assigned MRs by id. MR rows have no hide buttons and there is no MR notes call.

## Desired End State

Every MR row, in the main table and in "Do review", has "Ukryj do zmiany". Hidden MRs show dimmed under the one "Pokaż ukryte (n)" toggle in the main table (with the "Do review" tag where it applies). A comment by someone else, or leaving and re-joining the open lists, brings the MR back to its own section with "Zmienione". Issue hiding is unchanged.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Scope | Assigned and "Do review" MRs | User's decision | Research (user) |
| What brings an MR back | Comment by someone else, or leave + return to the open lists | FR-003 rule; approvals/commits/pipelines excluded | Research (user) |
| Comment data | New read-only `glab api .../merge_requests/<iid>/notes` `CliCall` + guard + fake-cli | User's decision; parser reused unchanged | Research (user) |
| Storage | `hidden_items` rows, no migration | v15 already allows gitlab MR rows; S-03 owns v16 | Research (user) |
| Merged/closed MR | O1: no extra call; leave + return only | Same as issues and GitHub; O2 is a possible follow-up | Plan (default by agent; orchestrator confirms with the user at the end) |
| Draft/ready toggle | Not a state change | Not in the user's rule; draft toggles are system notes | Plan (default by agent; orchestrator confirms with the user at the end) |
| MR fingerprint | Kind-aware `HideItem for GitlabItem`: empty for MRs, so labels never bring an MR back | One impl, issue fingerprint byte-for-byte the same | Plan (default by agent; orchestrator confirms with the user at the end) |
| Assigned + review MR | One record per global id, hidden from both places | Lists are disjoint by id after `split_review_requests` | Research (user) |
| Hidden review MRs | One `hiddenMergeRequests` list in the main table, same toggle | Main table already owns the toggle and mixes kinds | Plan (default by agent) |
| Hidden MR and "To do" | As hidden issues: not pickable, entry never ticked | Same behaviour for both kinds | Plan (default by agent) |
| Rounds | Issue round and MR round separate, one write each, same gate | Independent tables; existing functions each own one write | Plan (default by agent) |
| VPN drop mid-round | Shared stop: after an issue-check stop the MR calls are skipped, never two rounds in a row | One 20 s timeout per drop, not two; no starvation | Plan (default by agent) |
| "Do review" all hidden | Empty text plus "Ukryte do review: n (widoczne po „Pokaż ukryte”)." | Never reads as "nothing to review" | Plan (default by agent) |
| Commands | `hide_/unhide_gitlab_merge_request`, `dismiss_gitlab_merge_request_change` by `id` | Mirrors the issue commands, issue contract unchanged | Plan (default by agent) |
| Notes call key | Reuse `IssueRef` (project id + iid) | Same opaque, non-zero numbers; no extra guard surface | Plan (default by agent) |

## Scope

**In scope:** MR notes call and guard; snapshot list `hiddenMergeRequests` + filter count; MR records, partition and round; refresh wiring (`act_on_item`), three commands; UI buttons on every MR row, toggle count, texts; "Do zrobienia" flag/count; one PRD line; AGENTS.md app-data clause.

**Out of scope:** migration; showing a merge/close as "Zmienione" (O2); `glab api user`; hiding in "Zamknięte"; NEW-flag or seen-key changes beyond "hidden is never NEW"; roadmap edits; new FR.

## Architecture / Approach

The refresh round runs `check_round` (issues) and the new `check_mr_round` (MRs, over `merge_requests` + `review_requests`; no call when the issue checks just stopped, never two rounds in a row) with no lock, then under `hidden_gate` `commit_round` and `commit_mr_round`, each one store write. Every re-shape (round, failed round, command) reads both kinds' records together (`GitlabHidden`, `list_gitlab`) and partitions issues and MRs, so hiding one kind never un-hides the other. The UI keys busy rows by kind and id.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. MR notes call | `GlabMergeRequestNotes`, guard shape, fake routes, fetch | Guard near-misses; issue/MR notes told apart by path |
| 2. Snapshot model | `hidden_merge_requests` + count, TS mirror, enumerations | ~40 struct literals to update |
| 3. MR hiding core | Kind-aware fingerprint, records per host, partition | Issue fingerprint must not change |
| 4. MR round | `check_mr_round` / `commit_mr_round`, `mr_calls_allowed` + fake-cli tests | Review-only MRs must count as listed |
| 5. State shaping + commands | `GitlabHidden` in every re-shape, `act_on_item`, 3 commands | A re-shape with issue records only would un-hide MRs |
| 6. Round wiring | MR checks and commit in `refresh_round`, MR cursor and skip flag | Both commits under the one gate, lock order kept |
| 7. UI + docs | Buttons, toggle, texts, "Do zrobienia", PRD, AGENTS.md | Shared AGENTS.md line with S-03 |

**Prerequisites:** develop at 76cd680 (S-01, S-06 merged); `npm install` in the worktree.
**Estimated effort:** ~7 implementer sessions, one commit per phase.

## Open Risks & Assumptions

- The MR notes response shape (`author.username`, `system`, `created_at`) comes from GitLab's documented API, not a live call; Phase 1 has a manual check by the user.
- MRs move `updated_at` on pushes and pipelines, so hidden MRs cost more notes calls than issues (each call advances the baseline).
- Without `gitlab.user`, the user's own MR comments bring an MR back (same as issues; the notice covers it).
- A merged hidden MR is not shown as "Zmienione"; it disappears and its record is pruned after 30 days (O1). O1, "labels never bring an MR back" and "draft/ready ignored" are defaults chosen by agent; the orchestrator surfaces them to the user for confirmation at the end of the slice.
- Shared with S-03: `AGENTS.md` app-data line, PRD paragraph area, `lib.rs` handler list, `new_assignments/mod.rs` (two small additions), `cli/mod.rs` counters if S-03 adds a call.

## Success Criteria (Summary)

- The user hides any MR from either section and it stays hidden through approvals, pushes, pipelines and their own comments.
- A comment by someone else or a reopen brings it back marked "Zmienione"; "OK, zauważone" clears it.
- GitLab issue hiding and every existing test behave as before.
