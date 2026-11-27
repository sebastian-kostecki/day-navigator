---
project: "Nawigator dnia"
version: 8
status: draft
created: 2026-11-20
updated: 2026-11-27
prd_version: 2
main_goal: quality
top_blocker: decisions
milestone_id: sources-github-parity
milestone_seq: 4
milestone_status: open
---

# Roadmap: Nawigator dnia

> Derived from GitHub issues #28–#35 (GitHub milestone "M-4: Źródła – GitHub na równi z GitLabem"), `context/foundation/prd.md` (v2) + the codebase baseline.
> Edit-in-place; the next milestone continues this file. Archive only when an independent roadmap replaces it.
> Slices below are listed in dependency order. The "At a glance" table is the index.
> M-3's decomposition (S-01–S-08, all done) was replaced in place by M-4; M-1–M-3 are recorded in `## Milestone History` and their archived changes in `## Done`.

## Milestone

**M-4: Sources — GitHub on a par with GitLab** — Status: open

- **Intent:** "Projects (GitHub)" behaves like "Work (GitLab)": a refresh interval from settings with the next try shown, a NEW flag and new-assignment notices, hiding with resurfacing on change, all on one shared refresh mechanism for both sources. "Work (GitLab)" closes its own gaps: the last list stays visible without VPN, the view's type and "Hidden" section are remembered between runs, merge requests where the user is a reviewer get a "Do review" section, and merge requests can be hidden too. Outcome-scoped; no dates.
- **Source materials:** GitHub issues #28–#35 (GitHub milestone "M-4: Źródła – GitHub na równi z GitLabem"); `context/foundation/prd.md` v2 for the existing source rules (FR-001–FR-005, FR-015, FR-023, FR-024, FR-026). The later GitHub milestones M-5 and M-6 stay outside this milestone.
- **Done when:** every F-NN and S-NN below is `done`, and the preserved behaviour still holds: "never an empty list instead of an error", a calm "GitLab unavailable" without VPN (FR-015), the read-only `glab`/`gh` rule with every new call covered by the guard test, and no work data in the repo, tests or logs.
- **Scope anchors:** The user decided on 2026-11-27 that M-4 adds no new FR to the PRD; each issue is anchored here and each `MS-NN` number matches its issue number:
  - MS-28: One shared refresh skeleton for GitLab and GitHub (thread, interval, wake after sleep, "Check now", snapshot publication, `lastSuccessMs`, `nextRefreshAtMs`); the source-specific fetch and rules stay in each source's module; no change in behaviour or commands (issue #28).
  - MS-29: `githubRefresh.minutes` in `settings.json` (same range and default as GitLab), validated in Rust, and "every N min — next try in M min (HH:MM)" in GitHub's unavailable and error states (issue #29).
  - MS-30: GitHub seen-assignment keys in today's shape, a NEW flag since the last visit to the view, and a Windows notice of a new assignment with the FR-004 batch threshold (issue #30).
  - MS-31: Hide and show in a GitHub row, a "Hidden (n)" section with the hide date, resurfacing marked "Changed" on a state change or a comment by someone else, snapshots in the app data store, data through a new read-only `gh` call (issue #31).
  - MS-32: Without VPN, "Work (GitLab)" shows the calm unavailable card and under it the session's last list, dimmed, and the view header shows that list's counts (issue #32; the user chose the fix over `wontfix` on 2026-11-27).
  - MS-33: Hide on a GitLab merge request, resurfacing on a state change or a comment by someone else, data through a new read-only `glab api` call (issue #33).
  - MS-34: The type choice (issues/MRs) and the "Hidden" section's expansion in "Work (GitLab)" are remembered between runs; the view filter stays in memory only, so no work data is stored (issue #34; decided 2026-11-27).
  - MS-35: Merge requests where the user is a reviewer appear in a "Do review" section of "Work (GitLab)", under the same settings filter (FR-002); a new review request gives a Windows notice by the FR-004 mechanism and threshold, bypassing the filter; rows have "+" to "To do"; hiding (MS-33) covers them once it lands (issue #35; decided 2026-11-27, lifting the PRD Non-Goal "reviewer MRs outside v1").

## Vision recap

One developer on one Windows 11 machine uses the app every day for their work GitLab, their private GitHub projects, their own tasks, a "To do" list and a work-time counter. M-1–M-3 delivered the day flow, the Konsola look and a trustworthy work-time counter, but GitHub stayed a plain list next to a GitLab view that hides, flags and notifies. M-4 brings GitHub to the same level on one shared refresh mechanism and closes the GitLab view's remaining gaps.

## North star

**S-01: User can hide a GitHub issue and see it come back marked "Changed" when its state changes or someone else comments** — hiding is the GitLab behaviour behind the secondary success criterion ("hidden issues do not clutter the list") and the hardest piece of parity (a migration, a new `gh` call, shared rules), so a quality-first milestone proves itself here first.

> "North star" here means the smallest end-to-end slice whose delivery proves the milestone works. It is placed as early as its prerequisites allow, because the rest of the milestone matters most if this one works.

## At a glance

| ID   | Change ID                  | Outcome (user can …)                                                              | Prerequisites | PRD refs                      | Status   |
| ---- | -------------------------- | --------------------------------------------------------------------------------- | ------------- | ----------------------------- | -------- |
| F-01 | source-refresh-skeleton    | (foundation) GitLab and GitHub refresh on one shared skeleton, behaviour unchanged | —             | MS-28, FR-015                 | ready    |
| S-01 | github-hide-resurface      | hide a GitHub issue and see it come back marked "Changed"                         | F-01          | MS-31, FR-003, FR-005         | proposed |
| S-02 | github-refresh-interval    | set GitHub's refresh interval and see when the next try happens                    | F-01          | MS-29, FR-005, FR-024         | proposed |
| S-03 | github-new-and-notify      | see a NEW flag on GitHub items and get a notice of a new assignment                | F-01          | MS-30, FR-004, FR-005, FR-023 | proposed |
| S-04 | gitlab-offline-last-list   | see the last GitLab list, dimmed, under the unavailable card without VPN           | —             | MS-32, FR-015                 | ready    |
| S-05 | gitlab-view-memory         | find "Work (GitLab)" with the same type and "Hidden" section after a restart       | —             | MS-34, FR-001, FR-003         | ready    |
| S-06 | gitlab-review-requests     | see merge requests waiting for their review in a "Do review" section               | F-01          | MS-35, FR-001, FR-002, FR-004 | proposed |
| S-07 | gitlab-hide-merge-requests | hide a GitLab merge request and see it come back marked "Changed"                 | S-01, S-06    | MS-33, FR-003                 | proposed |

## Streams

Navigation aid — groups items that share a Prerequisites chain. Canonical ordering still lives in the dependency graph below; this table is the proposed reading order across parallel tracks.

| Stream | Theme                      | Chain                           | Note                                                                                                                |
| ------ | -------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| A      | GitHub parity              | `F-01` → `S-01`, `S-02`, `S-03` | Quality goal: one refresh skeleton first, then each GitHub behaviour on it; S-01 and S-03 each add a migration.      |
| B      | GitLab view fixes          | `S-04`; `S-05`                  | Standalone; both touch the GitLab view's UI, S-05 also the store (a migration), so they run next to F-01.           |
| C      | Review requests, MR hiding | `S-06` → `S-07`                 | Joins Stream A at `F-01` (S-06 adds a GitLab fetch) and at `S-01` (S-07 reuses the hiding rules generalised there). |

Migrations: S-01, S-03, S-05 and S-07 each need one (S-06 reuses today's seen-assignment keys). Only one migration lands per wave of parallel work, so the migration slices run in different waves.

## Baseline

What's already in place in the codebase as of `2026-11-27` (researched).
Slices below assume these are present and do NOT re-scaffold them.

- **Frontend:** present — "Work (GitLab)" with the type filter, NEW flag, hidden section, filter bar and calm offline card (`src/gitlab/`); "Projects (GitHub)" with labels, "+" to "To do" and a dimmed stale list (`src/github/`); the GitLab list state lives in memory in `GitlabListViewProvider`.
- **Backend / API:** present — `gitlab_refresh.rs` (interval from `gitlabRefresh.minutes`, hidden-issue round, new-assignment notices, seen-until mark) and `github_refresh.rs` (a fixed 10-minute interval, no `nextRefreshAtMs`), which repeat each other's thread, loop and publication code; `cli/` with 9 read-only `CliCall` variants and the guard test.
- **Data:** present — app data store at schema v13; `hidden_issues` keyed by GitLab host and issue id (issues only); `seen_assignments` with kind `issue`/`mergeRequest`; `gitlab_view_marks` for the GitLab view's seen-until mark.
- **Auth:** not applicable — single user; `glab`/`gh` sessions only.
- **Deploy / infra:** present — `.github/workflows/ci.yml` publishes the NSIS installer on a version bump with release notes.
- **Observability:** present — tray status icon, Windows notices, app log, "Źródła" view, today's event log.

## Foundations

### F-01: Shared source refresh skeleton

- **Outcome:** (foundation) GitLab and GitHub refresh through one shared skeleton (thread with a `ThreadDown` stand-in, interval, wake after sleep, "Check now", snapshot publication, `lastSuccessMs` and `nextRefreshAtMs`), with the fetch and rules left in each source's module, and no change in behaviour, commands or existing test bodies.
- **Change ID:** source-refresh-skeleton
- **PRD refs:** MS-28, FR-015
- **Unlocks:** S-01, S-02, S-03, S-06 (each adds a GitHub behaviour or a GitLab fetch on the shared skeleton instead of a second copy of the loop).
- **Prerequisites:** —
- **Parallel with:** S-04, S-05
- **Blockers:** —
- **Unknowns:** —
- **Risk:** A refactor of the two busiest Rust files (GitLab's refresh round holds six mutexes); it goes first and alone on those files, and the existing tests stay unchanged so a behaviour change shows up as a failing test.
- **Status:** ready

## Slices

### S-01: Hide a GitHub issue, resurface on change

- **Outcome:** user can hide and show a GitHub issue in its row, see a "Hidden (n)" section with the hide date, and see a hidden issue come back marked "Changed" when its state changes or someone other than them comments.
- **Change ID:** github-hide-resurface
- **PRD refs:** MS-31, FR-003, FR-005
- **Prerequisites:** F-01
- **Parallel with:** S-02, S-04, S-05, S-06
- **Blockers:** —
- **Unknowns:**
  - Where the GitHub login comes from to tell the user's own comments apart (`gh api user`, read-only). — Owner: user. Block: no (settled before its wave).
  - Can the rules in `hidden_issues/rules.rs` be shared without a copy, and can the new storage hold GitLab merge requests too, so S-07 needs no migration? — Owner: team. Block: no (settled in `/10x-plan`).
- **Risk:** A migration and a new `gh` call; it runs in a wave with no other migration, and the shared hiding rules it settles are what S-07 builds on.
- **Status:** proposed

### S-02: GitHub refresh interval and next try

- **Outcome:** user can set GitHub's refresh interval in `settings.json` (`githubRefresh.minutes`, same range and default as GitLab) and see "every N min — next try in M min (HH:MM)" in GitHub's unavailable and error states.
- **Change ID:** github-refresh-interval
- **PRD refs:** MS-29, FR-005, FR-024
- **Prerequisites:** F-01
- **Parallel with:** S-01, S-03, S-04, S-05, S-06
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Small once F-01 exposes the interval and `nextRefreshAtMs` for every source; shares the GitHub view with S-01 and S-03.
- **Status:** proposed

### S-03: GitHub NEW flag and new-assignment notice

- **Outcome:** user can see a NEW flag on GitHub items that appeared since their last visit to the view and gets a Windows notice of a new assigned GitHub issue, with more than 3 at once as one combined notice.
- **Change ID:** github-new-and-notify
- **PRD refs:** MS-30, FR-004, FR-005, FR-023
- **Prerequisites:** F-01
- **Parallel with:** S-02, S-04, S-05, S-06
- **Blockers:** —
- **Unknowns:**
  - Are GitHub notices needed (private projects, usually self-assigned), or is the NEW flag enough? — Owner: user. Block: no (settled before its wave).
- **Risk:** A migration (the seen-assignment kind CHECK and a GitHub view mark); it runs in a later wave than S-01 so the two migrations do not collide.
- **Status:** proposed

### S-04: Last GitLab list without VPN

- **Outcome:** user can see, without VPN, the calm "GitLab unavailable" card with the last success and next try, and under it the session's last list, dimmed as in GitHub; the view header shows that list's counts instead of only "source unavailable".
- **Change ID:** gitlab-offline-last-list
- **PRD refs:** MS-32, FR-015
- **Prerequisites:** —
- **Parallel with:** F-01, S-01, S-02, S-03, S-05, S-06
- **Blockers:** —
- **Unknowns:** —
- **Risk:** A UI condition introduced by the Konsola alignment; the fix must keep the calm card (no red, no alarm) and make clear the list is stale.
- **Status:** ready

### S-05: Remembered GitLab view state

- **Outcome:** user can close and reopen the app and find "Work (GitLab)" with the same type choice (issues/MRs) and the same "Hidden" section expansion; the view filter still resets.
- **Change ID:** gitlab-view-memory
- **PRD refs:** MS-34, FR-001, FR-003
- **Prerequisites:** —
- **Parallel with:** F-01, S-01, S-02, S-03, S-04, S-06
- **Blockers:** —
- **Unknowns:** —
- **Risk:** A migration and a new kind of record in the app data dir (no work data); the AGENTS.md list of app data and the "in memory only" rule change with it.
- **Status:** ready

### S-06: "Do review" section for merge requests

- **Outcome:** user can see the GitLab merge requests where they are a reviewer in a "Do review" section of "Work (GitLab)", under the same settings filter, add them to "To do" with "+", and get a Windows notice of a new review request by the FR-004 mechanism and threshold, whatever the filter says.
- **Change ID:** gitlab-review-requests
- **PRD refs:** MS-35, FR-001, FR-002, FR-004
- **Prerequisites:** F-01
- **Parallel with:** S-01, S-02, S-03, S-04, S-05
- **Blockers:** —
- **Unknowns:** —
- **Risk:** A new `glab api` call and a second source of merge requests in the refresh round; review requests reuse today's seen-assignment keys so they add no migration, and an MR both assigned and to review must not notify twice.
- **Status:** proposed

### S-07: Hide a GitLab merge request

- **Outcome:** user can hide a GitLab merge request (assigned or in "Do review") and see it come back marked "Changed" when its state changes or someone other than them comments.
- **Change ID:** gitlab-hide-merge-requests
- **PRD refs:** MS-33, FR-003
- **Prerequisites:** S-01, S-06
- **Parallel with:** S-02, S-03
- **Blockers:** —
- **Unknowns:**
  - What else brings a merge request back: an approval, a new commit, a pipeline change? — Owner: user. Block: no (settled before its wave).
- **Risk:** Builds on the hiding rules S-01 generalises and on S-06's review section; a migration unless S-01's storage already holds merge requests.
- **Status:** proposed

## Backlog Handoff

| Roadmap ID | Change ID                  | Suggested issue title                                              | Ready for `/10x-plan` | Notes |
| ---------- | -------------------------- | ------------------------------------------------------------------ | --------------------- | ----- |
| F-01       | source-refresh-skeleton    | Wspólny mechanizm odświeżania źródeł (GitLab i GitHub)             | yes                   | Issue #28 |
| S-01       | github-hide-resurface      | GitHub: ukrywanie zgłoszeń z powrotem po zmianie                   | no                    | Issue #31; north star; after F-01 |
| S-02       | github-refresh-interval    | GitHub: interwał odświeżania w ustawieniach i czas następnej próby | no                    | Issue #29; after F-01 |
| S-03       | github-new-and-notify      | GitHub: flaga NOWE i powiadomienie o nowym przypisaniu             | no                    | Issue #30; after F-01; migration wave apart from S-01 |
| S-04       | gitlab-offline-last-list   | GitLab: ostatnia lista znika przy braku VPN                        | yes                   | Issue #32 |
| S-05       | gitlab-view-memory         | GitLab: filtr widoku zapamiętany między uruchomieniami             | yes                   | Issue #34 |
| S-06       | gitlab-review-requests     | Decyzja: MR-y, w których jestem reviewerem                         | no                    | Issue #35; after F-01 |
| S-07       | gitlab-hide-merge-requests | GitLab: ukrywanie merge requestów                                  | no                    | Issue #33; after S-01 and S-06 |

## Open Roadmap Questions

1. **Nazwa produktu** — robocza „Nawigator dnia”. Owner: użytkownik. Block: none.
2. **PRD notes for M-4** — no new FR (decided 2026-11-27); S-06 lifts the Non-Goal "reviewer MRs outside v1" and S-07 widens hiding to merge requests, each with a one-line note in the PRD (as M-3 did for autostart and the week view), without new or changed FR numbers. Owner: user. Block: none.
3. **AGENTS.md rule changes** — S-05 (remembered view state in the app data dir, and the "in memory only" line for the GitLab list state), S-01 and S-07 (hidden snapshots of GitHub issues and GitLab MRs) and S-03 (GitHub seen keys and view mark) each update the app-data list in AGENTS.md together with the change. Owner: user. Block: none (each settled in its `/10x-plan`).

## Parked

- **Writing to GitLab and GitHub** — Why parked: PRD Non-Goals; read-only (no closing, status changes, comments or assignment).
- **Obsidian vault access** — Why parked: PRD Non-Goals; no reading or writing. Any future vault access starts from a new plan, and the PRD Non-Goal is unchanged.
- **Other work-time automation** — Why parked: PRD Non-Goals; no idle detection, activity tracking or automatic stop.
- **Deleting tasks (FR-009)** — Why parked: PRD Non-Goals ("Poza v1"). Reviewer MRs moved into M-4 (S-06) on 2026-11-27.
- **Remembering the GitLab view filter values** — Why parked: decided 2026-11-27 (MS-34); the values are work data (project, label and milestone names), so the filter stays in memory only.
- **Editing settings in the app** — Why parked: the app reads `settings.json` and never writes it; panels stay read-only.
- **Signed installer (MS-13, issue #13)** — Why parked: decided by the user on 2026-11-24. For one user on one machine, a signature does not remove the SmartScreen warning right away, because reputation builds with downloads. Azure Artifact Signing is not open to individuals in Poland, and an OV certificate costs a yearly fee. Free OSS signing (SignPath Foundation) asks for proof of reputation the project does not have yet. Research: `context/archive/2026-11-24-installer-signing/research.md`. Comes back if the app is ever shared with others and has users to show.
- **Breaks and pomodoro, distraction blocker, morning "plan the day"** — Why parked: PRD "considered, deliberately NOT excluded"; no FR and no issue covers them.
- **GitHub milestones M-5 and M-6** — Why parked: outside M-4; each opens as the next milestone of this file after M-4 closes (lists and daily comfort, code tidy-up and distribution).

## Milestone History

- **M-1: The narrowed day flow runs in the app** (`full-day-flow`) — closed 2026-11-23. The day flow from PRD v1 runs in the app: GitLab issues and MRs (filter, hiding, notifications), GitHub issues, own tasks, the "To do" list, and the work-time counter with its norm report and reminders. FR-001–FR-008 and FR-010–FR-018 are covered.
- **M-2: Konsola mockup gaps closed and the codebase cleaned up** (`konsola-gaps-and-cleanup`) — closed 2026-11-24. The Konsola mockup gaps are now in the app: the event log, the days table, the reminders panel, the app-data details, the GitLab NEW flag, hidden date and filter bar, GitHub labels, and "+" to "To do". They sit on a cleaner base (shared UI tools and timer state, no vault layer), and the design-system artifact matches the app. Releases carry a changelog and real metadata (0.2.1, 0.3.0). FR-019–FR-026 and MS-09–MS-12 and MS-14–MS-16 are covered. MS-13 (installer signing) was parked.
- **M-3: Work time and the tray** (`worktime-and-tray`) — closed 2026-11-27. The work-time counter and its norm report can be trusted and reached without opening the window: Polish public holidays count out of the norm, the daily norm is configurable with a half day off, the counter starts and stops from the tray (with today's time in the tooltip and a running icon), the app can start with Windows into the tray, the month report copies to the clipboard for a spreadsheet, work-time entries carry a short note, STOP shows an end-of-day summary, and "Work time" shows a week summary against the week's norm. MS-17, MS-18, MS-19 and MS-23–MS-27 are covered.

## Done

Entries up to 2026-11-23 belong to M-1; their IDs refer to M-1's items (or to its earlier decomposition, archived at `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md`), not to the F-NN/S-NN above. Entries from 2026-11-24 belong to M-2 and those from 2026-11-26 and 2026-11-27 to M-3. New entries for M-4 are appended below them; each entry's IDs refer to the milestone it closed in.

- **F-01: (foundation) the app reads and writes only inside vault folders configured per space (Work, Private), never sees excluded paths, changes only the targeted line or section of a note (rest of the file byte-identical), and sends deletions to the vault trash — proven against a small invented test vault.** — Archived 2026-11-20 → `context/archive/2026-11-20-safe-vault-access/`. Lesson: —.
- **F-03: (foundation) a private GitHub repository exists, every change is built and tested in CI against invented data only, and a merge to `main` publishes a Windows installer as a GitHub Release named "Nawigator dnia" only when it bumps the version in `src-tauri/Cargo.toml`; every other merge only runs CI.** — Archived 2026-11-21 → `context/archive/2026-11-20-ci-release-pipeline/`. Lesson: —.
- **F-02: (foundation) the app lives in the tray with a status icon (OK / source error / not running), and any source failure (missing configured path, failed command, logged-out `glab`/`gh`) is reported there and as a Windows notice within 1 minute; screens show the error instead of an empty list.** — Archived 2026-11-21 → `context/archive/2026-11-20-source-health-status/`. Lesson: —.
- **S-05: User can create a task from the template in the work vault and change its status from the app.** — Archived 2026-11-21 → `context/archive/2026-11-20-work-tasks-from-template/`. Lesson: —.
- **F-01: (foundation) the app keeps its own data (own tasks, the "To do" list, hidden-issue snapshots, work-time entries) in its app data dir. A write is never half-applied, and the data survives closing the app, restarting Windows and a crash — proven by tests on invented data.** — Archived 2026-11-23 → `context/archive/2026-11-23-app-data-store/`. Lesson: —.
- **F-02: (foundation) the app runs the logged-in `glab`/`gh` with a fixed program and fixed arguments, read-only, and reports each call as one of: data, unavailable (no VPN/network), logged out, tool missing, error. A fake CLI makes these states testable without any work data.** — Archived 2026-11-23 → `context/archive/2026-11-23-read-only-cli-runner/`. Lesson: —.
- **S-01: User can open the "Work (GitLab)" view and see the issues and MRs assigned to them. Without VPN the view says "GitLab unavailable" calmly, with the time of the last successful refresh, and raises no alarm. A logged-out `glab` shows as an error in the tray. The app gets its navigation between views and the fixed header area.** — Archived 2026-11-23 → `context/archive/2026-11-23-gitlab-assigned-list/`. Lesson: —.
- **S-08: user can start and stop work time from a counter always visible at the top of the window. A running counter survives closing the app, a Windows restart and a crash. The user can correct an entry's start or end time or add a missing entry.** — Archived 2026-11-23 → `context/archive/2026-11-23-work-timer/`. Lesson: —.
- **S-06: user can add a task in the Work or Private space (default Work while the counter runs, otherwise Private), edit its title and short plain note, set an optional due date and priority, and mark it done. The vault-based task screen from the previous decomposition is removed.** — Archived 2026-11-23 → `context/archive/2026-11-23-own-tasks/`. Lesson: —.
- **S-07: user can pick today's items from the GitLab list and their own tasks (and from GitHub once S-05 lands) into "To do", up to the limit from settings (default 5). They tick items off; unticked items carry over to the next day; items closed in GitLab or GitHub tick themselves; ticking an own task marks it done.** — Archived 2026-11-23 → `context/archive/2026-11-23-todo-list/`. Lesson: —.
- **S-09: user can see the day and month sums, how far the month is from the norm so far (8 h × workdays that have passed, Monday–Friday minus days they mark as off), and the average needed per remaining workday to meet the month's norm.** — Archived 2026-11-23 → `context/archive/2026-11-23-work-norm-report/`. Lesson: —.
- **S-10: user gets a reminder when the counter has been running longer than the threshold from settings ("are you still working?") and when it is not running after the set hour on a workday.** — Archived 2026-11-23 → `context/archive/2026-11-23-timer-reminders/`. Lesson: —.
- **S-03: user can hide an issue. It comes back, marked as changed, when its status changes or someone other than the user (bots included) comments on it.** — Archived 2026-11-23 → `context/archive/2026-11-23-hide-and-resurface/`. Lesson: —.
- **S-04: user gets a Windows notification for every new issue or MR assigned to them, whatever the filter says, within 15 minutes while on VPN; more than 3 at once arrive as one combined notification.** — Archived 2026-11-23 → `context/archive/2026-11-23-new-assignment-notify/`. Lesson: —.
- **S-05: user can open the "Projects (GitHub)" view and see the open issues assigned to them from the repositories listed in settings; a logged-out or missing `gh` shows as an error, not an empty list.** — Archived 2026-11-23 → `context/archive/2026-11-23-github-assigned-issues/`. Lesson: —.
- **S-02: user can set in settings which assigned items the list shows — by project, labels, milestone and state.** — Archived 2026-11-23 → `context/archive/2026-11-23-gitlab-visibility-filter/`. Lesson: —.
- **S-01: user can add an item to "To do" with a "+" button in each row of "Work (GitLab)" and "Projects (GitHub)", blocked at the limit with the same hint as the picker; an item already added shows the IN TO DO flag and a filled button.** — Archived 2026-11-24 → `context/archive/2026-11-24-todo-add-from-sources/`. Lesson: —.
- **F-03: (foundation) `src-tauri/src/vault/`, the `vault:*` / `exclusions:*` health sources and their tests and fixtures are gone; AGENTS.md no longer carries the vault contract; the roadmap's parked item is resolved.** — Archived 2026-11-24 → `context/archive/2026-11-24-remove-vault-layer/`. Lesson: —.
- **S-10: user can see the real description and author in the exe and installer properties instead of "A Tauri App" / "you".** — Archived 2026-11-24 → `context/archive/2026-11-24-app-metadata/`. Lesson: —.
- **S-09: user can see in each GitHub Release what changed since the previous `app-v*` tag; the unsigned-installer note stays until signing exists.** — Archived 2026-11-24 → `context/archive/2026-11-24-release-notes/`. Lesson: —.
- **F-01: (foundation) the tools that grew locally in views — `useNow`, `plural`, `Tabs`, a table/grid component, a `Segmented` size for 40 px fields, a priority chip, `ErrorState.message` as a node, `disabled` for `SourceState` actions — live in `src/components/` and the existing views use them, with no visible change.** — Archived 2026-11-24 → `context/archive/2026-11-24-shared-ui-tools/`. Lesson: —.
- **F-04: (foundation) the open Manual rows of `context/changes/konsola-design-system/plan.md` are checked by the user on the real machine (offline fonts, layout at 1280 and 800 px, window state, GitLab with and without VPN, logged-out `gh`, own tasks, work time, keyboard-only navigation) and the change is archived.** — Archived 2026-11-24 → `context/archive/2026-11-23-konsola-design-system/`. Lesson: —.
- **F-02: (foundation) one work-timer state in the UI feeds the header, the dashboard and "Work time" (one listener, one tick), with no change in behaviour or in Rust commands.** — Archived 2026-11-24 → `context/archive/2026-11-24-shared-work-timer-state/`. Lesson: —.
- **S-04: user can see the labels of each GitHub issue as text in its row, as in GitLab.** — Archived 2026-11-24 → `context/archive/2026-11-24-github-issue-labels/`. Lesson: —.
- **S-02: user can see a NEW flag on GitLab items that appeared since the last check, and the date next to hidden rows ("ukryte 18.11").** — Archived 2026-11-24 → `context/archive/2026-11-24-gitlab-new-and-hidden-date/`. Lesson: —.
- **S-03: user can see the active filter values above the GitLab list with a link to settings, and in the "GitLab unavailable" card the refresh interval and when the next try happens.** — Archived 2026-11-24 → `context/archive/2026-11-24-gitlab-filter-bar-retry/`. Lesson: —.
- **S-05: user can see a table of the month's days — entries, a bar against 8 h, the sum, the deviation and a "day off" switch — with today highlighted and days off hatched, and mark a day off from the table.** — Archived 2026-11-24 → `context/archive/2026-11-24-worktime-days-table/`. Lesson: —.
- **S-08: user can see a "Dziennik zdarzeń" panel under "Stan źródeł" on the dashboard with today's events: new assignments, ticking or removing "To do" items, START/STOP, a source going into error and back.** — Archived 2026-11-24 → `context/archive/2026-11-24-event-log/`. Lesson: —.
- **S-06: user can see a "Przypomnienia" panel with the reminder thresholds and whether reminders are on; no editing, the settings stay in the file.** — Archived 2026-11-24 → `context/archive/2026-11-24-worktime-reminders-panel/`. Lesson: —.
- **S-07: user can see in the "Dane aplikacji" card the time of the last backup, the number of kept backups ("7 z 7"), the schema version, and open the backups folder.** — Archived 2026-11-24 → `context/archive/2026-11-24-app-data-details/`. Lesson: —.
- **F-05: (foundation) the "Day Navigator — Konsola" artifact's README and components match the app: Flag tone `new`, `ItemRow` id, `ItemRow` labels, `HoursBar` with a required `norm` (from Rust), `DataTable` row states `current`/`hatched` and cell tones `info`/`warn`, numbered navigation with shortcuts, `Notice`, START/STOP confirmation, app and tray icons.** — Archived 2026-11-24 → `context/archive/2026-11-24-design-system-artifact-sync/`. Lesson: —.
- **S-01: user can see Polish public holidays (fixed dates and the Easter-based ones) counted out of the month's norm and shown in the days table like a day off, with the holiday's name; a setting turns the calendar off.** — Archived 2026-11-26 → `context/archive/2026-11-26-public-holidays-norm/`. Lesson: —.
- **S-06: user can add, edit and see a short plain-text note on a work-time entry.** — Archived 2026-11-26 → `context/archive/2026-11-26-work-entry-note/`. Lesson: —.
- **S-03: user can start and stop the work-time counter from the tray menu and see today's work time in the tray tooltip next to the source state.** — Archived 2026-11-26 → `context/archive/2026-11-26-tray-timer-control/`. Lesson: —.
- **S-02: user can set the daily norm in settings (default 8 h) and mark a half day off in the days table, and the report counts both.** — Archived 2026-11-27 → `context/archive/2026-11-26-configurable-daily-norm/`. Lesson: —.
- **S-08: user can see in "Work time" the current week's sum against the week's norm, day by day.** — Archived 2026-11-27 → `context/archive/2026-11-26-week-summary/`. Lesson: —.
- **S-05: user can copy the month report (days, entries, sums, deviation, days off) to the clipboard as text that pastes into a spreadsheet.** — Archived 2026-11-27 → `context/archive/2026-11-26-month-report-copy/`. Lesson: —.
- **S-07: user can see, after STOP, a summary of the day: time worked today, the month's deviation, what was ticked off and what carries over to tomorrow, with no decision forced.** — Archived 2026-11-27 → `context/archive/2026-11-26-end-of-day-summary/`. Lesson: —.
- **S-04: user can have the app start with Windows into the tray, without showing the window, controlled by a switch in `settings.json`.** — Archived 2026-11-27 → `context/archive/2026-11-26-windows-autostart/`. Lesson: —.
