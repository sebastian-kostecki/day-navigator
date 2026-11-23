//! Pure classification of a finished `glab` / `gh` process. This is the only
//! code that reads stderr, and the result never carries it.

use super::{CliError, Tool};

const UNAVAILABLE_MARKERS: &[&str] = &[
    "dial tcp",
    "proxyconnect",
    "i/o timeout",
    "no such host",
    "connection refused",
    "connectex",
    "error connecting to",
    "tls handshake timeout",
    "context deadline exceeded",
    "connection reset",
    "network is unreachable",
];

const LOGGED_OUT_MARKERS: &[&str] = &[
    "401 unauthorized",
    "(http 401)",
    "http 401",
    "auth login",
    "not logged in",
    "no token",
];

/// `http 5` followed by two digits: glab's `(HTTP 503)`, gh's `HTTP 502: ...`.
/// Does not match `https://`.
fn has_http_5xx(lower: &str) -> bool {
    const NEEDLE: &str = "http 5";
    lower.match_indices(NEEDLE).any(|(i, _)| {
        let rest = &lower.as_bytes()[i + NEEDLE.len()..];
        rest.len() >= 2 && rest[..2].iter().all(u8::is_ascii_digit)
    })
}

/// Order matters, the first match wins: exit 0 is data; `gh` exit 4 is logged
/// out; network or 5xx text is unavailable (before login text, because both
/// CLIs append login hints to network failures); login text is logged out;
/// anything else is `Failed`.
pub fn classify(tool: Tool, exit_code: Option<i32>, stderr: &str) -> Result<(), CliError> {
    if exit_code == Some(0) {
        return Ok(());
    }
    if tool == Tool::Gh && exit_code == Some(4) {
        return Err(CliError::LoggedOut { tool });
    }
    let lower = stderr.to_lowercase();
    if UNAVAILABLE_MARKERS.iter().any(|m| lower.contains(m)) || has_http_5xx(&lower) {
        return Err(CliError::Unavailable { tool });
    }
    if LOGGED_OUT_MARKERS.iter().any(|m| lower.contains(m)) {
        return Err(CliError::LoggedOut { tool });
    }
    Err(CliError::Failed { tool, exit_code })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GL: Tool = Tool::Glab;
    const GH: Tool = Tool::Gh;

    type Row = (
        &'static str,
        Tool,
        Option<i32>,
        &'static str,
        Result<(), CliError>,
    );

    fn unavailable(tool: Tool) -> Result<(), CliError> {
        Err(CliError::Unavailable { tool })
    }
    fn logged_out(tool: Tool) -> Result<(), CliError> {
        Err(CliError::LoggedOut { tool })
    }
    fn failed(tool: Tool, exit_code: Option<i32>) -> Result<(), CliError> {
        Err(CliError::Failed { tool, exit_code })
    }

    fn table() -> Vec<Row> {
        vec![
            (
                "glab logged out",
                GL,
                Some(1),
                r#"{"message":"401 Unauthorized"}glab: 401 Unauthorized (HTTP 401)"#,
                logged_out(GL),
            ),
            (
                "gh logged out, exit 4",
                GH,
                Some(4),
                "To get started with GitHub CLI, please run:  gh auth login",
                logged_out(GH),
            ),
            (
                "gh exit 4 with empty stderr",
                GH,
                Some(4),
                "",
                logged_out(GH),
            ),
            (
                "glab not logged in text",
                GL,
                Some(1),
                "You are not logged in to any GitLab host",
                logged_out(GL),
            ),
            (
                "glab dns failure",
                GL,
                Some(1),
                r#"Get "https://gitlab.example.invalid/api/v4/user": dial tcp: lookup gitlab.example.invalid: i/o timeout."#,
                unavailable(GL),
            ),
            (
                "glab dial timeout",
                GL,
                Some(1),
                r#"Get "https://gitlab.example.invalid/api/v4/user": dial tcp 192.0.2.1:443: i/o timeout."#,
                unavailable(GL),
            ),
            (
                "glab no such host",
                GL,
                Some(1),
                "dial tcp: lookup gitlab.example.invalid: no such host",
                unavailable(GL),
            ),
            (
                "gh no network",
                GH,
                Some(1),
                "error connecting to github.example.invalid\ncheck your internet connection or https://status.example.invalid",
                unavailable(GH),
            ),
            (
                "gh connectex refused",
                GH,
                Some(1),
                "proxyconnect tcp: dial tcp 192.0.2.1:8080: connectex: No connection could be made because the target machine actively refused it.",
                unavailable(GH),
            ),
            (
                "glab http 503",
                GL,
                Some(1),
                "glab: 503 Service Unavailable (HTTP 503)",
                unavailable(GL),
            ),
            (
                "gh http 502",
                GH,
                Some(1),
                "gh: HTTP 502: Bad Gateway (https://github.example.invalid/api/v3/user)",
                unavailable(GH),
            ),
            (
                "exit 0 with update notice is data",
                GL,
                Some(0),
                "A new version of glab has been released: v9.9.9\nhttps://gitlab.example.invalid/releases",
                Ok(()),
            ),
            (
                "exit 0 with login-looking noise is data",
                GH,
                Some(0),
                "To re-authenticate, run: gh auth login",
                Ok(()),
            ),
            (
                "unknown error exit 2",
                GL,
                Some(2),
                "something odd happened",
                failed(GL, Some(2)),
            ),
            ("no exit code, empty stderr", GH, None, "", failed(GH, None)),
            (
                "https url with 5xx-looking port and path is not http 5xx",
                GL,
                Some(1),
                "failed https://gitlab.example.invalid:503/api/v4/500 weird",
                failed(GL, Some(1)),
            ),
            (
                "glab network failure followed by login hint",
                GL,
                Some(1),
                "API call failed: Get \"https://gitlab.example.invalid/api/v4/user\": dial tcp 192.0.2.1:443: i/o timeout\nrun glab auth login --hostname gitlab.example.invalid",
                unavailable(GL),
            ),
            (
                "gh proxy failure followed by invalid token hint",
                GH,
                Some(1),
                "github.example.invalid: proxyconnect tcp: dial tcp 192.0.2.1:8080: connectex: connection refused\nThe token in keyring is invalid\nTo re-authenticate, run: gh auth login -h github.example.invalid",
                unavailable(GH),
            ),
        ]
    }

    #[test]
    fn classification_table() {
        for (name, tool, code, stderr, expected) in table() {
            assert_eq!(classify(tool, code, stderr), expected, "row: {name}");
        }
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(
            classify(GL, Some(1), "DIAL TCP: I/O TIMEOUT"),
            unavailable(GL)
        );
        assert_eq!(classify(GH, Some(1), "Not Logged In"), logged_out(GH));
    }

    #[test]
    fn no_error_representation_leaks_the_host() {
        let mut checked = 0;
        for (name, tool, code, stderr, _) in table() {
            if !stderr.contains("example.invalid") {
                continue;
            }
            let Err(err) = classify(tool, code, stderr) else {
                continue;
            };
            checked += 1;
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains("example.invalid"), "leak in row: {name}");
            }
        }
        assert!(checked > 0, "no sample exercised the no-leak check");
    }
}
