# Last GitLab list without VPN — Plan Brief

> Full plan: `context/changes/gitlab-offline-last-list/plan.md`
> Research: `context/changes/gitlab-offline-last-list/research.md`

## What & Why

Without VPN, "Praca (GitLab)" shows the calm "GitLab niedostępny" card but hides the session's last list, and the header says only "źródło niedostępne". "Projekty (GitHub)" shows its last list dimmed under the same kind of card. The user chose (2026-11-27) to fix this: the hidden list was a side effect of commit 0d187bc, not a decision. Roadmap M-4 S-04, MS-32.

## Starting Point

Rust already keeps the last list on a failed round (`src-tauri/src/gitlab/mod.rs:593-622`), and the dimming CSS exists (`GitlabView.css:9-11`). One UI condition (`&& !offline`, `src/gitlab/GitlabView.tsx:697`) hides the list, and the header checks `offline` before the counts (`GitlabView.tsx:604-613`).

## Desired End State

After one successful round, losing VPN shows the calm card with the last list dimmed under it, and the header shows `N widoczne · M ukryte`. With no VPN at app start, the card shows alone with "źródło niedostępne", as today.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Fix or wontfix | Fix: show the dimmed list | Hiding was not deliberate; every earlier plan asked for the dimmed list | User (2026-11-27) |
| Scope | UI-only, `GitlabView.tsx` only, no migration | Rust already keeps the list; parallel S-05 owns migration v14 | Research |
| Header text offline with a list | Counts only, no "nieaktualne" suffix | Same as GitHub; card and dimming already say it is stale | Research, default chosen by agent |
| "lista z HH:MM" banner | Not added | The card's last-success line carries that time | Research, default chosen by agent |
| Row actions on the dimmed list | All stay enabled | Same as GitHub stale rows and the GitLab error state; backend supports it offline | Research, default chosen by agent |
| Filter bar, "Zamknięte", footnote | Come back dimmed with `Lists` | Same as the error state today | Research, default chosen by agent |
| No VPN at app start | Unchanged: card alone, "źródło niedostępne" | The session has no last list yet | Research, default chosen by agent |
| Empty kept list offline | Normal empty message, dimmed, under the card | Same as the error state; the card is shown, so the empty list never replaces the failed source | Plan, default chosen by agent |

## Scope

**In scope:**
- Reorder the header summary so counts show first when items exist (as `GithubView.tsx:172-179`).
- Drop `&& !offline` from the `<Lists>` condition.

**Out of scope:**
- Rust, store, types, CSS, migrations; any new header suffix or banner; action gating; design-system README wording; PRD or roadmap edits.

## Architecture / Approach

The snapshot from Rust already carries the kept `items` in the `unavailable` state. The view only changes what it renders: `offline` still drives the calm card, `Lists` renders whenever `items` exist and the source is configured, and `stale={state !== "ok"}` dims it.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Show the kept list offline | Dimmed last list under the card, counts in the header | Merge overlap with S-05 in `GitlabView.tsx` (two small hunks) |

**Prerequisites:** none; parallel slices F-01 and S-05 are independent.
**Estimated effort:** one short session, one phase.

## Open Risks & Assumptions

- Hidden rows on a stale list are dimmed twice (about 0.33 opacity); only a manual look can settle readability. The error state already does this.
- F-01 must keep "failed round keeps items"; existing Rust tests cover it.

## Success Criteria (Summary)

- Without VPN after a successful round: calm card, dimmed last list, counts in the header.
- Start without VPN: card alone, "źródło niedostępne".
- All gates green; no file other than `src/gitlab/GitlabView.tsx` changes in the implementation phase.
