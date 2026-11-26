<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Polish public holidays in the work-time norm

- **Plan**: context/changes/public-holidays-norm/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4, 5, 6, 7
- **Date**: 2026-11-26
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 3 observations
- **Change range**: `455cbed..0b0e79a` (phase commits 0466d77, 12439c1, 794268f, 364bbae, 967cc2d, 1a0e672, 2a9a1dc)

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS (1 observation) |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS (1 observation) |
| Success Criteria | PASS (1 observation; manual rows pending for the user) |

## What was checked

**Holiday rules.** `holidays.rs` implements Meeus/Jones/Butcher exactly as the plan's reference (integer division, `None` before 1583, `from_ymd_opt`/`checked_add_days`, no panic path). The calendar has all 14 days with the plan's names; 6.01 is gated at `>= 2011` and 24.12 at `>= 2025`. Hand check: November 2026 = 21 weekdays − 11.11 = 20 → 160 h; December 2026 = 23 weekdays − 24.12 − 25.12 = 21 → 168 h (26.12 is a Saturday: by Kodeks pracy art. 130 §2 the statutory norm would be 160 h, but per user decision U1 the exchange day stays manual and the row shows the hint, so 168 h is the intended app figure). Both are pinned by `november_2026_has_a_160_hour_norm_and_december_168`. I also checked whether Wigilia on a Saturday has a special exemption from the "day off owed" rule. Current sources (PIP, labour-law portals as of Nov 2026) treat it like any other Saturday holiday, so `day_off_owed = holiday && Saturday` is correct.

**Classification and guard.** There is one `classify` (weekend > holiday > dayOff > workday), used by both `compute` and `day_kind`. `is_workday` now goes through `day_kind`, so a third copy cannot drift. `set_day_off` checks the weekend first (`NotAWorkday`), then the holiday (`Holiday`). Both checks run before `store.write`, in both directions, and a test pins "no row written". `MonthReport.days_off` filters out rows shadowed by a holiday. Under `Off` they come back (tested).

**Settings policy.** `load_work_time_settings` mirrors the reminders loader:
- it uses `deny_unknown_fields`;
- an absent or `null` block or field means the defaults, and so does a missing file;
- a BOM is stripped;
- `version` must be 1;
- only `"pl"` and `false` are accepted, `true`, `"PL"`, numbers, arrays and objects are refused;
- the error carries no detail.

Strict: `work_month_report` and `set_day_off` return `SettingsInvalid` (a surfaced error, never an empty list). Lenient: `work_timer_status` (header, START/STOP), `list_work_entries` and the reminder round fall back to the defaults with a debug log by kind only. The settings are read inside `blocking` but before `with_store` in `execute`, and before `with_store` in `reminder_round`, so the store lock is never held during the file read. Store access stays `StoreHandle::with_store` in `spawn_blocking` (via `blocking`).

**Contracts.**
- `DayKind::Holiday` serializes as `"holiday"` (camelCase), mirrored in `types.ts`.
- `DayReport.holiday` / `dayOffOwed` are mirrored as `string | null` / `boolean`.
- `WorkTimeError::{Holiday, SettingsInvalid}` serialize as `{"kind":"…"}` under the existing tag/content enum, mirrored in the TS union and in `workTimeErrorMessage`.
- Each Rust type change and its TS mirror landed in the same commit (p3, p4, p5).

**UI.**
- Holiday rows are hatched.
- The switch shows only for `workday` and `dayOff`, in both the table and "Wybrany dzień".
- The bar shows on a holiday only when hours were worked (R6).
- The hint text, the tooltip and the "Jak liczę" paragraphs match the plan word for word.
- New CSS is layout only, with tokens only (the colour grep prints nothing).
- `wt-days-note` / `wt-days-holiday` follow the existing view-level convention in `WorkTime.css` (`wt-days-num`, `wt-days-switch`, and `.wt-days .dn-hours` as the precedent for styling a `dn-` component from view CSS). `dn-` is for reusable components in `src/components/`, and no new reusable component was added, so the prefix is correct.

**Docs.** The README key name, the values `"pl"`/`false`, the default and the invalid-block exception all match the code. PRD lines 155, 156, 176, 211 and 226 changed as planned. The FR id count is unchanged (26 → 26), and the Non-Goal keeps the rest of its text.

**Parallel-slice footprint.**
- `lib.rs`, capabilities, `tauri.conf.json` and `Cargo.toml` are unchanged.
- `settings.rs` changed by exactly +2 lines.
- `WorkTimeView.tsx` has only the switch gate and two `HowCounted` paragraphs.
- README has 1 bullet, 1 JSON line, 1 table row and 1 sentence.
- `work_reminders.rs` re-wraps one `use crate::settings::{…}` import. This is a small merge-conflict surface, but it is needed.

**Hard rules.** No secrets. Tests use invented data only. No new CLI calls, no new files outside the store. Errors carry no values or paths.

## Gate results (run by the reviewer on 0b0e79a)

| Gate | Result |
|---|---|
| `npm run lint` | pass |
| `npm run format:check` | pass |
| `npm run build` | pass (tsc + Vite) |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass (lib: 517 passed; all integration binaries green, 0 failed) |
| `git diff --quiet 455cbed..HEAD -- src-tauri/src/lib.rs` | pass |
| Literal-colour grep on changed CSS | prints nothing |
| PRD greps (Non-Goal, FR count) | pass |

Manual rows 5.4, 5.5, 6.4–6.9 and 7.5 are unchecked and pending for the user, as the plan intends. None of them is ticked without evidence.

## Findings

### F1 — Saturday-holiday tooltip hides the holiday name

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/worktime/DaysTable.tsx:46
- **Detail**: On a Saturday holiday the cell text is `"<name> · należy się inny dzień wolny"`, truncated with an ellipsis, and the `title` is only the explanation sentence. On "Drugi dzień Bożego Narodzenia" at narrow widths the name can be cut, and the tooltip then does not carry it, so nothing on screen shows the full name. The plan specified that exact title, so this is a plan-level gap, not drift. The plan's own "truncated, with the full text in `title`" intent is better served by including the name.
- **Fix**: Set the Saturday branch of `holidayTitle` to `` `${row.holiday} — Święto w sobotę: pracodawca wyznacza za nie inny dzień wolny — oznacz go w tabeli` ``.
- **Decision**: PENDING

### F2 — Progress rows for phases 2, 4, 5 (and 3.5) lack commit SHAs

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/public-holidays-norm/plan.md:722-750
- **Detail**: The Progress convention says to append ` — <sha>` when a step lands. Rows 2.1–2.3, 3.5, 4.1–4.3 and 5.1–5.3 are `[x]` with no SHA, while phases 1, 3 (except 3.5), 6 and 7 have one. The commits exist: p2 = 12439c1, p3 = 794268f, p4 = 364bbae, p5 = 967cc2d.
- **Fix**: Append ` — 12439c1` to 2.1–2.3, ` — 794268f` to 3.5, ` — 364bbae` to 4.1–4.3 and ` — 967cc2d` to 5.1–5.3.
- **Decision**: PENDING

### F3 — Cosmetic nits in the touched shared files

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/work_reminders.rs:63, src-tauri/src/settings.rs:17, README.md:121
- **Detail**:
  - (a) `work_time_settings(…, Policy::Lenient).unwrap_or_else(|_| WorkTimeSettings::default())`: Lenient never returns `Err`, so the closure is dead code. It is harmless and defensive, and `unwrap_or_default()` says the same thing more briefly.
  - (b) The new `pub use work_time::{…};` sits directly on `const SUPPORTED_VERSION` with no blank line. rustfmt accepts it, but it reads as one block.
  - (c) The new README table row is wider than the column padding of the other rows. It renders correctly, but the source alignment is uneven, and Prettier is not run on README.

  None of these affects behaviour. If they are touched at all, keep the edits to these exact lines, because S-03/S-06 edit the same files.
- **Fix**: (a) use `.unwrap_or_default()`; (b) add one blank line after line 17 of `settings.rs`; (c) optionally re-pad the README table, or leave it to avoid a parallel-slice conflict.
- **Decision**: PENDING
