---
date: 2026-11-26T14:16:44+01:00
researcher: Claude (agent, for Sebastian Kostecki)
git_commit: d34904a4dc9f9400c8b386cc935a845b7ed6762b
branch: slice/month-report-copy
repository: day-navigator
topic: "Copy the month work-time report to the clipboard as TSV (S-05, MS-24, issue #24)"
tags: [research, work-time, month-report, clipboard, tsv, tauri-2]
status: complete
last_updated: 2026-11-26
last_updated_by: Claude (agent)
---

# Research: Copy the month work-time report to the clipboard as TSV

**Date**: 2026-11-26T14:16:44+01:00
**Researcher**: Claude (agent, for Sebastian Kostecki)
**Git Commit**: d34904a4dc9f9400c8b386cc935a845b7ed6762b
**Branch**: slice/month-report-copy (cut from `develop`, M-3 wave 1 merged)
**Repository**: day-navigator

## Research Question

Roadmap slice S-05 (`month-report-copy`, MS-24, issue #24): in "Czas pracy" a "Kopiuj miesiąc" button copies the month report (days, entries with their notes, sums, deviation, days off, holidays with names) to the clipboard as TSV that pastes into a spreadsheet. The user has already decided: TSV; clipboard only (no file); text built in Rust from the same month report the view shows, unit-tested on invented data; UI only triggers the copy and shows a confirmation or the error; per-entry note and holiday name included; anchored as MS-24, no new FRs. The research answers six questions: (1) the report's types and commands, (2) how to guarantee "same as the screen", (3) the clipboard mechanism, (4) spreadsheet-friendly formatting, (5) patterns to reuse, (6) merge risk with the parallel slices S-02, S-04, S-07 and S-08.

## Summary

1. **The report** is `MonthReport` / `DayReport` built by `report::compute` (`src-tauri/src/work_time/report.rs:122-183`) and served by the one command `work_month_report(day)` (`commands.rs:213-223`). It carries per-day sums, norms, deviations, kind and holiday name, but **no entries and no notes**: per day only `entry_count` (`report.rs:59-60`). Entries with notes come from `list_work_entries(day)` one day at a time (`commands.rs:124-134`, `mod.rs:218-257`). `compute` already loads every counted entry of the month, note included, in the same `store.read` (`report.rs:138`, `mod.rs:95-100,131-153`), so a copy builder can get the report and the entries from one snapshot without a new query shape.
2. **"Same as the screen"**: recompute in Rust at click time for the month the view shows (`day.slice(0, 7)` + `-01`, as `useMonthReport` does at `useMonthReport.ts:20,39`), through the same `compute` and the same `Policy::Strict` settings read. Do not pass the report back from the UI: it has no entries or notes, it would be a second, unvalidated input contract, and the screen's today row is not in it anyway (it is overlaid live in the UI, `DaysTable.tsx:13-26`).
3. **Clipboard**: least surface is `navigator.clipboard.writeText` in the click handler after `await invoke(...)` returns the text. It needs no crate, npm package, capability or CSP change. `http://tauri.localhost` is a secure context (`.localhost` hosts count as potentially trustworthy, per MDN), and WebView2 has no permission kind for clipboard writes. Fallback if the built-app check fails: `tauri-plugin-clipboard-manager` 2.5.0 used from Rust only (`app.clipboard().write_text`), which needs no npm package and no permissions, like `window-state`.
4. **Formatting** (agent default, user may revise): ISO dates `YYYY-MM-DD`. Every non-negative duration goes in two columns: `H:MM` (Excel reads it as a time or duration) and decimal hours with a comma, e.g. `7,50` (a number in pl-PL). Deviations go only as decimal hours with an ASCII minus (`-1,50`) and no `+`. A missing value is an empty cell. Values are rounded to the minute exactly as the screen does: unsigned values are floored, signed values are truncated toward zero (`src/components/time.ts:79-90`). In cell text (notes, holiday names), tab and control characters become a space. A cell that starts with `= + - @ " '` gets a leading `'`, so it is never a formula or a quoted field.
5. **Reuse**: the `execute`/`run_with_settings` command wrapper, which logs only the command name and the error kind (`commands.rs:80-106`); `WorkTimeError` and its TS mirror and Polish messages (`error.rs:10-42`, `types.ts:7-23`, `api.ts:120-161`); `Button` in the view head after a `dn-spacer` (pattern `GithubView.tsx:182-195`); `Notice tone="info" compact` inside an always-mounted `role="status"` for "Skopiowano." (pattern `OwnTaskEditPanel.tsx:134-141`); `Notice tone="error" compact` for a failure. The copy must not record an event and must not log the text.
6. **Merge risk**: the new logic goes in new files (`work_time/month_text.rs` + `month_text_tests.rs`, following the S-03 `today.rs` / `today_tests.rs` precedent). Shared files get only one-line or append-only edits: `mod.rs` (two `mod` lines), `commands.rs` (one appended command), `lib.rs` (one handler line), `api.ts` (one function), `WorkTimeView.tsx` (one button and one notice), `CHANGELOG.md`, `README.md`. The builder reads the report's own fields (`norm_ms`, `kind`, `deviation_ms`, `holiday`) and never `DAILY_NORM_MS`, and maps `DayKind` with an exhaustive `match`, so S-02's half day off or configurable norm arrives either automatically or as a compile error at merge. **No migration**: the slice only reads existing tables (`work_entries.note` v11, `days_off`).

## Detailed Findings

### 1. The month report: types, command, view

**Rust types** (`src-tauri/src/work_time/report.rs`):
- `DayKind { Workday, DayOff, Holiday, Weekend }`, serialized camelCase (`report.rs:23-30`). Classification rule: a weekend wins over a holiday, a weekday holiday wins over a marked day off (`report.rs:98-111`).
- `MonthReport` (`report.rs:32-47`): `month` (`YYYY-MM`), `phase` (`past|current|future`, `norm.rs:79-85`), `as_of_ms`, `worked_ms`, the flattened `NormFigures`, `days_off: Vec<String>` (this month's marked days, with days shadowed by a holiday left out, `report.rs:42-44,176-180`), `days: Vec<DayReport>` (every local day of the month, ascending).
- `NormFigures` (`norm.rs:101-111`): `norm_ms`, `norm_to_date_ms`, `deviation_ms`, `workdays`, `workdays_after_today`, `needed_per_day_ms`, `remaining_ms`. All options are `None` in a future month. A past month sets `norm_to_date_ms` and `deviation_ms` (against the full norm). The current month sets everything, and `needed_per_day_ms` only when a workday remains after today (`norm.rs:115-151`).
- `DayReport` (`report.rs:49-65`): `day`, `kind`, `worked_ms`, `norm_ms`, `deviation_ms: Option` (`None` for a day after today or a day with neither norm nor hours, `norm.rs:61-74`), `entry_count`, `holiday: Option<String>` (the name, on any kind of day), `day_off_owed` (a Saturday holiday).
- Norm per day: a fixed `DAILY_NORM_MS` = 8 h (`norm.rs:13-14`), 0 on a weekend, day off or holiday (`norm.rs:46-59`).

**Entries and notes** (`src-tauri/src/work_time/mod.rs`):
- The private `Entry { id, start, end, note }` (`mod.rs:95-100`) is filled by one mapper for all three SELECTs (`mod.rs:102-109`). `in_day(conn, start, end, now)` returns the counted entries that overlap a range, sorted by start (`mod.rs:131-153`).
- `report::compute` calls `in_day` over the whole month (`report.rs:138`), then splits each entry into per-day portions at local midnight (`report.rs:145-155`, `rules.rs:57-63`). The entries are used and then dropped. They are not in the output.
- The UI type `EntryRow` (`mod.rs:66-77`): `start_local` / `end_local` (`YYYY-MM-DDTHH:MM`, `rules.rs:8-9,24-31`), `day_portion_ms`, `note`. It is built only by `entries_for_day` (`mod.rs:218-257`).
- Note invariants: `normalize_note` trims, refuses CR/LF and limits a note to 200 characters (`note.rs:10-23`). **A tab inside a note is allowed** (only `\r` and `\n` are refused at `note.rs:16`), so the TSV builder must sanitize it.

**Command**: `work_month_report(app, day)` parses `day` with `parse_day` (`report.rs:196-206`) and runs `report::month_report` under `run_with_settings(.., Policy::Strict, ..)` (`commands.rs:213-223`). An invalid `workTime` block therefore fails it with `SettingsInvalid` (`commands.rs:36-53`). It is registered in `lib.rs:220`.

**View** (`src/worktime/`):
- `WorkTimeView` holds `day` (`WorkTimeView.tsx:89`), calls `useMonthReport(day)` (`:92`), and renders the head with the month switch (`:173-196`), `MonthReport` tiles (`:197`) and `DaysTable` (`:132-138`).
- `useMonthReport` keys on `month = day.slice(0, 7)` and fetches `workMonthReport(`${month}-01`)` (`useMonthReport.ts:19-20,39`). It refreshes on `work-time-changed`, on focus and visibility, and every 60 s except for a loaded past month (`useMonthReport.ts:49-112`). Its state is `loading | error(message, lastOkAt) | loaded(report)` (`useMonthReport.ts:11-14`).
- Screen formatting:
  - `formatHm`: unsigned values floored to the minute (`format.ts:22-24`, same rule in `components/time.ts:79-90`).
  - `formatSigned`: rounded toward zero, written `+H:MM` / `−H:MM` with U+2212, plain `0:00` under a minute (`format.ts:26-31`).
  - Day label `DD.MM wd` (`format.ts:45-51`).
- Days-table display rules:
  - The Suma cell shows `—` unless `workedMs > 0 || (day <= today && normMs > 0)` (`DaysTable.tsx:44`).
  - The deviation cell shows `—` for `null` (`DaysTable.tsx:104`).
  - The holiday name shows in the bar cell, with "należy się inny dzień wolny" on a Saturday holiday (`DaysTable.tsx:82-86`).
- Month tiles: "Miesiąc" (`workedMs` z `normMs`), "Odchylenie", "Średnio / dzień", "Dni wolne" (= `daysOff.length`, marked manually) (`MonthReport.tsx:8-49`). The tiles use the report snapshot and are not live. Only the "Dziś" tile and the today row in the table follow the live counter (`WorkTimeView.tsx:44-58,135`, `DaysTable.tsx:13-26`).
- TS mirror: `types.ts:1-142` (`MonthReport`, `DayReport`, `WorkTimeError` union at `types.ts:7-23`).

### 2. Guaranteeing "same report as the screen"

Inputs the screen uses: the month of `day`, `now` at fetch time, `chrono::Local`, `WorkTimeSettings` read under `Policy::Strict` (`commands.rs:214-221`, `commands.rs:80-94`).

**Option A: recompute in Rust (recommended).** Add a command `work_month_report_text(day)` that runs under the same `run_with_settings(.., Policy::Strict, ..)`. Inside one `store.read` it calls `report::compute(c, day, now, &Local, settings)` and `in_day(c, month_start, month_end, now)`, then formats. `compute` is `pub(super)` (`report.rs:122`), and `in_day` is private to `work_time`, but a child module can see it (`report.rs:15` already does `use super::in_day`). So a new `work_time/month_text.rs` can do this without widening visibility.
- Pros:
  - One contract (the `day` string the view already sends) and one rule set.
  - The figures are the report's own, so they cannot drift.
  - Entries and notes come from the same snapshot.
  - The today row equals the live screen value at click time: the screen's today figure is `todayClosedMs + running span` (`total.ts`), which matches `compute`'s portion up to `now`, as `today_tests.rs:31-43` proves for the header.
- Cons:
  - Between the last refresh (60 s at most, or the last `work-time-changed`) and the click, the copy can be newer than the non-live month tiles. That happens only while the counter runs, or right after a change in another window or the tray, and the copy is then the fresher truth.
  - A failure at copy time is possible even though the screen shows a loaded report. It surfaces as an error notice.

**Option B: the UI passes the shown `MonthReport` back.**
- Rejected: the report has no entries and no notes (`report.rs:49-65`), so Rust would still have to read the store.
- The webview would become a source of numbers that Rust formats without validating them: a second, wider input contract.
- The live today row is a UI overlay and is not in the object anyway.

**Guards** (agent default): enable "Kopiuj miesiąc" only while `report.state === "loaded"` for the shown month. A failed source then never yields an empty or stale copy, in the spirit of the "never render an empty list in place of a failed source" rule (AGENTS.md Hard Rules). Pass `${month}-01` exactly as `useMonthReport` does. A Rust test asserts that the builder's numbers equal the `MonthReport` fields for invented data, with entries across midnight, a running entry, a holiday, a day off and a note with a tab.

### 3. Clipboard in Tauri 2 on Windows

Repo versions: `tauri` 2.12.7 (`src-tauri/Cargo.lock:3920-3921`, `Cargo.toml:41`), `@tauri-apps/api` 2.12.5 (`package-lock.json:934-935`).
- CSP: `default-src 'self'; connect-src ipc: http://ipc.localhost; …` with `freezePrototype: true` (`tauri.conf.json:26-27`).
- Capabilities: event listen/unlisten and `opener:allow-open-url` only (`capabilities/default.json`).
- Plugins: opener, notification, single-instance, window-state, log (`Cargo.toml:42-50`). No clipboard plugin.

**`navigator.clipboard.writeText` (recommended)** (web sources, delegated web check):
- Secure context: MDN treats any `*.localhost` host as potentially trustworthy whatever the scheme, so `http://tauri.localhost` (built, `useHttpsScheme` off) and `http://localhost:1420` (dev) both expose `navigator.clipboard` (https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts , https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/writeText).
- Gesture: in Chromium, a write is allowed when the page is focused: `clipboard-write` is granted to the active page, otherwise transient activation is required (https://web.dev/articles/async-clipboard , MDN Clipboard API security). So click → `await invoke` → `writeText` should work in a focused window.
  - **Uncertain**: whether activation is still needed after a long `invoke`. Chromium keeps activation for about 5 s, a figure from memory with no primary source found. The command reads one month and should take well under that.
  - Tray-triggered or unfocused calls would likely fail with `NotAllowedError`. Not in scope: the button is in the view.
- WebView2: `CoreWebView2PermissionKind` has `ClipboardRead` but no write kind, so a write never prompts (https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2permissionkind).
- CSP and `freezePrototype` do not govern the Clipboard API.
- No Tauri v2 issue was found for `writeText` on Windows. One Microsoft Q&A thread reports failures when a write fires without a gesture or focus, via `ExecuteScriptAsync` (https://learn.microsoft.com/en-us/answers/questions/877299/).
- **Must be checked once in the built app** (`npm run tauri build -- --debug --no-bundle`), as AGENTS.md requires for webview-security behaviour: the paste works, and DevTools shows no CSP violation.

**`tauri-plugin-clipboard-manager` (fallback)** (https://v2.tauri.app/plugin/clipboard/ , https://docs.rs/crate/tauri-plugin-clipboard-manager/latest):
- 2.5.0 depends on `tauri ^2.12` and `arboard ^3.6`.
- Rust-only use: add the crate, register `.plugin(tauri_plugin_clipboard_manager::init())`, call `app.clipboard().write_text(text)` via `ClipboardExt`. Capabilities gate only webview IPC, so this needs no npm package and no permission. This matches the AGENTS.md rule for a Rust-only plugin ("A plugin used only from Rust gets no npm package and no permissions (`window-state`)").
- JS use would need `clipboard-manager:allow-write-text`.
- The plugin keeps one `arboard::Clipboard` behind a `Mutex` and drops it on exit. Only the read calls warn against running on the main thread (https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/clipboard-manager/src/desktop.rs), so `write_text` from the blocking pool looks fine.
- Cost: two new crates (plugin and arboard) under `cargo audit`. Benefit: the whole copy, text and write, sits in Rust and does not depend on webview focus.
- Using `arboard` or `clipboard-win` directly would re-implement what the plugin does, including the single-opener contention and drop-before-exit handling (https://docs.rs/arboard/latest/arboard/struct.Clipboard.html). Not recommended.

**Recommendation**: Option `navigator.clipboard.writeText`. The text travels over IPC exactly like the entries list already does (`list_work_entries` returns notes). No new dependency, permission or policy. Await the write and show an error notice on rejection; never fail silently. The fallback is decided only if the built-app check fails.

**Testing**: the builder is a pure function, unit-tested in Rust. The write itself and the JS path have no test runner (AGENTS.md: "The UI has no test runner"), so they need a manual check in the built app.

### 4. Spreadsheet-friendly TSV (agent defaults, user may revise)

Evidence (delegated web check; much of it is documented behaviour or forum reports, so it is marked as inference in the source report and nothing was pasted into a live Excel or Sheets):
- **Paste parsing**: Excel splits plain-text paste on tab and CR/LF, then reads each cell as if typed, using the Windows regional settings. Sheets uses the spreadsheet locale (https://support.google.com/docs/answer/58515).
- **Durations**:
  - Excel: `7:30` becomes a time. `40:15` becomes 1.677 days shown as `40:15` via an auto `[h]:mm` (MS KB 214386). `-1:30` stays text, since Excel has no negative times in the 1900 date system.
  - Sheets: `7:30` becomes a time of day. `40:15` probably becomes a duration (unverified). Negative times are unreliable.
- **Decimals** (pl-PL):
  - `7,5` becomes the number 7.5.
  - `7.5` becomes a date (7 May) in Excel (https://www.elektroda.com/rtvforum/topic3563169.html) and is never 7.5. Never emit a decimal point.
  - `-1,25` is a number. Avoid a leading `+`, which is a formula trigger.
- **Dates**: ISO `2026-11-03` is recognised by both apps in practically every locale. `03.11.2026` only in pl-PL.
- **Text cells**:
  - An unquoted tab splits a cell, and CR/LF starts a row.
  - A cell starting with `"` is read as a quoted field and can swallow later cells.
  - Cells starting with `= + - @` (and tab, CR, LF, full-width `＝＋－＠`) can be evaluated as formulas (https://community.owasp.org/attacks/CSV_Injection).
  - A leading `'` makes the cell text. Excel hides the apostrophe; Sheets likely also, but this is unverified.

**Recommended layout** (default):
- **Line endings and encoding**: LF line endings and no BOM. Both apps accept LF. **Uncertain**: whether WebView2 converts LF to CRLF on write. Check in the manual paste.
- **Block 1: month summary**, rows of `label<TAB>H:MM<TAB>hours`:
  - month (`2026-11`), the phase as a word, the "as of" local time;
  - Przepracowano, Norma miesiąca, Norma do dziś (when set), Odchylenie (hours only, signed decimal), Dni robocze, Dni robocze po dziś, Średnio / dzień (when set), Pozostało (when set);
  - Dni wolne (`days_off.len()`, as the tile shows).
  Fields that are `None` for the phase are omitted or left as empty cells, mirroring which tiles `MonthReport.tsx:8-49` shows.
- **Block 2: days table**, one row per day, a rectangle whose `SUM` equals the month. Columns: Data (`YYYY-MM-DD`), Dzień (`pn`…`nd`, `format.ts:43`), Rodzaj (`roboczy | wolny | święto | weekend`, mapped by exhaustive match), Święto (name, with a "należy się inny dzień wolny" suffix when `day_off_owed`), Wpisy (count), Suma `H:MM`, Suma h, Norma h, Odchylenie h (empty when `None`).
- **Block 3: entries table**, one row per entry portion per day (an entry over midnight appears under each day with its portion, as `compute` counts it). Columns: Data, Od (`HH:MM`, or `DD.MM HH:MM` when outside the day, as `WorkTimeEntries.tsx:21-26`), Do (or `trwa` for the running entry), Czas `H:MM`, Czas h, Notatka.
- Blank lines separate the blocks. Each block has a header row.
- **Numbers**:
  - Minutes first: unsigned values floored (`time.ts:79-90`), signed values truncated toward zero (`format.ts:26-31`). Then `H:MM` or `hours = minutes / 60` with two decimals and a decimal comma.
  - Caveat: `7:20` is `7,33`, so a column of two-decimal hours may not add up exactly to the month figure. The `H:MM` columns are exact.
  - A deviation is written only as decimal hours with an ASCII `-` and no `+`, so it stays a number. A screen-like `−1:30` text column is possible but is left out by default.
- **Cell sanitizer**, one function used for notes and holiday names:
  1. Replace tab and every control character with a space, then trim.
  2. If the first character is one of `= + - @ " '` or a full-width `＝＋－＠`, prefix `'`.
  3. A mid-text `"` stays.
- **Locale**: the decimal comma is right for pl-PL Excel and Sheets. It would not parse in an en-US sheet. Document "pl-PL" in the README and the CHANGELOG line.

### 5. Existing patterns to reuse

- **Command wrapper**: `run_with_settings` → `execute` reads settings before the store lock, runs on the blocking pool and logs only `name` and `e.kind()` (`commands.rs:68-106`). The commands module doc says errors are logged by kind only (`commands.rs:6-7`). The new command must not call `notify` (a read, not a write; compare `work_month_report` at `commands.rs:213-223`) and must add no `log::` line with content. Precedent check: S-06 counted `log::` lines in `work_time` (`context/archive/2026-11-26-work-entry-note/plan.md:225`).
- **Errors**: reuse `WorkTimeError` (`error.rs:10-42`). Its invariant is that no variant carries times or row values (`error.rs:3-4`). No new variant is needed for building the text. A clipboard rejection is a JS-side error and needs a Polish message in the UI only (e.g. "Nie udało się skopiować do schowka").
- **TS contract**: the command returns `String`, so no new TS type. One `api.ts` function, `workMonthReportText(day): Promise<string>` (pattern `api.ts:50-53`). `api.ts` is the only file that imports `invoke` (AGENTS.md Frontend Conventions). Messages reuse `workTimeErrorMessage` (`api.ts:120-161`).
- **UI**:
  - Button: `Button` (`variant`, `size`, `icon`; `Button.tsx:7-12`). Its `IconName` set has no copy icon (`Icon.tsx:3-21`), so use a text button or add a drawn icon (a design-system change; keep it text by default).
  - Placement: in the view head after `<span className="dn-spacer" />`, like the refresh button in `GithubView.tsx:182-195`.
  - Confirmation: an always-mounted `role="status"` holding `Notice tone="info" compact role={null}` "Skopiowano." (`OwnTaskEditPanel.tsx:134-141`). This is the only "done" confirmation pattern found in `src/` (grep `tone="info"`). There is no toast component in the UI; "toast" in Rust means Windows notifications (`gitlab_refresh.rs:56`).
  - Error: `Notice tone="error" compact`, as for the day-off refusal (`WorkTimeView.tsx:139-145`).
  - Styles: tokens only (`src/styles/tokens.css`). View CSS is layout only (`WorkTime.css:1`).
- **Event log and app log**: `NewEvent` has no copy variant (`events/mod.rs:72-84`), and none must be added. The note never reaches the app log or the event log (AGENTS.md Hard Rules, `note.rs:1-4`). Tests can reuse the S-06 sentinel approach: a sentinel note appears in the TSV but in no error JSON (`note.rs:63-73`, `archive/2026-11-26-work-entry-note/plan.md:197`).
- **Test fixture**: `today_tests.rs:1-43`: a `chrono_tz::Europe::Warsaw` zone, `resolve_local` instants, `Store::open(tempdir, MIGRATIONS, day)`, invented data.

### 6. Parallel slices and merge risk

Active worktrees, all at `d34904a`: `configurable-daily-norm` (S-02), `end-of-day-summary` (S-07), `week-summary` (S-08), `windows-autostart` (S-04), `month-report-copy` (`git worktree list`). Their plans were not read: this run is scoped to its own worktree. Expectations come from `context/foundation/roadmap.md:106-120,134-146,175-198`.

| Shared file | S-05 edit | Who else likely touches it | Keep it small by |
|---|---|---|---|
| `work_time/report.rs` | none (read `compute`, the fields) | S-02 (half day off, per-day norm), S-08 (week figures) | never editing it; the builder lives in `month_text.rs` |
| `work_time/norm.rs` | none | S-02 (`DAILY_NORM_MS` becomes a setting), S-08 | reading `norm_ms` from `DayReport` / `NormFigures`, never the constant |
| `work_time/mod.rs` | `pub mod month_text;` + `#[cfg(test)] mod month_text_tests;` | S-02, S-08 may add modules | two lines in the existing sorted lists (`mod.rs:11-30`) |
| `work_time/commands.rs` | one appended command | S-02 (settings), S-07 (STOP hook at `commands.rs:148-156`), S-08 | appending after `set_day_off`; no change to `execute` |
| `lib.rs` | one line in `generate_handler!` (`lib.rs:213-221`) | S-04 (autostart plugin), S-07, S-08 | one line after `work_month_report` |
| `WorkTimeView.tsx` | one button in `dn-view-head`, one status/notice | S-02 (half day off control), S-08 (week section) | putting the copy state in a new `useMonthCopy.ts` and the button in a small `MonthCopyButton.tsx` |
| `api.ts` | one function | S-02, S-08 | appending |
| `types.ts` | none (returns `string`) | S-02, S-08 | — |
| `CHANGELOG.md` / `README.md` | one `[Unreleased] / Added` bullet; one README "Czas pracy" bullet (`README.md:53-60`) | all | a separate bullet, no rewording of others' lines |

**Making new fields easy to add at merge**:
- The builder takes `&MonthReport` plus the per-day entries and formats only what the report says: `kind`, `norm_ms`, `deviation_ms`, `holiday`, `day_off_owed` for days, and the `NormFigures` fields for the summary.
- An exhaustive `match DayKind` (no `_` arm) means S-02's possible new kind (e.g. a half day off) fails to compile until a label is chosen, instead of silently printing a wrong word.
- A per-day norm from S-02 flows through `norm_ms` unchanged.
- The tests should build expected values from the report fields where possible (e.g. "the sum of days' Suma equals the month's `worked_ms` floored"), so a norm-rule change in S-02 does not break this slice's tests.

**Migration**: none. The copy reads `work_entries` (with `note`, migration v11 per `archive/2026-11-26-work-entry-note/plan.md:82`) and `days_off`, both already shipped. It writes nothing to the store or the file system. No AGENTS.md write-rule change is needed, because the clipboard is not a folder (roadmap unknown #3 at `roadmap.md:218` was only about the file option, which the user ruled out).

## Code References

- `src-tauri/src/work_time/report.rs:23-65` - `DayKind`, `MonthReport`, `DayReport`
- `src-tauri/src/work_time/report.rs:122-183` - `compute`: one-snapshot report, month entries loaded at `:138`
- `src-tauri/src/work_time/norm.rs:13-14,46-74,101-151` - fixed 8 h norm, day norm and deviation, `NormFigures`
- `src-tauri/src/work_time/mod.rs:66-77,95-153,218-257` - `EntryRow`, `Entry` with note, `in_day`, `entries_for_day`
- `src-tauri/src/work_time/note.rs:10-23` - note normalization (tabs allowed)
- `src-tauri/src/work_time/commands.rs:68-106,213-223` - wrapper with kind-only logging; `work_month_report`
- `src-tauri/src/work_time/error.rs:10-42` - `WorkTimeError`
- `src-tauri/src/work_time/today.rs`, `today_tests.rs:1-43` - new-file module precedent and test fixture
- `src-tauri/src/lib.rs:213-221` - work-time handler registration
- `src-tauri/tauri.conf.json:26-27`, `src-tauri/capabilities/default.json` - CSP, freezePrototype, permissions
- `src/worktime/WorkTimeView.tsx:89-92,132-145,173-197` - day state, report hook, head, notices
- `src/worktime/useMonthReport.ts:19-120` - month key, refresh policy, states
- `src/worktime/DaysTable.tsx:13-26,44,82-86,104` - live today overlay, display rules
- `src/worktime/MonthReport.tsx:8-49` - month tiles per phase
- `src/worktime/format.ts:22-51`, `src/components/time.ts:79-90` - rounding and labels the TSV must mirror
- `src/worktime/api.ts:50-53,120-161` - `workMonthReport`, error messages
- `src/worktime/WorkTimeEntries.tsx:21-26` - `HH:MM` / `DD.MM HH:MM` entry labels
- `src/own-tasks/OwnTaskEditPanel.tsx:134-141` - "Zapisano." status pattern
- `src/github/GithubView.tsx:182-195` - view-head action button after `dn-spacer`

## Architecture Insights

- Domain rules live in Rust, and the UI only displays (AGENTS.md Frontend Conventions). The TSV builder is a pure formatter over `MonthReport` plus entries, in its own module with no `tauri` import (the `today.rs` pattern). The command stays a thin wrapper in `commands.rs`, the only `work_time` file that imports `tauri` (`commands.rs:1-2`).
- "Same numbers" holds by construction: the same `compute` and settings policy, plus the same rounding as `formatHm` / `formatSigned`, implemented once in Rust with a comment pointing at `time.ts` / `format.ts`.
- The copy is a read. It does not emit `work-time-changed`, write the event log or touch the store's write path.

## Historical Context (from prior changes)

- `context/archive/2026-11-26-work-entry-note/plan.md:5,33,44`: the note was put in the shared `Entry` mapper so that `report::compute` can read it "pod przyszły S-05", and was kept out of `DayReport` / `MonthReport` on purpose, as S-05's scope. Supported: `report.rs:138` loads notes, and `report.rs:49-65` has none.
- `context/archive/2026-11-26-work-entry-note/plan.md:28,43,197,225`: the note never reaches the app log or the event log, with sentinel tests and a `log::` line count. Still in force (AGENTS.md Hard Rules).
- `context/foundation/roadmap.md:70`: S-05 and S-06 share the "Work time" view, and "if S-06 lands first, S-05 copies the note too". S-06 is done (`roadmap.md:173`), so the note is in scope.
- `context/foundation/roadmap.md:157-160`: the clipboard-vs-file and format unknowns are now settled by the user (clipboard only, TSV).
- `context/archive/2026-11-26-public-holidays-norm/`: holidays shadow marked days off in `days_off` and in classification (`report.rs:42-44,98-111`). The TSV inherits this by reading the report.

## Related Research

- `context/archive/2026-11-26-work-entry-note/research.md`
- `context/archive/2026-11-26-public-holidays-norm/research.md`
- `context/archive/2026-11-24-worktime-days-table/` (days table origin)

## Open Questions

None block the plan. The plan records these as "default chosen by agent, user may revise":
1. **Layout**: three blocks (summary, days, entries), or one flat entry table with day columns repeated.
2. **Deviation columns**: decimal hours only (default), or also a screen-like `−1:30` text column.
3. **Decimal precision**: two decimals (default), or a minutes column for exact sums.
4. **Fallback clipboard plugin** (Rust-only `tauri-plugin-clipboard-manager`): only if the built-app check of `navigator.clipboard.writeText` fails.

Verify once by hand, since it cannot be automated here: paste the copied month into pl-PL Excel and Google Sheets and check `H:MM`, `7,50`, `-1,50`, an ISO date, a note starting with `=` or `"`, and a note with a tab. Also check whether LF or CRLF reaches the clipboard.
