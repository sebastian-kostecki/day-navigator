---
project: "Nawigator dnia"
version: 2
status: draft
created: 2026-11-20
updated: 2026-11-22
prd_version: 2
main_goal: quality
top_blocker: decisions
milestone_id: full-day-flow
milestone_seq: 1
milestone_status: open
---

# Roadmap: Nawigator dnia

> Derived from `context/foundation/prd.md` (v2) + `context/foundation/tech-stack.md` + auto-researched codebase baseline.
> Edit-in-place; the next milestone continues this file. Archive only when an independent roadmap replaces it.
> Slices below are listed in dependency order. The "At a glance" table is the index.

## Milestone

**M-1: The full day flow runs in the app** — Status: open

- **Intent:** The PRD's day flow (steps 1–9: autostart → issue sync → "plan your day" → morning plan → work time → new-issue notification → breaks, focus blocks and blocker → end of work with day and month sums) runs in the app for both spaces (Work and Private), with the app as the single writer of issue notes and the time record, without breaking the note-integrity and data-boundary guardrails.
- **Source materials:** `context/foundation/prd.md` (v2)
- **Done when:** every F-NN and S-NN below is `done`.
- **Scope anchors:** FR-001–FR-007, FR-009–FR-024, FR-026, FR-027 (all must-have FRs), US-01, US-02, all five NFRs, Access Control. FR-008 and FR-025 (nice-to-have) are parked.

## Vision recap

A single developer on one Windows 11 machine switches during the day between company work (GitLab, work vault, Helper App, monthly work-time records) and private projects (GitHub, private vault, `context/` in WSL repos). The data already exists, but nothing fetches it automatically, nothing notifies in Windows, and nothing gives the day a rhythm, so the day starts without priorities, assigned issues are noticed late and the time record has gaps. The app is a local desktop layer that pulls the data itself, notifies, and keeps manual input limited to the user's own thoughts and decisions.

## North star

**S-01: User can make the morning plan in "Today" and have it saved in the work vault's daily note** — the north star is the smallest end-to-end slice whose success proves the core product idea, so it is placed as early as its prerequisites allow; here it directly proves primary success criterion #1 ("plan made in the app every workday") and the PRD's main pain point, and with `main_goal: quality` it is the first slice to exercise the safe vault-write contract on a real note.

## At a glance

| ID   | Change ID                 | Outcome (user can …)                                                                   | Prerequisites | PRD refs                      | Status   |
| ---- | ------------------------- | -------------------------------------------------------------------------------------- | ------------- | ----------------------------- | -------- |
| F-01 | safe-vault-access         | (foundation) guarded vault read/write: configured folders only, section-scoped writes   | —             | NFR-integrity, Access Control | done     |
| F-02 | source-health-status      | (foundation) every source failure shows in the tray icon and as a Windows notice        | —             | NFR-health, FR-001            | done     |
| F-03 | ci-release-pipeline       | (foundation) CI builds and tests every change; merges to main publish an installer      | —             | NFR-no-secrets, NFR-work-data | ready    |
| S-01 | morning-plan-today        | make the morning plan in "Today" and save it in the work daily note                     | F-01, F-02    | US-01, FR-003, FR-004         | ready    |
| S-02 | autostart-plan-nudge      | have the app start with Windows and get "Plan your day" on the first workday unlock     | S-01, F-03    | FR-001, FR-002, FR-023        | proposed |
| S-03 | quick-capture-inbox       | capture a thought via global shortcut and triage the inbox                               | S-01          | FR-005, FR-006, FR-007        | proposed |
| S-04 | issue-list-to-today       | browse work issues, pull one onto today, open it in the right Obsidian vault             | S-01          | FR-012, FR-014                | proposed |
| S-05 | work-tasks-from-template  | create a work task from the template and change its status                               | F-01          | FR-013                        | ready    |
| S-06 | settings-first-run-wizard | configure vaults, repos, tools, excluded paths and labels via a first-run wizard          | F-01          | FR-022                        | ready    |
| S-07 | gitlab-issue-sync-notify  | get assigned GitLab issues synced by the app and notified with "add to list"              | S-01, F-02    | US-02, FR-010, FR-011         | proposed |
| S-08 | time-history-migration    | move the work-time history into daily notes, verified by matching monthly sums            | S-01, F-01    | FR-027                        | proposed |
| S-09 | work-time-tracking        | start and stop work time, recorded in the daily note, with day and month sums             | S-08, F-02    | FR-016, FR-024                | proposed |
| S-10 | work-hours-reminders      | get reminded when it's work time and time isn't running, or it runs after hours           | S-09, S-02    | FR-017, FR-023                | proposed |
| S-11 | idle-detection            | get asked to stop work time when idle, and reminded when active with time stopped         | S-10          | FR-026                        | proposed |
| S-12 | break-reminders           | get micro and long break reminders during work time, snooze or pause them                 | S-09, S-02    | FR-018, FR-024                | proposed |
| S-13 | focus-blocks              | run a focus block on a task and have it logged in the day log                             | S-01          | FR-019, FR-005                | proposed |
| S-14 | distraction-blocker       | block listed sites and programs on a schedule and turn it off after a confirmation       | S-02, F-02    | FR-020, FR-021                | proposed |
| S-15 | end-of-day-close          | close the day: review done items, decide on leftovers, draft tomorrow                     | S-01, S-03    | FR-009, FR-005                | proposed |
| S-16 | private-issues-in-plan    | see private GitHub issues in "Today" and pull one into the private daily note             | S-01          | FR-015, FR-003                | proposed |
| S-17 | private-projects-overview | see private vault notes and each project's state from `context/` in the repos             | F-01, F-02    | FR-015                        | ready    |

## Streams

Navigation aid — groups items that share a Prerequisites chain. Canonical ordering still lives in the dependency graph below; this table is the proposed reading order across parallel tracks.

| Stream | Theme                   | Chain                                               | Note                                                                                     |
| ------ | ----------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| A      | Planning core           | `F-01` → `S-01` → `S-03` → `S-13` → `S-15`          | Safe vault writes first (quality goal), then the north star and the rest of the daily note. |
| B      | Health and work issues  | `F-02` → `S-04` → `S-05` → `S-07`                   | Joins Stream A at `S-01`; the app becomes the only writer of `Issues/`.                   |
| C      | Background rhythm       | `F-03` → `S-02` → `S-14`                            | Installed build enables autostart; the blocker relies on the background app and day-off mode. |
| D      | Work time               | `S-08` → `S-09` → `S-10` → `S-11` → `S-12`          | Joins Stream A at `S-01`; migration is verified before the app takes over the time record. |
| E      | Private space and setup | `S-06` → `S-16` → `S-17`                            | Mostly independent of the work-side chains; `S-16` joins Stream A at `S-01`.              |

## Baseline

What's already in place in the codebase as of `2026-11-20` (auto-researched + user-confirmed).
Foundations below assume these are present and do NOT re-scaffold them.

- **Frontend:** partial — Tauri starter UI only (`src/App.tsx` is the template demo); no "Today" screen.
- **Backend / API:** partial — Tauri builder in `src-tauri/src/lib.rs` with one demo command and the opener plugin; no tray, autostart, shortcut, notifications or scheduling.
- **Data:** absent — no settings store and no vault read/write (data lives as Markdown in the user's vaults).
- **Auth:** not applicable — single user, no login (PRD Access Control); GitLab/GitHub are reached only through the logged-in `glab`/`gh` tools, and no such integration exists yet.
- **Deploy / infra:** absent — no CI workflows and no Git remote; `tech-stack.md` plans installers published to GitHub Releases by GitHub Actions on merge to `main`.
- **Observability:** absent — no logging and no health status.

## Foundations

### F-01: Guarded vault access

- **Outcome:** (foundation) the app reads and writes only inside vault folders configured per space (Work, Private), never sees excluded paths, changes only the targeted line or section of a note (rest of the file byte-identical), and sends deletions to the vault trash — proven against a small invented test vault.
- **Change ID:** safe-vault-access
- **PRD refs:** NFR "Nienaruszalność notatek", NFR "Dane pracy nie opuszczają maszyny" (tests use invented data), Access Control ("Tylko wskazane miejsca", "Wykluczone ścieżki", "Rozdział Praca/Prywatne")
- **Unlocks:** S-01 (first daily-note write), S-05, S-06, S-08, S-17; verification path: invented test vault for every later slice that writes notes.
- **Prerequisites:** —
- **Parallel with:** F-02, F-03
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Sequenced first because `main_goal: quality` and the note-integrity guardrail make any vault write unsafe without it; kept to the access contract only — note formats (daily note, inbox, tasks, issues) belong to the slices that use them.
- **Status:** done

### F-02: Source health status

- **Outcome:** (foundation) the app lives in the tray with a status icon (OK / source error / not running), and any source failure (missing configured path, failed command, logged-out `glab`/`gh`) is reported there and as a Windows notice within 1 minute; screens show the error instead of an empty list.
- **Change ID:** source-health-status
- **PRD refs:** NFR "Widoczny stan zdrowia", FR-001 (Socrates resolution: tray icon shows state), US-01 acceptance criterion (missing vault path → explicit message)
- **Unlocks:** S-01 (error state instead of an empty plan), S-07 (sync failures), S-09, S-14, S-17.
- **Prerequisites:** —
- **Parallel with:** F-01, F-03
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Placed before user-facing slices because the quality goal does not defer observability, and the PRD forbids showing an empty plan instead of an error; limited to the reporting channel, not to any specific source.
- **Status:** done

### F-03: CI and release pipeline

- **Outcome:** (foundation) a private GitHub repository exists, every change is built and tested in CI against invented data only, and each merge to `main` publishes a Windows installer as a GitHub Release named "Nawigator dnia".
- **Change ID:** ci-release-pipeline
- **PRD refs:** NFR "Zero sekretów", NFR "Dane pracy nie opuszczają maszyny" (nothing from work in repo or CI), Open Question #7 (private now, may go public later), Open Question #1 (name); deployment target from `tech-stack.md`
- **Unlocks:** S-02 (autostart needs an installed build); verification path: CI gate for F-01's invented-vault tests.
- **Prerequisites:** —
- **Parallel with:** F-01, F-02, S-01, S-03–S-09, S-13, S-15–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Needed before autostart can be used daily; because the repo may go public later, nothing work-related may ever enter its history.
- **Status:** ready

## Slices

### S-01: Morning plan in "Today"

- **Outcome:** User can open "Today", see the Work section — own tasks, assigned issues and MRs already in the work vault (configured labels highlighted) and yesterday's leftovers, each requiring a today / inbox / drop decision — in the app's recommended order, pick 1–3 priorities plus an "if time" list, and have the plan saved in the work vault's daily note with links. The `Today` field in issues and tasks is not written, and the old lists (`DASHBOARD.md`, `ToDo List.md`, `Notatki bieżące.md`) are neither read nor written.
- **Change ID:** morning-plan-today
- **PRD refs:** US-01, FR-003, FR-004, Business Logic ("Wejścia i rekomendacja", "Workflow pozycji")
- **Prerequisites:** F-01, F-02, issue notes already present in the work vault (written today by Helper App's manual sync)
- **Parallel with:** F-03, S-05, S-06, S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** First real write to a live vault note; it also defines the daily-note format that the time migration (S-08) and the day log build on.
- **Status:** ready

### S-02: Autostart and "Plan your day" nudge

- **Outcome:** User can have the app start with Windows into the tray and, on the first startup or unlock of a workday with no plan yet, get a "Plan your day" notification that opens "Today" (with a fallback time from settings); workdays (default Mon–Fri) and a day-off mode are configurable, and a day off silences the app.
- **Change ID:** autostart-plan-nudge
- **PRD refs:** FR-001, FR-002, FR-023, Business Logic ("Rytm")
- **Prerequisites:** S-01, F-03
- **Parallel with:** S-03–S-09, S-13, S-15–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Needs "Today" to exist (the nudge opens it and checks whether a plan exists) and an installed build for autostart; silent startup failures are covered by F-02.
- **Status:** proposed

### S-03: Quick capture and inbox

- **Outcome:** User can press Ctrl+Alt+Space (default) anywhere in Windows, pick a space (Work by default while work time runs), save a thought to the day log or the inbox, and later triage each inbox item to today, tomorrow or delete; a non-empty inbox shows up in the morning plan.
- **Change ID:** quick-capture-inbox
- **PRD refs:** FR-006, FR-007, FR-005 (manual entries)
- **Prerequisites:** S-01
- **Parallel with:** F-03, S-02, S-04–S-14, S-16, S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Writes to both spaces, so a wrong default space would leak a private thought into the work vault — the space switch is the core of this slice.
- **Status:** proposed

### S-04: Work issue list to today

- **Outcome:** User can browse assigned work issues and tasks in the app, pull any of them onto today's list (including issues assigned long ago), and open its note in the right Obsidian vault with one click — or get an explicit message when that isn't possible.
- **Change ID:** issue-list-to-today
- **PRD refs:** FR-012, FR-014
- **Prerequisites:** S-01
- **Parallel with:** F-03, S-02, S-03, S-05–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Opening the wrong vault is the failure the PRD calls out; must fail loudly rather than silently.
- **Status:** proposed

### S-05: Work tasks from template

- **Outcome:** User can create a task from the template in the work vault and change its status from the app.
- **Change ID:** work-tasks-from-template
- **PRD refs:** FR-013
- **Prerequisites:** F-01
- **Parallel with:** F-02, F-03, S-01–S-04, S-06–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Status change is a single-line header edit — the first test of F-01's line-scoped write on existing user notes.
- **Status:** ready

### S-06: Settings and first-run wizard

- **Outcome:** User can set vault, repository and tool paths per space, excluded paths and highlighted issue labels in the app, guided by a wizard on first run.
- **Change ID:** settings-first-run-wizard
- **PRD refs:** FR-022
- **Prerequisites:** F-01
- **Parallel with:** F-02, F-03, S-01–S-05, S-07–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Until this lands, configuration comes from F-01's minimal settings; the wizard must never create a missing path silently (Access Control).
- **Status:** ready

### S-07: GitLab issue sync and notifications

- **Outcome:** User can have assigned GitLab issues and MRs synced by the app (as the only writer of `Issues/`, keeping the existing note format) on an interval and on demand ("sync now"), get a Windows notification for each new one with "add to list" (one summary notification from 3 new ones), and adding puts it on "if time" without touching the 1–3 priorities.
- **Change ID:** gitlab-issue-sync-notify
- **PRD refs:** US-02, FR-010, FR-011
- **Prerequisites:** S-01, F-02
- **Parallel with:** F-03, S-02–S-06, S-08–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Replaces Helper App's sync, so existing issue notes must keep their format and fields; until the user disables Helper App's sync command in Obsidian there are two writers, so the switch-over is part of this slice.
- **Status:** proposed

### S-08: Work-time history migration

- **Outcome:** User can move the existing work-time history from `Timeline.md` into daily notes in one run (dry run first), and the `Timeline.md` generated from the daily notes matches the old monthly sums; only then does the app take over the time record.
- **Change ID:** time-history-migration
- **PRD refs:** FR-027, Open Question #3 (resolution)
- **Prerequisites:** S-01, F-01
- **Parallel with:** F-03, S-02–S-07, S-13–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Creates or edits many daily notes at once and rewrites a billing-relevant file; monthly sums are the acceptance gate, while the format of individual entries may change, so Helper App's mail report must be checked against the regenerated file.
- **Status:** proposed

### S-09: Work time tracking

- **Outcome:** User can start and stop work time with a click (in the window and from the tray menu), have start and end written to the daily note, see the day and month sums, and have `Timeline.md` regenerated from the daily notes — the only time record.
- **Change ID:** work-time-tracking
- **PRD refs:** FR-016, FR-024 (start/stop from tray)
- **Prerequisites:** S-08, F-02
- **Parallel with:** F-03, S-02–S-07, S-13–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Work time is billed monthly; once this ships, Helper App's start/stop commands in Obsidian must be disabled so there is one writer.
- **Status:** proposed

### S-10: Work-hours reminders

- **Outcome:** User gets one reminder when it is work time on a workday (default Mon–Fri 8:00–16:00) and work time is not running, and one when it keeps running after work hours; work hours are configurable.
- **Change ID:** work-hours-reminders
- **PRD refs:** FR-017, FR-023 (notification hours)
- **Prerequisites:** S-09, S-02
- **Parallel with:** S-03–S-07, S-12–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** False alarms with flexible hours; one reminder per event, never a series.
- **Status:** proposed

### S-11: Idle detection

- **Outcome:** User gets asked whether to stop work time when the computer has been idle for N minutes (configurable) while time runs, and reminded when using the computer during work hours with time stopped; the app detects only activity, never window contents or program names.
- **Change ID:** idle-detection
- **PRD refs:** FR-026
- **Prerequisites:** S-10
- **Parallel with:** S-03–S-07, S-12–S-17
- **Blockers:** —
- **Unknowns:**
  - Default idle threshold N (not set by the PRD) — Owner: user. Block: no.
- **Risk:** Overlaps with S-10's reminders; must not double-notify for the same event.
- **Status:** proposed

### S-12: Break reminders

- **Outcome:** User gets micro-break (default every 20 min for 20 s) and longer-break (default every 60 min for 5 min) reminders while work time runs, can snooze or skip each one, pause breaks for N minutes with one click ("not now"), and pause all app notifications from the tray for a set time after which they resume.
- **Change ID:** break-reminders
- **PRD refs:** FR-018, FR-024 (time-limited pause)
- **Prerequisites:** S-09, S-02
- **Parallel with:** S-03–S-07, S-10, S-11, S-13–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** A forgotten pause would silence the app for good — every pause must be time-limited.
- **Status:** proposed

### S-13: Focus blocks

- **Outcome:** User can start a focus block (default 25 min) on a task, get a break notification when it ends, and have the completed block appended to the day log with the task name; background break reminders stay quiet during a block.
- **Change ID:** focus-blocks
- **PRD refs:** FR-019, FR-005 (automatic day-log entries)
- **Prerequisites:** S-01
- **Parallel with:** F-03, S-02–S-12, S-14–S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Interacts with S-12 (breaks are suppressed during a block); whichever lands second wires the rule.
- **Status:** proposed

### S-14: Distraction blocker

- **Outcome:** User can define lists of blocked websites and programs and a blocking schedule in the app; during the schedule they are unavailable, the app asks for elevated-permission consent explicitly and reverts its system changes when blocking ends, blocking can be turned off any time after a short confirmation, and a day off disables it.
- **Change ID:** distraction-blocker
- **PRD refs:** FR-020, FR-021, Access Control ("Jawny wyjątek: bloker rozpraszaczy")
- **Prerequisites:** S-02, F-02
- **Parallel with:** S-03–S-13, S-15–S-17
- **Blockers:** —
- **Unknowns:**
  - Which programs and sites count as work tools that must never be blocked (guardrail)? — Owner: user. Block: no.
- **Risk:** The only feature allowed outside configured folders and the one that changes the system; a failure to revert would break work, which the guardrail forbids.
- **Status:** proposed

### S-15: End-of-day close

- **Outcome:** User can close the day: see what got done (closed issues and tasks fed into the day log automatically), decide the fate of each unfinished item (tomorrow / inbox / drop), write one sentence about the day and draft tomorrow.
- **Change ID:** end-of-day-close
- **PRD refs:** FR-009, FR-005 (automatic day-log entries)
- **Prerequisites:** S-01, S-03
- **Parallel with:** F-03, S-02, S-04–S-14, S-16, S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Without the close, leftovers carry over undecided and the plan never closes; depends on the inbox as one of the destinations.
- **Status:** proposed

### S-16: Private issues in the plan

- **Outcome:** User can see open GitHub issues and PRs from their repositories in the Private section of "Today" and pull an open issue into the private part of the plan, saved in the private vault's daily note; the 1–3 priority limit is shared with Work and nothing is synced or notified.
- **Change ID:** private-issues-in-plan
- **PRD refs:** FR-015, FR-003 (Private section)
- **Prerequisites:** S-01
- **Parallel with:** F-03, S-02–S-15, S-17
- **Blockers:** —
- **Unknowns:** —
- **Risk:** `gh` is available only inside WSL; data from the two spaces must never cross.
- **Status:** proposed

### S-17: Private projects overview

- **Outcome:** User can see notes from the private vault and the state of each private project read from `context/` in the repositories (read-only).
- **Change ID:** private-projects-overview
- **PRD refs:** FR-015
- **Prerequisites:** F-01, F-02
- **Parallel with:** F-03, S-01–S-16
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Repositories live in WSL; `context/` is read-only for the app, and unreadable repos must show as errors, not as empty projects.
- **Status:** ready

## Backlog Handoff

| Roadmap ID | Change ID                 | Suggested issue title                                      | Ready for `/10x-plan` | Notes |
| ---------- | ------------------------- | ---------------------------------------------------------- | --------------------- | ----- |
| F-01       | safe-vault-access         | Guarded vault read/write with section-scoped edits         | done                  | Archived 2026-11-20 |
| F-02       | source-health-status      | Tray health status and source-failure notices              | done                  | Archived 2026-11-21 |
| F-03       | ci-release-pipeline       | Private repo, CI build/test and installer release on merge | yes                   | Run `/10x-plan ci-release-pipeline` |
| S-01       | morning-plan-today        | Morning plan in "Today" saved to the work daily note       | yes                   | Run `/10x-plan morning-plan-today` — the north star |
| S-02       | autostart-plan-nudge      | Autostart and "Plan your day" notification                 | no                    | Needs S-01, F-03 |
| S-03       | quick-capture-inbox       | Global-shortcut quick capture and inbox triage             | no                    | Needs S-01 |
| S-04       | issue-list-to-today       | Work issue list: pull to today, open in Obsidian           | no                    | Needs S-01 |
| S-05       | work-tasks-from-template  | Create work tasks from template and change status          | yes                   | Run `/10x-plan work-tasks-from-template` |
| S-06       | settings-first-run-wizard | Settings screen and first-run wizard                       | yes                   | Run `/10x-plan settings-first-run-wizard` |
| S-07       | gitlab-issue-sync-notify  | App-owned GitLab issue sync with notifications             | no                    | Needs S-01, F-02 |
| S-08       | time-history-migration    | Migrate work-time history into daily notes                 | no                    | Needs S-01, F-01 |
| S-09       | work-time-tracking        | Work time in daily notes with generated Timeline.md        | no                    | Needs S-08, F-02 |
| S-10       | work-hours-reminders      | Work-hours reminders                                       | no                    | Needs S-09, S-02 |
| S-11       | idle-detection            | Idle detection for the time record                         | no                    | Needs S-10 |
| S-12       | break-reminders           | Break reminders with snooze and timed pause                | no                    | Needs S-09, S-02 |
| S-13       | focus-blocks              | Focus blocks logged to the day log                         | no                    | Needs S-01 |
| S-14       | distraction-blocker       | Scheduled distraction blocker with confirmed off switch    | no                    | Needs S-02, F-02 |
| S-15       | end-of-day-close          | End-of-day close                                           | no                    | Needs S-01, S-03 |
| S-16       | private-issues-in-plan    | Private GitHub issues in the plan                          | no                    | Needs S-01 |
| S-17       | private-projects-overview | Private vault notes and project state from context/        | yes                   | Run `/10x-plan private-projects-overview` |

## Open Roadmap Questions

None open. All eight PRD Open Questions were resolved by the user on 2026-11-20; the resolutions are recorded in `context/foundation/prd.md` (`## Open Questions` and the decision notes under FR-004, FR-010, FR-016) and reflected in the slices above. Remaining per-slice unknowns (idle threshold in S-11, never-blocked work tools in S-14) are non-blocking.

## Parked

- **Week view (FR-008)** — Why parked: nice-to-have per PRD; supports no primary success criterion.
- **Private tasks from template (FR-025)** — Why parked: nice-to-have per PRD; private projects run on GitHub issues.
- **Program-usage log** — Why parked: the user limited activity tracking in v1 to idle detection (FR-026); a per-program log would record window and program data.
- **Own note editor** — Why parked: PRD Non-Goals; full Markdown editing stays in Obsidian ("Open in Obsidian", FR-014).
- **Phone and multiple machines** — Why parked: PRD Non-Goals; one Windows 11 computer.
- **Multiple users** — Why parked: PRD Non-Goals; personal tool, no accounts or roles.
- **Writing to GitLab/GitHub, integrations (Domowy Skarbnik, Menu Maker), sharing data with an AI assistant, a tamper-proof blocker** — Why parked: PRD Open Question #8 resolved — not in v1.

## Milestone History

## Done

- **F-01: (foundation) the app reads and writes only inside vault folders configured per space (Work, Private), never sees excluded paths, changes only the targeted line or section of a note (rest of the file byte-identical), and sends deletions to the vault trash — proven against a small invented test vault.** — Archived 2026-11-20 → `context/archive/2026-11-20-safe-vault-access/`. Lesson: —.
- **F-02: (foundation) the app lives in the tray with a status icon (OK / source error / not running), and any source failure (missing configured path, failed command, logged-out `glab`/`gh`) is reported there and as a Windows notice within 1 minute; screens show the error instead of an empty list.** — Archived 2026-11-21 → `context/archive/2026-11-20-source-health-status/`. Lesson: —.
