//! Assigned GitHub issues: the display model and parsing of `gh issue list
//! --json` output.
//!
//! Pure code: no `tauri`, no processes (those live in `fetch`). Invariant: no
//! error value, log line or message carries a repository name, a URL, a title,
//! a `serde_json` error or any other response content.

pub mod fetch;

use std::cmp::Reverse;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::cli::{CliError, GithubRepo, Tool};

/// One open issue assigned to the `gh` account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubIssue {
    /// `owner/name` as written in settings.
    pub repo: String,
    pub number: u64,
    /// `<owner/name lowercase>#<number>`: the stable key of the issue.
    pub reference: String,
    pub title: String,
    /// Only set for a link under this repository's `/issues/`; otherwise the
    /// row has no link.
    pub web_url: Option<String>,
    /// RFC 3339 text exactly as GitHub sent it.
    pub updated_at: String,
}

/// Why a repo or a round produced no list. No variant carries a value from the
/// outside.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum GithubError {
    #[error("no github repository is configured")]
    NotConfigured,
    #[error("the github settings block is invalid")]
    SettingsInvalid,
    #[error("github is unavailable (no network or server not responding)")]
    Unavailable,
    #[error("gh is logged out")]
    LoggedOut,
    #[error("gh was not found")]
    ToolMissing,
    #[error("gh failed ({})", exit_code_text(.exit_code))]
    Failed { exit_code: Option<i32> },
    #[error("the github response could not be read")]
    ResponseInvalid,
    #[error("the github issue list is incomplete")]
    Truncated,
    #[error("the app config dir cannot be resolved")]
    ConfigDirUnresolved,
    #[error("the github refresh thread is not running")]
    ThreadDown,
}

fn exit_code_text(exit_code: &Option<i32>) -> String {
    match exit_code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code".to_string(),
    }
}

impl From<CliError> for GithubError {
    fn from(e: CliError) -> Self {
        match e {
            CliError::Unavailable { .. } => GithubError::Unavailable,
            CliError::LoggedOut { .. } => GithubError::LoggedOut,
            CliError::ToolMissing { .. } => GithubError::ToolMissing,
            CliError::Failed { exit_code, .. } => GithubError::Failed { exit_code },
        }
    }
}

impl GithubError {
    /// Variant name, for logs and the snapshot.
    pub fn kind(&self) -> &'static str {
        match self {
            GithubError::NotConfigured => "NotConfigured",
            GithubError::SettingsInvalid => "SettingsInvalid",
            GithubError::Unavailable => "Unavailable",
            GithubError::LoggedOut => "LoggedOut",
            GithubError::ToolMissing => "ToolMissing",
            GithubError::Failed { .. } => "Failed",
            GithubError::ResponseInvalid => "ResponseInvalid",
            GithubError::Truncated => "Truncated",
            GithubError::ConfigDirUnresolved => "ConfigDirUnresolved",
            GithubError::ThreadDown => "ThreadDown",
        }
    }

    /// Fixed Polish description; never includes stderr, a repo name or response content.
    pub fn message_pl(&self) -> String {
        let tool = Tool::Gh;
        match self {
            GithubError::NotConfigured => {
                "GitHub nie jest skonfigurowany — dodaj \"github\": { \"repos\": [\"właściciel/nazwa\"] } do settings.json"
                    .to_string()
            }
            GithubError::SettingsInvalid => {
                "Nieprawidłowy blok \"github\" w settings.json — \"repos\" musi być listą (do 50) nazw w formie właściciel/nazwa"
                    .to_string()
            }
            GithubError::Unavailable => CliError::Unavailable { tool }.message_pl(),
            GithubError::LoggedOut => CliError::LoggedOut { tool }.message_pl(),
            GithubError::ToolMissing => CliError::ToolMissing { tool }.message_pl(),
            GithubError::Failed { exit_code } => CliError::Failed {
                tool,
                exit_code: *exit_code,
            }
            .message_pl(),
            GithubError::ResponseInvalid => {
                "Nie udało się odczytać odpowiedzi GitHuba".to_string()
            }
            GithubError::Truncated => {
                "Lista zgłoszeń jest niepełna — repozytorium ma za dużo przypisanych zgłoszeń"
                    .to_string()
            }
            GithubError::ConfigDirUnresolved => {
                "Nie można ustalić folderu konfiguracji aplikacji, więc nie ma skąd odczytać ustawień GitHuba"
                    .to_string()
            }
            GithubError::ThreadDown => "Wątek odświeżania GitHuba nie działa".to_string(),
        }
    }
}

// ---------------------------------------------------------------- parsing

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIssue {
    number: u64,
    title: String,
    url: String,
    updated_at: String,
}

/// Link kept only under `https://github.com/<owner>/<name>/issues/` of this
/// repo (case-insensitive), with no whitespace or control characters.
fn checked_url(url: &str, repo: &GithubRepo) -> Option<String> {
    let prefix = format!("https://github.com/{}/issues/", repo.as_str());
    let head = url.as_bytes().get(..prefix.len())?;
    if !head.eq_ignore_ascii_case(prefix.as_bytes()) {
        return None;
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    Some(url.to_string())
}

/// Reads the output of `gh issue list --json`: one JSON array. Any serde error
/// is `ResponseInvalid` with no content. Newest update first, ties by number.
pub fn parse_issues(repo: &GithubRepo, stdout: &str) -> Result<Vec<GithubIssue>, GithubError> {
    // The serde error is dropped on purpose: it can quote the response.
    let raw: Vec<RawIssue> =
        serde_json::from_str(stdout).map_err(|_| GithubError::ResponseInvalid)?;
    let key = repo.key();
    let mut keyed: Vec<(DateTime<FixedOffset>, GithubIssue)> = Vec::with_capacity(raw.len());
    for r in raw {
        if r.number == 0 {
            return Err(GithubError::ResponseInvalid);
        }
        let stamp = DateTime::parse_from_rfc3339(&r.updated_at)
            .map_err(|_| GithubError::ResponseInvalid)?;
        let issue = GithubIssue {
            repo: repo.as_str().to_string(),
            number: r.number,
            reference: format!("{key}#{}", r.number),
            title: r.title,
            web_url: checked_url(&r.url, repo),
            updated_at: r.updated_at,
        };
        keyed.push((stamp, issue));
    }
    keyed.sort_by_key(|a| (Reverse(a.0), a.1.number));
    Ok(keyed.into_iter().map(|(_, issue)| issue).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> GithubRepo {
        GithubRepo::parse("Octo-Test/Alpha").unwrap()
    }

    fn issue_json(number: u64, title: &str, url: &str, updated: &str) -> String {
        format!(r#"{{"number":{number},"title":"{title}","url":"{url}","updatedAt":"{updated}"}}"#)
    }

    fn url(n: u64) -> String {
        format!("https://github.com/octo-test/alpha/issues/{n}")
    }

    fn array(items: &[String]) -> String {
        format!("[{}]", items.join(","))
    }

    #[test]
    fn parses_sorts_newest_first_and_ties_by_number() {
        let json = array(&[
            issue_json(3, "Old", &url(3), "2026-01-01T00:00:00Z"),
            issue_json(9, "Tie high", &url(9), "2026-02-01T00:00:00Z"),
            issue_json(4, "Tie low", &url(4), "2026-02-01T00:00:00Z"),
            issue_json(1, "Newest", &url(1), "2026-03-01T00:00:00Z"),
        ]);
        let got = parse_issues(&repo(), &json).unwrap();
        let numbers: Vec<u64> = got.iter().map(|i| i.number).collect();
        assert_eq!(numbers, [1, 4, 9, 3]);
        assert_eq!(got[0].title, "Newest");
        assert_eq!(got[0].updated_at, "2026-03-01T00:00:00Z");
    }

    #[test]
    fn reference_is_lowercase_and_repo_keeps_the_settings_case() {
        let json = array(&[issue_json(7, "T", &url(7), "2026-01-01T00:00:00Z")]);
        let got = parse_issues(&repo(), &json).unwrap();
        assert_eq!(got[0].reference, "octo-test/alpha#7");
        assert_eq!(got[0].repo, "Octo-Test/Alpha");
    }

    #[test]
    fn link_is_kept_only_under_this_repos_issues_case_insensitively() {
        let urls = [
            ("https://github.com/Octo-Test/ALPHA/issues/1", true),
            ("https://github.com/octo-test/alpha/issues/1", true),
            ("https://github.com/octo-test/alpha/pull/1", false),
            ("https://github.com/octo-test/other/issues/1", false),
            ("https://github.com/octo-test/alpha2/issues/1", false),
            (
                "https://evil.example.invalid/octo-test/alpha/issues/1",
                false,
            ),
            ("http://github.com/octo-test/alpha/issues/1", false),
            ("https://github.com/octo-test/alpha/issues/1 x", false),
            ("https://github.com/octo-test/alpha/issues/1\\n", false),
            ("", false),
        ];
        for (u, kept) in urls {
            let json = array(&[issue_json(1, "T", u, "2026-01-01T00:00:00Z")]);
            let got = parse_issues(&repo(), &json).unwrap();
            assert_eq!(got[0].web_url.is_some(), kept, "{u:?}");
        }
    }

    #[test]
    fn empty_array_is_an_empty_list() {
        assert_eq!(parse_issues(&repo(), "[]").unwrap(), vec![]);
        assert_eq!(parse_issues(&repo(), " [ ] \n").unwrap(), vec![]);
    }

    #[test]
    fn invalid_output_is_response_invalid_and_leaks_nothing() {
        const MARKER: &str = "LEAK-MARKER-TITLE-7c1e";
        let zero = issue_json(0, MARKER, &url(0), "2026-01-01T00:00:00Z");
        let bad_stamp = issue_json(1, MARKER, &url(1), "yesterday");
        let truncated = format!(r#"[{{"number":1,"title":"{MARKER}","url":"x","updatedAt":"2026"#);
        let cases = [
            "".to_string(),
            "not json".to_string(),
            "{}".to_string(),
            format!("[{zero}]"),
            format!("[{bad_stamp}]"),
            truncated,
            // Two concatenated pages are not one array.
            "[][]".to_string(),
            format!(r#"[{{"number":"1","title":"{MARKER}","url":"x","updatedAt":"x"}}]"#),
            format!(r#"[{{"number":1,"title":"{MARKER}"}}]"#),
        ];
        for json in cases {
            let err = parse_issues(&repo(), &json).unwrap_err();
            assert_eq!(err, GithubError::ResponseInvalid, "{json}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains(MARKER), "{text}");
                assert!(!text.contains("octo"), "{text}");
            }
        }
    }

    #[test]
    fn error_serializes_as_a_tagged_union() {
        assert_eq!(
            serde_json::to_string(&GithubError::Failed { exit_code: Some(1) }).unwrap(),
            r#"{"kind":"Failed","detail":{"exitCode":1}}"#
        );
        assert_eq!(
            serde_json::to_string(&GithubError::Truncated).unwrap(),
            r#"{"kind":"Truncated"}"#
        );
    }

    #[test]
    fn cli_errors_map_to_github_errors() {
        let tool = Tool::Gh;
        assert_eq!(
            GithubError::from(CliError::Unavailable { tool }),
            GithubError::Unavailable
        );
        assert_eq!(
            GithubError::from(CliError::LoggedOut { tool }),
            GithubError::LoggedOut
        );
        assert_eq!(
            GithubError::from(CliError::ToolMissing { tool }),
            GithubError::ToolMissing
        );
        assert_eq!(
            GithubError::from(CliError::Failed {
                tool,
                exit_code: Some(1)
            }),
            GithubError::Failed { exit_code: Some(1) }
        );
    }

    #[test]
    fn messages_are_fixed_polish_text_without_repo_names() {
        let all = [
            GithubError::NotConfigured,
            GithubError::SettingsInvalid,
            GithubError::Unavailable,
            GithubError::LoggedOut,
            GithubError::ToolMissing,
            GithubError::Failed { exit_code: Some(1) },
            GithubError::Failed { exit_code: None },
            GithubError::ResponseInvalid,
            GithubError::Truncated,
            GithubError::ConfigDirUnresolved,
            GithubError::ThreadDown,
        ];
        for e in all {
            let m = e.message_pl();
            assert!(!m.is_empty());
            assert!(!m.contains("octo"), "{m}");
            assert!(!e.kind().is_empty());
        }
        assert_eq!(
            GithubError::Unavailable.message_pl(),
            "GitHub niedostępny (brak sieci lub serwer nie odpowiada)"
        );
    }
}
