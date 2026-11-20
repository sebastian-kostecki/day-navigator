mod common;

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::{fixture_dir, space_settings, test_vault, vault_root};
use day_navigator_lib::settings::Settings;
use day_navigator_lib::vault::fsops::{self, ReplaceError};
use day_navigator_lib::vault::{
    FieldValue, Heading, Space, Vault, VaultError, VaultPath, VaultRegistry, markdown,
};

const PLAN_NOTE: &str = "Dziennik/2026/2026-01-05.md";
const CRLF_NOTE: &str = "Dziennik/2026/2026-01-06.md";
const ISSUE_NOTE: &str = "Issues/Fake issue 101.md";
const TASK_NOTE: &str = "Tasks/Przykładowe zadanie.md";

fn vp(s: &str) -> VaultPath {
    VaultPath::parse(s).unwrap()
}

fn fixture_bytes(rel: &str) -> Vec<u8> {
    fs::read(fixture_dir().join(rel)).unwrap()
}

fn disk(root: &Path, rel: &str) -> Vec<u8> {
    fs::read(root.join(rel)).unwrap()
}

/// Original bytes with `span` replaced by `replacement`.
fn spliced(original: &[u8], span: markdown::Span, replacement: &[u8]) -> Vec<u8> {
    let mut out = original[..span.start].to_vec();
    out.extend_from_slice(replacement);
    out.extend_from_slice(&original[span.end..]);
    out
}

/// Panics if any `.dn-*.tmp` / `.dn-*.bak` file exists anywhere under `dir`.
fn assert_no_leftovers(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().unwrap().is_dir() {
            assert_no_leftovers(&entry.path());
        } else {
            assert!(
                !(name.contains(".dn-") && (name.ends_with(".tmp") || name.ends_with(".bak"))),
                "leftover file {name} in {}",
                dir.display()
            );
        }
    }
}

fn setup() -> (tempfile::TempDir, PathBuf, Vault) {
    let (tmp, reg) = test_vault();
    let root = vault_root(&tmp);
    let vault = reg.vault(Space::Work).unwrap();
    (tmp, root, vault)
}

// ---------------------------------------------------------------------------
// Edit operations
// ---------------------------------------------------------------------------

#[test]
fn replace_section_changes_only_that_section() {
    let (_tmp, root, vault) = setup();
    let heading = Heading::new(2, "Log");
    let note = vault.read_note(&vp(PLAN_NOTE)).unwrap();
    let hash = note.section(&heading).unwrap().hash;
    vault
        .replace_section(&vp(PLAN_NOTE), &heading, "- 10:00 Replaced\n", hash)
        .unwrap();

    let original = fixture_bytes(PLAN_NOTE);
    let span = markdown::find_section(&original, &heading).unwrap();
    assert_eq!(
        disk(&root, PLAN_NOTE),
        spliced(&original, span, b"- 10:00 Replaced\n")
    );
    assert_no_leftovers(&root);
}

#[test]
fn replace_section_in_crlf_bom_note_keeps_bom_and_crlf() {
    let (_tmp, root, vault) = setup();
    let heading = Heading::new(2, "Log");
    let hash = vault.read_note(&vp(CRLF_NOTE)).unwrap().section(&heading).unwrap().hash;
    vault
        .replace_section(&vp(CRLF_NOTE), &heading, "- new\n- lines\n", hash)
        .unwrap();

    let original = fixture_bytes(CRLF_NOTE);
    let span = markdown::find_section(&original, &heading).unwrap();
    let after = disk(&root, CRLF_NOTE);
    assert_eq!(&after[..3], &[0xEF, 0xBB, 0xBF]);
    assert_eq!(after, spliced(&original, span, b"- new\r\n- lines\r\n"));
}

#[test]
fn replace_field_replaces_list_block_only() {
    let (_tmp, root, vault) = setup();
    let hash = vault.read_note(&vp(ISSUE_NOTE)).unwrap().field("Status").unwrap().hash;
    vault
        .replace_field(
            &vp(ISSUE_NOTE),
            "Status",
            FieldValue::List(vec!["Done".to_string()]),
            hash,
        )
        .unwrap();

    let original = fixture_bytes(ISSUE_NOTE);
    let span = markdown::find_field(&original, "Status").unwrap();
    assert_eq!(
        disk(&root, ISSUE_NOTE),
        spliced(&original, span, b"Status:\n  - Done\n")
    );
}

#[test]
fn replace_field_replaces_scalar_only() {
    let (_tmp, root, vault) = setup();
    let hash = vault.read_note(&vp(TASK_NOTE)).unwrap().field("Status").unwrap().hash;
    vault
        .replace_field(&vp(TASK_NOTE), "Status", FieldValue::Scalar("Done".into()), hash)
        .unwrap();

    let original = fixture_bytes(TASK_NOTE);
    let span = markdown::find_field(&original, "Status").unwrap();
    assert_eq!(
        disk(&root, TASK_NOTE),
        spliced(&original, span, b"Status: Done\n")
    );
}

#[test]
fn append_to_section_inserts_after_last_body_line() {
    let (_tmp, root, vault) = setup();
    let heading = Heading::new(2, "Log");
    vault
        .append_to_section(&vp(PLAN_NOTE), &heading, "- 11:00 Appended")
        .unwrap();

    let original = fixture_bytes(PLAN_NOTE);
    // The fixture's Log body ends at EOF with a newline: the line lands right after it.
    assert!(original.ends_with(b"\n"));
    let mut expected = original.clone();
    expected.extend_from_slice(b"- 11:00 Appended\n");
    assert_eq!(disk(&root, PLAN_NOTE), expected);
    assert_no_leftovers(&root);
}

#[test]
fn stale_section_hash_is_conflict_with_path_and_file_untouched() {
    let (_tmp, root, vault) = setup();
    let heading = Heading::new(2, "Log");
    let hash = vault.read_note(&vp(PLAN_NOTE)).unwrap().section(&heading).unwrap().hash;
    // Someone edits the same section after the caller read it.
    let mut edited = fixture_bytes(PLAN_NOTE);
    edited.extend_from_slice(b"\n- 09:30 Edited elsewhere");
    fs::write(root.join(PLAN_NOTE), &edited).unwrap();

    let err = vault
        .replace_section(&vp(PLAN_NOTE), &heading, "- mine\n", hash)
        .unwrap_err();
    match err {
        VaultError::Conflict { path } => assert_eq!(path, PLAN_NOTE),
        other => panic!("expected Conflict, got {other:?}"),
    }
    assert_eq!(disk(&root, PLAN_NOTE), edited);
    assert_no_leftovers(&root);
}

#[test]
fn change_elsewhere_in_note_is_preserved() {
    let (_tmp, root, vault) = setup();
    let plan = Heading::new(2, "Plan");
    let hash = vault.read_note(&vp(PLAN_NOTE)).unwrap().section(&plan).unwrap().hash;
    // The Log section changes after the caller read the Plan section.
    let mut edited = fixture_bytes(PLAN_NOTE);
    edited.extend_from_slice(b"\n- 09:30 Edited elsewhere");
    fs::write(root.join(PLAN_NOTE), &edited).unwrap();

    vault
        .replace_section(&vp(PLAN_NOTE), &plan, "- [x] only this\n\n", hash)
        .unwrap();

    let after = String::from_utf8(disk(&root, PLAN_NOTE)).unwrap();
    assert!(after.contains("- [x] only this"));
    assert!(after.ends_with("- 09:30 Edited elsewhere"), "outside edit lost: {after}");
    assert!(!after.contains("Invented task one"));
}

#[test]
fn file_changing_between_read_and_replace_is_retried_then_succeeds() {
    let (_tmp, root, vault) = setup();
    let plan = Heading::new(2, "Plan");
    let hash = vault.read_note(&vp(PLAN_NOTE)).unwrap().section(&plan).unwrap().hash;
    let calls = Cell::new(0);
    let target = root.join(PLAN_NOTE);
    vault
        .apply(&vp(PLAN_NOTE), &|bytes| {
            calls.set(calls.get() + 1);
            let out = day_navigator_lib::vault::edit::replace_section(bytes, &plan, "- p\n\n", &hash);
            if calls.get() == 1 {
                // Simulated Obsidian write after our read, in another section.
                let mut b = bytes.to_vec();
                b.extend_from_slice(b"\n- 12:00 Obsidian");
                fs::write(&target, b).unwrap();
            }
            out
        })
        .unwrap();
    assert_eq!(calls.get(), 2);
    let after = String::from_utf8(disk(&root, PLAN_NOTE)).unwrap();
    assert!(after.contains("- p\n"));
    assert!(after.ends_with("- 12:00 Obsidian"));
    assert_no_leftovers(&root);
}

#[test]
fn file_changing_on_every_attempt_gives_conflict_after_three_attempts() {
    let (_tmp, root, vault) = setup();
    let calls = Cell::new(0);
    let target = root.join(PLAN_NOTE);
    let err = vault
        .apply(&vp(PLAN_NOTE), &|bytes| {
            calls.set(calls.get() + 1);
            let mut b = bytes.to_vec();
            b.extend_from_slice(b"\n- churn");
            fs::write(&target, &b).unwrap();
            b.extend_from_slice(b" mine");
            Ok(b)
        })
        .unwrap_err();
    assert_eq!(calls.get(), 3);
    assert!(matches!(err, VaultError::Conflict { ref path } if path == PLAN_NOTE), "{err:?}");
    assert_no_leftovers(&root);
}

#[test]
fn writes_into_excluded_and_hidden_and_non_markdown_are_refused() {
    let (_tmp, root, vault) = setup();
    let excluded = vp("Dostepy/invented-secret.md");
    let before = disk(&root, "Dostepy/invented-secret.md");
    assert!(matches!(
        vault.append_to_section(&excluded, &Heading::new(1, "x"), "a"),
        Err(VaultError::Excluded { .. })
    ));
    assert!(matches!(
        vault.create_note(&vp("Dostepy/new.md"), "x"),
        Err(VaultError::Excluded { .. })
    ));
    assert!(matches!(vault.trash(&excluded), Err(VaultError::Excluded { .. })));
    assert_eq!(disk(&root, "Dostepy/invented-secret.md"), before);
    assert!(!root.join("Dostepy/new.md").exists());
    assert!(!root.join(".trash").exists());

    // Hidden folders cannot even be named.
    assert!(matches!(
        VaultPath::parse(".obsidian/app.json"),
        Err(VaultError::Hidden { .. })
    ));
    assert!(matches!(
        vault.create_note(&vp("notes.txt"), "x"),
        Err(VaultError::NotMarkdown { .. })
    ));
    assert!(matches!(vault.trash(&vp("notes.txt")), Err(VaultError::NotMarkdown { .. })));
}

// ---------------------------------------------------------------------------
// create_note
// ---------------------------------------------------------------------------

#[test]
fn create_note_writes_content_as_given_and_creates_parents() {
    let (_tmp, root, vault) = setup();
    let content = "---\r\nStatus: Todo\r\n---\r\n# Nowa\r\n";
    vault.create_note(&vp("Dziennik/2027/2027-01-01.md"), content).unwrap();
    assert_eq!(disk(&root, "Dziennik/2027/2027-01-01.md"), content.as_bytes());
    assert_no_leftovers(&root);
}

#[test]
fn create_note_on_existing_file_is_already_exists_and_untouched() {
    let (_tmp, root, vault) = setup();
    let err = vault.create_note(&vp(TASK_NOTE), "overwrite").unwrap_err();
    assert!(matches!(err, VaultError::AlreadyExists { .. }), "{err:?}");
    assert_eq!(disk(&root, TASK_NOTE), fixture_bytes(TASK_NOTE));
    assert_no_leftovers(&root);
}

#[test]
fn create_note_after_root_deleted_is_root_missing_and_creates_nothing() {
    let (tmp, root, vault) = setup();
    fs::remove_dir_all(&root).unwrap();
    let err = vault
        .create_note(&vp("Dziennik/2027/2027-01-01.md"), "x")
        .unwrap_err();
    assert!(matches!(err, VaultError::RootMissing { .. }), "{err:?}");
    assert!(!root.exists());
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    assert!(matches!(
        vault.trash(&vp(TASK_NOTE)),
        Err(VaultError::RootMissing { .. })
    ));
    assert!(matches!(
        vault.append_to_section(&vp(PLAN_NOTE), &Heading::new(2, "Log"), "x"),
        Err(VaultError::RootMissing { .. })
    ));
}

// ---------------------------------------------------------------------------
// trash
// ---------------------------------------------------------------------------

#[test]
fn trash_moves_note_into_root_trash() {
    let (_tmp, root, vault) = setup();
    let name = vault.trash(&vp(TASK_NOTE)).unwrap();
    assert_eq!(name, "Przykładowe zadanie.md");
    assert!(!root.join(TASK_NOTE).exists());
    assert_eq!(disk(&root, ".trash/Przykładowe zadanie.md"), fixture_bytes(TASK_NOTE));
    // Trashed notes are no longer readable through the vault.
    assert!(matches!(
        vault.read_note(&vp(TASK_NOTE)),
        Err(VaultError::NotFound { .. })
    ));
    assert!(matches!(vault.trash(&vp(TASK_NOTE)), Err(VaultError::NotFound { .. })));
}

#[test]
fn trash_collision_gets_numeric_suffix_and_keeps_existing_entry() {
    let (_tmp, root, vault) = setup();
    let path = vp("Notes/duplicate-headings.md");
    let first = vault.trash(&path).unwrap();
    assert_eq!(first, "duplicate-headings.md");
    vault.create_note(&path, "# second\n").unwrap();
    assert_eq!(vault.trash(&path).unwrap(), "duplicate-headings 2.md");
    vault.create_note(&path, "# third\n").unwrap();
    assert_eq!(vault.trash(&path).unwrap(), "duplicate-headings 3.md");

    assert_eq!(
        disk(&root, ".trash/duplicate-headings.md"),
        fixture_bytes("Notes/duplicate-headings.md")
    );
    assert_eq!(disk(&root, ".trash/duplicate-headings 2.md"), b"# second\n");
    assert_eq!(disk(&root, ".trash/duplicate-headings 3.md"), b"# third\n");
}

#[cfg(windows)]
#[test]
fn trash_refuses_trash_junction_pointing_outside_the_vault() {
    let (tmp, root, vault) = setup();
    let outside = tmp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    junction::create(&outside, root.join(".trash")).unwrap();

    let err = vault.trash(&vp(TASK_NOTE)).unwrap_err();
    assert!(matches!(err, VaultError::OutsideVault { .. }), "{err:?}");
    assert!(root.join(TASK_NOTE).exists(), "note must not move");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn trash_refuses_trash_junction_pointing_to_other_vault_folder() {
    let (_tmp, root, vault) = setup();
    junction::create(root.join("Notes"), root.join(".trash")).unwrap();
    let err = vault.trash(&vp(TASK_NOTE)).unwrap_err();
    assert!(matches!(err, VaultError::OutsideVault { .. }), "{err:?}");
    assert!(root.join(TASK_NOTE).exists());
}

// ---------------------------------------------------------------------------
// Atomic replace: failure branches, read-only, leftovers, metadata
// ---------------------------------------------------------------------------

fn scratch_file(content: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("x.md");
    fs::write(&target, content).unwrap();
    (tmp, target)
}

fn dir_names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn replace_atomic_replaces_content_and_leaves_no_files() {
    let (tmp, target) = scratch_file(b"old");
    fsops::replace_atomic(&target, b"new", b"old", "x.md").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

#[test]
fn replace_atomic_reports_changed_when_target_differs_and_cleans_temp() {
    let (tmp, target) = scratch_file(b"actual");
    let err = fsops::replace_atomic(&target, b"new", b"stale", "x.md").unwrap_err();
    assert!(matches!(err, ReplaceError::Changed));
    assert_eq!(fs::read(&target).unwrap(), b"actual");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

#[test]
fn error_1176_moves_temp_onto_target_and_cleans_up() {
    let (tmp, target) = scratch_file(b"old");
    let replace = |t: &Path, temp: &Path, backup: &Path| {
        // State after ERROR_UNABLE_TO_MOVE_REPLACEMENT: original under the backup
        // name, new content only in the temp file.
        assert!(temp.exists(), "temp must exist before recovery");
        fs::rename(t, backup).unwrap();
        Err(1176)
    };
    fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

#[test]
fn error_1177_restores_original_and_reports_error() {
    let (tmp, target) = scratch_file(b"old");
    let replace = |t: &Path, temp: &Path, backup: &Path| {
        assert!(temp.exists());
        fs::rename(t, backup).unwrap();
        Err(1177)
    };
    let err = fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap_err();
    assert!(matches!(err, ReplaceError::Failed(VaultError::Io { .. })), "{err:?}");
    assert_eq!(fs::read(&target).unwrap(), b"old");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

#[test]
fn failed_recovery_keeps_temp_and_backup_and_names_both() {
    for code in [1176u32, 1177] {
        let (tmp, target) = scratch_file(b"old");
        // Moves the original aside, then puts a folder where the target was so
        // the recovery rename cannot succeed.
        let replace = move |t: &Path, _temp: &Path, backup: &Path| {
            fs::rename(t, backup).unwrap();
            fs::create_dir(t).unwrap();
            Err(code)
        };
        let err = fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap_err();
        let ReplaceError::Failed(VaultError::ReplaceIncomplete { path, temp, backup }) = err else {
            panic!("expected ReplaceIncomplete for {code}");
        };
        assert_eq!(path, "x.md");
        // Neither file was deleted: the original and the new bytes both survive.
        assert_eq!(fs::read(tmp.path().join(&backup)).unwrap(), b"old", "code {code}");
        assert_eq!(fs::read(tmp.path().join(&temp)).unwrap(), b"new", "code {code}");
        assert!(temp.starts_with(".x.md.dn-") && temp.ends_with(".tmp"));
        assert!(backup.starts_with(".x.md.dn-") && backup.ends_with(".bak"));
    }
}

#[test]
fn sharing_violation_is_retried_then_busy_and_cleans_temp() {
    let (tmp, target) = scratch_file(b"old");
    let calls = Cell::new(0);
    let replace = |_: &Path, _: &Path, _: &Path| {
        calls.set(calls.get() + 1);
        Err(32)
    };
    let err = fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap_err();
    assert!(matches!(err, ReplaceError::Failed(VaultError::Busy { .. })), "{err:?}");
    assert_eq!(calls.get(), 5);
    assert_eq!(fs::read(&target).unwrap(), b"old");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

#[test]
fn lock_violation_that_clears_succeeds_on_retry() {
    let (tmp, target) = scratch_file(b"old");
    let calls = Cell::new(0);
    let replace = |t: &Path, temp: &Path, _: &Path| {
        calls.set(calls.get() + 1);
        if calls.get() < 3 {
            Err(33)
        } else {
            fs::rename(temp, t).map_err(|_| 1u32)
        }
    };
    fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

#[test]
fn other_replace_errors_are_io_and_clean_temp() {
    let (tmp, target) = scratch_file(b"old");
    let replace = |_: &Path, _: &Path, _: &Path| Err(1175);
    let err = fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap_err();
    assert!(matches!(err, ReplaceError::Failed(VaultError::Io { .. })), "{err:?}");
    assert_eq!(fs::read(&target).unwrap(), b"old");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
}

fn set_readonly(path: &Path, ro: bool) {
    let mut perm = fs::metadata(path).unwrap().permissions();
    perm.set_readonly(ro);
    fs::set_permissions(path, perm).unwrap();
}

#[test]
fn read_only_target_is_read_only_error_without_retry_or_replace() {
    let (tmp, target) = scratch_file(b"old");
    set_readonly(&target, true);
    let calls = Cell::new(0);
    let replace = |_: &Path, _: &Path, _: &Path| {
        calls.set(calls.get() + 1);
        Err(5)
    };
    let err = fsops::replace_atomic_with(&target, b"new", b"old", "x.md", &replace).unwrap_err();
    assert!(matches!(err, ReplaceError::Failed(VaultError::ReadOnly { .. })), "{err:?}");
    assert_eq!(calls.get(), 0);
    assert_eq!(fs::read(&target).unwrap(), b"old");
    assert_eq!(dir_names(tmp.path()), vec!["x.md"]);
    set_readonly(&target, false);
}

#[test]
fn read_only_note_through_vault_is_read_only_and_unchanged() {
    let (_tmp, root, vault) = setup();
    let heading = Heading::new(2, "Log");
    let hash = vault.read_note(&vp(PLAN_NOTE)).unwrap().section(&heading).unwrap().hash;
    let file = root.join(PLAN_NOTE);
    set_readonly(&file, true);
    let err = vault
        .replace_section(&vp(PLAN_NOTE), &heading, "x\n", hash)
        .unwrap_err();
    set_readonly(&file, false);
    assert!(matches!(err, VaultError::ReadOnly { ref path } if path == PLAN_NOTE), "{err:?}");
    assert_eq!(disk(&root, PLAN_NOTE), fixture_bytes(PLAN_NOTE));
    assert_no_leftovers(&root);
}

fn age(path: &Path, secs: u64) {
    let f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(secs)).unwrap();
}

#[test]
fn next_write_removes_old_leftovers_of_that_note_only() {
    let (_tmp, root, vault) = setup();
    let dir = root.join("Dziennik/2026");
    let mine_old_tmp = dir.join(".2026-01-05.md.dn-aaaa1111.tmp");
    let mine_old_bak = dir.join(".2026-01-05.md.dn-bbbb2222.bak");
    let mine_fresh = dir.join(".2026-01-05.md.dn-cccc3333.tmp");
    let other_old = dir.join(".2026-01-06.md.dn-dddd4444.tmp");
    let unrelated_old = dir.join(".2026-01-05.md.dn-eeee5555.txt");
    for p in [&mine_old_tmp, &mine_old_bak, &mine_fresh, &other_old, &unrelated_old] {
        fs::write(p, b"leftover").unwrap();
    }
    for p in [&mine_old_tmp, &mine_old_bak, &other_old, &unrelated_old] {
        age(p, 300);
    }

    vault
        .append_to_section(&vp(PLAN_NOTE), &Heading::new(2, "Log"), "- x")
        .unwrap();

    assert!(!mine_old_tmp.exists(), "old .tmp of this note should be removed");
    assert!(!mine_old_bak.exists(), "old .bak of this note should be removed");
    assert!(mine_fresh.exists(), "fresh file may belong to a running write");
    assert!(other_old.exists(), "other notes' leftovers are not touched");
    assert!(unrelated_old.exists(), "only .tmp/.bak are removed");
    for p in [&mine_fresh, &other_old, &unrelated_old] {
        fs::remove_file(p).unwrap();
    }
    assert_no_leftovers(&root);
}

#[cfg(windows)]
#[test]
fn creation_time_is_preserved_by_replace_section() {
    let (_tmp, root, vault) = setup();
    let file = root.join(PLAN_NOTE);
    let created = fs::metadata(&file).unwrap().created().unwrap();
    std::thread::sleep(Duration::from_millis(100));

    let heading = Heading::new(2, "Log");
    let hash = vault.read_note(&vp(PLAN_NOTE)).unwrap().section(&heading).unwrap().hash;
    vault
        .replace_section(&vp(PLAN_NOTE), &heading, "- changed\n", hash)
        .unwrap();

    assert_eq!(fs::metadata(&file).unwrap().created().unwrap(), created);
    assert!(String::from_utf8(fs::read(&file).unwrap()).unwrap().contains("- changed"));
}

// ---------------------------------------------------------------------------
// Smoke test against a scratch copy of the invented vault (manual)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "manual: needs DN_SMOKE_VAULT pointing at a scratch copy of the fixture vault"]
fn smoke_against_scratch_vault() {
    let dir = std::env::var("DN_SMOKE_VAULT").expect("set DN_SMOKE_VAULT to a scratch copy of the fixture vault");
    let root = PathBuf::from(dir);
    assert!(
        root.join(".dn-test-vault").is_file(),
        "refusing to run: {} has no .dn-test-vault marker",
        root.display()
    );
    let settings = Settings {
        work: Some(space_settings(&root, &["Dostepy"])),
        private: None,
    };
    let vault = VaultRegistry::from_settings(&settings).vault(Space::Work).unwrap();

    let plan = Heading::new(2, "Plan");
    let hash = vault.read_note(&vp(PLAN_NOTE)).unwrap().section(&plan).unwrap().hash;
    vault
        .replace_section(&vp(PLAN_NOTE), &plan, "- [x] Smoke plan\n\n", hash)
        .unwrap();

    let hash = vault.read_note(&vp(ISSUE_NOTE)).unwrap().field("Status").unwrap().hash;
    vault
        .replace_field(
            &vp(ISSUE_NOTE),
            "Status",
            FieldValue::List(vec!["Smoke".to_string()]),
            hash,
        )
        .unwrap();

    vault
        .append_to_section(&vp(PLAN_NOTE), &Heading::new(2, "Log"), "- 23:59 Smoke log")
        .unwrap();
    vault
        .create_note(&vp("Dziennik/2027/2027-01-01.md"), "# 2027-01-01\n")
        .unwrap();
    let trashed = vault.trash(&vp("Notes/duplicate-headings.md")).unwrap();
    println!("trashed as .trash/{trashed}");
    assert_no_leftovers(&root);
}
