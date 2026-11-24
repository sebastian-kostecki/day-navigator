# Repository Guidelines

Day Navigator is a personal Windows 11 desktop app built on Tauri 2: a Rust backend in `src-tauri/` and a React 19 + TypeScript UI (Vite) in `src/`. Requirements (in Polish, brownfield MVP): `@context/foundation/prd.md`.

## Hard Rules

- No secrets anywhere: no passwords, tokens or keys in code, settings or the repo. Reach GitLab and GitHub only through the logged-in `glab` / `gh` CLIs; if logged out, report it instead of adding credentials.
- Run `glab` / `gh` from Rust with a fixed program name and fixed argument lists: no shell, no string-built commands, no user-supplied flags. Read-only: never call a subcommand that creates, edits, closes, comments or assigns. An unreachable work GitLab (no VPN) is a calm "unavailable" state, not an alarm; a logged-out CLI is an error.
- Work data (work vault content, company issues, hostnames) never goes into the repo, tests, fixtures, logs or the private space. Tests use invented data only.
- Vaults are out of scope: the app never reads or writes a vault.
- Outside folders configured in settings, the app writes only its own state under its bundle identifier: the app config dir (`settings.json` and the window-state plugin's file), the app data dir (own tasks, the "To do" list, hidden-issue snapshots, seen-assignment keys (opaque account key, kind, id), the GitLab view's seen-until mark (opaque account key, one instant), work-time entries, today's event log (kind, local time, and the item reference and title or source name the dashboard shows; other days are pruned)) and the app log dir. Never user content or vault data there. If a configured path is missing, show an error; never create it silently.
- Never render an empty plan or list in place of a failed source; surface the failure.
- Never bypass the pre-push hook (`--no-verify`) and never commit the denylist or its terms. `core.hooksPath` is relative, so push only from a checkout that contains `.githooks/pre-push`.
- Change-scoped docs (research, plans, reviews) go in `context/changes/<change-id>/`.

## Project Structure

- `src/` — React UI; entry `main.tsx`, root component `App.tsx`. One folder per feature (`src/health/`, `src/own-tasks/`); shared UI in `src/components/`.
- `src-tauri/src/lib.rs` — Tauri builder: register plugins and `#[tauri::command]` handlers here. `main.rs` only calls `day_navigator_lib::run()`.
- `src-tauri/capabilities/default.json` — permissions for the `main` window; add a plugin's permissions here when you add the plugin.
- `src-tauri/tauri.conf.json` — app identity, window and bundle config.
- `src-tauri/src/cli/` — the only code that starts `glab` / `gh`. A new call is a new `CliCall` variant with fixed arguments (read-only, covered by the guard test). Tests use the `fake-cli` bin (test-support only), never real work data.

## Frontend Conventions (React 19, no router)

- A feature folder holds `<Name>View.tsx` (the screen), `api.ts` (the only file that imports `invoke` from `@tauri-apps/api/core`; one typed function per command), `types.ts` (TS mirrors of Rust serde types) and `use<Name>.ts` (loading/error/data state).
- Navigation: `App.tsx` owns the active view in one piece of state and renders a fixed header (the work-time counter, always visible) plus the active view. No router or global state library without a plan that says so. The timer state is one `WorkTimerProvider` (a React context, `src/worktime/`) around the header and the views; read it through `useWorkTimer()` and never subscribe to the timer status a second time.
- Every view that shows a source renders loading, error (via `src/components/ErrorState.tsx`, with the time of the last successful refresh) and data. "Empty" is a data state with an explicit message.
- Styles: `src/styles/tokens.css` is the only place for values; no literal colours in any other CSS. Component CSS sits next to the component with a `dn-` prefix; view CSS is layout only. The theme is always dark (no `prefers-color-scheme`). `ErrorState` renders `SourceState kind="error"`. Design system: https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY
- Domain rules (hidden-issue reappearance, "To do" limit and carry-over, work-time norm and deviation) live in Rust and are unit-tested there; the UI only displays results and sends user actions.

## Rust ↔ TypeScript Contracts

- `#[tauri::command]` input/output types are serde structs with `#[serde(rename_all = "camelCase")]`; error enums derive `thiserror::Error` + `Serialize` with `#[serde(tag = "kind", content = "detail")]` (see `SettingsError` in `src-tauri/src/settings.rs`).
- The TS mirror is a tagged union on `kind`. Change the Rust type and its TS mirror in the same commit.

## App Data Store

- App-owned records go only through `store::StoreHandle` (`src-tauri/src/store/`: SQLite, WAL, daily backup in `backups/`, 7 kept). No ad-hoc files for app data.
- A schema change = append a `Migration` to `store/schema.rs`; never edit a shipped one. Changes spanning records happen in one `write`. Store errors never carry row values or paths.
- Store commands are `async` and call `with_store` inside `spawn_blocking` (the `on_store` helper in `lib.rs`; the work-time commands use the generic `blocking`), never from a sync command: the health round holds the store lock during reopen and backup.
- The tray reports the store as "Dane aplikacji" (`app-data`). A damaged, newer or lost database is refused, never reset.
- Restoring: quit the app, copy a file from `backups/` over `day-navigator.db`, delete `day-navigator.db-wal` and `-shm`. Starting over on purpose: quit the app and clear both `day-navigator.db*` and `backups/` (a missing database next to existing backups is refused as `Missing`).
- Data slices reuse the crash harness in `src-tauri/tests/common/crash.rs`.

## Tauri 2 Only

- No v1 idioms: no `allowlist` in `tauri.conf.json`, no `tauri::api::*`, no `@tauri-apps/api/tauri` (use `@tauri-apps/api/core`).
- Adding a plugin = crate in `src-tauri/Cargo.toml` + `.plugin(...)` in `lib.rs` + npm `@tauri-apps/plugin-*` on the same major.minor + its permissions in `capabilities/default.json`. A plugin used only from Rust gets no npm package and no permissions (`window-state`).
- Docs: https://v2.tauri.app/ , https://v2.tauri.app/plugin/ , https://docs.rs/tauri/2

## Build and Development Commands

- `npm install` — install frontend dependencies.
- `npm run tauri dev` — run the app; Vite serves on fixed port 1420 (`strictPort`), so free that port first.
- `npm run build` — `tsc` type check plus Vite build into `dist/`.
- `npm run lint` / `npm run format:check` (`npm run format` to fix) — ESLint and Prettier for `src/`.
- In `src-tauri/`: `cargo build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo audit`.

## Coding Style and Versions

- TypeScript runs `strict` with `noUnusedLocals` and `noUnusedParameters` (`@tsconfig.json`); `npm run build` must finish with zero errors.
- Versions: React 19, TypeScript 6 (do not upgrade to 7 until `typescript-eslint` supports it; 8.73 refuses TS 7.0), Vite 8. Prefer current React 19 APIs over older patterns.
- Rust uses edition 2024 and needs rustc 1.90 or newer (required by tauri 2.12). Code is `rustfmt`-formatted and clippy-clean.
- Keep the npm `@tauri-apps/*` packages and the Rust `tauri*` crates on the same major.minor; `npm run tauri dev` prints a mismatch error otherwise.

## Testing

Run `cargo test` in `src-tauri/` (unit, property and integration tests). Tests use invented data only. `src-tauri/tests/common` holds `test_store_dir()` and the crash harness (`crash.rs`). The UI has no test runner. Before every commit run `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.

## Commits and Pull Requests

The remote is a private GitHub repository, reached only through `gh`. CI is `.github/workflows/ci.yml`: every push runs secret scan, audits, lint/format (ESLint, Prettier, rustfmt, clippy) and build/test; a push to `main` also publishes the NSIS installer as a GitHub Release.

- Release: bump `version` in `src-tauri/Cargo.toml`, run `cargo build` to refresh `Cargo.lock`, commit both and push to `main`. No bump means nothing is published (CI still runs). `Cargo.toml` is the only version source; the `package.json` version is unused.
- One-time setup per clone: `git config core.hooksPath .githooks`, and a denylist of work terms at `%USERPROFILE%\.day-navigator\push-denylist.txt` (the pre-push hook blocks pushes without it).
- Stack background: `@context/foundation/tech-stack.md`, `@context/foundation/stack-assessment.md`.
