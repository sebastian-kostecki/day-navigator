# Hide and Resurface GitLab Issues — Plan Brief

> Full plan: `context/changes/hide-and-resurface/plan.md`

## What & Why

Roadmap S-03 (PRD FR-003, US-02): the user can hide an issue in "Praca (GitLab)". The issue comes back by itself, marked as changed, when its status changes or someone other than the user comments on it (bots count). Hidden issues should stop cluttering the work list without ever making a real change go unnoticed.

## Starting Point

The S-01 list is fetched by the `gitlab-refresh` thread every 5 minutes and held in memory. It keeps no issue id, state or comment data, lists only open issues, and has no row actions. The F-01 SQLite store has no table for hidden issues. Nothing reads comments, and the only identity is the optional `gitlab.user` login.

## Desired End State

- **Hiding:** issue rows have "Ukryj". Hidden issues sit in a collapsed "Ukryte (N)" section with "Pokaż", and survive restarts.
- **What resurfaces:** a successful round brings a hidden issue back with a reason ("zmiana statusu", "nowy komentarz", "wróciło na listę"):
  - its open/closed state or a scoped label (`key::value`) changed;
  - a non-system comment by anyone but `gitlab.user` appeared;
  - it dropped off the list and came back.
- **The mark** stays until the user presses "OK".
- **Failures:** a failed check keeps the issue hidden and says so. A store failure shows the full list plus a notice.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| What "status" means | Open/closed state + scoped labels (`key::value`) | Board moves via workflow labels are how status changes on GitLab, and no settings are needed. |
| Which comments count | Non-system notes by anyone except `gitlab.user`, bots included | Matches "bots count" while GitLab's own activity entries don't turn hiding into noise. |
| Hideable items | Issues only | FR-003 speaks of issues; MR review threads would resurface constantly. |
| Hidden issue drops off the list | Keep the record; resurface as "returned" when it reappears; prune after 30 days absent | A reopen is caught without extra calls per round. |
| "Changed" mark lifetime | Stored; stays until dismissed ("OK") or re-hidden | A change can't slip by over a restart or a missed day. |
| Seeing hidden issues | Collapsed "Ukryte (N)" with "Pokaż" | A mis-click is reversible. |
| Who "the user" is | `gitlab.user` from settings; no `glab api user` call | Only one new CLI call; without it every new comment resurfaces and the view says so. |
| Comment check fails | Issue stays hidden, re-checked next round, view shows the failed count | Surfaces the failure without blanking or flooding the list. |
| Item key | GitLab global issue `id` + host | `reference` changes when a project moves. |
| Comment baseline | The issue's `updated_at` as last seen; advanced on own-only checks | Server time, no local clock skew; a comment landing mid-round is still caught next round. |

## Scope

**In scope:**
- Parsing `id`, `iid`, `project_id` and `state`.
- One new guarded read-only call (`glab api projects/<id>/issues/<iid>/notes`) with fake-cli scenarios.
- Migration v3 `hidden_issues`, pure rules, store ops, crash test.
- Reconciliation in the refresh round.
- Hide, unhide and dismiss commands.
- UI row actions, mark, hidden section and notices.

**Out of scope:**
- Hiding MRs; system notes as comments; non-scoped label or other field changes.
- `glab api user`; fetching dropped-off issues individually.
- The S-02 filter (applies after this partition when it lands); S-04 notifications.
- Storing titles, references or comment text; any write to GitLab.

## Architecture / Approach

A new Rust module `hidden_issues` holds the pure rules, the store ops (one `write` each) and a `tauri`-free `reconcile_round`. After a successful fetch, the refresh thread:
1. reads the hide records;
2. checks comments only for hidden issues whose `updated_at` moved, with no lock held;
3. applies every transition in one store write, re-validated against each row;
4. partitions the full list into visible (with marks) and hidden;
5. publishes `gitlab-changed`.

The commands change one row and re-partition the in-memory list. A shared mutex keeps a late round from overwriting a newer command result. The UI only renders and sends ids.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Issue identity + notes call | Stable ids/state; guarded `GlabIssueNotes` call; value-free notes parser; fake-cli scenarios | Widening the read-only guard to a path-segment endpoint without opening anything else |
| 2. Store + rules | Migration v3; unit-tested resurfacing rules; ops; crash test | Getting timestamp, author and 30-day boundaries exactly right |
| 3. Round wiring + commands | Reconciliation each round; snapshot with hidden/marks/failures; 3 commands; TS mirrors | Race between a round and a user action (handled by re-validation + gate) |
| 4. UI | Ukryj, mark + OK, "Ukryte (N)" + Pokaż, notices; manual check on real GitLab | Real GitLab label and comment behaviour differing from fake data |

**Prerequisites:** S-01 and F-01 done (yes). `gitlab.user` set in settings.json for own-comment filtering.
**Estimated effort:** ~3–4 sessions across 4 phases.

## Open Risks & Assumptions

- **Scoped labels:** assumes the company GitLab expresses status through scoped labels or open/closed. A board using plain labels for status would not resurface on moves.
- **Unrelated scoped labels:** a scoped label unrelated to workflow (e.g. `priority::high`) also resurfaces an issue. Accepted.
- **Conflict with S-07:** `todo-list` (S-07) is starting in parallel and may also extend `GitlabItem`. This plan adds `id`, which S-07 can reuse; expect a merge in `gitlab/mod.rs` and `types.ts`.
- **Hiding from a stale list:** with VPN off, the hide uses the last fetched data. The older baseline can resurface the issue earlier than needed. This is conservative and accepted.

## Success Criteria (Summary)

- An unchanged hidden issue never appears in "Zgłoszenia"; a status, scoped-label or other-person comment change brings it back marked, every time.
- Own comments (with `gitlab.user` set) and GitLab system notes never resurface an issue.
- No failure (VPN off, failed check, store error) shows an empty list or silently changes what is hidden; no work data reaches logs or tests.
