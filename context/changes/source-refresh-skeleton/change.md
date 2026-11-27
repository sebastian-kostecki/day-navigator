---
change_id: source-refresh-skeleton
title: Shared source refresh skeleton for GitLab and GitHub
status: plan_reviewed
created: 2026-11-27
updated: 2026-11-27
archived_at: null
---

## Notes

Roadmap M-4, F-01 "Shared source refresh skeleton", anchor MS-28 (GitHub issue #28). GitLab and GitHub refresh through one shared skeleton (thread with `ThreadDown` stand-in, interval, wake after sleep, "Check now", snapshot publication, `lastSuccessMs`, `nextRefreshAtMs`); the source-specific fetch and rules stay in each source module (`src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/github_refresh.rs`). No change in behaviour, Tauri commands or TS types; existing test bodies unchanged (tests may only move/import). The skeleton must make it easy for S-01 (GitHub hiding), S-02 (GitHub interval + `nextRefreshAtMs`), S-03 (GitHub NEW flag/notices) and S-06 (second GitLab MR fetch) to build on it, without implementing them. No store migration; minimal `lib.rs` edits (parallel slices S-04/S-05 touch `src/gitlab/`, `store/schema.rs`, `lib.rs`).

Scope after planning (plan "Hand-off to later slices"): F-01 delivers for GitHub only a round that returns its `Duration` plus the shared `next_refresh_at_ms` helper; S-02 adds the `GithubSnapshot` fields (`refreshMinutes`, `nextRefreshAtMs`, `refreshSettingInvalid`), their carry-over and the TS type itself. `lastSuccessMs` stays in the rules `next_snapshot`; wake after sleep and "Check now" stay in `runtime.rs` through each state's `wake()`.
