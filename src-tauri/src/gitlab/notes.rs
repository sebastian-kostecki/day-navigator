//! An issue's notes (comments), reduced to the three facts the hide-and-resurface
//! rule needs: who wrote it, whether GitLab generated it, and when.
//!
//! Pure code. The note `body` is never deserialized, so comment text cannot
//! reach a value, a log or an error.

use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use super::{GitlabError, parse_pages};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueNote {
    /// `author.username`.
    pub author: String,
    /// A GitLab-generated note ("changed milestone", "mentioned in ..."), not a comment.
    pub system: bool,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Deserialize)]
struct RawAuthor {
    username: String,
}

#[derive(Deserialize)]
struct RawNote {
    author: RawAuthor,
    system: bool,
    created_at: String,
}

/// Notes in the order GitLab sent them (newest first). Multi-page output is read
/// in full. Any serde or timestamp error is `ResponseInvalid` with no content.
pub fn parse_notes(json: &str) -> Result<Vec<IssueNote>, GitlabError> {
    let raw: Vec<RawNote> = parse_pages(json)?;
    raw.into_iter()
        .map(|r| {
            let created_at = DateTime::parse_from_rfc3339(&r.created_at)
                .map_err(|_| GitlabError::ResponseInvalid)?;
            Ok(IssueNote {
                author: r.author.username,
                system: r.system,
                created_at,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(author: &str, system: bool, created: &str) -> String {
        format!(
            r#"{{"id":5,"body":"invented text","author":{{"id":3,"username":"{author}","name":"Invented"}},"system":{system},"created_at":"{created}","updated_at":"{created}"}}"#
        )
    }

    #[test]
    fn parses_author_system_flag_and_time_and_ignores_the_body() {
        let json = format!(
            "[{},{}]",
            note("invented.other", false, "2026-03-04T10:00:00.000Z"),
            note("invented.user", true, "2026-03-03T09:00:00+01:00")
        );
        let notes = parse_notes(&json).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].author, "invented.other");
        assert!(!notes[0].system);
        assert_eq!(
            notes[0].created_at,
            DateTime::parse_from_rfc3339("2026-03-04T10:00:00Z").unwrap()
        );
        assert!(notes[1].system);
        assert_eq!(
            notes[1].created_at,
            DateTime::parse_from_rfc3339("2026-03-03T08:00:00Z").unwrap()
        );
    }

    #[test]
    fn a_note_without_a_body_field_is_fine() {
        let json =
            r#"[{"author":{"username":"a"},"system":false,"created_at":"2026-03-04T10:00:00Z"}]"#;
        assert_eq!(parse_notes(json).unwrap().len(), 1);
        assert!(parse_notes("[]").unwrap().is_empty());
    }

    #[test]
    fn concatenated_pages_yield_all_notes() {
        let json = format!(
            "[{}]\n[{}][]",
            note("a", false, "2026-03-04T10:00:00Z"),
            note("b", false, "2026-03-01T10:00:00Z")
        );
        let notes = parse_notes(&json).unwrap();
        assert_eq!(
            notes.iter().map(|n| n.author.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn invalid_shapes_are_response_invalid() {
        let ok = note("a", false, "2026-03-04T10:00:00Z");
        let cases = [
            String::new(),
            "{}".to_string(),
            "null".to_string(),
            "[1]".to_string(),
            "[{}]".to_string(),
            format!("[{ok}"),
            format!("[{ok}] trailing"),
            // missing author, system, created_at
            r#"[{"system":false,"created_at":"2026-03-04T10:00:00Z"}]"#.to_string(),
            r#"[{"author":{"username":"a"},"created_at":"2026-03-04T10:00:00Z"}]"#.to_string(),
            r#"[{"author":{"username":"a"},"system":false}]"#.to_string(),
            // wrong types
            r#"[{"author":"a","system":false,"created_at":"2026-03-04T10:00:00Z"}]"#.to_string(),
            r#"[{"author":{"username":"a"},"system":"no","created_at":"2026-03-04T10:00:00Z"}]"#
                .to_string(),
            // bad timestamps
            note("a", false, "yesterday"),
            format!("[{}]", note("a", false, "2026-03-04")),
        ];
        for c in &cases {
            assert_eq!(parse_notes(c), Err(GitlabError::ResponseInvalid), "{c}");
        }
    }

    #[test]
    fn malformed_response_content_never_leaks() {
        const MARKER: &str = "LEAK-MARKER-NOTE-4b2d";
        let bodies = [
            // truncated array containing the marker
            format!(
                r#"[{{"body":"{MARKER}","author":{{"username":"{MARKER}"}},"system":false,"created_at":"2026-03"#
            ),
            // wrong type: serde would quote the string
            format!(
                r#"[{{"author":{{"username":"a"}},"system":"{MARKER}","created_at":"2026-03-04T10:00:00Z"}}]"#
            ),
            // bad timestamp holding the marker
            format!(r#"[{{"author":{{"username":"a"}},"system":false,"created_at":"{MARKER}"}}]"#),
            format!(r#"{{"{MARKER}":1}}"#),
            format!(r#""{MARKER}""#),
        ];
        for body in &bodies {
            let err = parse_notes(body).unwrap_err();
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains(MARKER), "leak: {text}");
                assert!(!text.contains("example.invalid"), "leak: {text}");
            }
        }
    }
}
