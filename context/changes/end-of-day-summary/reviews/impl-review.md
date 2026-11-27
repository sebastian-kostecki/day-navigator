<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: End-of-day summary after STOP

- **Plan**: context/changes/end-of-day-summary/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4
- **Date**: 2026-11-27
- **Verdict**: APPROVED
- **Findings**: 0 critical, 1 warning, 4 observations

Commits reviewed on top of the `develop` merge base `d34904a`: `fe39556` (p1), `097aecb` (p2), `34be0c3` (p3), `7f37603` (p4). The review ran non-interactively (no user reachable); no code was changed.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | WARNING |
| Success Criteria | PASS |

## Gates run for this review

- `cargo fmt --check`: pass. `cargo clippy --all-targets -- -D warnings`: pass (exit 0). `cargo test`: pass (562 unit tests in the lib crate plus every integration binary green, 0 failed).
- `npm run lint`, `npm run format:check`, `npm run build`: pass.
- Plan grep checks: `tauri` is imported only in `src-tauri/src/day_summary/commands.rs`. No `DAILY_NORM_MS`, `3_600_000` or 8 h literal in `src-tauri/src/day_summary` or `src/day-summary`. `norm.rs`, `report.rs`, `schema.rs`, `Cargo.toml`, `Cargo.lock`, `package*.json`, `tauri.conf.json` and `capabilities/` are unchanged against `d34904a`. `src/day-summary` and `App.tsx` do not read the timer status and do not listen to `work-time-changed`; `workTimerStatus()` is called only in `useWorkTimer.ts`. `@tauri-apps/api/core` is imported only in `src/day-summary/api.ts`. There are no literal colours in `src/day-summary`. The PRD no longer lists "zamknięcie dnia" and names MS-19.
- Manual rows 2.7, 2.8 and 3.7–3.12 are still unticked. They are pending for the human (see F5) and are not counted as failures.

## What was checked and holds

- **Plan match**: every planned file exists and matches its contract, including types, `compose`, `today_ms`, `read`, the error enum and its `From` mappings, the command, `notice.rs`, `stop_notice`, the `runtime.rs` STOP branch, the `stop()` → `boolean` change, `onStopped`, `useDaySummary`, the panel, the `App.tsx` mount and the docs and PRD. No file is changed outside the plan's list.
- **One timer state**: the panel reads `useWorkTimer()` only to close itself on START. `useDaySummary` listens to `todo-changed` only. `start` and `stop` returning `Promise<boolean>` breaks no caller: `TodoView` and `WorkTimeView` read only `timer` and `nowMs`, and the header calls `void start()`.
- **The panel cannot close itself when it opens (normal path)**: `apply` drops a response only when `id < applied`. So once `stop_work` has resolved, the shared state is either the STOP's own status or a newer refresh that was issued after the commit (`work-time-changed`). Both show "stopped", so `running` is false when the panel mounts. The one exception is F2.
- **STOP refresh vs the panel's read**: `day_summary` is invoked only after `stop_work` resolved, so it always reads the committed state. Its `todo::list` roll-over runs inside `with_store` on the blocking pool, in one store call after `status`, on the same local day. It emits nothing, so there is no `todo-changed` loop (only mutations and the GitLab/GitHub refresh emit it).
- **Tray toast after the lock is released**: `stop_notice` awaits `blocking` (with `with_store` returning) before `toast`. The error branch of `run_tray_action` is unchanged, and `refresh_tray` is still called only on errors, as before. The header STOP sends no toast, and the tray STOP calls no `onStopped`.
- **Signed formatter**: Rust `ms / 60_000` truncates toward zero like `Math.trunc`. It gives `0:00` for anything under one minute (including −59 999, where JS `-0 === 0`), U+2212 for minus, `+` otherwise and `unsigned_abs` for `i64::MIN`. This matches `formatHm(ms, true)` exactly. A proptest pins the body shape, so no title, reference or count can reach the toast.
- **Contracts**: `DaySummary` uses camelCase (`day`, `todayMs`, `monthDeviationMs`, `done`, `carryOver`). `DaySummaryError` is tagged with `kind`/`detail`, and the TS mirror matches it variant for variant. Errors carry no values; foreign variants map to `StoreError::Sql { reason }` with a fixed text, the `own_task_default_space` precedent. Logs carry `kind()` only.
- **After midnight**: the summary covers the new day (A9, tested: 22:00 → 00:30 read at 00:31 gives the new date and 30 min).
- **React 19 and hooks**: lint is clean. `retry` sets `loading` in the handler, not in the effect. The subscription is made before the first load, the latest request wins, and StrictMode's double mount is safe.
- **Domain tests** (invented data only): the done / carry-over split and its order against `todo::list`, a carried-over flag, yesterday's ticks rolled over, the closed total, the running total, the deviation passed through and compared with `status` (no norm literal), an empty day, after midnight, `compose`, `today_ms` clamping, and error serialization and mappings.
- **Merge risk** (diffed against the four parallel branches):
  - `lib.rs`: the module line and the handler line do not touch S-04's `pub mod autostart;` (at least two unchanged lines between them) or S-05/S-08's handler lines near `work_month_report`.
  - `runtime.rs`: the edits are about 150 lines away from S-04's `probe_round` edit.
  - `README.md`: the edited tray bullet sits one unchanged line below S-05/S-08's inserts.
  - S-02 adds `WorkTimeSettings::daily_norm_ms`. The tests use `WorkTimeSettings::default()` and pass the deviation through, so no test breaks.
  - The expected conflicts are only in `CHANGELOG.md` ([Unreleased] / Added) and the PRD sentence shared with S-04/S-08. The plan documents both: keep every slice's own removal.

## Findings

### F1 — Panel CSS copies the `.panel-title` text style

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/day-summary/DaySummaryPanel.css:14-19 (used at src/day-summary/DaySummaryPanel.tsx:52)
- **Detail**: `.dn-day-summary-head` sets `margin: 0; font-size: 14px; line-height: 20px; font-weight: 600`. That is a copy of the `.panel-title` text style in `src/styles/tokens.css:159-164`. The plan's contract says "CSS: layout only, tokens only", and AGENTS.md says tokens.css is the only place for values and view CSS is layout only. The copy would drift if the text style changes.
- **Fix**: Give the `<h3>` the shared text class, `className="panel-title"` (or `label-caps` if a quieter sub-heading is wanted), and delete the `.dn-day-summary-head` rule from `DaySummaryPanel.css`.
- **Decision**: PENDING — real problem, low severity, fix before merge.

### F2 — A stale timer refresh can close the panel right after it opens

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src/day-summary/DaySummaryPanel.tsx:66-71; src/worktime/useWorkTimer.ts:60-66
- **Detail**: `useWorkTimer` orders responses by issue id, not by commit. Suppose a `refresh()` is issued while `stop_work` is in flight, from `focus`, `visibilitychange` or the day-end timeout (for example a STOP right at midnight), and it reads the store before the STOP commits. Its "running" answer then has a higher id and can be applied after the STOP's status. The panel sees `running` and closes itself at once, and the summary is never seen. No data is affected, the window is normally already focused when "TAK, STOP" is clicked, and the header already has the same pre-existing flicker. This is not a plan defect: the plan's Key Discoveries cover only the `work-time-changed` refresh, which is issued after the commit.
- **Fix**: Optional hardening, if it shows up in use: close only on a START that happened after the panel opened. Record `openedAt = Date.now()` in a ref at mount and call `onClose()` only when `timer.status.running.startMs >= openedAt`. A stale pre-STOP status has an earlier `startMs`.
  - Strength: removes the race without touching the shared timer hook.
  - Tradeoff: one ref and one comparison in the panel. If the system clock moves back between STOP and START, an edge case could keep a stale panel open until "Zamknij".
  - Confidence: MED — the race needs a refresh issued during a STOP of a few milliseconds, and I did not reproduce it.
  - Blind spot: not observed in a running app.
- **Decision**: ACCEPTED — note only, no fix required.

### F3 — Domain tests tick own tasks only through "Do zrobienia"

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/day_summary/tests.rs:434-460
- **Detail**: The plan's first test case is "Own task done today and a GitLab entry ticked today land in `done`". The test ticks the own task's "Do zrobienia" entry with `todo::set_ticked`. It does not mark the task done in "Moje zadania", and it has no GitLab entry closed in source. The summary relies only on `TodoItem.ticked` from `todo::list`, whose own tests cover how done and closed-in-source entries become ticked, so the risk is low.
- **Fix**: Optional: add one case that marks the own task done through `own_tasks` and one GitLab entry with `closed_in_source`, and assert that both are in `done`.
- **Decision**: ACCEPTED — note only.

### F4 — Phase 1 Progress rows have no commit SHA

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/end-of-day-summary/plan.md:423-429
- **Detail**: The Progress convention is "Append ` — <commit sha>` when a step lands". Phases 2–4 carry their SHAs. Rows 1.1–1.7 do not, although they landed in `fe39556`.
- **Fix**: Append ` — fe39556` to rows 1.1–1.7.
- **Decision**: PENDING — real problem, nit severity (docs only).

### F5 — Manual checks and the built-app CSP check are still open

- **Severity**: 🔍 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/end-of-day-summary/plan.md:444-445, 460-465
- **Detail**: Rows 2.7, 2.8 and 3.7–3.12 are unticked. They cover the toast figures against the header and the dashboard, no toast on START or header STOP, the panel lists and dismissal, no panel on a tray STOP, a tick while the panel is open, and the built app with no CSP violation and no "IPC custom protocol failed". The code shows no new resource kind (no inline style, no external request), so CSP should be unaffected, but AGENTS.md requires checking it in `npm run tauri build -- --debug --no-bundle`. One more expected behaviour to confirm: with an invalid `workTime` block (or S-02's invalid `dailyNormHours`), the dashboard tile shows "—" while the summary and the toast use the default calendar and norm (A8, `Policy::Lenient` as in START/STOP).
- **Fix**: The human runs the manual rows before `/10x-archive`.
- **Decision**: PENDING — human checklist, not a code problem.
