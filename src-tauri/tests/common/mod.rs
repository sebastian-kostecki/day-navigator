//! Shared helpers: copy the invented fixture vault into a temp dir and build a registry.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use day_navigator_lib::settings::{Settings, SpaceSettings, TaskSettings};
use day_navigator_lib::vault::{VaultPath, VaultRegistry};
use tempfile::TempDir;

pub fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault")
}

pub fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), &to).unwrap();
        }
    }
}

/// Panics if any `.dn-*.tmp` / `.dn-*.bak` file exists anywhere under `dir`.
pub fn assert_no_leftovers(dir: &Path) {
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

pub fn space_settings(root: &Path, excluded: &[&str]) -> SpaceSettings {
    SpaceSettings {
        vault: root.to_path_buf(),
        excluded: excluded
            .iter()
            .map(|e| VaultPath::parse(e).unwrap())
            .collect(),
        tasks: None,
    }
}

/// Invented task settings matching the fixture vault's `Tasks/` and `Templates/`.
pub fn test_tasks_settings() -> TaskSettings {
    TaskSettings {
        folder: VaultPath::parse("Tasks").unwrap(),
        template: VaultPath::parse("Templates/Task.md").unwrap(),
        statuses: ["Todo", "In progress", "Done"].map(String::from).to_vec(),
        done_status: "Done".to_string(),
        priorities: ["High", "Medium", "Low"].map(String::from).to_vec(),
    }
}

/// Vault root inside the temp dir returned by the helpers below.
pub fn vault_root(tmp: &TempDir) -> PathBuf {
    tmp.path().join("vault")
}

/// Work space over a temp copy of the fixture, with `Dostepy` excluded.
pub fn test_vault() -> (TempDir, VaultRegistry) {
    test_vault_with(|root| Settings {
        work: Some(space_settings(root, &["Dostepy"])),
        private: None,
    })
}

/// Same copy, with custom settings built from the copied vault root.
pub fn test_vault_with(make: impl FnOnce(&Path) -> Settings) -> (TempDir, VaultRegistry) {
    let tmp = tempfile::tempdir().unwrap();
    let root = vault_root(&tmp);
    copy_dir(&fixture_dir(), &root);
    let registry = VaultRegistry::from_settings(&make(&root));
    (tmp, registry)
}

/// A not-yet-created own data folder inside a temp dir, for `Store::open`.
pub fn test_store_dir() -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("data");
    (tmp, dir)
}
