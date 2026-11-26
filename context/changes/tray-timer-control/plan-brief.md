# START/STOP from the tray and today's work time in the tooltip — Plan Brief

> Full plan: `context/changes/tray-timer-control/plan.md`
> Research: `context/changes/tray-timer-control/research.md`

## What & Why

Roadmap S-03 (issue #23, MS-23 / FR-013): start and stop the work-time counter from the tray menu and see today's work time in the tray tooltip, without opening the window. The counter is a guardrail ("time is never lost"), so the tray must use the same Rust state and event log as the header, with no second counter.

## Starting Point

The counter state is already only in Rust (one running `work_entries` row), and `start` / `stop` write the entry and the event-log row in one transaction. The header commands emit `work-time-changed` / `events-changed`, which the one `WorkTimerProvider` subscription and the event log already follow. The tray is static except for the health snapshot: three fixed menu items, an icon and a tooltip chosen from the source state only, written only by the health round.

## Desired End State

The tray menu starts with "START" or "STOP"; a click acts at once and the header, "Czas pracy" and the event log update on their own. The tooltip reads, for example, `Nawigator dnia — OK` / `Czas pracy dziś: 3:42 (trwa)`, never over 127 UTF-16 units. The icon shows an amber dot while the counter runs; a source error still shows the error icon.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Tray STOP confirmation | None; logged in the event log | A menu item is a deliberate open-then-click | User (D1) |
| Running icon | New "running" variant; error keeps priority | The counter state is visible at a glance without hiding failures | User (D2) |
| PRD anchor | MS-23, no new FR, no PRD edit | Scope stays the roadmap slice | User (D3) |
| START/STOP path | One `start_timer` / `stop_timer` in Rust for header and tray | S-07 and S-04 hook into one place | User (D4) |
| Tray writer | Health round stays the only writer; work-time writes wake it | Two writers would overwrite each other's tooltip | Plan (agent default, research option A) |
| Menu shape | One item "START"/"STOP", acts on the shown action | A stale label can never invert the user's intent | Plan (agent default) |
| Tooltip | Two lines, health first, work line kept whole on truncation | The work line must survive the 127-unit cap | Plan (agent default; `" · "` fallback) |
| Icon priority | error > idle (starting/stalled) > running > ok | Errors and a stalled check outrank the counter | Plan (agent default) |
| Failures | Value-free toast; "already changed" refusals only refresh | No window to show an error in; same rule as the UI | Plan (agent default) |
| No-data states | Item disabled, "wczytywanie…" / "niedostępny"; never "0:00" | Never show an empty value in place of a failed source | Plan (agent default) |
| Event-log origin | Not recorded; same rows as the header | Issue says "tak jak dziś"; avoids a migration | Research / Plan |

## Scope

**In scope:**
- Rust read of today's live total (`work_time/today.rs`) and pure tray rules (`tray_view.rs`) with unit tests and a proptest
- `tray-running.svg` and its generated `running.png`
- Shared START/STOP functions, tray refresh after every work-time write, the menu item and its handler
- README and CHANGELOG lines

**Out of scope:**
- Any frontend, TS type, capability, CSP or config change
- Header confirmation changes, success toasts, end-of-day notice (S-07), autostart (S-04)
- Event-log origin column or any migration; roadmap and PRD edits
- Updating the Konsola design-system artifact (manual follow-up for the user)

## Architecture / Approach

Pure rules in `tauri`-free modules, side effects in `runtime.rs` and `commands.rs`. A tray click (main thread) only reads the shown action and spawns `start_timer` / `stop_timer` on the async runtime. Those run the existing domain write, emit the two existing events (so the UI follows) and wake the health round. The health round reads today's work next to its store probe and publishes icon, tooltip and menu item together under its existing `published` guard.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Today's work total | `today_work` equal to the header's total, tested | Midnight and clock-behind edge cases |
| 2. Tray view rules | Icon, tooltip, item and toast texts, 127-unit proptest | Work line lost to truncation |
| 3. Running tray icon | SVG, PNG by the documented recipe, four-state `tray_image` | Running `npx tauri icon` without `-o` overwrites bundle icons |
| 4. Shared START/STOP and live tray state | One START/STOP path; tray follows header within one round | Lock held across a blocking tray call |
| 5. Tray menu item | START/STOP in the menu, failure toast, docs | Store or lock touched on the main thread |

**Prerequisites:** none; node_modules installed; no migration.
**Estimated effort:** ~5 short implementer sessions, one commit per phase; manual checks after Phases 4 and 5.

## Open Risks & Assumptions

- Windows tray tooltip line break is not verified on this machine; fallback is one constant (`" · "`).
- A full health round per work-time write is assumed cheap; if the check thread stalls, the tray item is disabled until it recovers.
- Parallel slices S-01, S-04, S-06 touch `lib.rs`, `runtime.rs`, `work_time/mod.rs`, `commands.rs`; edits there are kept to a few local lines.

## Success Criteria (Summary)

- From the tray alone the user can start and stop the counter, and the header and event log show the same state at once.
- The tooltip always shows today's work time (or an explicit "niedostępny"), within 127 units.
- The icon shows a running counter, and a failing source still shows the error icon.
