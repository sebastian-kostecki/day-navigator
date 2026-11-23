# New Assignment Notifications — Plan Brief

> Full plan: `context/changes/new-assignment-notify/plan.md`

## What & Why

Roadmap S-04 (PRD FR-004, NFR "Świeżość", NFR "Lekkość w tle"): the user gets a Windows notification for every new GitLab issue or MR assigned to them, whatever the list shows. It should arrive within 15 minutes while on VPN, so a new assignment is seen the same day without checking by hand. More than 3 at once arrive as one combined notification, so a reconnected VPN never floods the screen.

## Starting Point

The `gitlab-refresh` thread fetches the full assigned list every fixed 5 min (and on start, wake, resume or button), but keeps it in memory only and never compares rounds. `runtime::toast` already shows health toasts through the notification plugin. The store is at migration v6 and holds no record of which items were seen.

## Desired End State

- **Toasts:** after each successful round, items never seen for this account notify:
  - 1–3: one toast each, `Nawigator dnia — nowe zgłoszenie` / `nowy MR` with `<reference> — <title>`;
  - more than 3: one `nowe przypisania (N)` toast with 3 lines and `i N−3 więcej`.
- **Catch-up and silence:** items assigned while the app was closed notify on the first round after start. A fresh install or a changed account seeds silently. Restarts, VPN reconnects and hiding never re-notify.
- **Interval:** the refresh interval is `gitlabRefresh.minutes` in `settings.json` (default 10, allowed 1–14). An invalid value keeps 10 min and shows a notice in "Praca (GitLab)".

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Items assigned while the app was closed | Stored seen set per account (migration v7); first round after start notifies unseen items | "Every new assignment" must include overnight ones, and a restart must not repeat. |
| First round for an account | Silent seed, via a per-account marker that is never pruned | No burst on install, upgrade or a changed `gitlab.user`; a marker (not "no rows") keeps a long-empty list from swallowing its next item. |
| What counts as "new" when an id returns | Never seen for this account; forgotten after 30 days absent from successful rounds | Reopens and short unassign/reassign blips stay quiet; same 30-day window as S-03. |
| Item key | (opaque `account_key`, kind, GitLab global id) | Issue and MR ids are separate sequences and can collide; no hostname is stored. |
| Diff input | The full unpartitioned list of a `GitlabState::Ok` round | Hiding, un-hiding or S-03 resurfacing must not look new, and failed rounds keep stale items. |
| Toast content | Reference + title (title cut to 100 chars, 40 in a combined line); logs carry counts only | The toast should say whether to act; it stays on this machine, while logs keep the work-data guardrail. |
| More than 3 new | One combined toast, 3 lines + "i N więcej" | PRD FR-004, matching the existing health toast pattern. |
| Store write fails | Session fallback: compare with the last successful round's keys; on recovery suppress only keys the fallback toasted | Notifications keep working without repeating every round, no duplicate after the store recovers, and nothing seen silently during the failure is lost. |
| Refresh interval | Own `gitlabRefresh` block, integer 1–14, default 10; invalid → 10 min + view notice | The wait starts after a round ends, so 14 (plus round time) stays within the ≤ 15 min NFR, and a typo never stops the list or notifications. |
| "New" mark in the list | None; toast only | Matches the roadmap outcome; keeps the slice small. |

## Scope

**In scope:**
- Migration v7 (`assignment_accounts`, `seen_assignments`), pure rules, one write per round, crash test.
- Toast text, wiring into the refresh round, session fallback.
- `gitlabRefresh.minutes` loader, configurable loop wait, `refreshSettingInvalid` snapshot field, its TS mirror and view notice.
- AGENTS.md app-data list update.

**Out of scope:**
- A "Nowe" mark in the list; toast click or actions.
- Notifying resurfaced issues or reviewer MRs.
- S-02 filter work; any new `glab` call; telling reopen from reassignment.
- Focus Assist handling; a settings UI; storing titles, references or hosts.

## Architecture / Approach

A new `tauri`-free module `new_assignments` holds the rules, the toast text, the session fallback and `record_round`, which runs as one store write per round. After a successful round, `refresh_round` releases the gate and publishes the list, then:
1. records the full list's keys for its account;
2. filters the new keys through the session fallback;
3. maps them to items;
4. shows `notices_for(...)` through the gated `runtime::toast`.

The interval is read at the start of each round and returned to the loop's `recv_timeout`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Seen-set store and rules | Migration v7, pure new/seed/forget rules, `record_round`, unit, integration and crash tests | Getting the seed marker and the prune order right (prune after upsert, this account only) |
| 2. Toasts in the refresh round | 1–3 or combined toasts on Ok rounds from the full list, session fallback, count-only logs; manual check on the real GitLab | Toasting from a stale or partitioned list, or while holding a lock |
| 3. Refresh-interval setting | `gitlabRefresh.minutes` 1–14 (default 10), loop wait from settings, invalid-setting notice in the view | A settings typo leaking into the GitLab block or stopping the refresh |

**Prerequisites:** S-01 and F-01 done (yes); VPN access to the company GitLab for the manual checks.
**Estimated effort:** ~2–3 sessions across 3 phases.

## Open Risks & Assumptions

- **Silent reassignment:** being reassigned an item you held within the last 30 days gives no toast, because the list cannot tell it from a reopen. Accepted.
- **Missed toasts:** Focus Assist or a dev build ("Windows PowerShell" sender) can hide toasts. There is no in-app trace of a missed toast, because no list mark was chosen.
- **Freshness:** the wait starts after a round ends, so the worst case is the interval plus round time. The maximum of 14 min keeps this within 15 min; the default of 10 min keeps a clear margin.
- **Changed account:** changing `gitlab.user` (or the host) seeds the new account silently, so an item assigned in that same round does not notify.

## Success Criteria (Summary)

- A newly assigned issue or MR shows a toast with its reference and title within one refresh interval while on VPN.
- Restarting, reconnecting the VPN or hiding issues never repeats a toast; more than 3 new give one combined toast.
- The refresh interval follows `gitlabRefresh.minutes`, and an invalid value is visible in the view instead of silently breaking anything.
