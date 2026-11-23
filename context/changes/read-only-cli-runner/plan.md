# Read-only CLI runner Implementation Plan

## Overview

Roadmap foundation F-02. The app gets one place that starts `glab` and `gh`: a closed list of read-only calls, run without a shell, with fixed arguments and a 20 s limit. Each call ends in exactly one of five states: data, unavailable (no VPN or network, or the server is down), logged out, tool missing, error. A fake CLI binary, built only for tests, produces every state on invented data. S-01 (GitLab list) and S-05 (GitHub issues) build on it; this change has no view, tray source or Tauri command.

## Current State Analysis

- Nothing starts a process today: there is no `std::process::Command`, `glab` or `gh` anywhere in `src-tauri/src` or `src-tauri/tests`.
- The archived source-health plan left CLI calls and the `CREATE_NO_WINDOW` console-flash concern to the first CLI slice (`context/archive/2026-11-20-source-health-status/plan.md:58`).
- Both CLIs are installed on Windows: `glab` 1.124.0 and `gh` 2.105.0. `gh auth status` exits 0, so `gh` is logged in on Windows. This settles PRD/roadmap Open Question #1 in favour of `gh.exe` on Windows (decision below); updating the roadmap text is left to `/10x-roadmap`.
- Observed failure shapes (probed against invented `.invalid` hosts, 2026-11-23):
  - `glab` exits **1 for every failure**. No network: `Get "https://<host>/api/v4/user": dial tcp: lookup <host>: i/o timeout.` or `dial tcp <ip>:443: i/o timeout.` Logged out: `{"message":"401 Unauthorized"}glab: 401 Unauthorized (HTTP 401)`. Unknown command: `Unknown command "x" for "glab"`.
  - `gh` exits **4 when logged out** (`To get started with GitHub CLI, please run:  gh auth login`) and 1 without network (`error connecting to <host>` / `check your internet connection ...`).
  - stderr carries the full URL with the hostname. For the work GitLab that hostname is work data, so stderr must not leave the classifier.
- A bin target with `required-features = ["test-support"]` is skipped by `cargo build` and `cargo build --bins`, and is built by `cargo test`, where `env!("CARGO_BIN_EXE_<name>")` points at it (checked with a throwaway crate using the same self-dev-dependency trick as `src-tauri/Cargo.toml:41-44`).

## Desired End State

`day_navigator_lib::cli` exposes:

- `CliCall`, a closed enum of allowed calls. F-02 ships `GlabVersion`, `GlabAuthStatus`, `GhVersion`, `GhAuthStatus`. Each maps to one `Tool` and one fixed `&'static [&'static str]` argument list. A test proves every variant is read-only.
- `CliRunner::system().run(call) -> Result<String, CliError>`. It is blocking; callers wrap it in `spawn_blocking` the way `lib.rs:74` does. `Ok(stdout)` is the data state. `CliError` is `Unavailable`, `LoggedOut`, `ToolMissing` or `Failed { exit_code }`; each carries only the tool and, for `Failed`, the exit code, plus a fixed Polish description.
- `fake-cli`, a test-only binary that plays each scenario. `tests/cli_runner.rs` drives every state through a real process.

Verify with `cargo test` (all states, exact arguments, timeout and kill, missing program, no stderr leak), the full pre-commit gate from `AGENTS.md`, and a manual run of the ignored real-CLI smoke test with the VPN on and off.

### Key Discoveries:

- Error enum pattern: `thiserror::Error` + `Serialize` with `#[serde(tag = "kind", content = "detail")]` (`src-tauri/src/vault/error.rs:11-13`, required by `AGENTS.md`).
- Keep pure rules free of `tauri` and of side effects, as `src-tauri/src/health/mod.rs:1-2` does. Here `classify` is the pure rule and `runner` the side effect.
- Test-only API is gated with `#[cfg(any(test, feature = "test-support"))]`, enabled for `cargo test` alone by the self dev-dependency (`src-tauri/Cargo.toml:36-44`).
- Blocking work runs off the main thread via `tauri::async_runtime::spawn_blocking` (`src-tauri/src/lib.rs:72-85`). The runner stays synchronous and the callers wrap it.
- Log hygiene precedent: logs carry ids and kinds only, never paths or messages (`context/archive/2026-11-20-source-health-status/plan.md:112`; `src-tauri/src/runtime.rs:294-301`).
- The package already has the `src/main.rs` bin (`day-navigator`). A second bin needs `default-run` so `cargo run` / `npm run tauri dev` still know which one to start.

## What We're NOT Doing

- No GitLab/GitHub queries for issues or MRs. Those calls are added as new `CliCall` variants by S-01 and S-05.
- No tray or health-registry source for `glab`/`gh`, no calm `Unavailable` state in `SourceState`, no login probe loop. S-01 adds these (its roadmap outcome includes the tray).
- No Tauri command and no TS mirror of `CliError`. Nothing in the UI calls the runner yet. The first slice that returns `CliError` from a command adds the TS mirror in the same commit.
- No WSL path for `gh`. If `gh.exe` is missing on Windows, the result is `ToolMissing`.
- No arguments built from settings or user input. A later call that needs a value (e.g. an `owner/repo` from settings in S-05) adds a validated newtype inside its `CliCall` variant; that is S-05's job.
- No program-path setting. Programs are resolved from `PATH` only.
- No surfacing of stderr text anywhere (panel, log, toast, error value).
- No automated check that no console window flashes. `cargo test` runs from a console, so a flash cannot show there. The first visible check is S-01's manual verification, when the GUI app calls the runner.
- No retry or back-off. The caller decides when to call again.

## Implementation Approach

The closed `CliCall` enum is the security contract: a write call or a string-built argument cannot be expressed, and every new call shows up in one file for review. Classification is a pure function over `(tool, exit code, stderr)` so the fragile part (matching CLI text) is covered by fast table tests on invented samples. The runner is the only code that touches processes. It adds the time limit, kill, fixed environment, output caps and the `ToolMissing` mapping, and it drops stderr as soon as the classifier has seen it.

Decisions settled during planning: a timeout of 20 s means `Unavailable`; HTTP 5xx means `Unavailable` (calm); errors carry kind and exit code only; read-only is enforced by the closed enum; tests use a real fake binary; tray wiring stays with S-01/S-05; `gh` is `gh.exe` on Windows.

## Critical Implementation Details

**Pipe draining and timeout ordering.** Read stdout and stderr on two threads from the moment the child is spawned. If they are read only after the process exits, a child that fills a pipe buffer blocks forever and every call ends in the timeout. Poll `try_wait` until the deadline, then `kill` and `wait`. After the child is gone, join the readers with a short bound (about 2 s). A grandchild that inherited the pipes can keep them open, and the runner must not hang on it.

**Classification order.** Exit code 0 is data, whatever stderr says (update notices and similar noise). For a non-zero exit: `gh` exit code 4 is logged out regardless of text; then unavailable (network or 5xx text); then logged out (login text); then fall back to `Failed`. Network text goes before login text because both CLIs append login hints to network failures. `glab auth status` adds "run glab auth login --hostname …" whenever the token is stored in plain text in its config, even when the call succeeded (glab 1.124 `internal/commands/auth/status/status.go:251-253`). `gh auth status` reports a refused or unresolved connection, but not a timeout, as "The token … is invalid" plus "To re-authenticate, run: gh auth login" (gh 2.105 `pkg/cmd/auth/status/status.go:505-515`, 142-149; both checked 2026-11-23). A transport failure means auth was never checked, so the network text wins.

## Phase 1: Call contract and classification

### Overview

Pure code with no processes: the call list, the error type with its Polish descriptions, the classifier and the read-only guard, all unit-tested on invented data.

### Changes Required:

#### 1. CLI module root

**File**: `src-tauri/src/cli/mod.rs` (new), `src-tauri/src/lib.rs`

**Intent**: Define the vocabulary every source slice uses. `lib.rs` gets `pub mod cli;` next to the other modules.

**Contract**:
- `pub enum Tool { Glab, Gh }`. It serializes as `"glab"` / `"gh"` (`#[serde(rename_all = "lowercase")]`) and has `program(self) -> &'static str` (`"glab"` / `"gh"`).
- `pub enum CliCall { GlabVersion, GlabAuthStatus, GhVersion, GhAuthStatus }` (`Copy`, `Debug`, `Eq`) with:
  - `pub const ALL: &'static [CliCall]`
  - `fn tool(self) -> Tool`
  - `fn args(self) -> &'static [&'static str]`: `["version"]`, `["auth", "status"]`, `["--version"]`, `["auth", "status"]`
  - `fn name(self) -> &'static str`: a stable log key such as `"glab:version"`, `"gh:auth-status"`
- `CliError`, derived `Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize`, with `#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]`. Its variants are `Unavailable { tool }`, `LoggedOut { tool }`, `ToolMissing { tool }` and `Failed { tool, exit_code: Option<i32> }`. The `Display` text is English with no stderr.
- `CliError::kind(&self) -> &'static str` returns the variant name, for logs. `CliError::message_pl(&self) -> String` returns a fixed Polish text that never includes stderr or a hostname:
  - Unavailable: `"GitLab niedostępny (brak VPN, sieci lub serwer nie odpowiada)"` / `"GitHub niedostępny (brak sieci lub serwer nie odpowiada)"`. The text names the server too, because a 5xx error and the 20 s timeout map here as well.
  - LoggedOut: `"Nie zalogowano w glab — uruchom glab auth login"` / the same for `gh`
  - ToolMissing: `"Nie znaleziono programu glab"` / `gh`
  - Failed: `"glab zakończył się błędem (kod N)"`, or `"... (bez kodu)"` when there is no exit code
- `pub mod classify; pub mod runner;` (the runner arrives in Phase 2) plus re-exports of `CliRunner` and `classify`.

#### 2. Classifier

**File**: `src-tauri/src/cli/classify.rs` (new)

**Intent**: Turn a finished process's exit code and stderr into a result, following the order in Critical Implementation Details. Only this function ever reads stderr.

**Contract**: `pub fn classify(tool: Tool, exit_code: Option<i32>, stderr: &str) -> Result<(), CliError>`, case-insensitive matching on fixed substrings:
The checks run in this order, and the first match wins:
1. Exit code `Some(0)` → `Ok(())`.
2. `gh` with exit code `Some(4)` → `LoggedOut`.
3. Unavailable: `dial tcp`, `proxyconnect`, `i/o timeout`, `no such host`, `connection refused`, `connectex`, `error connecting to`, `tls handshake timeout`, `context deadline exceeded`, `connection reset`, `network is unreachable`; or an HTTP 5xx status, meaning `http 5` followed by two digits (matches glab's `(HTTP 503)` and gh's `HTTP 502: Bad Gateway`, but not `https://`).
4. Logged out: stderr containing `401 unauthorized`, `(http 401)`, `http 401`, `auth login`, `not logged in`, `no token`.
5. Anything else → `Failed { tool, exit_code }`.

#### 3. Unit tests

**File**: `src-tauri/src/cli/mod.rs` and `src-tauri/src/cli/classify.rs` (`#[cfg(test)]` modules)

**Intent**: Pin the contract and the classifier on invented samples built from the shapes in Current State Analysis, with hosts like `gitlab.example.invalid` and `github.example.invalid`.

**Contract**:
- **Read-only guard:** for every `CliCall::ALL`, the arguments are non-empty. The first argument is `version` or `--version`, or the list is exactly `auth status`. No argument is one of `-X`, `--method`, `-f`, `-F`, `--field`, `--raw-field`, `--input`, `-d`, `--data`, and none contains whitespace or one of ``&|<>^%"`;$``.
- **Exhaustiveness:** a test helper `fn ordinal(c: CliCall) -> usize` uses an exhaustive `match` (no `_` arm) to give each variant a distinct ordinal `0..VARIANT_COUNT`, with `const VARIANT_COUNT: usize` defined next to it. The test asserts that `ALL.len() == VARIANT_COUNT` and that `ALL` maps one-to-one onto `0..VARIANT_COUNT`. A new variant first fails the build at the `match`; once the author adds an arm and bumps the count, the test fails until the variant is also in `ALL`.
- **Classification table:** one row per sample → expected result. It covers both tools' logged-out shapes, DNS and timeout shapes, `connectex` refused, `(HTTP 503)` and `HTTP 502:`, exit 0 with update-notice noise on stderr (→ data), an unknown error with exit 2 (→ `Failed { exit_code: Some(2) }`), `None` exit code with empty stderr (→ `Failed { exit_code: None }`), an `https://...` URL with a 5xx-looking port or path that must not count as HTTP 5xx, and two mixed rows that must give `Unavailable`: a glab `API call failed: … dial tcp …: i/o timeout` followed by `run glab auth login --hostname gitlab.example.invalid`, and a gh `proxyconnect tcp: dial tcp …: connectex: … refused` followed by `The token in keyring is invalid` and `To re-authenticate, run: gh auth login -h github.example.invalid`.
- **No-leak check:** for every error built from a sample whose stderr contains `example.invalid`, none of `Debug`, `Display`, `serde_json::to_string` or `message_pl` contains `example.invalid`.
- **Serialization:** `CliError::Failed` serializes as `{"kind":"Failed","detail":{"tool":"glab","exitCode":2}}`.

### Success Criteria:

#### Automated Verification:

- Unit tests pass: `cargo test --lib cli` (in `src-tauri/`)
- Format and lint are clean: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
- Full Rust suite still passes: `cargo test`

#### Manual Verification:

- Reviewer confirms each `CliCall` argument list is read-only and the Polish texts read naturally

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Process runner and fake CLI

### Overview

The process side: start the real program, limit time and output, map spawn failures, classify, and drop stderr. The test-only fake binary drives all five states through a real process.

### Changes Required:

#### 1. Runner

**File**: `src-tauri/src/cli/runner.rs` (new)

**Intent**: The only code in the app that starts `glab`/`gh`. It runs one `CliCall` with no shell and no prompt and returns the classified result within the time limit.

**Contract**:
- `pub struct CliRunner` holds the program for each tool, the timeout and, behind test-support only, extra environment variables.
- `CliRunner::system()` uses `Tool::program()` (resolved from `PATH` by `Command::new`) and `CliRunner::TIMEOUT = 20 s`.
- `#[cfg(any(test, feature = "test-support"))] pub fn with_programs(glab: PathBuf, gh: PathBuf, timeout: Duration, env: Vec<(String, String)>) -> Self` exists for tests only. Production code cannot inject a program path or environment variables.
- `pub fn run(&self, call: CliCall) -> Result<String, CliError>` does the following:
  - `Command::new(program).args(call.args())`, with stdin null and stdout/stderr piped. On Windows it adds `creation_flags(CREATE_NO_WINDOW)` (`0x0800_0000`, via `std::os::windows::process::CommandExt`, under `#[cfg(windows)]`).
  - Fixed environment on top of the inherited one: `NO_COLOR=1`, `CLICOLOR=0`, `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, `GH_NO_EXTENSION_UPDATE_NOTIFIER=1`, `GH_SPINNER_DISABLED=1`, `GLAB_NO_PROMPT=true`, `GLAB_CHECK_UPDATE=false`, `GLAB_NOTIFY_SKILL_UPDATES=false`. Names were checked against `gh help environment` and `glab config --help` on 2026-11-23. The app never sets a token.
  - Spawn error `NotFound` → `ToolMissing`. Any other spawn error → `Failed { exit_code: None }`.
  - The deadline passes → kill, wait, `Unavailable`.
  - Output caps: stdout 16 MiB, beyond that → `Failed { exit_code: None }`. Stderr keeps only its first 64 KiB for the classifier. Both pipes are drained to the end either way.
  - Exit 0 with stdout that is not UTF-8 → `Failed { exit_code: Some(0) }`.
  - Otherwise the result is `classify(...)` followed by `Ok(stdout)`.
- Logging: one line per call with `call.name()`, the outcome kind (`data` or `CliError::kind()`) and the elapsed ms. Data goes to `log::debug!`, failures to `log::info!`. Never stdout, stderr or a message.

#### 2. Fake CLI binary and Cargo wiring

**File**: `src-tauri/src/bin/fake_cli.rs` (new), `src-tauri/Cargo.toml`

**Intent**: A tiny program the tests put in place of `glab`/`gh`. The `FAKE_CLI_SCENARIO` environment variable picks what it prints, its exit code, or whether it hangs. It uses invented data only and is never part of the installer.

**Contract**:
- In `Cargo.toml`: `default-run = "day-navigator"` under `[package]`, plus `[[bin]] name = "fake-cli"`, `path = "src/bin/fake_cli.rs"`, `required-features = ["test-support"]`.
- The scenarios are:
  - `data`: stdout is a JSON object with the received argv, exit 0
  - `data-with-noise`: the same, plus an update notice on stderr
  - `glab-logged-out`, `gh-logged-out` (exit 4), `glab-no-network`, `gh-no-network`, `http-503`
  - `unknown-error` (exit 2)
  - `hang`: sleeps 5 s, then writes the file named in `FAKE_CLI_MARKER` if that variable is set, then sleeps on
  - `big-output`: just over 16 MiB on stdout
  - `non-utf8`
  - an unknown scenario: exit 99
- Its stderr samples use `gitlab.example.invalid` / `github.example.invalid` only.
- Keeping the fake out of the installer depends on two rules, so put both in a comment above the `[[bin]]` entry. (a) `path` stays exactly `src/bin/fake_cli.rs`. tauri-cli 2.12.6 builds with `cargo build --bins`, which skips the fake because its required feature is off. Its own binary list (`get_binaries`, `crates/tauri-cli/src/interface/rust.rs:937-1023`) drops `[[bin]]` entries whose required features are off. It also drops the matching `src/bin/*` file, which it matches by path only, since `fake-cli` ≠ `fake_cli`. The NSIS bundler copies every listed non-main binary into the installer, and fails if one was not built. (b) `test-support` is never added to `default` features.

#### 3. Integration tests

**File**: `src-tauri/tests/cli_runner.rs` (new)

**Intent**: Prove each state through a real process, using `env!("CARGO_BIN_EXE_fake-cli")` for both tools via `CliRunner::with_programs`.

**Contract** (one test per bullet):
- For every `CliCall::ALL` with `data`: `Ok`, and the echoed argv equals exactly `call.args()`. This proves fixed arguments and no shell.
- `data-with-noise` → `Ok`.
- Each logged-out, no-network and `http-503` scenario → the expected variant for the matching tool.
- `unknown-error` → `Failed { exit_code: Some(2) }`.
- `hang` with a 1 s timeout → `Unavailable`, returned in under 5 s. The marker file is still absent 6 s after the call returned, which proves the process was killed.
- Programs pointing at a nonexistent path in a temp dir → `ToolMissing`.
- `big-output` → `Failed { exit_code: None }`.
- `non-utf8` → `Failed { exit_code: Some(0) }`.
- No-leak: for every failing scenario, the error's `Debug`, `Display`, JSON and `message_pl` do not contain `example.invalid`.

#### 4. Real-CLI smoke test

**File**: `src-tauri/tests/cli_real.rs` (new)

**Intent**: A manual check against the installed CLIs that prints nothing but outcome kinds, so no work data reaches the console or the repo.

**Contract**: One `#[ignore]` test that runs `CliRunner::system()` for every `CliCall::ALL` and prints `<call.name()> -> <kind>` per line. It asserts nothing, because results depend on the machine. Run it with `cargo test --test cli_real -- --ignored --nocapture`.

#### 5. Agent guidance

**File**: `AGENTS.md`

**Intent**: Tell future slices where CLI calls live, so nobody adds a second `Command::new("glab")`.

**Contract**: One bullet under `## Project Structure`: `src-tauri/src/cli/` is the only code that starts `glab`/`gh`; a new call is a new `CliCall` variant with fixed arguments (read-only, covered by the guard test); tests use the `fake-cli` bin (test-support only), never real work data.

### Success Criteria:

#### Automated Verification:

- Integration tests pass: `cargo test --test cli_runner` (in `src-tauri/`)
- Full pre-commit gate passes: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
- Release build does not contain the fake: after `cargo build --release`, `src-tauri/target/release/fake-cli.exe` does not exist. This is a proxy check: the installer comes from `tauri build` in CI, and the rules in §2 keep the fake out of it.

#### Manual Verification:

- With VPN on, the smoke test prints `data` for `glab:version`, `gh:version` and `gh:auth-status`, and `data` or a clear `LoggedOut` for `glab:auth-status`
- With VPN off, the smoke test prints `Unavailable` for `glab:auth-status` within about 20 s, and `data` for `glab:version`
- `npm run tauri dev` still starts the app (`default-run` picks `day-navigator`)

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Read-only guard and exhaustiveness over `CliCall::ALL`.
- Classification table over invented stderr samples for both tools, including the exit-0-with-noise case, the `https://` non-match and `None` exit codes.
- No-leak of `example.invalid` through every error representation; the JSON shape of `CliError`.

### Integration Tests:

- Every state through the real `fake-cli` process, the exact argument echo, the timeout with proof of kill, a missing program, the output cap and non-UTF-8 output.

### Manual Testing Steps:

1. With VPN on, run `cargo test --test cli_real -- --ignored --nocapture` in `src-tauri/` and read the four `name -> kind` lines.
2. Disconnect the VPN, run it again, and confirm `glab:auth-status -> Unavailable` appears within about 20 s.
3. Run `npm run tauri dev` and confirm the app starts as before.

## Performance Considerations

Every call is one short process. The 20 s limit bounds the worst case. The runner is blocking by design, so callers must run it off the main thread (`spawn_blocking`), as the vault task commands do. Output is capped at 16 MiB (stdout) and 64 KiB (stderr) to bound memory.

## Migration Notes

None: new module, no stored data, no settings change.

## Open Risks & Assumptions

- Matching CLI text is fragile: a future `glab`/`gh` release may reword errors. Unknown text falls back to `Failed`, which is loud rather than silently calm. The fix is a new pattern plus a table row.
- `glab auth status` checks every configured host. If a host other than the work GitLab is configured and not logged in, it may exit 1 even with VPN on. The manual smoke test shows this. S-01 chooses its own call (for example `glab api user` on the default host) and does not depend on `auth status`.

## References

- Roadmap item: `context/foundation/roadmap.md` (F-02: Read-only CLI runner; S-01, S-05 build on it). This is the new-MVP roadmap, which at review time (2026-11-23) existed only uncommitted in the main checkout. The committed roadmap still uses the old IDs (F-02 = source health, S-01 = morning plan, S-05 = task template), and so does the archived source-health plan (S-07/S-16). Prerequisite: commit the new roadmap to `main` and merge it into this branch before `/10x-implement`.
- PRD: `context/foundation/prd.md` — FR-015, Constraints & Compatibility ("Aplikacja tylko czyta z GitLaba i GitHuba"), NFR "Nigdy pusta lista zamiast błędu", NFR "Dane pracy i vaulty", Open Question #1
- Error enum pattern: `src-tauri/src/vault/error.rs:11-13`
- Pure-rules split: `src-tauri/src/health/mod.rs:1-2`, `src-tauri/src/health/probe.rs:1-2`
- Blocking helper: `src-tauri/src/lib.rs:72-85`
- Test-support feature: `src-tauri/Cargo.toml:36-44`
- Earlier deferral of CLI calls and `CREATE_NO_WINDOW`: `context/archive/2026-11-20-source-health-status/plan.md:58`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Call contract and classification

#### Automated

- [x] 1.1 Unit tests pass: `cargo test --lib cli` — 36f4d9a
- [x] 1.2 Format and lint are clean: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` — 36f4d9a
- [x] 1.3 Full Rust suite still passes: `cargo test` — 36f4d9a

#### Manual

- [ ] 1.4 Reviewer confirms each `CliCall` argument list is read-only and the Polish texts read naturally

### Phase 2: Process runner and fake CLI

#### Automated

- [x] 2.1 Integration tests pass: `cargo test --test cli_runner` — 2fe28eb
- [x] 2.2 Full pre-commit gate passes — 2fe28eb
- [x] 2.3 Release build does not contain the fake — 2fe28eb

#### Manual

- [ ] 2.4 With VPN on, the smoke test prints `data` for `glab:version`, `gh:version` and `gh:auth-status`, and `data` or a clear `LoggedOut` for `glab:auth-status`
- [ ] 2.5 With VPN off, the smoke test prints `Unavailable` for `glab:auth-status` within about 20 s, and `data` for `glab:version`
- [ ] 2.6 `npm run tauri dev` still starts the app
