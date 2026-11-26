# Windows autostart into the tray — Plan Brief

> Full plan: `context/changes/windows-autostart/plan.md`
> Research: `context/changes/windows-autostart/research.md`

## What & Why

Roadmap S-04 (MS-17, issue #17): the user can have Nawigator dnia start with Windows into the tray, window hidden, by setting `"autostart": { "enabled": true }` in `settings.json`. START/STOP reminders (FR-018) only fire while the app runs, so starting it at sign-in keeps them working without a manual launch.

## Starting Point

`setup` ends with an unconditional `show_main` (`src-tauri/src/lib.rs:173`) on a window that is created hidden; the single-instance callback always shows the window. The app never reads its command line and never touches the registry. Settings blocks have one loader each (`settings/work_time.rs` is the template), and every user-relevant failure is a health source in "Źródła".

## Desired End State

With the key on, an installed build keeps one HKCU Run value `com.sebas.day-navigator` = `"<exe>" --autostart`; at sign-in the app starts in the tray with the window hidden. With the key off (the default) the value is removed. A manual start still shows the window. "Źródła" has an "Autostart" row (OK / NIESKONFIGUROWANE / calm NIEDOSTĘPNY in debug builds / BŁĄD), and a failure turns the tray red with a toast.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Default | Off; `autostart.enabled: true` turns it on | User decision | User |
| Sync | At each start: on → ensure value, off → remove it | User decision | User |
| Hidden start | `--autostart` launch goes to the tray; manual start shows the window | User decision | User |
| Rule exception | Narrow AGENTS.md exception for the one Run value | The entry is a write outside the app's folders | User |
| PRD | MS-17, no new FR; drop "autostart" from "not covered" | User decision | User |
| Mechanism | Direct HKCU Run value via `winreg 0.56`, no plugin | Plugin writes an unquoted path, tries HKLM first, overrides Task Manager | Research |
| Value | Name = bundle identifier; data = quoted exe + `--autostart` | Matches the "under its bundle identifier" rule; safe with spaces in the path | Research |
| Detection | Fixed `--autostart` argument, exact match | No other argument parsing | Research |
| Debug builds | Never touch the registry; key on → calm Unavailable | A dev exe in a worktree must not become a login item | Research |
| Display | New health source "Autostart", generic row, no view change; event log learns the key (`source_from_key` + TS `EventSourceId`) | Reuses tray colour, toast, event log; an unknown key would make today's event log unreadable | Research, plan review |
| Error kinds | `SettingsInvalid`; new `AutostartFailed` with `io::ErrorKind` only | Fixed, value-free texts | Plan |
| Uninstall | README note, no NSIS hook | Simplest safe option | Plan |
| Task Manager disable | Not read, not overridden; documented | Undocumented byte format; respects the user's choice | Plan |
| Re-sync | Once per start only; edits take effect after a restart | Matches the user's "at start" decision | Plan |

## Scope

**In scope:** `autostart` settings block; pure sync rules with an in-memory fake; HKCU Run value via `winreg`; start-up thread and health source; hidden start and single-instance guard; AGENTS.md exception; README, CHANGELOG, PRD wording.

**Out of scope:** `tauri-plugin-autostart`; any UI, capability or CSP change and any TS change beyond the one `EventSourceId` key; writing `settings.json`; HKLM, Task Scheduler or Startup folder; `StartupApproved`; NSIS uninstall hook; re-sync while running; real-registry tests; roadmap edit; store migration.

## Architecture / Approach

`settings/autostart.rs` reads the block. `autostart/mod.rs` (no `tauri`) holds `started_by_autostart`, `sync_action`, `run(settings, apply, &dyn RunEntry, exe) -> Outcome` and `status(&Outcome) -> SourceStatus`. `autostart/hkcu.rs` is the only registry code (`HkcuRun`). `autostart/startup.rs` runs `run` once on an `autostart-sync` thread (with `apply = !cfg!(debug_assertions)`), stores the status in managed state and wakes the health round, which appends it in `probe_round`. `lib.rs` skips `show_main` for `--autostart` in `setup` and in the single-instance callback.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Settings block and pure rules | Loader, `SourceId::Autostart` (with event-log key and TS mirror), sync rules and status mapping, fully unit-tested | Loader drifting from the `workTime` pattern |
| 2. Registry entry, sync and hidden start | `winreg` dep, `HkcuRun`, start-up thread, health row, hidden start, AGENTS.md exception | Real registry behaviour verifiable only by hand in an installed build |
| 3. Docs and PRD | README key/timing/Task Manager/uninstall, CHANGELOG bullet, PRD wording | Small merge conflicts with parallel slices on README and PRD lines |

**Prerequisites:** none (S-03 left start-up order for S-04). Manual rows need an installed release build and a Windows sign-in.
**Estimated effort:** ~1–2 sessions across 3 phases.

## Open Risks & Assumptions

- Not verified: how Explorer launches the quoted Run value at sign-in (documented-safe form; manual row 2.12).
- Window-state saving after a session where the window was never shown (manual row 2.14); a saved maximized state does not show the hidden window (tao skips it, checked in source; manual row 2.12).
- Release builds abort on panic, so the sync path must not panic (no `unwrap`/`expect`).
- Any release copy started with the key on (for example `target/release`) repoints the Run value until the installed app starts again.
- The Autostart row reads OK while Task Manager has the entry disabled (documented, not detected).
- A leftover Run value after uninstall points to a missing exe; README covers clean-up.

## Success Criteria (Summary)

- With the key on, signing in to Windows starts the app in the tray with the window hidden; with the key off, nothing starts and the entry is gone.
- A manual start still shows the window, and any sync failure is visible in "Źródła", the tray colour and a toast.
- Tests and debug builds never touch the real registry.
