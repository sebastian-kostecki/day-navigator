<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Notatka przy wpisie czasu pracy

- **Plan**: context/changes/work-entry-note/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3
- **Diff**: `455cbed..HEAD` (`fe53497` p1, `0b2ea3e` p2, `12ebc17` p3)
- **Date**: 2026-11-26
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 3 observations

No real problems (nothing must be fixed before merge). The three findings below are observations / nits.

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING (1 observation) |
| Architecture | PASS |
| Pattern Consistency | WARNING (2 observations) |
| Success Criteria | PASS |

## Plan adherence (summary)

Every planned change is present and matches the contract (MATCH):

- `store/schema.rs`: v11 appended after v10 with the planned comment and SQL; no shipped migration edited (diff has no removed lines).
- `work_time/mod.rs`: `Entry.note` read at index 3 in all three SELECTs (`counted_by_id`, `running_entry`, `in_day`), so `report::compute` (S-05) gets it with no extra query; `EntryRow.note` after `day_portion_ms`; `add`/`update` call `note::normalize_note` first, before time parsing and before the single `store.write`; INSERT/UPDATE use bound `?3` only.
- `work_time/note.rs`: `NOTE_MAX = 200`, trim, `\r`/`\n` check before the length check, `chars().count()`; unit tests for trim, blank, edge breaks, 200/201 × "ż" and "👍", `\n`/`\r\n`/`\r`, no-leak, JSON shape.
- `error.rs`: `NoteTooLong { max }`, `NoteMultiline`, `kind()` arms; messages carry only the limit. Enum keeps `tag = "kind", content = "detail"`; TS mirror in `types.ts` in the same commit (`0b2ea3e`); `EntryRow` mirror in the same commit as Rust (`fe53497`).
- `commands.rs`: only the two entry commands gain `note: String`; still go through `run` (spawn_blocking via `blocking`), no new `log::`.
- TS: `api.ts` remains the only `invoke` importer; Polish messages as planned; hook, `SaveFn`, `EntryForm` (`initialNote`, `useId` counter, no `maxLength`, no client trim), `EntryItem`, `LoadedEntries`, `EntriesPanel` types updated; `WorkTimeView.tsx` and `lib.rs` untouched.
- CSS: `.wt-note` after `.wt-row-error`, `.wt-note-field`/`.wt-note-count` after `.wt-form-fields .dn-field`, tokens only (colour grep empty). `.wt-row` already has `flex-wrap: wrap`, so `flex-basis: 100%` puts the note on its own line; ellipsis works on the blockified flex item with `min-width: 0`.
- Tests: upgrade v10→v11 with closed/running/removed rows and position assertion `== 10`; CHECK counts characters and refuses `\n`/`\r`; CHECK error carries no sentinel; persistence across reopen; note never in the event log (sentinel `LIKE` count 0, add/update add no events); refusals (`Overlap`, `NotFound`) carry no sentinel; table snapshot includes the note; validation order (`NoteMultiline` beats `EndNotAfterStart` and an unparsable time). Crash proof in a new `tests/work_entry_note_crash.rs` reusing `tests/common/crash.rs` with the same `KillAt` list.
- Docs: AGENTS.md line 11 and CHANGELOG `[Unreleased] / ### Added` exactly as planned.

Scope: nothing in the "NOT doing" list was touched (no header/tray/toast/event fields, no `DayReport`/`MonthReport` change, no README/roadmap/PRD edit, no version bump). No EXTRA changes.

Safety: no secrets; only invented data (`Wymyślona notatka`, `SENTINEL-invented-…`); the only `log::` lines in `work_time` are the three pre-existing ones in `commands.rs` (`e.kind()` and the emit error); `Entry` has no `Debug` derive and no log formats entries; SQL uses bound parameters only.

Merge-friendliness with S-01 / S-03: shared-file hunks are small (`commands.rs` 2 hunks in the entry commands, `types.ts` +4, `api.ts` two regions, `WorkTime.css` away from `.wt-days`). Expected friction, already documented in the plan: any new `work_time::add`/`update` call brought by S-01 needs a `""` argument (compiler flags it), a parallel migration forces renumbering v11 and the `== 10` assertion, and CHANGELOG `### Added` under `[Unreleased]` may conflict textually.

## Findings

### F1 — Counter counts the untrimmed value, Rust counts the trimmed one

- **Severity**: OBSERVATION
- **Impact**: LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/WorkTimeEntries.tsx:102
- **Detail**: `charCount(note)` counts leading/trailing spaces, while `normalize_note` trims before counting (`note.rs:15-21`). A 200-character note with surrounding spaces shows e.g. `202/200` yet saves fine. The opposite (counter under 200, Rust refuses) cannot happen, so this is cosmetic. The plan specified "count code points" and "no client trim" for submit, not explicitly for the counter.
- **Fix**: Render `charCount(note.trim())` in the counter (keep sending the untrimmed `note`), or accept as is.
- **Decision**: PENDING

### F2 — Other control characters pass the one-line rule; SQLite `length()` stops at NUL

- **Severity**: OBSERVATION
- **Impact**: LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/work_time/note.rs:16
- **Detail**: Only `\r`/`\n` are refused inside the note. Plan decision 2 accepts U+2028 on purpose, but U+0000, tab, U+0085 (NEL) and other control characters are not discussed and are stored. The CHECK in v11 is only a second line of defence and SQLite `length()` counts characters up to the first NUL, so it would undercount a note with an embedded `\0`; Rust's `chars().count()` still enforces 200, so the limit holds. Practically unreachable from `<input type="text">`, reachable only through a crafted `invoke`.
- **Fix**: Leave as is (Rust is the limit), or in a follow-up refuse `c.is_control()` inside the trimmed note as `NoteMultiline`-like refusal and add a unit test; no migration change needed.
- **Decision**: PENDING

### F3 — Colour and font rules in view CSS

- **Severity**: OBSERVATION
- **Impact**: LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/WorkTime.css:154-161, :215-220
- **Detail**: AGENTS.md says view CSS is layout only; `.wt-note` (`color`) and `.wt-note-count` (`font-family`, `font-variant-numeric`, `color`) add presentation to `WorkTime.css`. Values are tokens only, and the plan (decision 7) chose this deliberately following the existing `.wt-duration` precedent, so this is pre-existing drift the slice follows, not a new break.
- **Fix**: No change in this slice; if the rule is to be enforced, move entry-row presentation into a `dn-` component in a separate change.
- **Decision**: PENDING

## Success criteria

Gates re-run by the reviewer on `HEAD` (`12ebc17`), all green:

| Command | Result |
|---|---|
| `npm run lint` | pass (no output) |
| `npm run format:check` | pass ("All matched files use Prettier code style!") |
| `npm run build` | pass (tsc + vite build) |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass (lib 492 passed; `work_time.rs` 13; `work_entry_note_crash.rs` 1 + ignored writer; all other suites ok) |
| `git grep -n "log::" -- src-tauri/src/work_time` | exactly 3 lines, all in `commands.rs` |
| colour grep on `WorkTime.css` | no match |
| `git grep -n "invoke" -- src/worktime` | only `api.ts` |
| AGENTS.md / CHANGELOG greps (3.4) | new wording found; old sentence gone |

Progress rows: all Automated rows (1.1–1.4, 2.1–2.5, 3.1–3.4) are truthfully checked. Manual rows 1.5 and 2.6–2.11 are correctly left open for the user (they need the real-data procedure from "Ręczne przebiegi dzielą prawdziwą bazę": back up the app data dir before the first v11 run, restore it afterwards). Nit: the Progress convention asks for ` — <commit sha>` on landed rows; none are appended (`fe53497`, `0b2ea3e`, `12ebc17`).
