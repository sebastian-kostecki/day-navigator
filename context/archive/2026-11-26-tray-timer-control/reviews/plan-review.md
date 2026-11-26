<!-- PLAN-REVIEW-REPORT -->
# Plan Review: START/STOP from the tray and today's work time in the tray tooltip

- **Plan**: `context/changes/tray-timer-control/plan.md`
- **Mode**: Deep (code checks run inline, no sub-agent)
- **Date**: 2026-11-26
- **Verdict**: SOUND (all findings fixed in the plan)
- **Findings**: 0 critical, 2 warnings, 4 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

Grounding: 9/9 paths ✓ (`runtime.rs`, `health/mod.rs`, `work_time/{mod,commands,rules,reminders}.rs`, `lib.rs`, `icons/source/tray-{ok,error}.svg`, `README.md`, `CHANGELOG.md`), 14/14 symbols ✓, brief↔plan ✓, Progress↔Phase ✓ (5/5 phases, 30/30 criteria rows, no checkboxes in phase blocks).

What was checked against the code:

- Line refs hold: `tray_image` `runtime.rs:139-145`, `build_tray` `:149-185`, `update_tray` `:206-216`, `publish` `:228-237`, `check_round` `:296-355` (heartbeat stamp `:305-308`), locking comment `:5-9`. `health::tooltip` `:257-289`, `cap_utf16` `:291-307` (private), `TOOLTIP_MAX_UTF16` `:17-19` (private), proptest `:723-737`. `work_time::start`/`stop` `mod.rs:257-294`, `status` `:166-202`, `in_day`/`running_entry` private (`:109-141`), `rules::portion` `rules.rs:57-63`, `reminder_facts` `reminders.rs:51-73`, commands `commands.rs:68-87`.
- Threads: the menu callback runs on the main thread. Tauri 2.12.7 `MenuItem::set_text`/`set_enabled` (`menu/normal.rs:94`, `:105`) go through `run_item_main_thread!` (`menu/mod.rs:26-40`) and block on a channel off the main thread. Menu inner types are `Send + Sync` (`menu/mod.rs:94-95`). `TrayIcon::set_icon`/`set_tooltip` (`tray/mod.rs:513`, `:537`), `PredefinedMenuItem::separator` (`menu/predefined.rs:15`) and `Manager::try_state` (`lib.rs:771`) exist. The plan holds no lock across them except `published`. The store is never touched on the main thread. `with_store` waits on a mutex and opens the store if needed (`store/mod.rs:242-250`); `Busy` only comes from SQLite busy/locked.
- The health round is the only tray writer; `work_time::commands` is the only writer of work entries (no other `work_entries` writer in `src-tauri/src`). So refreshing through `notify` covers every work-time write.
- The icon recipe was re-run in this review: `npx tauri icon src-tauri/icons/source/tray-ok.svg -o <scratch> --png 32` writes only `32x32.png`, and its SHA-256 equals `src-tauri/icons/tray/ok.png`. The PowerShell IHDR check prints `32x32 type 6` on `ok.png`. The bottom-right dot (824,824) lies on the ring away from the needle, which points north-east with its tail to the south-west.
- The UI claims hold: the single `listen(WORK_TIME_CHANGED)` is in `useWorkTimer.ts:85-122`; `WorkTimerHeader.tsx:21-24` drops a stale question; `total.ts`/`format.ts` give the header formula and `formatHm` flooring.
- Parallel slices: the `lib.rs` edit is one line (`pub mod tray_view;`). The `work_time/mod.rs` edit is two module lines. `commands.rs` changes `start_work`/`stop_work`/`notify`. `settings.rs` is untouched. The `runtime.rs` edits are unavoidable and local to the tray functions.
- `docs/reference/contract-surfaces.md` and `context/foundation/lessons.md` are absent.

## Findings

### F1 — `check_round` contract names a missing `today` and a `u64` clock

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 4 — §2 `check_round`
- **Detail**: The plan says to call `today_work(store, now_ms, &Local)` with "the same `today` as the probe". `check_round` has no `today` variable: the date is computed inline in the `probe_round(...)` call (`runtime.rs:301`). `runtime::now_ms()` returns `u64`, but `today_work` takes `i64`. A fresh implementer would hit a type error or have to guess.
- **Fix**: Hoist `let today = Local::now().date_naive();` and pass `chrono::Utc::now().timestamp_millis()` (as `commands::run` does), not `now_ms()`.
- **Decision**: FIXED (applied to the plan by the agent; no user reachable)

### F2 — The shown action can disagree with the label while it changes

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 5 — §1 `update_tray`; Critical Implementation Details, "Act on the shown intent"
- **Detail**: The plan sets the text, then the enabled flag, then `shown`, and gives no reason for that order. Between the two blocking main-thread setters and the `shown` write, a click can act on the previous action while the label already shows the new one. Usually this is harmless (`AlreadyRunning`/`NotRunning`). But it breaks the plan's own rule "act on the shown intent", and an implementer could reorder the writes, which makes it worse: the old label paired with the new action inverts the user's intent. The plan also never says how `update_tray` finds the managed item.
- **Fix**: Store `None` in `shown` before the setters and the new action after them, each with a short lock. Get the item with `app.try_state::<TrayWorkItem>()`. Recorded as A15.
- **Decision**: FIXED (applied to the plan by the agent)

### F3 — `tray_view::TrayIcon` reuses the name of `tauri::tray::TrayIcon`

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 2 — §2; Phase 3 — §3
- **Detail**: `runtime.rs` works with `tauri::tray::TrayIcon` (its module doc names `TrayIcon::set_icon`, and `tray_by_id` returns one). A pure enum with the same name in the same file invites a wrong import or a confusing doc.
- **Fix**: Rename the enum to `TrayIconKind` (Phase 2 contract, Phase 3 `tray_image`). Recorded as A17.
- **Decision**: FIXED (applied to the plan by the agent)

### F4 — Phase 1 tests: helpers and call style left open

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 — §3 Tests
- **Detail**: The helpers the plan asks for already exist in `work_time/tests.rs:14-28` (`t()`, `open()`). If the tests called `crate::work_time::start(`, Phase 4's check 4.5 (`git grep "work_time::(start|stop)\(" -- src-tauri/src` lists nothing) would fail for a reason unrelated to the shared path. The "removed entry" fixture also needs `remove`, which the plan did not list.
- **Fix**: Point to the existing helpers, list `remove`, and require `use super::*;` with unqualified calls.
- **Decision**: FIXED (applied to the plan by the agent)

### F5 — Header REC is red, the tray running dot is amber

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 — §1 Icon source
- **Detail**: `.dn-rec` uses `--danger` (#ff7b72), which is the same red as `tray-error.svg`. The plan's amber dot is right under D2 (red in the tray means a source error). But an implementer or reviewer may "align" the dot to the header's red.
- **Fix**: State the difference and its reason in the Phase 3 intent. Recorded as A18. The Konsola artifact follow-up (5.12) should document it.
- **Decision**: FIXED (applied to the plan by the agent)

### F6 — A transient `Busy` would toast "Dane aplikacji są niedostępne"

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 — §2 `failure_notice`
- **Detail**: `Store(_)` → "…niedostępne — szczegóły w panelu „Źródła”" also covers `Store(Busy)`. `Busy` is transient and never shows in the "Źródła" panel, so the toast would send the user to a panel that says OK.
- **Fix**: Give `Store(Busy)` the generic body ("Otwórz aplikację i spróbuj ponownie.") and add a test row. Recorded as A16.
- **Decision**: FIXED (applied to the plan by the agent)

## Triage (non-interactive)

No user was reachable, so the agent fixed every finding in the plan and recorded each choice as an agent default (A15–A18) in the plan's Decisions. `plan-brief.md` needed no change: none of the fixes changes its scope, decisions or phases.

Fixed: F1, F2, F3, F4, F5, F6 (6). Skipped / Accepted / Dismissed: none.

► Verdict after fixes: SOUND. No open question needs the user. The Konsola design-system artifact update (5.12) stays a manual follow-up for the user.
