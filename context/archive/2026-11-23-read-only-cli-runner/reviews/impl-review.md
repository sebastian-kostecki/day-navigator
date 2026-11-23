<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Read-only CLI runner Implementation Plan

- **Plan**: context/changes/read-only-cli-runner/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-23
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 5 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

## Automated verification (run 2026-11-23)

- `cargo fmt --check`: pass
- `cargo clippy --all-targets -- -D warnings`: pass
- `cargo test --lib cli`: 8 passed
- `cargo test --test cli_runner`: 9 passed (7.1 s, hang test)
- `cargo test` (full): all suites pass (lib 107, cli_runner 9, health_probe 8, tasks 22, vault_guard 18, vault_write 33; cli_real 1 ignored)
- `npm run lint`, `npm run format:check`, `npm run build`: pass
- `cargo build --release`: only `day-navigator.exe` in `target/release`, no `fake-cli.exe` (pass). Release build has one pre-existing warning outside this change (`src/lib.rs:150`, unused `mut`)

## Findings

### F1 — Reader join: 4 s worst case, and it skips classification

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/cli/runner.rs:155-160
- **Detail**: The plan says to join the readers with a bound of "about 2 s". The code calls `recv_timeout(2 s)` twice, one after the other, so the worst case is about 4 s. When either reader times out, the runner returns `Failed { exit_code: status.code() }` without calling `classify`. That decision is not in the plan. Two consequences: a glab exit 1 with network text while a grandchild holds the pipe gives `Failed`, not the calm `Unavailable`; and exit 0 gives `Failed(Some(0))`, which the user sees as "zakończył się błędem (kod 0)".
- **Fix**: Use one shared deadline (`Instant::now() + READER_JOIN_BOUND`) for both receives. If only stdout is missing on a non-zero exit, still classify with the stderr that arrived. Otherwise return `Failed { exit_code: None }`. Record the choice in the plan as an addendum.
  - Strength: Restores the planned 2 s bound and keeps network failures calm, as FR-015 requires.
  - Tradeoff: A few more branches in `run_inner`. The path cannot be reached with today's calls, and testing it needs a fake scenario that leaves a grandchild running.
  - Confidence: MED — the logic is clear, but the grandchild case is hard to reproduce on Windows.
  - Blind spot: Whether glab or gh ever leave a helper process holding the pipes in practice.
- **Decision**: FIXED — one shared 2 s join deadline; stderr classified even when stdout never finished; plan addendum; tests for both lingering-grandchild paths

### F2 — The child inherits the app's working directory

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/runner.rs:105
- **Detail**: `Command` never sets `current_dir`, so glab/gh run in the app's working directory: `src-tauri/` of this repo under `tauri dev`, or the launch directory for the installed app. The current `version` and `auth status` calls do not depend on it. The repo-scoped calls planned for S-01 and S-05 (`issue list`, `mr list`) infer the repo and host from the git checkout in the cwd. That would give the "fixed argument list" contract a hidden input the caller does not control.
- **Fix A ⭐ Recommended**: Set a fixed, neutral `current_dir` (`std::env::temp_dir()`) in `run_inner` now, and note in the module doc that future calls pass `--repo` or `-R` explicitly.
  - Strength: A one-line change; the contract stays truly fixed before any slice relies on it.
  - Tradeoff: Practically none. The temp dir always exists, and the app only reads from it.
  - Confidence: HIGH — std behaviour is well defined.
  - Blind spot: Whether glab reads any config relative to the cwd. Not checked, but its config lives under the user profile.
- **Fix B**: Leave the runner as it is and add a requirement to S-01/S-05: every repo-scoped `CliCall` carries `--repo` or `-R`.
  - Strength: No code change now.
  - Tradeoff: The guarantee depends on each future slice remembering it.
  - Confidence: MED — it depends on discipline in later slices.
  - Blind spot: CLI commands that fall back to the cwd even when `-R` is given.
- **Decision**: FIXED via Fix A — current_dir(temp_dir()), module doc and plan addendum; test runs_in_the_neutral_temp_dir_not_the_app_dir

### F3 — `thread::spawn` panics, and release builds abort on panic

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/runner.rs:175
- **Detail**: `spawn_reader` uses `std::thread::spawn`, which panics if the OS refuses a thread. The release profile has `panic = "abort"` (Cargo.toml:74), so the whole app would close. In a test build, a failure on the second reader leaves a running child with one pipe that nothing drains. The existing pattern in `runtime.rs:176-188` uses `thread::Builder::new().name(..).spawn(..)` and handles the error.
- **Fix**: Switch to `thread::Builder` with the names `cli-stdout` and `cli-stderr`. If spawning fails, kill and wait the child and return `Failed { exit_code: None }`.
- **Decision**: FIXED — thread::Builder (cli-stdout / cli-stderr); a refused thread kills and reaps the child and returns Failed { exit_code: None }

### F4 — On Windows, `kill` does not stop grandchildren, and reader threads are left detached

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/cli/runner.rs:141-151
- **Detail**: On a timeout or a `try_wait` error (the latter path is not in the plan), the child is killed with `TerminateProcess`, which stops only the direct process. The reader threads are left detached; each can hold up to 16 MiB until a grandchild closes the inherited pipes. Today's calls do not start any helpers, so the risk is theoretical.
- **Fix**: Record this under Open Risks in the plan. Add a Job Object with `KILL_ON_JOB_CLOSE` only when a future call can start helper processes.
- **Decision**: FIXED — recorded under Open Risks in the plan; stop() doc points there

### F5 — `test-support` feature comment is out of date

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/Cargo.toml:56-57
- **Detail**: The comment says the feature exposes only the vault write layer. It now also gates `CliRunner::with_programs` and the `fake-cli` bin.
- **Fix**: Extend the comment: "…and `CliRunner::with_programs` plus the `fake-cli` bin (CLI tests)".
- **Decision**: FIXED — comment now lists CliRunner::with_programs and the fake-cli bin

### F6 — Real hostname `githubstatus.com` in the test samples

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/bin/fake_cli.rs:8, src-tauri/src/cli/classify.rs:140
- **Detail**: The plan says stderr samples use only `gitlab.example.invalid` and `github.example.invalid`. The gh sample copies the real `https://githubstatus.com` from the gh message. It is public, not work data, but it breaks the "invented data only" rule.
- **Fix**: Replace it with `https://status.example.invalid` in both places.
- **Decision**: FIXED — status.example.invalid in fake_cli.rs and classify.rs

### F7 — Manual checks pending while status is `implemented`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/read-only-cli-runner/plan.md:287-301
- **Detail**: Rows 1.4 and 2.4–2.6 are still `[ ]`: the review of the argument lists and Polish texts, the smoke test with VPN on and off, and `npm run tauri dev`. No row is ticked without evidence; they are honestly pending. They do have to be done before archiving, especially 2.5 (`glab:auth-status -> Unavailable` without VPN) and 2.6 (`default-run`).
- **Fix**: Run `cargo test --test cli_real -- --ignored --nocapture` with the VPN on and off, then `npm run tauri dev`, and tick the rows.
- **Decision**: PENDING — manual checks (VPN on/off smoke test, npm run tauri dev) are left to the user

### F8 — Minor polish in the test API and Display

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/cli/mod.rs:93, src-tauri/src/cli/runner.rs:69-77
- **Detail**: `Failed` displays as `exit code Some(2)` because Display uses Debug formatting. `with_programs` does not have the `#[doc(hidden)]` that the vault's `apply_for_tests` has, and it converts paths with `to_string_lossy`, which mangles non-UTF-8 paths (test-only).
- **Fix**: Format the exit code as `2` or `none` in Display, add `#[doc(hidden)]` to `with_programs`, and store the programs as `OsString`.
- **Decision**: FIXED — Display shows exit code N / no exit code (unit test), #[doc(hidden)], programs stored as OsString

## Triage summary (2026-11-23)

- Fixed: F1, F2 (Fix A), F3, F4 (plan Open Risks), F5, F6, F8 (7)
- Pending: F7, the manual checks the user runs: smoke test with VPN on and off, and `npm run tauri dev` (1)
