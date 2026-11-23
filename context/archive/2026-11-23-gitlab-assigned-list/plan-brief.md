# Assigned GitLab issues and MRs ("Praca (GitLab)") — Plan Brief

> Full plan: `context/changes/gitlab-assigned-list/plan.md`

## What & Why

Roadmap slice S-01, the north star: the user opens "Praca (GitLab)" and sees the open merge requests and issues assigned to them in the work GitLab. The slice proves the whole source path end to end: logged-in `glab` → list → a calm "GitLab niedostępny" state without VPN, never an empty list in place of a failure (FR-001, FR-015, US-01).

## Starting Point

F-02 left a safe, read-only runner for `glab`/`gh` with classified failures and a fake CLI for tests, but no GitLab query. Health knows only OK / not configured / error, so any failure is an alarm. The UI stacks two views with no navigation. Outside a git repo `glab api` targets gitlab.com, and on this machine `glab` is configured only for gitlab.com.

## Desired End State

The app opens on "Praca (GitLab)": two sections, "Merge requesty" and "Zgłoszenia", newest updated first, and each title opens the item in the browser. A background thread refreshes every 5 minutes and on demand. Without VPN the view shows a calm notice and keeps the last list from this session greyed out with its time. The tray stays green with a tooltip note. A logged-out `glab` is a red tray icon with a toast and an error in the view. A fixed header with navigation stays at the top.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| GitLab host source | `gitlab.host` in `settings.json`, validated, passed as `--hostname` | Explicit, independent of glab's global default, and never in the repo |
| Refresh model | Backend thread: startup, every 5 min, resume, button / view open | The tray knows about a logged-out glab with the view closed, and S-04 reuses the loop |
| Without VPN | Keep the last list (memory only), greyed, "lista z HH:MM" | You still see what you had to do; the banner keeps it from looking current |
| Layout | Two sections, MRs first, newest updated first | MRs need faster reaction and don't drown among issues |
| Open in browser | Title opens `web_url`, only https on the configured host | The list leads to the work while the app stays read-only |
| Tray for "unavailable" | Green icon, tooltip "OK (GitLab niedostępny)", no toast | Exactly "no alarm" from FR-015, still visible on hover |
| Last-refresh time | Memory only, nothing persisted | No work data or migration on disk; a time without the list says little |
| Partial results | None: one failed call fails the round | Never a half list that looks complete |

## Scope

**In scope:**
- `gitlab.host` setting (own loader, independent of the vault settings) and the `GitlabHost` newtype
- Two `glab api` GET calls (assigned open issues and MRs, paginated) with an extended read-only guard
- Pure parsing and model, URL check, snapshot rule; fetch tested with `fake-cli`
- Refresh thread, `gitlab_snapshot` / `refresh_gitlab` commands, `gitlab-changed` event
- GitLab as a health source with a calm `Unavailable` state; tooltip and toast texts
- App navigation, fixed header, `ErrorState` with last-success time, the GitLab view

**Out of scope:**
- Visibility filter (S-02), hiding (S-03), notifications (S-04), GitHub (S-05)
- MRs where the user is only a reviewer; any write to GitLab
- Persisting the list or the refresh time; a settings UI; GitLab on a custom port
- Changes to the vault health sources

## Architecture / Approach

`settings.json` → `GitlabHost` → `CliCall::GlabAssigned*` → `CliRunner` → pure `gitlab/` (parse, sort, URL check, snapshot rule) → `gitlab_refresh` thread → `GitlabState` → `gitlab-changed` event to `src/gitlab/` and a source status to the health loop (tray). Pure rules stay free of `tauri`, the same split as `health/` and `runtime.rs`. No work data ever reaches an error value, a log or the repo.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. GitLab backend contract | Setting, calls + guard, model, parsing, fetch, fake-CLI tests | A parse error leaking response content; the guard getting looser than needed |
| 2. Background refresh, source health and tray | Thread, commands, event, calm `Unavailable`, tray behaviour | Lock or timing coupling between the health and refresh threads |
| 3. Navigation, header and the view | Default "Praca (GitLab)" view in every state, navigation | A failure state rendered like an empty list |

**Prerequisites:** F-02 merged (done). With VPN on, run `glab auth login --hostname <work-host>` on Windows once and add `"gitlab": { "host": "<work-host>" }` to `settings.json` before the manual checks.
**Estimated effort:** ~3 sessions, one per phase.

## Open Risks & Assumptions

- The work GitLab supports `scope=assigned_to_me` on `/issues` and `/merge_requests` (GitLab 9+) and returns `references.full` (GitLab 12.6+). An older server fails as `ResponseInvalid`, loudly, not as an empty list.
- glab error wording may change; unknown text falls back to `Failed` (F-02 behaviour).
- A stale list can show an item that was closed or reassigned in the meantime. The greyed style and the "lista z HH:MM" banner are the mitigation.

## Success Criteria (Summary)

- With VPN on, the assigned open MRs and issues match the GitLab web UI, and each opens in the browser.
- Without VPN, a calm notice and the last list appear, never an empty list or an alarm.
- A logged-out `glab` shows up as an error in the tray and in the view, and no work data appears in logs, tests or the repo.
