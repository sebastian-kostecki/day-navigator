mod common;

use std::fs;

use common::{space_settings, test_vault, test_vault_with, vault_root};
use day_navigator_lib::settings::Settings;
use day_navigator_lib::vault::{EntryKind, Space, VaultError, VaultPath};

fn vp(s: &str) -> VaultPath {
    VaultPath::parse(s).unwrap()
}

#[test]
fn parse_rejects_traversal_absolute_unc_ads_and_reserved() {
    let bad = [
        "",
        "../x.md",
        "a/../x.md",
        "a/./x.md",
        "a//x.md",
        "a/",
        "/abs.md",
        "\\abs.md",
        "\\\\server\\share\\x.md",
        "C:/x.md",
        "C:\\x.md",
        "a.md:stream",
        "CON",
        "nul.md",
        "a/com1.txt",
        "LPT9",
        "trail.",
        "trail ",
        "a<b.md",
        "a|b.md",
        "a?b.md",
        "a*b.md",
        "a\"b.md",
        "ctl\u{1}.md",
    ];
    for input in bad {
        assert!(VaultPath::parse(input).is_err(), "should reject {input:?}");
    }
}

#[test]
fn parse_rejects_dot_segments_as_hidden() {
    for input in [
        ".obsidian/app.json",
        "a/.hidden/x.md",
        ".trash/x.md",
        "..x/y.md",
    ] {
        assert!(
            matches!(VaultPath::parse(input), Err(VaultError::Hidden { .. })),
            "{input}"
        );
    }
    assert!(matches!(
        VaultPath::parse("a/../b"),
        Err(VaultError::InvalidPath { .. })
    ));
}

#[test]
fn parse_accepts_non_ascii_and_spaces_and_normalises() {
    let p = vp("Tasks\\Przykładowe zadanie.md");
    assert_eq!(p.to_string(), "Tasks/Przykładowe zadanie.md");
    assert!(p.is_markdown());
    assert!(vp("A/B.MD").is_markdown());
    assert!(!vp("notes.txt").is_markdown());
}

#[test]
fn read_note_reads_fixture_and_strips_bom_in_text() {
    let (_tmp, reg) = test_vault();
    let vault = reg.vault(Space::Work).unwrap();
    let note = vault.read_note(&vp("Dziennik/2026/2026-01-06.md")).unwrap();
    assert_eq!(&note.bytes()[..3], &[0xEF, 0xBB, 0xBF]);
    assert!(note.text().starts_with("# 2026-01-06"));
    let task = vault
        .read_note(&vp("Tasks/Przykładowe zadanie.md"))
        .unwrap();
    assert!(task.text().contains("Status: Todo"));
}

#[test]
fn read_note_rejects_non_markdown_missing_and_non_utf8() {
    let (tmp, reg) = test_vault();
    let vault = reg.vault(Space::Work).unwrap();
    assert!(matches!(
        vault.read_note(&vp("notes.txt")),
        Err(VaultError::NotMarkdown { .. })
    ));
    assert!(matches!(
        vault.read_note(&vp("Notes/missing.md")),
        Err(VaultError::NotFound { .. })
    ));
    fs::write(vault_root(&tmp).join("Notes/bad.md"), [0xFF, 0xFE, 0x41]).unwrap();
    assert!(matches!(
        vault.read_note(&vp("Notes/bad.md")),
        Err(VaultError::NotUtf8 { .. })
    ));
}

#[test]
fn excluded_prefix_is_case_insensitive_and_whole_segment() {
    let (tmp, reg) = test_vault();
    let vault = reg.vault(Space::Work).unwrap();
    for p in ["Dostepy/invented-secret.md", "dostepy/INVENTED-secret.md"] {
        assert!(
            matches!(vault.read_note(&vp(p)), Err(VaultError::Excluded { .. })),
            "{p}"
        );
    }
    // Look-alikes are not excluded.
    assert!(vault.read_note(&vp("Dostepy-stare/old.md")).is_ok());
    fs::write(vault_root(&tmp).join("Dostepy.md"), "# sibling\n").unwrap();
    assert!(vault.read_note(&vp("Dostepy.md")).is_ok());
    fs::create_dir_all(vault_root(&tmp).join("Notes/Dostepy")).unwrap();
    fs::write(vault_root(&tmp).join("Notes/Dostepy/x.md"), "# nested\n").unwrap();
    assert!(vault.read_note(&vp("Notes/Dostepy/x.md")).is_ok());
}

#[test]
fn excluded_file_entry_is_refused_and_unlisted() {
    let (tmp, reg) = test_vault_with(|root| Settings {
        work: Some(space_settings(root, &["Notes/duplicate-headings.md"])),
        private: None,
    });
    let _ = tmp;
    let vault = reg.vault(Space::Work).unwrap();
    assert!(matches!(
        vault.read_note(&vp("Notes/duplicate-headings.md")),
        Err(VaultError::Excluded { .. })
    ));
    assert!(vault.list(Some(&vp("Notes"))).unwrap().is_empty());
}

#[test]
fn list_omits_excluded_hidden_and_non_markdown() {
    let (_tmp, reg) = test_vault();
    let vault = reg.vault(Space::Work).unwrap();
    let root = vault.list(None).unwrap();
    let names: Vec<_> = root.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Dostepy-stare",
            "Dziennik",
            "Issues",
            "Notes",
            "Tasks",
            "Templates"
        ]
    );
    assert!(root.iter().all(|e| e.kind == EntryKind::Dir));

    let year = vault.list(Some(&vp("Dziennik/2026"))).unwrap();
    let names: Vec<_> = year.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["2026-01-05.md", "2026-01-06.md"]);
    assert!(year.iter().all(|e| e.kind == EntryKind::Note));
    assert_eq!(year[0].path.to_string(), "Dziennik/2026/2026-01-05.md");

    assert!(matches!(
        vault.list(Some(&vp("Dostepy"))),
        Err(VaultError::Excluded { .. })
    ));
    assert!(matches!(
        vault.list(Some(&vp("Nope"))),
        Err(VaultError::NotFound { .. })
    ));
}

#[test]
fn root_missing_after_delete_mid_test() {
    let (tmp, reg) = test_vault();
    let vault = reg.vault(Space::Work).unwrap();
    assert!(vault.read_note(&vp("Tasks/Przykładowe zadanie.md")).is_ok());
    fs::remove_dir_all(vault_root(&tmp)).unwrap();
    assert!(matches!(
        vault.read_note(&vp("Tasks/Przykładowe zadanie.md")),
        Err(VaultError::RootMissing { .. })
    ));
    assert!(matches!(
        vault.list(None),
        Err(VaultError::RootMissing { .. })
    ));
    assert!(matches!(
        reg.vault(Space::Work),
        Err(VaultError::RootMissing { .. })
    ));
}

#[test]
fn roots_overlap_when_nested_or_equal() {
    let (tmp, reg) = test_vault_with(|root| {
        let inner = root.join("Notes");
        Settings {
            work: Some(space_settings(root, &[])),
            private: Some(space_settings(&inner, &[])),
        }
    });
    let _ = &tmp;
    assert!(matches!(
        reg.vault(Space::Work),
        Err(VaultError::RootsOverlap)
    ));
    assert!(matches!(
        reg.vault(Space::Private),
        Err(VaultError::RootsOverlap)
    ));

    let (_tmp2, reg2) = test_vault_with(|root| Settings {
        work: Some(space_settings(root, &[])),
        private: Some(space_settings(root, &[])),
    });
    assert!(matches!(
        reg2.vault(Space::Work),
        Err(VaultError::RootsOverlap)
    ));
}

#[test]
fn separate_roots_do_not_overlap() {
    let (tmp, reg) = test_vault_with(|root| {
        let other = root.parent().unwrap().join("private");
        fs::create_dir_all(&other).unwrap();
        Settings {
            work: Some(space_settings(root, &[])),
            private: Some(space_settings(&other, &[])),
        }
    });
    let _ = tmp;
    assert!(reg.vault(Space::Work).is_ok());
    assert!(reg.vault(Space::Private).is_ok());
}

#[test]
fn space_not_configured() {
    let (_tmp, reg) = test_vault();
    assert!(matches!(
        reg.vault(Space::Private),
        Err(VaultError::SpaceNotConfigured(Space::Private))
    ));
}

#[cfg(windows)]
#[test]
fn junction_escape_is_rejected() {
    let (tmp, reg) = test_vault();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("x.md"), "# outside\n").unwrap();
    junction::create(&outside, vault_root(&tmp).join("Notes/link")).unwrap();
    let vault = reg.vault(Space::Work).unwrap();
    assert!(matches!(
        vault.read_note(&vp("Notes/link/x.md")),
        Err(VaultError::OutsideVault { .. })
    ));
    assert!(matches!(
        vault.list(Some(&vp("Notes/link"))),
        Err(VaultError::OutsideVault { .. })
    ));
    let names: Vec<_> = vault
        .list(Some(&vp("Notes")))
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, ["duplicate-headings.md"]);
}

#[cfg(windows)]
#[test]
fn junction_into_excluded_is_rejected() {
    let (tmp, reg) = test_vault();
    junction::create(
        vault_root(&tmp).join("Dostepy"),
        vault_root(&tmp).join("Notes/link"),
    )
    .unwrap();
    let vault = reg.vault(Space::Work).unwrap();
    assert!(matches!(
        vault.read_note(&vp("Notes/link/invented-secret.md")),
        Err(VaultError::Excluded { .. })
    ));
    assert!(matches!(
        vault.list(Some(&vp("Notes/link"))),
        Err(VaultError::Excluded { .. })
    ));
    let names: Vec<_> = vault
        .list(Some(&vp("Notes")))
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, ["duplicate-headings.md"]);
}

#[cfg(windows)]
#[test]
fn junction_into_hidden_is_rejected() {
    let (tmp, reg) = test_vault();
    junction::create(
        vault_root(&tmp).join(".obsidian"),
        vault_root(&tmp).join("Notes/cfg"),
    )
    .unwrap();
    let vault = reg.vault(Space::Work).unwrap();
    assert!(matches!(
        vault.list(Some(&vp("Notes/cfg"))),
        Err(VaultError::Hidden { .. })
    ));
}

#[test]
fn notes_beyond_260_characters_are_readable_and_listed() {
    let (tmp, reg) = test_vault();
    let root = vault_root(&tmp);
    let vault = reg.vault(Space::Work).unwrap();
    let seg = "g".repeat(100);
    let dir_rel = format!("Deep/{seg}/{seg}");
    let note_rel = format!("{dir_rel}/long note.md");
    let abs = root.join(&note_rel);
    assert!(
        abs.as_os_str().len() > 260,
        "test path must exceed MAX_PATH"
    );
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    fs::write(&abs, b"# Long\n").unwrap();

    let note = vault.read_note(&vp(&note_rel)).unwrap();
    assert_eq!(note.text(), "# Long\n");
    let listed = vault.list(Some(&vp(&dir_rel))).unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0].name, "long note.md");
    assert_eq!(listed[0].kind, EntryKind::Note);
}

#[test]
fn exclusion_matches_nfd_folder_names_from_nfc_settings() {
    // Settings typed on Windows (NFC `ę`), folder stored as NFD (`e` + U+0328).
    let nfc = "Dost\u{0119}py";
    let nfd = "Doste\u{0328}py";
    let (tmp, reg) = test_vault_with(|root| Settings {
        work: Some(space_settings(root, &[nfc])),
        private: None,
    });
    let root = vault_root(&tmp);
    fs::create_dir(root.join(nfd)).unwrap();
    fs::write(root.join(nfd).join("x.md"), b"# invented\n").unwrap();
    let vault = reg.vault(Space::Work).unwrap();

    assert!(matches!(
        vault.read_note(&vp(&format!("{nfd}/x.md"))),
        Err(VaultError::Excluded { .. })
    ));
    assert!(matches!(
        vault.list(Some(&vp(nfd))),
        Err(VaultError::Excluded { .. })
    ));
    let names: Vec<String> = vault
        .list(None)
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert!(!names.iter().any(|n| n == nfd), "{names:?}");
    assert!(vault.unmatched_exclusions().unwrap().is_empty());
}

#[test]
fn unmatched_exclusions_lists_entries_that_match_nothing() {
    let (_tmp, reg) = test_vault_with(|root| Settings {
        work: Some(space_settings(
            root,
            &["dostepy", "Dostpey", "Notes/duplicate-headings.md"],
        )),
        private: None,
    });
    let vault = reg.vault(Space::Work).unwrap();
    let unmatched: Vec<String> = vault
        .unmatched_exclusions()
        .unwrap()
        .iter()
        .map(|p| p.to_string())
        .collect();
    assert_eq!(unmatched, vec!["Dostpey".to_string()]);
}
