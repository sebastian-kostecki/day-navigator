# Read-only CLI runner — Plan Brief

> Full plan: `context/changes/read-only-cli-runner/plan.md`

## What & Why

Roadmap foundation F-02. The app needs one safe way to run the logged-in `glab` and `gh` before any GitLab or GitHub view exists. That way is read-only, uses no shell and fixed arguments, and never lets work data reach logs or tests. Each call ends in one of five states (data, unavailable, logged out, tool missing, error), so later views can say "GitLab unavailable" instead of showing an empty list. The rule and the tests are built separately from the first view, so the security contract is not mixed with UI work.

## Starting Point

Nothing in the app starts a process yet. The health module already shows the pattern to follow: pure rules without `tauri`, side effects kept apart, and error enums tagged by `kind`. Both CLIs are installed on Windows (`glab` 1.124, `gh` 2.105, and `gh` is logged in). `glab` exits 1 for every failure, so telling the states apart means reading its stderr. That stderr contains the full URL with the hostname.

## Desired End State

`day_navigator_lib::cli` has a closed list of allowed calls (version and `auth status` for each tool) and a blocking `CliRunner::system().run(call)` that returns `Ok(stdout)` or a `CliError` (`Unavailable`, `LoggedOut`, `ToolMissing`, `Failed { exitCode }`). The error carries no stderr text. A test-only `fake-cli` binary produces every state through a real process, so S-01 and S-05 can test their views without real work data.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Timeout | 20 s, then kill, reported as `Unavailable` | With the VPN off, `glab` often ends in a timeout, and that must stay calm (FR-015). |
| HTTP 5xx with VPN on | `Unavailable` (calm) | A server outage is not the user's problem to fix, so no alarm; the view still says it plainly. |
| What an error carries | Kind, tool and exit code only; stderr is dropped after classification | stderr holds the work hostname, so nothing that could leak is kept. |
| Read-only enforcement | Closed `CliCall` enum with fixed `&'static` arguments, plus a guard test | A write call cannot even be expressed, and every new call is reviewed in one file. |
| Test double | A real `fake-cli` bin behind `test-support` | Also covers the risky process parts (timeout, kill, missing program), and stays out of release builds (checked). |
| Tray scope | Not in F-02; S-01/S-05 wire it in | Matches the roadmap; F-02 stays a narrow security contract. |
| `gh` on Windows | `gh.exe` from `PATH`, no WSL | It is installed and logged in on Windows now; one code path for both tools. |

## Scope

**In scope:**
- The `cli` module: `Tool`, `CliCall`, `CliError` with fixed Polish texts, and a pure `classify()`
- A runner with a time limit, kill, `CREATE_NO_WINDOW`, fixed no-prompt and no-update environment variables, and output caps
- The `fake-cli` bin, integration tests for every state, and an ignored smoke test against the real CLIs
- One `AGENTS.md` line: CLI calls live only in `src-tauri/src/cli/`

**Out of scope:**
- Issue or MR queries (S-01, S-05), tray sources and a calm state in `SourceState` (S-01)
- Tauri commands and the TS mirror of `CliError` (the first slice that returns it adds them)
- WSL, a program-path setting, arguments from settings, retries
- An automated check for console-window flashes (first visible in S-01)

## Architecture / Approach

`CliCall` (closed list, fixed arguments) → `CliRunner::run`, which spawns the program without a shell, with stdin null, `CREATE_NO_WINDOW` and the fixed environment; drains stdout and stderr on threads; and kills the process at 20 s → `classify(tool, exit code, stderr)` → `Ok(stdout)` or `CliError`. stderr is dropped at that point. Logs record only the call name, the outcome kind and the time. Tests swap the program path for `fake-cli`, whose behaviour is chosen with `FAKE_CLI_SCENARIO`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Call contract and classification | Call list, error type, pure classifier, read-only guard and unit tests | Matching CLI text, which future CLI releases may reword |
| 2. Process runner and fake CLI | Runner with timeout, kill and caps; `fake-cli`; integration and smoke tests; `AGENTS.md` line | Pipe draining and kill order on Windows; `default-run` for `tauri dev` |

**Prerequisites:** none (the roadmap lists none); `glab` and `gh` installed on Windows for the manual smoke test.
**Estimated effort:** ~1–2 sessions across 2 phases.

## Open Risks & Assumptions

- CLI error text can change between releases. Unknown text falls back to `Failed` (loud, not silently calm), and the fix is one new pattern plus a table row.
- `glab auth status` checks every configured host and may exit 1 even with VPN on. The smoke test shows this, and S-01 chooses its own call anyway.
- Open Roadmap Question #1 (`gh` only in WSL) looks resolved by `gh.exe` on Windows. Updating the roadmap text is left to `/10x-roadmap`.

## Success Criteria (Summary)

- `cargo test` drives all five states through a real process on invented data, and no error representation contains the test hostname.
- With the VPN off, the smoke test reports `glab` as `Unavailable` within about 20 s; with it on, as `data`.
- The release build contains no fake binary, and `npm run tauri dev` still starts the app.
