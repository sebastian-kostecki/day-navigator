---
project: "Nawigator dnia"
version: 3
status: draft
created: 2026-11-20
updated: 2026-11-23
prd_version: 1
main_goal: quality
top_blocker: decisions
milestone_id: full-day-flow
milestone_seq: 1
milestone_status: open
---

# Roadmap: Nawigator dnia

> Derived from `context/foundation/prd.md` (brownfield, v1 of 2026-11-22) + auto-researched codebase baseline.
> Edit-in-place; the next milestone continues this file. Archive only when an independent roadmap replaces it.
> Slices below are listed in dependency order. The "At a glance" table is the index.
> Re-decomposed on 2026-11-23 for the narrowed MVP; the previous decomposition of this milestone is archived at `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md`.

## Milestone

**M-1: The narrowed day flow runs in the app** — Status: open

- **Intent:** The PRD's day flow 1–8 runs in the app: assigned GitLab issues and MRs with hiding and notifications, assigned GitHub issues, own tasks, a "To do" list picked from the three lists, and a work-time counter with a monthly-norm report. It holds the guardrails throughout: time is never lost, no VPN is not an empty list, work data never leaks, vaults stay untouched.
- **Source materials:** `context/foundation/prd.md` (v1, brownfield, 2026-11-22)
- **Done when:** every F-NN and S-NN below is `done`, and the preserved behaviour (FR-016: CI green, an installer only on a version bump) still holds.
- **Scope anchors:** FR-001–FR-008, FR-010–FR-018 (all must-have; FR-009 was removed in the PRD), US-01, US-02, all five NFRs under `## Constraints & Compatibility`, and the `[removed] S-05` item of `## Scope of Change`.

## Vision recap

One developer on one Windows 11 machine works for a company (issues in a work GitLab that is reachable only over VPN, work time settled monthly) and on private projects (issues on GitHub). The previous plan was too big. The narrowed MVP needs to be usable quickly: one window with three sources (GitLab, GitHub, own tasks), a "To do" list picked from them, and a work-time counter that says how far the month is from the 8 h/day norm. The app only shows what needs attention, reads from GitLab and GitHub without writing there, and does not touch Obsidian vaults.

## North star

**S-01: User can see their assigned GitLab issues and MRs in the "Work (GitLab)" view** — the user named GitLab as the most urgent source, and this slice proves the whole source path end to end: logged-in CLI → list → a calm "GitLab unavailable" state without VPN.

> The north star is the smallest end-to-end slice whose delivery proves the product works. It is placed as early as its prerequisites allow, because everything else matters only if this works.

## At a glance

| ID   | Change ID               | Outcome (user can …)                                                                  | Prerequisites | PRD refs                                  | Status   |
| ---- | ----------------------- | ------------------------------------------------------------------------------------- | ------------- | ----------------------------------------- | -------- |
| F-01 | app-data-store          | (foundation) app-owned data survives close, restart and crash                          | —             | NFR Trwałość czasu pracy, Constraints      | done |
| F-02 | read-only-cli-runner    | (foundation) logged-in `glab`/`gh` run read-only, with classified result states        | —             | Access Control Changes, NFR Nigdy pusta lista | done |
| S-01 | gitlab-assigned-list    | see assigned GitLab issues and MRs, or a calm "unavailable" state without VPN          | F-02          | FR-001, FR-015, FR-016, US-01              | done |
| S-02 | gitlab-visibility-filter| choose which assigned GitLab items the list shows                                      | S-01          | FR-002                                    | ready |
| S-03 | hide-and-resurface      | hide an issue and get it back when its status changes or someone else comments         | S-01, F-01    | FR-003, US-02                             | ready |
| S-04 | new-assignment-notify   | get a Windows notification for every new assigned issue or MR                          | S-01, F-01    | FR-004, NFR Świeżość, NFR Lekkość w tle    | ready |
| S-05 | github-assigned-issues  | see open GitHub issues assigned to them from the repositories listed in settings       | F-02          | FR-005, FR-015                            | blocked  |
| S-06 | own-tasks               | add, edit and complete their own tasks in the Work or Private space                    | F-01          | FR-006, FR-007, FR-008, FR-010            | done     |
| S-07 | todo-list               | pick today's items from the lists into "To do", tick them, carry the rest to tomorrow  | S-01, S-06    | FR-011, FR-012, US-01                     | done |
| S-08 | work-timer              | start and stop work time from an always-visible counter and correct entries            | F-01          | FR-013, FR-017, NFR Trwałość czasu pracy  | done |
| S-09 | work-norm-report        | see day and month sums and how far the month is from the norm                          | S-08          | FR-014                                    | ready |
| S-10 | timer-reminders         | get reminded when the counter runs too long or is not running on a workday             | S-08          | FR-018                                    | ready |

## Streams

Navigation aid — groups items that share a Prerequisites chain. Canonical ordering still lives in the dependency graph below; this table is the proposed reading order across parallel tracks.

| Stream | Theme              | Chain                                                       | Note                                                                                              |
| ------ | ------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| A      | Issue sources      | `F-02` → `S-01` → `S-02` → `S-03` → `S-04`, plus `S-05`      | The north star first; `S-05` branches off `F-02` and waits on the `gh` decision; `S-03`/`S-04` also need `F-01`. |
| B      | Own data and "To do" | `F-01` → `S-06` → `S-07`                                  | Durable store first (quality goal); `S-07` joins Stream A at `S-01`.                              |
| C      | Work time          | `S-08` → `S-09` → `S-10`                                     | Starts from `F-01` in Stream B; can run alongside Stream A from the start.                         |

## Baseline

What's already in place in the codebase as of `2026-11-23` (auto-researched + user-confirmed).
Foundations below assume these are present and do NOT re-scaffold them.

- **Frontend:** partial — React 19 + Vite; `src/App.tsx` stacks `TasksView` (S-05 of the previous decomposition) and `HealthPanel`; no navigation, no header counter, none of the MVP views. Conventions documented in `AGENTS.md`.
- **Backend / API:** partial — Tauri commands for vault tasks and health (`src-tauri/src/lib.rs`); tray, notifications, single instance and log (`src-tauri/src/runtime.rs`); no `glab`/`gh` invocation anywhere.
- **Data:** partial — `settings.json` in the app config dir (vault paths, task settings); the vault layer exists but the MVP does not use it; no app-owned data store for tasks, "To do", hidden issues or work time.
- **Auth:** not applicable — single user, no login; GitLab and GitHub are reached through the logged-in CLIs, not wired yet.
- **Deploy / infra:** present — CI with secret scan, audits, lint/format, build and tests, plus the installer published on a version bump (`.github/workflows/ci.yml`).
- **Observability:** present — tray status icon and Windows notices on source failure (previous F-02), app log; the probes cover settings and vault paths only, not `glab`/`gh`.

## Foundations

### F-01: App-owned data store

- **Outcome:** (foundation) the app keeps its own data (own tasks, the "To do" list, hidden-issue snapshots, work-time entries) in its app data dir. A write is never half-applied, and the data survives closing the app, restarting Windows and a crash — proven by tests on invented data.
- **Change ID:** app-data-store
- **PRD refs:** NFR "Trwałość czasu pracy", Constraints & Compatibility ("Vaulty Obsidian: MVP ich nie czyta i nie zapisuje"), Guardrail "Czas pracy nie ginie"
- **Unlocks:** S-03, S-04, S-06, S-07, S-08; verification path: crash/restart durability tests every data slice reuses.
- **Prerequisites:** —
- **Parallel with:** F-02, S-01, S-02, S-05
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Sequenced first because the quality goal and the "time is never lost" guardrail make every data slice unsafe without it. Kept to the storage contract only; each slice defines its own records.
- **Status:** done

### F-02: Read-only CLI runner

- **Outcome:** (foundation) the app runs the logged-in `glab`/`gh` with a fixed program and fixed arguments, read-only, and reports each call as one of: data, unavailable (no VPN/network), logged out, tool missing, error. A fake CLI makes these states testable without any work data.
- **Change ID:** read-only-cli-runner
- **PRD refs:** Access Control Changes (sessions only, zero own secrets), Constraints & Compatibility ("Aplikacja tylko czyta z GitLaba i GitHuba"), NFR "Nigdy pusta lista zamiast błędu", NFR "Dane pracy i vaulty"
- **Unlocks:** S-01, S-05; reduces Open Roadmap Question #1 (`gh` on Windows); verification path: fake-CLI tests for every source slice.
- **Prerequisites:** —
- **Parallel with:** F-01, S-06, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Placed before the north star because a read-only, no-shell call path and invented-data tests are hard rules. Building them inside the first slice would mix the security contract with the view. This foundation does not cover listing issues; that is S-01.
- **Status:** done

## Slices

### S-01: Assigned GitLab issues and MRs in "Work (GitLab)"

- **Outcome:** user can open the "Work (GitLab)" view and see the issues and MRs assigned to them. Without VPN the view says "GitLab unavailable" calmly, with the time of the last successful refresh, and raises no alarm. A logged-out `glab` shows as an error in the tray. The app gets its navigation between views and the fixed header area.
- **Change ID:** gitlab-assigned-list
- **PRD refs:** FR-001, FR-015, FR-016 (preserved — the north star ships through the existing release path), US-01 (first half), NFR "Nigdy pusta lista zamiast błędu"
- **Prerequisites:** F-02
- **Parallel with:** F-01, S-05, S-06, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** The north star; it is the first time real work data passes through the app, so the guardrail that no work data reaches logs or tests is exercised here first.
- **Status:** done

### S-02: Visibility filter for GitLab items

- **Outcome:** user can set in settings which assigned items the list shows — by project, labels, milestone and state.
- **Change ID:** gitlab-visibility-filter
- **PRD refs:** FR-002
- **Prerequisites:** S-01
- **Parallel with:** S-03, S-04, S-05, S-06, S-07, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** The filter applies to the list only; notifications deliberately bypass it (FR-004), so S-04 must not reuse the filtered set.
- **Status:** ready

### S-03: Hide an issue and resurface it on change

- **Outcome:** user can hide an issue. It comes back, marked as changed, when its status changes or someone other than the user (bots included) comments on it.
- **Change ID:** hide-and-resurface
- **PRD refs:** FR-003, US-02
- **Prerequisites:** S-01, F-01
- **Parallel with:** S-02, S-04, S-05, S-06, S-07, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Needs a stored snapshot of each hidden item to compare against. The user's own comments must not count, which depends on knowing the logged-in identity.
- **Status:** ready

### S-04: Notification for new assignments

- **Outcome:** user gets a Windows notification for every new issue or MR assigned to them, whatever the filter says, within 15 minutes while on VPN; more than 3 at once arrive as one combined notification.
- **Change ID:** new-assignment-notify
- **PRD refs:** FR-004, NFR "Świeżość", NFR "Lekkość w tle"
- **Prerequisites:** S-01, F-01
- **Parallel with:** S-02, S-03, S-05, S-06, S-07, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:**
  - Default refresh frequency (must meet ≤ 15 min; PRD Open Question #3) — Owner: user. Block: no.
- **Risk:** Background refresh is the first always-on work in the app; it must stay light and must not turn a reconnected VPN into a burst of notifications.
- **Status:** ready

### S-05: Assigned GitHub issues in "Projects (GitHub)"

- **Outcome:** user can open the "Projects (GitHub)" view and see the open issues assigned to them from the repositories listed in settings; a logged-out or missing `gh` shows as an error, not an empty list.
- **Change ID:** github-assigned-issues
- **PRD refs:** FR-005, FR-015
- **Prerequisites:** F-02
- **Parallel with:** F-01, S-01, S-02, S-03, S-04, S-06, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:**
  - How does the Windows app reach GitHub, given `gh` is installed only in WSL — install it on Windows, or call it through WSL? (PRD Open Question #1) — Owner: user. Block: yes.
- **Risk:** Kept off the north star path because of the `gh` decision; once that is made, it reuses F-02 and the view pattern from S-01.
- **Status:** blocked

### S-06: Own tasks

- **Outcome:** user can add a task in the Work or Private space (default Work while the counter runs, otherwise Private), edit its title and short plain note, set an optional due date and priority, and mark it done. The vault-based task screen from the previous decomposition is removed.
- **Change ID:** own-tasks
- **PRD refs:** FR-006, FR-007, FR-008, FR-010, Scope of Change `[removed] S-05`
- **Prerequisites:** F-01
- **Parallel with:** F-02, S-01, S-02, S-03, S-04, S-05, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** The space default reads the counter's state; until S-08 lands the default falls back to Private, so the slice does not depend on S-08.
- **Status:** done

### S-07: "To do" list

- **Outcome:** user can pick today's items from the GitLab list and their own tasks (and from GitHub once S-05 lands) into "To do", up to the limit from settings (default 5). They tick items off; unticked items carry over to the next day; items closed in GitLab or GitHub tick themselves; ticking an own task marks it done.
- **Change ID:** todo-list
- **PRD refs:** FR-011, FR-012, US-01 (second half)
- **Prerequisites:** S-01, S-06
- **Parallel with:** S-02, S-03, S-04, S-05, S-08, S-09, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** It completes US-01. GitHub items join once S-05 is unblocked, so the GitHub decision does not hold up the daily flow.
- **Status:** done

### S-08: Work-time counter

- **Outcome:** user can start and stop work time from a counter always visible at the top of the window. A running counter survives closing the app, a Windows restart and a crash. The user can correct an entry's start or end time or add a missing entry.
- **Change ID:** work-timer
- **PRD refs:** FR-013, FR-017, NFR "Trwałość czasu pracy"
- **Prerequisites:** F-01
- **Parallel with:** F-02, S-01, S-02, S-03, S-04, S-05, S-06, S-07
- **Blockers:** —
- **Unknowns:**
  - Record from 2026-12-01 with December entries added by hand until this ships, or start at the first full month after it ships? (PRD Open Question #2) — Owner: user. Block: no.
- **Risk:** Can start in parallel with the north star; the 2026-12-01 start date is the reason not to push it late.
- **Status:** done

### S-09: Work-norm report

- **Outcome:** user can see the day and month sums, how far the month is from the norm so far (8 h × workdays that have passed, Monday–Friday minus days they mark as off), and the average needed per remaining workday to meet the month's norm.
- **Change ID:** work-norm-report
- **PRD refs:** FR-014
- **Prerequisites:** S-08
- **Parallel with:** F-02, S-01, S-02, S-03, S-04, S-05, S-06, S-07, S-10
- **Blockers:** —
- **Unknowns:** —
- **Risk:** The core calculation behind a primary success criterion; it lives in the backend and is unit-tested on invented months (holidays, days off, partial weeks).
- **Status:** ready

### S-10: Timer reminders

- **Outcome:** user gets a reminder when the counter has been running longer than the threshold from settings ("are you still working?") and when it is not running after the set hour on a workday.
- **Change ID:** timer-reminders
- **PRD refs:** FR-018
- **Prerequisites:** S-08
- **Parallel with:** F-02, S-01, S-02, S-03, S-04, S-05, S-06, S-07, S-09
- **Blockers:** —
- **Unknowns:**
  - Default hour for the "not running" reminder and default "running too long" threshold (10 h was proposed; PRD Open Question #3) — Owner: user. Block: no.
- **Risk:** Reminders depend on the workday rule shared with S-09; if S-09 lands first, they reuse its days-off list.
- **Status:** ready

## Backlog Handoff

| Roadmap ID | Change ID                | Suggested issue title                                   | Ready for `/10x-plan` | Notes |
| ---------- | ------------------------ | ------------------------------------------------------- | --------------------- | ----- |
| F-01       | app-data-store           | Durable app-owned data store                            | no                    | Done; archived 2026-11-23 |
| F-02       | read-only-cli-runner     | Read-only runner for glab/gh with result states         | no                    | Done; archived 2026-11-23 |
| S-01       | gitlab-assigned-list     | "Work (GitLab)" view: assigned issues and MRs           | no                    | Done; archived 2026-11-23 |
| S-02       | gitlab-visibility-filter | Filter visible GitLab items                             | yes                   | Run `/10x-plan gitlab-visibility-filter` |
| S-03       | hide-and-resurface       | Hide issues; resurface on status change or comment      | yes                   | Run `/10x-plan hide-and-resurface` |
| S-04       | new-assignment-notify    | Notify on new assigned issues and MRs                   | yes                   | Run `/10x-plan new-assignment-notify` |
| S-05       | github-assigned-issues   | "Projects (GitHub)" view: assigned open issues          | no                    | Blocked on Open Roadmap Question #1 |
| S-06       | own-tasks                | Own tasks in Work/Private; remove vault task screen     | no                    | Done; archived 2026-11-23 |
| S-07       | todo-list                | "To do" list with limit and carry-over                  | yes                   | Run `/10x-plan todo-list` |
| S-08       | work-timer               | Durable work-time counter with corrections              | no                    | Done; archived 2026-11-23 |
| S-09       | work-norm-report         | Work-norm report (sums, deviation, needed per day)      | yes                   | Run `/10x-plan work-norm-report` |
| S-10       | timer-reminders          | Reminders for a long-running or missing counter         | yes                   | Run `/10x-plan timer-reminders` |

## Open Roadmap Questions

1. **`gh` w Windows** — narzędzie dostępu do GitHuba jest tylko w WSL; jak aplikacja na Windows ma sięgać do GitHuba (instalacja w Windows czy wywołanie przez WSL)? Owner: użytkownik. Block: S-05.
2. **Start ewidencji 2026-12-01 a gotowość licznika** — czy wpisy z grudnia do czasu wdrożenia dopisać ręcznie (FR-017), czy przesunąć start ewidencji na pierwszy pełny miesiąc po wdrożeniu? Owner: użytkownik. Block: none (informs S-08).
3. **Wartości domyślne** — godzina przypomnienia o niebiegnącym liczniku (FR-018); próg „licznik biegnie długo” (propozycja: 10 h); częstotliwość odświeżania (NFR ≤ 15 min). Owner: użytkownik. Block: none (informs S-04, S-10).
4. **Los PRD v2 i roadmapy** — resolved for the roadmap on 2026-11-23 (this re-decomposition). Still open: does the unused vault layer (previous F-01, `src-tauri/src/vault/`) stay dormant or get removed? And what happens to the orphaned change folders `context/changes/morning-plan-today/` and `context/changes/private-projects-overview/` from the previous decomposition? Owner: użytkownik. Block: none.
5. **Nazwa produktu** — robocza „Nawigator dnia”. Owner: użytkownik. Block: none.

## Parked

- **Writing to GitLab and GitHub** — Why parked: PRD Non-Goals; read-only (no closing, status changes, comments or assignment).
- **Obsidian vault access** — Why parked: PRD Non-Goals; no reading or writing (daily plan, journal, `Tasks/`, `Issues/`) — vault writes come "later".
- **Work-time automation** — Why parked: PRD Non-Goals; no automatic holiday calendar, idle detection or activity tracking.
- **MRs where the user is a reviewer; deleting tasks (FR-009)** — Why parked: PRD Non-Goals, from the challenge round ("Poza v1").
- **The rest of the PRD v2 plan** (breaks and focus blocks, distraction blocker, quick capture and inbox, end-of-day close, week view, morning "plan your day" nudge, autostart) — Why parked: PRD "considered, deliberately NOT excluded"; no FR covers them, so they are not built in this MVP.

## Milestone History

## Done

Entries below come from this milestone's previous decomposition (archived at `context/foundation/archive/2026-11-23-roadmap-full-day-flow.md`); their IDs refer to that decomposition, not to the F-NN/S-NN above.

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
