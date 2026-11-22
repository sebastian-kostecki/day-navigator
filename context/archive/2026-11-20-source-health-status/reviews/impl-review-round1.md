<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Source Health Status Implementation Plan

- **Plan**: context/changes/source-health-status/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-21
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

## Automated verification (re-run 2026-11-21)

| Command | Result |
|---------|--------|
| `cargo test` (src-tauri) | PASS: 135 passed, 1 ignored |
| `cargo build` (src-tauri) | PASS |
| `cargo audit` (src-tauri) | PASS: the same 6 allowed warnings as at 41be777, none new |
| `npm run build` | PASS |
| `grep -rn "tauri" src-tauri/src/health/` | PASS: only the two module-doc lines, no `use tauri` |
| `grep -rn "greet" src src-tauri/src` | PASS: no matches |

Data boundary: no log line, tooltip or toast carries an absolute path, an exclusion entry or `reason` text. Every planned contract item is present; nothing is MISSING and nothing from "What We're NOT Doing" was done.

## Findings

### F1 — Watchdog race: false "stalled" toast after a slow round

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/runtime.rs:261
- **Detail**: `check_round` clears `stale` at line 261, writes the snapshot, then makes the blocking tray, toast and emit calls, and only stamps the heartbeat at line 286. If the round itself took more than 75 s (the hung-drive case the watchdog exists for), a watchdog tick inside the 261–286 window sees an old heartbeat and `was_stale == false`. It overwrites the fresh snapshot with `NotRunning`, logs "check loop stalled" again and sends a second stall toast. Tray and emit calls from the two threads also race outside the lock, so the tray and the stored snapshot can disagree until the next tick about 15 s later. Because the check thread always clears `stale` first, the watchdog's "resumed" branch (313–323) is effectively dead: the log records "stalled" but never "resumed". The plan (plan.md:298) gives clearing to the watchdog.
- **Fix A ⭐ Recommended**: Stamp the heartbeat right after `probe_round` returns, and make the watchdog the only owner of `stale` (the check thread computes `overall` from the current flag and never resets it).
  - Strength: Closes the window with a small diff in one file; the "resumed" log line comes back as the plan describes.
  - Tradeoff: The tray stays grey for up to 15 s after a slow round finishes, until the next watchdog tick clears it.
  - Confidence: HIGH — the race is visible by reading lines 261–312; the fix removes the second writer of `stale`.
  - Blind spot: No automated test covers the threaded runtime, so a regression would go unnoticed.
- **Fix B**: Do Fix A and also move the watchdog step into a pure `health::watchdog_step(prev_stale, age, gap) -> Action` with unit tests for the stalled, resumed and no-op cases.
  - Strength: Fits the plan's "every decision as a pure function" design and makes the stall state machine testable without Tauri.
  - Tradeoff: More code and tests in the `health/` module for a personal app.
  - Confidence: MED — straightforward, but the API shape still has to be decided.
  - Blind spot: None significant.
- **Decision**: FIXED (Fix A): heartbeat stamped right after the probe; the watchdog is the only owner of `stale`; check_round reads the flag under the snapshot lock. runtime.rs

### F2 — Resume from sleep may still raise a false stall on the second tick (unconfirmed)

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/health/mod.rs:249 (is_stale), src-tauri/src/runtime.rs:293-300
- **Detail**: `is_stale` skips only the one tick whose own gap was long. The next tick, 15 s later, has a normal gap, but `heartbeat_age` still includes the time spent asleep. If `Instant` (QPC) counts suspended time and the check thread's `recv_timeout` does not, the check thread may not have finished a round by then, and a false "sprawdzanie stanu nie działa" toast fires. The Windows timer semantics were not verified, and no manual check in the plan covers sleep/resume.
- **Fix**: After a long watchdog gap, wake the check thread and add a grace period: mark the loop stale only after two consecutive normal-gap ticks with an old heartbeat, or measure age from `max(last_heartbeat, resumed_at)`.
  - Strength: Covers the resume case whatever the OS timer semantics are; the change stays inside `is_stale` and the watchdog loop.
  - Tradeoff: A real stall is reported up to 15 s later (about 90 s instead of 75 s), still within the plan's "about 90 s" manual check.
  - Confidence: MED — the logic gap is clear, but whether it happens on this machine is unverified.
  - Blind spot: Windows `Sleep`/`QueryPerformanceCounter` behaviour across hibernate and modern standby.
- **Decision**: FIXED: new pure `health::stall_age` (+ test `second_tick_after_resume_is_not_stale`); on a long watchdog gap the watchdog records the resume and wakes the check thread; clearing a stall still uses the raw heartbeat age. Sleep/resume still to confirm by hand.

### F3 — useHealth: an event can be lost, and a listen failure can be masked

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src/health/useHealth.ts:29-50
- **Detail**: The plan says "subscribe with `listen` first, then invoke `health_status`". The code starts both IPC calls at once without awaiting `listen`, so an event emitted before the listener is registered is lost; the panel then shows an older snapshot for up to 30 s. A `listen` rejection sets `loadError`, but a later successful invoke clears it (`setLoadError(null)` at line 45), so the panel shows a snapshot that never updates, with no error. That goes against the rule never to hide a failed source. The `ErrorState` retry button (HealthPanel.tsx:30) calls `checkNow`, which never retries the failed `listen`/`invoke`.
- **Fix**: Invoke `health_status` inside `listen(...).then(...)`; keep a listen failure in its own sticky error; make the retry re-run subscribe and load instead of `check_health_now`.
  - Strength: Restores the planned order and closes the silent-stale path.
  - Tradeoff: Startup takes one extra IPC round-trip before the first snapshot appears; the hook gets a little more complex.
  - Confidence: HIGH — both issues follow directly from lines 29–50.
  - Blind spot: How often `listen` really fails in Tauri 2 is unknown (probably close to never).
- **Decision**: FIXED: invoke runs only after listen resolves; a failed subscribe never loads; new `retry` re-runs subscribe+load and is the ErrorState retry.

### F4 — Empty source list rendered when the loop stalls before the first check

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/health/HealthPanel.tsx:36
- **Detail**: A stall before any check yields `{overall: "notRunning", sources: [], checkedAtMs: null}`. `starting` is false, so the panel renders the "Sprawdzanie stanu nie działa." notice above an empty `<ul>`. The failure is visible, but next to an empty list rather than in place of it.
- **Fix**: Render `ErrorState` (or no list) when `overall === "notRunning"` and `checkedAtMs` is null.
- **Decision**: FIXED: HealthPanel renders ErrorState "Sprawdzanie stanu nie działa" when notRunning with no finished check.

### F5 — Transition log prints "ok" for Unknown and NotConfigured

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/runtime.rs:281
- **Detail**: `from_kind.unwrap_or("ok")`: `from_kind` is `None` for Unknown and NotConfigured too, so the first check logs `health: vault:work ok -> RootMissing`, which is misleading when reading the log.
- **Fix**: Log `None` in the from position as `-` (the to position can stay `ok`, since a recovery always ends in Ok).
- **Decision**: FIXED: missing from-kind logs as `-`.

### F6 — `core:default` lets the webview drive the health tray

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/capabilities/default.json:7
- **Detail**: `core:default` includes `core:tray:default` (tauri-2.12.7 build.rs:213), which grants `set_icon`/`set_tooltip`/`set_visible` to the webview. Script running in the webview could fake the health status, the app's reliable channel. This capability predates the change, but the health tray makes it matter. There is no XSS sink today (React escapes every message).
- **Fix**: Replace `core:default` with the explicit core defaults it needs, leaving out `core:tray:default`.
- **Decision**: FIXED: capabilities/default.json lists the core:default members explicitly, without core:tray:default.

### F7 — Config-dir fallback built by hand twice

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/runtime.rs:211-228, src-tauri/src/lib.rs:66-68
- **Detail**: Both places build `VaultError::SettingsInvalid { reason: "cannot resolve app config dir" }` by hand. `probe_round` also builds its `SourceStatus` itself instead of going through `probe`/`message_pl`, so its panel text differs from every other settings error. This Polish decision logic sits in the Tauri layer instead of in `health/`.
- **Fix**: Add a pure `health::probe::unresolved_config_dir()` returning the `ProbeResult`, and call it from both places.
- **Decision**: FIXED: `health::probe::unresolved_config_dir()` (+ unit test) used by lib.rs setup and runtime::probe_round; reason text is now Polish and goes through message_pl.

### F8 — Demo leftovers outside the explicit removal list

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: index.html:2-7, src/assets/react.svg, public/vite.svg, public/tauri.svg
- **Detail**: The plan's intent is "demo removed, all UI text Polish". `index.html` still has `lang="en"`, the `vite.svg` favicon and `<title>Tauri + React + Typescript</title>`; `react.svg` and `tauri.svg` are no longer referenced.
- **Fix**: Set `lang="pl"` and `<title>Nawigator dnia</title>`, drop the favicon link, and delete the three unused SVGs.
- **Decision**: FIXED: index.html lang="pl", title "Nawigator dnia", favicon link dropped; react.svg, vite.svg and tauri.svg deleted.

### F9 — All 15 manual checks pending while change is "implemented"

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/source-health-status/plan.md:474-509
- **Detail**: Every Automated row is `[x]` and re-verified. Every Manual row (1.5, 2.5–2.12, 3.5–3.7) is still `[ ]`, so none is falsely ticked, but the epilogue commit 7cc0e6e closed the plan with them open. The core end-to-end proof (rename the vault folder, red icon and toast within 60 s, recovery) has not been recorded. F1 and F2 also affect 2.9.
- **Fix**: Run the manual checks (after the F1/F2 decision) and tick them before `/10x-archive`.
- **Decision**: SKIPPED: manual GUI checks need a human (tray, toasts, renaming the vault folder); run them before /10x-archive.

## Triage summary (2026-11-21)

- Fixed: F1 (Fix A), F2, F3, F4, F5, F6, F7, F8 (8)
- Skipped: F9 (manual checks need a human) (1)
- After the fixes: `cargo build` clean, `cargo test` 136 passed / 1 ignored, `npm run build` passes, both grep checks clean.
