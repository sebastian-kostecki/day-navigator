# Changelog

All notable changes to Nawigator dnia. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

Releasing: add entries under `[Unreleased]` as you go. Before bumping `version`
in `src-tauri/Cargo.toml`, move them to a new `## [<version>] - <YYYY-MM-DD>`
section. CI publishes that section as the GitHub Release description and
refuses to publish a version without one (`scripts/release-notes.sh`).

## [Unreleased]

### Changed

- The exe and the installer show the author in their copyright field; the
  package description and author are no longer template values.
- Release descriptions come from this changelog, with a link to the full
  list of changes since the previous release.

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
