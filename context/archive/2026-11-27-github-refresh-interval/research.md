---
date: 2026-11-27T16:54:20+01:00
researcher: Claude (Opus 5.5, research subagent)
git_commit: 00e6a19b6653f988e73b4c7744f9dbb5050a6981
branch: slice/github-refresh-interval
repository: day-navigator
topic: "S-02 github-refresh-interval: githubRefresh.minutes and the next-try text in GitHub's unavailable and error states"
tags: [research, codebase, github_refresh, source_refresh, settings, github-view, gitlab-view]
status: complete
last_updated: 2026-11-27
last_updated_by: Claude (Opus 5.5, research subagent)
---

# Research: S-02 github-refresh-interval

**Date**: 2026-11-27T16:54:20+01:00
**Researcher**: Claude (Opus 5.5, research subagent)
**Git Commit**: 00e6a19b6653f988e73b4c7744f9dbb5050a6981 (base `develop`, wave 1 merged)
**Branch**: slice/github-refresh-interval
**Repository**: day-navigator

## Research Question

Roadmap M-4, S-02 (anchor MS-29, issue #29, PRD refs FR-005, FR-024): the user sets GitHub's refresh interval in `settings.json` as `githubRefresh.minutes`, with the same range and default as `gitlabRefresh.minutes` and validated in Rust the same way, and sees "Odświeżam co N min — następna próba za M min (HH:MM)" in GitHub's unavailable and error states, worded exactly like GitLab's text. A missing key keeps today's behaviour; an invalid value is reported the way an invalid `gitlabRefresh.minutes` is. Binding user decisions: same range and default as GitLab; the app only reads `settings.json`; no migration; reuse GitLab's next-try formatting instead of copying it, if it can be shared without churn in GitLab files.

Questions answered below: how `gitlabRefresh.minutes` is parsed, validated, defaulted and surfaced; how GitLab computes and exposes `refreshMinutes` / `nextRefreshAtMs`; where the GitLab next-try text lives and whether it can be shared; how `github_refresh.rs` uses its fixed interval and what its snapshot and TS mirror look like; how settings changes reach the loops; the GitHub view's unavailable and error states; tests to extend; merge points with S-01 and S-06.

Scope note: all areas were located files, so the research ran locally without sub-agents (default chosen by agent). Parallel slices' worktrees were not read; S-01/S-06 overlap comes from `context/foundation/roadmap.md` and the orchestrator brief.

## Summary

- **Setting (Rust).** `gitlabRefresh.minutes` is read by `load_gitlab_refresh_minutes` (`src-tauri/src/settings.rs:422-433`) and `parse_gitlab_refresh_minutes` (`settings.rs:435-460`). Default `DEFAULT_REFRESH_MINUTES = 10` (`settings.rs:401`), range `1..=REFRESH_MINUTES_MAX` with max 14 (`settings.rs:404`, check at `:456`). A file-level problem (unreadable, bad UTF-8, bad JSON, `version` ≠ 1), an absent or `null` block, `{}` and `"minutes": null` give `Ok(10)`; a non-object block, an unknown key (`deny_unknown_fields`, `:412-417`) or a value that is not an integer in 1..=14 gives `Err(RefreshSettingInvalid)` (unit struct with no detail, `:409-410`). The parser is hard-wired to the key `"gitlabRefresh"` (`:442`), so GitHub needs either a parameterised parser or a second copy.
- **Round (GitLab).** `refresh_interval` (`src-tauri/src/gitlab_refresh.rs:518-527`) maps the loader result to `(minutes, invalid)`: `Err` gives `(10, true)`, no settings path gives `(10, false)`. It runs at the top of every round (`:537-538`), so a change applies from the next round without a restart. The round writes `refresh_setting_invalid` and `refresh_minutes` on every round, whatever the fetch outcome (`:648-649`), and sets `next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait))` right before the snapshot store (`:700-702`). The round returns `wait` to `source_refresh::run_loop` (`:714`, `source_refresh.rs:214-223`).
- **Snapshot (GitLab).** `GitlabSnapshot` has `refresh_setting_invalid: bool`, `refresh_minutes: u32`, `next_refresh_at_ms: Option<u64>` (`src-tauri/src/gitlab/mod.rs:491-502`). `starting()` sets them to `false`, `DEFAULT_REFRESH_MINUTES`, `None` (`:531-535`). `next_snapshot` carries all three over from `prev` as round flags (`:569-575`). TS mirror: `src/gitlab/types.ts:105-114`.
- **Next-try text (UI).** `retryText(refreshMinutes, nextRefreshAtMs, refreshing, now)` is a module-private function in `src/gitlab/GitlabView.tsx:68-81`. It uses `formatClock` (`src/components/time.ts:17-19`), and `formatClock` is imported into `GitlabView.tsx` (`:19`) only for `retryText` (`:80`). Its one call is in the calm "GitLab niedostępny" card (`:644-657`, call at `:655`); the GitLab error state (`:658-665`) does not show it. The invalid-setting notice is a `Notice tone="warn"` (`:666-672`).
- **Sharing.** Moving `retryText` into `src/components/time.ts` and exporting it from `src/components/index.ts` keeps one copy. In `GitlabView.tsx` that changes three spots: delete `:68-81`, drop `formatClock,` from the import (`:19`, otherwise `noUnusedLocals` fails), add `retryText,` to the same import list. No GitLab behaviour, test or Rust file changes.
- **GitHub today.** `github_refresh.rs` has `const REFRESH_INTERVAL: Duration = Duration::from_secs(10 * 60)` (`:39`), and `refresh_round` returns it (`:195`). The round never reads an interval setting and never sets a next-try instant. `GithubSnapshot` (`src-tauri/src/github/mod.rs:232-242`) has `state, repos, last_success_ms, checked_at_ms, refreshing, seq` and nothing else. The TS mirror `src/github/types.ts:40-49` has the same six fields.
- **F-01 hand-off.** The archived F-01 plan (`context/archive/2026-11-27-source-refresh-skeleton/plan.md:40, 321`, `change.md:14`) left S-02 three jobs: add `refreshMinutes`, `nextRefreshAtMs`, `refreshSettingInvalid` to `GithubSnapshot` with their carry-over in `github::next_snapshot`; add the setting; add the TS type. It also left two tools: GitHub's round already returns a `Duration`, and `source_refresh::next_refresh_at_ms(wait)` exists (`source_refresh.rs:206-209`).
- **GitHub view.** The unavailable card (`src/github/GithubView.tsx:209-218`) is a calm `SourceState` with no children. The error state is the local `SourceError` (`:99-132`): a `LoggedOut` variant with a fragment message and a generic one with `state.message`, both through `ErrorState`, whose `message` is a `ReactNode` rendered inside a `<p>` (`src/components/ErrorState.tsx:6-7, 23`). So the retry text fits as children of the calm card and appended to `message` in both `ErrorState` calls. No GitHub `Notice` exists today for a settings problem.
- **No migration, no new command, no CLI change.** The slice touches settings parsing, one snapshot type and its mirror, one round, one view and one shared UI helper.

## Detailed Findings

### 1. `gitlabRefresh.minutes`: parse, validate, default, surface

- Constants: `DEFAULT_REFRESH_MINUTES: u32 = 10` (`src-tauri/src/settings.rs:401`), `REFRESH_MINUTES_MAX: u64 = 14` with a comment that 15 would exceed a 15-minute limit because the wait starts after the round (`:402-404`).
- Error type: `pub struct RefreshSettingInvalid;` (`:406-410`). It is not serialized and not a Tauri error; it never reaches the UI as a value. Only the boolean `refresh_setting_invalid` does.
- Raw block: `RawGitlabRefresh { #[serde(default)] minutes: Option<serde_json::Value> }` with `deny_unknown_fields` (`:412-417`). `minutes` is taken as a raw JSON value and checked with `as_u64()` plus the range (`:453-458`), so `"10"`, `10.5`, `true`, `-1`, `0`, `15` are refused.
- Loader: `load_gitlab_refresh_minutes(path)` (`:422-433`) reads the file, strips a BOM, and gives `Ok(10)` for an unreadable file or bad UTF-8. `parse_gitlab_refresh_minutes` (`:435-460`) gives `Ok(10)` for bad JSON, a `version` other than 1, an absent key or `null`; `Err` for a non-object; `Ok(10)` for `minutes` absent or `null`. The doc comment says the file-level fallback exists because "the GitLab block already reports the file" (`:419-421`).
- For GitHub the same fallback holds: `load_github_settings` (`:157-168`) reports an unreadable file as `GithubError::SettingsInvalid` (`:161`) and bad UTF-8, bad JSON and a wrong `version` the same way (`:166`, `:172-175`); a missing file is `Ok(None)` → `NotConfigured` (`:160`). So a GitHub refresh loader with the same file-level `Ok(10)` rule never hides a file problem.
- `check_settings` ignores unknown top-level keys whatever their shape (`:52-55`, `:83-91`), so a new `githubRefresh` key needs no change there.
- Round mapping: `refresh_interval(&Option<PathBuf>) -> (u32, bool)` (`src-tauri/src/gitlab_refresh.rs:515-527`).
- Surface in the UI: `GitlabSnapshot.refresh_setting_invalid` (`src-tauri/src/gitlab/mod.rs:491-493`) → TS `refreshSettingInvalid` (`src/gitlab/types.ts:105-106`) → `Notice tone="warn"` with the text "Nieprawidłowe ustawienie `gitlabRefresh.minutes` (dozwolone 1–14) — odświeżam co 10 min" (`src/gitlab/GitlabView.tsx:666-672`). The comment on `:666` says "1–14" and "10 min" mirror `REFRESH_MINUTES_MAX` and `DEFAULT_REFRESH_MINUTES`.
- Not surfaced elsewhere: on the inspected paths, `load_gitlab_refresh_minutes` has one production caller (`gitlab_refresh.rs:520`), and the health probe reads the file check (`check_settings`, `src-tauri/src/health/probe.rs:17`) and the reminder settings (`:144`), not the refresh block. The tray and "Źródła" do not show an invalid refresh block. `README.md:135` says "An invalid block is reported in the **Źródła** view", which does not hold for `gitlabRefresh` on these paths (pre-existing doc drift, outside this slice unless the plan chooses to touch it).

### 2. GitLab `refreshMinutes` / `nextRefreshAtMs`

- Fields and doc comments: `refresh_minutes` "The interval (minutes) the last round used. A round flag: set on every round." and `next_refresh_at_ms` "When the wait after the last round ends (UTC ms); `None` before the first round." (`src-tauri/src/gitlab/mod.rs:499-502`).
- `starting()`: `refresh_minutes: DEFAULT_REFRESH_MINUTES`, `next_refresh_at_ms: None`, `refresh_setting_invalid: false` (`:531-535`). `DEFAULT_REFRESH_MINUTES` comes from `use crate::settings::DEFAULT_REFRESH_MINUTES;` (`:20`); the rules module may import `settings` because `settings.rs` imports no `tauri` (`settings.rs:1`).
- `next_snapshot` carries the three over from `prev` (`:569-575`). In GitLab's round that is overwritten right after (`gitlab_refresh.rs:648-649, 701`). The carry-over matters for snapshots built outside the round: the `ThreadDown` stand-in (`gitlab_refresh.rs:461`) and the hidden-issue commands (test `act_on_issue_keeps_the_refresh_setting_flag`, `gitlab_refresh.rs:1437-1453`).
- Timing: `next.next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait))` sits after the "To do" reconcile and right before `*state.write() = next` (`gitlab_refresh.rs:698-702`); comment "The wait starts when the snapshot is stored." The helper is `now_ms().saturating_add(wait.as_millis() as u64)` (`source_refresh.rs:206-209`). The real wait starts a little later, when `run_loop` calls `recv_timeout` after the round returns (`source_refresh.rs:214-217`); the gap is the publish, the toasts and `end_round`.
- Serialization: camelCase `refreshMinutes`, `nextRefreshAtMs` (test `snapshot_serializes_the_filter_and_the_interval_fields`, `gitlab/mod.rs:2043-2063`; full JSON in the test at `:2079`).

### 3. The next-try text and sharing it

- Exact helper (`src/gitlab/GitlabView.tsx:68-81`):
  - `every = "Odświeżam co ${refreshMinutes} min"`;
  - `refreshing` → `"${every} — sprawdzam teraz…"`;
  - `nextRefreshAtMs === null` → `"${every}."`;
  - `left <= 0` → `"${every} — następna próba lada chwila."`;
  - else `"${every} — następna próba za ${Math.ceil(left / 60_000)} min (${formatClock(nextRefreshAtMs)})."`
- `formatClock(ms)` is local `HH:MM` (`src/components/time.ts:16-19`), exported on its own line in `src/components/index.ts` (`export { formatClock } from "./time";`).
- The countdown ticks with `useNow(RELATIVE_TICK_MS)` (`GitlabView.tsx:568`; `RELATIVE_TICK_MS = 30_000`, `src/components/useNow.ts:4`) and uses `nowMs = Math.max(now, lastSuccessMs ?? 0)` (`:594`). `GithubView` already has the same pair (`src/github/GithubView.tsx:137, 163`).
- GitLab call site: only the calm unavailable card (`GitlabView.tsx:644-657`), after the fixed VPN text. The GitLab error state (`:658-665`) has no retry text. For GitHub the slice asks for it in the error state too, so GitHub goes one state further than GitLab with the same wording.
- Sharing options (decision for the plan):
  - **A, recommended:** move `retryText` unchanged into `src/components/time.ts` (the "shared time formatting for the views" module, `time.ts:1`) and export it from `src/components/index.ts`. GitLab diff: delete `GitlabView.tsx:68-81`, remove `formatClock,` from the import (`:19`), add `retryText,` to the import list (`:2-23`). About 16 lines, no behaviour change.
  - B: export `retryText` from `GitlabView.tsx` and import it into `GithubView.tsx`. This avoids deleting GitLab lines but makes one feature import another's screen file. AGENTS.md puts shared UI in `src/components/` (AGENTS.md "Project Structure"), so B goes against the convention.
  - C: copy it. The user ruled this out unless sharing causes churn; option A's churn is small.

### 4. GitHub refresh today (`github_refresh.rs`, `github/mod.rs`, `src/github/`)

- Interval: `REFRESH_INTERVAL` (`src-tauri/src/github_refresh.rs:39`), returned at the end of `refresh_round` (`:195`). `run_loop` uses the returned `Duration` (`:128-131`).
- Round order (`:163-196`): `before` + `before_status` → `begin_round` → `run_fetch` (re-reads `load_github_settings` every round, `:144-161`) → `next_snapshot` with `kept_config` → `kept_config` store → status, labels → `reconcile_todo` → `*state.write() = next` (`:188`) → `publish` → `notify_todo_changed` → `end_round`. The point that matches GitLab's `next_refresh_at_ms` placement is just before `:188`.
- Snapshot: `GithubSnapshot` (`src-tauri/src/github/mod.rs:232-242`), built by struct literal in two places only: `starting()` (`:244-256`) and `next_snapshot` (`:291-298`). Every other construction on the inspected paths uses `..GithubSnapshot::starting()`: `github_refresh.rs:239-244`, `health/probe.rs:302-305, 347-350`, `todo/commands.rs:244-251`. New fields therefore need edits only in those two literals.
- `next_snapshot` (`github/mod.rs:284-330`) builds a fresh snapshot and keeps `repos` and `last_success_ms` only while the repo list is unchanged. Adding the three fields means carrying them over from `prev` in the literal at `:291-298`, as GitLab does.
- `ThreadDown` stand-in: `|prev| github::next_snapshot(prev, None, None, Err(GithubError::ThreadDown), now_ms()).0` (`github_refresh.rs:124`). With carry-over, a thread that never starts shows the `starting()` values: `refreshMinutes = 10`, `nextRefreshAtMs = null`.
- TS mirror: `src/github/types.ts:40-49` (header comment `:1` names the Rust source). The only other TS consumer of `GithubSnapshot` is `src/todo/TodoView.tsx:18, 144` via `useGithub({ refreshOnMount: false })`. It reads fields, never builds the type, so new fields do not break it.

### 5. How settings changes reach the loops

- Both rounds re-read `settings.json` at the start of each round: GitLab's interval and filter (`gitlab_refresh.rs:537-539`) and config (`:493-513`); GitHub's config (`github_refresh.rs:144-161`). There is no file watcher (no `notify`/`watch` crate in `src-tauri/Cargo.toml` on inspection).
- A changed interval applies from the round after the current wait ends, or at once when something wakes the thread: "Check now" (`refresh_github`, `github_refresh.rs:96-101`), the tray "check" item (`src-tauri/src/runtime.rs:261-265`), `check_health_now` (`runtime.rs:126-139`), resume after sleep (`runtime.rs:509-516`), or the view's mount refresh (`src/github/useGithub.ts:44-50`). The settings path is fixed at startup (`src-tauri/src/lib.rs:164, 179-180`).
- Default chosen by agent: GitHub reads `githubRefresh.minutes` at the top of each round, as GitLab does. That keeps "a change needs no restart" and is what `gitlab_refresh.rs:515-517` documents.

### 6. GitHub view: unavailable and error states

- Unavailable: calm `SourceState` "GitHub niedostępny", `lastOk`, action "Sprawdź teraz", `actionDisabled={refreshing}`, no children (`src/github/GithubView.tsx:209-218`). `SourceState` renders children inside `<p className="dn-state-text">` (`src/components/SourceState/SourceState.tsx:49`).
- Error: `SourceError` (`GithubView.tsx:99-132`), called at `:219-221`. `LoggedOut` → `ErrorState` "GitHub: brak sesji" with a fragment message; other kinds → `ErrorState` "Błąd GitHuba" with `state.message`. `ErrorState` takes `message: ReactNode`, "inline content only… rendered inside a `<p>`" (`src/components/ErrorState.tsx:6-7`). The retry text can be appended after a space to both messages. `SourceError` would need `refreshMinutes`, `nextRefreshAtMs`, `refreshing` and `nowMs` (or a ready string) as new props.
- Error kinds that reach this state: every `GithubError` except `NotConfigured` and `Unavailable` (`github/mod.rs:313-322`): `SettingsInvalid`, `LoggedOut`, `ToolMissing`, `Failed`, `ResponseInvalid`, `Truncated`, `ConfigDirUnresolved`, `ThreadDown` (`github/mod.rs:41-61`). For `ThreadDown` no round will come, and `retryText` with `nextRefreshAtMs = null` prints "Odświeżam co 10 min.", which would be untrue. See Open Questions.
- A per-repo failure under `state: ok` is not the error state: the repo panel shows its own `Notice` (`GithubView.tsx:84-87`) and the tray shows "RepoFailed" (`health/probe.rs:106-118`). The slice wording ("unavailable and error states") does not cover it on this reading.
- Invalid-setting report: GitHub has no counterpart of GitLab's `refreshSettingInvalid` notice yet. "Reported the way an invalid `gitlabRefresh.minutes` is" means a `Notice tone="warn"` in "Projekty (GitHub)" naming `githubRefresh.minutes`, shown whatever the state, with the default interval in use. `GithubView.tsx` already imports `Notice` (`:5`).
- The loading screen (`GithubView.tsx:152-159`) shows while the state is `starting` with no repos, so the next-try text never shows before the first round.

### 7. Tests to extend (all invented data)

- `settings.rs` tests for the GitLab loader, to mirror for `githubRefresh`: defaults and file-level fallbacks (`:1391-1418`), range edges 1/7/14 (`:1420-1427`), invalid blocks (`:1429-1450`), BOM (`:1452-1460`), a broken block leaving the other loaders green (`:1462-1475`), the loader ignoring a broken neighbour (`:1477-1482`). New cases worth adding: a broken `gitlabRefresh` leaves `githubRefresh` valid and the reverse.
- `gitlab_refresh.rs::refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes` (`:1455-1465`) is the model for a GitHub interval test.
- `github/mod.rs` tests: `starting_snapshot_is_empty` (`:565-572`), `seq_always_increases` (`:702-714`), `snapshot_serializes_camel_case_without_repo_names_in_errors` (`:716-738`). Models for the new fields: `gitlab/mod.rs:2003-2032` (carry-over across a keeping and a dropping round, plus `starting()` defaults) and `:2043-2063` (camelCase serialization, `null` before the first round).
- `source_refresh.rs` has `next_refresh_at_ms_is_now_plus_the_wait` (`:325-331`) and `begin_refresh_marks_a_github_snapshot_refreshing` (`:309-315`); the helper needs no new test.
- The UI has no test runner (AGENTS.md "Testing"), so the view texts are manual checks.

### 8. Merge points with S-01 and S-06 (same base `00e6a19`)

From `context/foundation/roadmap.md:103-115, 166-176` and the orchestrator brief:

| File | S-02 (this slice) | S-01 github-hide-resurface | S-06 gitlab-review-requests | Risk |
| --- | --- | --- | --- | --- |
| `src-tauri/src/github/mod.rs` | append 3 fields to `GithubSnapshot`, `starting()`, `next_snapshot` literal; tests | likely appends hidden-section fields to the same struct and literals | — | textual conflict at the struct tail and both literals; resolve by keeping both field sets |
| `src-tauri/src/github_refresh.rs` | drop `REFRESH_INTERVAL`, read the interval at the top of the round, set `next_refresh_at_ms` before the store, return `wait`; one test | adds a hidden-issue round, likely a gate like GitLab's around the store | — | medium; S-02's edits are the first lines and the store line of `refresh_round` |
| `src/github/types.ts` | append 3 fields | appends hidden fields | — | trivial tail conflict |
| `src/github/GithubView.tsx` | children for the calm card, retry text in `SourceError`, a warn `Notice` | hide/show buttons in rows, "Hidden (n)" section | — | low to medium; different regions |
| `src/components/time.ts`, `index.ts` | add and export `retryText` | — | — | none known |
| `src/gitlab/GitlabView.tsx` | delete `:68-81`, swap `formatClock` for `retryText` in the import | — | "Do review" section, likely new imports | low; import list may need a manual merge |
| `src-tauri/src/settings.rs` | GitHub refresh loader (parameterised or second copy) and tests | none expected (login from `gh api user`, roadmap `:110-111`) | none expected | low |
| `src-tauri/src/gitlab_refresh.rs` | none if GitHub gets its own mapping; one line if GitLab delegates to a shared helper | — | second MR fetch in the round | avoid touching it |
| `README.md` | `githubRefresh` in the example and the table (`:110, 127`) | maybe | maybe | low |
| `lib.rs`, `cli/`, `store/schema.rs`, `AGENTS.md` | none | yes (commands, `CliCall`, migration v15, app-data list) | yes (`cli/`, maybe `lib.rs`) | none for S-02 |

S-01's future hide command will rebuild the GitHub snapshot outside the round; with carry-over in `next_snapshot`, or a clone-and-modify like GitLab's `act_on_issue`, the three new fields survive it. S-01's planner should keep that (GitLab model: `gitlab_refresh.rs:1437-1453`).

## Code References

- `src-tauri/src/settings.rs:401-460` - refresh-minutes constants, error type, raw block, loader and parser (hard-wired to `"gitlabRefresh"` at `:442`)
- `src-tauri/src/settings.rs:157-203` - `load_github_settings`; file-level problems are `SettingsInvalid`
- `src-tauri/src/settings.rs:1387-1482` - GitLab refresh-minutes tests to mirror
- `src-tauri/src/gitlab_refresh.rs:515-527` - `refresh_interval` mapping to `(minutes, invalid)`
- `src-tauri/src/gitlab_refresh.rs:537-538, 648-649, 698-702, 714` - interval read, round flags, `next_refresh_at_ms` before the store, returned wait
- `src-tauri/src/gitlab_refresh.rs:1455-1465` - interval test
- `src-tauri/src/gitlab/mod.rs:491-502, 531-535, 569-575` - GitLab snapshot fields, defaults, carry-over
- `src-tauri/src/gitlab/mod.rs:2003-2063` - carry-over and serialization tests
- `src-tauri/src/source_refresh.rs:206-223` - `next_refresh_at_ms` helper and `run_loop`
- `src-tauri/src/github_refresh.rs:39, 124, 163-196` - fixed interval, `ThreadDown` stand-in, round
- `src-tauri/src/github/mod.rs:232-256, 284-330` - `GithubSnapshot`, `starting()`, `next_snapshot`
- `src/gitlab/GitlabView.tsx:19, 68-81, 644-672` - `formatClock` import, `retryText`, calm card, error state, invalid-setting notice
- `src/gitlab/types.ts:105-114` - TS mirror of the GitLab interval fields
- `src/github/types.ts:40-49` - TS mirror of `GithubSnapshot`
- `src/github/GithubView.tsx:99-132, 161-221` - `SourceError`, view state, calm card, error call
- `src/components/time.ts:16-19`, `src/components/index.ts` - `formatClock` and its export
- `src/components/ErrorState.tsx:4-26`, `src/components/SourceState/SourceState.tsx:12-65` - slots for the retry text
- `README.md:107-135` - settings example, block table and the invalid-block sentence

## Architecture Insights

- The rules modules (`github/`, `gitlab/`) stay free of `tauri` and own the snapshot shape and its carry-over. The `*_refresh.rs` modules own the setting reads and the side effects. `source_refresh.rs` owns the loop and the instant helper. S-02 fits this split: the setting read and the `next_refresh_at_ms` call go in `github_refresh.rs`, the fields and their carry-over in `github/mod.rs`, the parser in `settings.rs`.
- Round flags (`refresh_minutes`, `next_refresh_at_ms`, `refresh_setting_invalid`) are set on every round whatever the fetch outcome and carried over by `next_snapshot`. Item-bound fields are reset. GitHub should use the same rule.
- Settings loaders read their block on their own, and a broken neighbour block never affects them (tests `settings.rs:985-1008, 1462-1482`). The GitHub refresh loader should keep that independence from both `github` and `gitlabRefresh`.
- Option for avoiding a second loop of interval code: a small generic helper (for example `source_refresh::refresh_interval(settings_path, loader)`, or a `parse_refresh_minutes(text, key)` in `settings.rs` behind two thin public loaders). Which one, and whether GitLab is switched to it, is a plan decision. The low-conflict path leaves `gitlab_refresh.rs` untouched because S-06 edits it.

## Historical Context (from prior changes)

- `context/archive/2026-11-27-source-refresh-skeleton/change.md:14` - supported: F-01 hands S-02 the `GithubSnapshot` fields (`refreshMinutes`, `nextRefreshAtMs`, `refreshSettingInvalid`), their carry-over and the TS type.
- `context/archive/2026-11-27-source-refresh-skeleton/plan.md:40, 321, 343, 348` - supported: GitHub's round returns `Duration` (still `REFRESH_INTERVAL`) and `next_refresh_at_ms` exists for S-02 (`github_refresh.rs:195`, `source_refresh.rs:206-209`).
- `context/archive/2026-11-27-source-refresh-skeleton/plan-brief.md:87` - partial: "S-02 can … change only `github_refresh.rs`, `github/mod.rs`, settings and the TS mirror". Holds for Rust and types; the UI also needs `GithubView.tsx`, a shared helper in `src/components/`, and the small GitLab import change if `retryText` is shared.
- `context/archive/2026-11-27-source-refresh-skeleton/research.md:165` - supported: a generic interval reader over a block name is an S-02 choice.
- `context/archive/2026-11-24-gitlab-filter-bar-retry/plan-brief.md:42` - supported: that change introduced `refresh_minutes`, `next_refresh_at_ms` and `retryText` in the calm card, ticking with `useNow(RELATIVE_TICK_MS)`; current code matches (`GitlabView.tsx:568, 655`).

## Related Research

- `context/archive/2026-11-27-source-refresh-skeleton/research.md`
- `context/archive/2026-11-27-gitlab-offline-last-list/research.md` (the calm GitLab card with the last list, same view region)

## Open Questions

1. **`ThreadDown` in the error state.** With the thread down, no round comes, but `retryText` would print "Odświeżam co 10 min." (`nextRefreshAtMs = null`). Suggested default: no retry text when `kind === "ThreadDown"` (the kind is already compared in the view, `GithubView.tsx:108-109`). For the plan to settle.
2. **Shared interval helper scope.** Whether GitLab's `refresh_interval` / parser moves onto a shared helper in this slice (small edits in `gitlab_refresh.rs` / `settings.rs`, merge risk with S-06) or only GitHub uses a new parameterised path. Recommended: parameterise the parser in `settings.rs` (GitLab's public loader signature and tests unchanged), and keep `gitlab_refresh.rs` untouched.
3. **README.** Adding `githubRefresh` to the example and table (`README.md:110, 127`) seems in scope; correcting the "Źródła" sentence (`README.md:135`) for refresh blocks is optional.
4. **Notice wording.** Suggested: "Nieprawidłowe ustawienie `githubRefresh.minutes` (dozwolone 1–14) — odświeżam co 10 min", the GitLab notice with the key swapped (`GitlabView.tsx:668-671`).
