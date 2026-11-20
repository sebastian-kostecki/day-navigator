//! The only code that writes bytes into a vault: atomic replace, no-clobber
//! create and no-clobber move. Paths here are absolute and already guarded by
//! `Vault`; `label` is the vault-relative name used in errors.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::error::VaultError;

const ERR_SHARING_VIOLATION: u32 = 32;
const ERR_LOCK_VIOLATION: u32 = 33;
const ERR_UNABLE_TO_MOVE_REPLACEMENT: u32 = 1176;
const ERR_UNABLE_TO_MOVE_REPLACEMENT_2: u32 = 1177;
#[cfg(windows)]
const ERR_ALREADY_EXISTS: u32 = 183;
#[cfg(windows)]
const ERR_FILE_EXISTS: u32 = 80;

const RETRY_DELAYS_MS: [u64; 4] = [30, 60, 120, 240];
const LEFTOVER_MIN_AGE: Duration = Duration::from_secs(60);

/// Outcome of `replace_atomic` other than success.
#[derive(Debug)]
pub enum ReplaceError {
    /// The target no longer matches the bytes the edit was computed from.
    Changed,
    Failed(VaultError),
}

impl From<VaultError> for ReplaceError {
    fn from(e: VaultError) -> Self {
        ReplaceError::Failed(e)
    }
}

/// Replace call: `(target, temp, backup)`; `Err` carries the OS error code.
pub type ReplaceFn<'a> = &'a dyn Fn(&Path, &Path, &Path) -> Result<(), u32>;

fn io_err(label: &str, e: &std::io::Error) -> VaultError {
    if e.kind() == std::io::ErrorKind::NotFound {
        VaultError::NotFound {
            path: label.to_string(),
        }
    } else {
        VaultError::Io {
            path: label.to_string(),
            kind: format!("{:?}", e.kind()),
        }
    }
}

fn file_name_of(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Vault-relative sibling of `label` named `name`.
fn sibling_label(label: &str, name: &str) -> String {
    match label.rfind('/') {
        Some(i) => format!("{}/{}", &label[..i], name),
        None => name.to_string(),
    }
}

/// Atomically replaces `target` with `new_bytes`, provided it still holds `expected_current`.
pub fn replace_atomic(
    target: &Path,
    new_bytes: &[u8],
    expected_current: &[u8],
    label: &str,
) -> Result<(), ReplaceError> {
    replace_atomic_with(target, new_bytes, expected_current, label, &os_replace)
}

/// Same as `replace_atomic`, with the platform replace call injected (tests force failure branches).
pub fn replace_atomic_with(
    target: &Path,
    new_bytes: &[u8],
    expected_current: &[u8],
    label: &str,
    replace: ReplaceFn<'_>,
) -> Result<(), ReplaceError> {
    let dir = target.parent().ok_or_else(|| VaultError::NotFound {
        path: label.to_string(),
    })?;
    let name = file_name_of(target);

    // 0. Crash leftovers of this note.
    if target.is_file() {
        remove_leftovers(dir, &name);
    }

    // 1. Temp file in the same folder, flushed, synced and closed.
    let temp = write_temp(dir, &name, new_bytes, label)?;

    // 2. Re-read: the target must be what the edit was computed from, and writable.
    //    A note still locked by another writer reads as `Busy`.
    let precheck = || -> Result<(), ReplaceError> {
        let current = fs::read(target).map_err(|e| {
            if is_lock_error(&e) {
                VaultError::Busy {
                    path: label.to_string(),
                }
            } else {
                io_err(label, &e)
            }
        })?;
        if current != expected_current {
            return Err(ReplaceError::Changed);
        }
        let meta = fs::metadata(target).map_err(|e| io_err(label, &e))?;
        if meta.permissions().readonly() {
            return Err(VaultError::ReadOnly {
                path: label.to_string(),
            }
            .into());
        }
        Ok(())
    };
    if let Err(e) = precheck() {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }

    // 3-5. Replace. Sharing/lock violations, and a 1176 that left both files in
    // place, are retried with backoff. Every retry re-runs the step-2 check first:
    // a write that landed while the note was locked is never overwritten.
    let backup = temp.with_extension("bak");
    let mut attempt = 0;
    loop {
        let code = match replace(target, &temp, &backup) {
            Ok(()) => {
                let _ = fs::remove_file(&backup);
                return Ok(());
            }
            Err(code) => code,
        };
        let retryable = match code {
            ERR_SHARING_VIOLATION | ERR_LOCK_VIOLATION => true,
            // Documented for ReplaceFileW with a backup name: "the replaced and
            // replacement files retain their original file names" — the target
            // is untouched, so this is safe to retry.
            ERR_UNABLE_TO_MOVE_REPLACEMENT => !backup.exists(),
            _ => false,
        };
        if retryable {
            loop {
                if attempt == RETRY_DELAYS_MS.len() {
                    let _ = fs::remove_file(&temp);
                    return Err(if code == ERR_UNABLE_TO_MOVE_REPLACEMENT {
                        VaultError::Io {
                            path: label.to_string(),
                            kind: format!("ReplaceFile failed ({code}), note unchanged"),
                        }
                    } else {
                        VaultError::Busy {
                            path: label.to_string(),
                        }
                    }
                    .into());
                }
                std::thread::sleep(Duration::from_millis(RETRY_DELAYS_MS[attempt]));
                attempt += 1;
                match precheck() {
                    Ok(()) => break,
                    // Still locked: wait again without calling replace.
                    Err(ReplaceError::Failed(VaultError::Busy { .. })) => {}
                    Err(e) => {
                        let _ = fs::remove_file(&temp);
                        return Err(e);
                    }
                }
            }
            continue;
        }
        match code {
            ERR_UNABLE_TO_MOVE_REPLACEMENT => {
                // Undocumented state, handled defensively: the original was moved
                // to the backup name and the new content is only in the temp file.
                return match move_replace(&temp, target) {
                    Ok(()) => {
                        let _ = fs::remove_file(&backup);
                        Ok(())
                    }
                    Err(_) => Err(incomplete(label, &temp, &backup)),
                };
            }
            ERR_UNABLE_TO_MOVE_REPLACEMENT_2 => {
                // Original was moved to the backup name; put it back.
                return match move_replace(&backup, target) {
                    Ok(()) => {
                        let _ = fs::remove_file(&temp);
                        Err(VaultError::Io {
                            path: label.to_string(),
                            kind: format!("ReplaceFile failed ({ERR_UNABLE_TO_MOVE_REPLACEMENT_2}), original restored"),
                        }
                        .into())
                    }
                    Err(_) => Err(incomplete(label, &temp, &backup)),
                };
            }
            _ => {
                let _ = fs::remove_file(&temp);
                return Err(VaultError::Io {
                    path: label.to_string(),
                    kind: format!("ReplaceFile os error {code}"),
                }
                .into());
            }
        }
    }
}

fn is_lock_error(e: &std::io::Error) -> bool {
    matches!(
        e.raw_os_error().map(|c| c as u32),
        Some(ERR_SHARING_VIOLATION | ERR_LOCK_VIOLATION)
    )
}

fn incomplete(label: &str, temp: &Path, backup: &Path) -> ReplaceError {
    VaultError::ReplaceIncomplete {
        path: label.to_string(),
        temp: sibling_label(label, &file_name_of(temp)),
        backup: sibling_label(label, &file_name_of(backup)),
    }
    .into()
}

/// Writes `bytes` to a new `.<name>.dn-<random>.tmp` file and returns its path.
/// On any failure the temp file is removed.
fn write_temp(dir: &Path, name: &str, bytes: &[u8], label: &str) -> Result<PathBuf, VaultError> {
    let mut tf = new_temp(dir, name).map_err(|e| io_err(label, &e))?;
    tf.write_all(bytes)
        .and_then(|_| tf.flush())
        .and_then(|_| tf.as_file().sync_all())
        .map_err(|e| io_err(label, &e))?;
    // Drop the handle (ReplaceFileW needs the replacement unopened), keep the file.
    tf.into_temp_path()
        .keep()
        .map_err(|e| io_err(label, &e.error))
}

fn new_temp(dir: &Path, name: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let prefix = format!(".{name}.dn-");
    tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".tmp")
        .rand_bytes(8)
        .tempfile_in(dir)
}

/// Removes this note's `.dn-*.tmp` files older than a minute. Best effort.
/// `.dn-*.bak` files are never deleted: after a failed recovery a backup can be
/// the only copy of the original note (`ReplaceIncomplete` names it).
fn remove_leftovers(dir: &Path, name: &str) {
    let prefix = format!(".{name}.dn-");
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let fname = entry.file_name().to_string_lossy().into_owned();
        if !fname.starts_with(&prefix) || !fname.ends_with(".tmp") {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age >= LEFTOVER_MIN_AGE);
        if old {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Creates `target` with `bytes`; an existing target is `AlreadyExists`.
pub fn create_new(target: &Path, bytes: &[u8], label: &str) -> Result<(), VaultError> {
    let dir = target.parent().ok_or_else(|| VaultError::NotFound {
        path: label.to_string(),
    })?;
    let mut tf = new_temp(dir, &file_name_of(target)).map_err(|e| io_err(label, &e))?;
    tf.write_all(bytes)
        .and_then(|_| tf.flush())
        .and_then(|_| tf.as_file().sync_all())
        .map_err(|e| io_err(label, &e))?;
    // On failure the error owns the temp file and removes it when dropped.
    tf.persist_noclobber(target).map(|_| ()).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            VaultError::AlreadyExists {
                path: label.to_string(),
            }
        } else {
            io_err(label, &e.error)
        }
    })
}

/// Moves `from` to `to` without replacing an existing `to`.
pub fn move_noclobber(from: &Path, to: &Path, label: &str) -> Result<(), VaultError> {
    imp_move_noclobber(from, to).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists
            || matches!(e.raw_os_error(), Some(c) if is_exists_code(c))
        {
            VaultError::AlreadyExists {
                path: label.to_string(),
            }
        } else {
            io_err(label, &e)
        }
    })
}

#[cfg(windows)]
fn is_exists_code(c: i32) -> bool {
    c as u32 == ERR_ALREADY_EXISTS || c as u32 == ERR_FILE_EXISTS
}
#[cfg(not(windows))]
fn is_exists_code(_c: i32) -> bool {
    false
}

#[cfg(windows)]
fn wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn imp_move_noclobber(from: &Path, to: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let (f, t) = (wide(from), wide(to));
    // SAFETY: both buffers are NUL-terminated and outlive the call.
    if unsafe { MoveFileExW(f.as_ptr(), t.as_ptr(), 0) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn imp_move_noclobber(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::hard_link(from, to)?;
    fs::remove_file(from)
}

/// Moves `from` onto `to`, replacing it (recovery renames).
#[cfg(windows)]
fn move_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MoveFileExW};
    let (f, t) = (wide(from), wide(to));
    // SAFETY: both buffers are NUL-terminated and outlive the call.
    if unsafe { MoveFileExW(f.as_ptr(), t.as_ptr(), MOVEFILE_REPLACE_EXISTING) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn move_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}

/// `ReplaceFileW` keeping the original's creation time, attributes and ACLs.
#[cfg(windows)]
fn os_replace(target: &Path, temp: &Path, backup: &Path) -> Result<(), u32> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{
        REPLACEFILE_IGNORE_ACL_ERRORS, REPLACEFILE_IGNORE_MERGE_ERRORS, ReplaceFileW,
    };
    let (t, r, b) = (wide(target), wide(temp), wide(backup));
    // SAFETY: all three buffers are NUL-terminated and outlive the call; the two
    // trailing pointers are reserved and must be null.
    let ok = unsafe {
        ReplaceFileW(
            t.as_ptr(),
            r.as_ptr(),
            b.as_ptr(),
            REPLACEFILE_IGNORE_MERGE_ERRORS | REPLACEFILE_IGNORE_ACL_ERRORS,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        // SAFETY: plain thread-local error read right after the failed call.
        Err(unsafe { GetLastError() })
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn os_replace(target: &Path, temp: &Path, _backup: &Path) -> Result<(), u32> {
    fs::rename(temp, target).map_err(|e| e.raw_os_error().unwrap_or(1) as u32)
}
