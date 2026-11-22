---
project: day-navigator
assessed_at: 2026-11-22T21:54:30+01:00
agent_readiness: ready-with-compensation
context_type: brownfield
stack_components:
  language: Rust (edition 2024) + TypeScript 6.0 (strict)
  framework: Tauri 2.12 + React 19.3
  build_tool: Vite 8.3 + Cargo (tauri-cli 2)
  test_runner: cargo test (frontend: null)
  package_manager: npm + cargo
  ci_provider: GitHub Actions
  deployment_target: NSIS installer in GitHub Releases (tauri-action)
gates_passed: 11
gates_failed: 2   # 1 failed (React conventions) + 1 partial (Tauri 2 in training data)
---

## Stack Components

**Language — Rust and TypeScript.** The backend is Rust, edition 2024 (`src-tauri/Cargo.toml`), and needs rustc 1.90 or newer for tauri 2.12 (`AGENTS.md`). The UI is TypeScript 6.0.3 (resolved in `package-lock.json`). `tsconfig.json` sets `strict: true`, `noUnusedLocals` and `noUnusedParameters`. The two sides share contracts through serde-serialised types: Rust error enums use `#[serde(tag = "kind", content = "detail")]` (`src-tauri/src/tasks.rs:21`, `src-tauri/src/vault/error.rs:12`), and `src/tasks/api.ts` mirrors them as TypeScript tagged unions.

**Framework — Tauri 2 and React 19.** Tauri resolves to 2.12.7 (`src-tauri/Cargo.lock`), with the `tray-icon` feature and the plugins `opener`, `notification`, `single-instance` and `log`. Commands are registered in `src-tauri/src/lib.rs`, and permissions live in `src-tauri/capabilities/default.json`. React resolves to 19.3.1. There is no router: `src/App.tsx` stacks the views and says so ("No navigation yet (S-01): the views are stacked."). The UI is grouped by feature (`src/health/`, `src/tasks/`), with shared UI in `src/components/ErrorState.tsx`.

**Build tool — Vite and Cargo.** Vite 8.3.8 with `@vitejs/plugin-react` 6 (`vite.config.ts`) builds the frontend. `npm run build` runs `tsc && vite build`. Cargo, through `@tauri-apps/cli` 2, builds the backend and the bundle; the release profile uses LTO and `panic = "abort"`.

**Test runner — cargo test only.** `src-tauri/tests/` holds the integration tests `health_probe.rs`, `tasks.rs`, `vault_guard.rs` and `vault_write.rs`, a shared `common/` (`test_vault()`) and an invented fixture vault. `proptest` is a dev-dependency, and the `test-support` feature exposes the unguarded write layer to tests only. The frontend has no test runner (no Vitest, Jest or Playwright config).

**Package managers.** npm (`package-lock.json`) and cargo (`Cargo.lock`). CI runs `npm ci` and `cargo build/test --locked`.

**CI/CD.** GitHub Actions (`.github/workflows/ci.yml`) runs gitleaks on the full history, `cargo audit`, `npm audit --audit-level=high`, the pre-push hook self-test (`scripts/test-pre-push.sh`), `npm run build`, `cargo build --locked` and `cargo test --locked`. A version bump on `main` publishes an installer through `tauri-apps/tauri-action@v1`.

**Instruction files.** `AGENTS.md` exists, and there is no `CLAUDE.md`. No linter or formatter is configured: no ESLint or Prettier, no `rustfmt.toml`, and neither clippy nor `cargo fmt --check` runs in CI.

## Quality Gate Assessment

| Component            | Typed | Convention | Training Data | Documented | Verdict                    |
|----------------------|-------|------------|---------------|------------|----------------------------|
| Rust (edition 2024)  | ✓     | —          | —             | —          | pass                       |
| TypeScript 6 (strict)| ✓     | —          | —             | —          | pass                       |
| Tauri 2.12           | —     | ✓          | ~             | ✓          | pass with note             |
| React 19 (no router) | —     | ✗          | ✓             | ✓          | fail on conventions        |
| Vite 8               | —     | ✓          | ✓             | ✓          | pass                       |
| cargo test           | —     | —          | ✓             | ✓          | pass                       |

Legend: ✓ = pass, ✗ = fail, ~ = partial, — = not applicable. 13 applicable checks: 11 pass, 1 partial, 1 fail.

### Gate Details

- **Rust, typed ✓.** Rust is typed by the language. Contracts that cross into the UI are serde types with explicit tagging (`tasks.rs:21`, `vault/error.rs:12`, `health/mod.rs:53`).
- **TypeScript, typed ✓.** `tsconfig.json:18-20` sets `strict: true`, `noUnusedLocals` and `noUnusedParameters`, and `npm run build` fails on type errors (`package.json` script `tsc && vite build`). One caveat: the TS types in `src/*/api.ts` are hand-mirrored from Rust and nothing generates them, so drift between the two is caught only at runtime. The compensation below covers it.
- **Tauri, convention-based ✓.** Tauri fixes the layout (`src-tauri/`, `tauri.conf.json`, `capabilities/*.json`), the command model (`#[tauri::command]` registered in the builder) and the plugin and permission pairing. `AGENTS.md` ("Project Structure") documents where each part goes.
- **Tauri, popular in training data ~.** Tauri is the leading desktop framework in the Rust family. Tauri 2 (stable since late 2024) changed core APIs from v1, though: the v1 allowlist became capabilities and permissions, many core APIs moved into plugins, and import paths changed (`@tauri-apps/api/core`). Training data mixes v1 and v2, so agents tend to produce v1 idioms (`allowlist`, `tauri::api::*`, `@tauri-apps/api/tauri`).
- **Tauri, well documented ✓.** Versioned official docs for v2 exist at v2.tauri.app, with per-plugin pages.
- **React, convention-based ✗.** This is the textbook failing case: Vite + React with no router (`src/App.tsx` stacks the views). The new PRD adds four views (GitLab, GitHub, My tasks, To do) and a timer that is always visible, so how navigation, state and Tauri calls are organised has to be decided and written down. `AGENTS.md` describes `src/` only as "React UI; entry `main.tsx`, root component `App.tsx`". The code already follows a feature-folder pattern, but nothing documents it.
- **React, training data ✓ / documented ✓.** React is the most common UI library in the JS family, and react.dev has current docs for 19. One version caveat: React 19 APIs (`use`, actions, ref-as-prop) are newer than much of the training corpus.
- **Vite ✓ ✓ ✓.** The config is minimal and conventional (`vite.config.ts`, fixed port 1420, `strictPort`), Vite is very common, and vite.dev has versioned docs. Vite 8 is recent, but the project barely touches its config, so that carries little risk.
- **cargo test ✓ ✓.** It is the standard Rust test harness and is well documented. The project's own conventions (fixture copied into a temp dir, never touched in place; invented data only) are in `AGENTS.md` ("Testing").

## Gaps & Compensation

1. **The React UI has no convention (fails on conventions).** Without a documented pattern, each new view an agent adds (four in this MVP) can invent its own state handling, its own `invoke` calls and its own error rendering. That breaks the PRD guardrail "never an empty list in place of a failed source" in different ways in different places. *Compensation:* document the feature-folder pattern that already exists, a single navigation rule, one place for Tauri calls, and mandatory error states.
2. **Training data mixes Tauri v1 and v2 (partial).** Agents may write v1 config or imports that do not compile, or forget to add capability permissions for a new plugin. *Compensation:* add explicit v2-only rules, a "do not use" list and links to the v2 docs.
3. **Hand-mirrored Rust↔TS contracts (typed, with a caveat).** A renamed field or a new error variant on the Rust side silently breaks the UI. *Compensation:* add a rule that changes both sides in the same commit, plus the serde casing convention.
4. **No lint or format gate (outside the four criteria, still a gap for agents).** Agents get no fast mechanical feedback on style or common mistakes. *Compensation:* add `cargo fmt --check` and `cargo clippy -- -D warnings` to the documented verification commands. Adding them to CI is for `/10x-health-check` to judge.
5. **No frontend tests (outside the four criteria).** UI logic in the new MVP (the "To do" limit, carry-over, the hidden/changed display) could only be checked by hand. *Compensation:* the rule below keeps the domain rules in Rust, where `cargo test` covers them, and the UI thin. Whether to add a frontend runner is for `/10x-health-check`.
6. **Hard rules in `AGENTS.md` collide with the new PRD (not a criteria gap, but it will mislead agents).** The current hard rule allows app-owned state only as "`settings.json`, and the app log dir". The new PRD needs app-owned storage for own tasks, the "To do" list, hidden-issue snapshots and work-time entries. It also states that the MVP does not touch vaults at all, and it calls GitLab/GitHub through `glab`/`gh`, which the rules mention only for credentials. Without an update, a careful agent will either refuse to store data or put it in a vault. *Compensation:* replace that rule and add a CLI-invocation rule, as below.

### Recommended Instruction File Additions

Paste into `AGENTS.md`. The first block replaces the existing "Read and write only inside folders configured in settings…" hard rule.

```markdown
- Vaults are out of scope for the current MVP (`@context/foundation/prd.md`): no code path reads or writes a vault. Outside configured folders the app writes only its own state under its bundle identifier: the app config dir (`settings.json`), the app data dir (own tasks, the "To do" list, hidden-issue snapshots, work-time entries) and the app log dir. Never user content or vault data there. If a configured path is missing, show an error; never create it silently.
- GitLab and GitHub are reached only by running the logged-in `glab` / `gh` CLIs from Rust with a fixed program name and fixed argument lists (no shell, no string-built commands, no user-supplied flags). Read-only: never call a subcommand that creates, edits, closes, comments or assigns. An unreachable work GitLab (no VPN) is a calm "unavailable" state, not an alarm; a logged-out CLI is an error.
```

```markdown
## Frontend Conventions (React 19, no router)

- One folder per feature in `src/` (existing: `src/health/`, `src/tasks/`): `<Name>View.tsx` (the screen), `api.ts` (the only file that imports `invoke` from `@tauri-apps/api/core`; one typed function per command), `types.ts` (TS mirrors of Rust serde types), `use<Name>.ts` (loading/error/data state). Shared UI goes in `src/components/`.
- Navigation: `App.tsx` owns the active view in one piece of state and renders a fixed header (the work-time counter, always visible) plus the active view. Do not add a router library or global state library without a plan that says so.
- Every view that shows a source renders three distinct states: loading, error (via `src/components/ErrorState.tsx`, with the time of the last successful refresh) and data. "Empty" is a data state with an explicit message; never render an empty list for a failed source.
- Domain rules (hidden-issue reappearance, "To do" limit and carry-over, work-time norm and deviation) live in Rust and are unit-tested there; the UI only displays results and sends user actions.
```

```markdown
## Rust ↔ TypeScript Contracts

- Every `#[tauri::command]` input/output type is a serde struct with `#[serde(rename_all = "camelCase")]`; error enums use `#[derive(thiserror::Error, Serialize)]` with `#[serde(tag = "kind", content = "detail")]` (see `src-tauri/src/vault/error.rs`).
- The TS mirror in the feature's `api.ts`/`types.ts` is a tagged union on `kind`. Change the Rust type and its TS mirror in the same commit; `npm run build` must pass afterwards.
```

```markdown
## Tauri 2 Only

- This is Tauri 2 (2.12). Do not use v1 idioms: no `allowlist` in `tauri.conf.json`, no `tauri::api::*`, no `@tauri-apps/api/tauri` (use `@tauri-apps/api/core`).
- Adding a plugin = Rust crate in `src-tauri/Cargo.toml` + `.plugin(...)` in `lib.rs` + npm `@tauri-apps/plugin-*` on the same major.minor + its permissions in `src-tauri/capabilities/default.json`.
- Docs: https://v2.tauri.app/ (guides), https://v2.tauri.app/plugin/ (plugins), https://docs.rs/tauri/2 (Rust API).
- Versions in this repo: React 19, TypeScript 6, Vite 8, Rust edition 2024. Prefer current APIs (React 19 ref-as-prop, `use`) over older patterns.
```

```markdown
## Verification (add to "Build and Development Commands")

- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`) before every commit, alongside `cargo test`, `cargo build` and `npm run build`.
```

## Summary

**Overall readiness: ready with compensation.** The stack is typed end to end (Rust plus strict TypeScript), every component is mainstream in its language family and has versioned official docs, and the backend already has strong testing conventions (invented fixture vault, property tests, a test-only feature flag) backed by a CI that gates every push.

**Strengths:** strict typing on both sides; Tauri's enforced project layout and permission model; a tested backend; CI with secret scanning, audits, build and tests; an `AGENTS.md` that already states the data-safety rules.

**Gaps:** the React UI has no documented conventions, which matters now because the MVP adds four views and a persistent header. Training data mixes Tauri v1 and v2. The Rust↔TS contracts are mirrored by hand. There is no lint or format gate and no frontend tests. The `AGENTS.md` storage rule contradicts the new PRD's app-owned data.

**Recommended next step:** paste the blocks above into `AGENTS.md`, especially the storage-rule replacement, before the first MVP slice is planned. Then run `/10x-health-check` to audit dependency health, test coverage and CI against these gaps.
