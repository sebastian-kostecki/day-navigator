//! The two small `gh` answers the hidden-issue rules need: the logged-in
//! login and one issue's comments (author and creation time only).
//!
//! Pure code. The comment body, reactions, URLs and every other field are never
//! deserialized, so no comment text can reach a value, a log or an error. A
//! serde error is dropped: it can quote the response.

use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use super::GithubError;

/// The login of the logged-in account. Debug is opaque so a login never
/// reaches a log.
#[derive(Clone, PartialEq, Eq)]
pub struct GithubLogin(String);

impl GithubLogin {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for GithubLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GithubLogin(..)")
    }
}

/// One comment: who wrote it (`None` for a deleted account or a missing
/// login) and when. Debug does not print the author.
#[derive(Clone, PartialEq, Eq)]
pub struct GithubComment {
    pub author: Option<String>,
    pub created_at: DateTime<FixedOffset>,
}

impl std::fmt::Debug for GithubComment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GithubComment")
            .field("author", &self.author.as_ref().map(|_| ".."))
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Deserialize)]
struct RawLogin {
    login: String,
}

/// Reads the output of `gh api user`: one JSON object with a `login`. An empty
/// login, one over 255 bytes or one with whitespace or control characters is
/// `ResponseInvalid`.
pub fn parse_login(stdout: &str) -> Result<GithubLogin, GithubError> {
    let raw: RawLogin = serde_json::from_str(stdout).map_err(|_| GithubError::ResponseInvalid)?;
    let bad = raw.login.is_empty()
        || raw.login.len() > 255
        || raw
            .login
            .chars()
            .any(|c| c.is_whitespace() || c.is_control());
    if bad {
        return Err(GithubError::ResponseInvalid);
    }
    Ok(GithubLogin(raw.login))
}

#[derive(Deserialize)]
struct RawComments {
    comments: Vec<RawComment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawComment {
    #[serde(default)]
    author: Option<RawAuthor>,
    created_at: String,
}

#[derive(Deserialize)]
struct RawAuthor {
    #[serde(default)]
    login: Option<String>,
}

/// Reads the output of `gh issue view --json comments`. A missing or `null`
/// author, a missing or `null` login and an empty login all give
/// `author: None`; an unreadable `createdAt` or any serde error is
/// `ResponseInvalid`.
pub fn parse_comments(stdout: &str) -> Result<Vec<GithubComment>, GithubError> {
    let raw: RawComments =
        serde_json::from_str(stdout).map_err(|_| GithubError::ResponseInvalid)?;
    let mut comments = Vec::with_capacity(raw.comments.len());
    for r in raw.comments {
        let created_at = DateTime::parse_from_rfc3339(&r.created_at)
            .map_err(|_| GithubError::ResponseInvalid)?;
        let author = r
            .author
            .and_then(|a| a.login)
            .filter(|login| !login.is_empty());
        comments.push(GithubComment { author, created_at });
    }
    Ok(comments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_is_read_from_an_object() {
        let login = parse_login(r#"{"login":"invented-me","id":1,"type":"User"}"#).unwrap();
        assert_eq!(login.as_str(), "invented-me");
        assert_eq!(format!("{login:?}"), "GithubLogin(..)");
    }

    #[test]
    fn login_refuses_bad_shapes() {
        let long = format!(r#"{{"login":"{}"}}"#, "a".repeat(256));
        let bad = [
            "",
            "[]",
            "{}",
            r#"{"login":null}"#,
            r#"{"login":""}"#,
            r#"{"login":"a b"}"#,
            r#"{"login":"a\nb"}"#,
            r#"{"login":"a\tb"}"#,
            r#"{"login":5}"#,
            long.as_str(),
        ];
        for b in bad {
            assert_eq!(parse_login(b), Err(GithubError::ResponseInvalid), "{b:?}");
        }
        let ok = format!(r#"{{"login":"{}"}}"#, "a".repeat(255));
        assert!(parse_login(&ok).is_ok());
    }

    #[test]
    fn comments_keep_author_and_time_only() {
        let out = r#"{"comments":[
            {"author":{"login":"invented-a"},"createdAt":"2026-03-10T09:00:00Z","body":"BODY","reactionGroups":[]},
            {"author":{"login":"invented-b"},"createdAt":"2026-03-11T09:00:00+01:00","url":"u"}
        ]}"#;
        let comments = parse_comments(out).unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].author.as_deref(), Some("invented-a"));
        assert_eq!(
            comments[0].created_at,
            DateTime::parse_from_rfc3339("2026-03-10T09:00:00Z").unwrap()
        );
        assert_eq!(comments[1].author.as_deref(), Some("invented-b"));
    }

    #[test]
    fn ghost_missing_null_and_empty_authors_are_none() {
        let out = r#"{"comments":[
            {"author":null,"createdAt":"2026-03-10T09:00:00Z"},
            {"createdAt":"2026-03-10T09:00:00Z"},
            {"author":{},"createdAt":"2026-03-10T09:00:00Z"},
            {"author":{"login":null},"createdAt":"2026-03-10T09:00:00Z"},
            {"author":{"login":""},"createdAt":"2026-03-10T09:00:00Z"}
        ]}"#;
        let comments = parse_comments(out).unwrap();
        assert_eq!(comments.len(), 5);
        assert!(comments.iter().all(|c| c.author.is_none()));
    }

    #[test]
    fn empty_list_is_fine() {
        assert_eq!(parse_comments(r#"{"comments":[]}"#).unwrap(), vec![]);
    }

    #[test]
    fn bad_input_is_response_invalid() {
        let bad = [
            "",
            "[]",
            "{}",
            r#"{"comments":null}"#,
            r#"{"comments":[{"author":null}]}"#,
            r#"{"comments":[{"author":null,"createdAt":"yesterday"}]}"#,
            r#"{"comments":[{"author":null,"createdAt":null}]}"#,
            r#"{"comments":[{"author":"x","createdAt":"2026-03-10T09:00:00Z"}]}"#,
            r#"{"comments":[{"author":null,"createdAt":"2026-03"#,
        ];
        for b in bad {
            assert_eq!(
                parse_comments(b),
                Err(GithubError::ResponseInvalid),
                "{b:?}"
            );
        }
    }

    #[test]
    fn debug_carries_no_author() {
        let out = r#"{"comments":[{"author":{"login":"invented-secret-name"},"createdAt":"2026-03-10T09:00:00Z"}]}"#;
        let comments = parse_comments(out).unwrap();
        assert!(!format!("{comments:?}").contains("invented-secret-name"));
    }
}
