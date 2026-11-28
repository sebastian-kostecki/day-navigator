# S-02 github-refresh-interval Implementation Plan

## Overview

Roadmap M-4, slice S-02, anchored to **MS-29** (issue #29; PRD refs FR-005, FR-024; no new FR). The user sets GitHub's refresh interval in `settings.json` as `githubRefresh.minutes`, with the same range (1–14) and default (10) as `gitlabRefresh.minutes` and validated in Rust by the same parser, and sees "Odświeżam co N min — następna próba za M min (HH:MM)." in GitHub's unavailable and error states, worded exactly like GitLab's text because it is the same function. A missing key keeps today's 10-minute behaviour; an invalid value gives the default interval and a warn notice in "Projekty (GitHub)", the way an invalid `gitlabRefresh.minutes` gives one in "Praca (GitLab)".

## Current State Analysis

- `gitlabRefresh.minutes` is read by `load_gitlab_refresh_minutes` / `parse_gitlab_refresh_minutes` (`src-tauri/src/settings.rs:422-460`), hard-wired to the key `"gitlabRefresh"` (`:442`). Default `DEFAULT_REFRESH_MINUTES = 10` (`:401`), max `REFRESH_MINUTES_MAX = 14` (`:404`). File-level problems give `Ok(10)`; a block-level problem gives `Err(RefreshSettingInvalid)` (`:406-410`).
- GitLab maps that result to `(minutes, invalid)` in `refresh_interval` (`src-tauri/src/gitlab_refresh.rs:515-527`), reads it at the top of every round (`:537-538`), sets the round flags on every round (`:648-649`), sets `next_refresh_at_ms` right before the store (`:700-701`) and returns the wait (`:714`).
- GitHub uses a fixed `REFRESH_INTERVAL` of 10 minutes (`src-tauri/src/github_refresh.rs:39`, returned at `:195`). `GithubSnapshot` (`src-tauri/src/github/mod.rs:232-242`) has no interval fields; it is built by struct literal only in `starting()` (`:246-256`) and `next_snapshot` (`:291-298`); every other site uses `..GithubSnapshot::starting()`. TS mirror: `src/github/types.ts:40-49`.
- The next-try text is `retryText` (module-private, `src/gitlab/GitlabView.tsx:68-81`), using `formatClock` (`src/components/time.ts:17-19`), which `GitlabView.tsx` imports only for `retryText` (`:19`).
- GitHub's calm unavailable card has no children (`src/github/GithubView.tsx:209-218`); the error state is the local `SourceError` (`:99-132`) with two `ErrorState` calls whose `message` is inline `ReactNode` (`src/components/ErrorState.tsx:6-7`). No GitHub notice for a refresh-setting problem exists.
- Full findings: `context/changes/github-refresh-interval/research.md`.

## Desired End State

- `settings.rs` has one refresh-minutes parser parameterised by the block key, behind two public loaders: `load_gitlab_refresh_minutes` (signature and behaviour unchanged) and a new `load_github_refresh_minutes`. Each reads only its own block; a broken neighbour never moves it.
- Each GitHub round reads `githubRefresh.minutes`, waits that long, and stores `refreshMinutes`, `refreshSettingInvalid` and `nextRefreshAtMs` in the snapshot. The fields are round flags, carried over by `github::next_snapshot`, and mirrored in `src/github/types.ts`.
- `retryText` lives in `src/components/time.ts`, exported from `src/components/index.ts`, used by both views. GitLab's output is byte-for-byte the same as before.
- "Projekty (GitHub)" shows the retry text in the calm "GitHub niedostępny" card and after the message in both error variants (not for kind `ThreadDown`), and a warn `Notice` naming `githubRefresh.minutes` when the setting is invalid.
- `README.md` documents `githubRefresh`.
- Verify: all six gates green; Rust unit tests listed per phase pass; the manual checks below pass in the integrated app.

### Key Discoveries:

- The GitHub round already returns a `Duration` to `source_refresh::run_loop`, and `source_refresh::next_refresh_at_ms(wait)` exists (`src-tauri/src/source_refresh.rs:206-209`), left by F-01 for this slice (`context/archive/2026-11-27-source-refresh-skeleton/change.md:14`).
- `load_github_settings` already reports an unreadable file, bad UTF-8, bad JSON and a wrong `version` as `GithubError::SettingsInvalid` (`settings.rs:157-175`), so the GitHub refresh loader may keep the same file-level `Ok(10)` rule without hiding a file problem.
- `check_settings` ignores unknown top-level keys (`settings.rs:52-55, 83-91`): `githubRefresh` needs no change there.
- `github/mod.rs` may import `crate::settings::DEFAULT_REFRESH_MINUTES`: `settings.rs` imports no `tauri`, and `gitlab/mod.rs:20` already does this.
- The `ThreadDown` stand-in (`github_refresh.rs:124`) goes through `next_snapshot` from `starting()`, so with carry-over it shows `refreshMinutes = 10`, `nextRefreshAtMs = null`; no round ever follows, so the UI shows no retry text for that kind.

## What We're NOT Doing

- No migration, no new Tauri command, no new `CliCall`, no change to `lib.rs`, `cli/`, `store/`, `AGENTS.md` (S-01 owns the wave's migration v15).
- No edit to `src-tauri/src/gitlab_refresh.rs` (S-06 edits it). GitLab keeps its own `refresh_interval`; unifying the two mappings is left for later (default chosen by agent).
- No change to GitLab behaviour or texts: the GitLab error state still has no retry text; only `retryText`'s location changes.
- No writes to `settings.json`; the app only reads it.
- No retry text for a per-repo failure under `state: ok` (that is not the source's error state) and none for `refreshError` (a failed `refresh_github` command).
- No tray, health-probe or "Źródła" report of an invalid `githubRefresh` (GitLab has none either). The pre-existing drift in `README.md:135` ("An invalid block is reported in the **Źródła** view") is not rewritten; the new README row states where the GitHub notice shows (default chosen by agent).
- No file watcher: a changed value applies from the next round (after the current wait or a wake such as "Sprawdź teraz").
- No edit to `context/foundation/roadmap.md` (forbidden for this run by the wave orchestrator; the roadmap status sync of `/10x-plan` is skipped).

## Implementation Approach

Three phases, each one commit on green gates, Rust before UI:

1. Settings: parameterise the parser by key and add the GitHub loader with tests. Nothing calls it yet.
2. Rust round + snapshot + TS mirror in one commit (AGENTS.md "Change the Rust type and its TS mirror in the same commit"): the three snapshot fields, their carry-over, the round reading the setting, and `src/github/types.ts`.
3. UI and docs: move `retryText` to `src/components/time.ts`, swap GitLab's import, render the text and the notice in `GithubView.tsx`, add the README row.

Decisions marked "default chosen by agent" were made without the user (non-interactive run); the user's binding decisions are: same range and default as GitLab; the app only reads `settings.json`; no migration; share GitLab's next-try formatting if that causes no churn in GitLab files.

## Critical Implementation Details

- **Round ordering (Phase 2).** Read the interval at the top of `refresh_round`, before `begin_round`, as GitLab does (`gitlab_refresh.rs:537-538`). Set `refresh_minutes` and `refresh_setting_invalid` on the snapshot after `next_snapshot`, whatever the fetch outcome. Set `next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait))` after `reconcile_todo` and immediately before `*state.write() = next` ("the wait starts when the snapshot is stored"), and return that same `wait`.
- **`noUnusedLocals` (Phase 3).** After `retryText` leaves `GitlabView.tsx`, `formatClock` must be removed from its import list in the same edit, or `npm run build` fails.

## Phase 1: Refresh-minutes parser by key and the GitHub loader

### Overview

One parser in `settings.rs` serves both blocks; a new public loader reads `githubRefresh.minutes`. GitLab's loader keeps its signature, behaviour and tests.

### Changes Required:

#### 1. Parser and loaders

**File**: `src-tauri/src/settings.rs`

**Intent**: Replace the hard-wired `"gitlabRefresh"` lookup with a key parameter so GitHub reuses the same validation instead of a copy (default chosen by agent, per the research recommendation). Add the GitHub loader next to the GitLab one.

**Contract**:
- Private `fn parse_refresh_minutes(text: &str, key: &str) -> Result<u32, RefreshSettingInvalid>` (renamed from `parse_gitlab_refresh_minutes`, body unchanged except `value.get(key)`).
- Private `fn load_refresh_minutes(path: &Path, key: &str) -> Result<u32, RefreshSettingInvalid>` holding today's read/BOM/UTF-8 logic.
- `pub fn load_gitlab_refresh_minutes(path: &Path) -> Result<u32, RefreshSettingInvalid>`: unchanged signature, delegates with `"gitlabRefresh"`.
- New `pub fn load_github_refresh_minutes(path: &Path) -> Result<u32, RefreshSettingInvalid>`: delegates with `"githubRefresh"`. Doc comment: a file-level problem is `Ok(10)` because the `github` block already reports the file (`load_github_settings`).
- `RawGitlabRefresh` renamed `RawRefresh` (same fields, `deny_unknown_fields`); doc comment of `RefreshSettingInvalid` names both blocks. `DEFAULT_REFRESH_MINUTES` and `REFRESH_MINUTES_MAX` are shared, unchanged.

#### 2. Tests

**File**: `src-tauri/src/settings.rs` (tests module)

**Intent**: Prove the GitHub loader has GitLab's defaults, edges and refusals, and that the two keys are independent both ways. Existing GitLab refresh tests stay as they are. Invented data only (`*.example.invalid`, `octo-test/*`).

**Contract**: new tests (a `github_refresh_text(block)` helper next to `refresh_text`):
- `github_refresh_minutes_defaults_and_file_level_problems_give_ten`: missing file, no block, `null`, `{}`, `{"minutes":null}`, bad JSON, `version` 2, no `version`, bad UTF-8, a directory → `Ok(10)`.
- `github_refresh_minutes_range_edges`: 1, 7, 14 → `Ok(n)`.
- `github_refresh_minutes_invalid_blocks_are_refused`: the same ten blocks as `refresh_minutes_invalid_blocks_are_refused` → `Err(RefreshSettingInvalid)`.
- `github_refresh_minutes_bom_is_stripped`.
- `refresh_keys_are_independent`: valid `gitlabRefresh` + invalid `githubRefresh` → GitLab `Ok(n)`, GitHub `Err`; the reverse; only `gitlabRefresh` set → GitHub `Ok(10)`; only `githubRefresh` set → GitLab `Ok(10)`.
- `broken_github_refresh_block_leaves_the_other_loaders_green`: `check_settings`, `load_github_settings`, `load_gitlab_refresh_minutes`, `load_todo_limit` stay green.
- `github_refresh_loader_ignores_a_broken_github_block`: `"github":"nope"` + `"githubRefresh":{"minutes":6}` → `Ok(6)`.

### Success Criteria:

#### Automated Verification:

- `cargo test` passes, including the new `github_refresh_*` tests, `refresh_keys_are_independent` and the unchanged GitLab `refresh_minutes_*` tests
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (the new loader has no production caller until Phase 2; `settings` is a `pub mod` in `lib.rs:16`, so the `pub fn` is not dead code and needs no `#[allow]`)
- `npm run lint`, `npm run format:check` and `npm run build` pass
- `git diff --stat` shows no change outside `src-tauri/src/settings.rs` (apart from this plan's Progress rows)

**Implementation Note**: Commit on green, e.g. `feat(settings): read githubRefresh.minutes with the shared refresh parser`.

---

## Phase 2: GitHub round uses the setting; snapshot fields and TS mirror

### Overview

The GitHub snapshot gains the three round flags, the round reads the setting and stores the next-try instant, and the TS mirror changes in the same commit.

### Changes Required:

#### 1. Snapshot fields and carry-over

**File**: `src-tauri/src/github/mod.rs`

**Intent**: Give GitHub the same round flags as GitLab so the UI can show the interval, the next try and an invalid setting. Append the fields after `seq` (small tail edit; S-01 will also append, see Merge Points).

**Contract**:
- `GithubSnapshot` gains, in this order after `seq`: `pub refresh_setting_invalid: bool` ("True when `githubRefresh.minutes` is invalid and the default interval is used. A round flag: set on every round."), `pub refresh_minutes: u32` ("The interval (minutes) the last round used. A round flag."), `pub next_refresh_at_ms: Option<u64>` ("When the wait after the last round ends (UTC ms); `None` before the first round."). Serialized camelCase: `refreshSettingInvalid`, `refreshMinutes`, `nextRefreshAtMs`.
- `starting()`: `false`, `DEFAULT_REFRESH_MINUTES` (new `use crate::settings::DEFAULT_REFRESH_MINUTES;`), `None`.
- `next_snapshot`: the literal copies all three from `prev` with a comment "Round flags, carried over: the round sets them." They survive every outcome (Ok, keeping and dropping errors).

#### 2. The round

**File**: `src-tauri/src/github_refresh.rs`

**Intent**: Replace the fixed interval with the setting, read every round so a change needs no restart.

**Contract**:
- Remove `const REFRESH_INTERVAL`.
- New private `fn refresh_interval(settings_path: &Option<PathBuf>) -> (u32, bool)`: `None` path → `(DEFAULT_REFRESH_MINUTES, false)`; `Ok(m)` → `(m, false)`; `Err(_)` → `(DEFAULT_REFRESH_MINUTES, true)`. Same shape as GitLab's (`gitlab_refresh.rs:515-527`), kept as a small private copy so `gitlab_refresh.rs` stays untouched (default chosen by agent).
- `refresh_round`: `let (minutes, refresh_invalid) = refresh_interval(settings_path); let wait = Duration::from_secs(u64::from(minutes) * 60);` at the top; after `next_snapshot`, `next.refresh_setting_invalid = refresh_invalid; next.refresh_minutes = minutes;`; right before `*state.write() = next`, `next.next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait));`; return `wait`. `next` becomes `let (mut next, next_kept)`. Ordering per "Critical Implementation Details".
- Import `load_github_refresh_minutes` and `DEFAULT_REFRESH_MINUTES` from `crate::settings`.
- No log line carries the minutes value beyond what exists (logging stays state kinds and elapsed time).

#### 3. TS mirror

**File**: `src/github/types.ts`

**Intent**: Mirror the three new fields in the same commit.

**Contract**: `GithubSnapshot` gains, after `seq`: `refreshSettingInvalid: boolean` (doc: invalid `githubRefresh.minutes`, default interval in use), `refreshMinutes: number`, `nextRefreshAtMs: number | null` (doc: `null` before the first round). Comments mirror `src/gitlab/types.ts:105-114`.

#### 4. Tests

**Files**: `src-tauri/src/github/mod.rs`, `src-tauri/src/github_refresh.rs` (tests modules)

**Intent**: Pin the defaults, the carry-over, the serialisation and the interval mapping. Invented data only.

**Contract**:
- `github/mod.rs`: extend `starting_snapshot_is_empty` (or add `starting_snapshot_has_the_default_interval`) with `refresh_minutes == DEFAULT_REFRESH_MINUTES`, `next_refresh_at_ms == None`, `!refresh_setting_invalid`.
- `github/mod.rs`: `snapshot_carries_the_refresh_interval_fields_across_rounds`: `prev` with `refresh_minutes: 3`, `next_refresh_at_ms: Some(123_456)`, `refresh_setting_invalid: true`; an `Ok` round, a keeping `Unavailable` round, a dropping `NotConfigured` round and a `ThreadDown` round all keep the three values (model: `gitlab/mod.rs:2003-2032`).
- `github/mod.rs`: `snapshot_serializes_the_interval_fields`: `refreshMinutes` 7, `nextRefreshAtMs` 42, `refreshSettingInvalid` true; `starting()` serialises `nextRefreshAtMs` as `null`, `refreshMinutes` 10, `refreshSettingInvalid` false (model: `gitlab/mod.rs:2043-2063`).
- `github_refresh.rs`: `refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes`: `None` → `(10, false)`; `githubRefresh.minutes` 3 → `(3, false)`; 15 → `(10, true)`; a file with only `gitlabRefresh` (3 or 15) → `(10, false)` (model: `gitlab_refresh.rs:1455-1465`).

### Success Criteria:

#### Automated Verification:

- `cargo test` passes, including the four tests above and all existing `github`, `github_refresh`, `health`, `todo` and `source_refresh` tests
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- `npm run lint`, `npm run format:check` and `npm run build` pass (types.ts compiles; no consumer breaks)
- `grep -n REFRESH_INTERVAL src-tauri/src/github_refresh.rs` finds nothing
- `git diff --stat` shows changes only in `src-tauri/src/github/mod.rs`, `src-tauri/src/github_refresh.rs` and `src/github/types.ts` (apart from this plan's Progress rows)

**Implementation Note**: One commit for the Rust type and its TS mirror, e.g. `feat(github): refresh on githubRefresh.minutes and expose the next try`.

---

## Phase 3: Shared next-try text in the GitHub view, notice and README

### Overview

`retryText` moves to the shared time module; GitHub renders it in its unavailable and error states and shows the invalid-setting notice; the README documents the key.

### Changes Required:

#### 1. Shared `retryText`

**Files**: `src/components/time.ts`, `src/components/index.ts`, `src/gitlab/GitlabView.tsx`

**Intent**: One copy of the next-try wording for both views (user's decision: share if no churn in GitLab files; the GitLab diff is a deletion plus an import swap, no behaviour change).

**Contract**:
- `time.ts`: `export function retryText(refreshMinutes: number, nextRefreshAtMs: number | null, refreshing: boolean, now: number): string`, moved verbatim (same four outputs) with its doc comment; it calls the module's `formatClock`.
- `index.ts`: the existing `export { formatClock } from "./time";` becomes `export { formatClock, retryText } from "./time";` (default chosen by agent: one-line change).
- `GitlabView.tsx`: delete the local `retryText` and its doc comment (`:68-81`); in the `../components` import list remove `formatClock,` and add `retryText,` (after `plural,`). The call at `:655` is unchanged.

#### 2. GitHub view

**File**: `src/github/GithubView.tsx`

**Intent**: Show the interval and next try in GitHub's unavailable and error states, and report an invalid setting the way GitLab does.

**Contract**:
- In `GithubView`, right after `nowMs`, compute `retryNote = retryText(snapshot.refreshMinutes, snapshot.nextRefreshAtMs, refreshing, nowMs)` (the existing `nowMs` and 30 s tick). Not `retry`: that name is already taken by `useGithub()`'s reload callback (`const { …, retry } = useGithub()`, used by the `loadError` state), and a second `retry` does not compile.
- Calm "GitHub niedostępny" `SourceState`: children = the retry text only (default chosen by agent: GitHub's card has no lead-in text today; none is added).
- `SourceError` gains a prop `retryNote: string | null`. Both `ErrorState` calls render the message followed by `" "` and `retryNote` when not `null` (inline, inside the existing `<p>`). `GithubView` passes `null` when `state.kind === "ThreadDown"` (default chosen by agent: no round ever comes, so "Odświeżam co 10 min." would be untrue); the `"ThreadDown"` literal gets a comment that it mirrors `GithubError::kind` in `src-tauri/src/github/mod.rs`, like `"LoggedOut"`.
- After the error state and before `TodoPicksNotices`, when `snapshot.refreshSettingInvalid`: `<Notice tone="warn">Nieprawidłowe ustawienie <code>githubRefresh.minutes</code> (dozwolone 1–14) — odświeżam co 10 min</Notice>`, with the comment that "1–14" and "10 min" mirror `REFRESH_MINUTES_MAX` and `DEFAULT_REFRESH_MINUTES` in `src-tauri/src/settings.rs` (default chosen by agent: GitLab's wording with the key swapped). Shown whatever the source state, like GitLab's.
- Import `retryText` from `../components`. No new CSS, no new resource (no CSP change).

#### 3. README

**File**: `README.md`

**Intent**: Document the new key where `gitlabRefresh` is documented (default chosen by agent).

**Contract**: add `"githubRefresh": { "minutes": 10 },` to the example after the `github` line, and a table row after `github`: `githubRefresh` — "GitHub background refresh interval in minutes, 1–14 (default 10). An invalid value shows a notice in **Projekty (GitHub)** and uses 10." Keep the table aligned (Prettier does not format README, but keep the column style).

### Success Criteria:

#### Automated Verification:

- `npm run lint`, `npm run format:check` and `npm run build` pass (no unused `formatClock` import)
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass
- `grep -rn "function retryText" src` finds exactly one definition, in `src/components/time.ts`
- `git diff --stat` shows changes only in `src/components/time.ts`, `src/components/index.ts`, `src/gitlab/GitlabView.tsx`, `src/github/GithubView.tsx` and `README.md` (apart from this plan's Progress rows)

#### Manual Verification:

These run after integration of the wave, in the app, by the user (they stay unticked until then):

- Without a `githubRefresh` key, with GitHub unreachable (network off), "GitHub niedostępny" shows "Odświeżam co 10 min — następna próba za M min (HH:MM)." and no warn notice
- With `"githubRefresh": {"minutes": 3}`, after "Sprawdź teraz" the text says "co 3 min", the HH:MM is about 3 min ahead, and the next automatic round comes about 3 min later without a restart
- An invalid `githubRefresh.minutes` (`0`, `15`, `"10"`) shows the warn notice naming `githubRefresh.minutes` and "co 10 min"; GitLab shows no refresh notice, and an invalid `gitlabRefresh.minutes` shows no GitHub notice
- In the GitHub error state (for example `"github": {"repos": "nope"}`) the retry text follows the error message; while a round runs it reads "sprawdzam teraz…"
- GitLab regression: without VPN the "GitLab niedostępny" card shows the same retry text as before, and the `gitlabRefresh` notice is unchanged

**Implementation Note**: Commit on green, e.g. `feat(github): show the refresh interval and next try in the GitHub view`.

---

## Testing Strategy

### Unit Tests:

- Settings: GitHub defaults, file-level fallbacks, range edges 1/7/14, the ten invalid blocks, BOM, independence of `gitlabRefresh` and `githubRefresh` both ways, neighbour blocks unaffected (Phase 1).
- Snapshot: `starting()` defaults, carry-over through Ok / keeping / dropping / `ThreadDown` rounds, camelCase serialisation of the three fields with `null` before the first round (Phase 2).
- Round: `refresh_interval` mapping, including that `gitlabRefresh` never moves GitHub's interval (Phase 2).
- The UI has no test runner (AGENTS.md "Testing"): the texts are manual checks.

### Integration Tests:

- None new: the round needs an `AppHandle`; its ordering is reviewed against `gitlab_refresh.rs:537-714`.

### Manual Testing Steps:

See `## Progress` → Manual. Before a dev run, back up the app data database as usual. Test values in `settings.json` are the user's own; tests in the repo use invented data only.

## Performance Considerations

One extra small read of `settings.json` per GitHub round (at most every minute); negligible.

## Migration Notes

None: no schema change, no stored data. A missing `githubRefresh` keeps the 10-minute behaviour.

## Merge Points with S-01 and S-06

Same base `develop` 00e6a19. Edits in shared files stay small and appended; no reordering or reformatting of unrelated code.

| File | S-02 (this slice) | S-01 github-hide-resurface | S-06 gitlab-review-requests | Expected merge |
| --- | --- | --- | --- | --- |
| `src-tauri/src/github/mod.rs` | 3 fields appended after `seq`; `starting()` and `next_snapshot` literals; new `use crate::settings::DEFAULT_REFRESH_MINUTES`; tests | likely appends hidden-section fields to the same struct and both literals | — | textual conflict at the struct tail and the two literal tails: keep both field sets. S-01's hide command must keep the three fields (carry-over or clone-and-modify, as GitLab's `act_on_issue`) |
| `src-tauri/src/github_refresh.rs` | drop `REFRESH_INTERVAL`; `refresh_interval` fn; first lines of `refresh_round`, `mut next`, two flag lines, `next_refresh_at_ms` before the store, `wait` return; one test; imports | adds a hidden-issue step and likely a gate around the store | — | medium: keep S-02's `next_refresh_at_ms` line immediately before the store, inside any gate S-01 adds |
| `src/github/types.ts` | 3 fields after `seq` | appends hidden fields | — | trivial tail conflict: keep both |
| `src/github/GithubView.tsx` | `retryText` import; `retry` value; calm-card children; `SourceError` `retry` prop; warn `Notice` | row hide/show buttons, "Hidden (n)" section, maybe imports | — | low to medium: different regions; the `../components` import list may need a manual merge |
| `src/gitlab/GitlabView.tsx` | delete local `retryText`; `formatClock` → `retryText` in the import list | — | "Do review" section, likely new imports | low: import list may need a manual merge; make sure `formatClock` is not re-added unused |
| `src/components/time.ts`, `src/components/index.ts` | add and export `retryText` | — | — | none known |
| `src-tauri/src/settings.rs` | parser by key, GitHub loader, tests | none expected | none expected | low |
| `README.md` | `githubRefresh` example line and table row | maybe | maybe | low: adjacent table rows |
| `src-tauri/src/gitlab_refresh.rs`, `lib.rs`, `cli/`, `store/schema.rs`, `AGENTS.md` | untouched | yes (commands, `CliCall`, migration v15, app-data list) | yes (`gitlab_refresh.rs`, `cli/`, maybe `lib.rs`) | none for S-02 |

## References

- Research: `context/changes/github-refresh-interval/research.md` (commit e51f8c8)
- F-01 hand-off: `context/archive/2026-11-27-source-refresh-skeleton/plan.md`, `change.md:14`
- GitLab model: `src-tauri/src/gitlab_refresh.rs:515-527, 537-538, 648-649, 700-714`; `src-tauri/src/gitlab/mod.rs:491-502, 531-535, 569-575, 2003-2063`; `src/gitlab/GitlabView.tsx:68-81, 644-672`
- Roadmap: `context/foundation/roadmap.md` S-02 (MS-29)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Refresh-minutes parser by key and the GitHub loader

#### Automated

- [x] 1.1 cargo test passes with the new github_refresh_* tests, refresh_keys_are_independent and the unchanged GitLab refresh_minutes_* tests — d9ac21f
- [x] 1.2 cargo fmt --check and cargo clippy --all-targets -- -D warnings pass — d9ac21f
- [x] 1.3 npm run lint, npm run format:check and npm run build pass — d9ac21f
- [x] 1.4 git diff --stat shows no change outside src-tauri/src/settings.rs — d9ac21f

### Phase 2: GitHub round uses the setting; snapshot fields and TS mirror

#### Automated

- [x] 2.1 cargo test passes with the snapshot default, carry-over, serialisation and refresh_interval tests and all existing tests — 50c8b78
- [x] 2.2 cargo fmt --check and cargo clippy --all-targets -- -D warnings pass — 50c8b78
- [x] 2.3 npm run lint, npm run format:check and npm run build pass — 50c8b78
- [x] 2.4 REFRESH_INTERVAL no longer exists in github_refresh.rs — 50c8b78
- [x] 2.5 git diff --stat shows changes only in github/mod.rs, github_refresh.rs and src/github/types.ts — 50c8b78

### Phase 3: Shared next-try text in the GitHub view, notice and README

#### Automated

- [x] 3.1 npm run lint, npm run format:check and npm run build pass — 7abae12
- [x] 3.2 cargo fmt --check, cargo clippy --all-targets -- -D warnings and cargo test pass — 7abae12
- [x] 3.3 exactly one retryText definition, in src/components/time.ts — 7abae12
- [x] 3.4 git diff --stat shows changes only in time.ts, index.ts, GitlabView.tsx, GithubView.tsx and README.md — 7abae12

#### Manual

- [x] 3.5 Without a githubRefresh key, with GitHub unreachable (network off), "GitHub niedostępny" shows "Odświeżam co 10 min — następna próba za M min (HH:MM)." and no warn notice — manual 2026-11-28
- [x] 3.6 With "githubRefresh": {"minutes": 3}, after "Sprawdź teraz" the text says "co 3 min", the HH:MM is about 3 min ahead, and the next automatic round comes about 3 min later without a restart — manual 2026-11-28
- [x] 3.7 An invalid githubRefresh.minutes (0, 15, "10") shows the warn notice naming githubRefresh.minutes and "co 10 min"; GitLab shows no refresh notice, and an invalid gitlabRefresh.minutes shows no GitHub notice — manual 2026-11-28
- [x] 3.8 In the GitHub error state (for example "github": {"repos": "nope"}) the retry text follows the error message; while a round runs it reads "sprawdzam teraz…" — manual 2026-11-28
- [x] 3.9 GitLab regression: without VPN the "GitLab niedostępny" card shows the same retry text as before, and the gitlabRefresh notice is unchanged — manual 2026-11-28
