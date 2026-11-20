# Guarded Vault Access Implementation Plan

## Overview

Build the Rust foundation (roadmap F-01) through which every later slice reads and writes Obsidian notes. It reads and writes only inside the vault root configured for each space (Work, Private) and never shows excluded paths. A write changes only the targeted frontmatter field or heading section and leaves every other byte identical. A write never overwrites a concurrent edit in the same section, and deletion moves notes to the vault's `.trash/`. All of this is proven by fixture and property tests against an invented vault.

## Current State Analysis

- The app is the Tauri 2 starter. `src-tauri/src/lib.rs:1-14` registers one demo command (`greet`) and the opener plugin. There is no settings store, no file access and no `tauri-plugin-fs`.
- `src-tauri/Cargo.toml:20-24` has only `tauri`, `tauri-plugin-opener`, `serde` and `serde_json`. There are no tests anywhere, and `AGENTS.md` ("Testing") says no test framework exists yet.
- `src-tauri/capabilities/default.json:6-9` grants only `core:default` and `opener:default`, so the webview has no file-system permission today. This plan keeps it that way.
- The app identifier `com.sebas.day-navigator` (`src-tauri/tauri.conf.json:5`) makes Tauri's `app_config_dir()` resolve to `%APPDATA%\com.sebas.day-navigator\`. That is where the seed architecture puts settings (`context/foundation/shape-notes.md:262`).
- Real vault shapes the primitives must handle (`notes/idea.md:28`, `context/foundation/shape-notes.md:260-261`):
  - In issue notes the frontmatter `Status` field is a YAML list, so a block spans several lines.
  - In task notes `Status` is a single value.
  - Daily notes live under year folders (`Dziennik/RRRR/RRRR-MM-DD.md`).
  - Both vaults are git repositories, and obsidian-git commits the work vault every 3 h.
- PRD constraints:
  - NFR "Nienaruszalność notatek" (`context/foundation/prd.md:163`): an app write changes only the targeted line or section, and nothing is deleted permanently.
  - Access Control (`prd.md:192-196`): the app touches only configured folders and never creates a configured path silently. Excluded paths are never read, indexed or shown. Work and Private data never cross.
  - NFR "Dane pracy nie opuszczają maszyny" (`prd.md:164`): tests and fixtures use invented data only.

## Desired End State

`day_navigator_lib` exposes a `vault` module (plus `settings`) that later slices call from Rust:

- `VaultRegistry` is built from `settings.json` and stored in Tauri managed state. A missing or invalid settings file becomes a stored typed error, and the app does not crash.
- `Vault` provides these operations for one space:
  - `read_note`, `list`
  - `replace_field`, `replace_section`, `append_to_section`
  - `create_note`, `trash`
- Every operation rejects these cases with a typed `VaultError`:
  - paths outside the root, including through junctions
  - excluded or hidden paths
  - non-Markdown targets
  - a missing root
  - Work and Private roots that overlap
- Replace operations take the hash of the field or section as the caller last saw it, and return `Conflict` if it has changed.
- Writes are atomic and keep the file's creation time and attributes on Windows.
- `cargo test` in `src-tauri/` runs fixture tests against `src-tauri/tests/fixtures/vault/` (invented data) and proptest properties. Together they prove that bytes outside the edited span are identical, across LF, CRLF, BOM, fenced code and frontmatter.

Verify with `cargo test`, `cargo build`, `npm run build` and `cargo audit`. Then run the ignored smoke test once against a scratch copy of the invented vault that is open in Obsidian (see Manual Testing Steps).

### Key Discoveries:

- `src-tauri/src/lib.rs:8-13`: the single `tauri::Builder` chain. A `.setup(...)` hook to register managed state goes here.
- `src-tauri/Cargo.toml:6` uses `edition = "2024"`, and AGENTS.md requires rustc ≥ 1.90. All new crates must build on that.
- `context/foundation/roadmap.md:99`: F-01 is limited to the access contract. Note formats (daily note, inbox, tasks, issues) belong to the slices that use them.
- `context/foundation/roadmap.md:94`: the invented test vault is the verification path for every later slice that writes notes. The fixture and test helper must be reusable.

## What We're NOT Doing

- **No settings UI and no settings writes.** F-01 only reads `settings.json`; the user hand-writes it until S-06 adds the wizard. There is also no hot reload.
- **No frontend access.** No Tauri commands for vault operations, no TypeScript wrapper and no `tauri-plugin-fs`. Each slice adds intent-level commands that own its note format.
- **No note formats.** Daily note, inbox, task and issue layouts are not defined here.
- **No file watching.**
- **No file logging.** F-02 handles surfacing errors in the tray and as notices.
- **No CI.** That is F-03.
- **No trashing folders, restoring from trash, renaming or moving notes.** Renaming and moving stay in Obsidian (PRD Non-Goals).
- **No non-Markdown files.** Only `.md` notes are read, listed or edited.
- **No inserting missing pieces.** There is no "insert section" or "insert frontmatter field". A missing section or field is an error.
- **No full YAML parsing.** Field edits work line by line on top-level keys only, with no nested keys.
- **No setext headings.** Only ATX headings (`#`–`######`) count as section boundaries.
- **No glob patterns in excluded paths.** Entries match by vault-relative prefix only.
- **No Windows Recycle Bin.** Deletion goes to the vault's `.trash/`.
- **No cross-process lock with Obsidian.** A race of a few milliseconds between the last re-read and the replace is accepted (see Critical Implementation Details).

## Implementation Approach

The work splits into three layers so the integrity guarantee can be tested where it is cheapest:

1. **Path guard (Phase 1).** It turns an untrusted vault-relative string into a checked absolute path, or a typed error. Read and list sit directly on it.
2. **Pure span logic (Phase 2).** Bytes in, bytes out. It finds frontmatter field blocks and ATX heading sections, and builds the new file content as `prefix ++ replacement ++ suffix`. Byte-identity is then a property of pure functions that proptest can hammer without touching the disk.
3. **Write path (Phase 3).** It joins the two: re-read, compute the edit, write a temp file in the same folder, check the file hasn't changed, then replace atomically. Every write goes through one helper, so there is exactly one place where bytes reach a vault.

Decisions settled in planning:

- **Excluded paths match by vault-relative prefix.** Matching uses whole path segments and ignores case. For example, `Dostepy` excludes `Dostepy/x.md` but not `Dostepy-stare/x.md`, `Dostepy.md` or `Klienci/Dostepy/x.md`.
- **A section runs to the next heading of the same or higher level.** Headings inside fenced code or frontmatter don't count. A duplicate heading or a missing one is an error.
- **Conflicts are checked per section.** Replace operations carry the section or field hash the caller last saw; a mismatch is a `Conflict`.
- **The generic primitive set ships now.**
- **Only the configured root must exist.** Subfolders under an existing root, and `.trash/`, may be created.
- **Tests are fixture-based plus property-based.**
- **The API is Rust-only.**

## Critical Implementation Details

**Re-checking after canonicalisation.** Containment is checked on the *canonical* target path. That means `dunce::canonicalize` for existing paths, and the deepest existing ancestor for paths not created yet. The excluded and hidden rules are then re-run on the vault-relative remainder of the canonical path. Otherwise a junction inside the vault, such as `Notes/link → Dostepy`, would bypass an exclusion without ever leaving the root.

**ReplaceFileW failure branches.** `ReplaceFileW` is always called with an explicit backup name (`.<file name>.dn-<random>.bak` in the same folder), so the original always has a known name:
- The temp file's handle must be closed before the call (`NamedTempFile::into_temp_path()`). ReplaceFileW opens the replacement with no sharing, and an open handle fails with a sharing violation (32) even when full sharing is allowed (verified on this machine).
- `ERROR_UNABLE_TO_MOVE_REPLACEMENT` (1176): the original now sits under the backup name, and the new content is only in the temp file. Rename the temp file onto the target (`MoveFileExW`).
- `ERROR_UNABLE_TO_MOVE_REPLACEMENT_2` (1177): the original was moved to the backup name, and the temp file keeps its name. Move the backup back onto the target (`MoveFileExW`), restoring the original, then return an error.
- In both branches, never delete the temp or backup file. If the recovery rename also fails, return `ReplaceIncomplete { path, temp, backup }`, which names both files. The ordinary clean-up that runs on every other failure skips these branches.
- After a successful replace, delete the backup.
- `ERROR_ACCESS_DENIED` (5) on a read-only target is not a lock. Check `FILE_ATTRIBUTE_READONLY` first and return `ReadOnly { path }`; do not retry.

**Git line-ending normalisation of fixtures.** The fixture vault deliberately contains CRLF and BOM files. Without a `.gitattributes` rule marking `src-tauri/tests/fixtures/**` as `-text`, git on Windows (with `core.autocrlf`) rewrites them and the byte-identity tests stop meaning anything.

**Accepted race.** The last re-read (step 4 of the write path) and the replace (step 5) are a few milliseconds apart, with no lock shared with Obsidian. A write that lands in that window is lost. This is accepted. obsidian-git and Obsidian's file recovery cover it, and narrowing the window further would need a file lock that Obsidian does not honour.

## Phase 1: Settings and path guard

### Overview

Settings loading, the `Space` / `VaultPath` / `Vault` / `VaultRegistry` types, and all guard rules, with guarded `read_note` and `list`. Also the invented fixture vault, the reusable test helper, and registration in Tauri managed state.

### Changes Required:

#### 1. Dependencies

**File**: `src-tauri/Cargo.toml`

**Intent**: Add the crates the guard needs, plus test-only crates. All of them are pure Rust or Windows-system bindings and need no network or secrets.

**Contract**:
- `[dependencies]`: `thiserror` (2.x), `dunce` (1.x), `sha2` (0.10), `tempfile` (3.x). `tempfile` is a runtime dependency because Phase 3 uses it for temp-file creation.
- `[target.'cfg(windows)'.dependencies]`: `windows-sys` with features `Win32_Foundation` and `Win32_Storage_FileSystem`, used in Phase 3.
- `[dev-dependencies]`: `proptest` (1.x). On Windows only, also `junction` (1.x) for junction-escape tests that run without admin rights.

#### 2. Settings model and loader

**File**: `src-tauri/src/settings.rs` (new). It is top level because S-06 will add non-vault settings.

**Intent**: Parse the hand-written settings file into a typed model, and fail closed on anything malformed. This module reads and never writes.

**Contract**:
- `pub fn load_settings(path: &Path) -> Result<Settings, VaultError>`
  - A missing file returns `NotConfigured { settings_path }`.
  - Unparseable JSON, a wrong `version`, a non-absolute vault path or an invalid excluded entry returns `SettingsInvalid { reason }`.
- Schema, version 1. `private` (or `work`) may be `null` or absent, which means the space is not configured:

```json
{
  "version": 1,
  "spaces": {
    "work":    { "vault": "D:\\vaults\\example-work", "excluded": ["Dostepy", "Klienci/hasla.md"] },
    "private": { "vault": "D:\\vaults\\example-private", "excluded": [] }
  }
}
```

- Excluded entries are parsed with `VaultPath::parse` when settings load.

#### 3. Error type

**File**: `src-tauri/src/vault/error.rs` (new)

**Intent**: Give every refusal and failure its own typed variant, so F-02 and the slices can show an explicit message instead of an empty list.

**Contract**:
- `#[derive(Debug, thiserror::Error, serde::Serialize)] pub enum VaultError`, with at least these variants:
  - `NotConfigured`, `SettingsInvalid`, `SpaceNotConfigured(Space)`
  - `RootMissing { space, root }`, `RootsOverlap`
  - `InvalidPath { path, reason }`, `OutsideVault { path }`
  - `Excluded { path }`, `Hidden { path }`, `NotMarkdown { path }`
  - `NotFound { path }`, `AlreadyExists { path }`, `NotUtf8 { path }`
  - `NoFrontmatter`, `FieldNotFound { key }`, `FieldAmbiguous { key }`
  - `SectionNotFound { heading }`, `SectionAmbiguous { heading, count }`
  - `InvalidValue { reason }`, `Conflict { path }`, `Busy { path }`, `ReadOnly { path }`
  - `ReplaceIncomplete { path, temp, backup }` (Phase 3)
  - `Io { path, kind }`
- **Invariant**: no variant carries note content. Paths are vault-relative, except `RootMissing.root`, which is the user's own configured path.

#### 4. Vault-relative path type

**File**: `src-tauri/src/vault/path.rs` (new)

**Intent**: The only way to name a file inside a vault. It rejects anything that could escape the root, address a Windows device, or reach hidden folders.

**Contract**:
- `pub struct VaultPath` holds normalised segments.
- `VaultPath::parse(&str) -> Result<VaultPath, VaultError>` accepts `/` or `\` as separators. It rejects (`InvalidPath`, or `Hidden` for dot-segments):
  - empty input, a leading separator, a drive prefix (`C:`) or UNC
  - empty, `.` or `..` segments
  - `:` (alternate data streams), control characters and `<>"|?*`
  - segments ending in a dot or a space
  - reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`), with or without an extension
  - any segment starting with `.`
- `Display` joins segments with `/`.
- `fn starts_with_ci(&self, prefix: &VaultPath) -> bool` compares lowercased whole segments.
- `fn is_markdown(&self) -> bool` is true for an `.md` extension, ignoring case.

#### 5. Vault and registry

**File**: `src-tauri/src/vault/mod.rs` (new). It declares the submodules and re-exports the public API.

**Intent**: Bind one configured root and its exclusions to one `Space`. Check the root and containment on *every* operation, because a drive can disappear after startup. Implement the read-only operations.

**Contract**:
- `pub enum Space { Work, Private }`
- `pub struct VaultRegistry` with:
  - `from_settings(&Settings) -> VaultRegistry`
  - `vault(&self, Space) -> Result<Vault, VaultError>`
- `vault()` checks, in order:
  1. The space is configured, else `SpaceNotConfigured`.
  2. The root is an existing directory, else `RootMissing`.
  3. The canonical roots are neither equal nor nested, else `RootsOverlap`. This applies only when the other space's root also exists.
- `Vault` has no method that takes another space's path. Separation holds by construction.
- `vault/*` and `settings.rs` do not import `tauri`. `VaultState` and the `.setup` hook live only in `lib.rs`. Test binaries do not get Tauri's Common-Controls manifest (tauri#13419, fixed only in tauri-build 2.8), so a test that reaches Tauri symbols crashes with `0xc0000139`.
- `Vault::resolve(&VaultPath) -> Result<PathBuf, VaultError>` is the internal guard:
  - `Excluded` if any excluded entry is a case-insensitive prefix.
  - Joins the path onto the canonical root and canonicalises the target, or its deepest existing ancestor.
  - `OutsideVault` if the result does not start with the canonical root.
  - Re-runs the excluded and hidden checks on the canonical relative remainder.
- `Vault::read_note(&VaultPath) -> Result<Note, VaultError>`:
  - The target must be `.md` and must exist (`NotFound`).
  - The bytes must be UTF-8, after an optional BOM (`NotUtf8`).
- `Note` holds the vault path and raw bytes. It exposes `text()` without the BOM. Phase 2 adds `field()` and `section()`.
- `Vault::list(dir: Option<&VaultPath>) -> Result<Vec<Entry>, VaultError>`:
  - Non-recursive, sorted by name.
  - `Entry { name, path: VaultPath, kind: Dir | Note }`.
  - Omits dot-entries, non-`.md` files, excluded entries and entries whose canonical path leaves the root.
  - Listing an excluded directory returns `Excluded`.

#### 6. Tauri wiring

**File**: `src-tauri/src/lib.rs`

**Intent**: Load settings once at startup and keep the result, success or error, in managed state for later slices. Settings errors must not abort startup.

**Contract**:
- Declare `pub mod settings; pub mod vault;`.
- Add `.setup(...)`:
  - Resolve `app.path().app_config_dir()` joined with `settings.json`.
  - `app.manage(VaultState(Result<VaultRegistry, VaultError>))`.
  - A failure of `app_config_dir()` is stored as an error too.
- The `greet` demo and the opener plugin are left as they are.

#### 7. Invented fixture vault, test helper and guard tests

**Files**:
- `src-tauri/tests/fixtures/vault/**` (new)
- `.gitattributes` (new)
- `src-tauri/tests/common/mod.rs` (new)
- `src-tauri/tests/vault_guard.rs` (new)

**Intent**: A small, obviously invented vault that this phase and every later slice tests against, plus a helper that copies it into a temp dir and builds a registry, so that tests never touch the fixture itself.

**Contract**:
- The fixture contains only invented content:
  - `Issues/Fake issue 101.md`: frontmatter with a `Status` list, plus `Labels` and `Priority`.
  - `Tasks/Przykładowe zadanie.md`: a single `Status`. The name includes non-ASCII on purpose.
  - `Dziennik/2026/2026-01-05.md`: LF line endings, with `## Plan`, `### Notatki`, `## Log`, and a fenced block that contains `## Not a heading`.
  - `Dziennik/2026/2026-01-06.md`: CRLF line endings and a UTF-8 BOM.
  - `Notes/duplicate-headings.md`.
  - `Dostepy/invented-secret.md` (excluded in the test settings).
  - `Dostepy-stare/old.md`.
  - `.obsidian/app.json` (dummy).
  - `notes.txt` (not Markdown).
  - `.dn-test-vault`, a marker file that the smoke test requires.
- `.gitattributes` has `src-tauri/tests/fixtures/** -text`.
- `common::test_vault() -> (TempDir, VaultRegistry)` uses the Work space with `excluded: ["Dostepy"]`. A variant takes custom settings, for overlap and missing-root cases.

### Success Criteria:

#### Automated Verification:

- Backend builds: `cargo build` (in `src-tauri/`)
- Guard tests pass: `cargo test --test vault_guard` (in `src-tauri/`). They cover:
  - traversal, absolute, UNC, ADS and reserved-name rejection
  - dot-segment rejection
  - excluded prefix (case-insensitive, `Dostepy-stare/old.md` and `Dostepy.md`-style siblings not excluded)
  - list omitting excluded, hidden and non-`.md` entries
  - `RootMissing` after the root is deleted mid-test
  - `RootsOverlap` for nested roots
  - `SpaceNotConfigured`
  - junction escape and junction-into-excluded rejection (Windows)
- Settings unit tests pass: `cargo test settings` (in `src-tauri/`). They cover missing file → `NotConfigured`, malformed JSON, a relative vault path, an invalid excluded entry and `null` space.
- Frontend type check and build pass: `npm run build`
- No advisories in newly added crates: `cargo audit` (in `src-tauri/`)

#### Manual Verification:

- `npm run tauri dev` starts the app normally with no `settings.json` in `%APPDATA%\com.sebas.day-navigator\`, and again with a hand-written one pointing at a scratch copy of the fixture vault (no panic in the console).
- Every file in `src-tauri/tests/fixtures/vault/` is invented: no real note, issue, hostname or work path.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Markdown spans and pure edits

### Overview

Pure functions that locate a frontmatter field block or a heading section in a note's bytes, hash it, and build the edited bytes. Proptest proves that everything outside the span is untouched.

### Changes Required:

#### 1. Span finding

**File**: `src-tauri/src/vault/markdown.rs` (new)

**Intent**: Find byte ranges with rules matching what Obsidian treats as frontmatter and sections, ignoring look-alikes inside fenced code.

**Contract**:
- `pub struct Heading { level: u8 /* 1..=6 */, text: String }`
- `pub struct Span { start: usize, end: usize }`, as byte offsets aligned to line starts.
- `detect_eol(&[u8]) -> Eol`: the first line ending in the file decides. LF if there is none.
- **Frontmatter**:
  - It exists only if the file (after the BOM) starts with `---` plus EOL.
  - It ends at the next line that is exactly `---` or `...`.
- **`find_field(bytes, key) -> Result<Span, VaultError>`**:
  - It matches a top-level line `key:` at column 0, with a case-sensitive key.
  - The block continues over following lines that are indented or start with `- `.
  - It stops at the next column-0 key, a column-0 `#` comment, a blank line or the end of the frontmatter. None of these belong to the block.
  - Errors: `NoFrontmatter`, `FieldNotFound`, or `FieldAmbiguous` for a duplicate key.
- **`find_section(bytes, &Heading) -> Result<Span, VaultError>`**:
  - The span is the section *body*: from the line after the matching ATX heading up to the next ATX heading of the same or higher level, or EOF.
  - A heading matches when its level is equal and its text is exactly equal, after trimming and stripping optional closing `#`s.
  - An ATX heading follows CommonMark: 0–3 spaces of indent, then 1–6 `#`, then a space, a tab or end of line. A line like `#tag` (an Obsidian tag) is not a heading, and a line indented 4 or more spaces is not one either.
  - Lines inside fenced code (```` ``` ```` or `~~~`, closed by the same fence) and inside frontmatter are never headings.
  - Errors: `SectionNotFound`, or `SectionAmbiguous { count }`.
- **`span_hash(bytes, Span) -> SpanHash`**: SHA-256 hex of the span bytes. `SpanHash` is a newtype with equality.

#### 2. Pure edits

**File**: `src-tauri/src/vault/edit.rs` (new)

**Intent**: Build the complete new file bytes for each primitive as `original[..start] ++ replacement ++ original[end..]`, with inserted text normalised to the file's EOL.

**Contract**:
- `replace_field(bytes, key, &FieldValue, expected: &SpanHash) -> Result<Vec<u8>, VaultError>`
  - `FieldValue::Scalar(String)` renders as `key: value`.
  - `FieldValue::List(Vec<String>)` renders as `key:` plus one `<indent>- item` per element. The indent is copied from the existing block's first item, or two spaces if there is none. An empty list renders as `key: []`.
  - A value containing CR or LF returns `InvalidValue`.
  - Values are written verbatim, with no quoting. A value that would need YAML quoting returns `InvalidValue`. That covers leading or trailing whitespace, `: `, ` #`, a leading one of `[ ] { } & * ! | > ' " % @ `` ` ``, a leading `- ` or `? `, and an empty string. This way a write never breaks Obsidian's Properties block.
- `replace_section(bytes, &Heading, new_body: &str, expected: &SpanHash) -> Result<Vec<u8>, VaultError>`
  - Adds a trailing EOL if the body lacks one and the section is not at EOF.
- `append_to_section(bytes, &Heading, lines: &str) -> Result<Vec<u8>, VaultError>`
  - Inserts after the last non-blank line of the body, so blank lines before the next heading stay put.
  - With a blank body, it inserts directly after the heading line.
  - If the heading is the last line with no EOL, it first adds one EOL.
- In both replace functions, if the hash of the span found in `bytes` ≠ `expected`, they return `Conflict`.
- The BOM, if present, always stays at offset 0.
- `Note::field(key) -> Result<FieldView { value_text, hash }, VaultError>` and `Note::section(&Heading) -> Result<SectionView { body, hash }, VaultError>` live in `vault/mod.rs`. Callers get the expected hash from these.

#### 3. Unit and property tests

**File**: `src-tauri/src/vault/markdown.rs` and `src-tauri/src/vault/edit.rs` (`#[cfg(test)]` modules)

**Intent**: Cover each rule with a named example, and prove byte-identity on generated notes.

**Contract**: Property tests live in a `mod proptests` inside each file, so `cargo test proptests` selects them.
- **Example tests**:
  - `## Plan` / a / `### Notatki` / b / `## Log`: the span covers a, `### Notatki` and b.
  - A heading inside a fence is ignored.
  - A line-start `#tag` inside `## Plan` does not end the section.
  - A duplicate heading gives `SectionAmbiguous`.
  - A list-valued `Status` is replaced as one block.
  - A CRLF file gets CRLF in the inserted text.
  - The BOM is preserved.
  - A heading on the last line with no EOL.
  - `key: []` is rendered for an empty list.
- **Proptest generators** build notes from fragments:
  - optional BOM
  - optional frontmatter with scalar and list keys
  - ATX headings of levels 1–6, including the target, fenced blocks containing heading look-alikes, text and blank lines
  - LF or CRLF, with or without a trailing EOL
- **Properties**:
  1. For every successful edit, the bytes before `span.start` and after `span.end` equal the original.
  2. Replacing a section with its own current body returns the original bytes. For fields this holds only when the generator emits them in canonical form (`key: value`, or a block list with a consistent indent), because `FieldValue` rendering normalises the rest.
  3. Span finding never panics, and spans start and end on line boundaries.
  4. Inserted text contains only the file's EOL.

### Success Criteria:

#### Automated Verification:

- Span and edit unit tests pass: `cargo test -- vault::markdown vault::edit` (in `src-tauri/`)
- Property tests pass with default case count: `cargo test proptests` (in `src-tauri/`)
- Backend builds: `cargo build` (in `src-tauri/`)

#### Manual Verification:

- Open a scratch copy of the fixture vault in Obsidian. The collapsible range Obsidian shows for `## Plan` in `Dziennik/2026/2026-01-05.md` matches the span the example test expects (it includes `### Notatki`, and the fenced `## Not a heading` is not a heading).

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Safe writes and trash

### Overview

One atomic, conflict-checked write path that every mutating operation uses. It adds the `Vault` edit methods, `create_note` and `trash`, plus integration tests on temp copies of the fixture vault.

### Changes Required:

#### 1. File-system primitives

**File**: `src-tauri/src/vault/fsops.rs` (new)

**Intent**: Keep the only code that writes into a vault in one place, so that no partial file is ever visible and Obsidian's file metadata survives an edit.

**Contract**:
- `replace_atomic(target, new_bytes, expected_current: &[u8]) -> Result<(), VaultError>`:
  0. Delete this note's leftover `.<file name>.dn-*.tmp` and `.dn-*.bak` files in the target's folder that are older than 1 minute. These are leftovers from a crash, and without this step obsidian-git would commit them.
  1. Write a temp file in the target's folder, named `.<file name>.dn-<random>.tmp`. The dot prefix keeps Obsidian from showing it. Flush and sync it, then close its handle (`into_temp_path()`).
  2. Re-read the target. If its bytes ≠ `expected_current`, delete the temp file and return an internal `Changed` signal. If the target is read-only, delete the temp file and return `ReadOnly`.
  3. Replace. On Windows this uses `ReplaceFileW(target, temp, backup, REPLACEFILE_IGNORE_MERGE_ERRORS | REPLACEFILE_IGNORE_ACL_ERRORS, …)`, with `backup` = `.<file name>.dn-<random>.bak` in the same folder. This keeps the creation time, attributes and ACLs. Delete the backup after success. Elsewhere it uses `std::fs::rename`.
  4. On a sharing violation (32) or lock violation (33), retry with backoff (about 5 tries, ≤ 1 s in total), then return `Busy`.
  5. Handle errors 1176 and 1177 as described in Critical Implementation Details. The Windows replace call sits behind an injectable function, so tests can force these branches.
  6. Delete the temp file on every other failure.
- `create_new(target, bytes) -> Result<(), VaultError>`: temp file, then `NamedTempFile::persist_noclobber`. An existing target returns `AlreadyExists`.
- `move_noclobber(from, to) -> Result<(), VaultError>`: Windows `MoveFileExW(from, to, 0)`, which does not replace. Elsewhere it uses `hard_link` plus `remove_file`.

#### 2. Vault edit operations

**File**: `src-tauri/src/vault/mod.rs`

**Intent**: Public mutating API. Each operation is guard → read → pure edit → `replace_atomic`. It recomputes from fresh bytes when the file changed between read and replace.

**Contract**:
- `replace_field(&VaultPath, key, FieldValue, expected: SpanHash)`, `replace_section(&VaultPath, &Heading, body, expected: SpanHash)` and `append_to_section(&VaultPath, &Heading, lines)`, each returning `Result<(), VaultError>`.
- On `Changed`, recompute from the newly read bytes, up to 3 attempts in total, then return `Conflict`.
- The section-hash check in Phase 2 runs on each attempt. So an Obsidian edit elsewhere in the note is kept, and an edit to the same section returns `Conflict`.
- `create_note(&VaultPath, content: &str) -> Result<(), VaultError>`:
  - The target must be `.md` and must not exist.
  - It creates missing parent folders only under the existing canonical root. The containment and exclusion check is re-run after the folders are created and before the file is written.
  - The content is written as given.
- `trash(&VaultPath) -> Result<String, VaultError>`:
  - The target must be an existing, non-excluded `.md` file.
  - The destination comes from an internal `trash_dest()`, not from `VaultPath` or `resolve()`, which reject dot-segments. It is the only code allowed to name `.trash`:
    - Only the root-level `.trash` is accepted. It is created under the root if missing, then canonicalised and checked for containment, so a `.trash` junction pointing outside the vault is refused (`OutsideVault`).
    - The hidden rule is skipped only for that one segment.
  - The destination is `.trash/<file name>`, flat, as in Obsidian. On collision it becomes `<stem> 2.md`, `<stem> 3.md` and so on, matching Obsidian's numbering. An existing trash entry is never overwritten.
  - It moves the file with `move_noclobber` and returns the file name inside `.trash/`.
- All mutating operations first check `RootMissing` through `VaultRegistry::vault`, so a missing root never leads to creating anything.

#### 3. Integration tests and smoke test

**File**: `src-tauri/tests/vault_write.rs` (new)

**Intent**: Prove each operation end to end on a temp copy of the fixture vault, including the integrity guarantee at the file level.

**Contract**:
- One test per operation. Each compares bytes outside the edited span against the untouched fixture file.
- Conflict cases:
  - a changed section → `Conflict`
  - a change elsewhere in the note → success, with that change preserved
- `create_note`:
  - on an existing file → `AlreadyExists`
  - creates `Dziennik/2027/` inside the root
  - root deleted → `RootMissing` and nothing created
- Writes or trash into excluded or hidden paths are refused.
- Trash name collision gets a ` 2` suffix, and the existing trash entry is unchanged. A `.trash` junction pointing outside the root is refused (Windows).
- Replace failure branches, forced through the injectable replace function:
  - 1176: the target ends up with the new content.
  - 1177: the target ends up with the original content.
  - In neither branch is any temp or backup file deleted before recovery.
- A read-only target returns `ReadOnly` without retrying.
- Leftover `.dn-*.tmp` and `.dn-*.bak` files older than 1 minute are removed by the next write to that note.
- No `.dn-*.tmp` or `.dn-*.bak` files remain after any test.
- On Windows only: creation time is unchanged after `replace_section`.
- `#[ignore] smoke_against_scratch_vault`: it runs each operation against the directory in the env var `DN_SMOKE_VAULT`. It refuses to run unless that directory contains the fixture's `.dn-test-vault` marker, so it can only ever target a scratch copy of the invented vault.

#### 4. Contributor guide

**File**: `AGENTS.md`

**Intent**: Replace the "No test framework is configured yet" note with how tests now run, and the invented-vault rule.

**Contract**:
- The "Testing" section states that `cargo test` runs in `src-tauri/`, that fixtures live in `src-tauri/tests/fixtures/vault/` (invented data only, kept byte-exact by `.gitattributes`), and that new vault-writing slices reuse `tests/common`.

### Success Criteria:

#### Automated Verification:

- Write-path integration tests pass: `cargo test --test vault_write` (in `src-tauri/`)
- Full suite passes: `cargo test` (in `src-tauri/`)
- Backend builds: `cargo build` (in `src-tauri/`)
- Frontend type check and build pass: `npm run build`
- No advisories in newly added crates: `cargo audit` (in `src-tauri/`)

#### Manual Verification:

- With a scratch copy of the fixture vault open in Obsidian (target note open in the editor), `DN_SMOKE_VAULT=<copy> cargo test --test vault_write -- --ignored smoke_against_scratch_vault` succeeds. Obsidian then shows the updated sections without a conflict prompt, and `git diff --no-index` against the fixture shows only the targeted lines changed.
- After the smoke run, the trashed note appears in Obsidian's `.trash` folder (Obsidian's "Deleted files" view if enabled), and no `.dn-*.tmp` file is visible in the vault.
- The `AGENTS.md` Testing section reads correctly and matches the commands above.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Testing Strategy

### Unit Tests:

- `VaultPath::parse`: every rejection class, plus valid non-ASCII and space-containing names.
- Settings loader: missing, malformed, wrong version, relative root, invalid excluded entry, `null` space.
- Span finding:
  - same-or-higher boundary, fences (backtick and tilde, nested lengths)
  - frontmatter look-alikes (`---` later in the body)
  - duplicates
  - heading with closing `#`s
- Pure edits: list and scalar fields, append placement before trailing blank lines, EOL normalisation, BOM.
- Proptest properties: outside-span identity, identity on no-op replace, no panics, line-aligned spans, EOL consistency.

### Integration Tests:

- Guard (`vault_guard.rs`):
  - exclusion semantics, hidden folders, non-Markdown
  - missing root mid-run, overlap, unconfigured space
  - junction escape (Windows)
- Writes (`vault_write.rs`):
  - each primitive against the fixture
  - section-level conflict vs edit elsewhere
  - create with parents, trash collisions
  - temp-file cleanup, creation time preserved (Windows)
  - forced 1176/1177 replace branches, read-only target, `.trash` junction

### Manual Testing Steps:

1. Copy `src-tauri/tests/fixtures/vault/` to a scratch folder outside the repo, and `git init` + commit it so diffs are visible.
2. Open the scratch folder as a vault in Obsidian and open `Dziennik/2026/2026-01-05.md`.
3. Run `DN_SMOKE_VAULT=<scratch> cargo test --test vault_write -- --ignored smoke_against_scratch_vault` in `src-tauri/`. In PowerShell, set `$env:DN_SMOKE_VAULT` first.
4. In Obsidian, confirm that the `## Plan` content and the `Status` field changed, the log line was appended and the trashed note sits in `.trash/`.
5. Run `git -C <scratch> diff`. Only the targeted lines and sections differ, and line endings in the CRLF note are unchanged.
6. Start `npm run tauri dev` with and without `settings.json` and confirm that it starts cleanly both times.

## Performance Considerations

Notes are small (KB). Each operation reads the whole file, at most 3 times on retries, and hashes one span, which is negligible. Nothing runs in the background, so the NFR "Lekkość w tle" is unaffected.

## Migration Notes

There is no existing data to migrate. Until S-06 ships the wizard, the user creates `%APPDATA%\com.sebas.day-navigator\settings.json` by hand in the schema shown in Phase 1. Real vault paths live only in that file on the user's machine, never in the repo.

## References

- Roadmap item: `context/foundation/roadmap.md:89-100` (F-01)
- PRD: `context/foundation/prd.md:163` (NFR integrity), `:164` (work data), `:192-196` (Access Control)
- Vault shapes: `notes/idea.md:28`, `context/foundation/shape-notes.md:260-262`
- Tauri builder: `src-tauri/src/lib.rs:8-13`
- Capabilities (no fs plugin): `src-tauri/capabilities/default.json:6-9`
- Brief: `context/changes/safe-vault-access/plan-brief.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Settings and path guard

#### Automated

- [ ] 1.1 Backend builds: `cargo build`
- [ ] 1.2 Guard tests pass: `cargo test --test vault_guard`
- [ ] 1.3 Settings unit tests pass: `cargo test settings`
- [ ] 1.4 Frontend type check and build pass: `npm run build`
- [ ] 1.5 No advisories in newly added crates: `cargo audit`

#### Manual

- [ ] 1.6 App starts cleanly with and without settings.json
- [ ] 1.7 Fixture vault contains only invented data

### Phase 2: Markdown spans and pure edits

#### Automated

- [ ] 2.1 Span and edit unit tests pass
- [ ] 2.2 Property tests pass
- [ ] 2.3 Backend builds: `cargo build`

#### Manual

- [ ] 2.4 Obsidian section folding matches expected spans

### Phase 3: Safe writes and trash

#### Automated

- [ ] 3.1 Write-path integration tests pass: `cargo test --test vault_write`
- [ ] 3.2 Full suite passes: `cargo test`
- [ ] 3.3 Backend builds: `cargo build`
- [ ] 3.4 Frontend type check and build pass: `npm run build`
- [ ] 3.5 No advisories in newly added crates: `cargo audit`

#### Manual

- [ ] 3.6 Smoke test on scratch vault open in Obsidian shows only targeted changes
- [ ] 3.7 Trashed note in .trash and no temp files visible
- [ ] 3.8 AGENTS.md Testing section matches the commands
