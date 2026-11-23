//! App-owned data store: one SQLite file in the app's own data folder.
//!
//! No `tauri` import; time is passed in. Opening follows a fixed order (see
//! `Store::open`) so every refusal happens before anything writes to the file.

mod backup;
pub mod error;
pub mod schema;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::NaiveDate;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub use error::StoreError;

pub const DB_FILE: &str = "day-navigator.db";
const BACKUP_DIR: &str = "backups";

pub struct Migration {
    pub version: u32,
    pub sql: &'static str,
}

pub struct Store {
    conn: Connection,
    dir: PathBuf,
    last_backup_attempt: Option<NaiveDate>,
}

/// `backups/` holds at least one file named `day-navigator-YYYY-MM-DD.db`.
fn has_dated_backup(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir.join(BACKUP_DIR)) else {
        return false;
    };
    entries.flatten().any(|e| {
        let name = e.file_name();
        let name = name.to_string_lossy();
        name.strip_prefix("day-navigator-")
            .and_then(|r| r.strip_suffix(".db"))
            .is_some_and(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
    })
}

impl Store {
    /// Open sequence (order is load-bearing): create the folder, refuse a lost
    /// file, open, `busy_timeout`, `quick_check`, compare `user_version`, then
    /// set WAL / FULL / foreign keys, then apply pending migrations. WAL is a
    /// header write, so it comes only after every check.
    pub fn open(
        dir: &Path,
        migrations: &[Migration],
        today: NaiveDate,
    ) -> Result<Store, StoreError> {
        schema::validate(migrations)?;
        fs::create_dir_all(dir).map_err(|e| StoreError::DirUnavailable {
            kind: format!("{:?}", e.kind()),
        })?;

        let path = dir.join(DB_FILE);
        match fs::metadata(&path) {
            Ok(meta) if meta.len() == 0 => {
                return Err(StoreError::Damaged {
                    reason: "data file is empty".into(),
                });
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if has_dated_backup(dir) {
                    return Err(StoreError::Missing);
                }
            }
            Err(e) => {
                return Err(StoreError::Io {
                    kind: format!("{:?}", e.kind()),
                });
            }
        }

        let conn = Connection::open(&path)?;
        conn.busy_timeout(Duration::from_secs(5))?;

        let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(StoreError::Damaged {
                reason: "integrity check failed".into(),
            });
        }

        let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let supported = migrations.last().map_or(0, |m| m.version);
        if found < 0 || found > i64::from(supported) {
            return Err(StoreError::NewerSchema {
                found: u32::try_from(found).unwrap_or(u32::MAX),
                supported,
            });
        }

        let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::Io {
                kind: "WAL mode unavailable".into(),
            });
        }
        conn.execute_batch("PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")?;

        let mut store = Store {
            conn,
            dir: dir.to_path_buf(),
            last_backup_attempt: None,
        };
        // After the checks, before migrations: a failing migration always has
        // a same-day copy from before it. Never fails the open.
        store.backup_if_due(today);
        store.migrate(migrations, found as u32)?;
        Ok(store)
    }

    fn migrate(&mut self, migrations: &[Migration], current: u32) -> Result<(), StoreError> {
        for m in migrations.iter().filter(|m| m.version > current) {
            let failed = |e: rusqlite::Error| StoreError::MigrationFailed {
                version: m.version,
                reason: error::reason_of(&e),
            };
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(failed)?;
            tx.execute_batch(m.sql).map_err(failed)?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", m.version))
                .map_err(failed)?;
            tx.commit().map_err(failed)?;
        }
        Ok(())
    }

    pub fn read<T, E: From<StoreError>>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<T, E> {
        f(&self.conn)
    }

    /// Commits only if `f` returns Ok; any Err (or panic) rolls back the whole
    /// transaction.
    pub fn write<T, E: From<StoreError>>(
        &mut self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| E::from(e.into()))?;
        let value = f(&tx)?;
        tx.commit().map_err(|e| E::from(e.into()))?;
        Ok(value)
    }

    /// Cheap liveness query for the health round.
    pub fn check(&self) -> Result<(), StoreError> {
        self.conn
            .query_row("SELECT 1", [], |r| r.get::<_, i64>(0))?;
        Ok(())
    }
}
