# CI and Release Pipeline — Plan Brief

> Full plan: `context/changes/ci-release-pipeline/plan.md`

## What & Why

Roadmap foundation F-03 has three parts:
- a private GitHub repo,
- CI that builds, tests and audits every change on invented data only,
- a Windows installer of "Nawigator dnia" published as a GitHub Release from `main`.

Autostart (S-02) needs an installed build. The repo may go public later, so nothing secret or work-related may ever enter its history.

## Starting Point

There is no git remote and no `.github/`, and `gh` is not installed on this machine. The app still uses the starter identity (`productName` and window title `day-navigator`), the version is defined in three places, and the bundle builds both MSI and NSIS. Some tests compile only on Windows (junctions), so CI needs a Windows runner. The existing 10-commit history contains no work data.

## Desired End State

- **Every push** runs gitleaks, `cargo audit`, `npm audit` and a hook self-test on Linux, and build plus `cargo test` on Windows.
- **A push to `main` whose `src-tauri/Cargo.toml` version has no release yet** publishes:
  - release `Nawigator dnia v<version>`,
  - tag `app-v<version>`,
  - a per-user NSIS installer.
- **A push without a version bump** stays green and publishes nothing. A red run never publishes.
- **A local pre-push hook** blocks outgoing commits that contain any term from a work-term denylist kept outside the repo.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Visibility, name, trigger | Private repo; release named "Nawigator dnia"; from `main` | Settled by PRD Open Questions #1/#7 and tech-stack. | Roadmap / PRD |
| Versioning | Bump by hand in `Cargo.toml`; release skipped if `app-vX.Y.Z` already exists | tauri-action silently overwrites existing releases, and skipping saves Windows minutes. | Plan |
| Installer | NSIS only, per-user | No admin rights, no MSI version limits or VBScript dependency. | Plan |
| CI checks | build + test + `cargo audit` + `npm audit --audit-level=high` | Matches AGENTS.md commands; adds no clippy or fmt cleanup work. | Plan |
| Gate | One workflow; `release` needs all checks | GitHub Free cannot require status checks on private repos. | Plan |
| Data guard | gitleaks in CI + local pre-push denylist outside the repo | CI catches secrets; only a local list can catch company terms without itself becoming work data. | Plan |
| App name | `productName` and title "Nawigator dnia"; identifier unchanged | Product name shows everywhere; the app-data path stays stable. | Plan |
| Version source | `src-tauri/Cargo.toml` (drop `version` from `tauri.conf.json`) | One place to bump; Tauri falls back to Cargo's version. | Plan |

## Scope

**In scope:** Tauri config (name, NSIS, version source); `index.html` title; `.githooks/pre-push` and its self-test script; `.gitattributes` LF rules; `.github/workflows/ci.yml`; AGENTS.md release and hook rules; creating the private repo and the first release.

**Out of scope:** code signing and the updater (would need secret keys); MSI; automatic versioning; clippy and fmt gates; `pull_request` trigger and path filters; branch protection (needs a paid plan); attestations and checksums; renaming the identifier or crates; a work-term check in CI.

## Architecture / Approach

`ci.yml` runs on every push with four jobs:
- `checks` (Linux)
- `build-test` (Windows)
- `version` (Linux, `main` only; decides `publish`)
- `release` (Windows; `needs` all three, runs only if `publish`)

Cheap work stays on Linux to save Windows minutes. Locally, `core.hooksPath=.githooks` runs the pre-push hook. It reads `%USERPROFILE%\.day-navigator\push-denylist.txt`, refuses the push if that file is missing, and reports only the commit, file and line number, never the term.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. App identity and installer config | Local `Nawigator dnia_0.1.0_x64-setup.exe`, single version source | `productName` change renames the executable |
| 2. Local pre-push guard | Hook + self-test with invented terms | Hook prints a term, or a blank denylist line matches everything |
| 3. CI and release workflow | `ci.yml` with gated release; AGENTS.md updated | A wrong gate lets a red build or an unbumped version publish |
| 4. Private repo and first release | Repo, green first run, `app-v0.1.0`, release skipped on a push without a bump | The push is irreversible: the hook must be active first |

**Prerequisites:**
- Install `gh` and run `gh auth login --scopes workflow` (manual).
- Create the work-term denylist locally.
- Commit or vet the main checkout's uncommitted work before the first push.

**Estimated effort:** about 2 sessions. Phases 1–3 together, Phase 4 interactively with the user.

## Open Risks & Assumptions

- gitleaks runs as the CLI over the full history, not as `gitleaks-action`. The action fails on a first push that contains the root commit, skips merge commits, and v2 targets the removed Node 20 runtime.
- The `version` job publishes only on an explicit "release not found" or a release missing its installer, and fails on any other `gh` error, so an API error never overwrites a release.
- Free Actions budget is about 1,200 Windows minutes a month. Release builds (LTO) are estimated at 10–20 minutes cold.
- An unsigned installer is blocked outright if Windows Smart App Control is on.
- The roadmap says "each merge publishes". In this plan, only merges with a version bump publish, which is a deliberate narrowing.

## Success Criteria (Summary)

- The private repo is green on every push, and `app-v0.1.0` "Nawigator dnia" installs per user and launches.
- A push without a version bump never republishes or overwrites a release.
- A commit containing a denylisted work term cannot leave the machine.
