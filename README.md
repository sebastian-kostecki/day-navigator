<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="Nawigator dnia icon">
</p>

<h1 align="center">Nawigator dnia</h1>

<p align="center">
  One place for a developer's working day on Windows: assigned GitLab and GitHub work,
  a short "To do" list, your own tasks and a work-time counter.
</p>

<p align="center">
  <a href="../../releases/latest"><img src="https://img.shields.io/github/v/release/sebastian-kostecki/day-navigator?style=for-the-badge&label=Release" alt="Latest release"></a>
  <a href="../../actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/sebastian-kostecki/day-navigator/ci.yml?branch=main&style=for-the-badge&label=CI" alt="CI status"></a>
  <a href="#requirements"><img src="https://img.shields.io/badge/Platform-Windows%2011-0078D4?style=for-the-badge" alt="Platform: Windows 11"></a>
  <a href="#license"><img src="https://img.shields.io/badge/License-MIT-blue?style=for-the-badge" alt="License: MIT"></a>
</p>

<p align="center">
  <a href="https://v2.tauri.app/" target="_blank"><img src="https://img.shields.io/badge/Tauri%202-24C8DB?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri 2"></a>
  <a href="https://www.rust-lang.org/" target="_blank"><img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="https://react.dev/" target="_blank"><img src="https://img.shields.io/badge/React%2019-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React 19"></a>
  <a href="https://www.typescriptlang.org/" target="_blank"><img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript"></a>
  <a href="https://vite.dev/" target="_blank"><img src="https://img.shields.io/badge/Vite-646CFF?style=for-the-badge&logo=vite&logoColor=white" alt="Vite"></a>
  <a href="https://www.sqlite.org/" target="_blank"><img src="https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite"></a>
</p>

<p align="center">
  <a href="../../releases/latest">Download</a> ·
  <a href="CHANGELOG.md">Changelog</a> ·
  <a href="#development">Development</a>
</p>

---

Nawigator dnia (Polish for "day navigator") is a small personal desktop app built with
[Tauri 2](https://v2.tauri.app/), Rust and React. Its interface is in Polish. It runs from
the system tray, shows what is assigned to you, lets you pick today's work and keeps track of
the hours you spend on it.

## Features

- **Do zrobienia (To do):** a daily list picked from GitLab, GitHub and your own tasks, with
  a limit (5 by default). Unticked items carry over to the next day. Items closed in GitLab or
  GitHub tick themselves off. Use "+" on any row to add it.
- **GitLab:** the issues and merge requests assigned to you, refreshed in the background.
  Includes a visibility filter (projects, labels, milestones, recently closed), a NEW flag for
  items that appeared since your last visit, and hiding an item until it changes. Windows
  notifications report new assignments.
- **GitHub:** open issues assigned to you in the repositories you list, with their labels.
- **Moje zadania (My tasks):** your own tasks in a Work or Private space, with an optional due
  date and priority.
- **Czas pracy (Work time):** a START/STOP counter that is always visible in the header and
  survives restarts and crashes. Also includes:
  - day and month sums against a daily norm (8 hours by default, set in `settings.json`), with
    Polish public holidays counted out and a half day off counting half the norm;
  - a table of the month's days and days off;
  - a summary of the current week (Monday to Sunday, also across months) against the week's norm, day by day;
  - a "Kopiuj miesiąc" button that copies the shown month (summary, days, entries with notes,
    holidays) as tab-separated text for a spreadsheet; numbers use a decimal comma, for Excel or
    Google Sheets set to Polish (pl-PL); in a sheet with another locale the decimal hours may
    paste as text or as a wrong number (7,50 as 750), while the H:MM columns stay correct;
  - reminders when the counter runs too long, or is not running after a set hour on a workday;
  - START/STOP from the tray menu; the tray tooltip shows today's work time and the tray icon
    shows when the counter runs;
  - after STOP: a day summary in the window (header STOP) or a short notification (tray STOP).
- **Źródła (Sources):** the health of GitLab, GitHub and the app's own data in one panel and
  in the tray icon, plus today's event log. A failed source shows an error, never an empty
  list.
- **Keyboard:** keys `1`–`6` switch between the views.

## Privacy and safety

- **No passwords, tokens or keys.** The app stores none of them. It reaches GitLab and GitHub
  only through the official [`glab`](https://gitlab.com/gitlab-org/cli) and
  [`gh`](https://cli.github.com/) command-line tools you are already logged in to.
- **Read-only.** The app runs those tools with fixed, read-only commands. It never creates,
  edits, closes, comments on or assigns anything.
- **Local data.** Your data stays on your machine. Settings are in the app config folder.
  Your own tasks, the "To do" list and work time are kept in a local SQLite database in the
  app data folder, with a daily backup (7 kept). With `autostart` on, the app also keeps one
  entry in the current user's Windows Run key (see Configuration).
- **No VPN.** When the work GitLab cannot be reached, the app shows a calm "unavailable"
  state with the time of the last successful refresh.

## Requirements

- Windows 11 (x64). This is the only system it is built and tested for.
- [`glab`](https://gitlab.com/gitlab-org/cli) logged in to your GitLab, for the GitLab view.
- [`gh`](https://cli.github.com/) logged in to GitHub, for the GitHub view.

## Install

Download `Nawigator.dnia_<version>_x64-setup.exe` from the
[latest release](../../releases/latest) and run it.

The installer is not code-signed, so Windows SmartScreen may show "Windows protected your PC".
Choose **More info → Run anyway**. Every release is built by CI from this repository.

## Configuration

Settings are a JSON file at `%APPDATA%\com.sebas.day-navigator\settings.json`. Every block is
optional. Change them in the app or edit the file by hand.

The gear button in the header opens the **Ustawienia** dialog. The "Ustawienia" buttons in
**Czas pracy** (Przypomnienia panel) and in the GitLab filter bar open it at their section, and
"Otwórz settings.json" in the dialog footer opens the file. The dialog edits the file like this:

- Each section has its own "Zapisz" and writes only its own blocks, in one step (a temporary
  `settings.json.tmp` in the same folder, then a rename).
- The order of the top-level keys, unknown top-level keys, a BOM and CRLF line endings are kept.
  Whitespace and the key order inside saved blocks are normalised. Keep a copy
  first if you want your exact formatting back.
- The whole file is written again, so a repeated key keeps only its last value, and a very large or
  very precise number in an unknown block may be rounded. A `settings.json` that is a symbolic link
  is replaced by a regular file; the link target is not changed.
- Text is written as typed (list entries trimmed); the dialog shows it normalised, for example
  filter entries in lower case. Values shown as defaults are written too: saving **GitLab** adds
  `"gitlabRefresh": { "minutes": 10 }`. An empty GitLab host or an empty repository list removes
  that block.
- A missing file is created on save; a missing config folder is not.
- A file that is not valid JSON, or has the wrong `version`, is never overwritten: fix it by hand.
- If a block changed on disk after the dialog loaded it, the save is refused and nothing is
  written. An invalid block is replaced only after you confirm it.
- A save takes effect without a restart.

```json
{
  "version": 1,
  "gitlab": { "host": "gitlab.example.com" },
  "gitlabRefresh": { "minutes": 10 },
  "gitlabFilter": {
    "projects": { "include": ["acme/backend"], "exclude": ["acme/backend/legacy"] },
    "labels": { "exclude": ["blocked"] },
    "closedDays": 7
  },
  "github": { "repos": ["octo-org/alpha", "octo-org/beta"] },
  "githubRefresh": { "minutes": 10 },
  "todo": { "limit": 5 },
  "autostart": { "enabled": true },
  "reminders": { "longRunningHours": 10, "notRunningAfter": "09:00" },
  "workTime": { "holidays": "pl", "dailyNormHours": 8 },
  "notices": { "newAssignments": true, "sourceProblems": true, "daySummary": true }
}
```

| Block           | What it sets                                                                                       |
| --------------- | -------------------------------------------------------------------------------------------------- |
| `gitlab`        | GitLab host for `glab`. Optional `user` filters by assignee.                                       |
| `gitlabRefresh` | Background refresh interval in minutes, 1–14 (default 10).                                         |
| `gitlabFilter`  | `include` / `exclude` lists for projects, labels and milestones; `closedDays` 1–30.                |
| `github`        | `owner/name` repositories to read issues from (up to 50).                                          |
| `githubRefresh` | GitHub background refresh interval in minutes, 1–14 (default 10). An invalid value shows a notice in **Projekty (GitHub)** and uses 10. |
| `todo`          | "To do" list limit, 1–20 (default 5).                                                              |
| `autostart`     | `enabled`: `true` starts the app with Windows into the tray, window hidden; off by default. A save in the dialog applies at once (release builds); after a hand edit, restart the app. |
| `reminders`     | `longRunningHours` 1–24 and `notRunningAfter` `"HH:MM"`; `false` turns a reminder off.             |
| `workTime`      | `holidays`: `"pl"` (default) counts Polish public holidays out of the norm; `false` turns the calendar off. `dailyNormHours`: the daily norm as a number of hours, 1–10 in whole minutes (for example `7.5`; missing or `null` = 8). One value, no history: changing it recalculates every month, closed ones included. |
| `notices`       | Windows notices, each `true` by default: `newAssignments` (new GitLab assignments and review requests), `sourceProblems` (GitLab or GitHub failing and coming back), `daySummary` (the day summary after a tray STOP). |

An invalid block is reported in the **Źródła** view and does not affect the other blocks. An invalid `notices` block is instead shown in the **Ustawienia** dialog, and all notices stay on. Another exception is `workTime`: an invalid block is reported in the **Czas pracy** view (month report, days table and week summary) and in the dashboard's month tiles, not in **Źródła**; START/STOP keeps working. With an invalid `dailyNormHours` the report error names it, the dashboard tiles show "raport niedostępny", and the status, entries and reminders fall back to 8 hours.

A day can be a whole or a half day off: in the days table the "Wolne" control has three buttons ("nie" / "½" / "tak"), and the "Wybrany dzień" panel has "Dzień" with "roboczy" / "pół dnia wolnego" / "wolny". A half day off stays a workday with half the daily norm; it is refused on weekends and holidays, and the "counter not running" reminder stays silent that day. The "Dni wolne" tile counts a half day as 0.5, and "Średnio / dzień" is the time needed per full workday.

With `autostart` on, the app keeps one entry named `com.sebas.day-navigator` in the current user's Run key, pointing at the copy of the app that last started, or saved the Autostart section, with the key on, and removes it when the key is off. A start by Windows goes straight to the tray with the window hidden; a manual start shows the window. Disabling the app in Task Manager → Startup apps also works, and the app leaves that choice alone (the row still reads OK). Before uninstalling, switch it off in the dialog (or set `enabled` to `false` and start the app once), or remove the entry in Task Manager. The result shows as "Autostart" in **Źródła**.

## Development

Prerequisites: [Node.js](https://nodejs.org/) with npm, and Rust 1.90 or newer. See also the
[Tauri prerequisites for Windows](https://v2.tauri.app/start/prerequisites/).

```powershell
npm install
npm run tauri dev
```

Checks run by CI on every push:

```powershell
npm run lint; npm run format:check; npm run build
cd src-tauri; cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test
```

The UI is in `src/`, with one folder per feature. The Rust backend is in `src-tauri/`. Agent
and contributor rules are in [AGENTS.md](AGENTS.md).

### Releasing

1. Move the `[Unreleased]` entries in [CHANGELOG.md](CHANGELOG.md) into a new
   `## [<version>] - <date>` section.
2. Bump `version` in `src-tauri/Cargo.toml` and run `cargo build` to refresh `Cargo.lock`.
3. Push to `main`. CI builds the NSIS installer and publishes it as a GitHub Release, using
   the changelog section as the release notes.

## Contributing

Issues and pull requests are welcome. Before you open a pull request, run the checks listed
under [Development](#development). Tests and fixtures use invented data only; never commit
real hostnames, issue contents or other work data. The full rules for the codebase are in
[AGENTS.md](AGENTS.md).

## License

[MIT](LICENSE) © 2026 Sebastian Kostecki.
