<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: "To do" List

- **Plan**: context/changes/todo-list/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 7 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

Automated checks (all passed on 2026-11-23): `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (216 unit tests plus the integration and crash suites, 0 failed).

Unplanned additions (harmless, justified): `ItemKind` gained `Deserialize` (`src-tauri/src/gitlab/mod.rs:17`, needed for the `add_todo_gitlab_item` argument); `TodoError::kind()` and the `From` impls; `isTodoStale()` reload-on-refusal and a `NotFound` message in `src/todo/api.ts`; extra tests.

## Findings

### F1 — A successful list from another account ticks every GitLab entry

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:200 (with src-tauri/src/todo/mod.rs reconcile_gitlab, src-tauri/src/cli/mod.rs:133)
- **Detail**: Reconcile treats "missing from a successful list" as closed, but a successful list can also come from a different configuration:
  - a changed `host` or `user`;
  - a typo in `user`: `assignee_username=<typo>` returns an empty list with success;
  - `glab` logged in as a bot with no `user` set: its assigned list is empty.

  `next_snapshot` replaces the items on any success (`gitlab/mod.rs:262-266`), and `reconcile_todo` then ticks every open live GitLab entry with `ticked_by='source'` and the "zamknięte w GitLabie" label. At midnight roll-over closes them all, so each has to be re-picked by hand. No rows are lost, but the label is false. Unticking does not help: the next round ticks the entry again until the config is fixed. The plan's key discovery (`state=opened` means "missing = closed") assumed the configuration never changes.
- **Fix A ⭐ Recommended**: Stamp each GitLab entry with a fingerprint of the (host, user) it was picked under (migration v4, a nullable column holding a hash, so no hostname in clear text), and reconcile only rows whose fingerprint matches this round's config. Treat a NULL fingerprint on existing rows as "current config".
  - Strength: Covers a switch, a typo and a bot login. Entries from another account stay open and show the existing "niezweryfikowany" note.
  - Tradeoff: A new migration plus changes to `add_gitlab`, `reconcile_gitlab` and their tests.
  - Confidence: MED — the shape is straightforward, but the domain and the tests both change.
  - Blind spot: How often the user actually edits the GitLab block. Not verified.
- **Fix B**: Accept the risk and record it in the plan as a known limitation ("after changing `gitlab.host`/`user`, entries from the old list show as closed").
  - Strength: No code. Config edits are rare in a single-user app.
  - Tradeoff: A typo in `user` marks the whole list closed, and after midnight it has to be re-picked.
  - Confidence: HIGH — nothing changes.
  - Blind spot: The bot-login case may be more common than a deliberate change.
- **Decision**: FIXED (Fix A) — migration v4 `gitlab_account`, `gitlab::account_key`, snapshot `account`; reconcile scoped to the account; tests added; plan addendum.

### F2 — "GitLab niedostępny" shows at every app start

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/todo/TodoView.tsx:110
- **Detail**: `unverified = gitlabState !== "ok" && …` is also true while the snapshot is `null` or the state is `"starting"`. "Do zrobienia" is the default tab, so every start with open GitLab entries says "GitLab niedostępny … (ostatnie odświeżenie brak)" until the first round finishes, even with VPN up. That is a false statement on the first screen. The plan specified "not `ok`" literally, so this is a flaw in the plan. The picker already handles `starting` separately ("Pobieram przypisane z GitLaba…").
- **Fix**: Show the line only when `gitlabState` is `unavailable`, `error` or `notConfigured`, not when it is `null` or `starting`.
- **Decision**: FIXED — line shown only for a load error or unavailable/error/notConfigured.

### F3 — Picker sections miss the AGENTS.md source states

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/todo/TodoView.tsx:284, src/todo/TodoView.tsx:314
- **Detail**: AGENTS.md says "every view that shows a source renders … error (via ErrorState, with the time of the last successful refresh)" and "Empty is a data state with an explicit message". In the picker:
  - The own-task `ErrorState` has neither `onRetry` nor `lastSuccess`. It recovers only through an unrelated successful To-do reload.
  - A failed `useGitlab` load is plain notice text without retry; `useGitlab().retry` is not used.
  - When GitLab is `ok` with zero assigned items, nothing renders under the "GitLab" heading.
- **Fix**: Pass `onRetry` and `lastSuccess` to the own-task `ErrorState`, wire `useGitlab().retry` for a GitLab load error, and render "Brak przypisanych w GitLabie." for an empty successful list.
- **Decision**: FIXED — retry + last-success on both picker errors; "Brak przypisanych w GitLabie." for an empty successful list.

### F4 — The UI can show the state from before a committed mutation

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/todo/useTodo.ts:53, src/todo/useTodo.ts:126, src-tauri/src/todo/commands.rs:72-75
- **Detail**: Two edge cases:
  - **Newest request wins.** A reload started during a mutation (from `todo-changed` or the midnight timer) gets a higher id. If the backend serves it before the mutation, the pre-mutation list is applied and the mutation's newer result is discarded. A failed mutation also bumps `latest` and discards an in-flight reload.
  - **Second transaction fails.** The mutation and the follow-up `list` are two transactions. If the mutation commits and `list` fails, the UI shows an error. `isTodoStale` is false for `Store`, so nothing reloads.

  Both are rare, and both heal on the next focus.
- **Fix**: In `mutate`, call `reload()` after any error, and after a success whose id is no longer the latest.
- **Decision**: FIXED — `mutate` reloads after any error and after a superseded success; `isTodoStale` removed.

### F5 — To-do store errors are logged without the inner kind, and reconcile warns every round

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/todo/error.rs:37, src-tauri/src/gitlab_refresh.rs:246
- **Detail**:
  - `TodoError::kind()` returns plain `"Store"`, while `work_time/commands.rs:20` logs `s.kind()` (for example `Busy` or `Damaged`). The To-do logs cannot tell the store failures apart.
  - `reconcile_todo` logs `warn` on every successful GitLab round (every 5 min) while the store is Missing, Damaged or NewerSchema. The store module deliberately logs open failures at debug level, because the health round reports them.
- **Fix**: Log the inner `StoreError::kind()` for `TodoError::Store`, and log reconcile store failures at `debug`.
- **Decision**: FIXED — `TodoError::kind()` returns the store kind; reconcile store failures logged at debug.

### F6 — `"limit": null` is accepted as the default

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/settings.rs:137-187
- **Detail**: The plan maps only a missing file, an absent block, a `null` block and an absent `limit` to 5; "anything else" is `SettingsInvalid`. The code also maps `{"todo": {"limit": null}}` to 5, and a test pins it. `DEFAULT_TODO_LIMIT` lives in `todo/mod.rs:31`, not in `settings.rs`, which is harmless.
- **Fix**: Record `"limit": null` → default as an addendum to the plan (it matches the `null` block rule).
- **Decision**: FIXED — recorded in the plan addendum (`"limit": null` → default).

### F7 — The JSON-shape test pins only part of `TodoList`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/todo/tests.rs:546
- **Detail**: `list_json_shape` asserts `day`, `limit`, `limitError.kind`, `openCount`, `carriedOver`, `pickedOn`, `entry.kind`, `taskId`, `space`, `itemKind`, `closedInSource` and `webUrl`. It does not assert `id`, `ticked`, `pickedAt`, `title` or `reference`, which `TodoView.tsx` reads, so a rename there would pass `cargo test` and break the UI silently. It also lives in `tests.rs` rather than next to the commands; that is fine.
- **Fix**: Add assertions for `id`, `ticked`, `pickedAt`, `entry.title` and `entry.reference`.
- **Decision**: FIXED — `list_json_shape` also pins id, ticked, pickedAt, title, reference and the absent account.

### F8 — Adding a GitLab item can slip past the reconcile

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/gitlab_refresh.rs:200-201, src-tauri/src/todo/commands.rs:108-112
- **Detail**: `reconcile_todo(app, &next)` runs before `*state.write() = next`. An `add_todo_gitlab_item` in that window finds the item in the old snapshot and inserts it after the reconcile has already run against a list where the item is absent. The entry stays open until the next successful round, at most 5 minutes or one "Odśwież". It fixes itself.
- **Fix**: Accept it (it heals within one round) and note it in the plan.
- **Decision**: ACCEPTED — heals within one round; recorded in the plan addendum.

### F9 — Entries ticked on a "future" day become invisible and block re-picking

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/todo/mod.rs:107-120, src-tauri/src/todo/mod.rs:218-231
- **Detail**: Visibility uses `tick date == today` and roll-over uses `tick date < today`. If the clock or time zone moves back across a date (manual date tests such as 3.10, or travel), an entry ticked on the later date is neither shown nor closed. It stays live and makes a re-pick return `AlreadyOnList` for something the user cannot see. Not reproduced; this follows from reading the code.
- **Fix**: Roll over entries whose tick date is `!= today`, not only `< today`.
- **Decision**: FIXED — roll-over closes ticks on any other day; test added; plan addendum.

### F10 — Manual rows were ticked together without recorded evidence

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/todo-list/plan.md:388-408 (commit c2e30fa)
- **Detail**: Rows 2.3–2.4 and 3.4–3.11 were all ticked in one commit, 3 minutes after the epilogue commit. Two of them need real setup:
  - 3.10 needs the system date moved forward on a backed-up store;
  - 2.3 needs an item closed in GitLab while on VPN.

  The commit says the user confirmed them, and the repo has no other evidence.
- **Fix**: Confirm that 2.3 and 3.10 were actually exercised; untick any that were not.
- **Decision**: DISMISSED — the user confirmed 2.3 and 3.10 were run.
