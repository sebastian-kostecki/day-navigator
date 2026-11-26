<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Notatka przy wpisie czasu pracy — plan implementacji

- **Plan**: context/changes/work-entry-note/plan.md
- **Mode**: Deep
- **Date**: 2026-11-26
- **Verdict**: REVISE → SOUND after fixes
- **Findings**: 0 critical, 2 warnings, 2 observations

Run non-interactively (no user reachable): every finding below was fixed in the plan; judgement calls are recorded in the plan's Decisions (items 13–14) as "default chosen by agent, user may revise".

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | PASS |

## Grounding

Grounding: 14/14 paths ✓ (all files the plan modifies exist; `work_time/note.rs` and `tests/work_entry_note_crash.rs` are new as planned), 16/16 symbols ✓ (`Entry`, `row_to_entry`, `counted_by_id`, `running_entry`, `in_day`, `EntryRow`, `add`, `update`, `resolve_or_keep`, `WorkTimeError::kind`, `run`, `SaveFn`, `EntryForm`, `charCount`, `MIGRATIONS`/`validate`, `run_writer_until`/`KillAt`/`MAX_WRITER_COMMITS`), line refs ✓ (spot-checked ~30, all match), brief↔plan ✓, Progress↔Phase ✓ (1.1–1.5, 2.1–2.11, 3.1–3.4; one `## Progress`; no checkboxes in phase blocks).

Verified against the code:
- All `add`/`update` call sites match the plan's list (`work_time/tests.rs`, `norm_tests.rs:45`, `tests/work_time.rs:98/152/178`); no other Rust caller; only `api.ts` invokes the two commands; no exhaustive match on `WorkTimeError` outside the module (`lib.rs:113` and `work_reminders.rs:68` use wildcards).
- No test builds an `EntryRow` literal, so Phase 1 compiles with the new field; `Entry` has no `Debug` derive; `report::compute` reads entries through `in_day` (`report.rs:125`), so `Entry.note` is ready for S-05.
- `update` with 7 parameters stays under clippy's `too_many_arguments` limit; rusqlite tuple params cover the 5-tuple `UPDATE`.
- `serde_json` is a regular dependency, usable from integration tests; `events::list_today(store, today)` exists.
- Bundled SQLite is 3.54.1 (`libsqlite3-sys-0.40.0/sqlite3/sqlite3.h:149`), so `ADD COLUMN … NOT NULL DEFAULT '' CHECK (…)` is valid and checks existing rows.
- Baseline of the grep gates holds today: `log::` in `work_time` is exactly `commands.rs:37/39/47`; `invoke` only in `src/worktime/api.ts`; no literal colours in `WorkTime.css`.
- Leak paths: commands log `e.kind()` only; global log level Warn; `NewEvent::WorkStarted/WorkStopped` carry no data; no `{:?}` of entry types anywhere.

## Findings

### F1 — Manual passes migrate the real, shared database with no copy/restore step

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 1 Manual 1.5, Phase 2 Manual 2.6–2.11, Manual Testing Steps
- **Detail**: Every worktree, `tauri dev` and the installed release share the identifier `com.sebas.day-navigator` (`tauri.conf.json:4`), so a dev build migrates the user's daily `%APPDATA%\com.sebas.day-navigator\day-navigator.db` to v11. Afterwards the installed 0.10.1 (supports v10) refuses it as `NewerSchema` — "Dane aplikacji" red, the work timer unusable — until a release with v11 ships; S-01/S-03 dev builds without v11 refuse it too; and if the migration is renumbered at merge, the v11-with-note database fails a different v11 (`MigrationFailed`) and is refused, never reset. Row 1.5 even asks for a run on real data. The own-tasks plan (`context/archive/2026-11-23-own-tasks/plan.md:78-86`) solved exactly this with a "Dev runs share the real database" procedure; this plan omits it.
- **Fix**: Add a Critical Implementation Detail "Ręczne przebiegi dzielą prawdziwą bazę" with the copy-aside-before / restore-after procedure (also after another worktree's dev run or a renumbering, unless the v11 release is installed); reference it from Manual 1.5 (Phase block and Progress), the Phase 1 Implementation Note, Manual Testing Steps and Migration Notes.
  - Strength: Same procedure already used and archived for own-tasks; keeps the user's daily app working through the slice.
  - Tradeoff: Entries typed during manual tests are lost on restore (they are invented anyway).
  - Confidence: HIGH — identifier and `NewerSchema` refusal confirmed in code and archived plans.
  - Blind spot: None significant.
- **Decision**: FIXED (applied in plan; Decision 13, default chosen by agent)

### F2 — Merge with S-01/S-03 can break the build through new `add`/`update` call sites

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: Phase 2 §3–§5; Implementation Approach (merge notes)
- **Detail**: Phase 2 changes the public signatures of `work_time::add` and `work_time::update`. A parallel branch (most likely S-01's holiday-norm tests in `norm_tests.rs`, `work_time/tests.rs` or `tests/work_time.rs`) that adds new calls with the old signature merges textually clean but fails `cargo test`/clippy afterwards. The plan covers migration renumbering and the CHANGELOG conflict, not this one. It also does not say where the new CSS rules go in `WorkTime.css`, which S-01 edits too.
- **Fix**: Add a Critical Implementation Detail "Merge z równoległymi slice'ami" (the branch merged second runs `cargo test` and adds `""` to new call sites; migration number and the upgrade test's position assertion move together) and one fixing the CSS placement next to `.wt-row-error` and `.wt-form-actions`.
- **Decision**: FIXED (applied in plan; Decision 14, default chosen by agent)

### F3 — SQLite version open question can be closed now

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Migration Notes; plan-brief "Open Risks"
- **Detail**: Research and brief left "bundled SQLite version unknown, CHECK on ADD COLUMN needs ≥ 3.37" open. `libsqlite3-sys 0.40.0` bundles SQLite 3.54.1, so the behaviour is settled; the upgrade test still pins it.
- **Fix**: State the version and source in Migration Notes and the brief's risk list.
- **Decision**: FIXED

### F4 — Small precision gaps in success criteria

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 2 Automated 2.1, 2.4; Testing Strategy
- **Detail**: 2.1 and Testing Strategy name `src/work_time/tests.rs` (reads as a path in the UI `src/`; the file is `src-tauri/src/work_time/tests.rs`). 2.4 says "only the three existing lines" without stating it as an exact, checkable count.
- **Fix**: Use full `src-tauri/...` paths; phrase 2.4 as "exactly three lines, all in `commands.rs`" (Phase block and Progress kept identical).
- **Decision**: FIXED

## Considered and not raised

- `.wt-note` / `.wt-note-count` set `color` in view CSS, which AGENTS.md calls "layout only"; the plan records this as Decision 7 following the existing `.wt-duration` precedent and uses tokens only. Left as the plan's documented default.
- Rewording "Never user content or vault data there." in `AGENTS.md:11` goes slightly beyond user decision 1, but resolves a real contradiction (own-task notes already live there) and is already recorded as an agent default (Decision 8).
- The crash test for a single-row insert adds little beyond SQLite atomicity, but AGENTS.md requires data slices to reuse the harness; kept. Its load (`find_overlap` scans ~n rows per insert, at most a few thousand rows per run) is acceptable; `NOW_MS = BASE + 400 days` leaves room for 288 000 two-minute entries, above 10 × `MAX_WRITER_COMMITS`.
- Phase 2 is large (rule, errors, domain, commands, ~20 test call sites, TS, form), but splitting it would separate the `invoke` contract from its TS mirror or leave a commit where edits clear notes; one phase is the right unit.

## Triage summary

- Fixed: F1, F2, F3, F4 (4)
- Skipped / Accepted / Dismissed: none
- Verdict after fixes: REVISE → SOUND
