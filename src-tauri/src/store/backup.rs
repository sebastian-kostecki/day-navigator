//! Daily backup of the store: one consistent copy per calendar day, newest 7 kept.
//!
//! A failure is logged (error kind and file names only, never row data or
//! absolute paths) and never blocks the store.

use std::fs;
use std::path::Path;

use chrono::NaiveDate;

use super::{BACKUP_DIR, Store};

const KEEP: usize = 7;
const PREFIX: &str = "day-navigator-";
const SUFFIX: &str = ".db";

fn final_name(date: NaiveDate) -> String {
    format!("{PREFIX}{date}{SUFFIX}")
}

fn temp_name(date: NaiveDate) -> String {
    format!(".{PREFIX}{date}{SUFFIX}.tmp")
}

/// The date of a backup file named `day-navigator-YYYY-MM-DD.db`.
pub(super) fn dated(name: &str) -> Option<NaiveDate> {
    name.strip_prefix(PREFIX)
        .and_then(|r| r.strip_suffix(SUFFIX))
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

impl Store {
    /// Writes today's backup unless one exists or was already attempted today.
    pub fn backup_if_due(&mut self, today: NaiveDate) {
        if self.last_backup_attempt == Some(today) {
            return;
        }
        self.last_backup_attempt = Some(today);

        let dir = self.dir.join(BACKUP_DIR);
        let final_file = final_name(today);
        if dir.join(&final_file).exists() {
            return;
        }
        if let Err(kind) = self.write_backup(&dir, today) {
            log::warn!("daily backup {final_file} failed: {kind}");
            return;
        }
        prune(&dir, today);
    }

    fn write_backup(&self, dir: &Path, today: NaiveDate) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| format!("{:?}", e.kind()))?;
        let tmp = dir.join(temp_name(today));
        // VACUUM INTO refuses an existing target, so clear a stale one first.
        if tmp.exists() {
            fs::remove_file(&tmp).map_err(|e| format!("{:?}", e.kind()))?;
        }
        let tmp_str = tmp.to_str().ok_or_else(|| "non-UTF-8 path".to_string())?;
        // Code only: SQLite's message for a failed VACUUM INTO names the
        // target file with its absolute path.
        self.conn
            .execute("VACUUM INTO ?1", [tmp_str])
            .map_err(|e| match e.sqlite_error_code() {
                Some(code) => format!("{code:?}"),
                None => "sqlite error".to_string(),
            })?;
        // On disk before it can count as today's copy.
        // Write access: Windows refuses to flush a read-only handle.
        let synced = fs::OpenOptions::new()
            .write(true)
            .open(&tmp)
            .and_then(|f| f.sync_all());
        if let Err(e) = synced {
            let _ = fs::remove_file(&tmp);
            return Err(format!("{:?}", e.kind()));
        }
        // hard_link fails instead of replacing a file that appeared meanwhile.
        let linked = fs::hard_link(&tmp, dir.join(final_name(today)));
        let _ = fs::remove_file(&tmp);
        linked.map_err(|e| format!("{:?}", e.kind()))
    }
}

/// Keeps the newest `KEEP` dated backups up to `today` and removes leftover
/// `.tmp` files; any other file is left alone. Backups dated after `today`
/// (a clock that was once set forward) are kept and take no slot, so they
/// never push out today's copy.
fn prune(dir: &Path, today: NaiveDate) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut backups: Vec<(NaiveDate, String)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(date) = dated(&name) {
            if date <= today {
                backups.push((date, name));
            }
        } else if name.starts_with(&format!(".{PREFIX}"))
            && name.ends_with(".tmp")
            && let Err(e) = fs::remove_file(entry.path())
        {
            log::warn!("removing leftover {name} failed: {:?}", e.kind());
        }
    }
    backups.sort_by_key(|b| std::cmp::Reverse(b.0));
    for (_, name) in backups.into_iter().skip(KEEP) {
        if let Err(e) = fs::remove_file(dir.join(&name)) {
            log::warn!("removing old backup {name} failed: {:?}", e.kind());
        }
    }
}
