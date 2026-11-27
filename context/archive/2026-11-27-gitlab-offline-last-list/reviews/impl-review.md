<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Last GitLab list without VPN

- **Plan**: context/changes/gitlab-offline-last-list/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS (1 observation) |

## Scope and method

`git diff develop...HEAD`: code change is commit 545e47d only, `src/gitlab/GitlabView.tsx` 4+/4-; the rest is change docs (`change.md`, `plan-brief.md`, `plan.md`, `research.md`, `reviews/plan-review.md`) and the epilogue ea26dae (Progress SHAs). Planned files vs diff: the one planned code file is the one changed file; nothing missing, nothing extra. Reviewed inline (no sub-agents): the diff is two hunks in one file, and every claim below was checked against the current source.

## Plan adherence

| Planned change | Actual | Verdict |
|---|---|---|
| Summary order: items && configured → counts; offline → "źródło niedostępne"; error → "błąd źródła" (`GitlabView.tsx:604-613`) | Exactly that order, strings unchanged | MATCH |
| `<Lists>` condition drops `&& !offline` (`GitlabView.tsx:697`) | `items && state.state !== "notConfigured"`; `stale={state.state !== "ok"}` unchanged | MATCH |
| `const offline` stays and gates the card | Line 602 kept; used at 609 (summary) and 644 (card) only | MATCH |
| No Rust / store / command / TS type / CSS change, no migration | None | MATCH |

## Rendered order and states (verified in `GitlabView.tsx:615-713`)

Order is fixed by JSX: header → notConfigured card → calm "GitLab niedostępny" card → `ErrorState` (error) → notices → `TodoPicksNotices` → `Lists` → `refreshError`.

| State | Items | Header summary | Body | Change |
|---|---|---|---|---|
| ok | yes | `N widoczne · M ukryte` | full-opacity `Lists` | unchanged |
| ok | no | none | nothing (unreachable after a success) | unchanged |
| error | yes | counts | red `ErrorState`, dimmed `Lists` | unchanged |
| error | no | "błąd źródła" | `ErrorState` alone | unchanged |
| unavailable | yes | counts (was "źródło niedostępne") | calm card (lastOk, "Sprawdź teraz", next try), dimmed `Lists` under it | **new, as planned** |
| unavailable | no | "źródło niedostępne" | calm card alone | unchanged |
| unavailable | empty kept list | `0 widoczne · 0 ukryte` | calm card, dimmed empty message under it | new, plan default (card shown, so the empty list never replaces the failed source; same as the error state and GitHub) |
| starting | yes | counts | dimmed `Lists` | unchanged |
| notConfigured | any | none | config card | unchanged |

Parity with `src/github/GithubView.tsx:168-179,230`: same summary branch order, list shown whenever the source is configured and a list exists, dimmed when not ok (`.gitlab-lists.is-stale` and `.github-stale` both `opacity: 0.55`).

## Other gates checked for odd behaviour with the list now visible offline

- `offline` is referenced only at `GitlabView.tsx:602,609,644`; nothing else in the view gates on it.
- Filter bar, toolbar, view-filter row, hidden footnote and "Zamknięte" panel live inside `Lists` and come back dimmed, as in the error state today; Rust keeps `filter`, `filteredOut`, `closed` and `ownCommentsKnown` on an Unavailable round (`snapshot_unavailable_keeps_items`, `snapshot_keeps_hidden_state_with_the_items_on_unavailable_and_error_rounds` pass).
- Row actions (hide, unhide, dismiss, "+") act on the kept list and the store, never GitLab; behaviour unchanged from the error state.
- Seen mark: `useGitlabSeenMark` (`src/gitlab/useGitlabSeenMark.ts`, called from `App.tsx:37`) is view-based, not state-based; NOWE flags (`GitlabView.tsx:202`, `item.isNew` from Rust) now show offline before the visit marks them seen, which is an improvement over marking them seen unseen.
- Empty message: `emptyText(kind, filterActive)` renders under the calm card, never instead of it, so the AGENTS.md rule "Never render an empty plan or list in place of a failed source" holds.
- Notices (`filterInvalid`, `hiddenError`, `hiddenCheckFailed`, `ownCommentsKnown`) were never gated on `offline`; unchanged.

## AGENTS.md rules

UI-only, no `invoke` outside `api.ts`, no literal colours or new CSS, no new webview resource or connection (no CSP impact), no store change, no work data, no `glab`/`gh` change. The unavailable state stays calm (`SourceState kind="calm"`, `role="status"`, no red). Compliant.

## Success criteria

| Row | Command | Result |
|---|---|---|
| 1.1 | `npm run lint` | pass |
| 1.2 | `npm run format:check` | pass ("All matched files use Prettier code style!") |
| 1.3 | `npm run build` | pass (tsc + Vite, built) |
| 1.4 | `cargo fmt --check` | pass |
| 1.5 | `cargo clippy --all-targets -- -D warnings` | pass |
| 1.6 | `cargo test` | pass (638 unit + all integration suites; `snapshot_unavailable_keeps_items` ok) |
| 1.7 | `git diff --stat develop...HEAD -- src src-tauri` | pass (only `src/gitlab/GitlabView.tsx`, 4+/4-) |

Manual rows 1.8-1.14 are unchecked and pending for the user in the integrated build; no row is ticked without evidence.

## Findings

### F1 — Phase block criteria lag behind the Progress rows

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/gitlab-offline-last-list/plan.md:77,85-86
- **Detail**: The plan-review fixes (its F1, F2) were applied to `## Progress` only. Progress 1.7 uses `git diff --stat develop...HEAD`, but the Phase 1 "Automated Verification" bullet at line 77 still says `git diff --stat HEAD~1 -- src src-tauri` (which with the epilogue commit on top now compares against 545e47d and lists nothing). Progress 1.13 (empty kept list offline) and 1.14 (NOWE flags on the dimmed list) have no matching bullet in "Manual Verification", and line 86 ("No CSP check ... is needed") is still a bullet rather than a note. The `plan-review.md` decisions also remain `PENDING`. No effect on the code; only the plan's two copies of the criteria disagree.
- **Fix**: In `plan.md`, line 77: replace `git diff --stat HEAD~1 -- src src-tauri` with `git diff --stat develop...HEAD -- src src-tauri`; after line 85 add `- Successful round with an empty list, then VPN off: card on top, dimmed empty message under it, header \`0 widoczne · 0 ukryte\`` and `- NOWE flags from the last round still show on the dimmed offline list`; turn line 86 into a plain note under the list (`Note: no CSP check in the built app is needed (no new webview resource or connection).`). Optionally set the three `plan-review.md` decisions to `FIXED` (F1, F2) and `ACCEPTED` (F3). Docs only; no code change.
- **Decision**: PENDING
