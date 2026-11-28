<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Settings dialog

- **Plan**: context/changes/settings-button/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2, 3, 4, 5, 6
- **Date**: 2026-11-28
- **Verdict**: APPROVED
- **Findings**: 0 critical, 2 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

Automated criteria, run before the fixes: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (976 passed, 0 failed), `cargo audit` (allowed warnings only), `npm run lint`, `npm run format:check`, `npm run build`, and the `rg` checks of 4.3 and 6.2. The 16 manual rows were ticked on the user's confirmation, not observed by this review.

## Findings

### F1 — Close confirmation stays armed and closes the dialog by itself

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/settings/SettingsDialog.tsx
- **Detail**: With the confirmation shown the form stayed editable. As soon as no section was dirty the dialog closed: after "Zapisz" (the outcome and the autostart status were never seen) or after typing the old value back.
- **Fix**: Edit, save and reload withdraw the pending close request.
- **Decision**: FIXED

### F2 — The dialog can be closed while a save is in flight

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/settings/SettingsDialog.tsx
- **Detail**: Esc during a save offered "Zamknij bez zapisu" although the write still committed; a failure was lost with the unmounted component.
- **Fix**: Ignore close requests and disable "Zamknij" while a section is pending.
- **Decision**: FIXED

### F3 — The replace question does not say content is discarded

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/settings/sections/SectionShell.tsx
- **Detail**: An invalid block loads as defaults or empty fields; "Zastąp" writes those. A `gitlab` block with one typo is removed when the host field is left empty.
- **Fix**: Say that the current content in the file will be lost and name the blocks.
- **Decision**: FIXED

### F4 — A rewrite can change more than the saved section

- **Severity**: 💡 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/settings/document.rs
- **Detail**: A symlinked `settings.json` is replaced by a regular file (read from the code, not executed). Repeated keys collapse to the last one, and very large or precise numbers in unknown blocks may be rounded.
- **Fix**: Document both in the README. Following the link was rejected: the temp file and the write would then land outside the app config dir, against the AGENTS.md write rule.
- **Decision**: FIXED (documented; behaviour unchanged)

### F5 — The autostart status stays after the switch is changed

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: src/settings/useSettings.ts
- **Detail**: "Autostart włączony…" stayed next to a switch toggled off but not saved.
- **Fix**: An edit clears the section's sync status.
- **Decision**: FIXED

### F6 — The settings dialog imports its file opener from the GitLab feature

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/gitlab/api.ts
- **Detail**: After this change the dialog was the only user of `openSettingsFile`.
- **Fix**: Move the function, its message and `OpenSettingsError` to `src/settings/`.
- **Decision**: FIXED

### F7 — Three `Toast` variants exist only for a test

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Scope Discipline
- **Location**: src-tauri/src/settings/notices.rs
- **Detail**: The stall, tray-failure and reminder call sites never ask `allows`, so a gate added there would not fail the test behind row 3.2.
- **Fix**: Keep the variants and the test, and state the limit in the doc comment.
- **Decision**: ACCEPTED — the guarantee rests on review

### F8 — The plan text differs from the shipped behaviour

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Adherence
- **Location**: context/changes/settings-button/plan.md
- **Detail**: A save keeps other unsaved drafts (the plan says it replaces the whole view), and `ChipListField` also adds the typed text on blur. Both are intended.
- **Fix**: Add an addendum to the plan.
- **Decision**: FIXED

## After the fixes

The behaviour of F1, F2, F3 and F5 was checked in a browser against the Vite dev server with an invented in-page backend, not in the built app. The manual rows were not repeated.
