---
bootstrapped_at: 2026-11-20T12:07:07Z
starter_id: tauri
starter_name: Tauri
project_name: day-navigator
language_family: rust
package_manager: npm
cwd_strategy: subdir-then-move
bootstrapper_confidence: verified
phase_3_status: ok
audit_command: cargo audit
---

## Hand-off

Source: `context/foundation/tech-stack.md`

```yaml
starter_id: tauri
package_manager: cargo
project_name: day-navigator
hints:
  language_family: rust
  team_size: solo
  deployment_target: github-releases
  ci_provider: github-actions
  ci_default_flow: auto-deploy-on-merge
  bootstrapper_confidence: verified
  path_taken: standard
  quality_override: false
  self_check_answers: null
  has_auth: false
  has_payments: false
  has_realtime: false
  has_ai: false
  has_background_jobs: true
```

**Session override (confirmed by user):** `package_manager: cargo` → `npm` for this run only. Reason: create-tauri-app's `--manager cargo` only offers Rust-frontend templates (vanilla, Yew, Leptos, Sycamore, Dioxus), so `--template react-ts` needs a JS package manager. The hand-off body itself notes this. The file on disk was not changed.

### Why this stack

A solo developer building a personal Windows desktop app over 12 after-hours weeks, at small scale, with no in-app auth, payments, realtime or AI, but with substantial background work: interval sync of assigned issues, morning and work-time reminders, break intervals and a scheduled distraction blocker. Tauri is the recommended default for desktop in the Rust family and matches the user's own earlier decision for a local desktop app over CLI tools and files. It clears all four agent-friendly gates and its scaffolding is verified end to end. The Rust side covers the OS integrations the PRD leans on (tray, autostart, global shortcut, notifications, scheduled tasks), while the UI uses the card's default React + TypeScript template, chosen over the Vue setup of an earlier prototype. Installers are published as GitHub Releases by GitHub Actions on every merge to main. Note for scaffolding: the React template needs a JavaScript package manager for the frontend (npm), even though the card lists cargo for the Rust side.

## Pre-scaffold verification

| Signal      | Value                                          | Severity | Notes                                                                                                   |
| ----------- | ---------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------- |
| npm package | create-tauri-app v4.7.5 published 2026-10-09   | fresh    | resolved from cmd_template (`npm create tauri-app`); checked although language_family is rust, because the CLI ships via npm |
| GitHub repo | not run                                        | —        | card `docs_url` is `https://tauri.app`, not a GitHub URL — no repo recency signal available            |

Local toolchain observed (informational): Node v24.21.0, npm 11.19.0, cargo/rustc 1.89.0 at scaffold time — updated to 1.100.0 afterwards (see Post-scaffold audit).

## Scaffold log

**Resolved invocation**: `npm create tauri-app@latest .bootstrap-scaffold -- --template react-ts --manager npm --yes`
**Strategy**: subdir-then-move (scaffold into a temporary directory, then move files up)
**Exit code**: 0
**Files moved**: 37 (plus `.gitignore` append-merged)
**Conflicts (.scaffold siblings)**: none
**.gitignore handling**: append-merged (scaffold lines appended under `# from tauri`; no duplicates with existing lines)
**.bootstrap-scaffold cleanup**: deleted

**Post-scaffold rename:** the temporary directory name leaked into project identifiers, so before the move `bootstrap-scaffold` → `day-navigator` and `bootstrap_scaffold_lib` → `day_navigator_lib` in:
- `package.json` (`name`)
- `src-tauri/Cargo.toml` (package `name`, lib `name`)
- `src-tauri/src/main.rs` (`day_navigator_lib::run()`)
- `src-tauri/tauri.conf.json` (`productName`, `identifier` → `com.sebas.day-navigator`, window `title`)

**Files moved:**
.vscode/extensions.json, README.md, index.html, package.json, public/tauri.svg, public/vite.svg, src-tauri/.gitignore, src-tauri/Cargo.toml, src-tauri/build.rs, src-tauri/capabilities/default.json, src-tauri/icons/ (16 icon files), src-tauri/src/lib.rs, src-tauri/src/main.rs, src-tauri/tauri.conf.json, src/App.css, src/App.tsx, src/assets/react.svg, src/main.tsx, src/vite-env.d.ts, tsconfig.json, tsconfig.node.json, vite.config.ts

`context/` was preserved untouched (the scaffold shipped nothing under it).

Note: the template does not install dependencies — `npm install` (frontend) and the first `cargo build` (Rust side, via `npm run tauri dev`) have not run yet.

## Post-scaffold audit

_First attempt failed (cargo-audit not installed). Re-run on 2026-11-20 at the user's request after: `cargo install cargo-audit --locked` (v0.23.0), `npm install`, and `rustup update stable` (rustc 1.89.0 → 1.100.0, see toolchain note below)._

**Tool**: cargo audit (`cargo audit --json`, in `src-tauri/`) + npm audit (`npm audit --json`, frontend half)
**Summary**: 0 CRITICAL, 0 HIGH, 0 MODERATE, 0 LOW (both tools)
**Direct vs transitive**: no vulnerabilities; all 6 informational warnings below are transitive (pulled in by `tauri`), none in direct dependencies

#### CRITICAL / HIGH / MODERATE / LOW findings

None. npm: 0 vulnerabilities across 69 dependencies. cargo: 0 vulnerabilities.

#### Informational warnings (cargo audit, not vulnerabilities)

| Kind         | Crate             | Version | Advisory          |
| ------------ | ----------------- | ------- | ----------------- |
| unmaintained | chrono            | 0.4.45  | RUSTSEC-2026-0543 |
| unmaintained | proc-macro-error  | 1.0.4   | RUSTSEC-2024-0370 |
| unmaintained | zbus_names        | 4.3.5   | RUSTSEC-2026-0371 |
| unmaintained | zvariant          | 5.15.1  | RUSTSEC-2026-0370 |
| unmaintained | zvariant_derive   | 5.15.1  | RUSTSEC-2026-0372 |
| unsound      | glib              | 0.18.5  | RUSTSEC-2024-0429 |

`glib`, `zbus*` and `zvariant*` belong to the Linux (GTK/D-Bus) dependency tree and are not compiled into the Windows build.

#### Toolchain fix

The first `npm run tauri dev` reported version-mismatched Tauri packages: Rust `tauri` 2.11.6 vs npm `@tauri-apps/api` 2.12.5, and `tauri-plugin-opener` 2.5.5 vs npm 2.7.3. Cause: rustc 1.89 was too old. tauri 2.12 / opener 2.7 need Rust 1.90, so cargo's MSRV-aware resolver (edition 2024) picked older crates. Fixed with `rustup update stable` → rustc 1.100.0, then `cargo update` → tauri 2.12.7, tauri-plugin-opener 2.7.3.

#### Smoke test

- `npm run build` (tsc + vite): ok
- `cargo build` (debug): ok, no warnings
- `npm run tauri dev`: window "day-navigator" opens, no version-mismatch warning; stopped afterwards

## Hints recorded but not acted on

| Hint                    | Value                 |
| ----------------------- | --------------------- |
| bootstrapper_confidence | verified              |
| quality_override        | false                 |
| path_taken              | standard              |
| self_check_answers      | null                  |
| team_size               | solo                  |
| deployment_target       | github-releases       |
| ci_provider             | github-actions        |
| ci_default_flow         | auto-deploy-on-merge  |
| has_auth                | false                 |
| has_payments            | false                 |
| has_realtime            | false                 |
| has_ai                  | false                 |
| has_background_jobs     | true                  |

## Next steps

Next: a future skill will set up agent context (CLAUDE.md, AGENTS.md). For now, your project is scaffolded and verified — happy hacking.

Useful manual steps in the meantime:
- `git init` (if you have not already) to start your own repo history.
- Review any `.scaffold` siblings the conflict policy created and decide which version of each file to keep.
- Address audit findings per your project's risk tolerance — the full breakdown is in this log.
