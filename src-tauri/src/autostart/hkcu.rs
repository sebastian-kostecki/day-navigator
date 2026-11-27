//! The one production `RunEntry`: a single REG_SZ value under one fixed key in
//! HKCU. The only code in the app that touches the registry. HKCU only, never
//! HKLM, never `StartupApproved`. No test module here: tests use the in-memory
//! fake, never the real registry.

use std::io;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};

use super::{RUN_KEY, RunEntry, VALUE_NAME};

pub struct HkcuRun;

impl RunEntry for HkcuRun {
    fn read(&self) -> io::Result<Option<String>> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
            Ok(k) => k,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        match key.get_value::<String, _>(VALUE_NAME) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&self, value: &str) -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(RUN_KEY)?;
        key.set_value(VALUE_NAME, &value)
    }

    fn remove(&self) -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
            Ok(k) => k,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        match key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}
