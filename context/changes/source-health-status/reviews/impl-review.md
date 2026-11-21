<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Source Health Status Implementation Plan (round 2)

- **Plan**: context/changes/source-health-status/plan.md
- **Scope**: Full plan (committed b70ebf2..7cc0e6e plus the uncommitted round-1 triage fixes in the working tree)
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-21
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 3 warnings, 4 observations
- **Previous round**: reviews/impl-review-round1.md (F1–F8 fixed, F9 skipped)

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | WARNING |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | WARNING |

## Automated verification (re-run 2026-11-21, working tree)

| Command | Result |
|---------|--------|
| `cargo build` (src-tauri) | PASS |
| `cargo test` (src-tauri) | PASS: 136 passed, 1 ignored |
| `cargo audit` (src-tauri) | PASS: 6 allowed warnings, none new |
| `npm run build` | PASS |
| `grep -rn "tauri" src-tauri/src/health/` | PASS: only the two module-doc lines |
| `grep -rn "greet" src src-tauri/src` | PASS: no matches |

Plan drift: every contract item is present, there are 0 MISSING and every planned test case exists. Nothing from "What We're NOT Doing" was done. The round-1 fixes hold: no lock is held across tray, toast or emit calls, `stale` has a single writer, `useHealth` subscribes before it loads, and no empty list is rendered in place of a failure. The data boundary also holds: log lines, tooltips and toasts carry only source keys, fixed labels and error kinds.

## Findings

### F1 — A suspend of about 45–60 s can still raise a false stall

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/runtime.rs:293 (`if gap >= STALE_AFTER`), src-tauri/src/health/mod.rs:249
- **Detail**: The watchdog treats a wake-up as a resume only when its own gap is at least 75 s. The heartbeat can be up to about 30 s older than the watchdog's last wake, though. Example: the check thread stamps at t=0, the watchdog ticks at t=12, and the machine suspends at t=25 for 52 s. The watchdog then wakes with gap = 65 s (< 75, so no resume is recorded) while the heartbeat age is 77 s (> 75). If it runs before the check thread's post-resume probe finishes, the tray goes grey, "check loop stalled" is logged and the stall toast fires; everything clears 15 s later. This happens only if `Instant` (QPC) counts suspended time, which the code comments already treat as possible. Short suspends are common under Modern Standby. The plan's Critical Implementation Details require that a resume is never reported as a stall. The watchdog's set/clear state machine still has no tests (only `stall_age` and `is_stale` do), and that is how this gap slipped through again after round-1 F2.
- **Fix A**: Treat any watchdog gap above `2 * WATCHDOG_INTERVAL` (30 s) as a resume, and update the `is_stale` doc to name that threshold.
  - Strength: A one-line change in runtime.rs. With gap ≤ 30 s and no suspend, the heartbeat age stays at most about 60 s plus probe time, so it cannot cross 75 s.
  - Tradeoff: Leaves the watchdog loop untested, and this is its second resume bug.
  - Confidence: HIGH — the timeline follows directly from lines 288–302.
  - Blind spot: Windows QPC and `Sleep` behaviour across Modern Standby is still unverified on this machine.
- **Fix B ⭐ Recommended**: Do Fix A inside a new pure `health::watchdog_step(was_stale, age, gap, since_resume) -> Action` (Stall / Resume / Nothing), with table tests for the 45–60 s suspend, a long sleep, a real stall and clearing. runtime.rs then only carries out the returned action.
  - Strength: Fits the plan's "every decision as a pure function" approach. It gives the state machine that broke twice its first tests and keeps the raw-age vs `stall_age` asymmetry in one tested place.
  - Tradeoff: About 40 more lines in `health/` plus tests.
  - Confidence: MED — straightforward, though the shape of `Action` still has to be chosen.
  - Blind spot: None significant.
- **Decision**: FIXED (Fix B): pure `health::is_resume` (gap > 30 s) and `health::watchdog_step` -> `WatchdogAction`, with `watchdog_step_table` covering the 52 s suspend, long sleep, real stall, grace expiry and clearing; runtime.rs only carries out the action.

### F2 — The UI orders snapshots by wall-clock time, so a backward clock change freezes the panel

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src/health/useHealth.ts:23-29, src-tauri/src/runtime.rs:111-116, 260
- **Detail**: `checked_at_ms` comes from `SystemTime::now()`, and `accept` drops any snapshot whose `checkedAtMs` is lower than the one already shown. If the Windows clock moves back by more than one check interval (a manual change, or a large time-sync correction after hibernate), every later event and every `health_status` result is rejected until the clock catches up. A vault that has just gone missing then stays shown as OK, and "Sprawdź teraz" appears to do nothing. The tray is unaffected. The plan itself prescribed ordering by `checkedAtMs` (plan.md:387), so this is a flaw in the plan, carried faithfully into the code.
- **Fix**: Add a `seq: u64` to `HealthSnapshot`, bumped under the snapshot write lock in `check_round` and in both watchdog branches, and order by `seq` in `accept`. Keep `checkedAtMs` for display only.
  - Strength: Monotonic by construction. It also removes the strict/non-strict split in `accept`, since watchdog-only changes get a new `seq`. F4 can reuse it.
  - Tradeoff: Touches the serialised contract (Rust type, types.ts, tests) and the plan's Phase 3 contract.
  - Confidence: HIGH — the rejection follows directly from `a >= b` on wall-clock values.
  - Blind spot: How often this machine's clock really jumps back by more than 30 s is unknown; it is probably rare.
- **Decision**: FIXED: `HealthSnapshot.seq` bumped under the write lock (check round and both watchdog edges); `useHealth` orders by `seq` only; `checkedAtMs` is display-only.

### F3 — The plan does not record the round-1 deviations

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence / Scope Discipline
- **Location**: context/changes/source-health-status/plan.md:281-298, 372-375, 387-390
- **Detail**: The round-1 fixes changed several things the plan states explicitly:
  - the heartbeat is stamped right after the probe, not at step 8;
  - the check round no longer clears `stale`, because the watchdog owns it, so recovery from a stall can lag by up to 15 s;
  - the new `stall_age` rule, plus a wake-up of the check thread after a resume;
  - `capabilities/default.json` was changed, which contradicts "No capability changes";
  - `health_status` returns `Result<HealthSnapshot, String>`, as Tauri requires for async commands that borrow `State`;
  - the extra pieces `unresolved_config_dir`, `useHealth().retry`, the stall-before-first-check `ErrorState`, and the stall notice as a plain `<p>` (the plan calls `ErrorState` "the only error rendering").

  All of these are reasonable, but future reviews and `/10x-archive` will read the plan as ground truth.
- **Fix**: Add a short "Addendum (impl review)" section to plan.md that lists these deviations and their reasons.
- **Decision**: FIXED: plan.md has an "Addendum (implementation review, 2026-11-21)" listing every round-1 and round-2 deviation.

### F4 — Tray and event publishing are not ordered between the two threads

- **Severity**: 💬 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/runtime.rs:252-279, 306-324
- **Detail**: The stored snapshot is consistent, but `update_tray` and `emit` run outside any lock in both threads. Example at the end of a stall:
  1. The check thread reads `stale=true` and stores C (NotRunning).
  2. The watchdog then clears the flag and stores W (Error/Ok).
  3. If the check thread is descheduled, C's `set_icon` and emit can be dispatched after W's.

  The tray then stays grey, and the panel shows "notRunning" (equal `checkedAtMs`, last event wins) until the next round, about 30 s later. A similar edge at stall time gives a stall toast when a probe that took over 75 s finishes exactly on the tick. The comment at line 254 claims consistency only for the stored value.
- **Fix**: Add a `publish: Mutex<u64>` that no main-thread code takes. Inside it, re-read the snapshot and push tray plus emit only when its `seq` (F2) is newer than the last one published.
  - Strength: Tray, panel and snapshot always converge on the latest state. It cannot deadlock, because the main thread never takes this mutex.
  - Tradeoff: Depends on F2's `seq`. It holds a mutex across calls that block on the main thread, which the locking rule allows only because no main-thread code takes it.
  - Confidence: MED — the interleaving is real, but the window is narrow and self-heals within 30 s.
  - Blind spot: Not reproduced; Tauri's main-thread dispatch order for two `set_icon` calls is assumed FIFO.
- **Decision**: FIXED: `runtime::publish` under `published: Mutex<u64>` (taken only by the two health threads) re-reads the stored snapshot and pushes tray + emit only for a newer `seq`.

### F5 — A failed "Sprawdź teraz" is shown as a load failure

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/health/useHealth.ts:64-70, src/health/HealthPanel.tsx:23-33
- **Detail**: A `check_health_now` error sets `loadError`, so the whole panel becomes "Nie można odczytać stanu źródeł". Its retry calls `retry` (re-subscribe and reload), not a new check, and the next successful load or event silently clears the error. It can only happen when the check thread is gone, and the watchdog shows NotRunning in that case anyway.
- **Fix**: Keep a separate `checkError` in `useHealth` and show it inline next to the "Sprawdź teraz" button.
- **Decision**: FIXED: separate `checkError` in `useHealth`, shown as an inline ErrorState under "Sprawdź teraz" (retry = check again); load errors unchanged.

### F6 — The window's capabilities are still wider than the UI needs, and CSP is off

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/capabilities/default.json:7-14, src-tauri/tauri.conf.json (`"csp": null`, which predates this change)
- **Detail**: Dropping `core:tray:default` was right. The UI only invokes its own two commands and listens to one event, but it still gets the `app`, `image`, `menu`, `path`, `resources`, `webview` and `window` defaults. `core:event:default` also lets the webview emit a forged `health-changed` to itself. Nothing is exploitable today: React escapes every message, and no `log:*` or `notification:*` permission is granted.
- **Fix**: Narrow to `core:event:allow-listen` and `core:event:allow-unlisten` (plus what later slices need), and set a restrictive CSP. This could also be deferred to the first slice that adds UI surface.
- **Decision**: PARTLY FIXED: capabilities narrowed to `core:event:allow-listen`, `core:event:allow-unlisten` and `opener:default`. CSP deferred to follow-ups/review-fixes.md, because `csp` also applies under `tauri dev` and a wrong policy gives a blank window, which cannot be checked without the GUI.

### F7 — Round-1 fixes uncommitted; all 15 manual checks still pending

- **Severity**: 💬 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: working tree (git status); context/changes/source-health-status/plan.md:474-509
- **Detail**: The round-1 triage fixes are uncommitted: runtime.rs, health/mod.rs, probe.rs, lib.rs, default.json, useHealth.ts, HealthPanel.tsx, index.html, and the three SVG deletions. They are mixed with unrelated edits to `context/foundation/roadmap.md` (F-02 in-progress, S-05/S-06 ready). Every Manual row (1.5, 2.5–2.12, 3.5–3.7) is still `[ ]`; none is falsely ticked. The core end-to-end proof (rename the vault folder: red icon and toast within 60 s, then recovery) is still unrecorded. Manual check 2.9 depends on F1.
- **Fix**: Commit the review fixes (keep the roadmap edit separate), then run the manual checks and tick them before `/10x-archive`.
- **Decision**: PARTLY FIXED: review fixes committed (roadmap edit kept separate, uncommitted). The manual checks still need a human; listed in follow-ups/review-fixes.md.

## Triage summary (2026-11-21)

The user chose to have every finding fixed without walking through them one by one.

- Fixed: F1 (Fix B), F2, F3, F4, F5 (5)
- Partly fixed: F6 (CSP deferred), F7 (manual checks need a human) (2)
- After the fixes: `cargo build` clean, `cargo test` 137 passed / 1 ignored, `npm run build` passes, and no new clippy warnings in changed files.
