<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Work Tasks from Template

- **Plan**: context/changes/work-tasks-from-template/plan.md
- **Scope**: Full plan (all Automated rows done; all Manual rows still pending)
- **Reviewed phases**: 1, 2, 3
- **Date**: 2026-11-21
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 4 warnings, 6 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | WARNING |
| Architecture | WARNING |
| Pattern Consistency | WARNING |
| Success Criteria | WARNING |

Automated checks run during the review, all passing:
- `cargo test`: 141 passed, 0 failed, 1 ignored. The ignored test is the existing manual smoke test `vault_write.rs:786`.
- `cargo test -- settings vault::edit`: 36 + 1 passed.
- `cargo test proptests`: 6 passed.
- `cargo test --test tasks`: 19 passed.
- `cargo test tasks::`: 7 passed.
- `cargo build`: OK.
- `npm run build`: OK.
- `cargo audit`: 0 vulnerabilities, 6 allowed warnings. One of them is `chrono` RUSTSEC-2026-0543 (unmaintained).

Benign extras, not counted as findings:
- `TaskSettingsView.folder`
- the hint shown when every task is done
- the explicit trailing-dot title refusal

## Findings

### F1 — Status command can rewrite any vault note, not only tasks

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/tasks.rs:375 (set_status), src-tauri/src/lib.rs:86
- **Detail**: `set_work_task_status` parses the `path` it gets from the webview and passes it to `tasks::set_status`. That function checks only that `status` is in `cfg.statuses`. It never checks that `path` is a direct child of `cfg.folder`, or that it isn't `cfg.template`.
  - **Reach:** any non-excluded `.md` file in the work vault that has a scalar `Status` field. `VaultPath::parse` and `resolve` still block escapes from the vault root.
  - **Example target:** `Templates/Task.md`, which lives outside `Tasks/`, so the listing's template skip does not apply. Setting it to Done would write `Status: Done` and `Done: <today>` into the template.
  - **Why it matters:** the listing is narrow but the write command is wide. That conflicts with the rule "read and write only inside configured folders". `tauri.conf.json` has `"csp": null` (pre-existing), so any script injection could reach this command.
- **Fix**: In `tasks::set_status`, require `path.starts_with_ci(&cfg.folder)` with exactly one extra segment, and refuse `cfg.template`. Return a new `TaskError::NotATask { path }`, add its Polish message in `api.ts`, and add integration tests targeting `Templates/Task.md` and an `Issues/` note.
  - Strength: The rule then lives in the tauri-free domain, next to the matching rule in `list_tasks`, and is tested there.
  - Tradeoff: A new error variant, plus a TS message and a test.
  - Confidence: HIGH — `list_tasks` already has the same segment-count + `starts_with_ci` logic to reuse.
  - Blind spot: CSP hardening (`"csp": null`) is out of scope for this slice; it is worth its own follow-up.
- **Decision**: FIXED. `is_task_path` guard plus `TaskError::NotATask`, a Polish message, and test `status_change_outside_the_tasks_folder_is_refused`. CSP is not changed.

### F2 — ReplaceIncomplete hides the backup file and leaves the row stale

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/tasks/api.ts:138, src/tasks/TasksView.tsx:61-70
- **Detail**: The message drops two things from the `ReplaceIncomplete` error:
  - `detail.temp`
  - `detail.backup`

  `fsops.rs` notes that the `.dn-*.bak` file can be the only copy left of the original note, so the user loses the one pointer they need to recover it by hand. Separately, `changeStatus` skips `reload()` for every error except a conflict. After an incomplete replace, the row keeps showing a state the disk no longer has.
- **Fix**: Include `temp` and `backup` in the `ReplaceIncomplete` message, and reload after `ReplaceIncomplete` (and `Io`) as well as after a conflict.
- **Decision**: FIXED. The message names the backup and temp files. `needsReload()` reloads after Conflict, ReplaceIncomplete, Io and NotFound.

### F3 — A deleted tasks folder can be silently recreated on create

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture
- **Location**: src-tauri/src/vault/mod.rs:428-430, src-tauri/src/tasks.rs:327
- **Detail**: `create_task` checks that the folder exists through `vault.list`, then calls `vault.create_note`, which runs `fs::create_dir_all(parent)`. If the folder is deleted or renamed between those two calls, it is recreated without a word. That breaks "never create a configured path silently". The plan assumed "this slice creates none", but the primitive it calls does create folders, so this is a flaw in the plan as well as in the code. The race window is tiny.
- **Fix A ⭐ Recommended**: Add a `create_note` variant (or a flag) that requires the parent to exist, and use it in `create_task`.
  - Strength: Closes the gap at the primitive. A future slice that writes into a configured folder gets the safe behaviour for free.
  - Tradeoff: One more public `Vault` method, plus a test.
  - Confidence: HIGH — the change is local to `create_note`; `fsops::create_new` already fails if the parent is missing.
  - Blind spot: The error kind for a missing parent: map `NotFound` → `FolderMissing` in `create_task`.
- **Fix B**: Accept the race and document it in the plan as a known limit.
  - Strength: No code change.
  - Tradeoff: The project rule stays violated in principle.
  - Confidence: MED — only harmless if the user never renames the folder while the app is creating a task.
  - Blind spot: S-06 and later slices may copy the pattern.
- **Decision**: FIXED via Fix A. Added `Vault::create_note_in_existing_folder`; `create_task` maps its `NotFound` to `FolderMissing`. Test: `create_note_in_existing_folder_refuses_a_missing_parent_and_creates_nothing`.

### F4 — Priority and Due read errors show as blank

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/tasks.rs:158-165
- **Detail**: Both fields are read with `note.field("Priority").ok().and_then(|f| scalar_value(..).ok().flatten())`. If the field is duplicated (`FieldAmbiguous`), list-valued, or uses a layout the parser doesn't support:
  - the row shows no priority or due date;
  - the task sorts last;
  - nothing is flagged.

  This is a field-level case of "never render empty in place of a failed source". The plan defined problems only for `Status`/`Done`, so it isn't drift.
- **Fix A ⭐ Recommended**: Add `fieldProblems` to `TaskRow` for errors other than `FieldNotFound`, and show them in the row without disabling the status select.
  - Strength: The failure is surfaced, and the status stays editable, since `Status`/`Done` are fine.
  - Tradeoff: A new field in `TaskRow`, the TS type and the view.
  - Confidence: HIGH — the error is already available at the call site.
  - Blind spot: How much row clutter is acceptable in the UI.
- **Fix B**: Map those errors into the existing `problem`.
  - Strength: Two lines, no new shape.
  - Tradeoff: The status select is disabled for a note whose `Status` is perfectly editable.
  - Confidence: HIGH.
  - Blind spot: None significant.
- **Decision**: FIXED via Fix A. `TaskRow.fieldProblems` is shown in the row, and the status select stays enabled. Test: `unreadable_priority_or_due_is_a_field_problem_not_a_blank`.

### F5 — `DoneUpdate::Unchanged` means two things

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Architecture
- **Location**: src-tauri/src/tasks.rs:396, 424
- **Detail**: `Unchanged` covers two cases that differ in whether anything was written:
  - Re-selecting the same status: nothing is written.
  - A move between two non-done statuses (`(false, _) => DoneUpdate::Unchanged`): `Status` is written.

  The test at `tests/tasks.rs:322` asserts `Unchanged` after Todo→In progress. The UI treats both the same today, but a later caller reading `Unchanged` as "no write" would be wrong. The plan has the same ambiguity.
- **Fix**: Add `DoneUpdate::NoWrite` for the same-status early return (and its TS literal); keep `Unchanged` meaning "Status written, Done untouched".
- **Decision**: FIXED. `DoneUpdate::NoWrite` was added and documented, along with its TS literal.

### F6 — Gaps in the serde-contract, from_hex and leftover-file tests

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/vault/markdown.rs:73-91, src-tauri/tests/tasks.rs, src-tauri/src/vault/edit.rs:699-721
- **Detail**: Every test the plan lists exists. Four contract points the TS boundary depends on are not covered:
  - **`SpanHash::from_hex`:** no unit test for uppercase, 63 or 65 characters, or non-hex input.
  - **The JSON shape:** no test pins what the webview receives:
    - `TaskRow` has camelCase `statusHash`/`doneHash`, the hash as a bare string, and `path` as a string;
    - `TaskError` nests as `{kind:"Vault",detail:{kind:"Conflict",…}}`.

    `isConflict` and every TS type rely on this.
  - **Temp files:** no task write test checks that `.dn-*.tmp` files are cleaned up, as `tests/vault_write.rs` does.
  - **The new proptest:**
    - it uses `prop_assume!(len >= 2)`, which rejects about 66% of cases;
    - it always edits fields in file order, so the reverse order (`Done` before `Status`) is covered by only one unit test.
- **Fix**: Add a `from_hex` unit test, a `serde_json::to_value` shape test for `TaskRow`/`TaskError`/`DoneUpdate`, a leftover check in the task write tests, and iterate over both orders of a field pair in the proptest.
- **Decision**: FIXED.
  - Added `span_hash_from_hex_accepts_only_64_lowercase_hex` and `serialised_shapes_match_the_typescript_types`.
  - Moved `assert_no_leftovers` into `tests/common` and used it in the task write tests.
  - The proptest now iterates every ordered field pair, with no `prop_assume`.

### F7 — Sync commands on the main thread and a stale reload race

- **Severity**: 💡 OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/lib.rs:40-98, src/tasks/TasksView.tsx:31-41
- **Detail**: Two related problems:
  - **Blocking:** all four commands are plain sync `fn`s, so Tauri 2 runs them on the main thread. `list_work_tasks` reads every note while the window is blocked. On a OneDrive or Files-On-Demand vault, that can freeze the UI.
  - **Stale reload:** in the view, every write calls `reload()` and the last response to resolve wins, so an older listing can overwrite a newer one. The result is stale hashes and a spurious `Conflict` on the next edit. Today the sync commands hide this race, so fixing the blocking alone would expose it.
- **Fix**: Make the commands `async` (`spawn_blocking` around the vault calls) and, in the same change, guard `reload()` with a request counter in a ref so only the latest response is applied.
  - Strength: Fixes the UI freeze and the race it would expose, together.
  - Tradeoff: Touches every command; `tauri::State` lifetimes in async commands need care.
  - Confidence: MED — `apply`'s byte re-check already makes concurrent writes safe; the rest is mechanical.
  - Blind spot: Not measured; it may not matter at the expected size (a few hundred notes).
- **Decision**: FIXED. The three vault commands are `async` and run the I/O through `tauri::async_runtime::spawn_blocking`. `reload()` uses a request-id ref and applies only the latest response.

### F8 — Template rendering edge cases

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/tasks.rs:210-246
- **Detail**: Three edge cases:
  - **The `{{title}}` check ignores where the title lands.** `check_plain` checks the title alone, not its YAML context. It still lets through:
    - `,` and `'`, which break `aliases: [{{title}}]` or `'{{title}}'`;
    - `~`, `true` or `123`, which change the value's YAML type.

    The effect is limited to a malformed Properties block in the new note.
  - **`TemplateUnsupported.placeholder` can carry template content.** It is the text up to the next `}}`, found across lines. A stray `{{` therefore puts arbitrary body text into the error, against the module's "no error carries note content" invariant.
  - **The first offending token isn't always reported.** `<%` is checked before `{{…}}`, so the error doesn't always name the earliest token, as the plan asked.
- **Fix**: In the frontmatter, refuse titles containing `,` or `'`. Cut the reported token at the first line break and cap it at about 40 characters. Report whichever unsupported token comes first in the template.
- **Decision**: FIXED.
  - `first_unsupported` names the earliest token, and `token_label` cuts it to the first line, at most 40 characters.
  - A title inside the frontmatter now refuses `,` and `'`.
  - Three unit tests were added.

### F9 — Criterion 2.5 ticked despite the chrono advisory; manual checks pending

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/work-tasks-from-template/plan.md:501
- **Detail**: Criterion 2.5 says "No advisories in newly added crates (chrono)" and is `[x]`, but `cargo audit` reports RUSTSEC-2026-0543 (chrono unmaintained). `change.md` records it as an accepted exception, but the Progress row reads as a clean pass. Separately, all 8 Manual rows (1.5, 2.6, 2.7, 3.4–3.8) are still `[ ]`; they are pending, not false ticks.
- **Fix**: Append "— accepted warning RUSTSEC-2026-0543, see change.md" to row 2.5, then run the manual checks before archiving.
- **Decision**: FIXED. Row 2.5 now names the accepted advisory. The manual rows are still pending; they are the user's to run.

### F10 — Small leftovers and inconsistencies

- **Severity**: 💡 OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: various (see Detail)
- **Detail**:
  - **Unused logo:** `public/tauri.svg` was the greet demo's logo and nothing references it now. The plan said to drop unused logos.
  - **Duplicate checks:**
    - `settings.rs:156-160` checks statuses for duplicates case-sensitively, so `["Todo","todo"]` is accepted;
    - priorities are not checked for duplicates at all.
  - **Unreachable fallback:** `tasks.rs:345-350` has an `UnknownStatus { status: "" }` branch for empty statuses, which settings validation already prevents.
  - **Duplicate constant:** `tasks.rs:14` redefines the BOM as `&str`, next to the `[u8; 3]` constant in `vault/`.
  - **Unicode titles:** titles are not NFC-normalised. On NTFS an NFD twin of an existing NFC note creates a second, visually identical file, while `path.rs` compares in NFC elsewhere. Titles over 255 characters fail only with a generic `Io` error.
- **Fix**: Delete `public/tauri.svg`. Check statuses and priorities for case-insensitive duplicates. Replace the unreachable branch with `expect` or index `[0]` with a comment. Reuse the vault BOM constant. NFC-normalise the trimmed title and cap its length with an `InvalidTitle` reason.
- **Decision**: FIXED.
  - `public/tauri.svg` was removed.
  - Duplicate statuses and priorities are refused case-insensitively.
  - The empty-statuses fallback is now `SettingsInvalid`.
  - The shared `markdown::BOM` is reused.
  - Titles are NFC-normalised and capped at 255 UTF-16 units including `.md`.
  - Tests were added.
