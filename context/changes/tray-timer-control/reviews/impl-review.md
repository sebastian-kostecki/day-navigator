<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: START/STOP from the tray and today's work time in the tray tooltip

- **Plan**: `context/changes/tray-timer-control/plan.md`
- **Scope**: Full plan (base `455cbed`, phases p1–p5 at 211a452, bb8449e, 1aab6c9, b171395, 1a2ebb4)
- **Reviewed phases**: 1, 2, 3, 4, 5
- **Date**: 2026-11-26
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | WARNING |

## Must fix before archive

No code defect must be fixed. The one blocking item is the human check: the Manual rows 4.7–4.11 and 5.7–5.12 are open (F1). 4.7 decides whether `LINE_SEPARATOR` stays `"\n"`.

## Evidence

The review ran inline, without sub-agents: the diff has 6 code files and about 800 lines.

Diff vs plan (`git diff --stat 455cbed..HEAD`). Every planned file changed. No unplanned code file changed. No frontend, TS, capability, CSP or `tauri.conf.json` change (`git diff --quiet 455cbed..HEAD -- src src-tauri/capabilities src-tauri/tauri.conf.json` succeeds).

| File | Verdict | Note |
|------|---------|------|
| `src-tauri/src/work_time/today.rs` | MATCH | One `store.read`, sums `portion` over `in_day`, `running` from `running_entry` (any day); no `tauri`, no clock |
| `src-tauri/src/work_time/today_tests.rs` | MATCH | All 7 fixtures plus a stop case. Each fixture also checks the header formula over `status()`. Invented instants in `Europe/Warsaw`; unqualified `use super::*` calls |
| `src-tauri/src/work_time/mod.rs` | MATCH | Only `pub mod today;` and `#[cfg(test)] mod today_tests;` |
| `src-tauri/src/health/mod.rs` | MATCH | `TOOLTIP_MAX_UTF16` and `cap_utf16` made `pub`, with a doc; behaviour unchanged |
| `src-tauri/src/tray_view.rs` | MATCH | All contract items: `WorkLine`, `TrayIconKind`, `TimerAction`, `WorkItem`, `format_hm`, `work_line_text`, `LINE_SEPARATOR`, `tooltip`, `failure_notice` (Busy gets the generic body). Exact-text tests, a long-label test, a proptest, the icon and item tables, value-free toasts |
| `src-tauri/src/lib.rs` | MATCH | One line, `pub mod tray_view;`, alphabetical |
| `src-tauri/icons/source/tray-running.svg` | MATCH | Copy of the ok glyph, extra mask circle `824,824 r=248`, amber dot `r=176`, comment as specified |
| `src-tauri/icons/tray/running.png` | MATCH | Recipe re-run into the scratchpad: SHA-256 `f14734c9…6463` equals the committed file |
| `src-tauri/src/work_time/commands.rs` | MATCH | `start_timer` / `stop_timer` are the only `super::start\|stop(` calls (grep gives lines 76 and 88; `work_time::(start\|stop)\(` gives nothing). `start_work` / `stop_work` delegate, then read the status. `notify` wakes the tray |
| `src-tauri/src/runtime.rs` | MATCH | `refresh_tray` via `try_state`. `work: RwLock<WorkLine>` outside the snapshot. The work read sits after `probe_round`, before `stamp_heartbeat` and before the `seq` bump. `Busy` keeps the line; other errors give `Unavailable`, logged by `kind()`. `publish` copies the line under `published`. `update_tray` clears `shown`, runs the setters, then sets `shown`. Menu `work`, separator, then the existing items. The handler only reads `shown` and spawns. `run_tray_action`: failure toast via `failure_notice`, then a refresh |
| `README.md`, `CHANGELOG.md` | MATCH | One sub-bullet; an `### Added` entry under `[Unreleased]` |

What was checked:

- **User decisions.** D1: the tray STOP goes straight to `stop_timer`, with no question. The domain `stop` writes the `workStopped` event row in the same `store.write`, and `events-changed` is emitted. D2: `tray_icon` returns `Error` whenever `overall == Error`, and `Running` only for `Ok` with a running counter (table test). D3: no PRD, FR or roadmap edit. D4: one START and one STOP function, used by the header commands and by `run_tray_action`. Each has a doc comment that names the S-07 hook point.
- **One counter state.** The tray reads the same `work_entries` row as the header, and no second counter exists. The header follows the tray through the existing `work-time-changed` listener. No frontend file changed.
- **Threads and locks.** The main-thread menu callback takes only the `shown` mutex, for a short copy, then calls `tauri::async_runtime::spawn`. It never touches the store, `published` or the snapshot lock. The store runs on the blocking pool (`run` → `blocking`) or on the health thread. The `work` RwLock and the `shown` mutex are never held across a setter, emit or toast: `work_line()` returns a copy, and `set_shown` locks briefly. The only lock held across the tray and menu setters is `published`, which the main thread never takes. The health round (and the watchdog via `publish`) stays the only tray writer: `run_tray_action` only wakes it. A wake sent during a round stays queued and starts the next round.
- **Before the first read and on errors.** The item is built disabled with "Czas pracy: wczytywanie…" and `WorkLine::Loading`. A failed read gives `Unavailable` ("Czas pracy niedostępny") and a stalled check gives `NotRunning`; both disable the item. "0:00" appears only for a successful read of an empty day.
- **Tooltip.** The work line is built first. The health part is capped to `127 − 1 − len(line)` UTF-16 units. A proptest over every `Overall` and `WorkLine`, with emoji and Polish labels, holds ≤ 127 units and the whole work line at the end.
- **AGENTS.md.** No secrets. Tests use invented data only. The domain rules (today's total, icon, item, tooltip, toast texts) are unit-tested in Rust. Logs carry error kinds only. Toast texts carry no ids, times or detail (test with `entry_id: 4242`). Capabilities and CSP are unchanged. The icon follows the SVG source → recipe → 32x32 PNG convention, and the bundle icons are untouched. Edits in shared files are small and local: one line in `lib.rs`, two in `work_time/mod.rs`, two visibility changes in `health/mod.rs`, a README sub-bullet. The `runtime.rs` edits stay inside the tray functions and `check_round`.
- **Extra cost.** Each work-time write now runs one extra health round. `Store::check` is a single `sqlite_master` count, so this is negligible.

## Gates (run by the reviewer)

| Gate | Result |
|------|--------|
| `npm run lint` | pass |
| `npm run format:check` | pass |
| `npm run build` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass (lib 495 passed, all integration suites green, 0 failed) |
| 2.5 `tray_view.rs` has no `tauri` import (grep) | pass (no match) |
| 3.1 / 3.2 icon recipe and 32x32 RGBA | pass (recipe reproduces `running.png` byte for byte) |
| 4.5 one START/STOP path (grep) | pass (`commands.rs:76`, `:88` only) |
| 5.5 menu setters only in `update_tray` (grep) | pass (`runtime.rs:289`, `:292`) |
| 1.5 … 5.6 no frontend, capability or config change | pass |

## Findings

### F1 — Manual checks pending; the Phase 4 pause was skipped

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: `context/changes/tray-timer-control/plan.md:478-482`, `:497-502`
- **Detail**: All Manual rows are open (4.7–4.11, 5.7–5.12), and none is falsely ticked. The Phase 4 note asked for a pause for human confirmation before Phase 5. The run was non-interactive, so Phase 5 went ahead without it. These behaviours have no automated coverage: the line break rendering (4.7, which decides whether `LINE_SEPARATOR` stays `"\n"` or becomes `" · "`), the icon's legibility at 16 px (4.9), the header following a tray STOP and dropping its open question (5.9, 5.10), and CSP/IPC in a built app (5.11). 5.12 (the Konsola artifact) is outside the repo.
- **Fix**: The user runs the Manual rows in `npm run tauri dev` and in a `--debug --no-bundle` build, then ticks them. If 4.7 shows one line, set `tray_view::LINE_SEPARATOR` to `" · "` and fix the expected strings in `tray_view::tests::exact_tooltips`.
- **Decision**: PENDING

### F2 — Every health round re-applies the menu item and briefly disarms it

- **Severity**: 💡 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: `src-tauri/src/runtime.rs:286-296`
- **Detail**: `update_tray` runs on every round: every 30 s, after every work-time write, and on every watchdog publish. Each time it stores `None` in `shown` and makes two blocking main-thread calls (`set_text`, `set_enabled`), even when the item has not changed. A tray click that the main thread handles inside that window finds `shown == None` and does nothing, with no feedback. The window is a few milliseconds every 30 s, so a lost click is rare and the user simply clicks again; it never acts against the shown label. A related edge case: Windows may not repaint a popup menu that is already open, so a label change while the menu is open (only possible on a Loading/Unavailable → Known transition, which goes from disabled to enabled) shows the old text until the menu reopens. The plan accepted the clear-then-set order (A15); this is a refinement, not a defect.
- **Fix** (optional): Keep the last applied `WorkItem` in `TrayWorkItem`, for example `applied: Mutex<Option<WorkItem>>`. In `update_tray`, skip the clear and the setters when the new item equals it.
  - Strength: The disarm window then opens only when START/STOP actually changes, not every 30 s, and the item no longer costs two main-thread round trips per round.
  - Tradeoff: One more field and comparison in a shared file (`runtime.rs`) that S-01/S-06 also touch.
  - Confidence: MED — `WorkItem` is already `Copy + Eq`. Not measured how often clicks collide in practice.
  - Blind spot: Not verified how muda redraws an open popup on Windows.
- **Decision**: PENDING

### F3 — The ordering comment sits above `start_timer` but describes the commands

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: `src-tauri/src/work_time/commands.rs:70-73`
- **Detail**: The line comment "START/STOP notify as soon as the write commits and only then read the new status…" is now stacked directly above the `///` doc of `start_timer`. The status read it describes now happens in `start_work` / `stop_work` (`:94-104`), and `stop_timer` has no such note. This is harmless, but a reader of the S-07 hook point may misread where the ordering is enforced.
- **Fix**: Move the `//` comment above `start_work` (or fold it into one sentence in the `start_timer` / `stop_timer` docs: "callers read the status only after this returns").
- **Decision**: PENDING

## Fixes applied

- F2 (tray menu item re-applied every round): fixed. `TrayWorkItem` keeps the last applied `WorkItem`; `update_tray` skips the clear and the setters when the item is unchanged, and records it as applied only when both setters succeed.
- F3 (misplaced ordering comment): fixed. The comment now sits above `start_work`.

Commit: `fix(tray-timer-control): skip unchanged tray menu item updates (review)`.
