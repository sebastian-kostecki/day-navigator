//! Assigned GitLab issues and merge requests: the display model, parsing of
//! `glab api` output, and the rule for what the view shows after each round.
//!
//! Pure code: no `tauri`, no processes (those live in `fetch`). Invariant: no
//! error value, log line or message carries a hostname, a URL, a title, a
//! `serde_json` error or any other response content.

pub mod fetch;

use std::cmp::Reverse;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::cli::{CliError, GitlabHost, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Issue,
    MergeRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabItem {
    pub kind: ItemKind,
    /// `references.full`: `group/project#12` or `group/project!34`.
    pub reference: String,
    pub title: String,
    /// Only set for a link on the configured host; otherwise the row has no link.
    pub web_url: Option<String>,
    /// RFC 3339 text exactly as GitLab sent it.
    pub updated_at: String,
    pub labels: Vec<String>,
    pub draft: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignedItems {
    pub merge_requests: Vec<GitlabItem>,
    pub issues: Vec<GitlabItem>,
}

/// Why a round produced no list. No variant carries a value from the outside.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum GitlabError {
    #[error("gitlab host is not configured")]
    NotConfigured,
    #[error("the gitlab settings block is invalid")]
    SettingsInvalid,
    #[error("gitlab is unavailable (no network or server not responding)")]
    Unavailable,
    #[error("glab is logged out")]
    LoggedOut,
    #[error("glab was not found")]
    ToolMissing,
    #[error("glab failed ({})", exit_code_text(.exit_code))]
    Failed { exit_code: Option<i32> },
    #[error("the gitlab response could not be read")]
    ResponseInvalid,
}

fn exit_code_text(exit_code: &Option<i32>) -> String {
    match exit_code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code".to_string(),
    }
}

impl From<CliError> for GitlabError {
    fn from(e: CliError) -> Self {
        match e {
            CliError::Unavailable { .. } => GitlabError::Unavailable,
            CliError::LoggedOut { .. } => GitlabError::LoggedOut,
            CliError::ToolMissing { .. } => GitlabError::ToolMissing,
            CliError::Failed { exit_code, .. } => GitlabError::Failed { exit_code },
        }
    }
}

impl GitlabError {
    /// Variant name, for logs and the snapshot.
    pub fn kind(&self) -> &'static str {
        match self {
            GitlabError::NotConfigured => "NotConfigured",
            GitlabError::SettingsInvalid => "SettingsInvalid",
            GitlabError::Unavailable => "Unavailable",
            GitlabError::LoggedOut => "LoggedOut",
            GitlabError::ToolMissing => "ToolMissing",
            GitlabError::Failed { .. } => "Failed",
            GitlabError::ResponseInvalid => "ResponseInvalid",
        }
    }

    /// Fixed Polish description; never includes stderr, the host or response content.
    pub fn message_pl(&self) -> String {
        let tool = Tool::Glab;
        match self {
            GitlabError::NotConfigured => {
                "GitLab nie jest skonfigurowany — dodaj \"gitlab\": { \"host\": \"…\" } do settings.json"
                    .to_string()
            }
            GitlabError::SettingsInvalid => {
                "Nieprawidłowy blok \"gitlab\" w settings.json — \"host\" musi być samą nazwą hosta, np. gitlab.example.com"
                    .to_string()
            }
            GitlabError::Unavailable => CliError::Unavailable { tool }.message_pl(),
            GitlabError::LoggedOut => CliError::LoggedOut { tool }.message_pl(),
            GitlabError::ToolMissing => CliError::ToolMissing { tool }.message_pl(),
            GitlabError::Failed { exit_code } => CliError::Failed {
                tool,
                exit_code: *exit_code,
            }
            .message_pl(),
            GitlabError::ResponseInvalid => {
                "Nie udało się odczytać odpowiedzi GitLaba".to_string()
            }
        }
    }
}

// ---------------------------------------------------------------- parsing

#[derive(Deserialize)]
struct RawRefs {
    full: String,
}

#[derive(Deserialize)]
struct RawItem {
    title: String,
    web_url: String,
    updated_at: String,
    #[serde(default)]
    labels: Vec<String>,
    references: RawRefs,
    #[serde(default)]
    draft: bool,
}

/// Link kept only on `https://<host>/`, with no whitespace or control characters.
fn checked_url(url: &str, host: &GitlabHost) -> Option<String> {
    let prefix = format!("https://{}/", host.as_str());
    let head = url.as_bytes().get(..prefix.len())?;
    if !head.eq_ignore_ascii_case(prefix.as_bytes()) {
        return None;
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    Some(url.to_string())
}

fn parse_items(
    json: &str,
    host: &GitlabHost,
    kind: ItemKind,
) -> Result<Vec<GitlabItem>, GitlabError> {
    // The serde error is dropped on purpose: it can quote the response.
    let raw: Vec<RawItem> = serde_json::from_str(json).map_err(|_| GitlabError::ResponseInvalid)?;
    let mut keyed: Vec<(DateTime<FixedOffset>, GitlabItem)> = Vec::with_capacity(raw.len());
    for r in raw {
        let stamp = DateTime::parse_from_rfc3339(&r.updated_at)
            .map_err(|_| GitlabError::ResponseInvalid)?;
        let item = GitlabItem {
            kind,
            reference: r.references.full,
            title: r.title,
            web_url: checked_url(&r.web_url, host),
            updated_at: r.updated_at,
            labels: r.labels,
            draft: kind == ItemKind::MergeRequest && r.draft,
        };
        keyed.push((stamp, item));
    }
    keyed.sort_by(|a, b| (Reverse(a.0), &a.1.reference).cmp(&(Reverse(b.0), &b.1.reference)));
    Ok(keyed.into_iter().map(|(_, item)| item).collect())
}

/// Issues, newest update first (ties by reference).
pub fn parse_issues(json: &str, host: &GitlabHost) -> Result<Vec<GitlabItem>, GitlabError> {
    parse_items(json, host, ItemKind::Issue)
}

/// Merge requests, newest update first (ties by reference).
pub fn parse_merge_requests(json: &str, host: &GitlabHost) -> Result<Vec<GitlabItem>, GitlabError> {
    parse_items(json, host, ItemKind::MergeRequest)
}

// --------------------------------------------------------------- snapshot

/// What the view should say about the source. The host is not part of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GitlabState {
    Starting,
    Ok,
    NotConfigured,
    Unavailable { message: String },
    Error { kind: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabSnapshot {
    pub state: GitlabState,
    /// Last successful list in this session (memory only).
    pub items: Option<AssignedItems>,
    pub last_success_ms: Option<u64>,
    pub checked_at_ms: Option<u64>,
    pub refreshing: bool,
    pub seq: u64,
}

impl GitlabSnapshot {
    /// Before the first round has finished.
    pub fn starting() -> Self {
        GitlabSnapshot {
            state: GitlabState::Starting,
            items: None,
            last_success_ms: None,
            checked_at_ms: None,
            refreshing: false,
            seq: 0,
        }
    }
}

/// The next snapshot after one round, and the host the kept items now belong to.
///
/// `kept_host` is the host the previous snapshot's items came from; `host` is
/// this round's host (`None` when not configured or invalid). Success replaces
/// the items. `Unavailable` and the error kinds keep the items and
/// `last_success_ms` only while the host is unchanged; `NotConfigured`,
/// `SettingsInvalid` and a changed host drop them. `seq` always increases.
pub fn next_snapshot(
    prev: &GitlabSnapshot,
    kept_host: Option<&GitlabHost>,
    host: Option<&GitlabHost>,
    outcome: Result<AssignedItems, GitlabError>,
    now_ms: u64,
) -> (GitlabSnapshot, Option<GitlabHost>) {
    let seq = prev.seq + 1;
    let mut next = GitlabSnapshot {
        state: GitlabState::Ok,
        items: None,
        last_success_ms: None,
        checked_at_ms: Some(now_ms),
        refreshing: false,
        seq,
    };
    let items = match outcome {
        Ok(items) => {
            next.items = Some(items);
            next.last_success_ms = Some(now_ms);
            return (next, host.cloned());
        }
        Err(e) => e,
    };
    let same_host = host.is_some() && host == kept_host;
    let keeps = same_host
        && !matches!(
            items,
            GitlabError::NotConfigured | GitlabError::SettingsInvalid
        );
    next.state = match &items {
        GitlabError::NotConfigured => GitlabState::NotConfigured,
        GitlabError::Unavailable => GitlabState::Unavailable {
            message: items.message_pl(),
        },
        other => GitlabState::Error {
            kind: other.kind().to_string(),
            message: other.message_pl(),
        },
    };
    if keeps {
        next.items = prev.items.clone();
        next.last_success_ms = prev.last_success_ms;
        (next, kept_host.cloned())
    } else {
        (next, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> GitlabHost {
        GitlabHost::parse("gitlab.example.invalid").unwrap()
    }

    fn mr_json(title: &str, url: &str, updated: &str, reference: &str, extra: &str) -> String {
        format!(
            r#"{{"id":1,"iid":1,"title":"{title}","web_url":"{url}","updated_at":"{updated}","references":{{"short":"!1","relative":"!1","full":"{reference}"}},"state":"opened","author":{{"name":"Invented"}}{extra}}}"#
        )
    }

    fn url(path: &str) -> String {
        format!("https://gitlab.example.invalid/{path}")
    }

    #[test]
    fn parses_merge_requests_with_draft_labels_and_reference() {
        let json = format!(
            "[{}]",
            mr_json(
                "Add widget parser",
                &url("acme/widgets/-/merge_requests/34"),
                "2026-03-02T10:00:00.000Z",
                "acme/widgets!34",
                r#","draft":true,"labels":["backend","review"]"#
            )
        );
        let items = parse_merge_requests(&json, &host()).unwrap();
        assert_eq!(items.len(), 1);
        let i = &items[0];
        assert_eq!(i.kind, ItemKind::MergeRequest);
        assert_eq!(i.reference, "acme/widgets!34");
        assert_eq!(i.title, "Add widget parser");
        assert_eq!(
            i.web_url.as_deref(),
            Some("https://gitlab.example.invalid/acme/widgets/-/merge_requests/34")
        );
        assert_eq!(i.updated_at, "2026-03-02T10:00:00.000Z");
        assert_eq!(i.labels, ["backend", "review"]);
        assert!(i.draft);
    }

    #[test]
    fn draft_and_labels_default_and_issues_are_never_draft() {
        let json = format!(
            "[{}]",
            mr_json(
                "Crash on start",
                &url("acme/widgets/-/issues/12"),
                "2026-03-02T10:00:00Z",
                "acme/widgets#12",
                r#","draft":true"#
            )
        );
        let issues = parse_issues(&json, &host()).unwrap();
        assert_eq!(issues[0].kind, ItemKind::Issue);
        assert_eq!(issues[0].reference, "acme/widgets#12");
        assert!(!issues[0].draft);
        assert!(issues[0].labels.is_empty());

        let json = format!(
            "[{}]",
            mr_json(
                "No draft field",
                &url("a/b/-/merge_requests/1"),
                "2026-03-02T10:00:00Z",
                "a/b!1",
                ""
            )
        );
        assert!(!parse_merge_requests(&json, &host()).unwrap()[0].draft);
    }

    #[test]
    fn extra_fields_are_ignored_and_empty_array_is_empty() {
        assert!(parse_issues("[]", &host()).unwrap().is_empty());
        assert!(parse_merge_requests("  [ ]\n", &host()).unwrap().is_empty());
    }

    #[test]
    fn sorts_newest_first_by_instant_then_reference() {
        let a = mr_json(
            "A",
            &url("g/p/-/issues/3"),
            "2026-03-02T10:00:00Z",
            "g/p#3",
            "",
        );
        // Same instant as A, written with an offset.
        let b = mr_json(
            "B",
            &url("g/p/-/issues/1"),
            "2026-03-02T11:00:00+01:00",
            "g/p#1",
            "",
        );
        let c = mr_json(
            "C",
            &url("g/p/-/issues/2"),
            "2026-03-03T00:00:00Z",
            "g/p#2",
            "",
        );
        // Older text-wise larger string but earlier instant than A.
        let d = mr_json(
            "D",
            &url("g/p/-/issues/9"),
            "2026-03-02T23:00:00+14:00",
            "g/p#9",
            "",
        );
        let json = format!("[{a},{b},{c},{d}]");
        let refs: Vec<String> = parse_issues(&json, &host())
            .unwrap()
            .into_iter()
            .map(|i| i.reference)
            .collect();
        assert_eq!(refs, ["g/p#2", "g/p#1", "g/p#3", "g/p#9"]);
    }

    #[test]
    fn url_check_keeps_only_links_on_the_configured_host() {
        let cases: [(&str, bool); 8] = [
            ("https://gitlab.example.invalid/a/b/-/issues/1", true),
            ("HTTPS://GitLab.Example.Invalid/a/b/-/issues/1", true),
            ("https://other.example.invalid/a/b/-/issues/1", false),
            ("http://gitlab.example.invalid/a/b/-/issues/1", false),
            ("https://gitlab.example.invalid.evil.invalid/a/b", false),
            ("https://gitlab.example.invalid@evil.invalid/a/b", false),
            ("https://gitlab.example.invalid/a b", false),
            ("https://gitlab.example.invalid/a\\u0007b", false),
        ];
        for (u, kept) in cases {
            let json = format!("[{}]", mr_json("T", u, "2026-03-02T10:00:00Z", "g/p#1", ""));
            let got = parse_issues(&json, &host()).unwrap();
            assert_eq!(got[0].web_url.is_some(), kept, "{u:?}");
        }
        // Too short and non-ASCII heads must not panic.
        for u in ["", "https://", "ünïcödé-ünïcödé-ünïcödé-ünïcödé"] {
            let json = format!("[{}]", mr_json("T", u, "2026-03-02T10:00:00Z", "g/p#1", ""));
            assert!(parse_issues(&json, &host()).unwrap()[0].web_url.is_none());
        }
    }

    #[test]
    fn invalid_shapes_are_response_invalid() {
        let ok = mr_json("T", &url("a/b"), "2026-03-02T10:00:00Z", "g/p#1", "");
        let cases = [
            "".to_string(),
            "{}".to_string(),
            "null".to_string(),
            r#"{"items":[]}"#.to_string(),
            "[1]".to_string(),
            "[{}]".to_string(),
            format!("[{ok}"),
            // missing title
            r#"[{"web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03-02T10:00:00Z","references":{"full":"g/p#1"}}]"#.to_string(),
            // missing references
            r#"[{"title":"T","web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03-02T10:00:00Z"}]"#.to_string(),
            // missing full
            r#"[{"title":"T","web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03-02T10:00:00Z","references":{}}]"#.to_string(),
            // wrong type: title
            r#"[{"title":5,"web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03-02T10:00:00Z","references":{"full":"g/p#1"}}]"#.to_string(),
            // wrong type: labels
            format!(
                "[{}]",
                mr_json("T", &url("a"), "2026-03-02T10:00:00Z", "g/p#1", r#","labels":"x""#)
            ),
            // wrong type: draft
            format!(
                "[{}]",
                mr_json("T", &url("a"), "2026-03-02T10:00:00Z", "g/p#1", r#","draft":"yes""#)
            ),
            // bad timestamps
            format!("[{}]", mr_json("T", &url("a"), "yesterday", "g/p#1", "")),
            format!("[{}]", mr_json("T", &url("a"), "2026-03-02", "g/p#1", "")),
        ];
        for c in &cases {
            assert_eq!(
                parse_issues(c, &host()),
                Err(GitlabError::ResponseInvalid),
                "{c}"
            );
            assert_eq!(
                parse_merge_requests(c, &host()),
                Err(GitlabError::ResponseInvalid),
                "{c}"
            );
        }
    }

    #[test]
    fn malformed_response_content_never_leaks() {
        const MARKER: &str = "LEAK-MARKER-9f3a";
        let bodies = [
            // truncated array containing the marker
            format!(
                r#"[{{"title":"{MARKER}","web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03"#
            ),
            // wrong type: serde would quote the string
            format!(
                r#"[{{"title":"T","web_url":"u","updated_at":"{MARKER}","references":{{"full":"g/p#1"}}}}]"#
            ),
            format!(r#"{{"{MARKER}":1}}"#),
            format!(
                r#"[{{"title":"T","web_url":"u","updated_at":"2026-03-02T10:00:00Z","references":{{"full":"g/p#1"}},"draft":"{MARKER}"}}]"#
            ),
            format!(r#""{MARKER}""#),
        ];
        for body in &bodies {
            let err = parse_merge_requests(body, &host()).unwrap_err();
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

    #[test]
    fn error_kinds_serialization_and_cli_conversion() {
        assert_eq!(
            serde_json::to_string(&GitlabError::Failed { exit_code: Some(2) }).unwrap(),
            r#"{"kind":"Failed","detail":{"exitCode":2}}"#
        );
        assert_eq!(
            serde_json::to_string(&GitlabError::ResponseInvalid).unwrap(),
            r#"{"kind":"ResponseInvalid"}"#
        );
        let tool = Tool::Glab;
        assert_eq!(
            GitlabError::from(CliError::Unavailable { tool }),
            GitlabError::Unavailable
        );
        assert_eq!(
            GitlabError::from(CliError::LoggedOut { tool }),
            GitlabError::LoggedOut
        );
        assert_eq!(
            GitlabError::from(CliError::ToolMissing { tool }),
            GitlabError::ToolMissing
        );
        assert_eq!(
            GitlabError::from(CliError::Failed {
                tool,
                exit_code: Some(3)
            }),
            GitlabError::Failed { exit_code: Some(3) }
        );
        assert_eq!(GitlabError::LoggedOut.kind(), "LoggedOut");
        assert_eq!(
            GitlabError::Unavailable.message_pl(),
            CliError::Unavailable { tool }.message_pl()
        );
    }

    // ----- snapshot rule

    fn item(reference: &str) -> GitlabItem {
        GitlabItem {
            kind: ItemKind::Issue,
            reference: reference.to_string(),
            title: "T".into(),
            web_url: None,
            updated_at: "2026-03-02T10:00:00Z".into(),
            labels: vec![],
            draft: false,
        }
    }

    fn items(reference: &str) -> AssignedItems {
        AssignedItems {
            merge_requests: vec![],
            issues: vec![item(reference)],
        }
    }

    fn other_host() -> GitlabHost {
        GitlabHost::parse("other.example.invalid").unwrap()
    }

    #[test]
    fn snapshot_first_success_sets_items_and_time() {
        let prev = GitlabSnapshot::starting();
        let (s, kept) = next_snapshot(&prev, None, Some(&host()), Ok(items("g/p#1")), 1000);
        assert_eq!(s.state, GitlabState::Ok);
        assert_eq!(s.items, Some(items("g/p#1")));
        assert_eq!(s.last_success_ms, Some(1000));
        assert_eq!(s.checked_at_ms, Some(1000));
        assert!(!s.refreshing);
        assert_eq!(s.seq, 1);
        assert_eq!(kept, Some(host()));
    }

    #[test]
    fn snapshot_failure_after_success_keeps_items_and_last_success() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&host()),
            Ok(items("g/p#1")),
            1000,
        );
        for (err, label) in [
            (GitlabError::LoggedOut, "LoggedOut"),
            (GitlabError::ToolMissing, "ToolMissing"),
            (GitlabError::Failed { exit_code: Some(1) }, "Failed"),
            (GitlabError::ResponseInvalid, "ResponseInvalid"),
        ] {
            let (s, k) = next_snapshot(&ok, kept.as_ref(), Some(&host()), Err(err.clone()), 2000);
            assert_eq!(
                s.state,
                GitlabState::Error {
                    kind: label.to_string(),
                    message: err.message_pl()
                }
            );
            assert_eq!(s.items, Some(items("g/p#1")), "{label}");
            assert_eq!(s.last_success_ms, Some(1000), "{label}");
            assert_eq!(s.checked_at_ms, Some(2000));
            assert_eq!(k, Some(host()));
        }
    }

    #[test]
    fn snapshot_unavailable_keeps_items() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&host()),
            Ok(items("g/p#1")),
            1000,
        );
        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&host()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert_eq!(
            s.state,
            GitlabState::Unavailable {
                message: GitlabError::Unavailable.message_pl()
            }
        );
        assert_eq!(s.items, Some(items("g/p#1")));
        assert_eq!(s.last_success_ms, Some(1000));
        assert_eq!(k, Some(host()));
    }

    #[test]
    fn snapshot_failure_without_prior_success_has_no_items() {
        let (s, k) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&host()),
            Err(GitlabError::Unavailable),
            500,
        );
        assert!(s.items.is_none() && s.last_success_ms.is_none());
        assert_eq!(s.checked_at_ms, Some(500));
        assert!(k.is_none());
    }

    #[test]
    fn snapshot_not_configured_and_invalid_settings_drop_items() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&host()),
            Ok(items("g/p#1")),
            1000,
        );
        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            None,
            Err(GitlabError::NotConfigured),
            2000,
        );
        assert_eq!(s.state, GitlabState::NotConfigured);
        assert!(s.items.is_none() && s.last_success_ms.is_none());
        assert!(k.is_none());

        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            None,
            Err(GitlabError::SettingsInvalid),
            2000,
        );
        assert!(matches!(
            s.state,
            GitlabState::Error { ref kind, .. } if kind == "SettingsInvalid"
        ));
        assert!(s.items.is_none() && s.last_success_ms.is_none());
        assert!(k.is_none());
    }

    #[test]
    fn snapshot_host_change_drops_items_on_failure_and_replaces_on_success() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&host()),
            Ok(items("g/p#1")),
            1000,
        );
        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&other_host()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert!(matches!(s.state, GitlabState::Unavailable { .. }));
        assert!(s.items.is_none() && s.last_success_ms.is_none());
        assert!(k.is_none());

        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&other_host()),
            Ok(items("g/p#2")),
            3000,
        );
        assert_eq!(s.items, Some(items("g/p#2")));
        assert_eq!(s.last_success_ms, Some(3000));
        assert_eq!(k, Some(other_host()));
    }

    #[test]
    fn snapshot_seq_always_increases() {
        let mut prev = GitlabSnapshot::starting();
        let mut kept = None;
        let outcomes: [Result<AssignedItems, GitlabError>; 4] = [
            Ok(items("g/p#1")),
            Err(GitlabError::Unavailable),
            Err(GitlabError::NotConfigured),
            Ok(AssignedItems::default()),
        ];
        let mut last = prev.seq;
        for (n, outcome) in outcomes.into_iter().enumerate() {
            let (s, k) = next_snapshot(&prev, kept.as_ref(), Some(&host()), outcome, n as u64);
            assert!(s.seq > last);
            last = s.seq;
            prev = s;
            kept = k;
        }
    }

    #[test]
    fn snapshot_serializes_for_the_webview() {
        let mut s = GitlabSnapshot::starting();
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"state":{"state":"starting"},"items":null,"lastSuccessMs":null,"checkedAtMs":null,"refreshing":false,"seq":0}"#
        );
        s.state = GitlabState::Unavailable {
            message: "m".into(),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["state"]["state"], "unavailable");
        assert_eq!(v["state"]["message"], "m");
        s.state = GitlabState::NotConfigured;
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["state"]["state"], "notConfigured");
    }
}
