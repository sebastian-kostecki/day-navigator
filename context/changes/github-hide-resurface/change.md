---
change_id: github-hide-resurface
title: Hide and resurface GitHub issues in "Projekty (GitHub)"
status: plan_reviewed
created: 2026-11-27
updated: 2026-11-27
archived_at: null
---

## Notes

Slice S-01 `github-hide-resurface`, GitHub issue #31, anchor MS-31 (PRD refs FR-003, FR-005). Milestone M-4 north star.

Outcome: in "Projekty (GitHub)" the user can hide and show a GitHub issue from its row, sees a "Ukryte (n)" section with the hide date, and a hidden issue comes back marked "Zmienione" (as in GitLab) when its state changes or someone other than the user comments. Snapshots live in the app data store; the data comes through new read-only `gh` calls.

User's decisions:
- The GitHub login (to tell "someone other than me" comments) comes from a new read-only `gh api user` call — a new `CliCall` variant with fixed arguments, covered by the guard test — not from a setting.
- Only a state change or a comment by someone else brings an issue back (the FR-003 rule, same as GitLab). Nothing else (labels, edits, reactions) does.
- Mirror the GitLab behaviour and look ("Ukryte (n)", hide date, "Zmienione" flag, show action) so both sources behave alike.

Team questions for research (the plan settles them):
- Can the rules in `src-tauri/src/hidden_issues/rules.rs` be shared between GitLab and GitHub without a copy?
- A storage shape that can also hold GitLab merge requests later (source/kind in the key), so slice S-07 (hiding GitLab MRs, later wave) needs no migration of its own. Not implemented here; just do not close that door.
- Which `gh` calls (fixed args, read-only) give the issue state and comments with author logins and timestamps; how today's GitHub refresh fetches issues; how the GitLab hidden-issue flow (store, commands, UI, rules, snapshots) works end-to-end.

This slice owns migration v15 (the only migration in the wave). The app-data list in AGENTS.md must be updated in the same change to cover GitHub hidden snapshots.

Parallel slices in the wave: S-02 `github-refresh-interval`, S-06 `gitlab-review-requests`.
