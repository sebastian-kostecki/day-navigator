---
project: "Nawigator dnia"
version: 4
status: draft
created: 2026-11-20
updated: 2026-11-24
prd_version: 2
main_goal: quality
top_blocker: none
milestone_id: konsola-gaps-and-cleanup
milestone_seq: 2
milestone_status: open
---

# Roadmap: Nawigator dnia

> Derived from GitHub issues #1–#16 (2026-11-24), `context/foundation/prd.md` (v2, FR-019–FR-026) + the codebase baseline.
> Edit-in-place; the next milestone continues this file. Archive only when an independent roadmap replaces it.
> Slices below are listed in dependency order. The "At a glance" table is the index.
> M-1's decomposition (F-01/F-02, S-01–S-10, all done) was replaced in place by M-2; M-1 is recorded in `## Milestone History` and its archived changes in `## Done`.

## Milestone

**M-2: Konsola mockup gaps closed and the codebase cleaned up** — Status: open

- **Intent:** The app shows what the Konsola mockups show and 0.2.0 left out (event log, days table, reminders panel, app-data details, GitLab NEW flag and filter bar, GitHub labels, "+" to "To do"), on a cleaner base (shared UI tools and timer state, no vault layer), verified by hand, with releases that say what changed, carry real metadata and, once a signing method is chosen, are signed.
- **Source materials:** GitHub issues #1–#16 (open on 2026-11-24; #17–#19 are decision questions and stay outside); `context/foundation/prd.md` v2 for #1–#8.
- **Done when:** every F-NN and S-NN below is `done`, and the preserved behaviour still holds: FR-016 (CI green, an installer only on a version bump) and the read-only `glab`/`gh` rule.
- **Scope anchors:** FR-019–FR-026 in the PRD (issues #1–#8). Issues #9–#16 have no FR and are anchored here; each `MS-NN` number matches its issue number:
  - MS-09: One shared work-timer state in the UI used by the header, the dashboard and "Work time", with no change in behaviour or in Rust commands (issue #9).
  - MS-10: Shared UI tools moved from the views into `src/components/`: `useNow`, `plural`, `Tabs`, a table/grid component, a `Segmented` size for 40 px fields, a priority chip, `ErrorState.message` as a node, `disabled` for `SourceState` actions (issue #10).
  - MS-11: The unused vault layer removed, together with its health sources, tests and fixtures; AGENTS.md and the roadmap updated (issue #11).
  - MS-12: The open Manual rows of the Konsola design-system plan checked by hand, then the change archived (issue #12).
  - MS-13: The NSIS installer signed in CI without secrets in the repo, after a signing method is chosen; the release text updated (issue #13).
  - MS-14: The release description generated from the changes since the previous `app-v*` tag, keeping the unsigned-installer note until signing exists (issue #14).
  - MS-15: Real `description` and `authors` in `src-tauri/Cargo.toml`, checked in the exe and installer properties (issue #15).
  - MS-16: The "Day Navigator — Konsola" design-system artifact brought in line with the app (Flag tone `new`, `ItemRow` id, numbered navigation with shortcuts, `Notice`, START/STOP confirmation, app and tray icons) (issue #16).

## Vision recap

One developer on one Windows 11 machine uses the app every day for their work GitLab, their private GitHub projects, their own tasks, a "To do" list and a work-time counter. M-1 delivered that day flow. The Konsola mockups show more than the 0.2.0 release has, and building it left duplicated UI code and a dormant vault layer. M-2 closes those mockup gaps, cleans the base they are built on, and makes releases tell what they contain.

## North star

**S-01: User can add an item to "To do" with "+" straight from the GitLab and GitHub views** — the change that most shortens the daily flow from US-01 (picking today's work where you see it). It has no prerequisites, so it goes first.

> The north star is the smallest end-to-end slice whose delivery proves the milestone works. It is placed as early as its prerequisites allow, because the rest of the milestone matters most if this one works.

## At a glance

| ID   | Change ID                   | Outcome (user can …)                                                                   | Prerequisites | PRD refs        | Status   |
| ---- | --------------------------- | -------------------------------------------------------------------------------------- | ------------- | --------------- | -------- |
| F-01 | shared-ui-tools             | (foundation) shared UI tools live in `src/components/`, not copied across views         | —             | MS-10           | done     |
| F-02 | shared-work-timer-state     | (foundation) one work-timer state feeds the header, dashboard and "Work time"           | —             | MS-09           | done     |
| F-03 | remove-vault-layer          | (foundation) the vault layer and its health sources are gone                            | —             | MS-11           | done     |
| F-04 | konsola-design-system       | (foundation) the Konsola design system is checked by hand and its change archived       | —             | MS-12           | done     |
| F-05 | design-system-artifact-sync | (foundation) the design-system artifact matches the app                                 | F-04          | MS-16           | ready    |
| S-01 | todo-add-from-sources       | add an item to "To do" with "+" from the GitLab and GitHub views                        | —             | FR-026, US-01   | done     |
| S-02 | gitlab-new-and-hidden-date  | see a NEW flag on fresh GitLab items and the date a row was hidden                      | —             | FR-023          | done     |
| S-03 | gitlab-filter-bar-retry     | see the active filter values and when GitLab will be tried again                        | F-01          | FR-024          | done     |
| S-04 | github-issue-labels         | see labels on GitHub issues                                                             | —             | FR-025          | done     |
| S-05 | worktime-days-table         | see a table of the month's days against 8 h and mark days off there                     | F-01, F-02    | FR-020          | done |
| S-06 | worktime-reminders-panel    | see the reminder thresholds and whether reminders are on                                | —             | FR-021          | ready    |
| S-07 | app-data-details            | see the last backup, kept backups and schema version, and open the backups folder       | F-03          | FR-022          | planning |
| S-08 | event-log                   | see today's events in a "Dziennik zdarzeń" panel on the dashboard                       | F-03          | FR-019          | done     |
| S-09 | release-notes               | see in each release what changed since the previous one                                 | —             | MS-14, FR-016   | done     |
| S-10 | app-metadata                | see the real description and author in the exe and installer properties                 | —             | MS-15           | done     |
| S-11 | installer-signing           | install a release without a SmartScreen warning                                         | S-09          | MS-13, FR-016   | blocked  |

## Streams

Navigation aid — groups items that share a Prerequisites chain. Canonical ordering still lives in the dependency graph below; this table is the proposed reading order across parallel tracks.

| Stream | Theme                   | Chain                                          | Note                                                                                                   |
| ------ | ----------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| A      | Source views            | `S-01`, `S-02`, `S-04`; `F-01` → `S-03`         | The north star first; `F-01` lands before the first view slice that would add another local copy.       |
| B      | Work time               | `F-02` → `S-05`; `S-06`                         | `S-05` also needs `F-01` (joins Stream A at `F-01`); `S-06` shares the view, so plan it after `S-05`.    |
| C      | App data and events     | `F-03` → `S-07`, `S-08`                         | Quality goal: the health source set is settled before new health details and source-error events.     |
| D      | Releases                | `S-10`; `S-09` → `S-11`                         | Must keep FR-016; `S-11` waits on the signing decision.                                                 |
| E      | Design system           | `F-04` → `F-05`                                 | Manual check of the current UI first, then the artifact follows the checked app.                        |

## Baseline

What's already in place in the codebase as of `2026-11-24` (researched + user-confirmed).
Foundations below assume these are present and do NOT re-scaffold them.

- **Frontend:** present — React 19 views per feature (`src/gitlab/`, `src/github/`, `src/own-tasks/`, `src/todo/`, `src/worktime/`, `src/health/`), shared components in `src/components/`. Gaps from the issues: `useWorkTimer` subscribed in three places, no shared `useNow`/`plural`/`Tabs`/table component, no `HoursBar`.
- **Backend / API:** present — Tauri commands in `src-tauri/src/lib.rs`, the read-only CLI runner in `src-tauri/src/cli/`, GitLab/GitHub refresh, reminders. The dormant vault layer `src-tauri/src/vault/` is still there.
- **Data:** present — app data store in `src-tauri/src/store/` (SQLite, 4 migrations, daily backups); no event records.
- **Auth:** not applicable — single user; `glab`/`gh` sessions only.
- **Deploy / infra:** present — `.github/workflows/ci.yml` publishes the NSIS installer on a version bump with a fixed `releaseBody`, unsigned; `src-tauri/Cargo.toml` still has template `description` and `authors`.
- **Observability:** present — tray status icon, Windows notices, app log, "Stan źródeł" view.

## Foundations

### F-01: Shared UI tools

- **Outcome:** (foundation) the tools that grew locally in views — `useNow`, `plural`, `Tabs`, a table/grid component, a `Segmented` size for 40 px fields, a priority chip, `ErrorState.message` as a node, `disabled` for `SourceState` actions — live in `src/components/` and the existing views use them, with no visible change.
- **Change ID:** shared-ui-tools
- **PRD refs:** MS-10
- **Unlocks:** S-03 (next-try time needs `useNow`), S-05 (days table needs the table component and `plural`); new view slices reuse them instead of adding more copies.
- **Prerequisites:** —
- **Parallel with:** F-02, F-03, F-04, F-05, S-01, S-02, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Touches most views at once; it lands before the view slices so they build on the shared pieces, and it must not change what the user sees.
- **Status:** done

### F-02: Shared work-timer state

- **Outcome:** (foundation) one work-timer state in the UI feeds the header, the dashboard and "Work time" (one listener, one tick), with no change in behaviour or in Rust commands.
- **Change ID:** shared-work-timer-state
- **PRD refs:** MS-09
- **Unlocks:** S-05 (the days table's live "today" row reads the shared state instead of a fourth subscription).
- **Prerequisites:** —
- **Parallel with:** F-01, F-03, F-04, F-05, S-01, S-02, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** The counter is the always-visible header and a guardrail ("time is never lost"); the change must be behaviour-neutral, so START/STOP and the confirmation are re-checked by hand.
- **Status:** done

### F-03: Remove the vault layer

- **Outcome:** (foundation) `src-tauri/src/vault/`, the `vault:*` / `exclusions:*` health sources and their tests and fixtures are gone; AGENTS.md no longer carries the vault contract; the roadmap's parked item is resolved.
- **Change ID:** remove-vault-layer
- **PRD refs:** MS-11
- **Unlocks:** S-07 (app-data details extend a health card set without vault sources), S-08 (source-error events are recorded only for sources that still exist).
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-04, F-05, S-01, S-02, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Removes a hard rule from AGENTS.md ("Vaults are out of scope … existing vault code keeps its contract"); the rule about not touching vaults stays, only the dormant code goes.
- **Status:** done

### F-04: Manual check of the Konsola design system

- **Outcome:** (foundation) the open Manual rows of `context/changes/konsola-design-system/plan.md` are checked by the user on the real machine (offline fonts, layout at 1280 and 800 px, window state, GitLab with and without VPN, logged-out `gh`, own tasks, work time, keyboard-only navigation) and the change is archived.
- **Change ID:** konsola-design-system
- **PRD refs:** MS-12
- **Unlocks:** verification path — every UI slice in this milestone starts from a hand-checked 0.2.0 UI; F-05 syncs the artifact to the checked app.
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, S-01, S-02, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Only the user can run it (VPN, logged-out `gh`, real window); the plan already exists, so this needs no new `/10x-plan`.
- **Status:** done

### F-05: Design-system artifact in line with the app

- **Outcome:** (foundation) the "Day Navigator — Konsola" artifact's README and components match the app: Flag tone `new`, `ItemRow` id, `ItemRow` labels, `HoursBar` with a required `norm` (from Rust), `DataTable` row states `current`/`hatched` and cell tones `info`/`warn`, numbered navigation with shortcuts, `Notice`, START/STOP confirmation, app and tray icons.
- **Change ID:** design-system-artifact-sync
- **PRD refs:** MS-16
- **Unlocks:** verification path — UI reviews of the M-2 slices (the `new` flag tone in S-01/S-02, panels in S-05–S-08) check against a current design source instead of an outdated one.
- **Prerequisites:** F-04
- **Parallel with:** F-01, F-02, F-03, S-01, S-02, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** It edits a published artifact outside the repo; if the manual check (F-04) changes the UI, syncing first would document a state that does not ship.
- **Status:** ready

## Slices

### S-01: "+" to "To do" from the source views

- **Outcome:** user can add an item to "To do" with a "+" button in each row of "Work (GitLab)" and "Projects (GitHub)", blocked at the limit with the same hint as the picker; an item already added shows the IN TO DO flag and a filled button.
- **Change ID:** todo-add-from-sources
- **PRD refs:** FR-026, US-01
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-02, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** The source views must learn the "To do" state without a new poller (e.g. a list-change event); the limit and carry-over rules stay in Rust.
- **Status:** done

### S-02: GitLab NEW flag and hidden date

- **Outcome:** user can see a NEW flag on GitLab items that appeared since the last check, and the date next to hidden rows ("ukryte 18.11").
- **Change ID:** gitlab-new-and-hidden-date
- **PRD refs:** FR-023
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-01, S-04, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:**
  - What counts as "the last check" for NEW — the last refresh or the last time the user opened the view? — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** Reuses the seen-assignment keys and hidden-issue snapshots from M-1; they hold only opaque keys, so the "hidden since" date must be added without storing work content.
- **Status:** done

### S-03: GitLab filter bar and next retry

- **Outcome:** user can see the active filter values above the GitLab list with a link to settings, and in the "GitLab unavailable" card the refresh interval and when the next try happens.
- **Change ID:** gitlab-filter-bar-retry
- **PRD refs:** FR-024
- **Prerequisites:** F-01
- **Parallel with:** F-02, F-03, F-04, F-05, S-01, S-02, S-04, S-06, S-07, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Filter values are work data (project names, labels); they may be shown in the UI but must not reach logs.
- **Status:** done

### S-04: GitHub issue labels

- **Outcome:** user can see the labels of each GitHub issue as text in its row, as in GitLab.
- **Change ID:** github-issue-labels
- **PRD refs:** FR-025
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-01, S-02, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Changes the fixed `gh issue list` call; it stays read-only and covered by the guard test.
- **Status:** done

### S-05: Days table in "Work time"

- **Outcome:** user can see a table of the month's days — entries, a bar against 8 h, the sum, the deviation and a "day off" switch — with today highlighted and days off hatched, and mark a day off from the table.
- **Change ID:** worktime-days-table
- **PRD refs:** FR-020
- **Prerequisites:** F-01, F-02
- **Parallel with:** F-03, F-04, F-05, S-01, S-02, S-03, S-04, S-07, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Needs a per-day report from Rust; the norm rule stays in Rust and is unit-tested there, and the UI only displays it.
- **Status:** done

### S-06: Reminders panel in "Work time"

- **Outcome:** user can see a "Przypomnienia" panel with the reminder thresholds and whether reminders are on; no editing, the settings stay in the file.
- **Change ID:** worktime-reminders-panel
- **PRD refs:** FR-021
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-01, S-02, S-04, S-07, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Shares the "Work time" layout with S-05; planned after it so the two do not edit the same view at once.
- **Status:** ready

### S-07: App-data details in "Stan źródeł"

- **Outcome:** user can see in the "Dane aplikacji" card the time of the last backup, the number of kept backups ("7 z 7"), the schema version, and open the backups folder.
- **Change ID:** app-data-details
- **PRD refs:** FR-022
- **Prerequisites:** F-03
- **Parallel with:** F-01, F-02, F-04, F-05, S-01, S-02, S-03, S-04, S-05, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Store errors must still carry no paths or row values; the folder action opens only the app data dir.
- **Status:** planning

### S-08: Event log on the dashboard

- **Outcome:** user can see a "Dziennik zdarzeń" panel under "Stan źródeł" on the dashboard with today's events: new assignments, ticking or removing "To do" items, START/STOP, a source going into error and back.
- **Change ID:** event-log
- **PRD refs:** FR-019
- **Prerequisites:** F-03
- **Parallel with:** F-01, F-02, F-04, F-05, S-01, S-02, S-03, S-04, S-05, S-06, S-09, S-10
- **Blockers:** —
- **Unknowns:**
  - How long events are kept (today only, or longer with cleanup)? — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** The only new domain record in M-2 (a new migration); events hold only what the UI already shows, and nothing from them goes to the app log.
- **Status:** done

### S-09: Release notes from changes

- **Outcome:** user can see in each GitHub Release what changed since the previous `app-v*` tag; the unsigned-installer note stays until signing exists.
- **Change ID:** release-notes
- **PRD refs:** MS-14, FR-016 (preserved)
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-01, S-02, S-04, S-06, S-10
- **Blockers:** —
- **Unknowns:**
  - Commits since the previous tag or a CHANGELOG file? — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** Edits the release job; a release must still happen only on a version bump.
- **Status:** done

### S-10: App metadata

- **Outcome:** user can see the real description and author in the exe and installer properties instead of "A Tauri App" / "you".
- **Change ID:** app-metadata
- **PRD refs:** MS-15
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-01, S-02, S-04, S-06, S-09
- **Blockers:** —
- **Unknowns:** —
- **Risk:** `Cargo.toml` is the only version source; the change must not bump the version by accident and publish a release.
- **Status:** done

### S-11: Signed installer

- **Outcome:** user can install a release without a Windows SmartScreen warning; signing runs in CI with GitHub Actions secrets only, and the release text drops the "Run anyway" note.
- **Change ID:** installer-signing
- **PRD refs:** MS-13, FR-016 (preserved)
- **Prerequisites:** S-09
- **Parallel with:** F-01, F-02, F-03, F-04, F-05, S-01–S-08, S-10
- **Blockers:** a code-signing certificate or an Azure Trusted Signing account, with its identity check, from the provider.
- **Unknowns:**
  - Which signing method (code-signing certificate or Azure Trusted Signing), at what cost? — Owner: user. Block: yes.
- **Risk:** The first secret in CI; it lives only in GitHub Actions secrets, never in the repo, and the pre-push and secret-scan gates stay.
- **Status:** blocked

## Backlog Handoff

| Roadmap ID | Change ID                   | Suggested issue title                                   | Ready for `/10x-plan` | Notes |
| ---------- | --------------------------- | ------------------------------------------------------- | --------------------- | ----- |
| F-01       | shared-ui-tools             | Shared UI tools extracted from views                    | yes                   | Issue #10 |
| F-02       | shared-work-timer-state     | Shared work-timer state in the UI                       | yes                   | Issue #9 |
| F-03       | remove-vault-layer          | Remove the unused vault layer                           | yes                   | Issue #11 |
| F-04       | konsola-design-system       | Manual check of the Konsola design system               | no                    | Issue #12; plan exists — user runs the Manual rows, then `/10x-archive konsola-design-system` |
| F-05       | design-system-artifact-sync | Sync the design-system artifact with the app            | yes                   | Issue #16 |
| S-01       | todo-add-from-sources       | "+" to "To do" in the GitLab and GitHub views           | yes                   | Issue #8; north star |
| S-02       | gitlab-new-and-hidden-date  | GitLab: NEW flag and hidden date                        | yes                   | Issue #5 |
| S-03       | gitlab-filter-bar-retry     | GitLab: filter bar and refresh interval                 | yes                   | Issue #6 |
| S-04       | github-issue-labels         | GitHub: issue labels                                    | yes                   | Issue #7 |
| S-05       | worktime-days-table         | Days table in "Work time" (8 h bar)                     | yes                   | Issue #2; F-01, F-02 done |
| S-06       | worktime-reminders-panel    | Reminders panel in "Work time"                          | yes                   | Issue #3; best after S-05 |
| S-07       | app-data-details            | App-data details in "Stan źródeł"                       | yes                   | Issue #4 |
| S-08       | event-log                   | Event log on the dashboard                              | yes                   | Issue #1 |
| S-09       | release-notes               | Release description with the list of changes            | yes                   | Issue #14 |
| S-10       | app-metadata                | App metadata in Cargo.toml                              | yes                   | Issue #15 |
| S-11       | installer-signing           | Sign the installer                                      | no                    | Issue #13; blocked on the signing method |

## Open Roadmap Questions

1. **Nazwa produktu** — robocza „Nawigator dnia”. Owner: użytkownik. Block: none.
2. **Decyzje #17–#19 (autostart z Windows, widok tygodnia, zamknięcie dnia)** — poza M-2; jeśli zapadną na „tak”, trafiają do PRD i otwierają kolejny milestone. Owner: użytkownik. Block: none.

## Parked

- **Writing to GitLab and GitHub** — Why parked: PRD Non-Goals; read-only (no closing, status changes, comments or assignment).
- **Obsidian vault access** — Why parked: PRD Non-Goals; no reading or writing. The dormant vault code was removed in F-03 (`remove-vault-layer`); any future vault access starts from a new plan, and the PRD Non-Goal is unchanged.
- **Work-time automation** — Why parked: PRD Non-Goals; no automatic holiday calendar, idle detection or activity tracking.
- **MRs where the user is a reviewer; deleting tasks (FR-009)** — Why parked: PRD Non-Goals ("Poza v1").
- **Editing reminder settings in the app** — Why parked: issue #3 scopes the panel as read-only; the settings stay in the file.
- **Autostart, week view, end-of-day close and the rest of the PRD v2 plan** — Why parked: open decision issues #17–#19 and PRD "considered, deliberately NOT excluded"; no FR covers them.

## Milestone History

- **M-1: The narrowed day flow runs in the app** (`full-day-flow`) — closed 2026-11-23. The day flow from PRD v1 runs in the app: GitLab issues and MRs (filter, hiding, notifications), GitHub issues, own tasks, the "To do" list, and the work-time counter with its norm report and reminders. FR-001–FR-008 and FR-010–FR-018 are covered.

## Done

Entries up to 2026-11-23 belong to M-1; their IDs refer to M-1's items (or to its earlier decomposition, archived at `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md`), not to the F-NN/S-NN above. New entries for M-2 are appended below them.

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
