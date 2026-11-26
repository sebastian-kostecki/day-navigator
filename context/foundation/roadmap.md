---
project: "Nawigator dnia"
version: 5
status: draft
created: 2026-11-20
updated: 2026-11-25
prd_version: 2
main_goal: quality
top_blocker: decisions
milestone_id: worktime-and-tray
milestone_seq: 3
milestone_status: open
---

# Roadmap: Nawigator dnia

> Derived from GitHub issues #17–#19 and #23–#27 (GitHub milestone "M-3: Czas pracy i zasobnik", 2026-11-25), `context/foundation/prd.md` (v2) + the codebase baseline.
> Edit-in-place; the next milestone continues this file. Archive only when an independent roadmap replaces it.
> Slices below are listed in dependency order. The "At a glance" table is the index.
> M-2's decomposition (F-01–F-05, S-01–S-10, all done) was replaced in place by M-3; M-1 and M-2 are recorded in `## Milestone History` and their archived changes in `## Done`.

## Milestone

**M-3: Work time and the tray** — Status: open

- **Intent:** The work-time counter and its norm report can be trusted and reached without opening the window: public holidays and a configurable norm count in the norm, the counter is controlled from the tray and the app starts with Windows, the month report can be copied out, entries can carry a note, and the day and week can be summed up. Outcome-scoped; no dates.
- **Source materials:** GitHub issues #17, #18, #19, #23, #24, #25, #26, #27 (GitHub milestone "M-3: Czas pracy i zasobnik"); `context/foundation/prd.md` v2 for the existing work-time rules (FR-013, FR-014, FR-017, FR-018, FR-020, FR-021). The later GitHub milestones M-4–M-6 (#20, #28–#52) stay outside this milestone.
- **Done when:** every S-NN below is `done`, and the preserved behaviour still holds: the work-time guardrail ("time is never lost"), FR-016 (CI green, an installer only on a version bump) and the read-only `glab`/`gh` rule.
- **Scope anchors:** The issues have no FR in the PRD yet (see `## Open Roadmap Questions`), so each is anchored here; each `MS-NN` number matches its issue number:
  - MS-17: The app starts with Windows into the tray, controlled by a switch in `settings.json` (issue #17).
  - MS-18: A week summary in "Work time": the week's sum against the week's norm, day by day (issue #18).
  - MS-19: An end-of-day summary after STOP: time today, the month's deviation, what was ticked off and what carries over to tomorrow, with no forced decision (issue #19).
  - MS-23: START/STOP in the tray menu and today's work time in the tray tooltip (issue #23).
  - MS-24: The month report copied to the clipboard as text that pastes into a spreadsheet (issue #24).
  - MS-25: Polish public holidays counted out of the norm, computed locally, switchable in `settings.json` (issue #25).
  - MS-26: A configurable daily norm and a half day off (issue #26).
  - MS-27: An optional plain-text note on a work-time entry (issue #27).

## Vision recap

One developer on one Windows 11 machine uses the app every day for their work GitLab, their private GitHub projects, their own tasks, a "To do" list and a work-time counter. M-1 delivered that day flow and M-2 closed the Konsola mockup gaps on a cleaner base. Work-time records start on 2026-12-01; M-3 makes the counter's norm report something the user can trust for monthly settlement and makes the counter reachable without opening the window.

## North star

**S-01: User can see Polish public holidays counted out of the month's norm** — the norm report is the product's main promise ("at the end of each day I know how far I am from the norm"), and without holidays it is wrong in every month with a holiday on a workday, so a quality-first milestone proves itself here first.

> "North star" here means the smallest end-to-end slice whose delivery proves the milestone works. It is placed as early as its prerequisites allow, because the rest of the milestone matters most if this one works.

## At a glance

| ID   | Change ID               | Outcome (user can …)                                                          | Prerequisites | PRD refs              | Status   |
| ---- | ----------------------- | ----------------------------------------------------------------------------- | ------------- | --------------------- | -------- |
| S-01 | public-holidays-norm    | see Polish public holidays counted out of the month's norm                     | —             | MS-25, FR-014, FR-020 | ready    |
| S-02 | configurable-daily-norm | set the daily norm and mark a half day off                                     | S-01          | MS-26, FR-014, FR-020 | proposed |
| S-03 | tray-timer-control      | start and stop the counter from the tray and see today's time in its tooltip   | —             | MS-23, FR-013         | ready    |
| S-04 | windows-autostart       | have the app start with Windows into the tray                                  | —             | MS-17, FR-018         | ready    |
| S-05 | month-report-copy       | copy the month report to the clipboard and paste it into a spreadsheet         | —             | MS-24, FR-014, FR-020 | ready    |
| S-06 | work-entry-note         | add a short note to a work-time entry                                          | —             | MS-27, FR-017         | ready    |
| S-07 | end-of-day-summary      | see a summary of the day after STOP                                            | —             | MS-19, FR-013, FR-014 | ready    |
| S-08 | week-summary            | see the week's sum against the week's norm in "Work time"                      | S-01          | MS-18, FR-014         | proposed |

## Streams

Navigation aid — groups items that share a Prerequisites chain. Canonical ordering still lives in the dependency graph below; this table is the proposed reading order across parallel tracks.

| Stream | Theme                 | Chain                         | Note                                                                                                   |
| ------ | --------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------ |
| A      | Norm rules            | `S-01` → `S-02`, `S-08`       | Quality goal: the norm rule is settled (holidays) before it is extended (custom norm) and summed by week. |
| B      | Tray and start        | `S-03`; `S-04`                | Both touch the tray and app start-up in Rust; plan them one after the other, not at once.               |
| C      | Report and entries    | `S-05`; `S-06`                | Share the "Work time" view; if S-06 lands first, S-05 copies the note too.                              |
| D      | End of day            | `S-07`                        | Standalone; reads today's event log and the report that already exist.                                  |

## Baseline

What's already in place in the codebase as of `2026-11-25` (researched + user-confirmed).
Slices below assume these are present and do NOT re-scaffold them.

- **Frontend:** present — React 19 views per feature; "Work time" has the days table, the reminders panel and the entry list (`src/worktime/`); shared components in `src/components/`.
- **Backend / API:** present — 34 Tauri commands (`src-tauri/src/lib.rs`); `work_time` module with a fixed 8 h norm (`src-tauri/src/work_time/norm.rs:12`), the month report and manual days off (`report.rs`); the tray menu has only "Otwórz", "Sprawdź teraz" and "Zakończ" (`src-tauri/src/runtime.rs:152-185`).
- **Data:** present — app data store in `src-tauri/src/store/` (SQLite, 10 migrations, daily backups); work-time entries carry only start and end; the event log keeps today only.
- **Auth:** not applicable — single user; `glab`/`gh` sessions only.
- **Deploy / infra:** present — `.github/workflows/ci.yml` publishes the NSIS installer on a version bump with release notes; no autostart, no updater.
- **Observability:** present — tray status icon, Windows notices, app log, "Źródła" view, today's event log.

## Foundations

No foundations in M-3: every layer the slices need is present (see `## Baseline`), and each slice brings its own small technical piece (a plugin, a settings key, a migration) inside the slice that first uses it.

## Slices

### S-01: Public holidays in the norm

- **Outcome:** user can see Polish public holidays (fixed dates and the Easter-based ones) counted out of the month's norm and shown in the days table like a day off, with the holiday's name; a setting turns the calendar off.
- **Change ID:** public-holidays-norm
- **PRD refs:** MS-25, FR-014, FR-020
- **Prerequisites:** —
- **Parallel with:** S-03, S-04, S-05, S-06, S-07
- **Blockers:** —
- **Unknowns:**
  - The PRD Non-Goal "no automatic holiday calendar" must change; the user welcomed the calendar on 2026-11-25. — Owner: user. Block: no (recorded in `/10x-plan`).
  - A holiday on a Saturday gives another day off whose date the employer sets: stays a manual day off, with or without a hint in the table? — Owner: user. Block: no (settled in `/10x-plan`).
  - A day that is both a holiday and a marked day off. — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** Changes the norm rule every report shows; it goes first so S-02 and S-08 build on a settled rule, and the rule stays in Rust with tests on known dates.
- **Status:** ready

### S-02: Configurable daily norm and half day off

- **Outcome:** user can set the daily norm in settings (default 8 h) and mark a half day off in the days table, and the report counts both.
- **Change ID:** configurable-daily-norm
- **PRD refs:** MS-26, FR-014, FR-020
- **Prerequisites:** S-01
- **Parallel with:** S-03, S-04, S-05, S-06, S-07, S-08
- **Blockers:** —
- **Unknowns:**
  - Can the norm change within a month (a change of contract), which needs a norm history rather than one setting? — Owner: user. Block: no (settled in `/10x-plan`).
  - Is a half day off really needed, or is a full day enough? — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** Extends the same norm rule as S-01; planned after it so the two do not change the rule at once.
- **Status:** proposed

### S-03: START/STOP from the tray

- **Outcome:** user can start and stop the work-time counter from the tray menu and see today's work time in the tray tooltip next to the source state.
- **Change ID:** tray-timer-control
- **PRD refs:** MS-23, FR-013
- **Prerequisites:** —
- **Parallel with:** S-01, S-02, S-05, S-06, S-07, S-08
- **Blockers:** —
- **Unknowns:**
  - Does STOP from the tray need the same confirmation as the header, and how is it shown? — Owner: user. Block: no (settled in `/10x-plan`).
  - Does the tray icon get a "counter running" variant? — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** The counter is a guardrail ("time is never lost"): the tray must use the same state as the header and the event log, with no second counter.
- **Status:** ready

### S-04: Autostart with Windows

- **Outcome:** user can have the app start with Windows into the tray, without showing the window, controlled by a switch in `settings.json`.
- **Change ID:** windows-autostart
- **PRD refs:** MS-17, FR-018
- **Prerequisites:** —
- **Parallel with:** S-01, S-02, S-05, S-06, S-07, S-08
- **Blockers:** —
- **Unknowns:**
  - On or off by default? — Owner: user. Block: no (settled in `/10x-plan`).
  - The registry entry is a write outside the app's own folders; the AGENTS.md rule needs an exception. — Owner: user. Block: no (recorded in `/10x-plan`).
- **Risk:** Reminders (FR-018) only fire while the app runs; shares start-up code with S-03, so the two are planned one after the other.
- **Status:** ready

### S-05: Month report to the clipboard

- **Outcome:** user can copy the month report (days, entries, sums, deviation, days off) to the clipboard as text that pastes into a spreadsheet.
- **Change ID:** month-report-copy
- **PRD refs:** MS-24, FR-014, FR-020
- **Prerequisites:** —
- **Parallel with:** S-01, S-02, S-03, S-04, S-07, S-08
- **Blockers:** —
- **Unknowns:**
  - Clipboard only, or also a file in a folder set in settings (a file elsewhere breaks the AGENTS.md write rule)? — Owner: user. Block: no (settled in `/10x-plan`).
  - Format: TSV, CSV or Markdown. — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** The text must come from the same Rust report the view shows, so the copied numbers never differ from the screen.
- **Status:** ready

### S-06: Note on a work-time entry

- **Outcome:** user can add, edit and see a short plain-text note on a work-time entry.
- **Change ID:** work-entry-note
- **PRD refs:** MS-27, FR-017
- **Prerequisites:** —
- **Parallel with:** S-01, S-02, S-03, S-04, S-07, S-08
- **Blockers:** —
- **Unknowns:**
  - Entry notes must be added to the AGENTS.md list of what the app keeps in its data dir. — Owner: user. Block: no (recorded in `/10x-plan`).
- **Risk:** A new migration and user content in the store; the note must not reach the app log or the event log.
- **Status:** ready

### S-07: End-of-day summary

- **Outcome:** user can see, after STOP, a summary of the day: time worked today, the month's deviation, what was ticked off and what carries over to tomorrow, with no decision forced.
- **Change ID:** end-of-day-summary
- **PRD refs:** MS-19, FR-013, FR-014
- **Prerequisites:** —
- **Parallel with:** S-01, S-02, S-03, S-04, S-05, S-06, S-08
- **Blockers:** —
- **Unknowns:**
  - After STOP only, or also at a fixed hour; and what happens on STOP from the tray (S-03)? — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** Reads today's event log and the report that already exist; it must not add a second store of the day's state.
- **Status:** ready

### S-08: Week summary

- **Outcome:** user can see in "Work time" the current week's sum against the week's norm, day by day.
- **Change ID:** week-summary
- **PRD refs:** MS-18, FR-014
- **Prerequisites:** S-01
- **Parallel with:** S-02, S-03, S-04, S-05, S-06, S-07
- **Blockers:** —
- **Unknowns:**
  - A section in "Work time" or a separate view; a week that crosses months. — Owner: user. Block: no (settled in `/10x-plan`).
- **Risk:** The week's norm must use the same rule as the month (holidays from S-01), computed in Rust, not summed again in the UI.
- **Status:** proposed

## Backlog Handoff

| Roadmap ID | Change ID               | Suggested issue title                                        | Ready for `/10x-plan` | Notes |
| ---------- | ----------------------- | ------------------------------------------------------------ | --------------------- | ----- |
| S-01       | public-holidays-norm    | Czas pracy: polskie święta państwowe w normie                | yes                   | Issue #25; north star |
| S-02       | configurable-daily-norm | Czas pracy: konfigurowalna norma dzienna i pół dnia wolnego  | no                    | Issue #26; after S-01 |
| S-03       | tray-timer-control      | Zasobnik: START/STOP i bieżący czas pracy                    | yes                   | Issue #23 |
| S-04       | windows-autostart       | Decyzja: autostart z Windows                                 | yes                   | Issue #17; plan after S-03 or before it, not at once |
| S-05       | month-report-copy       | Czas pracy: kopiowanie raportu miesiąca do schowka           | yes                   | Issue #24 |
| S-06       | work-entry-note         | Czas pracy: notatka przy wpisie                              | yes                   | Issue #27 |
| S-07       | end-of-day-summary      | Decyzja: zamknięcie dnia                                     | yes                   | Issue #19 |
| S-08       | week-summary            | Decyzja: widok tygodnia                                      | no                    | Issue #18; after S-01 |

## Open Roadmap Questions

1. **Nazwa produktu** — robocza „Nawigator dnia”. Owner: użytkownik. Block: none.
2. **FR for the M-3 issues** — #17, #18, #19, #23, #24, #26 and #27 have no FR in the PRD, and #25 lifts a PRD Non-Goal ("no automatic holiday calendar"). Add FR-027+ to the PRD (as M-2 did with FR-019–FR-026), or keep them anchored as MS-NN here? Owner: user. Block: none (slices trace to MS-NN).
3. **AGENTS.md rule changes** — S-04 (a registry entry outside the app's folders), S-05 (a file outside the app's folders, if chosen) and S-06 (entry notes in the app data) each need a rule update in AGENTS.md together with the change. Owner: user. Block: none (each settled in its `/10x-plan`).

## Parked

- **Writing to GitLab and GitHub** — Why parked: PRD Non-Goals; read-only (no closing, status changes, comments or assignment).
- **Obsidian vault access** — Why parked: PRD Non-Goals; no reading or writing. The dormant vault code was removed in M-2; any future vault access starts from a new plan, and the PRD Non-Goal is unchanged.
- **Other work-time automation** — Why parked: PRD Non-Goals; no idle detection, activity tracking or automatic stop. The holiday calendar moved into M-3 (S-01) on 2026-11-25; the rest of the Non-Goal stays.
- **MRs where the user is a reviewer; deleting tasks (FR-009)** — Why parked: PRD Non-Goals ("Poza v1"). The reviewer question is open as issue #35 (GitHub milestone M-4).
- **Editing settings in the app** — Why parked: the app reads `settings.json` and never writes it; panels stay read-only.
- **Signed installer (MS-13, issue #13)** — Why parked: decided by the user on 2026-11-24. For one user on one machine, a signature does not remove the SmartScreen warning right away, because reputation builds with downloads. Azure Artifact Signing is not open to individuals in Poland, and an OV certificate costs a yearly fee. Free OSS signing (SignPath Foundation) asks for proof of reputation the project does not have yet. Research: `context/archive/2026-11-24-installer-signing/research.md`. Comes back if the app is ever shared with others and has users to show.
- **Breaks and pomodoro, distraction blocker, morning "plan the day"** — Why parked: PRD "considered, deliberately NOT excluded"; no FR and no issue covers them.
- **GitHub milestones M-4–M-6 (#20, #28–#52)** — Why parked: outside M-3; each opens as the next milestone of this file after M-3 closes (sources, GitHub parity, lists and daily comfort, code tidy-up and distribution).

## Milestone History

- **M-1: The narrowed day flow runs in the app** (`full-day-flow`) — closed 2026-11-23. The day flow from PRD v1 runs in the app: GitLab issues and MRs (filter, hiding, notifications), GitHub issues, own tasks, the "To do" list, and the work-time counter with its norm report and reminders. FR-001–FR-008 and FR-010–FR-018 are covered.
- **M-2: Konsola mockup gaps closed and the codebase cleaned up** (`konsola-gaps-and-cleanup`) — closed 2026-11-24. The Konsola mockup gaps are now in the app: the event log, the days table, the reminders panel, the app-data details, the GitLab NEW flag, hidden date and filter bar, GitHub labels, and "+" to "To do". They sit on a cleaner base (shared UI tools and timer state, no vault layer), and the design-system artifact matches the app. Releases carry a changelog and real metadata (0.2.1, 0.3.0). FR-019–FR-026 and MS-09–MS-12 and MS-14–MS-16 are covered. MS-13 (installer signing) was parked.

## Done

Entries up to 2026-11-23 belong to M-1; their IDs refer to M-1's items (or to its earlier decomposition, archived at `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md`), not to the F-NN/S-NN above. Entries from 2026-11-24 belong to M-2 and refer to M-2's items. New entries for M-3 are appended below them; each entry's IDs refer to the milestone it closed in.

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
