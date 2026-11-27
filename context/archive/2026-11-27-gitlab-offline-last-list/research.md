---
date: 2026-11-27T00:00:00+01:00
researcher: Claude (10x-research, non-interactive agent run)
git_commit: 2b6ac0861b564e2e096afc50f39e343f1fa41d11
branch: slice/gitlab-offline-last-list
repository: day-navigator
topic: "Last GitLab list without VPN (M-4 S-04, MS-32, issue #32)"
tags: [research, gitlab, github, stale-list, offline, ui]
status: complete
last_updated: 2026-11-27
last_updated_by: Claude (10x-research, non-interactive agent run)
---

# Research: Last GitLab list without VPN

**Date**: 2026-11-27 (time not recorded; Europe/Warsaw)
**Researcher**: Claude (10x-research, non-interactive agent run)
**Git Commit**: 2b6ac0861b564e2e096afc50f39e343f1fa41d11
**Branch**: slice/gitlab-offline-last-list
**Repository**: day-navigator

## Research Question

Without VPN, "Praca (GitLab)" should show the calm "GitLab niedostępny" card and under it the session's last list, dimmed, as "Projekty (GitHub)" does; the view header should show that list's counts instead of only "źródło niedostępne" (roadmap M-4 S-04, MS-32, GitHub issue #32). How does GitHub's stale list work, how does `GitlabView` choose offline / error / data, where does the last GitLab list live during the session and does it survive an offline round, what did commit 0d187bc do and why, what did the archived plans say, how do "Pokaż ukryte" and "+" behave on a stale list, and can the fix be UI-only with no store migration?

## Summary

- **The fix is UI-only.** It needs two edits in `src/gitlab/GitlabView.tsx` and nothing in Rust, the store, the TS types or the CSS.
- **The backend already keeps the last list when GitLab is offline.** `next_snapshot` copies `items`, `closed`, the filter state and `last_success_ms` from the previous snapshot when a round fails with Unavailable or Error, as long as the GitLab host and user are unchanged (`src-tauri/src/gitlab/mod.rs:593-622`). Rust tests cover this, for example `snapshot_unavailable_keeps_items` (`gitlab/mod.rs:1606`).
- **The list is hidden by a single UI condition.** `!offline` at `src/gitlab/GitlabView.tsx:697` skips `<Lists>` when `state.state === "unavailable"`. Commit 0d187bc added it, and no later commit touched it (`git log -S'!offline'`).
- **The header shows no counts offline** because the summary checks `offline` before it checks for items (`GitlabView.tsx:604-613`).
- **The dimming code is still there.** `Lists` sets `is-stale` when `stale={state.state !== "ok"}` (`GitlabView.tsx:483,700`), and `.gitlab-lists.is-stale { opacity: 0.55; }` is in `src/gitlab/GitlabView.css:9-11`. GitHub uses the same opacity (`src/github/GithubView.css:3-5`).
- **The row actions already work offline.** Hide, unhide and dismiss act on the in-memory kept list plus the store and publish a new snapshot. "+" (add to "Do zrobienia") looks the item up in the current snapshot. Neither calls GitLab.
- **Hiding the list was a side effect, not a decision.** Every plan that describes the offline view asks for a dimmed last list: `gitlab-assigned-list`, `konsola-design-system`, `gitlab-visibility-filter`, `gitlab-new-and-hidden-date` and `github-assigned-issues`. The 0d187bc commit message gives no reason for hiding it. This matches the user's 2026-11-27 decision to fix it.

## Detailed Findings

### How GitHub's stale list works (the target behaviour)

- **Where the list lives.** The last list is `snapshot.repos` from the `useGithub` hook (`src/github/useGithub.ts:13`). It is kept in Rust: `github::next_snapshot` keeps `repos` and `last_success_ms` on Unavailable and on error rounds while the repo config is unchanged (`src-tauri/src/github/mod.rs:307-325`). A test covers it, `unavailable_keeps_the_list_while_the_repos_are_unchanged`.
- **Visibility.** `showList = state.state !== "notConfigured" && repos.length > 0` (`src/github/GithubView.tsx:168`). The state is not checked, so Unavailable and Error both keep the list.
- **Header.** The summary shows counts first whenever `showList` is true. It falls back to "źródło niedostępne" or "błąd źródła" only when there is no list (`GithubView.tsx:172-179`).
- **Dimming.** The wrapper gets `github-stale` when `state.state !== "ok"` (`GithubView.tsx:230`), and `.github-stale { opacity: 0.55; }` is in `src/github/GithubView.css:3-5`. The opacity is a literal, not a token. That is allowed, because AGENTS.md forbids only literal colours outside `tokens.css`.
- **Card above the list.** The "GitHub niedostępny" `SourceState kind="calm"` with `lastOk` and "Sprawdź teraz" renders above the list (`GithubView.tsx:209-218`).
- **Actions on stale rows.** No GitHub row action is disabled while the list is stale. "+" and "Otwórz na GitHubie" stay enabled (`GithubView.tsx:48-68`).

### How GitlabView decides offline vs error vs data

The view picks what to render in this order:

1. `loadError` shows `ErrorState` alone (`GitlabView.tsx:570-581`).
2. No snapshot, or `starting` without items, shows the loading text (`GitlabView.tsx:583-590`).
3. `offline = state.state === "unavailable"` (`GitlabView.tsx:602`).
4. The header summary (`GitlabView.tsx:604-613`):
   - if `offline`, "źródło niedostępne";
   - else if there are items and the state is not `notConfigured`, the counts `N widoczne · M ukryte`;
   - else if `error`, "błąd źródła".

   Because `offline` comes first, the kept list's counts never show offline. In the `error` state with items, the counts already show.
5. The calm "GitLab niedostępny" card, with `lastOk`, "Sprawdź teraz" and `retryText` (the interval and the next try), renders when `offline` (`GitlabView.tsx:644-657`).
6. `ErrorState` "Błąd GitLaba" renders when the state is `error` (`GitlabView.tsx:658-665`).
7. `<Lists>` renders when `items && state.state !== "notConfigured" && !offline` (`GitlabView.tsx:697-704`), with `stale={state.state !== "ok"}`.

   So today the dimmed list does show in the `error` state, under `ErrorState`, but never when unavailable. `unavailable` is the only state where kept items exist and are hidden.

### Where the last GitLab list lives and whether it survives an offline round

- **In memory, in Rust.** The list is held in `GitlabRefreshState` (`src-tauri/src/gitlab_refresh.rs:66-91`):
  - `snapshot: RwLock<GitlabSnapshot>` is the snapshot itself;
  - `kept_full` is the unpartitioned last list ("Never serialized");
  - `kept_config`, `kept_filter` and `kept_new` sit alongside it.
- **Not persisted.** A grep of `src-tauri/src/store` for `GitlabSnapshot` and `last_success` finds nothing. Nothing in this inspected path loads a list at startup. The store holds only hidden-issue records, seen keys, the seen-until mark and todo rows.
- **A failed round keeps it.** In `next_snapshot`, `keeps = same_config && !matches!(NotConfigured | SettingsInvalid)` (`gitlab/mod.rs:593-598`). When `keeps` is true, `items`, `last_success_ms`, `hidden_check_failed`, `own_comments_known`, `hidden_error`, `filter_active`, `filter`, `filtered_out`, `closed` and `account` are all copied from `prev` (`gitlab/mod.rs:609-621`). After that, `reshape_kept` re-applies the current filter and hidden records to `kept_full` (`gitlab_refresh.rs:306-337`).
- **Cases that clear the list:**
  - the state becomes `NotConfigured` or `SettingsInvalid`;
  - the GitLab host or user changed;
  - the first round of the session fails, so there is nothing to keep. `GitlabSnapshot::starting()` has `items: None` (`gitlab/mod.rs:521-540`).
- **Consequence for the UI.** With no VPN at app start, `items` is null and the card shows alone, with no list and the header "źródło niedostępne". That is correct, because the session has no last list yet. The fix must keep this case.
- **Front end.** `useGitlab` keeps the newest snapshot by `seq` (`src/gitlab/useGitlab.ts:37-38`). The hook has no list cache of its own, and it does not need one.

### Commit 0d187bc and why the list is hidden

- **The diff.** In `0d187bc^`, the list rendered as `{items && state.state !== "notConfigured" && (<Lists … stale={state.state !== "ok"} …/>)}` under the card "GitLab niedostępny (brak VPN lub sieci)". The CSS was `.gitlab-lists.is-stale { opacity: 0.55; }`, so the list showed dimmed. 0d187bc ("feat(konsola-design-system): align views with the Konsola mockups", 2026-11-24) made three changes:
  - added `const offline` and `&& !offline`;
  - made the header summary "źródło niedostępne" when offline;
  - added `retryText` to the card.
- **The reason.** The commit message says only "calm offline card with 'Sprawdź teraz'". The one written source that could support hiding the list is in the design-system artifact docs:
  - `context/archive/2026-11-24-design-system-artifact-sync/artifact/project/components/Notice/README.md:10` and `README.md:55` say that `SourceState`/`ErrorState` replace the whole source list.
  - In the same artifact, `components/Notice/preview.html:10` shows "GitLab niedostępny — brak VPN. Pokazuję ostatnie dane." That says the last data is shown.
  - So the docs do not agree with each other, and none of them is an FR or a plan decision.
- **Later history.** `git log -S'!offline' -- src/gitlab/GitlabView.tsx` finds only 0d187bc. `context/archive/2026-11-24-gitlab-filter-bar-retry/plan.md:12,34,144` describes the hiding as the state of the code at that time ("lista … jest ukryta (`GitlabView.tsx:556`)"), and its "not doing" line says the filter bar does not show offline. That plan records the code as it found it. It did not require the list to be hidden.

### What earlier plans said (verdict per claim)

| Source | Claim | Verdict against current code |
|---|---|---|
| `context/archive/2026-11-23-gitlab-assigned-list/plan-brief.md:15,23,70` | Without VPN, a calm notice plus the last list from this session (memory only), greyed, with "lista z HH:MM" | Data part supported: Rust keeps the list in memory. Rendering contradicted: hidden since 0d187bc. The "lista z HH:MM" text exists nowhere in `src/`, and the card's `lastOk` now gives that time. |
| `…/gitlab-assigned-list/plan.md:25,260-261` | `unavailable`: calm notice, kept list greyed below; `error`: `ErrorState` with kept list greyed below | Supported for `error` (`GitlabView.tsx:658,697`). Contradicted for `unavailable` (`GitlabView.tsx:697`). |
| `context/archive/2026-11-23-konsola-design-system/plan.md:283,415,501` | Without VPN: calm grey block with last-success time, no red; stale list dimmed | Calm card supported. Dimmed list contradicted. This plan is the parent of the 0d187bc follow-up, so the follow-up went against its own plan. |
| `context/archive/2026-11-23-gitlab-visibility-filter/plan.md:185,382` | Offline, the open and closed sections stay with the stale look | Data part supported: `closed` is kept (`gitlab/mod.rs:620`). Rendering contradicted. |
| `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan.md:270,354` | Without VPN (card with the kept list), NOWE flags stay | Assumes the list is visible. Contradicted by the UI. The data (`isNew` on kept items) is kept. |
| `context/archive/2026-11-23-github-assigned-issues/plan.md:27,51,302` | GitHub unavailable: calm notice, last list greyed, "as GitLab does" | Supported for GitHub (`GithubView.tsx:168,230`). |
| `context/foundation/prd.md:75,92,163,198` (FR-015, FR-024) | Unavailable GitLab is shown plainly as a calm state, never an empty list | Supported by the card. The PRD does not mention dimming or the last list, so the fix needs no FR. |

The archived `gitlab-assigned-list` plan says the list shows dimmed. Per the user decision, it stays as it is, and this change restores that behaviour.

### "Pokaż ukryte", view filter and "+" on the restored stale list

- **Toolbar.** The toolbar controls are client state from `GitlabListViewProvider` (`src/gitlab/useGitlabListView.ts`), computed over `snapshot.items` (`GitlabView.tsx:444-474`). They work on the kept list with no backend call:
  - "Pokaż ukryte (n)";
  - the type `Segmented`;
  - the Projekty, Etykiety and Kamienie milowe `MultiSelect`s;
  - the project and label toggles in each row.
- **Hide, unhide and dismiss.** The commands `hide_gitlab_issue`, `unhide_gitlab_issue` and `dismiss_gitlab_change` (`src-tauri/src/hidden_issues/commands.rs:25-80`) call `GitlabRefreshState::act_on_issue`. That function:
  - finds the issue in `kept_full`;
  - writes the hidden record to the store;
  - re-shapes `snap.items` and bumps `seq`;
  - publishes `gitlab-changed`.

  Offline, a hide therefore moves the row into `hiddenIssues` straight away, with no successful round needed. With no list at all, `ListUnavailable` ("Lista GitLaba nie jest jeszcze dostępna.", `hidden_issues/error.rs:84`) cannot occur from the UI, because the list is not rendered.
- **"+" add to "Do zrobienia".** `add_todo_gitlab_item` (`src-tauri/src/todo/commands.rs:135-155`) looks the item up in `snapshot.items` and uses `snapshot.account`. It never checks reachability, so it works on a kept list. Remove is store-only. Todo reconcile (ticking closed items) and new-assignment notifications are skipped when the state is not Ok (`gitlab_refresh.rs:~744-761`), so a stale list never ticks or notifies anything.
- **Same as GitHub and the GitLab error state.** GitHub's stale rows keep every action enabled. GitLab rows under `ErrorState` already keep every action enabled today. So the restored offline list needs no action gating.
- **Open in GitLab.** The title link (`openGitlabItem`) stays clickable. Offline the browser will fail to reach the host. That is acceptable, and GitHub's stale rows behave the same way.
- **What else comes back offline.** These are part of `Lists` and show again offline, dimmed with the rest:
  - the "Filtr z ustawień" bar (`GitlabFilterBar`, first child of `gitlab-lists`, `GitlabView.tsx:484`);
  - the hidden-issue footnote;
  - the "Zamknięte (ostatnie N dni)" panel.

  This is how the `error` state behaves today. It goes against the "not doing" line of the archived filter-bar-retry plan (`plan.md:34`), but that line described the old code. It was not a requirement.

### Can it be UI-only, and which shared files are touched

- **UI-only: yes.** No Rust, store, command or type change is needed. No store migration is added.
- **Files this change touches.** Only `src/gitlab/GitlabView.tsx`:
  - drop `&& !offline` at line 697;
  - reorder the summary at lines 604-613 so the counts branch runs first when `items` exist, matching `GithubView.tsx:172-179`. The `offline` constant stays in use for the card at line 644.
- **Files not touched.** `GitlabView.css` (`is-stale` already exists), `useGitlab.ts` and `GitlabListViewProvider.tsx`.
- **Overlap with parallel slices.** `gitlab-view-memory` (S-05) also edits `src/gitlab/GitlabView.tsx`, plus `GitlabListViewProvider.tsx`, migration v14 and `lib.rs`. The merge overlap is limited to the summary block and the `<Lists>` condition. `source-refresh-skeleton` (F-01) refactors `gitlab_refresh.rs` and `github_refresh.rs` without behaviour change. It must keep the "failed round keeps items" behaviour, which the existing Rust tests cover (`gitlab/mod.rs:1576,1606,1835,1906`). This slice does not touch those files.
- **Verification.** The UI has no test runner. Checking the offline case needs a manual check without VPN, or with an unreachable host, in `npm run tauri dev` by the user. Agents are not allowed to run it in this slice. The check is: after one successful round, disconnect, and the card shows over the dimmed list while the header shows the counts. Then, starting with no VPN, the card shows alone and the header says "źródło niedostępne".

## Code References

- `src/gitlab/GitlabView.tsx:602` - `const offline = state.state === "unavailable"`
- `src/gitlab/GitlabView.tsx:604-613` - header summary; `offline` comes before the counts
- `src/gitlab/GitlabView.tsx:644-657` - calm "GitLab niedostępny" card (lastOk, Sprawdź teraz, retryText)
- `src/gitlab/GitlabView.tsx:697-704` - `<Lists>` condition with `!offline`; `stale={state.state !== "ok"}`
- `src/gitlab/GitlabView.tsx:483` - `is-stale` class on `gitlab-lists`
- `src/gitlab/GitlabView.css:9-11` - `.gitlab-lists.is-stale { opacity: 0.55; }`
- `src/gitlab/GitlabView.css:58-59` - `.gitlab-row.is-hidden { opacity: 0.6; }`
- `src/github/GithubView.tsx:168,172-179,209-218,230` - GitHub list visibility, summary, calm card, stale class
- `src/github/GithubView.css:3-5` - `.github-stale { opacity: 0.55; }`
- `src/gitlab/useGitlab.ts:37-38` - snapshot kept by `seq`; no client-side list cache
- `src-tauri/src/gitlab/mod.rs:593-622` - `next_snapshot` keeps items and related fields on failed rounds
- `src-tauri/src/gitlab/mod.rs:1606` - test `snapshot_unavailable_keeps_items`
- `src-tauri/src/gitlab_refresh.rs:66-91,306-337` - in-memory `GitlabRefreshState`, `kept_full`, `reshape_kept`
- `src-tauri/src/hidden_issues/commands.rs:25-80` - hide, unhide and dismiss work from the kept list and publish
- `src-tauri/src/todo/commands.rs:135-155` - `add_todo_gitlab_item` looks the item up in the current snapshot

## Architecture Insights

- **Who owns what.** Rust owns the "keep the last list on failure" rule and tests it, and it does so the same way for GitLab and GitHub. The views only decide what to render, which matches the AGENTS.md rule that domain rules live in Rust.
- **Stale means not ok.** Both views treat "stale" as `state !== "ok"` and dim the whole list wrapper with the same opacity. Nested opacity on hidden rows (0.6 inside 0.55) already applies in the GitLab error state.
- **The design-system docs disagree.** One part says `SourceState` replaces the whole list (`Notice/README.md:10`). Another shows that the last data is shown (`Notice/preview.html:10`). The GitHub view and every plan follow "card plus dimmed last list". Fixing that README line is outside this slice. It could be a follow-up on the design-system artifact.

## Historical Context (from prior changes)

- `context/archive/2026-11-23-gitlab-assigned-list/plan-brief.md:15,23,65,70` and `plan.md:25,260-261` - the original decision: keep the last list in memory, greyed, under a calm notice.
- `context/archive/2026-11-23-konsola-design-system/plan.md:283,415,501` - the stale list should be dimmed in the Konsola look.
- `context/archive/2026-11-23-gitlab-visibility-filter/plan.md:185,382` - the open and closed sections keep the stale look offline.
- `context/archive/2026-11-24-gitlab-new-and-hidden-date/plan.md:270,354` - NOWE flags stay on the kept list offline.
- `context/archive/2026-11-24-gitlab-filter-bar-retry/plan.md:12,34,144` - records the hidden offline list as the state of the code at the time.
- `context/archive/2026-11-23-github-assigned-issues/plan.md:27,51,302,331` - GitHub's greyed last list, "as GitLab does".
- `context/foundation/roadmap.md` (S-04, MS-32) - outcome and risk line: keep the calm card (no red, no alarm), and make it clear the list is stale.

## Related Research

- Not applicable. No prior research.md covers this. `context/archive/2026-11-23-gitlab-assigned-list/` has `plan-brief.md` and `plan.md` but no research file.

## Defaults chosen by agent (non-interactive run)

1. **Header text offline with a kept list.** Show the counts only (`N widoczne · M ukryte`), as GitHub does. Add no "· nieaktualne" suffix. The card, the dimming and "odświeżone X temu" already say the list is stale.
2. **No "lista z HH:MM" banner.** The archived plan's banner text is not added. The card's `lastOk` ("ostatnie udane odświeżenie: …") already carries that time, as on GitHub.
3. **Row actions stay enabled on the dimmed list**: hide, unhide, dismiss, "+", the project and label toggles and the title link. This is how GitHub stale rows and the GitLab error state work today, and the backend supports it offline.
4. **The filter bar, closed panel and footnote come back offline**, dimmed as part of `Lists`, the same as in the error state.
5. **No VPN at app start (no list in the session).** Unchanged: the card shows alone and the header says "źródło niedostępne".
6. **Notes in `change.md`.** They were written from the orchestrator brief, because the skill was called with an empty intent.

## Open Questions

- Should the design-system artifact's `Notice/README.md:10` ("`SourceState` replaces the whole list") be corrected so that it matches the restored behaviour? It is outside this slice. It is not blocking.
- Is the opacity of hidden rows on a stale list (0.6 × 0.55 ≈ 0.33) readable enough? The error state already behaves this way today. Only a manual visual check can settle it. It is not blocking.
