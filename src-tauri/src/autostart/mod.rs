//! Windows autostart rules: which launches are autostart launches, what the
//! Run entry must hold, the sync against any `RunEntry`, and the health status
//! of the result. No `tauri` import and no registry access here; the real
//! registry value is a `RunEntry` implemented elsewhere.

use std::ffi::OsStr;
use std::io;
use std::path::Path;

use crate::health::{SourceId, SourceState, SourceStatus};
use crate::settings::{AutostartSettings, AutostartSettingsInvalid};

#[cfg(windows)]
mod hkcu;
pub mod startup;

/// The argument the Run entry passes; an exact match marks an autostart launch.
pub const AUTOSTART_ARG: &str = "--autostart";
/// The key (under HKCU) that holds the one Run value.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// The Run value name: the bundle identifier.
pub const VALUE_NAME: &str = "com.sebas.day-navigator";

pub const AUTOSTART_INVALID_PL: &str = "Blok „autostart” w settings.json jest nieprawidłowy: enabled to true albo false. Wpis autostartu w Windows zostaje bez zmian.";
pub const AUTOSTART_DEV_PL: &str =
    "Autostart działa tylko w zainstalowanej wersji; ta kompilacja nie zmienia wpisu w Windows.";

/// True when any given argument is exactly `--autostart`. Callers pass the
/// arguments after the program name.
pub fn started_by_autostart<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter().any(|a| a.as_ref() == AUTOSTART_ARG)
}

/// The Run value for `exe`: the quoted path, a space, `--autostart`. `None`
/// when the path is not valid UTF-8.
pub fn expected_value(exe: &Path) -> Option<String> {
    let path = exe.to_str()?;
    Some(format!("\"{path}\" {AUTOSTART_ARG}"))
}

/// The one Run value. `read` of a missing value (or key) is `Ok(None)`;
/// `remove` of a missing value is `Ok(())`.
pub trait RunEntry {
    fn read(&self) -> io::Result<Option<String>>;
    fn write(&self, value: &str) -> io::Result<()>;
    fn remove(&self) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    Nothing,
    Write,
    Remove,
}

/// What to do so the entry matches the key. `expected` is only consulted when
/// `enabled`.
pub fn sync_action(enabled: bool, current: Option<&str>, expected: Option<&str>) -> SyncAction {
    match (enabled, current) {
        (true, Some(c)) if Some(c) == expected => SyncAction::Nothing,
        (true, _) => SyncAction::Write,
        (false, Some(_)) => SyncAction::Remove,
        (false, None) => SyncAction::Nothing,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailedStep {
    Read,
    Write,
    Remove,
    ExePath,
    Thread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    On,
    Off,
    DevOnly,
    Invalid,
    Failed {
        step: FailedStep,
        kind: io::ErrorKind,
    },
}

/// Syncs the entry with the settings. With `apply == false` (a debug build) the
/// entry is never touched. An invalid block or an unreadable value changes
/// nothing.
pub fn run(
    settings: Result<AutostartSettings, AutostartSettingsInvalid>,
    apply: bool,
    entry: &dyn RunEntry,
    exe: Option<&Path>,
) -> Outcome {
    let Ok(settings) = settings else {
        return Outcome::Invalid;
    };
    let enabled = settings.enabled;
    if !apply {
        return if enabled {
            Outcome::DevOnly
        } else {
            Outcome::Off
        };
    }
    let expected = if enabled {
        let Some(exe) = exe else {
            return Outcome::Failed {
                step: FailedStep::ExePath,
                kind: io::ErrorKind::NotFound,
            };
        };
        let Some(value) = expected_value(exe) else {
            return Outcome::Failed {
                step: FailedStep::ExePath,
                kind: io::ErrorKind::InvalidData,
            };
        };
        Some(value)
    } else {
        None
    };
    let current = match entry.read() {
        Ok(c) => c,
        Err(e) => {
            return Outcome::Failed {
                step: FailedStep::Read,
                kind: e.kind(),
            };
        }
    };
    match sync_action(enabled, current.as_deref(), expected.as_deref()) {
        SyncAction::Nothing => {}
        SyncAction::Write => {
            // `Write` is only chosen when on, so `expected` is set.
            let value = expected.as_deref().unwrap_or_default();
            if let Err(e) = entry.write(value) {
                return Outcome::Failed {
                    step: FailedStep::Write,
                    kind: e.kind(),
                };
            }
        }
        SyncAction::Remove => {
            if let Err(e) = entry.remove() {
                return Outcome::Failed {
                    step: FailedStep::Remove,
                    kind: e.kind(),
                };
            }
        }
    }
    if enabled { Outcome::On } else { Outcome::Off }
}

fn step_phrase(step: FailedStep) -> &'static str {
    match step {
        FailedStep::Read => "odczytać wpisu autostartu w rejestrze Windows",
        FailedStep::Write => "zapisać wpisu autostartu w rejestrze Windows",
        FailedStep::Remove => "usunąć wpisu autostartu z rejestru Windows",
        FailedStep::ExePath => "ustalić ścieżki programu",
        FailedStep::Thread => "uruchomić sprawdzania autostartu",
    }
}

/// `AUTOSTART_FAILED_PL`: step phrase and the `io::ErrorKind` name only.
fn failed_message(step: FailedStep, kind: io::ErrorKind) -> String {
    format!(
        "Nie udało się {} ({kind:?}). Aplikacja spróbuje ponownie przy następnym uruchomieniu.",
        step_phrase(step)
    )
}

/// The "Autostart" health source for an outcome.
pub fn status(outcome: &Outcome) -> SourceStatus {
    let state = match outcome {
        Outcome::On => SourceState::Ok,
        Outcome::Off => SourceState::NotConfigured,
        Outcome::DevOnly => SourceState::Unavailable {
            message: AUTOSTART_DEV_PL.to_string(),
        },
        Outcome::Invalid => SourceState::Error {
            kind: "SettingsInvalid".to_string(),
            message: AUTOSTART_INVALID_PL.to_string(),
        },
        Outcome::Failed { step, kind } => SourceState::Error {
            kind: "AutostartFailed".to_string(),
            message: failed_message(*step, *kind),
        },
    };
    SourceStatus {
        id: SourceId::Autostart,
        label: "Autostart".to_string(),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::ffi::OsString;
    use std::path::PathBuf;

    const EXE: &str = r"C:\Users\Test User\AppData\Local\Nawigator dnia\day-navigator.exe";
    const EXPECTED: &str =
        r#""C:\Users\Test User\AppData\Local\Nawigator dnia\day-navigator.exe" --autostart"#;

    #[derive(Default)]
    struct Fake {
        value: RefCell<Option<String>>,
        read_err: Cell<Option<io::ErrorKind>>,
        write_err: Cell<Option<io::ErrorKind>>,
        remove_err: Cell<Option<io::ErrorKind>>,
        reads: Cell<u32>,
        writes: Cell<u32>,
        removes: Cell<u32>,
    }

    impl Fake {
        fn with(value: &str) -> Fake {
            let f = Fake::default();
            *f.value.borrow_mut() = Some(value.to_string());
            f
        }
        fn calls(&self) -> u32 {
            self.reads.get() + self.writes.get() + self.removes.get()
        }
    }

    impl RunEntry for Fake {
        fn read(&self) -> io::Result<Option<String>> {
            self.reads.set(self.reads.get() + 1);
            match self.read_err.get() {
                Some(k) => Err(io::Error::from(k)),
                None => Ok(self.value.borrow().clone()),
            }
        }
        fn write(&self, value: &str) -> io::Result<()> {
            self.writes.set(self.writes.get() + 1);
            match self.write_err.get() {
                Some(k) => Err(io::Error::from(k)),
                None => {
                    *self.value.borrow_mut() = Some(value.to_string());
                    Ok(())
                }
            }
        }
        fn remove(&self) -> io::Result<()> {
            self.removes.set(self.removes.get() + 1);
            match self.remove_err.get() {
                Some(k) => Err(io::Error::from(k)),
                None => {
                    *self.value.borrow_mut() = None;
                    Ok(())
                }
            }
        }
    }

    fn on() -> Result<AutostartSettings, AutostartSettingsInvalid> {
        Ok(AutostartSettings { enabled: true })
    }

    fn off() -> Result<AutostartSettings, AutostartSettingsInvalid> {
        Ok(AutostartSettings { enabled: false })
    }

    fn exe() -> PathBuf {
        PathBuf::from(EXE)
    }

    #[test]
    fn started_by_autostart_matches_exactly() {
        assert!(!started_by_autostart(Vec::<String>::new()));
        assert!(started_by_autostart(["--autostart"]));
        assert!(started_by_autostart(["--foo", "--autostart", "bar"]));
        for near in [
            "--Autostart",
            "--autostart=1",
            "-autostart",
            "autostart",
            "",
        ] {
            assert!(!started_by_autostart([near]), "{near}");
        }
        assert!(started_by_autostart([OsString::from("--autostart")]));
        assert!(started_by_autostart(vec!["--autostart".to_string()]));
        assert!(!started_by_autostart([OsString::from("--other")]));
    }

    #[test]
    fn expected_value_quotes_a_path_with_a_space() {
        assert_eq!(expected_value(&exe()).as_deref(), Some(EXPECTED));
    }

    #[test]
    fn sync_action_table() {
        let e = Some(EXPECTED);
        assert_eq!(sync_action(true, Some(EXPECTED), e), SyncAction::Nothing);
        assert_eq!(sync_action(true, None, e), SyncAction::Write);
        assert_eq!(sync_action(true, Some("other"), e), SyncAction::Write);
        assert_eq!(sync_action(false, Some("x"), None), SyncAction::Remove);
        assert_eq!(sync_action(false, Some(EXPECTED), e), SyncAction::Remove);
        assert_eq!(sync_action(false, None, None), SyncAction::Nothing);
        assert_eq!(sync_action(false, None, e), SyncAction::Nothing);
    }

    #[test]
    fn invalid_settings_make_no_entry_call() {
        let f = Fake::with("keep");
        let out = run(Err(AutostartSettingsInvalid), true, &f, Some(&exe()));
        assert_eq!(out, Outcome::Invalid);
        assert_eq!(f.calls(), 0);
        assert_eq!(f.value.borrow().as_deref(), Some("keep"));
    }

    #[test]
    fn not_applying_makes_no_entry_call() {
        let f = Fake::with("keep");
        assert_eq!(run(on(), false, &f, Some(&exe())), Outcome::DevOnly);
        assert_eq!(run(off(), false, &f, Some(&exe())), Outcome::Off);
        assert_eq!(f.calls(), 0);
        assert_eq!(f.value.borrow().as_deref(), Some("keep"));
    }

    #[test]
    fn on_without_a_usable_exe_fails_before_any_entry_call() {
        let f = Fake::default();
        assert_eq!(
            run(on(), true, &f, None),
            Outcome::Failed {
                step: FailedStep::ExePath,
                kind: io::ErrorKind::NotFound
            }
        );
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            let bad = PathBuf::from(OsString::from_wide(&[0x43, 0xD800]));
            assert_eq!(
                run(on(), true, &f, Some(&bad)),
                Outcome::Failed {
                    step: FailedStep::ExePath,
                    kind: io::ErrorKind::InvalidData
                }
            );
        }
        assert_eq!(f.calls(), 0);
    }

    #[test]
    fn on_writes_a_missing_value() {
        let f = Fake::default();
        assert_eq!(run(on(), true, &f, Some(&exe())), Outcome::On);
        assert_eq!(f.value.borrow().as_deref(), Some(EXPECTED));
        assert_eq!(f.writes.get(), 1);
    }

    #[test]
    fn on_leaves_an_equal_value_alone() {
        let f = Fake::with(EXPECTED);
        assert_eq!(run(on(), true, &f, Some(&exe())), Outcome::On);
        assert_eq!(f.writes.get(), 0);
        assert_eq!(f.removes.get(), 0);
    }

    #[test]
    fn on_overwrites_a_different_value() {
        let f = Fake::with(r#""D:\old\day-navigator.exe" --autostart"#);
        assert_eq!(run(on(), true, &f, Some(&exe())), Outcome::On);
        assert_eq!(f.value.borrow().as_deref(), Some(EXPECTED));
        assert_eq!(f.writes.get(), 1);
    }

    #[test]
    fn off_removes_a_present_value_and_ignores_a_missing_one() {
        let f = Fake::with(EXPECTED);
        assert_eq!(run(off(), true, &f, Some(&exe())), Outcome::Off);
        assert_eq!(f.value.borrow().as_deref(), None);
        assert_eq!(f.removes.get(), 1);
        let g = Fake::default();
        assert_eq!(run(off(), true, &g, None), Outcome::Off);
        assert_eq!(g.removes.get(), 0);
        assert_eq!(g.writes.get(), 0);
    }

    #[test]
    fn a_read_failure_writes_and_removes_nothing() {
        for settings in [on(), off()] {
            let f = Fake::with("keep");
            f.read_err.set(Some(io::ErrorKind::PermissionDenied));
            assert_eq!(
                run(settings, true, &f, Some(&exe())),
                Outcome::Failed {
                    step: FailedStep::Read,
                    kind: io::ErrorKind::PermissionDenied
                }
            );
            assert_eq!(f.writes.get() + f.removes.get(), 0);
            assert_eq!(f.value.borrow().as_deref(), Some("keep"));
        }
    }

    #[test]
    fn write_and_remove_failures_are_reported() {
        let f = Fake::default();
        f.write_err.set(Some(io::ErrorKind::PermissionDenied));
        assert_eq!(
            run(on(), true, &f, Some(&exe())),
            Outcome::Failed {
                step: FailedStep::Write,
                kind: io::ErrorKind::PermissionDenied
            }
        );
        let g = Fake::with("x");
        g.remove_err.set(Some(io::ErrorKind::Other));
        assert_eq!(
            run(off(), true, &g, None),
            Outcome::Failed {
                step: FailedStep::Remove,
                kind: io::ErrorKind::Other
            }
        );
    }

    #[test]
    fn status_maps_every_outcome() {
        let s = status(&Outcome::On);
        assert_eq!(s.state, SourceState::Ok);
        assert_eq!(s.label, "Autostart");
        assert_eq!(s.id, SourceId::Autostart);
        assert_eq!(status(&Outcome::Off).state, SourceState::NotConfigured);
        assert_eq!(
            status(&Outcome::DevOnly).state,
            SourceState::Unavailable {
                message: AUTOSTART_DEV_PL.to_string()
            }
        );
        assert_eq!(
            status(&Outcome::Invalid).state,
            SourceState::Error {
                kind: "SettingsInvalid".to_string(),
                message: AUTOSTART_INVALID_PL.to_string()
            }
        );
        let failed = status(&Outcome::Failed {
            step: FailedStep::Write,
            kind: io::ErrorKind::PermissionDenied,
        });
        assert_eq!(
            failed.state,
            SourceState::Error {
                kind: "AutostartFailed".to_string(),
                message: "Nie udało się zapisać wpisu autostartu w rejestrze Windows (PermissionDenied). Aplikacja spróbuje ponownie przy następnym uruchomieniu.".to_string()
            }
        );
    }

    #[test]
    fn status_messages_hold_no_path_and_serialize_with_the_key() {
        let outcomes = [
            Outcome::On,
            Outcome::Off,
            Outcome::DevOnly,
            Outcome::Invalid,
            Outcome::Failed {
                step: FailedStep::ExePath,
                kind: io::ErrorKind::NotFound,
            },
            Outcome::Failed {
                step: FailedStep::Thread,
                kind: io::ErrorKind::Other,
            },
        ];
        for o in outcomes {
            let s = status(&o);
            let json = serde_json::to_value(&s).unwrap();
            assert_eq!(json["id"], "autostart");
            let text = json.to_string();
            assert!(!text.contains("Test User"), "{o:?}");
            assert!(!text.contains("C:"), "{o:?}");
        }
    }

    #[test]
    fn value_name_is_the_bundle_identifier() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], VALUE_NAME);
    }
}
