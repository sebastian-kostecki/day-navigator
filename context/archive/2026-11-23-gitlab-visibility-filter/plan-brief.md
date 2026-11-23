# GitLab Visibility Filter — Plan Brief

> Full plan: `context/changes/gitlab-visibility-filter/plan.md`

## What & Why

The user can say in `settings.json` which assigned GitLab items the "Praca (GitLab)" list shows: by project, labels and milestone, plus an opt-in section of recently closed items (roadmap S-02, PRD FR-002). The list gets less noisy, and nothing urgent is lost: notifications, hidden-issue tracking and "To do" ignore the filter.

## Starting Point

The round fetches open assigned MRs and issues, partitions the issues into visible and hidden, and publishes everything. "To do" reconciliation reads the published list, and treats a missing item as closed. Settings are hand-edited JSON; each block is loaded on its own and re-read every round.

## Desired End State

With a `gitlabFilter` block, the view shows only matching items. Each section has an "Ukryte filtrem: N" line, so an all-filtered list never reads as "nothing assigned". With `closedDays`, a "Zamknięte" section lists items closed or merged in the last N days. A typo in the block shows everything, plus a notice. Filtered-out items still notify, are still tracked when hidden, and still tick themselves off in "To do" only when they really close.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| State dimension | Opt-in "recently closed" section, `closedDays` 1–30, off by default | Meets FR-002's open/closed dimension. The bounded window keeps the fetch small and leaves the open-list contracts untouched. |
| Rule semantics | Include + exclude list per dimension; ANY within a list, AND across dimensions | Covers both "only these projects" and "everything except label blocked". |
| MRs | Filter applies to MRs and issues alike | One mental model, and the "To do" picker narrows the same way. Notifications still fire. |
| Filtered-out items | Count line per section, no rows | Honours "never an empty list in place of a failure" without cluttering the view. |
| Project matching | Exact path or group prefix on a `/` boundary, case-insensitive | One line covers a group; `acme/backend` never matches `acme/backend-old`. |
| Invalid block | Full list plus a notice; no tray alarm | Fails open like `gitlabRefresh`: a typo never hides work. |
| Where it lives | Separate top-level `gitlabFilter` block, pure Rust rule in `gitlab/filter.rs` | A filter typo never breaks the `gitlab` block, and domain rules stay in Rust (AGENTS.md). |
| What "closed" means | Issue `closed`; MR `closed` or `merged` (MRs fetched with `state=all`) | Merged MRs are the common way work ends; one call per kind. |
| Picking into "To do" | Only shown (filtered) items | The picker mirrors the list; existing entries keep reconciling on the full list. |

## Scope

**In scope:**
- `gitlabFilter` loader (fail-open) and the filter rule
- Milestone parsing
- Filtering in the round and in hide/unhide/dismiss
- "To do" reconciliation moved to the full list
- Two read-only closed-items CLI calls with guard tests
- View counts, notice and closed section

**Out of scope:**
- A settings screen
- A GitHub filter
- Filtering notifications, hidden tracking or reconciliation
- Rows for filtered-out items
- Closed history beyond 30 days
- Actions on closed items
- Tray changes
- Regex or wildcards

## Architecture / Approach

The round loads the filter, fetches and partitions as today, then applies the filter to the partitioned list just before `next_snapshot`. The unfiltered list stays the input for `kept_full`, "To do" reconciliation and notifications. A new `kept_filter` slot, written under the hidden-issue gate, lets `act_on_issue` re-filter. The closed fetch runs only after a successful open fetch. It goes into its own `closed` snapshot field, never into `AssignedItems`, and its failure shows only in that section.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Filter rule and settings block | `GitlabFilter`, matching, milestone parsing, fail-open loader | A loader error quoting a work value |
| 2. Apply the filter in the round | Filtered snapshot with counts; reconciliation on the full list; re-filter on hide | A filtered-out item ticking itself off in "To do" |
| 3. Recently closed fetch | Two guarded read-only calls, closed parser, isolated closed section | Loosening the read-only guard; GitLab rejecting a date-only `updated_after` |
| 4. UI | TS mirrors, counts, invalid notice, "Zamknięte" section | An all-filtered section looking like "nothing assigned" |

**Prerequisites:** S-01, S-03 and S-07 are done (list, hidden issues, "To do" reconciliation); `glab` is logged in for the manual checks.
**Estimated effort:** ~3–4 sessions across 4 phases.

## Open Risks & Assumptions

- GitLab is assumed to accept a date-only `updated_after=YYYY-MM-DD`; manual step 3.6 confirms it. If it does not, the guard needs a reviewed extension for a full timestamp.
- Filter values are work data. They live only in `settings.json`, never in logs, errors or tests.
- A hidden issue that comes back as "changed" but is filtered out stays off screen; its notifications and "To do" tracking are unaffected.

## Success Criteria (Summary)

- The user narrows the work list in `settings.json` and sees exactly the matching items, with counts of what the filter removed.
- No new assignment, "To do" auto-tick or hidden-issue return is lost because of the filter.
- A typo in the filter never hides work or raises an alarm.
