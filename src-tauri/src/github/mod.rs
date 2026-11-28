//! Assigned GitHub issues: the display model and parsing of `gh issue list
//! --json` output.
//!
//! Pure code: no `tauri`, no processes (those live in `fetch`). Invariant: no
//! error value, log line or message carries a repository name, a URL, a title,
//! a `serde_json` error or any other response content.

pub mod comments;
pub mod fetch;

use std::cmp::Reverse;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::cli::{CliError, GithubConfig, GithubRepo, Tool};
use crate::hidden_issues::ChangeReason;
use crate::settings::DEFAULT_REFRESH_MINUTES;
use fetch::RepoResult;

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
    /// Label names in the order GitHub sent them; empty when there are none.
    pub labels: Vec<String>,
    /// Why a hidden issue came back; set only on a marked, visible issue.
    pub changed: Option<ChangeReason>,
    /// When the issue was hidden (UTC epoch ms); set only on a hidden issue.
    pub hidden_at_ms: Option<i64>,
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
    #[serde(default)]
    labels: Vec<RawLabel>,
}

/// One label object of `gh`; its other fields are ignored.
#[derive(Deserialize)]
struct RawLabel {
    name: String,
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
            labels: r.labels.into_iter().map(|l| l.name).collect(),
            changed: None,
            hidden_at_ms: None,
        };
        keyed.push((stamp, issue));
    }
    keyed.sort_by_key(|a| (Reverse(a.0), a.1.number));
    Ok(keyed.into_iter().map(|(_, issue)| issue).collect())
}

// --------------------------------------------------------------- snapshot

/// What the view should say about the source. No repo name is part of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GithubState {
    Starting,
    Ok,
    NotConfigured,
    Unavailable { message: String },
    Error { kind: String, message: String },
}

/// Why one repository has no list. Fixed text only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoErrorView {
    pub kind: String,
    pub message: String,
}

/// One repository of the last successful round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoView {
    /// `owner/name` as written in settings.
    pub repo: String,
    /// The visible issues (a marked one carries `changed`).
    pub issues: Vec<GithubIssue>,
    /// The repo's hidden issues in list order; empty in a fetched,
    /// unpartitioned view.
    pub hidden: Vec<GithubIssue>,
    pub error: Option<RepoErrorView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSnapshot {
    pub state: GithubState,
    /// Last successful round in this session (memory only), in settings order.
    pub repos: Vec<RepoView>,
    pub last_success_ms: Option<u64>,
    pub checked_at_ms: Option<u64>,
    pub refreshing: bool,
    pub seq: u64,
    /// Hidden issues whose comment check failed this round.
    pub hidden_check_failed: u32,
    /// Fixed Polish text when the hide records could not be read or written;
    /// the repos then show every issue.
    pub hidden_error: Option<String>,
    /// True when `githubRefresh.minutes` is invalid and the default interval is
    /// used. A round flag: set on every round.
    pub refresh_setting_invalid: bool,
    /// The interval (minutes) the last round used. A round flag.
    pub refresh_minutes: u32,
    /// When the wait after the last round ends (UTC ms); `None` before the first
    /// round.
    pub next_refresh_at_ms: Option<u64>,
    /// References of visible issues that are new since the last visit to the
    /// GitHub view, in list order; never a hidden issue. Set by the refresh
    /// state on every path that shapes the list.
    pub new_references: Vec<String>,
}

impl GithubSnapshot {
    /// Before the first round has finished.
    pub fn starting() -> Self {
        GithubSnapshot {
            state: GithubState::Starting,
            repos: Vec::new(),
            last_success_ms: None,
            checked_at_ms: None,
            refreshing: false,
            seq: 0,
            hidden_check_failed: 0,
            hidden_error: None,
            refresh_setting_invalid: false,
            refresh_minutes: DEFAULT_REFRESH_MINUTES,
            next_refresh_at_ms: None,
            new_references: Vec::new(),
        }
    }
}

pub fn repo_view(result: RepoResult) -> RepoView {
    let (issues, error) = match result.outcome {
        Ok(issues) => (issues, None),
        Err(e) => (
            Vec::new(),
            Some(RepoErrorView {
                kind: e.kind().to_string(),
                message: e.message_pl(),
            }),
        ),
    };
    RepoView {
        repo: result.repo.as_str().to_string(),
        issues,
        hidden: Vec::new(),
        error,
    }
}

/// The next snapshot after one round, and the config the kept repos now belong to.
///
/// `kept` is the config the previous snapshot's repos came from; `config` is
/// this round's config (`None` when not configured or invalid). A successful
/// round, even with per-repo errors, replaces `repos` and `last_success_ms`.
/// `Unavailable` and the error kinds keep them only while the repo list is
/// unchanged; `NotConfigured`, `SettingsInvalid` and a changed list drop them.
/// `seq` always increases.
pub fn next_snapshot(
    prev: &GithubSnapshot,
    kept: Option<&GithubConfig>,
    config: Option<&GithubConfig>,
    outcome: Result<Vec<RepoResult>, GithubError>,
    now_ms: u64,
) -> (GithubSnapshot, Option<GithubConfig>) {
    let mut next = GithubSnapshot {
        state: GithubState::Ok,
        repos: Vec::new(),
        last_success_ms: None,
        checked_at_ms: Some(now_ms),
        refreshing: false,
        seq: prev.seq + 1,
        hidden_check_failed: 0,
        hidden_error: None,
        // Round flags, carried over: the round sets them.
        refresh_setting_invalid: prev.refresh_setting_invalid,
        refresh_minutes: prev.refresh_minutes,
        next_refresh_at_ms: prev.next_refresh_at_ms,
        new_references: Vec::new(),
    };
    let error = match outcome {
        Ok(results) => {
            next.repos = results.into_iter().map(repo_view).collect();
            next.last_success_ms = Some(now_ms);
            return (next, config.cloned());
        }
        Err(e) => e,
    };
    let keeps = config.is_some()
        && config == kept
        && !matches!(
            error,
            GithubError::NotConfigured | GithubError::SettingsInvalid
        );
    next.state = match &error {
        GithubError::NotConfigured => GithubState::NotConfigured,
        GithubError::Unavailable => GithubState::Unavailable {
            message: error.message_pl(),
        },
        other => GithubState::Error {
            kind: other.kind().to_string(),
            message: other.message_pl(),
        },
    };
    if keeps {
        next.repos = prev.repos.clone();
        next.last_success_ms = prev.last_success_ms;
        next.hidden_check_failed = prev.hidden_check_failed;
        next.hidden_error = prev.hidden_error.clone();
        next.new_references = prev.new_references.clone();
        (next, kept.cloned())
    } else {
        (next, None)
    }
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

    /// One otherwise valid issue whose `labels` value is the given raw JSON.
    fn with_labels(labels_json: &str) -> String {
        format!(
            r#"[{{"number":1,"title":"T","url":"{}","updatedAt":"2026-01-01T00:00:00Z","labels":{labels_json}}}]"#,
            url(1)
        )
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
    fn labels_keep_the_response_order_and_ignore_other_fields() {
        let json = with_labels(
            r#"[{"id":"x1","name":"bug","description":"d","color":"ff0000"},{"name":"needs-triage"},{"name":"area/ui"}]"#,
        );
        let got = parse_issues(&repo(), &json).unwrap();
        assert_eq!(got[0].labels, ["bug", "needs-triage", "area/ui"]);
    }

    #[test]
    fn missing_or_empty_labels_are_an_empty_list() {
        let missing = array(&[issue_json(1, "T", &url(1), "2026-01-01T00:00:00Z")]);
        assert!(
            parse_issues(&repo(), &missing).unwrap()[0]
                .labels
                .is_empty()
        );
        let empty = with_labels("[]");
        assert!(parse_issues(&repo(), &empty).unwrap()[0].labels.is_empty());
    }

    #[test]
    fn issue_serializes_its_labels() {
        let mut i = issue("octo-test/alpha", 1);
        i.labels = vec!["bug".to_string(), "needs-triage".to_string()];
        let v = serde_json::to_value(&i).unwrap();
        assert_eq!(v["labels"], serde_json::json!(["bug", "needs-triage"]));
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
            with_labels(&format!(r#""{MARKER}""#)),
            with_labels(&format!(r#"[{{"color":"{MARKER}"}}]"#)),
            with_labels(r#"[{"name":1}]"#),
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

    // ---- snapshot

    fn config(repos: &[&str]) -> GithubConfig {
        GithubConfig {
            repos: repos
                .iter()
                .map(|r| GithubRepo::parse(r).unwrap())
                .collect(),
        }
    }

    fn issue(repo: &str, n: u64) -> GithubIssue {
        GithubIssue {
            repo: repo.to_string(),
            number: n,
            reference: format!("{}#{n}", repo.to_ascii_lowercase()),
            title: format!("Invented {n}"),
            web_url: None,
            updated_at: "2026-03-03T08:00:00Z".to_string(),
            labels: vec![],
            changed: None,
            hidden_at_ms: None,
        }
    }

    fn result(repo: &str, outcome: Result<Vec<GithubIssue>, GithubError>) -> RepoResult {
        RepoResult {
            repo: GithubRepo::parse(repo).unwrap(),
            outcome,
        }
    }

    fn good_round() -> Vec<RepoResult> {
        vec![
            result("octo-test/alpha", Ok(vec![issue("octo-test/alpha", 1)])),
            result("octo-test/beta", Ok(vec![])),
        ]
    }

    fn prev_with_list() -> (GithubSnapshot, GithubConfig) {
        let cfg = config(&["octo-test/alpha", "octo-test/beta"]);
        let (snap, kept) = next_snapshot(
            &GithubSnapshot::starting(),
            None,
            Some(&cfg),
            Ok(good_round()),
            100,
        );
        assert_eq!(kept, Some(cfg.clone()));
        (snap, cfg)
    }

    #[test]
    fn hidden_fields_reset_on_success_and_carry_over_when_the_list_is_kept() {
        let (mut prev, cfg) = prev_with_list();
        assert_eq!(
            (prev.hidden_check_failed, prev.hidden_error.clone()),
            (0, None)
        );
        prev.hidden_check_failed = 2;
        prev.hidden_error = Some("fixed text".to_string());
        let (kept, _) = next_snapshot(
            &prev,
            Some(&cfg),
            Some(&cfg),
            Err(GithubError::Unavailable),
            300,
        );
        assert_eq!(kept.hidden_check_failed, 2);
        assert_eq!(kept.hidden_error.as_deref(), Some("fixed text"));
        let (fresh, _) = next_snapshot(&prev, Some(&cfg), Some(&cfg), Ok(good_round()), 400);
        assert_eq!((fresh.hidden_check_failed, fresh.hidden_error), (0, None));
        assert!(fresh.repos.iter().all(|r| r.hidden.is_empty()));
        let (dropped, _) = next_snapshot(
            &prev,
            Some(&cfg),
            None,
            Err(GithubError::NotConfigured),
            500,
        );
        assert_eq!(
            (dropped.hidden_check_failed, dropped.hidden_error),
            (0, None)
        );
    }

    #[test]
    fn starting_snapshot_is_empty() {
        let s = GithubSnapshot::starting();
        assert_eq!(s.state, GithubState::Starting);
        assert!(s.repos.is_empty());
        assert_eq!((s.seq, s.last_success_ms, s.checked_at_ms), (0, None, None));
        assert!(!s.refreshing);
        assert_eq!(s.refresh_minutes, DEFAULT_REFRESH_MINUTES);
        assert_eq!(s.next_refresh_at_ms, None);
        assert!(!s.refresh_setting_invalid);
        assert!(s.new_references.is_empty());
    }

    #[test]
    fn new_references_reset_on_success_and_carry_over_when_the_list_is_kept() {
        let (mut prev, cfg) = prev_with_list();
        prev.new_references = vec!["octo-test/alpha#1".into()];
        let (ok, _) = next_snapshot(&prev, Some(&cfg), Some(&cfg), Ok(good_round()), 300);
        assert!(ok.new_references.is_empty());
        let (kept, _) = next_snapshot(
            &prev,
            Some(&cfg),
            Some(&cfg),
            Err(GithubError::Unavailable),
            300,
        );
        assert_eq!(kept.new_references, ["octo-test/alpha#1"]);
        let (dropped, _) = next_snapshot(
            &prev,
            Some(&cfg),
            None,
            Err(GithubError::NotConfigured),
            300,
        );
        assert!(dropped.new_references.is_empty());
    }

    #[test]
    fn snapshot_serializes_new_references_in_camel_case() {
        let v = serde_json::to_value(GithubSnapshot::starting()).unwrap();
        assert_eq!(v["newReferences"], serde_json::json!([]));
    }

    #[test]
    fn snapshot_carries_the_refresh_interval_fields_across_rounds() {
        let (base, cfg) = prev_with_list();
        let prev = GithubSnapshot {
            refresh_minutes: 3,
            next_refresh_at_ms: Some(123_456),
            refresh_setting_invalid: true,
            ..base
        };
        let (ok, _) = next_snapshot(&prev, Some(&cfg), Some(&cfg), Ok(good_round()), 400);
        let (kept, _) = next_snapshot(
            &prev,
            Some(&cfg),
            Some(&cfg),
            Err(GithubError::Unavailable),
            400,
        );
        let (dropped, _) = next_snapshot(
            &prev,
            Some(&cfg),
            None,
            Err(GithubError::NotConfigured),
            400,
        );
        let (down, _) = next_snapshot(&prev, None, None, Err(GithubError::ThreadDown), 400);
        for s in [ok, kept, dropped, down] {
            assert_eq!(s.refresh_minutes, 3);
            assert_eq!(s.next_refresh_at_ms, Some(123_456));
            assert!(s.refresh_setting_invalid);
        }
    }

    #[test]
    fn snapshot_serializes_the_interval_fields() {
        let s = GithubSnapshot {
            refresh_minutes: 7,
            next_refresh_at_ms: Some(42),
            refresh_setting_invalid: true,
            ..GithubSnapshot::starting()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["refreshMinutes"], 7);
        assert_eq!(v["nextRefreshAtMs"], 42);
        assert_eq!(v["refreshSettingInvalid"], true);
        let v = serde_json::to_value(GithubSnapshot::starting()).unwrap();
        assert_eq!(v["nextRefreshAtMs"], serde_json::Value::Null);
        assert_eq!(v["refreshMinutes"], 10);
        assert_eq!(v["refreshSettingInvalid"], false);
    }

    #[test]
    fn success_replaces_repos_and_keeps_per_repo_errors() {
        let (prev, cfg) = prev_with_list();
        let round = vec![
            result(
                "octo-test/alpha",
                Err(GithubError::Failed { exit_code: Some(1) }),
            ),
            result("octo-test/beta", Ok(vec![issue("octo-test/beta", 5)])),
        ];
        let (next, kept) = next_snapshot(&prev, Some(&cfg), Some(&cfg), Ok(round), 200);
        assert_eq!(next.state, GithubState::Ok);
        assert_eq!(next.seq, prev.seq + 1);
        assert_eq!(next.last_success_ms, Some(200));
        assert_eq!(next.checked_at_ms, Some(200));
        assert!(!next.refreshing);
        assert_eq!(kept, Some(cfg));
        assert_eq!(next.repos.len(), 2);
        let err = next.repos[0].error.as_ref().unwrap();
        assert_eq!(err.kind, "Failed");
        assert!(next.repos[0].issues.is_empty());
        assert_eq!(next.repos[1].issues[0].number, 5);
        assert!(next.repos[1].error.is_none());
    }

    #[test]
    fn unavailable_keeps_the_list_while_the_repos_are_unchanged() {
        let (prev, cfg) = prev_with_list();
        let (next, kept) = next_snapshot(
            &prev,
            Some(&cfg),
            Some(&cfg),
            Err(GithubError::Unavailable),
            300,
        );
        assert_eq!(
            next.state,
            GithubState::Unavailable {
                message: GithubError::Unavailable.message_pl()
            }
        );
        assert_eq!(next.repos, prev.repos);
        assert_eq!(next.last_success_ms, Some(100));
        assert_eq!(next.checked_at_ms, Some(300));
        assert_eq!(kept, Some(cfg));
    }

    #[test]
    fn whole_source_errors_keep_the_list_with_their_kind() {
        let (prev, cfg) = prev_with_list();
        for e in [GithubError::LoggedOut, GithubError::ToolMissing] {
            let (next, kept) = next_snapshot(&prev, Some(&cfg), Some(&cfg), Err(e.clone()), 300);
            assert_eq!(
                next.state,
                GithubState::Error {
                    kind: e.kind().to_string(),
                    message: e.message_pl()
                }
            );
            assert_eq!(next.repos, prev.repos);
            assert_eq!(next.last_success_ms, Some(100));
            assert_eq!(kept, Some(cfg.clone()));
        }
    }

    #[test]
    fn a_changed_repo_list_drops_the_kept_list() {
        let (prev, cfg) = prev_with_list();
        let other = config(&["octo-test/alpha"]);
        for e in [GithubError::Unavailable, GithubError::LoggedOut] {
            let (next, kept) = next_snapshot(&prev, Some(&cfg), Some(&other), Err(e), 300);
            assert!(next.repos.is_empty());
            assert_eq!(next.last_success_ms, None);
            assert_eq!(kept, None);
        }
    }

    #[test]
    fn not_configured_and_invalid_settings_drop_the_list() {
        let (prev, cfg) = prev_with_list();
        let (next, kept) = next_snapshot(
            &prev,
            Some(&cfg),
            None,
            Err(GithubError::NotConfigured),
            300,
        );
        assert_eq!(next.state, GithubState::NotConfigured);
        assert!(next.repos.is_empty());
        assert_eq!(kept, None);

        // Even with an unchanged config, these two never keep.
        for e in [GithubError::NotConfigured, GithubError::SettingsInvalid] {
            let (next, kept) = next_snapshot(&prev, Some(&cfg), Some(&cfg), Err(e), 300);
            assert!(next.repos.is_empty());
            assert_eq!(next.last_success_ms, None);
            assert_eq!(kept, None);
        }
        let (next, _) = next_snapshot(
            &prev,
            Some(&cfg),
            None,
            Err(GithubError::SettingsInvalid),
            300,
        );
        assert_eq!(
            next.state,
            GithubState::Error {
                kind: "SettingsInvalid".to_string(),
                message: GithubError::SettingsInvalid.message_pl()
            }
        );
    }

    #[test]
    fn a_failure_without_a_config_never_keeps() {
        let (prev, cfg) = prev_with_list();
        let (next, kept) = next_snapshot(
            &prev,
            Some(&cfg),
            None,
            Err(GithubError::ConfigDirUnresolved),
            300,
        );
        assert!(next.repos.is_empty());
        assert_eq!(kept, None);
    }

    #[test]
    fn seq_always_increases() {
        let (prev, cfg) = prev_with_list();
        let (a, _) = next_snapshot(
            &prev,
            Some(&cfg),
            Some(&cfg),
            Err(GithubError::Unavailable),
            1,
        );
        let (b, _) = next_snapshot(&a, None, None, Err(GithubError::NotConfigured), 2);
        assert!(a.seq > prev.seq && b.seq > a.seq);
    }

    #[test]
    fn snapshot_serializes_camel_case_without_repo_names_in_errors() {
        let cfg = config(&["octo-test/alpha"]);
        let round = vec![result("octo-test/alpha", Err(GithubError::Truncated))];
        let (snap, _) = next_snapshot(&GithubSnapshot::starting(), None, Some(&cfg), Ok(round), 9);
        let v = serde_json::to_value(&snap).unwrap();
        assert_eq!(v["state"], serde_json::json!({"state": "ok"}));
        assert_eq!(v["lastSuccessMs"], 9);
        assert_eq!(v["repos"][0]["repo"], "octo-test/alpha");
        assert_eq!(v["repos"][0]["error"]["kind"], "Truncated");
        let message = v["repos"][0]["error"]["message"].as_str().unwrap();
        assert!(!message.contains("octo"), "{message}");
        let un = serde_json::to_value(GithubState::Unavailable {
            message: "m".into(),
        })
        .unwrap();
        assert_eq!(
            un,
            serde_json::json!({"state": "unavailable", "message": "m"})
        );
        let nc = serde_json::to_value(GithubState::NotConfigured).unwrap();
        assert_eq!(nc, serde_json::json!({"state": "notConfigured"}));
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
