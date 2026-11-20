# Guarded Vault Access — Plan Brief

> Full plan: `context/changes/safe-vault-access/plan.md`
> Roadmap item: F-01 in `context/foundation/roadmap.md`

## What & Why

This change builds the Rust layer through which every later slice reads and writes Obsidian notes in the Work and Private vaults. It enforces three PRD rules:

- The app touches only configured folders and never sees excluded paths.
- A write changes only the targeted line or section, so the rest of the file stays byte-identical.
- Nothing is deleted permanently.

It comes first because `main_goal: quality`: no vault write is safe without it, and it unlocks the north-star slice S-01.

## Starting Point

The app is the bare Tauri 2 starter: one demo command, no settings, no file access, no `tauri-plugin-fs` and no tests. The real vaults are git repos. Issue notes keep `Status` as a YAML list and tasks keep it as a single value. Obsidian may have a note open while the app writes it.

## Desired End State

`day_navigator_lib::vault` provides `read_note`, `list`, `replace_field`, `replace_section`, `append_to_section`, `create_note` and `trash`. Each call is tied to one space and guarded on every operation:

- **Paths:** the root must exist, roots must not overlap, and canonical containment holds even through junctions.
- **Filters:** excluded prefixes, hidden folders and non-`.md` files are refused.

Writes are atomic, keep Windows file metadata, and never overwrite a concurrent edit to the same section. Settings come from a hand-written `%APPDATA%\com.sebas.day-navigator\settings.json`. `cargo test` proves byte-identity on an invented fixture vault that later slices reuse.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Excluded-path matching | Vault-relative prefix, whole segments, case-insensitive | Predictable for a security boundary: `Dostepy` never catches `Dostepy-stare`. |
| Section boundary | Up to the next heading of the same or higher level; ignore fences and frontmatter; duplicate or missing heading is an error | Matches Obsidian's own sections and never guesses the target. |
| Concurrent edits | Re-read on every write, and replace ops carry a section/field hash → `Conflict` on mismatch | Edits elsewhere in the note survive, and a same-section edit is never overwritten. |
| Primitive scope | Generic set: field (scalar or list), section replace and append, create, trash, read, list | S-01, S-03, S-05 and S-13 compose it without reopening the safety layer. |
| Creating folders | The configured root must exist; subfolders and `.trash/` may be created inside it | Keeps the "never create a configured path" rule and lets year folders work. |
| Test depth | Fixture vault plus proptest byte-identity properties | Proves integrity beyond hand-picked cases; the fixture serves later slices. |
| Frontend exposure | Rust API only, no fs plugin; slices add intent-level commands | The webview can never write arbitrary vault paths. |
| Atomic replace | Temp file in the same folder, then `ReplaceFileW` on Windows | No partial files, and Obsidian's "created" time survives edits. |

## Scope

**In scope:**
- Settings loader (read only)
- `VaultPath` validation
- Guards and registry, stored in Tauri managed state
- Span finding and pure edits
- Atomic write path, create and trash
- Invented fixture vault and `.gitattributes`
- Guard, unit, property and integration tests
- AGENTS.md testing note

**Out of scope:**
- Settings UI and writing settings (S-06)
- Tauri commands and frontend access
- Note formats
- File watching and logging (F-02)
- CI (F-03)
- Folder trash, restore, rename and move
- Glob exclusions, setext headings and full YAML
- Inserting missing sections or fields
- Windows Recycle Bin

## Architecture / Approach

There are three layers:

1. **Path guard.** It turns an untrusted relative string into a checked absolute path, or a typed `VaultError`.
2. **Pure span logic.** Bytes in, bytes out: `prefix ++ replacement ++ suffix`. Proptest hammers this layer without touching the disk.
3. **Single write path.** Re-read, edit, write the temp file, check the file is unchanged, replace. This is the only code that puts bytes into a vault, and every mutating operation goes through it.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Settings and path guard | Settings, `VaultPath`, `Vault`/registry, guarded read and list, fixture vault, Tauri state | Junction or case tricks bypassing exclusion; mitigated by re-checking after canonicalisation |
| 2. Markdown spans and pure edits | Field and section spans, hashes, pure edits, proptest identity | Span rules diverging from Obsidian (fences, list fields, EOL/BOM) |
| 3. Safe writes and trash | Atomic conflict-checked writes, `create_note`, `trash`, integration and smoke tests | The `ReplaceFileW` failure branch and Obsidian holding the file open |

**Prerequisites:** none. rustc ≥ 1.90 and the npm toolchain are already in place.
**Estimated effort:** about 3 sessions, one per phase.

## Open Risks & Assumptions

- A few milliseconds between the last re-read and the replace are unprotected (Obsidian shares no lock). This is accepted, with git and Obsidian file recovery as the backstop.
- Vaults are assumed to be on local NTFS drives. Network or cloud-placeholder paths may canonicalise differently.
- Git must not normalise the CRLF and BOM fixtures; `.gitattributes` with `-text` is required.
- `.trash` is flat and collisions are numbered from ` 2`, matching Obsidian (checked in Obsidian 1.15.1). An existing entry is never overwritten.
- `ReplaceFileW` is always called with an explicit backup name, so the 1176 and 1177 failure branches can restore either the new or the original content.

## Success Criteria (Summary)

- A smoke run against a scratch copy of the invented vault, open in Obsidian, changes only the targeted lines (`git diff`), and the trashed note lands in `.trash/`.
- Excluded, hidden, out-of-root and missing-root cases all fail with an explicit typed error and never read or create anything.
- `cargo test` (fixture plus property tests), `cargo build`, `npm run build` and `cargo audit` are green.
