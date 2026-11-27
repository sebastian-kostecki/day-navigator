//! The optional one-line note on a work-time entry.
//!
//! Pure rule, no store, no clock, no logging. The value is typed by the user
//! and never goes into an error, the app log or the event log.

use super::WorkTimeError;

/// Longest note in characters. Mirrored by the CHECK in migration v11 and by
/// `ENTRY_NOTE_MAX` in the UI.
pub const NOTE_MAX: usize = 200;

/// Trims the note and enforces one line and `NOTE_MAX` characters. Empty (or
/// whitespace only) becomes `""`, meaning no note.
pub fn normalize_note(raw: &str) -> Result<String, WorkTimeError> {
    let trimmed = raw.trim();
    if trimmed.contains(['\r', '\n']) {
        return Err(WorkTimeError::NoteMultiline);
    }
    if trimmed.chars().count() > NOTE_MAX {
        return Err(WorkTimeError::NoteTooLong { max: NOTE_MAX });
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_both_sides() {
        assert_eq!(
            normalize_note("  Wymyślona notatka  ").unwrap(),
            "Wymyślona notatka"
        );
    }

    #[test]
    fn blank_becomes_empty() {
        for raw in ["", "   ", "\t\n "] {
            assert_eq!(normalize_note(raw).unwrap(), "");
        }
    }

    #[test]
    fn edge_line_breaks_are_trimmed() {
        assert_eq!(normalize_note("\nabc\n").unwrap(), "abc");
    }

    #[test]
    fn limit_counts_characters() {
        assert!(normalize_note(&"ż".repeat(200)).is_ok());
        assert!(normalize_note(&"👍".repeat(200)).is_ok());
        assert!(matches!(
            normalize_note(&"ż".repeat(201)),
            Err(WorkTimeError::NoteTooLong { max: 200 })
        ));
        assert!(matches!(
            normalize_note(&"👍".repeat(201)),
            Err(WorkTimeError::NoteTooLong { max: 200 })
        ));
    }

    #[test]
    fn inner_line_breaks_are_refused() {
        for raw in ["a\nb", "a\r\nb", "a\rb"] {
            assert!(matches!(
                normalize_note(raw),
                Err(WorkTimeError::NoteMultiline)
            ));
        }
    }

    #[test]
    fn errors_never_contain_the_note() {
        let sentinel = "SENTINEL-invented-note";
        let long = format!("{sentinel}{}", "x".repeat(NOTE_MAX));
        let multi = format!("{sentinel}\nsecond");
        for raw in [long, multi] {
            let e = normalize_note(&raw).unwrap_err();
            let text = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap());
            assert!(!text.contains(sentinel));
            assert!(!e.kind().contains(sentinel));
        }
    }

    #[test]
    fn error_json_shape() {
        let long = serde_json::to_value(WorkTimeError::NoteTooLong { max: NOTE_MAX }).unwrap();
        assert_eq!(
            long,
            serde_json::json!({"kind":"NoteTooLong","detail":{"max":200}})
        );
        let multi = serde_json::to_value(WorkTimeError::NoteMultiline).unwrap();
        assert_eq!(multi, serde_json::json!({"kind":"NoteMultiline"}));
    }
}
