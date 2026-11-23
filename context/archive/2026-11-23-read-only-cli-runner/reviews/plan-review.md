<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Read-only CLI runner Implementation Plan

- **Plan**: context/changes/read-only-cli-runner/plan.md
- **Mode**: Deep
- **Date**: 2026-11-23
- **Verdict**: REVISE (SOUND after fixes)
- **Findings**: 1 critical, 2 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | WARNING |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | FAIL |
| Plan Completeness | WARNING |

## Grounding
6/6 paths ✓, 5/5 symbols ✓, brief↔plan ✓, roadmap ✗ (the new-MVP roadmap is not committed)

Confirmed by verification (tauri-cli 2.12.6 source, gh 2.105 / glab 1.124 source, a scratch crate, Rust std 1.100):
- `tauri build` runs `cargo build --bins`, leaves out bins whose required features are off, and keeps the fake out of the NSIS installer.
- `tauri dev` runs a plain `cargo run`, and `default-run` is required.
- In the scratch crate, `cargo build`, `cargo build --release`, `cargo test` and `cargo clippy --all-targets -D warnings` all pass, and no second `fake_cli` bin is inferred.
- On Windows, a `.cmd` shim gives NotFound (`ToolMissing`), and `kill` stops only the direct child.

## Findings

### F1 — Logged-out check runs before network check

- **Severity**: ❌ CRITICAL
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Critical Implementation Details › Classification order; Phase 1 §2 Classifier
- **Detail**: Both CLIs print login hints on network failures. glab 1.124 `auth status` adds "run glab auth login --hostname …" whenever the token is stored in plain text, even on success (status.go:251-253). gh 2.105 `auth status` reports a refused connection or DNS failure as "The token … is invalid … run: gh auth login" (status.go:505-515, 142-149). Checking logged-out first would turn "VPN off" into LoggedOut, the alarm FR-015 forbids.
- **Fix**: Order the checks as: exit 0 → gh exit 4 → network/5xx → logged out → Failed. Add `proxyconnect`, and add two mixed table rows.
  - Strength: A transport failure means auth was never checked; confirmed in both CLIs' sources.
  - Tradeoff: A 401 whose message also has network words would show as calm. No such shape was observed.
  - Confidence: HIGH — confirmed in source and by a live gh run.
  - Blind spot: The glab case was checked in source only.
- **Decision**: FIXED

### F2 — Release check tests cargo, not the installer

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2 Cargo wiring; Success criterion 2.3
- **Detail**: CI builds with `tauri build`. Tauri's `get_binaries` drops the `src/bin` file by path only, because `fake-cli` ≠ `fake_cli`. NSIS copies every listed non-main binary into the installer. The plan never stated that the safety depends on an exact `path` and on `test-support` never being added to `default`.
- **Fix**: Document both rules in §2 and in a Cargo.toml comment, and label 2.3 a proxy check.
- **Decision**: FIXED

### F3 — Plan cites a roadmap the worktree doesn't have

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Overview; References
- **Detail**: The new-MVP roadmap exists only uncommitted in the main checkout. The committed roadmap on main and in this branch reuses F-02, S-01 and S-05 for other, older items.
- **Fix**: Commit the new roadmap to main and merge it here before `/10x-implement`. The plan now records this as a prerequisite in References.
- **Decision**: FIXED (the plan note is in; committing the roadmap is still the user's step)

### F4 — Polish "unavailable" text omits server outage

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: End-State Alignment
- **Location**: Phase 1 §1 `message_pl`
- **Detail**: HTTP 5xx and the timeout also map to Unavailable, but the text sent the user only to check the VPN or the network.
- **Fix**: Change to "… (brak VPN, sieci lub serwer nie odpowiada)" and the same for GitHub.
- **Decision**: FIXED

### F5 — Exhaustiveness test mechanism is not specified

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §3 "Exhaustiveness"
- **Detail**: An exhaustive `match` alone does not force a matching edit to `ALL`.
- **Fix**: Use an `ordinal()` helper with an exhaustive match, plus `VARIANT_COUNT`; the test asserts that `ALL` maps one-to-one onto `0..VARIANT_COUNT`.
- **Decision**: FIXED
