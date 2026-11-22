# Source Health Status — Plan Brief

> Full plan: `context/changes/source-health-status/plan.md`
> Roadmap item: F-02 in `context/foundation/roadmap.md`

## What & Why

The app moves into the Windows tray. A status icon shows OK, source error or not running, and a Polish toast fires when a source breaks and again when it recovers. The PRD requires every source failure to be visible within 1 minute without opening the window, and forbids showing an empty list in place of an error. S-01, S-07, S-09, S-14 and S-17 report into this channel.

## Starting Point

The app is the Tauri 2 starter plus F-01's guarded vault layer. Settings are loaded once at startup and never rechecked. There is no tray, notification, logging or background thread, and closing the window exits the app. The only sources today are `settings.json`, the two vault roots and their exclusions; `glab`/`gh` are not called yet.

## Desired End State

One tray icon:

- green when every configured source is OK;
- red when any source errors;
- grey while starting or when the check loop has stalled.

Every 30 s, and on "Sprawdź teraz", the app re-reads `settings.json`, swaps the vault registry and probes each source. Entering an error, changing error kind or recovering gives one toast per check. A source that stays broken gives no repeats. Closing the window hides it to the tray, and a second launch focuses the running app. A health panel replaces the demo UI and lists every source in Polish. It renders an `ErrorState`, never a blank list, when health data cannot be loaded.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Source scope | A generic registry plus probes for settings, vault roots and exclusions; `glab`/`gh` probes ship with S-07/S-16 | The roadmap limits F-02 to the reporting channel, and only these sources exist yet. |
| "Not running" | A dead process means no icon (accepted); a live process with a stalled check loop shows a grey icon | Every state the app draws is honest, with no external watchdog outside configured folders. |
| Toast policy | One toast per check on entering an error or changing error kind, plus a "działa ponownie" recovery toast; no repeats | Avoids notice storms and still confirms that a fix worked. |
| UI surface | Health panel + `health_status` / `check_health_now` / `health-changed` + reusable `ErrorState` | Proves "error instead of empty list" end to end before S-01 needs it. |
| Logging | tauri-plugin-log, rotating, transitions only: source key and error kind, no paths or content | Leaves a trail of past failures while keeping work data out of logs. |
| Settings reload | Re-read on every check; `VaultState` becomes swappable | A fixed `settings.json` turns the icon green within one check, with no restart. |
| Language | Polish user-facing text, produced in Rust; code, logs and error kinds stay English | Matches the PRD and keeps the tray, toast and panel wording identical. |
| Pause / day-off rule | The tray always shows health; toasts pass a `notices_allowed()` gate and are not replayed | Fixes the FR-023/FR-024 contract now, so the notice API does not need reshaping later. |
| Unconfigured space | Neutral "nie skonfigurowano", never red; but no space configured at all → settings error `NoSpaceConfigured` | Running with only a Work vault before S-06 is a valid setup; running with no vault is the PRD's "missing configured path" failure. |
| App-owned folders | The log dir and config dir under the bundle identifier become an explicit exception in the AGENTS.md hard rule | tauri-plugin-log writes outside configured folders; the exception is stated, not implied. |
| Background loop | `std::thread` + `recv_timeout(30 s)` and a 15 s watchdog that ignores resume-from-sleep gaps | Needs no `tokio`, "check now" wakes it instantly, and sleep never fakes a stall. |

## Scope

**In scope:**
- Pure `health/` module: types, probes, transition diff, toast and tooltip text, staleness rule
- Swappable `VaultState`
- Tray with three icons and a menu (Otwórz / Sprawdź teraz / Zakończ)
- Single instance and close-to-tray
- Check and watchdog threads
- Gated toasts and a sanitised rotating log
- Health panel, `useHealth` hook and `ErrorState`
- Removal of the `greet` demo

**Out of scope:**
- `glab`/`gh` probes and CLI calls
- An external watchdog or autostart (S-02)
- Toast click actions
- Pause and day-off modes themselves
- Repeated reminders
- Real screens
- Settings UI (S-06)
- File watchers
- npm plugin packages

## Architecture / Approach

The work splits into a pure core and a thin Tauri layer:

- **`health/` (pure, no Tauri):** `probe(settings_path)` → registry result + `Vec<SourceStatus>`. `transitions(prev, next)` → `notice_for(..)` gives at most one toast per check. `overall(..)`, `tooltip(..)` and `is_stale(..)` produce the presentation.
- **`runtime.rs` (Tauri side effects):** each round the check thread probes, swaps `VaultState`, updates the tray, sends a gated toast, logs, emits `health-changed` and stamps the heartbeat. The watchdog turns the icon grey on a real stall.
- **React:** reads the stored snapshot through `health_status` and follows `health-changed`. All Polish text comes from Rust.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Health core | Pure types, probes, transition and toast rules, tooltip cap, staleness rule, swappable settings state, tests | Toast and tooltip text leaking an absolute path, which would break the work-data boundary |
| 2. Tray and background runtime | Tray, single instance, close-to-tray, check and watchdog threads, gated toasts, log | Toasts can fail without notice (the plugin returns `Ok` early, and dev shows them as "Windows PowerShell"); the tray must lead |
| 3. Frontend contract and health panel | Commands, event, health panel, `useHealth`, `ErrorState`, demo removed | Panel rendering an empty list when the invoke fails instead of `ErrorState` |

**Prerequisites:** none. F-01 is done; rustc ≥ 1.90 and npm are in place.
**Estimated effort:** about 3 sessions, one per phase.

## Open Risks & Assumptions

- A toast can fail or be swallowed by Focus Assist without the app knowing. The tray icon is the guaranteed channel.
- Toasts show the app's identity only in an installed build with Start-menu shortcuts. A portable exe probably shows nothing (not tested).
- In release (`panic = "abort"`) a panic in a check kills the process, which shows as a missing icon. The grey state covers hangs only.
- The idle RAM budget (< 150 MB) is dominated by WebView2. Hiding the window does not free it; this is measured in Phase 2 but not optimised here.
- Third-party crates may log at `warn`. Their output is not sanitised by our rule.

## Success Criteria (Summary)

- Renaming the configured vault folder turns the tray red and shows one toast within 60 s. Renaming it back turns it green with one recovery toast.
- A broken or missing `settings.json` is shown as an explicit error in the tray and the panel, never as an empty screen, and is picked up without a restart once fixed.
- `cargo test`, `cargo build`, `cargo audit` and `npm run build` are green. The log holds transitions without paths.
