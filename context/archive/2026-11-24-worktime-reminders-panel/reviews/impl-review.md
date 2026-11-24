<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Panel „Przypomnienia” w „Czas pracy” — Implementation Plan

- **Plan**: context/changes/worktime-reminders-panel/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING (1 observation) |
| Scope Discipline | PASS |
| Safety & Quality | WARNING (1 observation) |
| Architecture | PASS |
| Pattern Consistency | WARNING (1 warning) |
| Success Criteria | PASS |

## Summary of evidence

- Diff `8227f4f..HEAD`: every file in the plan's "Changes Required" changed (`settings.rs`, `work_reminders.rs`, `lib.rs`, `types.ts`, `api.ts`, `useReminderSettings.ts`, `RemindersPanel.tsx`, `WorkTimeView.tsx`, `WorkTime.css`). The only file outside the plan is `context/foundation/roadmap.md` (S-06 `ready` → `in-progress`), which is normal lifecycle bookkeeping. No changes in `src/health/`, `src/components/` or the reminder thread's behaviour (all on the plan's NOT list).
- Phase 1 contract matches: `ReminderOrigin` (derives as specified), `load_reminder_settings_detailed`, `parse_reminder_settings` returning `(settings, origin)` with `load_reminder_settings` delegating and dropping the origin, `ReminderSettingsError` tagged `Serialize`, camelCase view types, a pure `thresholds` mapping, `ReminderSettingsQueryError { SettingsInvalid, Background }` with both `From` impls, the async `reminder_settings` command using `crate::blocking` and defaults for an unresolved config dir (same `settings_file::settings_path` the thread receives in `lib.rs:155-166`), registration in `generate_handler!`. Logs carry only the error kind.
- Tests cover every case the plan lists: origin (missing file, absent/null/`{}` block, null field, value, `false`, mixed), agreement between the detailed and plain loader over valid and invalid fixtures (including bad JSON, missing/unsupported version and non-UTF-8 bytes), BOM strip, mapping (defaults, `09:05`/`00:00` padding, 1 and 24 h edges, off → `None`), JSON shape of the view type and both error enums, and the `From` conversions. All data is invented.
- Phase 2 contract matches: the hook follows `useMonthReport` (latest request wins, `lastOkAt` kept, refetch on mount/focus/visibility/retry, no event subscription, no interval); the panel has loading/error/data states, `ErrorState` with "ostatnio OK HH:MM" and retry, the "Ustawienia" footer in every state with an inline `Notice` on open failure, no `useWorkTimer()` call; the side column gets `wt-stack`; the new CSS uses only token variables.
- Known adaptations reported by the implementers were checked: `formatHm`/`formatClock` from `./format` (the worktime folder's own formatters, same output as planned: `H:MM` and local `HH:MM`) — acceptable; `domyślnie` per row — matches the plan's desired-state table; `openSettingsFile` from `../gitlab/api` — planned; duplicated Polish text — see F1.
- Gates re-run in this review (all green): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (456 unit tests plus integration suites, 0 failed), `npm run lint`, `npm run format:check`, `npm run build`; `useWorkTimerSource` appears only in `useWorkTimer.ts` and `WorkTimerProvider.tsx`; no literal colours in `src/worktime`.
- Manual rows 2.5–2.9 are ticked (`353df00`). They are UI-only checks with no artefact expected in the diff; the code supports each one (defaults + `domyślnie`, `false` → `—` / `WYŁ.`, refetch on focus, error state with retry and last-OK time, the footer button reusing `open_settings_file`, which never creates the file).

## Findings

### F1 — Invalid-block wording duplicated in TS with no drift guard

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/api.ts:61 (copy of src-tauri/src/health/probe.rs:120)
- **Detail**: The plan says the panel's error uses the `REMINDERS_INVALID_PL` wording. The implementation copies the Polish sentence into a TS const, because the Rust const is private and the command error carries no detail. The two strings match today, but nothing ties them together: the Rust const has no pointer to the TS copy, and no test compares them. If the accepted shape changes (e.g. a new field or a different hour range) and only `probe.rs` is updated, "Stan źródeł" and the "Czas pracy" panel will describe the block differently. The project's rule is that Rust and its TS mirror change together; here that rule is enforced only by memory.
- **Fix A ⭐ Recommended**: Add a Rust unit test in `health/probe.rs` that reads `../../src/worktime/api.ts` with `include_str!` and asserts it contains `REMINDERS_INVALID_PL`, plus a comment on the Rust const naming the TS copy.
  - Strength: Turns the silent drift into a failing `cargo test`, with no change to the command contract or the UI.
  - Tradeoff: A Rust test that reads a frontend file is unusual in this repo (no `include_str!` in `src-tauri/src` today) and couples the crate to the TS file path.
  - Confidence: MED — the check is simple and both files sit in the same repo; the path coupling is the only cost.
  - Blind spot: Whether CI builds the crate from a checkout that always contains `src/` (it does today; a crate-only build would break).
- **Fix B**: Add only a cross-reference comment on the Rust const (`probe.rs:120`) that names `src/worktime/api.ts`.
  - Strength: Zero-cost, keeps the crate independent of the frontend.
  - Tradeoff: Still relies on someone reading the comment; no automated guard.
  - Confidence: HIGH — trivial edit.
  - Blind spot: None significant.
- **Decision**: FIXED (Fix A): `reminders_invalid_text_matches_the_worktime_panel` test in `health/probe.rs` reads `src/worktime/api.ts` via `include_str!` (went red on a changed TS copy, green restored); cross-reference comments on both copies.

### F2 — StatusPill glyph dropped compared with the plan's desired state

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/worktime/RemindersPanel.tsx:28
- **Detail**: The plan's desired-state table shows `● WŁ.` / `○ WYŁ.` (and Current State lists `StatusPill` as "`ok` ●, `calm` ○"). The panel passes `glyph={false}`, so only the text renders. This matches how `src/health/HealthPanel.tsx:55` and `src/health/SourceCard.tsx:37` use the pill, and the user accepted the look in manual check 2.6, so it is an undocumented minor deviation rather than a defect.
- **Fix**: Either drop `glyph={false}` to match the plan, or record the text-only pill as an accepted deviation in the plan's Phase 2 block.
- **Decision**: ACCEPTED: the text-only pill stays (consistent with the health views; the user accepted the look in manual check 2.6). Recorded here as an accepted deviation from the plan's desired-state table.

### F3 — A warning is logged on every refocus while the block is invalid

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_reminders.rs:174
- **Detail**: `reminder_settings` logs `log::warn!("reminders: settings unavailable: SettingsInvalid")` on every failed read. The hook calls the command on both `focus` and `visibilitychange`, so each return to the window while `settings.json` has an invalid block writes two warning lines. The state is already reported by the "Przypomnienia" health source, and the reminder thread deliberately skips the round without logging (`work_reminders.rs:53`). The log carries no path or value, so this is noise only, not a privacy issue.
- **Fix**: Log `SettingsInvalid` at `debug` (a user-editable file state the health source already shows) and keep `warn` for `Background`.
- **Decision**: FIXED: `SettingsInvalid` is logged at `debug`; `Background` stays at `warn`.
