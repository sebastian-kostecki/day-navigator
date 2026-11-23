mod common;

use std::fs;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::store::{DB_FILE, Migration, Store, StoreError, schema};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

const V1: Migration = Migration {
    version: 1,
    sql: "CREATE TABLE items(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);",
};
const V2_OK: Migration = Migration {
    version: 2,
    sql: "CREATE TABLE extra(id INTEGER PRIMARY KEY);",
};
const V2_BAD: Migration = Migration {
    version: 2,
    sql: "CREATE TABLE half(id INTEGER); INSERT INTO no_such_table VALUES (1);",
};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn digest(path: &std::path::Path) -> Vec<u8> {
    Sha256::digest(fs::read(path).unwrap()).to_vec()
}

fn insert(store: &mut Store, name: &str) {
    store
        .write::<_, StoreError>(|tx| {
            tx.execute("INSERT INTO items(name) VALUES (?1)", [name])?;
            Ok(())
        })
        .unwrap();
}

fn count(store: &Store) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row("SELECT count(*) FROM items", [], |r| r.get(0))?))
        .unwrap()
}

#[test]
fn fresh_open_creates_folder_file_and_reaches_latest_version() {
    let (_tmp, dir) = test_store_dir();
    let store = Store::open(&dir, &[V1, V2_OK], today()).unwrap();
    assert!(dir.join(DB_FILE).is_file());
    let v: i64 = store
        .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(v, 2);
    store.check().unwrap();
}

#[test]
fn pragmas_are_set() {
    let (_tmp, dir) = test_store_dir();
    let store = Store::open(&dir, &[V1], today()).unwrap();
    store
        .read::<_, StoreError>(|c| {
            let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
            let sync: i64 = c.query_row("PRAGMA synchronous", [], |r| r.get(0))?;
            let fk: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
            assert_eq!(mode, "wal");
            assert_eq!(sync, 2);
            assert_eq!(fk, 1);
            Ok(())
        })
        .unwrap();
}

#[test]
fn data_survives_reopen() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], today()).unwrap();
    insert(&mut store, "alpha");
    drop(store);
    let store = Store::open(&dir, &[V1], today()).unwrap();
    assert_eq!(count(&store), 1);
}

#[test]
fn failing_write_rolls_back() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], today()).unwrap();
    let r = store.write::<(), StoreError>(|tx| {
        tx.execute("INSERT INTO items(name) VALUES ('a')", [])?;
        Err(StoreError::Busy)
    });
    assert!(matches!(r, Err(StoreError::Busy)));
    assert_eq!(count(&store), 0);
}

#[test]
fn panicking_write_rolls_back() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], today()).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = store.write::<(), StoreError>(|tx| {
            tx.execute("INSERT INTO items(name) VALUES ('a')", [])?;
            panic!("boom");
        });
    }));
    assert!(r.is_err());
    assert_eq!(count(&store), 0);
    insert(&mut store, "after");
    assert_eq!(count(&store), 1);
}

#[test]
fn newer_schema_is_refused_and_file_untouched() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(DB_FILE);
    {
        // Default rollback-journal mode on purpose: a WAL switch before the
        // checks would change the bytes.
        let c = Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE items(id INTEGER); PRAGMA user_version = 5;")
            .unwrap();
    }
    let before = digest(&path);
    let r = Store::open(&dir, &[V1], today());
    assert!(matches!(
        r,
        Err(StoreError::NewerSchema {
            found: 5,
            supported: 1
        })
    ));
    assert_eq!(digest(&path), before);
}

#[test]
fn non_sqlite_file_is_damaged_and_untouched() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(DB_FILE);
    fs::write(
        &path,
        "this is plainly not a database, just some invented text ".repeat(20),
    )
    .unwrap();
    let before = digest(&path);
    let r = Store::open(&dir, &[V1], today());
    assert!(matches!(r, Err(StoreError::Damaged { .. })));
    assert_eq!(digest(&path), before);
}

#[test]
fn zero_byte_file_is_damaged_and_stays_empty() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(DB_FILE);
    fs::write(&path, b"").unwrap();
    let r = Store::open(&dir, &[V1], today());
    assert!(matches!(r, Err(StoreError::Damaged { .. })));
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
}

#[test]
fn missing_database_with_dated_backup_is_refused() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(dir.join("backups")).unwrap();
    fs::write(dir.join("backups/day-navigator-2026-11-22.db"), b"x").unwrap();
    let r = Store::open(&dir, &[V1], today());
    assert!(matches!(r, Err(StoreError::Missing)));
    assert!(!dir.join(DB_FILE).exists());
}

#[test]
fn undated_backup_file_does_not_count() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(dir.join("backups")).unwrap();
    fs::write(dir.join("backups/notes.txt"), b"x").unwrap();
    assert!(Store::open(&dir, &[V1], today()).is_ok());
}

#[test]
fn empty_dir_is_a_first_run() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    assert!(Store::open(&dir, &[V1], today()).is_ok());
    assert!(dir.join(DB_FILE).is_file());
}

#[test]
fn failing_migration_keeps_previous_version() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], today()).unwrap();
    insert(&mut store, "keep");
    drop(store);

    let r = Store::open(&dir, &[V1, V2_BAD], today());
    assert!(matches!(
        r,
        Err(StoreError::MigrationFailed { version: 2, .. })
    ));

    let c = Connection::open(dir.join(DB_FILE)).unwrap();
    let v: i64 = c
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 1);
    let n: i64 = c
        .query_row("SELECT count(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
    let half: i64 = c
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'half'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(half, 0);
}

#[test]
fn malformed_migration_list_is_refused_at_open() {
    let (_tmp, dir) = test_store_dir();
    let r = Store::open(&dir, &[V2_OK], today());
    assert!(matches!(r, Err(StoreError::MigrationFailed { .. })));
}

#[test]
fn validate_rules() {
    let gap = [
        V1,
        Migration {
            version: 3,
            sql: "",
        },
    ];
    let dup = [
        V1,
        Migration {
            version: 1,
            sql: "",
        },
    ];
    assert!(schema::validate(&gap).is_err());
    assert!(schema::validate(&dup).is_err());
    assert!(schema::validate(&[V2_OK]).is_err());
    assert!(schema::validate(&[V1, V2_OK]).is_ok());
    assert!(schema::validate(schema::MIGRATIONS).is_ok());
}

#[test]
fn constraint_error_carries_no_row_values() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], today()).unwrap();
    const SENTINEL: &str = "SENTINEL-invented-4f2a9";
    insert(&mut store, SENTINEL);
    let err = store
        .write::<(), StoreError>(|tx| {
            tx.execute("INSERT INTO items(name) VALUES (?1)", [SENTINEL])?;
            Ok(())
        })
        .unwrap_err();
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "Sql");
    assert!(json["detail"]["reason"].is_string());
    assert!(!json.to_string().contains(SENTINEL));
    assert!(!err.to_string().contains(SENTINEL));
}
