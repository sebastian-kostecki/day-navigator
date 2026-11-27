---
change_id: gitlab-offline-last-list
title: Last GitLab list without VPN
status: planned
created: 2026-11-27
updated: 2026-11-27
archived_at: null
---

## Notes

Roadmap M-4, S-04 "Last GitLab list without VPN", anchor MS-32 (GitHub issue #32).

Without VPN, "Praca (GitLab)" shows the calm "GitLab niedostępny" card (last success time and next try, no red, no alarm) and under it the session's last list, dimmed, as "Projekty (GitHub)" already does. The view header shows that list's counts instead of only "źródło niedostępne". Today the condition `!offline` in `src/gitlab/GitlabView.tsx` (from commit 0d187bc) hides it.

User decision (2026-11-27): fix it (show the dimmed last list), not `wontfix`; hiding the list was not a deliberate decision. No new FR in the PRD. Anything else: closest to GitHub's existing stale-list behaviour. UI-only, no store migration (parallel slices `source-refresh-skeleton` and `gitlab-view-memory` touch the refresh modules, `GitlabListViewProvider.tsx`, `GitlabView.tsx` and the store).

(Notes written by agent from the orchestrator brief; the skill was run with an empty intent: default chosen by agent.)
