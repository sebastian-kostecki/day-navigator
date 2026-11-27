# Changelog

All notable changes to Nawigator dnia. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

Releasing: add entries under `[Unreleased]` as you go. Before bumping `version`
in `src-tauri/Cargo.toml`, move them to a new `## [<version>] - <YYYY-MM-DD>`
section. CI publishes that section as the GitHub Release description and
refuses to publish a version without one (`scripts/release-notes.sh`).

## [Unreleased]

### Added

- "Czas pracy": a "Bieżący tydzień" panel shows the current Monday–Sunday week,
  also across a month or year boundary, day by day against the week's norm (the
  month's rule: holidays and days off count out, a half day off counts half), with the deviation up to today
  and what remains.
- The app can start with Windows into the tray (window hidden) with
  `"autostart": { "enabled": true }` in `settings.json`. Off by default; the
  state shows as "Autostart" in "Źródła".
- "Czas pracy": Polish public holidays count out of the month's norm and show
  in the days table like a day off, with their name; a Saturday holiday shows
  a hint that another day off is owed. Turn the calendar off with
  `"workTime": { "holidays": false }` in `settings.json`.
- "Czas pracy": a work-time entry can have a short one-line note (up to 200
  characters). Set it when adding or editing the entry; the day's entry list
  shows it under the times.
- The tray menu has START/STOP for the work-time counter (no confirmation,
  logged in the event log like the header). The tray tooltip shows today's
  work time under the source state, and the tray icon has a "counter running"
  variant (a source error still shows the error icon).
- "Czas pracy": the daily norm is set in `settings.json` with
  `"workTime": { "dailyNormHours": 8 }` (1–10 hours in whole minutes, default 8).
  There is one value and no history, so a change recalculates every month,
  closed ones included. An invalid value is reported and the dashboard tiles
  show "raport niedostępny".
- "Czas pracy": half a day off, in the "Wolne" control of the days table
  ("nie" / "½" / "tak") and in the "Dzień" control of the "Wybrany dzień"
  panel. It counts half the daily norm, and the "Dni wolne" tile counts it as
  0.5. Existing days off stay whole days.
- "Czas pracy": a "Kopiuj miesiąc" button copies the shown month to the
  clipboard as tab-separated text that pastes into a spreadsheet: a summary,
  one row per day (kind, holiday name, sum, norm, deviation) and one row per
  entry with its note. Numbers use a decimal comma, for pl-PL Excel and Google
  Sheets.
- After STOP in the header, a dismissible "Podsumowanie dnia" panel shows
  today's work time, the month's deviation, what was ticked off in "Do
  zrobienia" and what carries over to tomorrow (no question asked). After STOP
  from the tray, a Windows notification shows today's work time and the
  month's deviation.

## [0.10.1] - 2026-11-26

### Changed

- "Do zrobienia": the list uses the full width of the window. "Do wyboru"
  sits below it with GitLab, GitHub and "Moje zadania" side by side in three
  columns (one list in a narrow window), and "Stan źródeł" and "Dziennik
  zdarzeń" sit next to each other at the bottom.

## [0.10.0] - 2026-11-25

### Added

- "Praca (GitLab)": labels show as chips in their GitLab colours, with scoped
  `key::value` labels split as in GitLab, on assigned, hidden and closed rows.
- "Praca (GitLab)": "Projekty", "Etykiety" and "Kamienie milowe" lists with
  counts narrow the list inside the view. Clicking a label chip or a project
  in a row toggles it, active choices show as removable chips with
  "Wyczyść", and they last until the app closes. The `settings.json` filter
  still applies first.

### Changed

- The window has a dark title bar, starts in the app's background colour
  instead of white, and opens centred on first start.
- The app sets a Content Security Policy (only its own files and the
  backend) and freezes `Object.prototype` against prototype pollution.
- The entry in Windows Settings > Apps links to the project repository.
- The project is open source under the MIT license (`LICENSE`). The README
  has a contributing section, and `Cargo.toml` and `package.json` declare the
  license.

## [0.9.0] - 2026-11-24

### Changed

- The README now describes the app: features, privacy, requirements,
  install, configuration (`settings.json`) and development. It replaces the
  Tauri template text.
- The "Day Navigator — Konsola" design-system artifact now matches the app.
  The app itself does not change.

## [0.3.0] - 2026-11-24

### Added

- Dashboard: a "Dziennik zdarzeń" panel under "Stan źródeł" lists today's
  events: new GitLab assignments, ticking or removing "Do zrobienia" items,
  START/STOP, and a source going into error and back.
- "Czas pracy": a table of the month's days with entries, a bar against the
  day's norm, the sum and the deviation. Today is highlighted, days off are
  hatched, and a day off can be marked straight from the table.
- "Czas pracy": a read-only "Przypomnienia" panel shows the reminder
  thresholds, whether each one is on, and which values are defaults. An
  "Ustawienia" button opens the settings file.
- "Praca (GitLab)": a NOWE flag on items that appeared since the last visit,
  and the date next to hidden rows ("ukryte 18.11").
- "Praca (GitLab)": a FILTR bar shows the active filter values with a link to
  the settings, and the "GitLab niedostępny" card shows the refresh interval
  and when the next try happens.
- "Projekty (GitHub)": each issue row shows its labels.
- "Stan źródeł": the "Dane aplikacji" card shows the time of the last backup,
  the number of kept backups, the schema version, and a button that opens the
  backups folder.

### Changed

- The header, the dashboard and "Czas pracy" share one work-timer state, and
  the views share common UI components. No change in behaviour.

## [0.2.1] - 2026-11-24

### Added

- "Praca (GitLab)" and "Projekty (GitHub)": a "+" button on each row adds the
  item to "Do zrobienia", blocked at the list limit with the same hint as the
  picker; items already on the list show the "W DO ZROBIENIA" flag and a
  filled button that takes them off again.

### Changed

- The exe and the installer show the author in their copyright field; the
  package description and author are no longer template values.
- Release descriptions come from this changelog, with a link to the full
  list of changes since the previous release.

### Removed

- The unused Obsidian vault layer, and the vault and exclusion cards in
  "Stan źródeł". An older `settings.json` with vault paths still loads.

## [0.2.0] - 2026-11-24

### Added

- Source health: GitLab, GitHub and app data status in a panel and in the tray.
- Work tasks from a template in the "Zadania" view.
- App data store (SQLite) with daily backups.
- Read-only access to GitLab and GitHub through the logged-in `glab` and `gh`.
- "Praca (GitLab)": assigned GitLab issues, refreshed in the background, with
  an optional assignee filter.
- Work timer with an always-visible header counter and the "Czas pracy" view.
- "Moje zadania": own tasks.
- "Do zrobienia": a daily to-do list reconciled with GitLab.
- Work-time norm report with the deviation in the header and days off.
- Hiding an issue until it changes.
- Notifications about new assignments, with a refresh-interval setting.
- Work-timer reminders.
- "Projekty (GitHub)": assigned GitHub issues, also in "Do zrobienia".
- GitLab visibility filter, including recently closed issues.
- Konsola design system across all views, with a new app icon, notices and
  START/STOP confirmation; transparent app and tray icons.

### Fixed

- Dependency `url` updated to 2.5.9 (RUSTSEC-2026-0592).
- The build picks up changes in `src-tauri/icons`.

## [0.1.0] - 2026-11-21

### Added

- Safe vault access: settings, path guard, line-level edits and a vault trash.
- Windows installer (NSIS), built and published by CI on a version bump.
- Local pre-push guard against work terms.
