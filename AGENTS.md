# Repository Guidelines

Day Navigator is a personal Windows 11 desktop app built on Tauri 2: a Rust backend in `src-tauri/` and a React 19 + TypeScript UI (Vite) in `src/`. Requirements (in Polish): `@context/foundation/prd.md`.

## Hard Rules

- No secrets anywhere: no passwords, tokens or keys in code, settings or the repo. Reach GitLab and GitHub only through the logged-in `glab` / `gh` CLIs; if logged out, report it instead of adding credentials.
- Work data (work vault content, company issues, hostnames) never goes into the repo, tests, fixtures, logs or the private space. Tests use invented data only.
- A write to a vault note changes only the targeted line or section; the rest of the file stays byte-identical. Never hard-delete: move to the vault trash.
- Read and write only inside folders configured in settings. If a configured path is missing, show an error; never create it silently. The only exceptions are the distraction blocker and the app's own state folders under its bundle identifier (the app config dir holding `settings.json`, and the app log dir): app-owned state only, never user content or vault data.
- Never render an empty plan or list in place of a failed source; surface the failure.
- Never bypass the pre-push hook (`--no-verify`) and never commit the denylist or its terms. `core.hooksPath` is relative, so push only from a checkout that contains `.githooks/pre-push`.
- Change-scoped docs (research, plans, reviews) go in `context/changes/<change-id>/`.

## Project Structure

- `src/` — React UI; entry `main.tsx`, root component `App.tsx`.
- `src-tauri/src/lib.rs` — Tauri builder: register plugins and `#[tauri::command]` handlers here. `main.rs` only calls `day_navigator_lib::run()`.
- `src-tauri/capabilities/default.json` — permissions for the `main` window; add a plugin's permissions here when you add the plugin.
- `src-tauri/tauri.conf.json` — app identity, window and bundle config.

## Build and Development Commands

- `npm install` — install frontend dependencies.
- `npm run tauri dev` — run the app; Vite serves on fixed port 1420 (`strictPort`), so free that port first.
- `npm run build` — `tsc` type check plus Vite build into `dist/`.
- `cargo build` / `cargo audit` (in `src-tauri/`) — Rust build and advisory check.

## Coding Style and Versions

- TypeScript runs `strict` with `noUnusedLocals` and `noUnusedParameters` (`@tsconfig.json`); `npm run build` must finish with zero errors.
- Rust uses edition 2024 and needs rustc 1.90 or newer (required by tauri 2.12).
- Keep the npm `@tauri-apps/*` packages and the Rust `tauri*` crates on the same major.minor; `npm run tauri dev` prints a mismatch error otherwise.

## Testing

Run `cargo test` in `src-tauri/` (unit, property and integration tests). Fixtures live in `src-tauri/tests/fixtures/vault/`: invented data only, kept byte-exact (CRLF, BOM) by `.gitattributes`. Tests never touch the fixture itself; a slice that writes vault notes reuses `src-tauri/tests/common` (`test_vault()` copies the fixture into a temp dir). Also verify every change with `npm run build` and `cargo build`.

## Commits and Pull Requests

The remote is a private GitHub repository, reached only through `gh`. CI is `.github/workflows/ci.yml`: every push runs secret scan, audits and build/test; a push to `main` also publishes the NSIS installer as a GitHub Release.

- Release: bump `version` in `src-tauri/Cargo.toml`, run `cargo build` to refresh `Cargo.lock`, commit both and push to `main`. No bump means nothing is published (CI still runs). `Cargo.toml` is the only version source; the `package.json` version is unused.
- One-time setup per clone: `git config core.hooksPath .githooks`, and a denylist of work terms at `%USERPROFILE%\.day-navigator\push-denylist.txt` (the pre-push hook blocks pushes without it).
- Stack background: `@context/foundation/tech-stack.md`.
