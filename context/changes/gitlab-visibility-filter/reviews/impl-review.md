<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: GitLab Visibility Filter Implementation Plan

- **Plan**: context/changes/gitlab-visibility-filter/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical · 2 warnings · 7 observations

Note: phases 3 and 4 still have pending Manual rows (3.6, 4.3–4.9). Their code and Automated rows were reviewed in full; the Manual rows are treated as pending, not as failures.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | WARNING |

## Automated verification (2026-11-23)

| Command | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test` | PASS (449 unit + all integration suites, 0 failed) |
| `npm run lint` | PASS |
| `npm run format:check` | PASS |
| `npm run build` | PASS |

## Confirmed by review

- Hard rules hold:
  - The new `glab` calls use fixed arguments and are read-only.
  - `UpdatedAfter` can only be built from a `NaiveDate`.
  - The guard has its own written-out shape, and the existing shapes were not loosened.
  - Filter values never reach a log, an error or `Debug` output.
- "To do" reconciliation, notifications, `commit_round` and `kept_full` all use the unfiltered list.
- Closed items never enter `AssignedItems`.
- An invalid filter does not change the tray, which stays green.
- A failed closed fetch never blanks the open list.
- The lock order gate → `kept_config` → `kept_full` → `kept_filter` → snapshot holds in `refresh_round` and in `act_on_issue`.

## Findings

### F1 — A failed round does not reshape or clear the "Zamknięte" section

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:486-503 (with `next_snapshot` carrying `closed` in src-tauri/src/gitlab/mod.rs)
- **Detail**: A failed round that keeps the items reshapes the open list with this round's filter. The goal is that a `filterInvalid` notice never sits over a list filtered by the old rule. `next.closed`, though, is copied from the previous snapshot untouched. Suppose that, offline (no VPN), `gitlabFilter` becomes invalid or `closedDays` is removed. The view then shows three things at once:
  - the notice "pokazuję wszystkie przypisane";
  - an open list with everything;
  - a "Zamknięte" section still filtered by the old rule, or still present although it is switched off.

  This contradicts the `ClosedSection` doc ("None when closedDays is off or the filter is invalid") and the TS comment. The plan contradicts itself here: Phase 3 §3 says both "None when … invalid" and "kept with kept items". The implementation follows the keep rule.
- **Fix A ⭐ Recommended**: In the `None =>` reshape branch, set `next.closed = None` when `filter_invalid || filter.closed_days.is_none()`. Keep it unchanged otherwise.
  - Strength: A few lines, no new state. It fixes the two visible contradictions (invalid block, section switched off).
  - Tradeoff: If the rule changes offline but stays valid, the closed section still shows the old-rule result until the next successful round.
  - Confidence: HIGH — the branch already has `filter` and `filter_invalid` in scope.
  - Blind spot: Whether to also clear on a changed `closedDays` value (7 → 3) while offline.
- **Fix B**: Also keep the unfiltered closed list (a `kept_closed` slot next to `kept_full`) and re-apply the filter on kept rounds and in `act_on_issue`.
  - Strength: Full parity with the open list for every offline edit.
  - Tradeoff: More state under the gate, another lock in the order, and more tests for a rare offline-edit case.
  - Confidence: MEDIUM — straightforward, but it widens the lock surface.
  - Blind spot: A changed `closedDays` window still could not be honoured offline without a fetch.
- **Decision**: FIXED (Fix A) — `reshape_kept` drops `closed` when the filter is invalid or `closedDays` is off

### F2 — Kept-round re-filter is tested only through the helper

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/gitlab_refresh.rs:793 (`shape_items_partitions_then_filters_a_kept_list`)
- **Detail**: Progress 2.4 claims "a kept round re-filters kept_full". The test covers `shape_items` alone. Nothing exercises the wiring in `refresh_round` (`gitlab_refresh.rs:488-503`):
  - the `list_hidden` read;
  - the `next.items`, `filter_active` and `filtered_out` overwrite;
  - the `kept_filter` write;
  - the fallback when the store read fails.

  This is the branch where F1 and F3 live.
- **Fix**: Extract the `None =>` branch into a pure function (`reshape_kept(next, full, records_result, filter, …)`). Unit-test it for success, the store-error fallback and the default-filter case. Any F1 fix belongs there too.
- **Decision**: FIXED — kept-round branch extracted to the pure `reshape_kept`; 5 unit tests (re-shape, invalid block, closed off, store-read failure, no kept items)

### F3 — Kept-round reshape reads the store under the kept locks and falls back silently

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:494
- **Detail**: `store.with_store(… list_hidden …)` runs while holding `hidden_gate`, `kept_full` and `kept_filter`. The documented lock order does not mention the store. No site takes a `kept_*` lock while holding the store lock, so there is no deadlock today. Still, a health round that holds the store lock during reopen or backup blocks hide/unhide clicks for that long. If the read fails, the list keeps the old-rule shape while `filterInvalid` may be set (the notice-over-old-list case the plan meant to remove). The code comment documents this, but the plan does not.
- **Fix**: Read the hidden records before taking `kept_full`/`kept_filter` (under the gate only). Add one line to the lock-order doc saying the store is never taken under a `kept_*` lock.
- **Decision**: FIXED — hidden records read under the gate before any `kept_*` lock; lock-order doc updated

### F4 — "Set user" notice disappears when every hidden issue is filtered out

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/gitlab/GitlabView.tsx:274
- **Detail**: `hasHidden` now counts the filtered `items.hiddenIssues`. When the filter removes all hidden issues, the "set `user`" notice disappears. Those issues are still checked for comments, so the notice is still relevant.
- **Fix**: `hasHidden = (items?.hiddenIssues.length ?? 0) + snapshot.filteredOut.hiddenIssues > 0`.
- **Decision**: FIXED — `hasHidden` adds `filteredOut.hiddenIssues`

### F5 — Closed rows show `updatedAt`, but the section is sorted by closing time

- **Severity**: OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src/gitlab/GitlabView.tsx:71, :183
- **Detail**: `ClosedBlock` reuses `ItemRow`, which prints `formatUpdated(item.updatedAt)`. The section is sorted by closing time (`closed_at`/`merged_at`), so an item closed 5 days ago but commented on yesterday shows "yesterday" below items that look newer. The dates can then look out of order. The plan did not say which time the row shows (plan gap).
- **Fix**: Carry the closing time to the webview, for example as a `closedAt` field on closed entries, set in `parse_recently_closed`, which already computes it. Then render it in `ClosedBlock` instead of `updatedAt`, mirrored in `types.ts`.
  - Strength: The row date then matches the section's meaning and order.
  - Tradeoff: A Rust↔TS contract change and a new serialization test; `GitlabItem` or `ClosedSection` changes shape.
  - Confidence: MEDIUM — the instant is already computed in the parser and is simply dropped after sorting.
  - Blind spot: Whether the user actually cares; they may treat the section as "recent activity".
- **Decision**: FIXED — new `ClosedItem { #[serde(flatten)] item, closed_at }` (TS `ClosedItem = GitlabItem & { closedAt }`); closed rows show the closing time; serialization and fake-CLI tests extended

### F6 — Open-list parsing became stricter through the new optional fields

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab/mod.rs:208-216
- **Detail**: `milestone`, `state`, `closed_at` and `merged_at` are typed fields on the shared `RawItem`. A wrong type in any of them now turns the whole open list into `ResponseInvalid`, a red error. That includes `closed_at`, which only the closed parser needs. The plan asked for this for `milestone` ("like the other fields"). Real GitLab always sends strings, `null` or `{title}`, so the risk is low. Still, data that only the filter or the closed section needs can now take down the core list.
- **Fix**: Accept as planned, or deserialize these four leniently (a wrong type becomes `None`) via a small `deserialize_with` helper.
- **Decision**: ACCEPTED — the plan asked for strict typing of `milestone`; real GitLab sends strings/null

### F7 — `act_on_issue` does not use the shared `shape_items` helper

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/gitlab_refresh.rs:140
- **Detail**: The plan (Phase 2 §2) asks the kept round and `act_on_issue` to use the same partition-and-filter helper. `act_on_issue` calls `partition` and `filter_shown` inline, while the kept round uses `shape_items` (`:194`). The behaviour is identical today, but the two copies can diverge.
- **Fix**: Have `act_on_issue` call `shape_items(full, &records, &filter)`.
- **Decision**: FIXED — `act_on_issue` uses `shape_items`; the hidden cap is the pre-filter count (shown + filtered out)

### F8 — Cosmetic UI text details

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/gitlab/GitlabView.tsx:172, src/todo/TodoView.tsx:470
- **Detail**:
  - The heading reads "Zamknięte (ostatnie N dni) (count)". That matches the other headings but not the planned text. It shows "(0)" when the section has an error.
  - `closedDays: 1` gives "ostatnie 1 dni" and "w ostatnich 1 dniach".
  - With an active filter and nothing assigned at all, the picker says "Brak pasujących do filtra (ukryte filtrem: 0)".
- **Fix**: Hide the count on error. Use "ostatni dzień" / "z ostatniego dnia" for N = 1. Fall back to "Brak przypisanych w GitLabie." when the filtered count is 0.
- **Decision**: FIXED — count hidden on error, singular text for `closedDays: 1`, picker falls back to "Brak przypisanych w GitLabie." when nothing is filtered out

### F9 — Closed MR call re-downloads open MRs every round

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/mod.rs (`GlabRecentlyClosedMergeRequests`, `state=all`)
- **Detail**: With `closedDays` set, each successful round makes 2 extra `--paginate` calls. The MR call uses `state=all`, so it also fetches the open MRs updated in the window. This is bounded (≤31 days, no lock held, after the open fetch) and was a deliberate plan choice, one call instead of two. It is fine within the "Lekkość w tle" NFR.
- **Fix**: Accept. Revisit only if the refresh interval becomes short.
- **Decision**: ACCEPTED — deliberate plan choice, within the NFR

## Triage summary (2026-11-23)

- **Fixed**: F1 (Fix A), F2, F3, F4, F5, F7, F8 (7)
- **Accepted**: F6, F9 (2)
- **Contract change**: `ClosedSection.items` is now `ClosedItem[]` (a flattened `GitlabItem` plus `closedAt`). This replaces the plan's `Vec<GitlabItem>`; Rust and TS were changed together.
- **Re-verified after fixes**:
  - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (454 unit, 0 failed);
  - `npm run lint`, `npm run format:check`, `npm run build`.
