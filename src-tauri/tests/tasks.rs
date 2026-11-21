mod common;

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::{NaiveDate, NaiveDateTime};
use common::{fixture_dir, test_tasks_settings, test_vault, vault_root};
use day_navigator_lib::settings::TaskSettings;
use day_navigator_lib::tasks::{
    DoneUpdate, NewTask, TaskError, create_task, list_tasks, require_config, set_status,
};
use day_navigator_lib::vault::{Space, SpanHash, Vault, VaultError, VaultPath};
use tempfile::TempDir;

fn vp(s: &str) -> VaultPath {
    VaultPath::parse(s).unwrap()
}

fn now() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 3, 4)
        .unwrap()
        .and_hms_opt(9, 5, 0)
        .unwrap()
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 4).unwrap()
}

fn setup() -> (TempDir, Vault, TaskSettings) {
    let (tmp, reg) = test_vault();
    let vault = reg.vault(Space::Work).unwrap();
    (tmp, vault, test_tasks_settings())
}

fn new_task(title: &str) -> NewTask {
    NewTask {
        title: title.to_string(),
        priority: None,
        due: None,
    }
}

fn disk(root: &Path, rel: &str) -> Vec<u8> {
    fs::read(root.join(rel)).unwrap()
}

fn fixture(rel: &str) -> Vec<u8> {
    fs::read(fixture_dir().join(rel)).unwrap()
}

fn mtime(root: &Path, rel: &str) -> SystemTime {
    fs::metadata(root.join(rel)).unwrap().modified().unwrap()
}

fn hashes(vault: &Vault, rel: &str) -> (SpanHash, Option<SpanHash>) {
    let note = vault.read_note(&vp(rel)).unwrap();
    let status = note.field("Status").unwrap().hash;
    let done = note.field("Done").ok().map(|f| f.hash);
    (status, done)
}

fn replace(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(s.contains(from), "{from:?} not found");
    s.replacen(from, to, 1).into_bytes()
}

fn is_conflict(r: &Result<DoneUpdate, TaskError>) -> bool {
    matches!(r, Err(TaskError::Vault(VaultError::Conflict { .. })))
}

const OPEN: &str = "Tasks/Invented open task.md";
const DONE: &str = "Tasks/Invented done task.md";
const CRLF: &str = "Tasks/Invented crlf task.md";
const NO_DONE: &str = "Tasks/Przykładowe zadanie.md";

// ---------------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------------

#[test]
fn require_config_none_is_not_configured() {
    assert!(matches!(
        require_config(None),
        Err(TaskError::TasksNotConfigured)
    ));
    assert!(require_config(Some(&test_tasks_settings())).is_ok());
}

#[test]
fn list_sorts_by_status_due_then_title_and_keeps_problem_rows() {
    let (_tmp, vault, cfg) = setup();
    let rows = list_tasks(&vault, &cfg).unwrap();
    let titles: Vec<_> = rows.iter().map(|r| r.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Invented crlf task",
            "Przykładowe zadanie",
            "Invented open task",
            "Invented done task",
            "Invented duplicate status",
            "Invented list status",
        ]
    );
    let by = |t: &str| rows.iter().find(|r| r.title == t).unwrap();

    let open = by("Invented open task");
    assert_eq!(open.status.as_deref(), Some("In progress"));
    assert_eq!(open.priority.as_deref(), Some("High"));
    assert_eq!(open.due.as_deref(), Some("2026-02-01"));
    assert!(open.status_hash.is_some() && open.done_hash.is_some());
    assert!(open.problem.is_none());

    let done = by("Invented done task");
    assert_eq!(done.status.as_deref(), Some("Done"));
    assert!(done.problem.is_none());

    let no_done = by("Przykładowe zadanie");
    assert!(no_done.done_hash.is_none() && no_done.problem.is_none());
    assert!(no_done.status_hash.is_some());

    let crlf = by("Invented crlf task");
    assert_eq!(crlf.status.as_deref(), Some("Todo"));
    assert_eq!(crlf.due, None);

    assert!(matches!(
        by("Invented list status").problem,
        Some(VaultError::InvalidValue { .. })
    ));
    assert!(matches!(
        by("Invented duplicate status").problem,
        Some(VaultError::FieldAmbiguous { .. })
    ));
}

#[test]
fn list_skips_template_inside_tasks_folder() {
    let (tmp, vault, mut cfg) = setup();
    let root = vault_root(&tmp);
    fs::write(
        root.join("Tasks/Tmpl.md"),
        "---\nStatus: Todo\nDone:\n---\n# {{title}}\n",
    )
    .unwrap();
    cfg.template = vp("Tasks/tmpl.md");
    let rows = list_tasks(&vault, &cfg).unwrap();
    assert!(rows.iter().all(|r| r.title != "Tmpl"));
    assert_eq!(rows.len(), 6);
}

#[test]
fn list_with_missing_folder_is_an_error_not_an_empty_list() {
    let (_tmp, vault, mut cfg) = setup();
    cfg.folder = vp("Nope");
    assert!(matches!(
        list_tasks(&vault, &cfg),
        Err(TaskError::FolderMissing { .. })
    ));
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

#[test]
fn create_renders_template_and_sets_app_owned_fields() {
    let (tmp, vault, cfg) = setup();
    let root = vault_root(&tmp);
    let new = NewTask {
        title: "  Nowe zadanie testowe ".to_string(),
        priority: Some("High".to_string()),
        due: Some("2026-05-06".to_string()),
    };
    let path = create_task(&vault, &cfg, &new, now()).unwrap();
    assert_eq!(path.to_string(), "Tasks/Nowe zadanie testowe.md");
    let expected = "---\nStatus: Todo\nPriority: High\nDue: 2026-05-06\nIssue:\nToday:\nDone:\n---\n# Nowe zadanie testowe\n\nUtworzono 2026-03-04 09:05\n";
    assert_eq!(disk(&root, "Tasks/Nowe zadanie testowe.md"), expected.as_bytes());
}

#[test]
fn create_without_optional_fields_leaves_them_empty() {
    let (tmp, vault, cfg) = setup();
    create_task(&vault, &cfg, &new_task("Bez opcji"), now()).unwrap();
    let got = String::from_utf8(disk(&vault_root(&tmp), "Tasks/Bez opcji.md")).unwrap();
    assert!(got.contains("Status: Todo\nPriority:\nDue:\n"));
}

#[test]
fn create_keeps_crlf_and_bom_of_the_template() {
    let (tmp, vault, mut cfg) = setup();
    cfg.template = vp("Templates/Task CRLF.md");
    let new = NewTask {
        title: "Zadanie crlf".to_string(),
        priority: Some("Low".to_string()),
        due: None,
    };
    create_task(&vault, &cfg, &new, now()).unwrap();
    let expected = "\u{FEFF}---\r\nStatus: Todo\r\nPriority: Low\r\nDue:\r\nIssue:\r\nToday:\r\nDone:\r\n---\r\n# Zadanie crlf\r\n\r\nUtworzono 2026-03-04 09:05\r\n";
    assert_eq!(
        disk(&vault_root(&tmp), "Tasks/Zadanie crlf.md"),
        expected.as_bytes()
    );
}

fn assert_vault_unchanged(tmp: &TempDir) {
    let root = vault_root(tmp);
    let fresh = tempfile::tempdir().unwrap();
    common::copy_dir(&fixture_dir(), fresh.path());
    fn walk(dir: &Path, out: &mut Vec<(String, Vec<u8>)>, base: &Path) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), out, base);
            } else {
                let rel = e.path().strip_prefix(base).unwrap().to_string_lossy().into_owned();
                out.push((rel, fs::read(e.path()).unwrap()));
            }
        }
    }
    let (mut a, mut b) = (Vec::new(), Vec::new());
    walk(&root, &mut a, &root);
    walk(fresh.path(), &mut b, fresh.path());
    assert_eq!(a, b, "vault changed");
}

#[test]
fn create_refusals_write_nothing() {
    let (tmp, vault, cfg) = setup();

    // Existing note.
    let r = create_task(&vault, &cfg, &new_task("Invented open task"), now());
    assert!(matches!(
        r,
        Err(TaskError::Vault(VaultError::AlreadyExists { .. }))
    ));

    for bad in [
        "", "   ", "a:b", "a/b", "a\\b", "a[b", "a]b", "a#b", "a^b", "a|b", "trailing.", "CON",
    ] {
        let r = create_task(&vault, &cfg, &new_task(bad), now());
        assert!(
            matches!(r, Err(TaskError::InvalidTitle { .. })),
            "{bad:?}: {r:?}"
        );
    }

    let r = create_task(
        &vault,
        &cfg,
        &NewTask {
            title: "Ok".into(),
            priority: Some("Urgent".into()),
            due: None,
        },
        now(),
    );
    assert!(matches!(r, Err(TaskError::UnknownPriority { .. })));

    for due in ["2026-02-30", "2026-2-3", "tomorrow"] {
        let r = create_task(
            &vault,
            &cfg,
            &NewTask {
                title: "Ok".into(),
                priority: None,
                due: Some(due.into()),
            },
            now(),
        );
        assert!(matches!(r, Err(TaskError::InvalidDue { .. })), "{due}");
    }

    let mut missing = cfg.clone();
    missing.template = vp("Templates/Nope.md");
    assert!(matches!(
        create_task(&vault, &missing, &new_task("Ok"), now()),
        Err(TaskError::TemplateMissing { .. })
    ));

    let mut templater = cfg.clone();
    templater.template = vp("Templates/Task templater.md");
    assert!(matches!(
        create_task(&vault, &templater, &new_task("Ok"), now()),
        Err(TaskError::TemplateUnsupported { .. })
    ));

    let mut no_status = cfg.clone();
    no_status.template = vp("Templates/Task no status.md");
    match create_task(&vault, &no_status, &new_task("Ok"), now()) {
        Err(TaskError::TemplateFieldMissing { key }) => assert_eq!(key, "Status"),
        other => panic!("{other:?}"),
    }

    assert_vault_unchanged(&tmp);
}

#[test]
fn create_into_missing_folder_fails_and_creates_nothing() {
    let (tmp, vault, mut cfg) = setup();
    cfg.folder = vp("Nope/Sub");
    assert!(matches!(
        create_task(&vault, &cfg, &new_task("Ok"), now()),
        Err(TaskError::FolderMissing { .. })
    ));
    assert!(!vault_root(&tmp).join("Nope").exists());
    assert_vault_unchanged(&tmp);
}

// ---------------------------------------------------------------------------
// Status change
// ---------------------------------------------------------------------------

#[test]
fn todo_to_in_progress_changes_only_the_status_line() {
    let (tmp, vault, cfg) = setup();
    let (s, d) = hashes(&vault, CRLF);
    let r = set_status(&vault, &cfg, &vp(CRLF), "In progress", s, d, today()).unwrap();
    assert_eq!(r, DoneUpdate::Unchanged);
    assert_eq!(
        disk(&vault_root(&tmp), CRLF),
        replace(&fixture(CRLF), "Status: Todo", "Status: In progress")
    );
}

#[test]
fn moving_to_done_sets_done_in_one_write_and_keeps_crlf() {
    let (tmp, vault, cfg) = setup();
    let (s, d) = hashes(&vault, CRLF);
    let r = set_status(&vault, &cfg, &vp(CRLF), "Done", s, d, today()).unwrap();
    assert_eq!(r, DoneUpdate::Set);
    let expected = replace(
        &replace(&fixture(CRLF), "Status: Todo", "Status: Done"),
        "Done:\r\n",
        "Done: 2026-03-04\r\n",
    );
    assert_eq!(disk(&vault_root(&tmp), CRLF), expected);

    let (s, d) = hashes(&vault, OPEN);
    let r = set_status(&vault, &cfg, &vp(OPEN), "Done", s, d, today()).unwrap();
    assert_eq!(r, DoneUpdate::Set);
    let expected = replace(
        &replace(&fixture(OPEN), "Status: In progress", "Status: Done"),
        "Done:\n",
        "Done: 2026-03-04\n",
    );
    assert_eq!(disk(&vault_root(&tmp), OPEN), expected);
}

#[test]
fn moving_out_of_done_clears_done() {
    let (tmp, vault, cfg) = setup();
    let (s, d) = hashes(&vault, DONE);
    let r = set_status(&vault, &cfg, &vp(DONE), "Todo", s, d, today()).unwrap();
    assert_eq!(r, DoneUpdate::Cleared);
    let expected = replace(
        &replace(&fixture(DONE), "Status: Done", "Status: Todo"),
        "Done: 2026-01-10\n",
        "Done:\n",
    );
    assert_eq!(disk(&vault_root(&tmp), DONE), expected);
}

#[test]
fn note_without_done_field_changes_status_and_reports_absent() {
    let (tmp, vault, cfg) = setup();
    let (s, d) = hashes(&vault, NO_DONE);
    assert!(d.is_none());
    let r = set_status(&vault, &cfg, &vp(NO_DONE), "Done", s, d, today()).unwrap();
    assert_eq!(r, DoneUpdate::Absent);
    assert_eq!(
        disk(&vault_root(&tmp), NO_DONE),
        replace(&fixture(NO_DONE), "Status: Todo", "Status: Done")
    );
}

#[test]
fn same_status_writes_nothing() {
    let (tmp, vault, cfg) = setup();
    let root = vault_root(&tmp);
    let before = mtime(&root, OPEN);
    let (s, d) = hashes(&vault, OPEN);
    let r = set_status(&vault, &cfg, &vp(OPEN), "In progress", s, d, today()).unwrap();
    assert_eq!(r, DoneUpdate::Unchanged);
    assert_eq!(disk(&root, OPEN), fixture(OPEN));
    assert_eq!(mtime(&root, OPEN), before);
}

#[test]
fn unknown_status_is_refused() {
    let (tmp, vault, cfg) = setup();
    let (s, d) = hashes(&vault, OPEN);
    let r = set_status(&vault, &cfg, &vp(OPEN), "Blocked", s, d, today());
    assert!(matches!(r, Err(TaskError::UnknownStatus { .. })));
    assert_eq!(disk(&vault_root(&tmp), OPEN), fixture(OPEN));
}

#[test]
fn stale_status_hash_conflicts_and_changes_nothing() {
    let (tmp, vault, cfg) = setup();
    let root = vault_root(&tmp);
    let (s, d) = hashes(&vault, OPEN);
    let edited = replace(&fixture(OPEN), "Status: In progress", "Status: Todo");
    fs::write(root.join(OPEN), &edited).unwrap();
    let r = set_status(&vault, &cfg, &vp(OPEN), "Done", s, d, today());
    assert!(is_conflict(&r), "{r:?}");
    assert_eq!(disk(&root, OPEN), edited);
}

#[test]
fn done_edited_in_between_conflicts_and_changes_nothing() {
    let (tmp, vault, cfg) = setup();
    let root = vault_root(&tmp);
    let (s, d) = hashes(&vault, OPEN);
    let edited = replace(&fixture(OPEN), "Done:\n", "Done: 2026-01-01\n");
    fs::write(root.join(OPEN), &edited).unwrap();
    let r = set_status(&vault, &cfg, &vp(OPEN), "Done", s, d, today());
    assert!(is_conflict(&r), "{r:?}");
    assert_eq!(disk(&root, OPEN), edited);
}

#[test]
fn done_field_added_or_removed_since_listing_conflicts() {
    let (tmp, vault, cfg) = setup();
    let root = vault_root(&tmp);

    // Added: listed without Done, the note has one now.
    let (s, d) = hashes(&vault, NO_DONE);
    assert!(d.is_none());
    let added = replace(&fixture(NO_DONE), "Status: Todo\n", "Status: Todo\nDone:\n");
    fs::write(root.join(NO_DONE), &added).unwrap();
    let r = set_status(&vault, &cfg, &vp(NO_DONE), "Done", s, d, today());
    assert!(is_conflict(&r), "{r:?}");
    assert_eq!(disk(&root, NO_DONE), added);

    // Removed: listed with Done, the note has none now.
    let (s, d) = hashes(&vault, OPEN);
    assert!(d.is_some());
    let removed = replace(&fixture(OPEN), "Done:\n", "");
    fs::write(root.join(OPEN), &removed).unwrap();
    let r = set_status(&vault, &cfg, &vp(OPEN), "Done", s, d, today());
    assert!(is_conflict(&r), "{r:?}");
    assert_eq!(disk(&root, OPEN), removed);
}

#[test]
fn przykladowe_zadanie_fixture_is_untouched() {
    // Guards the byte-identity the other vault tests rely on.
    assert_eq!(
        fixture(NO_DONE),
        "---\nStatus: Todo\n---\n# Przykładowe zadanie\n\nInvented task used only for tests.\n".as_bytes()
    );
}
