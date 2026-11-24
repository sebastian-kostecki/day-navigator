//! Shared helpers for the store tests: a temp data folder and the crash harness.
#![allow(dead_code)]

pub mod crash;

use std::path::PathBuf;

use tempfile::TempDir;

/// A not-yet-created own data folder inside a temp dir, for `Store::open`.
pub fn test_store_dir() -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("data");
    (tmp, dir)
}
