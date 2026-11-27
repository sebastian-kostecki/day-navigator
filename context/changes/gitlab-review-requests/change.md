---
change_id: gitlab-review-requests
title: GitLab merge requests awaiting my review in "Praca (GitLab)"
status: planned
created: 2026-11-27
updated: 2026-11-27
archived_at: null
---

## Notes

Slice S-06 (roadmap anchor MS-35, GitHub issue #35; PRD refs FR-001, FR-002, FR-004). In "Praca (GitLab)" the user sees the merge requests where they are a reviewer in a "Do review" section, under the same settings filter as the rest (FR-002); each row has "+" to "Do zrobienia"; a new review request gives a Windows notice by the FR-004 mechanism and threshold, whatever the filter says.

User decisions (binding): yes to a "Do review" section (lifts the PRD Non-Goal "reviewer MRs outside v1"; one-line PRD note, no new or changed FR numbers); notices like FR-004 new-assignment notices (same mechanism and threshold, bypass the view filter); FR-002 settings filter applies; rows get "+" to "Do zrobienia"; hiding review MRs is NOT in this slice (comes with S-07); no migration — reuse `seen_assignments` keys with kind `mergeRequest`; an MR both assigned and awaiting review must not give two notices or appear twice confusingly. Data via a new read-only `glab api` `CliCall` variant with fixed arguments, covered by the guard test; without VPN behaves like the rest of the GitLab source (calm unavailable, last list dimmed). Tests use `fake-cli` and invented data only.
