# App Data Store — Plan Brief

> Full plan: `context/changes/app-data-store/plan.md`

## What & Why

Roadmap foundation F-01: the app needs its own durable store for own tasks, the "To do" list, hidden-issue snapshots and work-time entries. The PRD's guardrail "Czas pracy nie ginie" requires that closing the app, a Windows restart or a crash never loses the running counter or history, and every data slice (S-03, S-04, S-06, S-07, S-08) is unsafe without it. F-01 delivers only the storage contract; each slice adds its own records.

## Starting Point

Today the only app-owned file is the read-only `settings.json`. The only atomic-write code (`vault/fsops.rs`) is tied to vaults and cannot change two records at once. The tray health covers settings and vault paths only.

## Desired End State

A SQLite database `day-navigator.db` lives in `%APPDATA%\com.sebas.day-navigator\` with WAL and full sync, versioned migrations and atomic write transactions. A copy lands in `backups/` once a day, and the last 7 are kept. A damaged file or one from a newer app version is refused and reported, never reset. The tray and health panel show it as "Dane aplikacji". A test that kills a real writer process proves that no reported commit is lost and no write is half-applied, and later slices reuse that test harness.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Storage engine | SQLite via `rusqlite` (bundled), WAL, `synchronous=FULL` | Transactions across records (S-07 ticks an item and completes a task together) and crash safety come from the engine. |
| Damaged or newer store | Refuse and report; never touch the file | The hard rule forbids an empty list in place of a failure, and a reset would silently drop work-time history. |
| Backups | Daily copy into `backups/`, keep 7; failure logged, non-blocking | Gives a recovery path for a damaged file or a bad migration without blocking the app. |
| Tray health | New "Dane aplikacji" source (`app-data`), retried every round | A broken store raises an alarm before the user opens a view, reusing the F-02 machinery. |
| Crash proof | Kill a child writer process at varying moments | Exercises a real abrupt process death on Windows instead of relying on documentation. |
| Location | App data dir (Roaming): `%APPDATA%\com.sebas.day-navigator\` | Matches AGENTS.md and the PRD; sits next to `settings.json`. |
| Schema evolution | Append-only `Migration` list with `user_version`, one transaction each | A migration and its version bump commit or roll back together; shipped migrations are never edited. |

## Scope

**In scope:**
- `store` module: open sequence, pragmas, integrity check, migrations, `read`/`write`, `StoreError`
- Daily backup with retention of 7
- Child-process crash test and a reusable harness in `tests/common/crash.rs`
- `StoreHandle` in Tauri state, the "Dane aplikacji" health source, Polish phrases
- Store conventions and the restore steps in `AGENTS.md`

**Out of scope:**
- Any slice's tables or records; new commands or views
- Automatic repair, reset or restore; off-disk backups; export/import
- `tauri-plugin-sql`/`tauri-plugin-store`, frontend access to the store
- Vault code, `settings.json`, data migration from old time records

## Architecture / Approach

`store::Store` is plain Rust (no `tauri` import, today's date passed in). Opening runs a fixed sequence: create the folder, refuse an empty file or a missing one next to existing backups, open, run `quick_check`, refuse a newer schema, only then switch to WAL, take the daily backup, then migrate. Every refusal happens before any write, so a refused file stays byte-identical. `StoreHandle` (a mutex around an optional open store) is Tauri managed state. Later slices' commands call `with_store`, and the 30 s health round calls `status`, which reopens if needed, checks liveness and runs the daily backup, because the app lives in the tray for days.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Store core | `rusqlite`, `Store` API, errors, migrations, refusal and rollback tests | Errors leaking row values; the API shape later slices build on |
| 2. Daily backups | `VACUUM INTO` copy per day, keep 7, non-fatal failure | Skipped backups while the app stays open (handled via the health round) |
| 3. Crash-proof tests | Child-process kill test + reusable harness | Flaky or slow on the CI Windows runner |
| 4. App wiring and tray health | `StoreHandle`, "Dane aplikacji" source, AGENTS.md conventions | A settings error hiding the store's status (appended in `probe_round`, not `probe()`) |

**Prerequisites:** none (F-01 has no roadmap prerequisites); rustc 1.90+, MSVC toolchain already required by Tauri.
**Estimated effort:** ~3–4 sessions across 4 phases.

## Open Risks & Assumptions

- `rusqlite` 0.43 is assumed to build on rustc 1.90 and pass `cargo audit`; otherwise use the newest release that does.
- The crash test's kill timing is not deterministic; fixed kill points plus a ~64 KB payload widen the window, and the assertions hold for any kill moment.
- Backups sit on the same disk, so they don't protect against disk loss (accepted).

## Success Criteria (Summary)

- After a hard kill of the app, the store opens intact with every committed write and no half-applied one, proven by tests; surviving a Windows crash or power loss relies on `synchronous=FULL` (checked by reading the pragma back).
- A damaged, empty, missing (while backups exist) or newer store shows as a red "Dane aplikacji" source with a Polish message, the file stays untouched, and restoring a daily backup recovers it.
- Later slices can add a migration and use `with_store` without touching storage plumbing.
