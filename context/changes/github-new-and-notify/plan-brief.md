# GitHub NEW flag since the last visit — Plan Brief

> Full plan: `context/changes/github-new-and-notify/plan.md`
> Research: `context/changes/github-new-and-notify/research.md`

## What & Why

"Projekty (GitHub)" gets the blue **Nowe** flag GitLab already has: an issue is NEW when it appeared since the user's last visit to the GitHub view (S-03, issue #30, MS-30). GitHub gives no Windows notification; FR-004 notices stay GitLab-only (user decision). Planned non-interactively; agent choices are marked "default chosen by agent" in the plan.

## Starting Point

GitLab's NEW flag is a Rust rule on stored instants (`first_seen > seen-until mark`, nothing without a mark, silent seeding) with a mark saved on leaving the view (`useGitlabSeenMark`). The GitLab tables are GitLab-shaped; GitHub's refresh (`github_refresh.rs`) has no seen set; the store is at v15.

## Desired End State

Each successful GitHub round records the issues of the repos that loaded. The snapshot carries `newReferences`, and the view shows `Nowe` on those visible rows. Leaving the view (or closing the window while on it) saves the mark and clears the flags. The first-ever visit shows nothing NEW. No toast, no event-log entry, no new `gh` call.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Notifications | None for GitHub; NEW flag only | User decision | User |
| Migration | v16: `github_seen_repos`, `github_seen_issues`, `github_view_mark` | User decision (the wave's only migration); own tables avoid rebuilding the GitLab ones | User / Research |
| Module | New tauri-free `github_seen/`; imports `FORGET_AFTER_MS`, `baseline_first_seen` only | No edit in `new_assignments/` (shared with S-07) | Plan (agent default) |
| Repo key | Plain lowercase `owner/name`, as `hidden_items.scope` | No new kind of work data | Plan (agent default) |
| Where flags travel | `GithubSnapshot.new_references` (TS `newReferences`) | No edit in `hidden_issues/` or the 21 `GithubIssue` literals | Plan (agent default, orchestrator preference) |
| Seeding | Per repo, silent, stored at `min(mark, now)` | A repo added later never floods NEW under an older global mark | Plan (agent default) |
| Mark | One global row; value = `shown_until`, saved with `max()` | Same semantics as GitLab, never the clock | Research / Plan |
| First visit | No mark means nothing NEW | User decision ("seed like GitLab") | User / Plan |
| Prune | Rows and marker of repos no longer in settings dropped; 30-day forget inside loaded repos; failed repos untouched | Bounded table; re-adding seeds silently | Plan (agent default) |
| Hidden issues | Recorded, never flagged while hidden; flag returns on unhide if still fresh | "Pokaż ukryte" stays in-memory view state | Plan (agent default) |
| Visit end | Twin hook `useGithubSeenMark(view)` in `App.tsx`; view id only | No second timer subscription | Plan (agent default) |
| Store failure | Flags and `shown_until` unchanged; warn once by kind | Never marks unshown items as seen; no alarm | Plan (agent default) |
| PRD / roadmap | No edit | Roadmap names notes for S-06/S-07 only; roadmap is read-only in this wave | Plan (agent default) |

## Scope

**In scope:** migration v16; `github_seen` rules, store ops, error; snapshot field and TS mirror; refresh state (`kept_new`, `shown_until`, `last_mark`), round wiring, `act_on_issue` flags; `mark_github_seen` command; `useGithubSeenMark`; `Nowe` in `GithubView`; AGENTS.md app-data list.

**Out of scope:** toasts and event-log entries for GitHub; any change in `cli/`, `bin/fake_cli.rs`, `hidden_issues/`, `new_assignments/`, `source_refresh.rs`, `src/gitlab/`; "Pokaż ukryte" behaviour; PRD and roadmap.

## Architecture / Approach

`refresh_round` → `record_seen` (one `Store::write`, before the gate) → under `hidden_gate`: `accept_round` (drop keys the session's last mark covers, set `shown_until`) → partition → `settle_new` writes `next.new_references` via `github_seen::visible_new` → store and publish. `act_on_issue` re-applies the flags. `mark_github_seen` saves `shown_until` as the mark, clears the flags, publishes. The UI only renders `newReferences` and calls the command on the end of a visit.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Migration v16, rules and error | Tables, pure `plan_repo`, `GithubSeenError`, migration tests | Editing a shipped migration instead of appending |
| 2. Seen-set store operations | `record_round`, `mark_view_seen`, `round_input`; integration and crash tests | Seeding stored at `now` instead of `min(mark, now)` |
| 3. Snapshot contract | `new_references` + TS mirror, `visible_new` | Forgetting the keep branch of `next_snapshot` |
| 4. Refresh state and command | Round wiring, `act_on_issue` flags, `mark_github_seen` | Lock order and the record-before-gate sequence |
| 5. UI | `Nowe` flag, `useGithubSeenMark` in `App.tsx` | Flagging hidden rows |
| 6. AGENTS.md | One-line insertion in the app-data list | Textual conflict with S-07 on the same line |

**Prerequisites:** develop at 76cd680 (M-4 waves 1–2), research committed (430d6ef).
**Estimated effort:** ~6 small sessions, one commit per phase.

## Open Risks & Assumptions

- A repo failing in the round a visit ends: its earlier NEW flags are not shown that round, and the saved mark may cover them, so such an issue stays on the list unflagged (accepted; one global mark).
- If `main-hidden` is lost on close, no mark is saved and the flags show again (safe side, as GitLab). Quitting from the tray saves no mark.
- A change of `gh` login makes newly assigned issues NEW once (the key is the issue, not the login).
- Merge with S-07: AGENTS.md app-data line (keep both insertions) and two inserted lines in `lib.rs`.

## Success Criteria (Summary)

- The first visit to "GitHub" shows no `Nowe`; a later newly assigned issue shows `Nowe` until the user leaves the view.
- Hidden issues never show the flag; no Windows notification appears for GitHub.
- All gates green, including the v16 upgrade, integration and crash tests.
