# S-02 github-refresh-interval — Plan Brief

> Full plan: `context/changes/github-refresh-interval/plan.md`
> Research: `context/changes/github-refresh-interval/research.md`

## What & Why

MS-29 (issue #29, PRD FR-005, FR-024; no new FR): the user sets GitHub's refresh interval in `settings.json` as `githubRefresh.minutes` — same range (1–14) and default (10) as `gitlabRefresh.minutes`, validated by the same Rust parser — and sees "Odświeżam co N min — następna próba za M min (HH:MM)." in GitHub's unavailable and error states, worded exactly like GitLab's because it is the same function. GitHub is the last source on a fixed 10-minute interval with no next-try information.

## Starting Point

F-01 put both sources on `source_refresh.rs`; GitHub's round returns a `Duration` but always the fixed `REFRESH_INTERVAL` (`github_refresh.rs:39`), and `GithubSnapshot` has no interval fields. GitLab already has the parser (`settings.rs:422-460`, hard-wired to `"gitlabRefresh"`), the round flags, and a private `retryText` in `GitlabView.tsx`.

## Desired End State

A missing key behaves as today (10 min). A valid value changes GitHub's interval from the next round, no restart. An invalid value uses 10 min and shows a warn notice naming `githubRefresh.minutes` in "Projekty (GitHub)". The calm "GitHub niedostępny" card and both GitHub error variants show the shared next-try text; GitLab's view looks exactly as before.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Range and default | 1–14, default 10, shared constants | Same as GitLab | User |
| Settings file | Read only, never written | App writes only its own state | User |
| Migration | None | S-01 owns the wave's migration v15 | User |
| Next-try text | Move `retryText` to `src/components/time.ts`, export from `index.ts` | One copy; GitLab diff is a deletion and an import swap | User + Research (default chosen by agent for the location) |
| Parser reuse | Parameterise the parser by key; two thin public loaders | No copied validation; GitLab's loader signature and tests unchanged | Research (default chosen by agent) |
| GitLab round | `gitlab_refresh.rs` untouched; GitHub gets a small private `refresh_interval` | S-06 edits that file | Plan (default chosen by agent) |
| `ThreadDown` | No retry text | No round ever comes, so the text would be untrue | Research (default chosen by agent) |
| Invalid-setting report | Warn `Notice`, GitLab's wording with `githubRefresh.minutes` | "Reported the way GitLab's is" | Research (default chosen by agent) |
| Calm card body | Retry text only, no new lead-in sentence | Smallest change; GitHub's card has no text today | Plan (default chosen by agent) |
| README | Example line and table row for `githubRefresh`; `README.md:135` drift left alone | Document where `gitlabRefresh` is documented | Research (default chosen by agent) |
| Roadmap status sync | Skipped | Editing `roadmap.md` is forbidden for this run | Orchestrator |

## Scope

**In scope:**
- `githubRefresh.minutes` loader and tests (independence of both keys, defaults, edges, invalid)
- Three round flags on `GithubSnapshot` + carry-over + TS mirror; the round reads the setting and stores the next-try instant
- Shared `retryText`; GitHub calm card, error states, warn notice; README

**Out of scope:**
- Migration, commands, `CliCall`, `lib.rs`, `gitlab_refresh.rs`, tray / health / "Źródła" reporting
- Retry text in GitLab's error state, for per-repo failures, or for `refreshError`
- File watching (changes apply from the next round)

## Architecture / Approach

`settings.rs` parses the block (`parse_refresh_minutes(text, key)`); `github_refresh.rs` maps it to `(minutes, invalid)` at the top of each round, sets the flags after `next_snapshot`, sets `next_refresh_at_ms` right before the store and returns the wait; `github/mod.rs` owns the fields and carry-over; `GithubView.tsx` renders `retryText` from `src/components/time.ts`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Parser by key + GitHub loader | `load_github_refresh_minutes` with tests | Accidentally changing GitLab's parse behaviour (its tests stay unchanged) |
| 2. Round, snapshot fields, TS mirror | GitHub waits the configured interval and exposes the next try | Ordering of `next_refresh_at_ms` vs the store; merge with S-01 on the same struct |
| 3. Shared text, GitHub view, README | User-visible text and notice | Unused `formatClock` import in GitLab; import-list merge with S-01/S-06 |

**Prerequisites:** F-01 merged (base `develop` 00e6a19); research at e51f8c8.
**Estimated effort:** ~1 session, 3 small phases, one commit each.

## Open Risks & Assumptions

- S-01 appends fields to the same `GithubSnapshot` and literals and may gate the store in `refresh_round`: expect tail conflicts; S-01's hide command must keep the three fields.
- Manual checks need GitHub unreachable (network off) and a deliberately broken `github` block; they run after integration.

## Success Criteria (Summary)

- With `"githubRefresh": {"minutes": 3}` GitHub refreshes every ~3 min and the card says so with the next HH:MM.
- An invalid value gives the warn notice and 10 min; GitLab is unaffected and its text is unchanged.
- All six gates green after each phase.
