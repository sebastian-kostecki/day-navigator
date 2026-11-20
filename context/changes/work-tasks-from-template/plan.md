# Work Tasks from Template Implementation Plan

## Overview

Roadmap slice S-05 (PRD FR-013): the user creates a task in the work vault from the vault's own task template and changes its status from the app. It is the first slice that puts a user-visible view and Tauri commands on top of F-01's guarded vault API. It is also the first test of F-01's line-scoped write on notes the user already has: a status change edits only the `Status` line and, together with it, the `Done` line.

## Current State Analysis

- **The vault API is ready, Rust-only.** `Vault::create_note` (no-clobber; it creates parent folders under the existing root) and `Vault::replace_field` (one field, hash-checked, atomic) live in `src-tauri/src/vault/mod.rs:377-426`. Every write goes through the private `Vault::apply` (`mod.rs:350-374`), which re-reads, edits purely, replaces atomically and retries on `Changed`.
- **One field per write.** `replace_field` edits one field per atomic write. A status change that also sets `Done` would be two writes. If the second one returns `Conflict`, the note is left half-changed.
- **Empty values are refused.** `edit::check_plain` (`src-tauri/src/vault/edit.rs:35-65`) refuses empty strings, so `Done` cannot be cleared today. It also refuses values that would need YAML quoting, for example a leading `[` or a ` #`.
- **Settings are minimal.** They hold only `vault` + `excluded` per space (`src-tauri/src/settings.rs:13-44`). `RawSpace` is `deny_unknown_fields`, so a new key must be added to the parser explicitly. Settings are hand-written until S-06.
- **Managed state holds only the registry.** `lib.rs` keeps `VaultState(Result<VaultRegistry, VaultError>)`. The only command is the starter `greet`, and `src/App.tsx` is the starter greet demo. There are no TS wrappers and no frontend test runner.
- **Task note shape (seed).** A `Tasks/` note has a frontmatter template with `Status` (a single value), `Priority`, `Due`, `Issue`, `Today` and `Done` (`notes/idea.md:28,71`). The app "changes `Status`, `Today`, `Done`; creates from template". PRD decision #4 says the app does **not** write `Today` (`context/foundation/prd.md:104`).
- **Fixture.** `src-tauri/tests/fixtures/vault/Tasks/Przykładowe zadanie.md` has only `Status: Todo` (no `Done`). `vault_write.rs:18` relies on it, so it must stay byte-identical. `vault_guard.rs:144-148` asserts the exact root listing (`Dostepy-stare, Dziennik, Issues, Notes, Tasks`).
- **No local-date source.** The backend has no local-date or local-time dependency yet.

## Desired End State

On launch the app shows a "Zadania" view instead of the greet demo:

- **Task list.** It shows the work tasks in the configured tasks folder, each with title, status, priority and due date.
  - Each row has a status picker limited to the configured statuses.
  - Done tasks are hidden behind a "Pokaż zakończone" toggle.
  - Any failure is shown as an explicit message instead of an empty list. That covers settings missing, tasks not configured, the vault root missing, the folder missing, and per-note problems.
- **"Nowe zadanie" form.** The fields are a title (required), a priority from the configured list (optional) and a due date (optional).
  - Submitting creates `<tasks folder>/<title>.md` from the configured template note.
  - Core placeholders are filled in.
  - `Status` is set to the first configured status, and `Priority`/`Due` are set when given.
- **Status change.** It rewrites the `Status` line. Moving into the done status sets `Done: <today>`, and moving out of it clears `Done`. Both edits land in **one** atomic, hash-checked write.
  - If the note changed in Obsidian since the list was loaded, the app reports a conflict and reloads. It never overwrites the change.
  - Every byte outside the edited field lines stays identical.

Verify with `cargo test`, `cargo build`, `npm run build` and `cargo audit` in `src-tauri/`. Then run the manual steps against a scratch copy of the invented fixture vault, open in Obsidian.

### Key Discoveries:

- `src-tauri/src/vault/mod.rs:350-374`: `apply` is the single write path. A multi-field edit must be one `apply` closure, not several `replace_field` calls.
- `src-tauri/src/vault/edit.rs:126-139`: `replace_field` finds the span, checks the hash and splices. A multi-field version checks **all** hashes against the original bytes before splicing any field.
- `src-tauri/src/settings.rs:38-44`: `RawSpace` is `deny_unknown_fields`. Adding an optional `tasks` object is additive, so the schema stays at version 1.
- F-01 rule (archived plan, Phase 1 §5): `vault/*` and `settings.rs` never import `tauri`. Test binaries that touch Tauri symbols crash with `0xc0000139`. The new `tasks` module follows the same rule, and `lib.rs` holds only thin command glue.
- `src-tauri/tests/common/mod.rs`: `test_vault()` and `space_settings()` build `SpaceSettings` literally. Adding a field to `SpaceSettings` requires updating this helper.

## What We're NOT Doing

- **No private-space tasks (FR-025).** Tasks settings are parsed per space, but the commands serve only `Space::Work`.
- **No `Today` writes** (PRD decision #4). `Issue` and `Today` stay exactly as the template has them.
- **No issue picker or issue linking.** That belongs to S-04 (`issue-list-to-today`).
- **No "Open in Obsidian"** (FR-014, S-04).
- **No "Today" screen, navigation or tray integration** (S-01). The Zadania view is the only view for now.
- **No settings UI or settings writes** (S-06). The user extends `settings.json` by hand.
- **No Templater support and no `{{date:FORMAT}}`/`{{time:FORMAT}}` variants.** A template using them is refused with an explicit error.
- **No renaming, deleting or trashing tasks, and no editing of title, priority or due date after creation.** That stays in Obsidian.
- **No recursive scan of the tasks folder.** Subfolders are ignored.
- **No inserting missing frontmatter fields into existing notes.** This keeps F-01's "no inserting missing pieces" rule.
- **No frontend test runner, no E2E, no Atrament styling.** The view uses plain CSS.
- **No file watching.** The list reloads on demand and after each write.

## Implementation Approach

Three layers, so each guarantee is tested where it is cheapest:

1. **Vault primitives (Phase 1).** Two small F-01 extensions, both proven by the existing proptest style:
   - `FieldValue::Empty`, so a field can be cleared.
   - An atomic multi-field `replace_fields`.

   The same phase adds the `tasks` settings block.
2. **Task domain (Phase 2).** A tauri-free `tasks` module of pure functions plus `Vault` calls:
   - parse task fields
   - render the template
   - validate titles
   - compute the status-change edit, including the `Done` rule

   It takes `today`/`now` as parameters, so tests are deterministic. It is tested against the invented fixture vault through `tests/common`.
3. **Commands and view (Phase 3).** `lib.rs` turns managed state into calls to `tasks::*` and computes the local date and time. The React view calls typed wrappers and maps every error kind to a Polish message.

Decisions settled in planning:

- **The template is a vault note** at a path in settings. Only the Obsidian core placeholders `{{title}}`, `{{date}}` (`YYYY-MM-DD`) and `{{time}}` (`HH:mm`) are filled. Any other `{{…}}` or any `<%` is refused, and the error names the placeholder.
- **Statuses are an ordered list in settings.** The first one is the status of a new task. `done_status` names the done status. A note whose status is not in the list is still shown and can be moved to a listed status.
- **The `Done` rule:**
  - Moving into `done_status` sets `Done: <today>`.
  - Moving out of it sets `Done:` (empty).
  - Moving between two non-done statuses leaves `Done` alone.
  - Re-selecting the same status writes nothing.
  - *Assumption made in planning:* a note without a `Done` field still gets its `Status` changed, and the response says the `Done` field was absent. This is how existing hand-made tasks (like the fixture's) work without inserting fields.
- **The create form:** a title (required), a priority from the settings list (optional) and a due date `YYYY-MM-DD` (optional).
- **The file name is the title verbatim** (trimmed), as `<folder>/<title>.md`. Nothing is written in these cases:
  - characters that `VaultPath` refuses
  - `/` or `\` (they would create subfolders)
  - `[ ] # ^ |` (they break `[[links]]`)
  - an existing note
- **Configured paths are never created.** A missing tasks folder or template is an explicit error. Only the root-existence rule from F-01 applies to folders the app itself creates, and this slice creates none.
- **Tests:** Rust unit, property and fixture integration tests, plus manual UI checks.

## Critical Implementation Details

**Multi-field hashes are checked on the original bytes.** `replace_fields` must compare every expected hash against spans found in the *original* bytes, and only then apply the edits one after another, re-finding each span in the progressively edited bytes. Checking a hash after an earlier splice would compare against shifted bytes. Splicing before all checks pass would produce a half-edit that `apply` then writes.

**The template title in frontmatter.** If `{{title}}` appears inside the template's frontmatter, the title must also pass the plain-scalar check (`check_plain`). Otherwise the rendered note breaks Obsidian's Properties block. Placeholders in the body take the title verbatim.

**The template's BOM and line endings carry over.** The new note's bytes are the template's bytes with placeholders replaced and app-owned fields set through the pure edit layer. A BOM and CRLF in the template therefore carry over to the new note. `create_note` takes `&str`, so a leading BOM is passed as `\u{FEFF}`.

## Phase 1: Task settings and vault extensions

### Overview

The `tasks` settings block, a way to clear a field, and one atomic write for several fields. These are prerequisites for the domain, each proven in isolation.

### Changes Required:

#### 1. Tasks settings block

**File**: `src-tauri/src/settings.rs`

**Intent**: Let the user describe where tasks live and which statuses and priorities exist, without hard-coding work conventions in the repo. Fail closed on anything malformed.

**Contract**:
- `SpaceSettings` gains `pub tasks: Option<TaskSettings>`. `RawSpace` gains an optional `tasks` object that also uses `deny_unknown_fields`. The `version` stays 1.
- `pub struct TaskSettings { pub folder: VaultPath, pub template: VaultPath, pub statuses: Vec<String>, pub done_status: String, pub priorities: Vec<String> }`
- Validation (`SettingsInvalid` with a reason naming the space):
  - `folder` and `template` parse with `VaultPath::parse`, and `template` must be `.md`.
  - `statuses` is non-empty with no duplicates, and `done_status` is one of them.
  - Every status and priority passes the plain-scalar rule (expose `edit::check_plain` as `pub(crate)`).
  - `priorities` may be empty, which hides the priority field in the form.
- Example (invented values):

```json
"work": {
  "vault": "D:\\vaults\\example-work",
  "excluded": ["Dostepy"],
  "tasks": {
    "folder": "Tasks",
    "template": "Templates/Task.md",
    "statuses": ["Todo", "In progress", "Done"],
    "done_status": "Done",
    "priorities": ["High", "Medium", "Low"]
  }
}
```

#### 2. Empty field value

**File**: `src-tauri/src/vault/edit.rs`

**Intent**: Allow clearing a scalar field so `Done` can be emptied when a task is reopened.

**Contract**: `FieldValue::Empty` renders as `key:` plus the file's EOL. That is YAML null, which Obsidian shows as an empty property. `Scalar("")` is still refused.

#### 3. Atomic multi-field edit

**Files**: `src-tauri/src/vault/edit.rs`, `src-tauri/src/vault/mod.rs`

**Intent**: Change several frontmatter fields in one atomic, conflict-checked write, so a status change and its `Done` update land together or not at all.

**Contract**:
- Pure: `pub fn replace_fields(bytes, edits: &[(&str, FieldValue, SpanHash)]) -> Result<Vec<u8>, VaultError>`.
  - All hashes are checked against the original bytes first (`Conflict` on any mismatch), then the fields are spliced one after another (see Critical Implementation Details).
  - A duplicate key in `edits` returns `InvalidValue`, and an empty `edits` returns the bytes unchanged.
- `Vault::replace_fields(&VaultPath, edits) -> Result<(), VaultError>` goes through `apply`, exactly like `replace_field`.
- `VaultError` derives `Clone`, so `lib.rs` can return a stored settings error by value.
- `SpanHash` (`src-tauri/src/vault/markdown.rs:72-80`) serialises as its hex string and gains `SpanHash::from_hex(&str) -> Result<SpanHash, VaultError>`. That function accepts exactly 64 lowercase hex characters and otherwise returns `InvalidValue`. A hash can now make the round trip through the webview.

#### 4. Tests

**Files**: `src-tauri/src/settings.rs` and `src-tauri/src/vault/edit.rs` (`#[cfg(test)]`), `src-tauri/tests/common/mod.rs`

**Intent**: Prove the new settings rules and that the new edits keep every byte outside the edited fields.

**Contract**:
- Settings tests:
  - a valid block
  - an absent block → `tasks: None`
  - an unknown key
  - empty `statuses`
  - `done_status` not in the list
  - duplicate statuses
  - a non-`.md` template
  - an invalid folder path
  - a status that needs quoting
- Edit example tests:
  - `Empty` renders `Done:` with LF and with CRLF
  - `replace_fields` on `Status` + `Done`
  - a mismatched second hash leaves the bytes untouched (`Conflict`)
  - duplicate keys
- Proptest (in the existing `mod proptests`):
  - `Just(FieldValue::Empty)` joins the `field_value()` strategy, so every existing byte-identity property also covers clearing a field.
  - For `replace_fields` over two generated fields, with values drawn from the same strategy, the bytes outside both field spans equal the original.
- `tests/common`: `space_settings` sets `tasks: None`. A new `test_tasks_settings()` returns invented task settings that match the Phase 2 fixtures.

### Success Criteria:

#### Automated Verification:

- Settings and edit unit tests pass: `cargo test -- settings vault::edit` (in `src-tauri/`)
- Property tests pass: `cargo test proptests` (in `src-tauri/`)
- Full suite still passes: `cargo test` (in `src-tauri/`)
- Backend builds: `cargo build` (in `src-tauri/`)

#### Manual Verification:

- The settings example above, with invented paths, reads correctly. Its field names match the parser.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Task domain

### Overview

A tauri-free `tasks` module that lists tasks, renders the template, creates a task and changes its status, proven against new invented fixtures.

### Changes Required:

#### 1. Task module

**File**: `src-tauri/src/tasks.rs` (new, top level next to `settings.rs`, declared in `lib.rs`; no `tauri` import)

**Intent**: Own the task note format: which fields are read and written, how a template becomes a note, and the `Done` rule. Keep it deterministic by taking time as input.

**Contract**:
- `#[derive(Debug, Clone, thiserror::Error, serde::Serialize)] #[serde(tag = "kind", content = "detail")] pub enum TaskError` with these variants:
  - `Vault(VaultError)`, `TasksNotConfigured`
  - `FolderMissing { path }`, `TemplateMissing { path }`
  - `TemplateUnsupported { placeholder }`, `TemplateFieldMissing { key }`
  - `InvalidTitle { reason }`, `UnknownStatus { status }`, `UnknownPriority { priority }`, `InvalidDue { value }`

  No variant carries note content, which matches the F-01 invariant.
- `pub fn require_config(tasks: Option<&TaskSettings>) -> Result<&TaskSettings, TaskError>` returns `TasksNotConfigured` for `None`. Every command resolves its config through it.
- `pub struct TaskRow { path: VaultPath /* serialised as string */, title: String, status: Option<String>, priority: Option<String>, due: Option<String>, status_hash: Option<SpanHash>, done_hash: Option<SpanHash>, problem: Option<VaultError> }`
  - `title` is the file stem.
  - Scalar values are read from the field block: trimmed, with surrounding `"`/`'` stripped, and an empty value becomes `None`.
  - `problem` holds the per-note error. That includes any `read_note` error (`NotUtf8`, `Io`, `Busy`, …), any `find_field` error on `Status` or `Done` (`NoFrontmatter`, `FieldNotFound` for `Status`, `FieldAmbiguous`, `FieldLayoutUnsupported`), and a list-valued `Status` (`InvalidValue`). A missing `Done` is not a problem: it gives `done_hash: None`. Such rows stay in the list with the status picker disabled. They are never dropped. The UI turns `problem` into Polish text through `taskErrorMessage`.
- `pub fn list_tasks(vault: &Vault, cfg: &TaskSettings) -> Result<Vec<TaskRow>, TaskError>`
  - `FolderMissing` if the folder does not exist, or is not a directory. Only folder-level errors return `Err`. Per-note errors become `problem` and never fail the list.
  - The scan is non-recursive and covers `.md` notes only. Excluded and hidden notes are already omitted by `Vault::list`. The note at `cfg.template` is skipped when it lies inside the folder (compared with `starts_with_ci` on equal segment counts), so the template is never listed or edited as a task.
  - Sort order: configured status order first (unknown statuses after the listed ones), then `Due` ascending (empty last), then title case-insensitively.
- `pub fn render_template(template: &str, title: &str, now: NaiveDateTime) -> Result<String, TaskError>`
  - It replaces `{{title}}`, `{{date}}` and `{{time}}`.
  - Any other `{{…}}` or any `<%` returns `TemplateUnsupported` with the first offending token.
  - `{{title}}` inside frontmatter requires the title to pass `check_plain` (`InvalidTitle`). The frontmatter range comes from a new `pub(crate) fn frontmatter_span(bytes) -> Option<Span>` in `src-tauri/src/vault/markdown.rs`, which wraps the private `frontmatter()`.
- `pub struct NewTask { title: String, priority: Option<String>, due: Option<String> }`
- `pub fn create_task(vault, cfg, new: &NewTask, now: NaiveDateTime) -> Result<VaultPath, TaskError>`
  1. Validate the title: trim it; it must be non-empty; refuse `/ \ [ ] # ^ |`; `VaultPath::parse("<folder>/<title>.md")` must succeed.
  2. Validate `priority` (it must be in `cfg.priorities`) and `due` (strict `YYYY-MM-DD` and a real date).
  3. Check the folder exists (`FolderMissing`), then read the template (a missing one gives `TemplateMissing`).
  4. Render the template, then set `Status` = `statuses[0]`, plus `Priority`/`Due` when given, through `edit::replace_fields` on the rendered bytes. The expected hashes come from those same bytes. A field the template lacks gives `TemplateFieldMissing` (`Status` is always required).
  5. `vault.create_note`. `AlreadyExists` passes through as `Vault(AlreadyExists)`.
- `pub enum DoneUpdate { Set, Cleared, Unchanged, Absent }` and `pub fn set_status(vault, cfg, path: &VaultPath, status: &str, status_hash: SpanHash, done_hash: Option<SpanHash>, today: NaiveDate) -> Result<DoneUpdate, TaskError>`
  - `status` must be in `cfg.statuses` (`UnknownStatus`).
  - The previous status is read from the fresh note. If it equals `status`, nothing is written (`Unchanged`).
  - If `done_hash` is `Some` but the fresh note has no `Done` field, or `None` while the fresh note has one, the note changed since listing. It returns `Vault(Conflict)` with the path filled in, and writes nothing.
  - The edit list is `Status`, plus `Done` (`Scalar(today)` or `Empty`) when the move crosses `done_status` in either direction and the note has a `Done` field.
  - It is one `vault.replace_fields` call. A stale `status_hash` or `done_hash` returns `Vault(Conflict)`.
  - When the `Done` update is needed but the note has no `Done` field (and `done_hash` is `None`), it returns `Absent`.
- Local date and time types: add `chrono` (`default-features = false`, features `["clock", "serde"]`) to `src-tauri/Cargo.toml`. Only `lib.rs` calls `Local::now()`. `tasks.rs` receives the values.

#### 2. Invented fixtures

**Files** (new, invented data only, under `src-tauri/tests/fixtures/vault/`):
- `Templates/Task.md`: frontmatter `Status: Todo`, `Priority:`, `Due:`, `Issue:`, `Today:`, `Done:`; body `# {{title}}` and `Utworzono {{date}} {{time}}`.
- `Templates/Task CRLF.md`: the same template with CRLF line endings and a BOM.
- `Templates/Task templater.md`: contains `<% tp.file.title %>`.
- `Templates/Task no status.md`: frontmatter without `Status`.
- `Tasks/Invented open task.md`: `Status: In progress`, `Priority: High`, `Due: 2026-02-01`, `Done:`.
- `Tasks/Invented done task.md`: `Status: Done`, `Done: 2026-01-10`.
- `Tasks/Invented list status.md`: `Status` as a YAML list.
- `Tasks/Invented crlf task.md`: CRLF line endings, `Status: Todo`, `Done:`.
- `Tasks/Invented duplicate status.md`: two `Status:` lines (`FieldAmbiguous`).

**Intent**: Cover every listing, rendering and status-change branch with notes that are obviously invented. The existing `Tasks/Przykładowe zadanie.md` stays byte-identical and serves as the "no `Done` field" case.

**Contract**: `.gitattributes` already marks the fixtures `-text`. The root-listing assertion in `src-tauri/tests/vault_guard.rs:144-148` gains `Templates`.

#### 3. Integration tests

**File**: `src-tauri/tests/tasks.rs` (new)

**Intent**: Prove every domain operation end to end on a temp copy of the fixture vault, including byte-identity outside the edited fields.

**Contract**: Tests use `common::test_vault()` with `test_tasks_settings()`.
- **Listing:**
  - The sort order follows the rule above.
  - The list-status note and the duplicate-status note appear with a `problem`, and the rest of the list still loads.
  - With `template` pointing at a note inside the tasks folder (written into the temp copy), that note is not listed.
  - A missing folder returns `FolderMissing`, not an empty list.
  - `require_config(None)` returns `TasksNotConfigured`.
- **Rendering (unit tests in `tasks.rs`):**
  - The placeholders are filled.
  - `{{date:YYYY}}` and `<%` are refused.
  - `{{title}}` in frontmatter is checked with `check_plain`.
- **Create:**
  - The new note equals the expected rendered bytes.
  - The CRLF/BOM template keeps CRLF and the BOM.
  - Each of these is refused with nothing written:
    - an existing title (`AlreadyExists`)
    - titles with `:`, `/`, `[`, `#`, a trailing dot, or `CON`
    - an unknown priority
    - a bad due date such as `2026-02-30`
    - a missing template
    - the Templater template
    - the template without `Status`
  - Creating into a missing configured folder returns `FolderMissing`, and the folder is not created.
- **Status change:**
  - `Todo → In progress`: only the `Status` line differs.
  - `→ Done`: `Status` and `Done` change in one write, and nothing else differs.
  - `Done → Todo`: `Done:` is cleared.
  - On the CRLF note, CRLF is kept.
  - On `Przykładowe zadanie.md`, the status changes and the result is `Absent`.
  - The same status gives `Unchanged` and the file's bytes and mtime are untouched.
  - An unknown status is refused.
  - A stale `status_hash` gives `Conflict` and the file is unchanged.
  - A `Done` line edited in between gives `Conflict` and the file is unchanged.
  - A `Done` field added since listing (`done_hash: None`) or removed since listing (`done_hash: Some`) gives `Conflict` and the file is unchanged.

### Success Criteria:

#### Automated Verification:

- Task integration tests pass: `cargo test --test tasks` (in `src-tauri/`)
- Template unit tests pass: `cargo test tasks::` (in `src-tauri/`)
- Full suite passes, including the updated root listing: `cargo test` (in `src-tauri/`)
- Backend builds: `cargo build` (in `src-tauri/`)
- No advisories in newly added crates (`chrono`): `cargo audit` (in `src-tauri/`)

#### Manual Verification:

- Every new file under `src-tauri/tests/fixtures/vault/` is invented: no real task title, issue, hostname or work path.
- A scratch copy of `Templates/Task.md`, inserted into a note through Obsidian's core Templates plugin, gives the same placeholder results as `render_template`. This holds with the plugin's default date format `YYYY-MM-DD` and time format `HH:mm`.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Commands and the Zadania view

### Overview

Thin Tauri commands over `tasks::*`, typed TypeScript wrappers, and the "Zadania" view replacing the greet demo.

### Changes Required:

#### 1. Managed state and commands

**File**: `src-tauri/src/lib.rs`

**Intent**: Expose the three task operations to the webview for the Work space only, and turn every failure, including settings errors stored at startup, into a typed `TaskError`.

**Contract**:
- `.setup` loads settings once and manages two things:
  - `VaultState`, unchanged
  - a new `TasksState(Result<Option<TaskSettings>, VaultError>)` taken from `settings.work.tasks`
- Commands, all `Result<_, TaskError>`:
  - `list_work_tasks() -> Vec<TaskRow>`
  - `create_work_task(new: NewTask) -> String` (vault-relative path)
  - `set_work_task_status(path: String, status: String, status_hash: String, done_hash: Option<String>) -> DoneUpdate`
- The commands also expose the configured `statuses`, `done_status` and `priorities` through `get_work_task_settings() -> TaskSettingsView`, so the UI never hard-codes them.
- A settings error maps to `TaskError::Vault(<stored error>.clone())`. `None` goes through `tasks::require_config`, which returns `TasksNotConfigured`.
- Incoming `path` is parsed with `VaultPath::parse`, and incoming hashes with `SpanHash::from_hex`. Both errors pass through as `Vault(..)`.
- Commands keep Tauri 2's default argument casing, so JS passes camelCase keys (`statusHash`, `doneHash`). Serialised structs (`TaskRow`, `TaskSettingsView`, `NewTask`) use `#[serde(rename_all = "camelCase")]`, so the TS types use camelCase throughout.
- `Local::now()` is called here only.
- The `greet` command is removed. No new capability or permission is needed, because app commands are allowed by default and `src-tauri/capabilities/default.json` stays unchanged.

#### 2. TypeScript API

**File**: `src/tasks/api.ts` (new)

**Intent**: One typed boundary for the task commands, so the view never calls `invoke` with string names directly.

**Contract**:
- `TaskRow`, `NewTask`, `DoneUpdate`, `TaskSettingsView`, `VaultError` (`{ kind: string; detail?: unknown }`) and `TaskError` types mirror the Rust serde shapes. `TaskError` is a union in which `{ kind: "Vault"; detail: VaultError }` nests. A conflict is therefore `e.kind === "Vault" && e.detail.kind === "Conflict"`, exposed as `isConflict(e)`.
- There is one function per command.
- `taskErrorMessage(e: TaskError): string` returns a Polish message for every `TaskError` and `VaultError` kind, with a generic fallback that includes the kind. For example: "Brak pliku ustawień", "Zadania nie są skonfigurowane w settings.json", "Folder zadań nie istnieje: …", "Szablon używa nieobsługiwanego znacznika …", "Notatka zmieniła się w Obsidianie — odświeżono listę".

#### 3. Zadania view

**Files**: `src/tasks/TasksView.tsx` (new), `src/App.tsx`, `src/App.css`

**Intent**: The user-visible FR-013 loop: see the work tasks, change a status, create a task. It never shows an empty list in place of a failure.

**Contract**:
- `App.tsx` renders `<TasksView />` and drops the greet demo, its logos and `assets/react.svg` if they're unused.
- States:
  - loading
  - a load error (full-width message plus a "Spróbuj ponownie" button, no list)
  - loaded and empty ("Brak zadań w folderze …", shown only after a successful load)
  - loaded with rows
- **Row:** title, priority and due date, plus a status `<select>` with the configured statuses. The current unlisted value is shown as an extra disabled option.
  - A row with a `problem` shows `taskErrorMessage` of it, and its select is disabled.
  - While the write is in flight, the select is disabled.
- **After a status change:**
  - On success it reloads the list. On `Absent` it shows the notice "Notatka nie ma pola Done — zmieniono tylko status".
  - On `Conflict` it shows the conflict message and reloads.
  - On any other error it shows the message and keeps the row's previous value.
- A "Pokaż zakończone" checkbox, unchecked by default, hides rows whose status equals `done_status`.
- **The "Nowe zadanie" form:**
  - Fields: title (required), a priority `<select>` (empty option first, hidden when `priorities` is empty), and a due `<input type="date">`.
  - Submitting disables the form.
  - On success it clears the form and reloads.
  - On error it keeps the input and shows the message under the form.
- The view has no text field that writes anything other than these commands.

### Success Criteria:

#### Automated Verification:

- Frontend type check and build pass: `npm run build`
- Backend builds: `cargo build` (in `src-tauri/`)
- Full suite passes: `cargo test` (in `src-tauri/`)

#### Manual Verification:

- With `settings.json` pointing at a scratch copy of the fixture vault (with the tasks block from Phase 1), `npm run tauri dev` shows the Zadania view. The rows are in the expected order, the list-status note shows its problem, and the done task is hidden until "Pokaż zakończone" is ticked.
- Creating "Nowe zadanie testowe" with priority and due date produces a note that Obsidian shows with the filled Properties and heading. Creating the same title again shows the "already exists" message, and a title with `:` is refused before anything is written.
- Changing a task to `Done` and back, with the note open in Obsidian, updates `Status` and `Done` without a conflict prompt. `git diff` on the scratch vault shows only those lines changed.
- Editing the `Status` of a task in Obsidian and then changing it in the app (without reloading) shows the conflict message, and the Obsidian edit survives.
- Each of these shows an explicit message, never an empty list: no `settings.json`, no `tasks` block, a renamed tasks folder, and a renamed template (when creating).

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- Settings: the tasks block and every validation branch.
- Edits: `FieldValue::Empty` rendering (LF/CRLF), `replace_fields` ordering, hash checks on original bytes, duplicate keys.
- Proptest: byte-identity outside both spans for `replace_fields`.
- Template rendering: each placeholder, unsupported tokens, the `{{title}}`-in-frontmatter check.
- Field value parsing: quotes, empty values, list-valued `Status`.

### Integration Tests:

- `tests/tasks.rs` on a temp copy of the fixture vault:
  - listing order and problems
  - every create success and refusal path, each refusal with nothing written
  - every status transition with byte comparison against the untouched fixture
  - conflicts on a stale `Status` or `Done`
  - a missing folder or template

### Manual Testing Steps:

1. Copy `src-tauri/tests/fixtures/vault/` to a scratch folder outside the repo, then `git init` and commit it there.
2. Write `%APPDATA%\com.sebas.day-navigator\settings.json` with `work.vault` pointing at the scratch folder and the tasks block from Phase 1 (`folder: "Tasks"`, `template: "Templates/Task.md"`).
3. Open the scratch folder in Obsidian and run `npm run tauri dev`.
4. Walk through the Phase 3 manual checks, and run `git -C <scratch> diff` after each write.
5. Restore your real `settings.json` afterwards.

## Performance Considerations

Listing reads every `.md` note in one folder (KB each). A few hundred tasks is negligible, and nothing runs in the background. Each status change is one read-modify-replace with at most 3 attempts (F-01).

## Migration Notes

There is no data to migrate. To use the feature against the real work vault, the user adds the `tasks` block to their hand-written `settings.json`. The block needs:
- the real tasks folder and template path
- the status words the vault already uses
- the done status
- the priorities

The template must use core-Templates placeholders only, and must contain `Status`, plus `Priority`, `Due` and `Done` for the fields the app writes. Real values stay only in that file on the user's machine, never in the repo.

## References

- Roadmap item: `context/foundation/roadmap.md` (S-05)
- PRD: `context/foundation/prd.md:124` (FR-013), `:104` (no `Today` writes), `:162-163` (no empty list in place of an error; line-scoped writes), `:192-196` (Access Control)
- Task note shape: `notes/idea.md:28,71`, `context/foundation/shape-notes.md:260-261`
- Vault API: `src-tauri/src/vault/mod.rs:350-426`, `src-tauri/src/vault/edit.rs:35-139`
- Settings parser: `src-tauri/src/settings.rs:13-112`
- Test helper: `src-tauri/tests/common/mod.rs`
- Prior plan: `context/archive/2026-11-20-safe-vault-access/plan.md`
- Brief: `context/changes/work-tasks-from-template/plan-brief.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Task settings and vault extensions

#### Automated

- [x] 1.1 Settings and edit unit tests pass: `cargo test -- settings vault::edit`
- [x] 1.2 Property tests pass: `cargo test proptests`
- [x] 1.3 Full suite still passes: `cargo test`
- [x] 1.4 Backend builds: `cargo build`

#### Manual

- [ ] 1.5 Settings example matches the parser field names

### Phase 2: Task domain

#### Automated

- [ ] 2.1 Task integration tests pass: `cargo test --test tasks`
- [ ] 2.2 Template unit tests pass: `cargo test tasks::`
- [ ] 2.3 Full suite passes, including the updated root listing: `cargo test`
- [ ] 2.4 Backend builds: `cargo build`
- [ ] 2.5 No advisories in newly added crates: `cargo audit`

#### Manual

- [ ] 2.6 New fixture files contain only invented data
- [ ] 2.7 Obsidian core Templates gives the same placeholder results as render_template

### Phase 3: Commands and the Zadania view

#### Automated

- [ ] 3.1 Frontend type check and build pass: `npm run build`
- [ ] 3.2 Backend builds: `cargo build`
- [ ] 3.3 Full suite passes: `cargo test`

#### Manual

- [ ] 3.4 Zadania view lists tasks in order with problems and the done toggle
- [ ] 3.5 Creating a task works and duplicate or invalid titles are refused
- [ ] 3.6 Status change to Done and back edits only Status and Done
- [ ] 3.7 Status edited in Obsidian meanwhile gives a conflict and survives
- [ ] 3.8 Every missing source shows an explicit message, never an empty list
