<!-- PLAN-REVIEW-REPORT -->
# Plan Review: End-of-day summary after STOP

- **Plan**: `context/changes/end-of-day-summary/plan.md`
- **Mode**: Deep (code checks run inline, no sub-agent)
- **Date**: 2026-11-26
- **Verdict**: REVISE before fixes, SOUND after (every finding fixed in the plan; non-interactive run, no user reachable)
- **Findings**: 1 critical, 4 warnings, 2 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | FAIL (before fixes) |

## Grounding

Grounding: 28/28 paths ✓ (`work_time/{commands,mod,error,rules}.rs`, `todo/{mod,commands,error}.rs`, `runtime.rs`, `lib.rs`, `tray_view.rs`, `health/mod.rs`, `store/{mod,error}.rs`, `src/worktime/{useWorkTimer.ts,WorkTimerHeader.tsx}`, `src/App.tsx`, `src/components/{time.ts,ErrorState.tsx,Panel,StatStrip,ItemRow}`, `src/todo/{summary.ts,shortRef.ts,useTodo.ts,TodoView.tsx}`, `prd.md`, `README.md`, `CHANGELOG.md`), 18/18 symbols ✓, brief↔plan ✓, Progress↔Phase ✓ (4/4 phases, 30/30 criteria rows before fixes, no checkboxes in phase blocks).

What was checked against the code:

- Line refs hold: `stop_timer` `commands.rs:148-156`, `stop_work` `:166-170`, `Policy`/`work_time_settings` `:23-53` (both `pub`), `execute` `:80-106`; `run_tray_action` `runtime.rs:202-216` (success branch empty), `toast` `:353-361`; `status` `work_time/mod.rs:178-215` (takes `&Store`), live-total comment `:50`; `rules::local_day` `rules.rs:33` (`pub`, module `pub`); `todo::list` `todo/mod.rs:267-290` (`&mut Store`, one write, `_now` unused, limit only fills `TodoList.limit`), `roll_over_in` `:119-137` (idempotent, records no event), `visible` order open-then-ticked `:246-261`; `TodoItem`/`TodoEntry` derive `PartialEq, Eq`; `own_task_default_space` value-free mapping `lib.rs:110-124`; `APP_NAME` `pub(crate)` `health/mod.rs:21`, `Notice` `:146-149`; `tray_view::format_hm` `:88-92` (pub, floored); TS `formatHm(ms, true)` `time.ts:77-89`.
- `todo::list` from the summary is safe: it runs inside one `with_store` after `status`, on the blocking pool; roll-over is idempotent (a second roll-over closes nothing) and records no event; `list_todo` does the same and emits nothing, so no listener loop. No nested `with_store`, so no lock-order issue.
- The toast cannot hold the store lock: `stop_notice(&app)` returns the `Notice` only after `crate::blocking` (which owns the `with_store` call) has completed; `toast` is called after.
- Phase gates and dead code: every Rust item added in Phase 1 is reached from the registered `day_summary` command; every Phase 2 item from `run_tray_action`. TS exports added in Phase 1 but used in Phase 3 are fine for `noUnusedLocals`.
- CSP: no new resource kind (component CSS file, IPC only); no capability change (notification plugin is Rust-only).
- AGENTS.md: one timer subscription kept (the panel reads `useWorkTimer()` only); `invoke` only in `api.ts`; tokens only; `dn-` prefix; serde camelCase and tagged error enum; store work async on the blocking pool; no titles in logs; no 8 h; `norm.rs`/`report.rs`/`schema.rs` untouched.
- `docs/reference/contract-surfaces.md` and `context/foundation/lessons.md` are absent.

## Findings

### F1 — Phase 3 "one timer subscription" gate fails on today's code

- **Severity**: ❌ CRITICAL
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 3 — Success Criteria (row 3.3)
- **Detail**: The check said `git grep -n "listen(WORK_TIME_CHANGED" -- src` must list only `src/worktime/useWorkTimer.ts`. At this commit it also lists `src/worktime/useMonthReport.ts:62` and `src/worktime/useWorkEntries.ts:61` (they reload their own data on the event, which is allowed). The gate could never pass, so Phase 3 could never be committed green.
- **Fix**: Check what this slice could break instead: nothing in `src/day-summary` or `App.tsx` mentions `workTimerStatus`, `WORK_TIME_CHANGED`, `work-time-changed`, `startWork` or `stopWork`, and `workTimerStatus()` is called only in `useWorkTimer.ts`. Progress title 3.3 updated.
- **Decision**: FIXED

### F2 — `stop()` result tied to "status applied" loses the panel in a race

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 3 §1 (`useWorkTimer.ts`), Key Discoveries, A3
- **Detail**: The contract said `act` resolves `true` "when `call()` resolved and its status was applied". `stop_timer` emits `work-time-changed` before `stop_work` reads the status (`commands.rs:151-170`); the listener's `refresh` takes a higher request id (`useWorkTimer.ts:67-70,96-98`), and when its answer lands first `apply` drops the STOP's own status as stale (`:59-61`). An implementer making the result depend on `apply` would open no summary after a committed STOP, intermittently.
- **Fix**: `act` resolves `true` whenever `call()` resolved; the shared timer then already shows the post-STOP state either way. Key Discoveries, Critical Implementation Details, A3 and the brief updated; the pre-existing "STOP committed but follow-up status read failed → no panel" edge is documented in A3.
- **Decision**: FIXED

### F3 — Panel is a snapshot that sits above "Do zrobienia"

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 3 §3–4, A2
- **Detail**: The panel read once on mount. It is mounted above the active view, and "Do zrobienia" is the default view, so ticking a forgotten item after STOP (a natural end-of-day step) would leave "Przechodzi na jutro" contradicting the list right below it. It also made the `ErrorState` "last success" time impossible.
- **Fix A ⭐ Recommended**: `useDaySummary` re-reads on `todo-changed`, subscribe-then-load and latest-wins as `useTodo` does; a failed reload shows `ErrorState` with the last successful read time.
  - Strength: The panel always agrees with "Do zrobienia"; reuses an existing event and pattern; satisfies the AGENTS.md "last successful refresh" rule.
  - Tradeoff: One listener and a few lines more; one extra `status` + `todo::list` per tick while the panel is open.
  - Confidence: HIGH — `todo-changed` is emitted by every todo mutation (`todo/commands.rs:98-104`) and `day_summary` emits nothing, so no loop.
  - Blind spot: "done" in "Moje zadania" emits no `todo-changed` (`lib.rs:92-107`); documented as a known residue.
- **Fix B**: Keep the one-shot read and label the panel as a snapshot ("stan z HH:MM").
  - Strength: No new listener.
  - Tradeoff: The visible contradiction stays; the user has to close and STOP again to refresh.
  - Confidence: MED — honest, but works against the "how did the day go" purpose.
  - Blind spot: None significant.
- **Decision**: FIXED (Fix A; A2 refined, still an agent default; manual row 3.12 added)

### F4 — Grep and diff gates miss the slice's new files

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phases 1–4 — Success Criteria
- **Detail**: `git grep` searches tracked files only and `git diff --quiet HEAD` / `git diff --name-only HEAD` ignore untracked files. Before the phase commit every new file is untracked, so rows 1.5, 1.6, 2.5, 3.4, 3.5 would pass vacuously, and 2.6, 3.6, 4.3 would not see a new file in a forbidden place. Pathspecs are also relative to the current directory, and the Rust gates run in `src-tauri/`.
- **Fix**: `git grep --untracked`, `git status --porcelain -- <paths>` (empty = pass), and one sentence in Implementation Approach: run every git check from the worktree root. Row 2.6 also covers `src-tauri/Cargo.toml` (no new dependency). Progress title 2.6 updated.
- **Decision**: FIXED

### F5 — Notice proptest needs a `regex` crate the project does not have

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 §1
- **Detail**: The proptest was specified as matching a regular expression. `regex` is not a dependency or dev-dependency of `src-tauri/Cargo.toml` (only transitive in `Cargo.lock`), so the test would need an unplanned dependency change.
- **Fix**: Same property, checked by hand (`strip_prefix`/`strip_suffix`, `split_once(':')`, ASCII digits); the plan states that the slice adds no dependency.
- **Decision**: FIXED

### F6 — Executor details a fresh sub-agent would have to guess

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §1–3, Phase 3 §3–4, Key Discoveries
- **Detail**: (a) `read` passed `<local date-time of now_ms>` to `todo::list` without saying how; (b) `DaySummaryError` listed no `#[error]` messages and carried `rename_all_fields` with no struct variants; (c) the panel's "praca"/"prywatne" refs come from `SPACE_REF`, which is private to `TodoView.tsx:38`; (d) the worktree has no `node_modules`, so the first frontend gate fails without `npm install`; (e) eslint-plugin-react-hooks 7 flags synchronous `setState` in an effect body; (f) the dashboard "Odchylenie" tile reads `work_month_report` with `Policy::Strict` while `status` uses `Lenient`, and no view displays `monthDeviationMs` today, so A8's "as the header shows it" was inexact.
- **Fix**: (a) resolve `now_ms` once with `tz.timestamp_millis_opt(...).earliest()` → `InvalidLocalTime`, use `.naive_local()` and `.date()`; (b) messages in the `work_time/error.rs` style, no `rename_all_fields`; (c) a local `Record<Space, string>` in the panel (TodoView stays untouched); (d) `npm install` note in Key Discoveries; (e) set `loading` in `retry`, not in the effect body, and call `onClose` from the effect (never render `null` while running, or a tray STOP brings the old summary back); (f) A8 reworded and the Strict/Lenient difference recorded.
- **Decision**: FIXED

### F7 — Shared-file edits placed where parallel slices also append

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architectural Fitness
- **Location**: Phase 1 §3 (`lib.rs`), Phase 4, plan-brief Open Risks
- **Detail**: `generate_handler!` ends with `work_reminders::reminder_settings` and no trailing comma, so appending there conflicts with every parallel slice that adds a command (S-02, S-04, S-05, S-08). S-02 may also change the `work_time::status` or `work_time_settings` signature. CHANGELOG `[Unreleased]` and the PRD sentence are shared hotspots (the PRD one was already handled).
- **Fix**: `pub mod day_summary;` between `cli` and `events`; the handler right after `own_task_default_space,`; brief Open Risks name the S-02 signature risk (one-line fix at merge) and the CHANGELOG union.
- **Decision**: FIXED

## Triage summary

```
Fixed:     F1, F2, F3 (Fix A), F4, F5, F6, F7   (7)
Skipped:   —
Accepted:  —
Dismissed: —

► Verdict after fixes: REVISE → SOUND
```

User decisions D1–D5 are unchanged. Agent defaults A2, A3 and A8 were refined and stay marked "default chosen by agent, user may revise". No open question needs the user.
