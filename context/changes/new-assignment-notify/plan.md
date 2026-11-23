# New Assignment Notifications Implementation Plan

## Overview

Roadmap S-04 (PRD FR-004, NFR "Świeżość", NFR "Lekkość w tle"): the user gets a Windows notification for every new GitLab issue or MR assigned to them. This holds whatever the list shows, within 15 minutes while on VPN. When 1–3 items are new in one round, each gets its own toast; more than 3 arrive as one combined toast. The background refresh moves from a fixed 5 min to a setting that defaults to 10 min (allowed 1–14).

## Current State Analysis

- The `gitlab-refresh` thread runs a round at startup, on a wake (button, view open, tray "Sprawdź teraz", resume from sleep) and every fixed `REFRESH_INTERVAL` of 5 min (`src-tauri/src/gitlab_refresh.rs:38`, loop `:188-200`). S-01 built it for S-04 to reuse; nothing diffs or notifies today.
- A successful round holds the full unpartitioned list as `full` (`gitlab_refresh.rs:274-283`). `snapshot.items.issues` holds visible issues only, because S-03 hidden issues are split out. A failed or unavailable round keeps the old items with `state != Ok`. `reconcile_input` (`:344-349`) is the existing "act only on a successful round's list" rule.
- The fetch is complete or it errors: `state=opened`, `--paginate`, no page cap, a 16 MiB cap that fails hard (`src-tauri/src/gitlab/fetch.rs:12-32`, `src-tauri/src/cli/runner.rs:23`). A missing item therefore means closed or unassigned, never "beyond the last page".
- `GitlabItem.id` is GitLab's global id (`src-tauri/src/gitlab/mod.rs:51-75`). Issue ids and MR ids are separate sequences, so the same number can be both. `account_key` (`gitlab/mod.rs:20-32`) is an opaque SHA-256 of host plus user, already stored with "To do" rows.
- The list is in memory only. Without a stored seen set, every item looks new after each start. The store's latest migration is v6 (`src-tauri/src/store/schema.rs:106-122`).
- `runtime::toast` (`src-tauri/src/runtime.rs:234-241`, private) is the only place a toast is shown, gated by `health::notices_allowed()` (`src-tauri/src/health/mod.rs:362-367`, always `true` for now). `show()` drops OS errors; the toast is a courtesy and clicking it does nothing.
- `settings.json` is hand-edited. Each block has its own loader so a typo in one never breaks another. `RawGitlab` rejects unknown fields (`deny_unknown_fields`, `src-tauri/src/settings.rs:86-92`), so a new key there would make the whole list `SettingsInvalid`. The "To do" `limit` (`settings.rs:137-187`) is the pattern for a numeric setting with a default.

## Desired End State

- After each successful GitLab round, every assigned issue or MR whose (account, kind, id) has never been seen gives a toast. This covers items assigned while the app was closed, which notify on the first round after start.
- 1–3 new in one round: one toast each, titled `Nawigator dnia — nowe zgłoszenie` or `Nawigator dnia — nowy MR`, with body `<reference> — <title>`. More than 3: one toast `Nawigator dnia — nowe przypisania (N)` with 3 such lines plus `i N−3 więcej`.
- The first successful round for an account the store has never seen (fresh install, a changed host or `gitlab.user`) records everything silently.
- An id absent from successful rounds for 30 days is forgotten, so its later return notifies. Reopens and short unassign/reassign blips within 30 days stay silent.
- Restarts, VPN reconnects, hiding, unhiding and S-03 resurfacing never re-notify an already seen item. Unavailable, error, starting and not-configured rounds never notify and never change the seen set.
- `settings.json` accepts `"gitlabRefresh": {"minutes": N}` with N an integer 1–14 (default 10, also when absent). An invalid value keeps refreshing every 10 min and the "Praca (GitLab)" view says the setting is invalid.
- Logs carry counts and kinds only: never a reference, title, host or URL.

Verify: `cargo test` covers the rules, store, notice text and settings. A manual check on the real GitLab shows a toast for a newly assigned item within the interval, no repeat after a restart and no burst after VPN reconnects.

### Key Discoveries:

- The full list (hidden issues included) must be the input. Diffing `snapshot.items.issues` would treat un-hiding or resurfacing as new (`gitlab_refresh.rs:121-125`, `:278-282`).
- Issue/MR id collision: key by kind too. The fixture ids (issues 5001/5002, MRs 6001–6003, `src-tauri/src/bin/fake_cli.rs:71-149`) don't collide, so a test must build a collision explicitly.
- "No rows for this account" is not the same as "never seeded". An account whose list stayed empty for 30 days would have had all rows pruned, and its next real assignment would be swallowed silently. Seeding is therefore a separate per-account marker that is never pruned.
- Lock rule: no toast, emit or process call while holding `hidden_gate` or the snapshot lock (`gitlab_refresh.rs:5-13`). The store lock is taken inside `with_store` only.
- The watchdog already wakes the refresh thread after sleep (`runtime.rs:342-348`), so a laptop resuming catches up in one round. That round goes through the same 3-or-combined rule.

## What We're NOT Doing

- No "Nowe" mark or new-item state in the GitLab list; the toast is the whole user-facing feature.
- No toast click handling or actions (the desktop notification plugin has none; ruled out in `context/archive/2026-11-20-source-health-status/plan.md:60`).
- No notification for S-03 resurfaced issues, for status or comment changes, or for MRs where the user is a reviewer (PRD Non-Goals).
- No S-02 filter work. When the filter lands, notifications keep using the unfiltered full list.
- No new `glab` call. Notifications use the list the round already fetched.
- No distinguishing reopen from reassignment (the list cannot tell them apart; 30-day forget decided instead).
- No Focus Assist detection or replay of suppressed toasts; `notices_allowed()` stays the single gate.
- No settings UI; `settings.json` stays hand-edited.
- No storing of titles, references, URLs or hostnames: the seen set holds the opaque account key, kind, numeric id and timestamps only.

## Implementation Approach

A new `tauri`-free module `new_assignments` holds the pure rules, the toast text and one store operation, following the `hidden_issues` and `todo` split (rules pure, one `write` per operation, side effects in `gitlab_refresh.rs`).

- **Each successful round:** one store write records the full list for its account and returns the keys that are new. The refresh thread maps those keys back to items, builds the notices and shows them after the gate and publish, so nothing blocks on a toast.
- **Store failure:** a per-account set of the last successful round's keys, held in the refresh loop, covers this. While the store fails, items absent from that set still notify once, without repeating every round. When the store recovers, items the fallback already toasted are not notified again; items it only saw silently still notify.
- **Interval:** read from its own settings block at the start of each round. The loop waits the returned duration, and the round stores an "invalid setting" flag in the snapshot for the view.

## Critical Implementation Details

- **State sequencing:** in the seen write, read the account's seeded flag and seen keys, then upsert the current list (`last_seen_ms = now`, `first_seen_ms` set only on insert), then prune only this account's rows that are absent from the current list and stale. Pruning before the upsert would forget items that are present but were last seen more than 30 days ago (after a long holiday) and re-notify them.
- **Timing & lifecycle:** do the seen write and toasts after `drop(gate)` and `publish`, so the list reaches the view first and the gate never waits on a toast. Only a round whose next snapshot is `GitlabState::Ok` with a config may notify. Reuse the `reconcile_input` predicate shape and pass the unpartitioned `full` list, not `next.items`.

## Phase 1: Seen-set store and rules

### Overview

The durable seen set with its pure rules and one store write per round, proven by unit, integration and crash tests on invented data.

### Changes Required:

#### 1. Migration v7

**File**: `src-tauri/src/store/schema.rs`

**Intent**: Append the two tables S-04 needs. It is a new migration; shipped ones stay unchanged.

**Contract**: Migration `version: 7`:
- `assignment_accounts`: `account TEXT PRIMARY KEY`, `seeded_at_ms INTEGER NOT NULL`, `WITHOUT ROWID`.
- `seen_assignments`: `account TEXT NOT NULL REFERENCES assignment_accounts(account)`, `kind TEXT NOT NULL CHECK (kind IN ('issue','mergeRequest'))`, `item_id INTEGER NOT NULL CHECK (item_id > 0)`, `first_seen_ms INTEGER NOT NULL`, `last_seen_ms INTEGER NOT NULL`, `PRIMARY KEY (account, kind, item_id)`, `WITHOUT ROWID`.

`account` is `gitlab::account_key`, never a hostname or login. The comment above the migration says so, in the style of v4 and v6.

#### 2. Pure rules

**File**: `src-tauri/src/new_assignments/rules.rs` (new), `src-tauri/src/new_assignments/mod.rs` (new), `src-tauri/src/lib.rs` (`pub mod new_assignments;`)

**Intent**: Decide, without I/O, which of the current items are new, whether this round seeds the account, and which stored rows to forget.

**Contract**:
- `ItemKey { kind: ItemKind, id: u64 }` (`Copy`, `Eq`, `Hash`, `Ord`). Its `Debug` is opaque, like `IssueRef`, so ids never reach a log.
- `FORGET_AFTER_MS: i64 = 30 days`.
- `plan_round(seeded: bool, stored: &[SeenRow], current: &[ItemKey], now_ms: i64) -> RoundPlan` with:
  - `new: Vec<ItemKey>`: empty when `!seeded`; otherwise current keys not in `stored`, deduplicated, in `current` order;
  - `seed: bool`;
  - `forget: Vec<ItemKey>`: stored keys absent from `current` with `now_ms - last_seen_ms >= FORGET_AFTER_MS`.
- Boundaries: absent for exactly 30 days is forgotten; 30 days minus 1 ms is kept. A present key is never forgotten, however old its `last_seen_ms`.

#### 3. Store operation and error

**File**: `src-tauri/src/new_assignments/mod.rs`, `src-tauri/src/new_assignments/error.rs` (new)

**Intent**: Apply a round's plan in one transaction and return the new keys.

**Contract**:
- `pub fn record_round(store: &mut Store, account: &str, current: &[ItemKey], now_ms: i64) -> Result<Vec<ItemKey>, NewAssignmentError>`, run as one `Store::write`:
  1. read the account's seeded flag and rows;
  2. `plan_round`;
  3. insert the account marker when seeding;
  4. upsert every current key;
  5. delete the `forget` keys of this account only.
- Rows of other accounts are never touched.
- `NewAssignmentError` derives `thiserror::Error` + `Serialize` with `#[serde(tag = "kind", content = "detail")]`, one variant `Store(StoreError)` plus `kind()` for logs. It carries no row values.
- A helper `keys_of(items: &AssignedItems) -> Vec<ItemKey>` takes MRs, issues and `hidden_issues` (the full list).

#### 4. Tests

**File**: `src-tauri/src/new_assignments/tests.rs` (new), `src-tauri/tests/new_assignments.rs` (new), `src-tauri/tests/new_assignments_crash.rs` (new)

**Intent**: Prove the rules and durability on invented data, reusing `tests/common` (`test_store_dir`, the crash harness in `tests/common/crash.rs`).

**Contract**:
- Unit tests cover:
  - silent seed;
  - new after seed;
  - duplicate keys;
  - an issue and an MR with the same id are distinct;
  - the forget boundary (exactly 30 d and 30 d − 1 ms);
  - a present stale key is kept;
  - the empty-list trap: a seeded account whose rows were all forgotten still notifies its next item.
- The integration test covers:
  - the new migration applied over a database at the previous latest version (written against the migration list, not literal 6/7, so a renumbering at merge needs no test edit);
  - seed, then new, then reopen `StoreHandle` on the same dir with no repeat;
  - two accounts isolated, including pruning;
  - a 31-day gap with the item present gives no repeat.
- The crash test: a writer process runs `record_round` rounds and is killed at random. Every reopened database holds either the whole of a round or none of it, and never a seeded account without that round's rows.

#### 5. App-data rule in AGENTS.md

**File**: `AGENTS.md`

**Intent**: The hard rule lists what the app data dir may hold; add seen-assignment keys so the list stays true.

**Contract**: In the "Outside folders configured in settings…" rule, extend the app data dir list with "seen-assignment keys (opaque account key, kind, id)".

### Success Criteria:

#### Automated Verification:

- Rule, store and crash tests pass: `cd src-tauri && cargo test new_assignments`
- Full Rust suite passes: `cd src-tauri && cargo test`
- `cd src-tauri && cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean

#### Manual Verification:

- Starting the dev app against an existing database migrates it to v7 without errors, and the tray still reports "Dane aplikacji" as OK

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Toasts in the refresh round

### Overview

Turn a round's new keys into 1–3 separate toasts or one combined toast, wired into the refresh thread for successful rounds only, with the session fallback for store errors.

### Changes Required:

#### 1. Notice text

**File**: `src-tauri/src/new_assignments/notice.rs` (new)

**Intent**: Build the toast titles and bodies as pure functions so their wording and limits are unit-tested.

**Contract**:
- `pub fn notices_for(new_items: &[&GitlabItem]) -> Vec<health::Notice>`:
  - 0 items give none;
  - 1–3 give one notice each, with title `Nawigator dnia — nowe zgłoszenie` (issue) or `Nawigator dnia — nowy MR` (MR) and body `<reference> — <title>`;
  - more than 3 give exactly one notice, titled `Nawigator dnia — nowe przypisania (N)`, whose body is the first 3 `<reference> — <title>` lines plus `i N−3 więcej`.
- Order: the list's order (newest `updated_at` first, as fetched).
- A title is cut to 100 characters in a separate toast and to 40 characters in a line of the combined toast (on a `char` boundary, with `…` appended), so the combined toast keeps its `i N−3 więcej` line visible. Control characters in a reference or title become spaces.
- Uses `health::APP_NAME` and `health::MAX_NOTICE_LINES` (make them `pub(crate)` if needed) instead of a second literal 3. Never reads `web_url`.

#### 2. Session fallback

**File**: `src-tauri/src/new_assignments/mod.rs`

**Intent**: Keep notifications working, without repeats, while the store fails, and avoid a duplicate when it recovers.

**Contract**:
- `pub struct SessionSeen` holds two sets per account (`HashMap<String, …>`):
  - `last`: the keys of the last successful round, for the fallback diff;
  - `toasted_unstored`: the keys the fallback toasted while the store failed, for the recovery dedup.
- `pub fn resolve(&mut self, account: &str, current: &[ItemKey], stored: Result<Vec<ItemKey>, NewAssignmentError>) -> Vec<ItemKey>`:
  - store `Ok(new)` gives `new` minus `toasted_unstored`, then clears `toasted_unstored` (those keys are stored now);
  - store `Err` with `last` present gives `current` minus `last`, and adds the result to `toasted_unstored`;
  - store `Err` with no `last` gives nothing (a silent seed for this session) and adds nothing to `toasted_unstored`.
- In every case the account's `last` becomes `current`.
- Only keys the fallback actually toasted suppress a later `Ok`. Keys it saw silently do not: an item assigned overnight while the store failed in the first round still notifies once the store recovers.

#### 3. Round wiring

**File**: `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/runtime.rs`

**Intent**: After a successful round, record the full list, then show the notices, outside every lock.

**Contract**:
- `runtime::toast` becomes `pub(crate)`.
- `refresh_loop` owns a `SessionSeen` next to `resume_after` and passes it to `refresh_round`.
- In `refresh_round`, keep the round's `keys_of(&full)` and the full items before they move into `kept_full`. After `drop(gate)` and `publish`, the notify step runs only when the next snapshot is `GitlabState::Ok` and its `account` is set. That is a `notify_input(&GitlabSnapshot) -> Option<&str>` predicate shaped like `reconcile_input`.
- The notify step:
  1. calls `record_round` via `app.state::<StoreHandle>().with_store`;
  2. resolves through `SessionSeen`;
  3. maps the keys back to items from the full list;
  4. calls `toast` for each of `notices_for(...)`.
- Logging is `log::info!("gitlab: {n} new assignments")` when n > 0. A store error is logged at debug level by `kind()` only, like `reconcile_todo` (the health round already reports the store).

#### 4. Tests

**File**: `src-tauri/src/new_assignments/tests.rs`, `src-tauri/src/gitlab_refresh.rs` (unit tests), `src-tauri/tests/new_assignments.rs`

**Intent**: Prove the wording, the 3/combined threshold, the fallback and the "successful round only" rule.

**Contract**:
- `notices_for` tests cover:
  - 0, 1, 3 and 4 items (4 give one combined notice ending in `i 1 więcej`);
  - an MR and an issue title;
  - truncation at 100 characters (separate) and 40 characters (combined line) with a multibyte character at the cut;
  - control characters;
  - no host or URL in any title or body (an invented host in `web_url` never appears).
- `SessionSeen` tests cover:
  - store ok with nothing in session;
  - store ok overlapping the session (no duplicate);
  - store error after an earlier round (only the new item);
  - store error first (silent);
  - store error first, then store ok returning an overnight key: that key notifies once;
  - store error toasts a key, then store ok returns it: no duplicate;
  - two accounts.
- `notify_input` tests cover `Ok` with and without an account, and every other state.
- The integration test runs `fetch_assigned` via fake-cli `gitlab-assigned`, then `record_round`. The first round seeds; after deleting one seen row with SQL, the next round returns exactly that key.

### Success Criteria:

#### Automated Verification:

- Notice, fallback and predicate tests pass: `cd src-tauri && cargo test new_assignments` and `cargo test gitlab_refresh`
- Full Rust suite passes: `cd src-tauri && cargo test`
- `cd src-tauri && cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean

#### Manual Verification:

- On VPN with the installed or dev build, assigning a test issue to yourself in the company GitLab shows `Nawigator dnia — nowe zgłoszenie` with its reference and title by the next round (press "Odśwież" to shorten the wait)
- Restarting the app shows no toast for items already notified
- Turning the VPN off for at least one round, then on again, gives no toast for unchanged items
- Hiding and un-hiding an issue gives no toast
- The app log contains only counts for this feature: no reference, title or host

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Refresh-interval setting

### Overview

Replace the fixed 5-minute interval with `gitlabRefresh.minutes` (1–14, default 10), read every round, with a visible notice for an invalid value.

### Changes Required:

#### 1. Settings loader

**File**: `src-tauri/src/settings.rs`

**Intent**: Read the interval from its own top-level block, so a typo there never touches the GitLab, "To do" or vault settings.

**Contract**:
- `pub const DEFAULT_REFRESH_MINUTES: u32 = 10`, `REFRESH_MINUTES_MAX: u64 = 14`. The wait starts after a round ends, so a maximum of 15 would always exceed the ≤ 15 min NFR by the round time; 14 keeps a margin.
- `pub fn load_gitlab_refresh_minutes(path: &Path) -> Result<u32, RefreshSettingInvalid>`, mirroring `load_todo_limit` for the block itself:
  - a missing file, an absent or `null` `gitlabRefresh` block, `{}` or `"minutes": null` give 10;
  - a file-level problem (unreadable file, invalid UTF-8 or JSON, a wrong `version`) also gives `Ok(10)`, not `Err`: the GitLab block already reports the file as `SettingsInvalid`, and the notice must not point at `gitlabRefresh` for an error elsewhere;
  - only a block-level problem gives `Err`: a non-object block, an unknown key (`deny_unknown_fields`), or a value that is not an integer 1..=14.
- `RefreshSettingInvalid` is a unit struct with no detail.

#### 2. Loop and snapshot

**File**: `src-tauri/src/gitlab_refresh.rs`, `src-tauri/src/gitlab/mod.rs`

**Intent**: Wait the configured interval and tell the view when the setting is ignored.

**Contract**:
- Remove `REFRESH_INTERVAL`.
- `refresh_round` reads the setting first and returns the `Duration` to wait. An `Err`, or no settings path, gives 10 min. Only an `Err` sets `refresh_setting_invalid`.
- `GitlabSnapshot` gains `refresh_setting_invalid: bool` (serde `refreshSettingInvalid`, `false` in `starting()`). It is set on every round whatever the fetch outcome, and kept by `next_snapshot` and `act_on_issue` like the other round flags.
- `refresh_loop` passes the returned duration to `recv_timeout`.

#### 3. TS mirror and view notice

**File**: `src/gitlab/types.ts`, `src/gitlab/GitlabView.tsx`

**Intent**: Show the ignored setting where the user looks at GitLab, in the same notice style as the hidden-issue notices.

**Contract**:
- `GitlabSnapshot` gains `refreshSettingInvalid: boolean`, in the same commit as the Rust change.
- The view renders `Nieprawidłowe ustawienie gitlabRefresh.minutes (dozwolone 1–14) — odświeżam co 10 min` whenever it is `true`, in data, unavailable and error states.

#### 4. Tests

**File**: `src-tauri/src/settings.rs` (unit tests), `src-tauri/src/gitlab/mod.rs` (unit tests)

**Intent**: Cover every accepted and refused shape, block isolation, and that the flag survives a failed round.

**Contract**:
- Loader cases:
  - accepted: missing file, absent block, `null`, `{}`, `minutes: null`, invalid JSON and a wrong `version` (all give 10, no flag), and 1 and 14;
  - refused: 0, 15, `"10"`, `10.5`, `true`, `-1`, an unknown key and an array block.
- A broken `gitlabRefresh` block leaves `load_gitlab_settings`, `load_todo_limit` and `load_settings` working, and a broken `gitlab` block leaves this loader working.
- `next_snapshot` keeps `refresh_setting_invalid` across an unavailable round.
- The serialized snapshot carries `refreshSettingInvalid`.

### Success Criteria:

#### Automated Verification:

- Settings and snapshot tests pass: `cd src-tauri && cargo test settings` and `cargo test gitlab::`
- Full Rust suite passes: `cd src-tauri && cargo test`
- `cd src-tauri && cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean
- Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

#### Manual Verification:

- With `"gitlabRefresh": {"minutes": 1}` the view's "Ostatnie udane odświeżenie" time advances about every minute without pressing "Odśwież"; with the block removed it advances every 10 min
- With `"gitlabRefresh": {"minutes": 99}` the "Praca (GitLab)" view shows the invalid-setting notice, the list still loads and refreshes, and fixing the value clears the notice after the next round

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- `plan_round`:
  - silent seed;
  - new keys;
  - deduplication;
  - kind collision;
  - forget boundary at exactly 30 d;
  - present stale keys kept;
  - the empty-list trap.
- `notices_for`:
  - 0, 1, 3 and 4 items;
  - truncation on a multibyte boundary;
  - control characters;
  - no host or URL.
- `SessionSeen::resolve`: store ok, store error, first-round store error, recovery after a first-round store error (overnight key notifies), recovery after a fallback toast (no duplicate), two accounts.
- `notify_input`: every `GitlabState`.
- `load_gitlab_refresh_minutes`: every accepted and refused shape; block isolation.

### Integration Tests:

- `tests/new_assignments.rs`:
  - previous-latest to new migration;
  - seed, new, no repeat after reopening the store;
  - account isolation;
  - a 31-day gap with no repeat;
  - a fake-cli `gitlab-assigned` round, then deleting a row, gives exactly one new key.
- `tests/new_assignments_crash.rs`: killing the writer mid-round never leaves a partial round.

### Manual Testing Steps:

1. On VPN, assign a test issue to yourself in the company GitLab; within one interval (or after "Odśwież") a toast shows its reference and title.
2. Restart the app: no toast for that issue.
3. Disconnect the VPN for a round, reconnect: no toast for unchanged items.
4. Hide and un-hide an issue: no toast.
5. Set `gitlabRefresh.minutes` to 1, then 99, then remove it; check the "Ostatnie udane odświeżenie" spacing and the view notice.

## Performance Considerations

Each round adds one small SQLite transaction: one row per assigned item plus one account row, indexed by its primary key. It adds no new process and no new thread. At the default of 10 min the refresh runs half as often as the current 5 min, so idle CPU stays well under the 1% NFR. Toasts go through the existing plugin only when something is new.

## Migration Notes

Migration v7 only adds tables. On the first successful round after upgrading, the current account has no marker, so its whole list is seeded silently: no burst on upgrade. Rolling back to an older build is refused by the store (a newer database), as for every migration. Restoring from `backups/` follows the AGENTS.md procedure.

S-02, S-05 and S-10 can run in parallel and may also append a migration (v5 and v6 were both renumbered at merge for this reason). If another slice lands first, renumber this migration to the next free version at merge and update its comment ("Planned as v7; renumbered to vN…"), in the style of v5 and v6.

## References

- Roadmap item: `context/foundation/roadmap.md` (S-04)
- PRD: `context/foundation/prd.md` (FR-004, NFR "Świeżość", NFR "Lekkość w tle")
- Refresh loop and round: `src-tauri/src/gitlab_refresh.rs:188-339`
- Successful-round predicate: `src-tauri/src/gitlab_refresh.rs:344-349`
- Toast helper and gate: `src-tauri/src/runtime.rs:234-241`, `src-tauri/src/health/mod.rs:200-226`, `:362-367`
- Account key: `src-tauri/src/gitlab/mod.rs:20-32`
- Settings block pattern: `src-tauri/src/settings.rs:137-187`
- Similar store slice: `context/archive/2026-11-23-hide-and-resurface/plan.md`, `src-tauri/src/hidden_issues/`
- S-01 deferral to S-04: `context/archive/2026-11-23-gitlab-assigned-list/plan.md:43`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Seen-set store and rules

#### Automated

- [x] 1.1 Rule, store and crash tests pass: `cd src-tauri && cargo test new_assignments` — 6ef10d1
- [x] 1.2 Full Rust suite passes: `cd src-tauri && cargo test` — 6ef10d1
- [x] 1.3 `cd src-tauri && cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean — 6ef10d1

#### Manual

- [ ] 1.4 Starting the dev app against an existing database migrates it to v7 without errors, and the tray still reports "Dane aplikacji" as OK

### Phase 2: Toasts in the refresh round

#### Automated

- [x] 2.1 Notice, fallback and predicate tests pass: `cd src-tauri && cargo test new_assignments` and `cargo test gitlab_refresh`
- [x] 2.2 Full Rust suite passes: `cd src-tauri && cargo test`
- [x] 2.3 `cd src-tauri && cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean

#### Manual

- [ ] 2.4 On VPN with the installed or dev build, assigning a test issue to yourself in the company GitLab shows `Nawigator dnia — nowe zgłoszenie` with its reference and title by the next round (press "Odśwież" to shorten the wait)
- [ ] 2.5 Restarting the app shows no toast for items already notified
- [ ] 2.6 Turning the VPN off for at least one round, then on again, gives no toast for unchanged items
- [ ] 2.7 Hiding and un-hiding an issue gives no toast
- [ ] 2.8 The app log contains only counts for this feature: no reference, title or host

### Phase 3: Refresh-interval setting

#### Automated

- [ ] 3.1 Settings and snapshot tests pass: `cd src-tauri && cargo test settings` and `cargo test gitlab::`
- [ ] 3.2 Full Rust suite passes: `cd src-tauri && cargo test`
- [ ] 3.3 `cd src-tauri && cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean
- [ ] 3.4 Frontend gates pass: `npm run lint`, `npm run format:check`, `npm run build`

#### Manual

- [ ] 3.5 With `"gitlabRefresh": {"minutes": 1}` the view's "Ostatnie udane odświeżenie" time advances about every minute without pressing "Odśwież"; with the block removed it advances every 10 min
- [ ] 3.6 With `"gitlabRefresh": {"minutes": 99}` the "Praca (GitLab)" view shows the invalid-setting notice, the list still loads and refreshes, and fixing the value clears the notice after the next round
