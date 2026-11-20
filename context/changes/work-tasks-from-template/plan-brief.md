# Work Tasks from Template — Plan Brief

> Full plan: `context/changes/work-tasks-from-template/plan.md`

## What & Why

Roadmap slice S-05 (PRD FR-013, must-have): the user creates a task in the work vault from the vault's task template and changes its status from the app. Work to-dos stop getting lost. This is also the first real test of F-01's guarantee that an app write changes only the targeted lines of a note the user already owns.

## Starting Point

F-01 (`safe-vault-access`) delivered a Rust-only vault API: guarded `read_note`/`list`, hash-checked one-field `replace_field`, no-clobber `create_note`, all through a single atomic write path. There are no Tauri commands for it yet. The UI is still the starter's greet demo, and settings hold only vault roots and exclusions.

## Desired End State

The app opens on a "Zadania" view:
- It lists the work tasks from the configured folder with their status, priority and due date.
- Each task's status can be changed with a picker. Moving into the done status sets `Done` to today, and moving out of it clears `Done`, in the same single atomic write.
- A "Nowe zadanie" form creates `Tasks/<title>.md` from the vault template.
- If a note changed in Obsidian meanwhile, the app reports a conflict instead of overwriting.
- Every failure (settings, folder, template, a bad note) is shown explicitly, never as an empty list.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Template source | Vault note at a settings path; only core `{{title}}`/`{{date}}`/`{{time}}` are filled, anything else is refused | The template stays owned in Obsidian, and an unfillable token fails loudly instead of leaking into the note. |
| Status vocabulary | Ordered list in `settings.json` (first = new-task status) plus a named `done_status` | Matches the vault's own words without hard-coding work conventions in the repo. |
| `Done` field | Set to today on entering done, cleared (`Done:`) on leaving it | `Done` always agrees with `Status`, so the future day log (FR-005) can trust it. |
| Create form | Title (required), priority from settings, due date | Covers what the morning plan ranks by, without S-04's issue list. |
| File name | Title verbatim; refuse forbidden or link-breaking characters and existing names | Note name = title, as in Obsidian, so `[[links]]` behave predictably. |
| UI scope | Standalone "Zadania" view: list + status picker + create form | A complete FR-013 loop usable now, and reusable by S-01's "Today". |
| Testing | Rust unit, property and fixture integration tests, plus manual UI checks | Every write is proven where F-01 already tests, with no new toolchain. |
| Note without `Done` (planning assumption) | Status still changes, and the UI says `Done` was absent | Existing hand-made tasks stay usable without inserting fields, per F-01's rule. |
| Write atomicity | New `replace_fields`: all hashes checked on the original bytes, then one write | `Status` and `Done` land together or not at all. |

## Scope

**In scope:**
- A `tasks` settings block (folder, template, statuses, done status, priorities)
- `FieldValue::Empty` and an atomic multi-field `replace_fields`
- A tauri-free `tasks` module (list, render, create, set status)
- New invented fixtures and an integration test file
- Four Tauri commands, TS wrappers and the Zadania view

**Out of scope:**
- Private-space tasks (FR-025)
- `Today` writes (PRD decision #4)
- Issue picker or linking, and "Open in Obsidian" (S-04)
- The "Today" screen (S-01) and the settings UI (S-06)
- Templater and formatted date placeholders
- Rename, delete or field editing after creation
- Recursive folder scan and file watching
- A frontend test runner and Atrament styling

## Architecture / Approach

React view → `src/tasks/api.ts` (typed `invoke`) → thin commands in `lib.rs` (managed `TasksState`, which owns the local clock) → `tasks.rs` (pure rules plus `Vault` calls, no `tauri` import) → F-01 `Vault` (guard, pure edit, atomic replace). Time enters only at the command layer, so the domain is deterministic under test.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Task settings and vault extensions | The settings block, `FieldValue::Empty`, `replace_fields` with proptest | A hash checked after a splice would let a half-edit through. |
| 2. Task domain | List, template rendering, create, status change with the `Done` rule, fixtures and integration tests | Template edge cases (BOM/CRLF, `{{title}}` in frontmatter) breaking Properties. |
| 3. Commands and the Zadania view | 4 commands, TS API, the view with explicit error states | An error state collapsing into an empty list, against the PRD rule. |

**Prerequisites:** F-01 merged (it is). A scratch copy of the fixture vault and a hand-written `settings.json` for the manual checks.
**Estimated effort:** ~2–3 sessions across 3 phases.

## Open Risks & Assumptions

- The real work-vault template is assumed to use core-Templates syntax with `Status`, `Priority`, `Due` and `Done` in the frontmatter. If it uses Templater, creation is refused until the template is adapted.
- Obsidian's core Templates date and time formats are assumed to be the defaults (`YYYY-MM-DD`, `HH:mm`). A custom format would differ from what the app fills.
- `Done:` (YAML null) is assumed to display as an empty property in Obsidian; a manual check confirms it.
- A few-millisecond race with an Obsidian edit stays accepted, as in F-01.

## Success Criteria (Summary)

- From the app, the user creates a work task that opens in Obsidian with filled Properties, and changes its status with `Done` following.
- `git diff` on the vault after any status change shows only the `Status`/`Done` lines.
- Missing settings, folder or template, and concurrent edits, always produce an explicit message, never an empty list or a lost edit.
