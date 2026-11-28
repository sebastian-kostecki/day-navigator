//! The top-level `notices` block of `settings.json`: which Windows notices are
//! shown. Unlike the other blocks it fails open, because missing a notice is
//! worse than getting one.

use std::fs;
use std::path::Path;
use std::sync::Once;

use serde_json::Value;

use super::SUPPORTED_VERSION;
use super::field_error::{FieldError, NOT_TRUE_OR_FALSE, checked, object_with_keys, present};

/// Which notice kinds are on. All on unless the block says `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoticeSettings {
    pub new_assignments: bool,
    pub source_problems: bool,
    pub day_summary: bool,
}

impl Default for NoticeSettings {
    fn default() -> Self {
        NoticeSettings {
            new_assignments: true,
            source_problems: true,
            day_summary: true,
        }
    }
}

/// The Windows notices other than the source-health one, which
/// `health::filter_transitions` gates per source.
///
/// `Stall`, `TrayFailure` and `Reminder` only record the rule: their call
/// sites call `toast` directly and never ask, so no test here would notice a
/// gate added at one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toast {
    /// New GitLab assignments and review requests.
    NewAssignments,
    /// The end-of-day summary after a tray STOP.
    DaySummary,
    /// The health check stopped running.
    Stall,
    /// A tray START or STOP failed.
    TrayFailure,
    /// A work reminder; the `reminders` block turns those off.
    Reminder,
}

impl NoticeSettings {
    /// Whether the `notices` block lets this toast show. It never holds back
    /// the stall, tray-failure and reminder toasts, so their call sites do not
    /// ask.
    pub fn allows(self, toast: Toast) -> bool {
        match toast {
            Toast::NewAssignments => self.new_assignments,
            Toast::DaySummary => self.day_summary,
            Toast::Stall | Toast::TrayFailure | Toast::Reminder => true,
        }
    }
}

/// Reads only the top-level `notices` block, on its own. Shape:
///
/// ```json
/// "notices": { "newAssignments": true, "sourceProblems": true, "daySummary": true }
/// ```
///
/// Each field is `true` when absent or `null`, and so is a missing file or an
/// absent or `null` block. Any file-level or block problem keeps all notices on
/// and is logged once per process, without any detail.
pub fn load_notice_settings(path: &Path) -> NoticeSettings {
    read_notice_settings(path).unwrap_or_else(|| {
        static WARNED: Once = Once::new();
        WARNED.call_once(|| {
            log::warn!("the notices settings block cannot be read; all notices stay on");
        });
        NoticeSettings::default()
    })
}

/// `None` when something is wrong with the file or the block.
fn read_notice_settings(path: &Path) -> Option<NoticeSettings> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Some(NoticeSettings::default());
        }
        Err(_) => return None,
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).ok()?;
    let value: Value = serde_json::from_str(text).ok()?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return None;
    }
    match value.get("notices") {
        None => Some(NoticeSettings::default()),
        Some(block) => validate_notices(block).ok(),
    }
}

/// Checks a `notices` block, reporting every bad field.
pub fn validate_notices(block: &Value) -> Result<NoticeSettings, Vec<FieldError>> {
    let mut settings = NoticeSettings::default();
    if block.is_null() {
        return Ok(settings);
    }
    let map = object_with_keys(
        block,
        "notices",
        &["newAssignments", "sourceProblems", "daySummary"],
    )
    .map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    for (key, target) in [
        ("newAssignments", &mut settings.new_assignments),
        ("sourceProblems", &mut settings.source_problems),
        ("daySummary", &mut settings.day_summary),
    ] {
        match present(map, key) {
            None => {}
            Some(Value::Bool(on)) => *target = *on,
            Some(_) => errors.push(FieldError::new(format!("notices.{key}"), NOT_TRUE_OR_FALSE)),
        }
    }
    checked(settings, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str) -> NoticeSettings {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        fs::write(&p, text).unwrap();
        load_notice_settings(&p)
    }

    fn all_on() -> NoticeSettings {
        NoticeSettings {
            new_assignments: true,
            source_problems: true,
            day_summary: true,
        }
    }

    #[test]
    fn notices_default_to_all_on() {
        assert_eq!(NoticeSettings::default(), all_on());
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_notice_settings(&dir.path().join("settings.json")),
            all_on()
        );
        for text in [
            r#"{"version":1}"#,
            r#"{"version":1,"notices":null}"#,
            r#"{"version":1,"notices":{}}"#,
            r#"{"version":1,"notices":{"newAssignments":null,"sourceProblems":null,"daySummary":null}}"#,
        ] {
            assert_eq!(load(text), all_on(), "{text}");
        }
    }

    #[test]
    fn notices_false_is_respected_per_field() {
        for (field, want) in [
            (
                "newAssignments",
                NoticeSettings {
                    new_assignments: false,
                    ..all_on()
                },
            ),
            (
                "sourceProblems",
                NoticeSettings {
                    source_problems: false,
                    ..all_on()
                },
            ),
            (
                "daySummary",
                NoticeSettings {
                    day_summary: false,
                    ..all_on()
                },
            ),
        ] {
            let text = format!(r#"{{"version":1,"notices":{{"{field}":false}}}}"#);
            assert_eq!(load(&text), want, "{field}");
        }
    }

    #[test]
    fn notices_invalid_block_or_file_keeps_all_on() {
        for text in [
            // A valid `false` next to a bad value does not count.
            r#"{"version":1,"notices":{"newAssignments":false,"daySummary":"no"}}"#,
            r#"{"version":1,"notices":{"daySummary":false,"extra":1}}"#,
            r#"{"version":1,"notices":false}"#,
            r#"{"version":1,"notices":[false]}"#,
            r#"{"version":2,"notices":{"daySummary":false}}"#,
            r#"{"notices":{"daySummary":false}}"#,
            "{ not json",
        ] {
            assert_eq!(load(text), all_on(), "{text}");
        }
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be: unreadable, not "missing".
        assert_eq!(load_notice_settings(dir.path()), all_on());
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_notice_settings(&p), all_on());
    }

    const TOASTS: [Toast; 5] = [
        Toast::NewAssignments,
        Toast::DaySummary,
        Toast::Stall,
        Toast::TrayFailure,
        Toast::Reminder,
    ];

    fn held_back(settings: NoticeSettings) -> Vec<Toast> {
        TOASTS
            .into_iter()
            .filter(|toast| !settings.allows(*toast))
            .collect()
    }

    #[test]
    fn each_kind_off_holds_back_only_its_toast() {
        assert_eq!(held_back(all_on()), []);
        let off = |field: &str| load(&format!(r#"{{"version":1,"notices":{{"{field}":false}}}}"#));
        assert_eq!(held_back(off("newAssignments")), [Toast::NewAssignments]);
        assert_eq!(held_back(off("daySummary")), [Toast::DaySummary]);
        // Gated by source, not here.
        assert_eq!(held_back(off("sourceProblems")), []);
    }

    #[test]
    fn stall_tray_failure_and_reminder_toasts_are_never_held_back() {
        let all_off = NoticeSettings {
            new_assignments: false,
            source_problems: false,
            day_summary: false,
        };
        assert_eq!(
            held_back(all_off),
            [Toast::NewAssignments, Toast::DaySummary]
        );
    }

    #[test]
    fn an_absent_or_invalid_block_holds_back_nothing() {
        for text in [
            r#"{"version":1}"#,
            r#"{"version":1,"notices":{"newAssignments":false,"daySummary":"no"}}"#,
        ] {
            assert_eq!(held_back(load(text)), [], "{text}");
        }
    }

    #[test]
    fn notices_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(br#"{"version":1,"notices":{"daySummary":false}}"#);
        fs::write(&p, bytes).unwrap();
        assert!(!load_notice_settings(&p).day_summary);
    }
}
