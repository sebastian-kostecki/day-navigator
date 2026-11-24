//! Daily backup of the store: one consistent copy per calendar day, newest 7 kept.
//!
//! A failure is logged (error kind and file names only, never row data or
//! absolute paths) and never blocks the store.

use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;

use super::{BACKUP_DIR, Store};

const KEEP: usize = 7;
const PREFIX: &str = "day-navigator-";
const SUFFIX: &str = ".db";

/// The newest backup at or before today. `date` comes from the file name,
/// `at_ms` from the file's modified time (`None` when it cannot be read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastBackup {
    pub date: NaiveDate,
    pub at_ms: Option<u64>,
}

/// What `backups/` holds, read without the database. Carries no paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum BackupSummary {
    #[serde(rename_all = "camelCase")]
    Listed {
        kept: u32,
        keep: u32,
        last: Option<LastBackup>,
        stale: bool,
    },
    Unreadable,
}

/// Counts the dated backups up to `today` in `dir` (the `backups/` folder).
/// A missing folder is an empty, stale list; any other read failure is
/// `Unreadable` (logged at debug level with the error kind only).
pub(crate) fn summarize(dir: &Path, today: NaiveDate) -> BackupSummary {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return BackupSummary::Listed {
                kept: 0,
                keep: KEEP as u32,
                last: None,
                stale: true,
            };
        }
        Err(e) => {
            log::debug!("backups: summary unreadable: {:?}", e.kind());
            return BackupSummary::Unreadable;
        }
    };
    let mut kept: u32 = 0;
    let mut newest: Option<(NaiveDate, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        let Some(date) = dated(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        if date > today {
            continue;
        }
        kept += 1;
        if newest.as_ref().is_none_or(|(d, _)| date > *d) {
            newest = Some((date, entry.path()));
        }
    }
    let last = newest.map(|(date, path)| LastBackup {
        date,
        at_ms: fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64),
    });
    BackupSummary::Listed {
        kept,
        keep: KEEP as u32,
        stale: last.is_none_or(|l| l.date < today),
        last,
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, d).unwrap()
    }

    fn touch(dir: &Path, name: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        fs::write(&p, "invented").unwrap();
        p
    }

    fn listed(s: BackupSummary) -> (u32, u32, Option<LastBackup>, bool) {
        match s {
            BackupSummary::Listed {
                kept,
                keep,
                last,
                stale,
            } => (kept, keep, last, stale),
            BackupSummary::Unreadable => panic!("expected Listed"),
        }
    }

    #[test]
    fn a_missing_folder_is_an_empty_stale_list() {
        let tmp = tempfile::tempdir().unwrap();
        let s = summarize(&tmp.path().join("backups"), day(4));
        assert_eq!(
            s,
            BackupSummary::Listed {
                kept: 0,
                keep: 7,
                last: None,
                stale: true
            }
        );
    }

    #[test]
    fn counts_copies_and_reads_the_newest_time() {
        let tmp = tempfile::tempdir().unwrap();
        for d in [2, 3, 4] {
            touch(tmp.path(), &final_name(day(d)));
        }
        let when = UNIX_EPOCH + Duration::from_millis(1_700_000_123_000);
        fs::File::options()
            .write(true)
            .open(tmp.path().join(final_name(day(4))))
            .unwrap()
            .set_modified(when)
            .unwrap();
        let (kept, keep, last, stale) = listed(summarize(tmp.path(), day(4)));
        assert_eq!((kept, keep, stale), (3, 7, false));
        let last = last.unwrap();
        assert_eq!(last.date, day(4));
        assert_eq!(last.at_ms, Some(1_700_000_123_000));
    }

    #[test]
    fn future_copies_are_not_counted() {
        let tmp = tempfile::tempdir().unwrap();
        for d in 1..=7 {
            touch(tmp.path(), &final_name(day(d)));
        }
        touch(tmp.path(), &final_name(day(20)));
        let (kept, _, last, stale) = listed(summarize(tmp.path(), day(7)));
        assert_eq!(kept, 7);
        assert_eq!(last.unwrap().date, day(7));
        assert!(!stale);
    }

    #[test]
    fn a_copy_from_yesterday_is_stale() {
        let tmp = tempfile::tempdir().unwrap();
        touch(tmp.path(), &final_name(day(3)));
        let (kept, _, last, stale) = listed(summarize(tmp.path(), day(4)));
        assert_eq!(kept, 1);
        assert_eq!(last.unwrap().date, day(3));
        assert!(stale);
    }

    #[test]
    fn foreign_and_temp_files_are_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        touch(tmp.path(), &final_name(day(4)));
        touch(tmp.path(), &temp_name(day(4)));
        touch(tmp.path(), "notes.txt");
        touch(tmp.path(), "day-navigator-nie-data.db");
        let (kept, _, _, _) = listed(summarize(tmp.path(), day(4)));
        assert_eq!(kept, 1);
    }

    #[test]
    fn a_file_in_place_of_the_folder_is_unreadable() {
        let tmp = tempfile::tempdir().unwrap();
        let p = touch(tmp.path(), "backups");
        assert_eq!(summarize(&p, day(4)), BackupSummary::Unreadable);
    }
}
