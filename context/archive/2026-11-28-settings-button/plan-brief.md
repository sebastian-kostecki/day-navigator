# Settings dialog — Plan Brief

> Full plan: `context/changes/settings-button/plan.md`

## What & Why

One place in the UI to manage every setting. A gear button in the header opens a modal "Ustawienia" dialog with switches and inputs for all existing `settings.json` blocks, plus new on/off switches for Windows notices. Today every setting is edited by hand in a JSON file. This change lifts the roadmap's parked item "Editing settings in the app".

## Starting Point

`settings.json` is hand-written and the app only reads it (`README.md:102`, `src-tauri/src/settings.rs:1`):
- Nine blocks, each validated on its own with value-free errors.
- Nearly all re-read live. The exception is autostart, which syncs once at startup.
- No write path, no modal, and no gear icon exist.

Only reminders can be switched off among the Windows notices.

## Desired End State

From any view, the gear opens a dialog with seven tabs: GitLab, GitHub, Do zrobienia, Czas pracy, Przypomnienia, Powiadomienia and Autostart. Each tab shows current values (or defaults) and has its own "Zapisz".
- A save writes only that section's blocks, keeps the file's key order and unknown keys, and takes effect without a restart. Autostart, for example, syncs the Windows entry right away.
- Bad values get Polish messages on the exact field or chip.
- A block changed on disk in the meantime is never overwritten.
- A broken block can be replaced after a confirmation.
- A missing file is created on Save.
- A malformed file is never touched.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Form factor | Modal native `<dialog>`, gear button in the header | Reachable from every view; the browser handles the focus trap and backdrop. |
| New options | `notices` block: `newAssignments`, `sourceProblems` (GitLab and GitHub only), `daySummary` (default on) | Matches the existing toast sites; app-fault alerts (stall, settings, app data, autostart) stay always on. |
| Save model | Save per section, each writing only its blocks | Mirrors block-by-block validation; a typo in one place never blocks another. |
| Missing file | Created on explicit Save (config dir must exist) | Creation is a visible user action, so it keeps the "never silently" rule. |
| Conflicts | Per-block fingerprint; refuse and offer "Wczytaj ponownie" | A hand edit is never silently overwritten. |
| File format | `serde_json` `preserve_order`, 2-space pretty, BOM and CRLF kept, atomic temp+rename | The file stays readable and diff-friendly next to hand edits. |
| Validation | Per-field errors from shared Rust validators (loaders reuse them) | One source of truth; the accept set stays identical by construction. |
| List editing | Chips with an add field (paste of many lines supported) | The user's choice; paste keeps 100-entry lists practical. |
| Autostart | Apply on Save via the same sync as startup | The switch does what it says, no restart. |
| Invalid block on disk | Error state, defaults, explicit "Zastąpić?" before replacing | The UI can repair it, never by accident. |

## Scope

**In scope:**
- All nine existing blocks and the new `notices` block.
- A Rust writer and the `settings_view` / `save_settings_section` commands.
- The three notice gates.
- Applying autostart on save.
- The `settings-changed` event and refresh wakes.
- Shared `Dialog` and `ChipListField` components, the gear icon and the `--scrim` token.
- Pointing the existing "Ustawienia" entry points at the dialog.
- Updates to README, AGENTS.md, roadmap, PRD note, CHANGELOG and the design-system artifact.

**Out of scope:**
- Credential or CLI-auth fields.
- A start-view setting and close-button behaviour.
- A separate review-request toggle.
- Gating the stall or tray-failure toasts.
- Editing a malformed file.
- A file watcher.
- An open-dialog shortcut.
- `tauri-plugin-fs`.
- A version bump or release.

## Architecture / Approach

The Rust side owns every rule:
- Block validators return field errors and are shared with the loaders.
- A document module keeps BOM, line-ending style and key order.
- A section model turns typed form input (numbers as strings) into block `Value`s.
- The save runs under one write lock: re-read, fingerprint check, invalid-block confirmation, validate, splice, then temp file + `sync_all` + rename. Side effects (autostart sync, event, wakes) come after.

The UI (`src/settings/`) is a provider plus a dialog with seven section components on the typed API. It only displays results and sends user actions.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Field-level validators + `notices` | Shared validators with field errors; `notices` loader | Changing what loaders accept; guarded by unchanged loader tests |
| 2. Writer + read/save commands | Atomic, order-preserving section save with conflict and replace rules; TS mirrors | Data loss on a hand-written file; covered by writer and round-trip tests |
| 3. Save side effects | Autostart applied on save, three notice gates, `settings-changed`, wakes | Gating the wrong toast; tests per kind |
| 4. Dialog groundwork | `Dialog`, `ChipListField`, gear icon, `--scrim`, provider, header button, shortcut guard | First modal in the app; CSP and focus checks |
| 5. Seven sections | Full UI with per-section save, errors, conflict and replace flows; repointed entry points | Field-error mapping gaps; unmapped keys shown, never dropped |
| 6. Docs and policy | README, AGENTS.md, roadmap, PRD, CHANGELOG, design system | Stale "never writes" lines |

**Prerequisites:** M-4 wave 2 merged state on `m4-wave2-integration`. Back up `settings.json` and the app data dir before dev runs.
**Estimated effort:** ~5–6 implementation sessions across 6 phases. Phases 2 and 5 are the largest.

## Open Risks & Assumptions

- Enabling `preserve_order` changes `serde_json::Map` crate-wide. This is assumed harmless because `Value` equality ignores order, and it is checked by the full `cargo test`.
- `fs::rename` over a file held open by an editor may fail on Windows. That is reported as `Io` with the file untouched, never as a partial write.
- The first save normalises whitespace in the whole file. Key order and unknown keys are kept, and the README tells the user.
- GitLab and GitHub values travel over IPC to the dialog. They never appear in errors, logs, the store or tests.

## Success Criteria (Summary)

- Every setting can be seen and changed from the gear dialog, and takes effect without a restart.
- A bad value, a concurrent hand edit or a broken file never corrupts or silently overwrites `settings.json`.
- Each of the three notice kinds can be turned off independently, while app-fault alerts stay on.
