# Last GitLab list without VPN — Implementation Plan

## Overview

Without VPN, "Praca (GitLab)" keeps the calm "GitLab niedostępny" card and shows the session's last list under it, dimmed, the way "Projekty (GitHub)" already does. The view header shows that list's counts (`N widoczne · M ukryte`) instead of only "źródło niedostępne". Roadmap M-4 S-04, anchor MS-32 (GitHub issue #32). User decision 2026-11-27: fix it, not `wontfix`; hiding the list was a side effect, not a decision.

## Current State Analysis

- Rust already keeps the last list when a round fails as Unavailable or Error: `next_snapshot` copies `items`, `closed`, the filter fields and `last_success_ms` from the previous snapshot while the host and user are unchanged (`src-tauri/src/gitlab/mod.rs:593-622`, test `snapshot_unavailable_keeps_items` at `gitlab/mod.rs:1606`).
- The UI hides it with one condition: `items && state.state !== "notConfigured" && !offline` (`src/gitlab/GitlabView.tsx:697`), added in 0d187bc with no stated reason.
- The header summary checks `offline` before the counts (`src/gitlab/GitlabView.tsx:604-613`), so the kept list's counts never show offline. In the `error` state with items the counts already show.
- Dimming is still wired: `Lists` sets `is-stale` when `stale={state.state !== "ok"}` (`GitlabView.tsx:483,700`), and `.gitlab-lists.is-stale { opacity: 0.55; }` exists (`src/gitlab/GitlabView.css:9-11`), same value as `.github-stale` (`src/github/GithubView.css:3-5`).
- Row actions (hide, unhide, dismiss, "+") work on the kept list without calling GitLab (`src-tauri/src/hidden_issues/commands.rs:25-80`, `src-tauri/src/todo/commands.rs:135-155`). Todo reconcile and notifications are skipped when the state is not Ok.
- GitHub's target pattern: counts first when the list exists, then "źródło niedostępne", then "błąd źródła" (`src/github/GithubView.tsx:172-179`); list rendered whenever repos exist and the source is configured (`GithubView.tsx:168,230`).

## Desired End State

- After at least one successful GitLab round in the session, losing VPN shows: the header with `N widoczne · M ukryte` and "odświeżone X temu"; the calm "GitLab niedostępny" card (last success time, "Sprawdź teraz", next try, no red); under it the full `Lists` block (filter bar, toolbar, table, hidden footnote, "Zamknięte" panel) dimmed by `is-stale`.
- With no VPN at app start (no list in the session yet), nothing changes: the card shows alone and the header says "źródło niedostępne".
- `error` and `ok` states render exactly as today.

### Key Discoveries:

- `src/gitlab/GitlabView.tsx:697` — the only gate hiding the kept list offline.
- `src/gitlab/GitlabView.tsx:604-613` — summary order to flip; pattern to copy is `src/github/GithubView.tsx:172-179`.
- `src/gitlab/GitlabView.tsx:602,644` — `offline` constant stays; it still drives the card.
- `src/gitlab/GitlabView.css:9-11` — dimming CSS already exists; no CSS change.

## What We're NOT Doing

- No Rust, store, command, TS type or CSS change; no store migration (parallel slice `gitlab-view-memory` owns migration v14).
- No "· nieaktualne" suffix in the header (default chosen by agent: GitHub shows counts only; the card, dimming and "odświeżone X temu" already say it is stale).
- No "lista z HH:MM" banner from the archived `gitlab-assigned-list` plan (default chosen by agent: the card's `lastOk` already carries that time, as on GitHub). The archived plan text stays as it is.
- No gating of row actions on the dimmed list (default chosen by agent: hide, unhide, dismiss, "+", project/label toggles and the title link stay enabled, as on GitHub stale rows and the GitLab error state today).
- No change to the "no VPN at app start" case (default chosen by agent: card alone, header "źródło niedostępne").
- No fix to the design-system artifact's `Notice/README.md:10` wording; possible follow-up, outside this slice.
- No new FR in the PRD; no roadmap edit in this phase (the orchestrator owns roadmap status).

## Implementation Approach

Two edits in one file, both following GitHub's existing stale-list pattern. The filter bar, the "Zamknięte" panel and the hidden-issue footnote come back offline dimmed as part of `Lists` (default chosen by agent), the same as in the `error` state today. A kept list that is empty shows its normal dimmed empty message under the card (default chosen by agent: same as the `error` state today; the card is shown, so the empty list never stands in for the failed source).

## Phase 1: Show the kept list offline

### Overview

Render `<Lists>` in the `unavailable` state when the session has a kept list, and show its counts in the header.

### Changes Required:

#### 1. Header summary order

**File**: `src/gitlab/GitlabView.tsx` (summary block, ~lines 604-613)

**Intent**: Show the kept list's counts whenever `items` exist and the source is configured, regardless of `offline`/`error`; fall back to "źródło niedostępne" (offline) and then "błąd źródła" (error) only when there are no items. Mirrors `GithubView.tsx:172-179`.

**Contract**: Branch order becomes: (1) `items && state.state !== "notConfigured"` → `N widoczne · M ukryte` (existing `plural` text unchanged); (2) `offline` → `"źródło niedostępne"`; (3) `state.state === "error"` → `"błąd źródła"`. No new strings.

#### 2. Lists render condition

**File**: `src/gitlab/GitlabView.tsx` (`<Lists>` condition, ~line 697)

**Intent**: Drop `&& !offline` so the kept list renders under the calm card; `stale={state.state !== "ok"}` already dims it.

**Contract**: Condition becomes `items && state.state !== "notConfigured"`. `const offline` (line 602) stays and still gates the card (line 644). No other line in the file changes; keep the diff to these two spots (shared file with `gitlab-view-memory`).

### Success Criteria:

#### Automated Verification:

- Lint passes: `npm run lint`
- Formatting passes: `npm run format:check`
- Type check and build pass: `npm run build`
- Rust formatting passes: `cargo fmt --check` (in `src-tauri/`)
- Clippy passes: `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- Rust tests pass, including `snapshot_unavailable_keeps_items`: `cargo test` (in `src-tauri/`)
- Only `src/gitlab/GitlabView.tsx` changes among code files: `git diff --stat develop...HEAD -- src src-tauri` lists that one file

#### Manual Verification:

- With VPN on, after one successful GitLab round, turn VPN off and wait for (or force with "Odśwież") a failed round: the calm "GitLab niedostępny" card shows (last success time, "Sprawdź teraz", next try, no red), and under it the last list, dimmed; the header shows `N widoczne · M ukryte` and "odświeżone X temu"
- Start the app with VPN off (no successful round yet): the card shows alone, no list, header "źródło niedostępne"
- On the dimmed offline list, "Pokaż ukryte" shows hidden rows; their nested dimming (about 0.33 opacity) is still readable
- On the dimmed offline list, hide, unhide, dismiss and "+" (add to "Do zrobienia") work without errors; the filter bar, "Zamknięte" panel and footnote show dimmed
- Turn VPN back on and refresh: the list returns to full opacity with fresh data; `error` and `ok` states look as before
- Successful round with an empty list, then VPN off: card on top, dimmed empty message under it, header `0 widoczne · 0 ukryte`
- NOWE flags from the last round still show on the dimmed offline list

Note: no CSP check in the built app is needed (no new webview resource or connection).

**Implementation Note**: After automated verification passes, commit the phase. The Manual rows stay unchecked; the user checks them later in the integrated build.

---

## Testing Strategy

### Unit Tests:

- No new tests: the UI has no test runner and Rust is unchanged. Existing Rust tests (`gitlab/mod.rs:1576,1606,1835,1906`) keep covering "failed round keeps items".

### Integration Tests:

- None added.

### Manual Testing Steps:

1. VPN on, open "Praca (GitLab)", wait for a successful round; note the counts.
2. VPN off, press "Odśwież": card plus dimmed list, same counts in the header.
3. Toggle "Pokaż ukryte", hide one row, add one row to "Do zrobienia": all work offline.
4. VPN on, press "Odśwież": list back to normal.
5. Quit, start with VPN off: card alone, "źródło niedostępne".

## Performance Considerations

None: the list already exists in the snapshot; only its rendering is enabled.

## Migration Notes

None. No store schema change.

## Shared-file notes (parallel slices)

- `gitlab-view-memory` (S-05) also edits `src/gitlab/GitlabView.tsx` (plus `GitlabListViewProvider.tsx`, migration v14, `lib.rs`). This slice's overlap is limited to the summary block (~604-613) and the `<Lists>` condition (~697); whichever merges second rebases these two small hunks.
- `source-refresh-skeleton` (F-01) refactors `src-tauri/src/gitlab_refresh.rs` and `github_refresh.rs` without behaviour change; this slice relies on its "failed round keeps items" behaviour (covered by the Rust tests above) and touches neither file.

## References

- Research: `context/changes/gitlab-offline-last-list/research.md`
- Target pattern: `src/github/GithubView.tsx:168,172-179,230`
- Kept-list rule: `src-tauri/src/gitlab/mod.rs:593-622`
- Original decision: `context/archive/2026-11-23-gitlab-assigned-list/plan.md:25,260-261`
- Roadmap: `context/foundation/roadmap.md` (M-4, S-04, MS-32)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Show the kept list offline

#### Automated

- [x] 1.1 Lint passes: `npm run lint` — 545e47d
- [x] 1.2 Formatting passes: `npm run format:check` — 545e47d
- [x] 1.3 Type check and build pass: `npm run build` — 545e47d
- [x] 1.4 Rust formatting passes: `cargo fmt --check` — 545e47d
- [x] 1.5 Clippy passes: `cargo clippy --all-targets -- -D warnings` — 545e47d
- [x] 1.6 Rust tests pass: `cargo test` — 545e47d
- [x] 1.7 Only `src/gitlab/GitlabView.tsx` changes among code files: `git diff --stat develop...HEAD -- src src-tauri` — 545e47d

#### Manual

- [ ] 1.8 VPN off after a successful round: calm card plus dimmed last list, header shows counts
- [ ] 1.9 Start with VPN off: card alone, header "źródło niedostępne"
- [ ] 1.10 Hidden rows on the dimmed offline list are readable
- [ ] 1.11 Row actions and filter bar, "Zamknięte" panel, footnote work dimmed offline
- [ ] 1.12 VPN back on: list returns to normal; error and ok states unchanged
- [ ] 1.13 Empty kept list offline: card, dimmed empty message under it, header `0 widoczne · 0 ukryte`
- [ ] 1.14 NOWE flags still show on the dimmed offline list

Note: no CSP check needed, no new resource or connection.
