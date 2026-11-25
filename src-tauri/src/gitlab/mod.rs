//! Assigned GitLab issues and merge requests: the display model, parsing of
//! `glab api` output, and the rule for what the view shows after each round.
//!
//! Pure code: no `tauri`, no processes (those live in `fetch`). Invariant: no
//! error value, log line or message carries a hostname, a URL, a title, a
//! `serde_json` error or any other response content.

pub mod fetch;
pub mod filter;
pub mod notes;

use std::cmp::Reverse;

use chrono::{DateTime, FixedOffset};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::{CliError, GitlabConfig, GitlabHost, Tool};
use crate::settings::DEFAULT_REFRESH_MINUTES;

use self::filter::GitlabFilter;

/// Opaque, stable key of the account a list comes from: SHA-256 (hex) of the
/// host and the configured user. Carries neither value in readable form, so it
/// may be stored with "To do" entries; a changed host or user (or a typo in
/// the user) gives a different key.
pub fn account_key(config: &GitlabConfig) -> String {
    let mut hasher = Sha256::new();
    hasher.update(config.host.as_str().as_bytes());
    hasher.update(b"\n");
    if let Some(user) = &config.user {
        hasher.update(user.as_str().as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Issue,
    MergeRequest,
}

/// Why a hidden issue came back to the list (S-03). The stored reason when
/// several apply is `Status`, then `Comment`, then `Returned`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeReason {
    Status,
    Comment,
    Returned,
}

/// One label: its name and, when GitLab sent valid colours, the background and
/// text colour (`#rgb` or `#rrggbb`, lowercase). Both colours are set or both
/// are `None`. Work data: serde to the UI only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabLabel {
    pub name: String,
    pub color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabItem {
    pub kind: ItemKind,
    /// GitLab's global id: the stable key of a hide record (a reference changes
    /// when a project moves).
    pub id: u64,
    /// Rust-only: builds the notes call. Not sent to the UI.
    #[serde(skip)]
    pub project_id: u64,
    /// Rust-only: builds the notes call. Not sent to the UI.
    #[serde(skip)]
    pub iid: u64,
    /// `references.full`: `group/project#12` or `group/project!34`.
    pub reference: String,
    pub title: String,
    /// Only set for a link on the configured host; otherwise the row has no link.
    pub web_url: Option<String>,
    /// RFC 3339 text exactly as GitLab sent it.
    pub updated_at: String,
    pub labels: Vec<GitlabLabel>,
    pub draft: bool,
    /// Set on a visible issue that carries a "changed" mark (S-03).
    pub changed: Option<ChangeReason>,
    /// The milestone title, matched by the visibility filter and shown for the
    /// view's milestone filter. Work data: it travels to the UI only through
    /// serde, never into a log or an error.
    pub milestone: Option<String>,
    /// First seen after the view's seen-until mark (S-02). Set by the round,
    /// only on merge requests and visible issues; the parser gives `false`.
    pub is_new: bool,
    /// When the user hid the issue, UTC ms (S-02). Only on `hidden_issues`
    /// items; the parser gives `None`.
    pub hidden_at_ms: Option<i64>,
}

impl GitlabItem {
    /// The label names in the order GitLab sent them.
    pub fn label_names(&self) -> impl Iterator<Item = &str> {
        self.labels.iter().map(|l| l.name.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignedItems {
    pub merge_requests: Vec<GitlabItem>,
    /// Issues the user has not hidden; a returned one carries `changed` (S-03).
    pub issues: Vec<GitlabItem>,
    /// Issues the user has hidden (S-03). Empty in a fetched list; the round
    /// fills it when it partitions `issues`.
    pub hidden_issues: Vec<GitlabItem>,
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
    #[error("the app config dir cannot be resolved")]
    ConfigDirUnresolved,
    #[error("the gitlab refresh thread is not running")]
    ThreadDown,
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
            GitlabError::ConfigDirUnresolved => "ConfigDirUnresolved",
            GitlabError::ThreadDown => "ThreadDown",
        }
    }

    /// Fixed Polish description; never includes stderr, the host or response content.
    pub fn message_pl(&self) -> String {
        let tool = Tool::Glab;
        match self {
            GitlabError::NotConfigured => {
                "GitLab nie jest skonfigurowany — dodaj \"gitlab\": { \"host\": \"…\" } do settings.json (opcjonalnie \"user\": login, gdy glab jest zalogowany na konto serwisowe)"
                    .to_string()
            }
            GitlabError::SettingsInvalid => {
                "Nieprawidłowy blok \"gitlab\" w settings.json — \"host\" musi być samą nazwą hosta, np. gitlab.example.com, a opcjonalny \"user\" samym loginem"
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
            GitlabError::ConfigDirUnresolved => {
                "Nie można ustalić folderu konfiguracji aplikacji, więc nie ma skąd odczytać ustawień GitLaba"
                    .to_string()
            }
            GitlabError::ThreadDown => "Wątek odświeżania GitLaba nie działa".to_string(),
        }
    }
}

// ---------------------------------------------------------------- parsing

#[derive(Deserialize)]
struct RawRefs {
    full: String,
}

#[derive(Deserialize)]
struct RawMilestone {
    title: String,
}

/// A label as GitLab sends it: a bare name (a server that ignores
/// `with_labels_details`) or an object with details. Any other type fails the
/// parse.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawLabel {
    Name(String),
    Detailed {
        name: String,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        text_color: Option<String>,
    },
}

/// `#` and 3 or 6 hex digits, written lowercase; anything else is `None`.
fn checked_color(color: &str) -> Option<String> {
    let digits = color.strip_prefix('#')?;
    if !matches!(digits.len(), 3 | 6) || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(color.to_ascii_lowercase())
}

impl RawLabel {
    fn into_label(self) -> GitlabLabel {
        match self {
            RawLabel::Name(name) => GitlabLabel {
                name,
                color: None,
                text_color: None,
            },
            RawLabel::Detailed {
                name,
                color,
                text_color,
            } => {
                // Both colours or neither: a lone colour could be unreadable.
                let pair = color
                    .as_deref()
                    .and_then(checked_color)
                    .zip(text_color.as_deref().and_then(checked_color));
                let (color, text_color) = match pair {
                    Some((c, t)) => (Some(c), Some(t)),
                    None => (None, None),
                };
                GitlabLabel {
                    name,
                    color,
                    text_color,
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct RawItem {
    id: u64,
    iid: u64,
    project_id: u64,
    title: String,
    web_url: String,
    updated_at: String,
    #[serde(default)]
    labels: Vec<RawLabel>,
    references: RawRefs,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    milestone: Option<RawMilestone>,
    /// Read only by the recently-closed parser.
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    merged_at: Option<String>,
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

/// Reads the output of `glab api --paginate`: with piped stdout glab writes each
/// page's array in turn (`[...][...]`), never one merged array. Empty output and
/// any serde error are `ResponseInvalid` with no content.
pub(crate) fn parse_pages<T: DeserializeOwned>(json: &str) -> Result<Vec<T>, GitlabError> {
    let mut all = Vec::new();
    let mut pages = 0usize;
    // The serde error is dropped on purpose: it can quote the response.
    for page in serde_json::Deserializer::from_str(json).into_iter::<Vec<T>>() {
        all.extend(page.map_err(|_| GitlabError::ResponseInvalid)?);
        pages += 1;
    }
    if pages == 0 {
        return Err(GitlabError::ResponseInvalid);
    }
    Ok(all)
}

/// One raw item as a display item plus its `updated_at` instant.
fn build_item(
    r: RawItem,
    host: &GitlabHost,
    kind: ItemKind,
) -> Result<(DateTime<FixedOffset>, GitlabItem), GitlabError> {
    // GitLab ids start at 1. A zero would key a hide record wrongly and build
    // a notes endpoint the read-only guard refuses (`projects/0/...`).
    if r.id == 0 || r.project_id == 0 || r.iid == 0 {
        return Err(GitlabError::ResponseInvalid);
    }
    let stamp =
        DateTime::parse_from_rfc3339(&r.updated_at).map_err(|_| GitlabError::ResponseInvalid)?;
    let item = GitlabItem {
        kind,
        id: r.id,
        project_id: r.project_id,
        iid: r.iid,
        reference: r.references.full,
        title: r.title,
        web_url: checked_url(&r.web_url, host),
        updated_at: r.updated_at,
        labels: r.labels.into_iter().map(RawLabel::into_label).collect(),
        draft: kind == ItemKind::MergeRequest && r.draft,
        changed: None,
        milestone: r.milestone.map(|m| m.title),
        is_new: false,
        hidden_at_ms: None,
    };
    Ok((stamp, item))
}

fn parse_items(
    json: &str,
    host: &GitlabHost,
    kind: ItemKind,
) -> Result<Vec<GitlabItem>, GitlabError> {
    let raw: Vec<RawItem> = parse_pages(json)?;
    let mut keyed: Vec<(DateTime<FixedOffset>, GitlabItem)> = Vec::with_capacity(raw.len());
    for r in raw {
        keyed.push(build_item(r, host, kind)?);
    }
    keyed.sort_by(|a, b| (Reverse(a.0), &a.1.reference).cmp(&(Reverse(b.0), &b.1.reference)));
    Ok(keyed.into_iter().map(|(_, item)| item).collect())
}

/// The closing instant that makes an item "recently closed", or `None` when it
/// is not closed (an issue only in state `closed`; an MR in state `merged` by
/// `merged_at` or `closed` by `closed_at`) or the time is missing or unreadable.
fn closing_time(r: &RawItem, kind: ItemKind) -> Option<DateTime<FixedOffset>> {
    let text = match (kind, r.state.as_deref()?) {
        (ItemKind::Issue, "closed") | (ItemKind::MergeRequest, "closed") => r.closed_at.as_deref(),
        (ItemKind::MergeRequest, "merged") => r.merged_at.as_deref(),
        _ => None,
    }?;
    DateTime::parse_from_rfc3339(text).ok()
}

/// Items closed (issues), or closed or merged (MRs), at or after `cutoff`, each
/// with its closing instant, newest closing first (ties by reference).
/// `updated_after` only pre-filters on the server, so the decision is made here
/// with the closing time. A missing `state` or closing time drops the item; a
/// closing time of the wrong type is `ResponseInvalid`.
pub fn parse_recently_closed(
    json: &str,
    host: &GitlabHost,
    kind: ItemKind,
    cutoff: DateTime<FixedOffset>,
) -> Result<Vec<(DateTime<FixedOffset>, GitlabItem)>, GitlabError> {
    let raw: Vec<RawItem> = parse_pages(json)?;
    let mut closed: Vec<(DateTime<FixedOffset>, GitlabItem)> = Vec::new();
    for r in raw {
        let Some(at) = closing_time(&r, kind).filter(|at| *at >= cutoff) else {
            continue;
        };
        let (_, item) = build_item(r, host, kind)?;
        closed.push((at, item));
    }
    closed.sort_by(|a, b| (Reverse(a.0), &a.1.reference).cmp(&(Reverse(b.0), &b.1.reference)));
    Ok(closed)
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

/// How many items the visibility filter removed from each section.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilteredOut {
    pub merge_requests: u32,
    pub issues: u32,
    pub hidden_issues: u32,
}

/// A recently closed item with the instant it was closed or merged (RFC 3339),
/// which orders the section and is what its row shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedItem {
    #[serde(flatten)]
    pub item: GitlabItem,
    pub closed_at: String,
}

/// The optional "recently closed" section (`closedDays`): its own items, its own
/// filter count and its own error, so a failed closed fetch never touches the
/// open list. Never part of `AssignedItems`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedSection {
    pub days: u32,
    pub items: Vec<ClosedItem>,
    pub filtered_out: u32,
    /// Fixed Polish text when the closed fetch failed; `items` is then empty.
    pub error: Option<String>,
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
    /// Hidden issues whose comment check failed this round (kept on a failed round).
    pub hidden_check_failed: u32,
    /// True when `gitlab.user` is set, so the user's own comments are known.
    pub own_comments_known: bool,
    /// Fixed Polish text when the hidden state could not be read or written;
    /// the list then shows every issue.
    pub hidden_error: Option<String>,
    /// True when `gitlabRefresh.minutes` is invalid and the default interval is
    /// used. Set on every round, whatever the fetch outcome.
    pub refresh_setting_invalid: bool,
    /// True when the visibility filter narrows `items`. Belongs to the items.
    pub filter_active: bool,
    /// The filter that shaped `items`. Belongs to the items. Values are work
    /// data: `Debug` stays opaque and they only travel to the UI through serde.
    pub filter: GitlabFilter,
    /// The interval (minutes) the last round used. A round flag: set on every round.
    pub refresh_minutes: u32,
    /// When the wait after the last round ends (UTC ms); `None` before the first round.
    pub next_refresh_at_ms: Option<u64>,
    /// True when the `gitlabFilter` block is invalid and everything is shown.
    /// A round flag like `refresh_setting_invalid`: set on every round.
    pub filter_invalid: bool,
    /// What the filter removed from each section of `items`. Belongs to the items.
    pub filtered_out: FilteredOut,
    /// Recently closed items; `None` when `closedDays` is off or the filter is
    /// invalid. Belongs to the items.
    pub closed: Option<ClosedSection>,
    /// `account_key` of the config `items` came from; set iff `items` is. Kept
    /// next to the items so a reader never pairs them with another account.
    /// Not sent to the webview.
    #[serde(skip)]
    pub account: Option<String>,
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
            hidden_check_failed: 0,
            own_comments_known: false,
            hidden_error: None,
            refresh_setting_invalid: false,
            filter_active: false,
            filter: GitlabFilter::default(),
            refresh_minutes: DEFAULT_REFRESH_MINUTES,
            next_refresh_at_ms: None,
            filter_invalid: false,
            filtered_out: FilteredOut::default(),
            closed: None,
            account: None,
        }
    }
}

/// The next snapshot after one round, and the (host, user) the kept items now belong to.
///
/// `kept` is the config the previous snapshot's items came from; `config` is
/// this round's config (`None` when not configured or invalid). Success replaces
/// the items. `Unavailable` and the error kinds keep the items and
/// `last_success_ms` only while host and user are unchanged; `NotConfigured`,
/// `SettingsInvalid` and a changed host or user drop them. `seq` always increases.
pub fn next_snapshot(
    prev: &GitlabSnapshot,
    kept: Option<&GitlabConfig>,
    config: Option<&GitlabConfig>,
    outcome: Result<AssignedItems, GitlabError>,
    now_ms: u64,
) -> (GitlabSnapshot, Option<GitlabConfig>) {
    let seq = prev.seq + 1;
    let mut next = GitlabSnapshot {
        state: GitlabState::Ok,
        items: None,
        last_success_ms: None,
        checked_at_ms: Some(now_ms),
        refreshing: false,
        seq,
        hidden_check_failed: 0,
        own_comments_known: false,
        hidden_error: None,
        // A round flag, not tied to the list: carried over, the round sets it.
        refresh_setting_invalid: prev.refresh_setting_invalid,
        filter_active: false,
        filter: GitlabFilter::default(),
        // Round flags, carried over: the round sets them.
        refresh_minutes: prev.refresh_minutes,
        next_refresh_at_ms: prev.next_refresh_at_ms,
        filter_invalid: prev.filter_invalid,
        filtered_out: FilteredOut::default(),
        closed: None,
        account: None,
    };
    let items = match outcome {
        Ok(items) => {
            next.items = Some(items);
            next.last_success_ms = Some(now_ms);
            // The round sets `hidden_check_failed`, `hidden_error`, the filter
            // fields and `closed` after reconciling; they describe this round only.
            next.own_comments_known = config.is_some_and(|c| c.user.is_some());
            next.account = config.map(account_key);
            return (next, config.cloned());
        }
        Err(e) => e,
    };
    let same_config = config.is_some() && config == kept;
    let keeps = same_config
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
        // The hidden state belongs to the kept list: no reconciliation ran.
        next.hidden_check_failed = prev.hidden_check_failed;
        next.own_comments_known = prev.own_comments_known;
        next.hidden_error = prev.hidden_error.clone();
        // The filter state belongs to the kept list too.
        next.filter_active = prev.filter_active;
        next.filter = prev.filter.clone();
        next.filtered_out = prev.filtered_out;
        next.closed = prev.closed.clone();
        next.account = prev.account.clone();
        (next, kept.cloned())
    } else {
        (next, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::GitlabUser;

    fn host() -> GitlabHost {
        GitlabHost::parse("gitlab.example.invalid").unwrap()
    }

    fn mr_json(title: &str, url: &str, updated: &str, reference: &str, extra: &str) -> String {
        format!(
            r#"{{"id":1,"iid":1,"project_id":1,"title":"{title}","web_url":"{url}","updated_at":"{updated}","references":{{"short":"!1","relative":"!1","full":"{reference}"}},"state":"opened","author":{{"name":"Invented"}}{extra}}}"#
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
        assert_eq!(i.label_names().collect::<Vec<_>>(), ["backend", "review"]);
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

    fn labelled(labels_json: &str) -> String {
        format!(
            "[{}]",
            mr_json(
                "T",
                &url("a"),
                "2026-03-02T10:00:00Z",
                "g/p#1",
                &format!(r#","labels":{labels_json}"#)
            )
        )
    }

    fn parsed_labels(labels_json: &str) -> Vec<GitlabLabel> {
        let json = labelled(labels_json);
        let issues = parse_issues(&json, &host()).unwrap();
        assert_eq!(
            parse_merge_requests(&json, &host()).unwrap()[0].labels,
            issues[0].labels
        );
        issues.into_iter().next().unwrap().labels
    }

    fn label(name: &str, colors: Option<(&str, &str)>) -> GitlabLabel {
        GitlabLabel {
            name: name.to_string(),
            color: colors.map(|c| c.0.to_string()),
            text_color: colors.map(|c| c.1.to_string()),
        }
    }

    #[test]
    fn detailed_labels_keep_name_order_and_lowercase_colours() {
        let got = parsed_labels(
            r##"[{"id":1,"name":"workflow::doing","color":"#AABBCC","text_color":"#FFFFFF","description":"d"},{"id":2,"name":"bug","color":"#abc","text_color":"#000"}]"##,
        );
        assert_eq!(
            got,
            [
                label("workflow::doing", Some(("#aabbcc", "#ffffff"))),
                label("bug", Some(("#abc", "#000"))),
            ]
        );
    }

    #[test]
    fn plain_name_labels_have_no_colours() {
        assert_eq!(
            parsed_labels(r#"["backend","review"]"#),
            [label("backend", None), label("review", None)]
        );
        // The two forms may be mixed in one list.
        assert_eq!(
            parsed_labels(r##"["backend",{"name":"bug","color":"#ff0000","text_color":"#fff"}]"##),
            [
                label("backend", None),
                label("bug", Some(("#ff0000", "#fff")))
            ]
        );
    }

    #[test]
    fn colour_validation_accepts_hex_in_three_or_six_digits_only() {
        assert_eq!(checked_color("#abc").as_deref(), Some("#abc"));
        assert_eq!(checked_color("#AABBCC").as_deref(), Some("#aabbcc"));
        assert_eq!(checked_color("#0a9F3c").as_deref(), Some("#0a9f3c"));
        for bad in [
            "",
            "#",
            "red",
            "abc",
            "aabbcc",
            "#abcd",
            "#ab",
            "#abcde",
            "#aabbccd",
            "#ggg000",
            "#abc ",
            " #abc",
            "##abc",
            "#ab;c",
            "#ÀBC",
            "rgb(1,2,3)",
            "#abc\n",
        ] {
            assert_eq!(checked_color(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn an_invalid_or_missing_colour_makes_both_colours_none() {
        for body in [
            r##"{"name":"x","color":"red","text_color":"#fff"}"##,
            r##"{"name":"x","color":"#fff","text_color":"white"}"##,
            r##"{"name":"x","color":"#abcd","text_color":"#fff"}"##,
            r##"{"name":"x","color":"#ggg000","text_color":"#fff"}"##,
            // a missing or null text colour
            r##"{"name":"x","color":"#fff"}"##,
            r##"{"name":"x","color":"#fff","text_color":null}"##,
            // a missing or null colour
            r##"{"name":"x","text_color":"#fff"}"##,
            r##"{"name":"x","color":null,"text_color":"#fff"}"##,
            r#"{"name":"x"}"#,
        ] {
            assert_eq!(
                parsed_labels(&format!("[{body}]")),
                [label("x", None)],
                "{body}"
            );
        }
    }

    #[test]
    fn labels_of_the_wrong_shape_are_response_invalid_without_content() {
        const MARKER: &str = "LEAK-MARKER-LABEL-6e21";
        for labels in [
            r#"[5]"#.to_string(),
            r#"[null]"#.to_string(),
            r#"[["a"]]"#.to_string(),
            r##"[{"color":"#fff"}]"##.to_string(),
            r#"[{"name":5}]"#.to_string(),
            format!(r#"[{{"name":"{MARKER}","color":5}}]"#),
            format!(r#"[{{"name":["{MARKER}"]}}]"#),
            format!(r#""{MARKER}""#),
        ] {
            let json = labelled(&labels);
            for err in [
                parse_issues(&json, &host()).unwrap_err(),
                parse_merge_requests(&json, &host()).unwrap_err(),
            ] {
                assert_eq!(err, GitlabError::ResponseInvalid, "{labels}");
                let shown = [
                    format!("{err:?}"),
                    err.to_string(),
                    serde_json::to_string(&err).unwrap(),
                    err.message_pl(),
                ];
                for text in shown {
                    assert!(!text.contains(MARKER), "leak: {text}");
                }
            }
        }
    }

    #[test]
    fn labels_serialize_as_objects_with_camel_case_colours() {
        let json = labelled(r##"[{"name":"bug","color":"#D9534F","text_color":"#FFF"},"plain"]"##);
        let item = &parse_issues(&json, &host()).unwrap()[0];
        let v = serde_json::to_value(item).unwrap();
        assert_eq!(
            v["labels"],
            serde_json::json!([
                {"name": "bug", "color": "#d9534f", "textColor": "#fff"},
                {"name": "plain", "color": null, "textColor": null}
            ])
        );
        assert_eq!(item.label_names().collect::<Vec<_>>(), ["bug", "plain"]);
    }

    fn raw_json(id: u64, iid: u64, project_id: u64, reference: &str) -> String {
        format!(
            r#"{{"id":{id},"iid":{iid},"project_id":{project_id},"title":"T","web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03-02T10:00:00Z","references":{{"full":"{reference}"}}}}"#
        )
    }

    #[test]
    fn identity_fields_are_kept_and_not_all_are_serialized() {
        let json = format!("[{}]", raw_json(901, 12, 77, "g/p#12"));
        let i = &parse_issues(&json, &host()).unwrap()[0];
        assert_eq!((i.id, i.iid, i.project_id), (901, 12, 77));
        let v = serde_json::to_value(i).unwrap();
        assert_eq!(v["id"], 901);
        assert!(v.get("iid").is_none() && v.get("projectId").is_none());
        let mr = &parse_merge_requests(&json, &host()).unwrap()[0];
        assert_eq!((mr.id, mr.iid, mr.project_id), (901, 12, 77));
    }

    #[test]
    fn a_missing_identity_field_is_response_invalid() {
        for field in ["id", "iid", "project_id"] {
            let mut v: serde_json::Value =
                serde_json::from_str(&raw_json(1, 2, 3, "g/p#1")).unwrap();
            v.as_object_mut().unwrap().remove(field);
            let json = format!("[{v}]");
            assert_eq!(
                parse_issues(&json, &host()),
                Err(GitlabError::ResponseInvalid),
                "{field}"
            );
        }
    }

    #[test]
    fn a_zero_identity_field_is_response_invalid() {
        for (id, iid, project_id) in [(0, 2, 3), (1, 0, 3), (1, 2, 0)] {
            let json = format!("[{}]", raw_json(id, iid, project_id, "g/p#1"));
            assert_eq!(
                parse_issues(&json, &host()),
                Err(GitlabError::ResponseInvalid),
                "{id}/{iid}/{project_id}"
            );
            assert_eq!(
                parse_merge_requests(&json, &host()),
                Err(GitlabError::ResponseInvalid)
            );
        }
    }

    #[test]
    fn concatenated_pages_are_read_in_full_and_sorted_together() {
        let a = raw_json(1, 1, 1, "g/p#1");
        let b = raw_json(2, 2, 1, "g/p#2");
        let c = raw_json(3, 3, 1, "g/p#3");
        let json = format!("[{a},{b}]\n[{c}]");
        let ids: Vec<u64> = parse_issues(&json, &host())
            .unwrap()
            .iter()
            .map(|i| i.id)
            .collect();
        assert_eq!(ids, [1, 2, 3]);
        // An empty page between full ones is fine.
        let json = format!("[{a}][][{b}]");
        assert_eq!(parse_issues(&json, &host()).unwrap().len(), 2);
    }

    #[test]
    fn parse_pages_rejects_empty_trailing_garbage_and_truncated_pages() {
        let a = raw_json(1, 1, 1, "g/p#1");
        for bad in [
            String::new(),
            "  \n\t".to_string(),
            format!("[{a}] garbage"),
            format!("[{a}][{a}"),
            format!("[{a}]{{}}"),
        ] {
            assert_eq!(
                parse_pages::<serde_json::Value>(&bad),
                Err(GitlabError::ResponseInvalid),
                "{bad}"
            );
        }
        assert_eq!(parse_pages::<serde_json::Value>("[]").unwrap().len(), 0);
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
    fn milestone_title_is_read_when_present_and_serialized() {
        let present = mr_json(
            "T",
            &url("a"),
            "2026-03-02T10:00:00Z",
            "g/p#1",
            r#","milestone":{"id":4,"title":"Sprint 42"}"#,
        );
        let null = mr_json(
            "T",
            &url("a"),
            "2026-03-02T10:00:00Z",
            "g/p#1",
            r#","milestone":null"#,
        );
        let absent = mr_json("T", &url("a"), "2026-03-02T10:00:00Z", "g/p#1", "");
        for (body, want) in [(present, Some("Sprint 42")), (null, None), (absent, None)] {
            let json = format!("[{body}]");
            for items in [
                parse_issues(&json, &host()).unwrap(),
                parse_merge_requests(&json, &host()).unwrap(),
            ] {
                assert_eq!(items[0].milestone.as_deref(), want);
                let v = serde_json::to_value(&items[0]).unwrap();
                assert_eq!(v["milestone"], serde_json::json!(want));
            }
        }
        for wrong in [
            r#","milestone":"Sprint 42""#,
            r#","milestone":{"title":5}"#,
            r#","milestone":{}"#,
        ] {
            let json = format!(
                "[{}]",
                mr_json("T", &url("a"), "2026-03-02T10:00:00Z", "g/p#1", wrong)
            );
            assert_eq!(
                parse_issues(&json, &host()),
                Err(GitlabError::ResponseInvalid),
                "{wrong}"
            );
            assert_eq!(
                parse_merge_requests(&json, &host()),
                Err(GitlabError::ResponseInvalid),
                "{wrong}"
            );
        }
    }

    fn cutoff() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z").unwrap()
    }

    /// One closed-list entry: `fields` are raw JSON members appended after `state`.
    fn closed_json(id: u64, reference: &str, fields: &str) -> String {
        format!(
            r#"{{"id":{id},"iid":{id},"project_id":1,"title":"T","web_url":"https://gitlab.example.invalid/a","updated_at":"2026-03-10T10:00:00Z","references":{{"full":"{reference}"}}{fields}}}"#
        )
    }

    fn closed_refs(kind: ItemKind, entries: &[String]) -> Result<Vec<String>, GitlabError> {
        let json = format!("[{}]", entries.join(","));
        parse_recently_closed(&json, &host(), kind, cutoff())
            .map(|v| v.into_iter().map(|(_, i)| i.reference).collect())
    }

    #[test]
    fn closed_parse_keeps_merged_and_closed_mrs_and_drops_other_states() {
        let entries = [
            closed_json(
                1,
                "g/p!1",
                r#","state":"merged","merged_at":"2026-03-05T00:00:00Z","closed_at":null"#,
            ),
            closed_json(
                2,
                "g/p!2",
                r#","state":"closed","closed_at":"2026-03-04T00:00:00Z""#,
            ),
            closed_json(3, "g/p!3", r#","state":"opened","closed_at":null"#),
            closed_json(
                4,
                "g/p!4",
                r#","state":"locked","closed_at":"2026-03-04T00:00:00Z""#,
            ),
        ];
        assert_eq!(
            closed_refs(ItemKind::MergeRequest, &entries).unwrap(),
            ["g/p!1", "g/p!2"]
        );
    }

    #[test]
    fn closed_parse_keeps_only_closed_issues() {
        let entries = [
            closed_json(
                1,
                "g/p#1",
                r#","state":"closed","closed_at":"2026-03-05T00:00:00Z""#,
            ),
            closed_json(2, "g/p#2", r#","state":"opened","closed_at":null"#),
            // A merge-request state means nothing for an issue.
            closed_json(
                3,
                "g/p#3",
                r#","state":"merged","merged_at":"2026-03-05T00:00:00Z""#,
            ),
        ];
        assert_eq!(closed_refs(ItemKind::Issue, &entries).unwrap(), ["g/p#1"]);
    }

    #[test]
    fn closed_parse_decides_by_the_closing_time_not_by_updated_at() {
        // Updated after the cutoff (see `closed_json`) but closed before it.
        let entries = [
            closed_json(
                1,
                "g/p#1",
                r#","state":"closed","closed_at":"2026-02-28T23:59:59Z""#,
            ),
            closed_json(
                2,
                "g/p#2",
                r#","state":"closed","closed_at":"2026-03-01T00:00:00Z""#,
            ),
            // The same instant written with an offset, just before the cutoff.
            closed_json(
                3,
                "g/p#3",
                r#","state":"closed","closed_at":"2026-03-01T00:59:59+01:00""#,
            ),
        ];
        assert_eq!(closed_refs(ItemKind::Issue, &entries).unwrap(), ["g/p#2"]);
    }

    #[test]
    fn closed_parse_drops_a_missing_state_or_closing_time_without_an_error() {
        let entries = [
            closed_json(1, "g/p#1", ""),
            closed_json(2, "g/p#2", r#","state":"closed""#),
            closed_json(3, "g/p#3", r#","state":"closed","closed_at":null"#),
            closed_json(4, "g/p#4", r#","state":"closed","closed_at":"yesterday""#),
            closed_json(5, "g/p#5", r#","closed_at":"2026-03-05T00:00:00Z""#),
        ];
        assert_eq!(
            closed_refs(ItemKind::Issue, &entries).unwrap(),
            Vec::<String>::new()
        );
        let mr = [
            closed_json(
                6,
                "g/p!6",
                r#","state":"merged","closed_at":"2026-03-05T00:00:00Z""#,
            ),
            closed_json(7, "g/p!7", r#","state":"merged","merged_at":"nope""#),
        ];
        assert_eq!(
            closed_refs(ItemKind::MergeRequest, &mr).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn closed_parse_wrong_type_closing_time_is_response_invalid() {
        for fields in [
            r#","state":"closed","closed_at":5"#,
            r#","state":"merged","merged_at":{}"#,
            r#","state":5"#,
        ] {
            let json = format!("[{}]", closed_json(1, "g/p#1", fields));
            for kind in [ItemKind::Issue, ItemKind::MergeRequest] {
                assert_eq!(
                    parse_recently_closed(&json, &host(), kind, cutoff()),
                    Err(GitlabError::ResponseInvalid),
                    "{fields}"
                );
            }
        }
    }

    #[test]
    fn closed_parse_sorts_by_closing_time_then_reference() {
        let entries = [
            closed_json(
                1,
                "g/p#1",
                r#","state":"closed","closed_at":"2026-03-02T00:00:00Z""#,
            ),
            closed_json(
                2,
                "g/p#2",
                r#","state":"closed","closed_at":"2026-03-04T00:00:00Z""#,
            ),
            // Same instant as #2, written with an offset: ties go by reference.
            closed_json(
                3,
                "g/a#3",
                r#","state":"closed","closed_at":"2026-03-04T01:00:00+01:00""#,
            ),
        ];
        assert_eq!(
            closed_refs(ItemKind::Issue, &entries).unwrap(),
            ["g/a#3", "g/p#2", "g/p#1"]
        );
    }

    #[test]
    fn closed_parse_returns_the_closing_instant_and_keeps_the_item_fields() {
        let json = format!(
            "[{}]",
            closed_json(
                1,
                "g/p!1",
                r#","state":"merged","merged_at":"2026-03-05T12:00:00Z","milestone":{"title":"Sprint 1"}"#
            )
        );
        let got = parse_recently_closed(&json, &host(), ItemKind::MergeRequest, cutoff()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].0,
            DateTime::parse_from_rfc3339("2026-03-05T12:00:00Z").unwrap()
        );
        assert_eq!(got[0].1.kind, ItemKind::MergeRequest);
        assert_eq!(got[0].1.milestone.as_deref(), Some("Sprint 1"));
        assert!(got[0].1.changed.is_none());
    }

    #[test]
    fn open_items_without_state_or_closing_fields_still_parse() {
        let json = format!("[{}]", raw_json(1, 2, 3, "g/p#1"));
        assert_eq!(parse_issues(&json, &host()).unwrap().len(), 1);
        assert_eq!(parse_merge_requests(&json, &host()).unwrap().len(), 1);
    }

    #[test]
    fn closed_parse_errors_never_leak_response_content() {
        const MARKER: &str = "LEAK-MARKER-CLOSED-2d7b";
        let bodies = [
            format!(r#"[{{"title":"{MARKER}","state":"closed","closed_at":"2026-03"#),
            format!(
                r#"[{{"id":1,"iid":1,"project_id":1,"title":"T","web_url":"u","updated_at":"2026-03-10T10:00:00Z","references":{{"full":"g/p#1"}},"state":"closed","closed_at":["{MARKER}"]}}]"#
            ),
            format!(r#"{{"{MARKER}":1}}"#),
        ];
        for body in &bodies {
            let err = parse_recently_closed(body, &host(), ItemKind::Issue, cutoff()).unwrap_err();
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

    fn closed_section(error: Option<&str>) -> ClosedSection {
        ClosedSection {
            days: 7,
            items: if error.is_some() {
                vec![]
            } else {
                vec![ClosedItem {
                    item: item("g/p#9"),
                    closed_at: "2026-03-02T10:00:00+00:00".into(),
                }]
            },
            filtered_out: 1,
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn snapshot_keeps_closed_with_the_items_and_drops_it_with_them() {
        let prev = GitlabSnapshot {
            items: Some(items("g/p#1")),
            last_success_ms: Some(1000),
            closed: Some(closed_section(None)),
            ..GitlabSnapshot::starting()
        };
        let (s, _) = next_snapshot(
            &prev,
            Some(&config()),
            Some(&config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert_eq!(s.closed, Some(closed_section(None)));
        for (cfg, err) in [
            (Some(other_user_config()), GitlabError::Unavailable),
            (None, GitlabError::NotConfigured),
            (None, GitlabError::SettingsInvalid),
        ] {
            let (s, _) = next_snapshot(&prev, Some(&config()), cfg.as_ref(), Err(err), 2000);
            assert_eq!(s.items, None);
            assert_eq!(s.closed, None);
        }
        // A successful round starts clean: the round fills `closed`.
        let (s, _) = next_snapshot(
            &prev,
            Some(&config()),
            Some(&config()),
            Ok(items("g/p#1")),
            3000,
        );
        assert_eq!(s.closed, None);
    }

    #[test]
    fn snapshot_serializes_the_closed_section() {
        let s = GitlabSnapshot {
            closed: Some(closed_section(None)),
            ..GitlabSnapshot::starting()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["closed"]["days"], 7);
        assert_eq!(v["closed"]["filteredOut"], 1);
        assert_eq!(v["closed"]["error"], serde_json::Value::Null);
        // The item is flattened next to its closing time.
        assert_eq!(v["closed"]["items"][0]["reference"], "g/p#9");
        assert_eq!(
            v["closed"]["items"][0]["closedAt"],
            "2026-03-02T10:00:00+00:00"
        );
        assert!(v["closed"]["items"][0].get("item").is_none());
        let failed = GitlabSnapshot {
            closed: Some(closed_section(Some("fixed text"))),
            ..GitlabSnapshot::starting()
        };
        let v = serde_json::to_value(&failed).unwrap();
        assert_eq!(v["closed"]["error"], "fixed text");
        assert_eq!(v["closed"]["items"], serde_json::json!([]));
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
        assert_eq!(
            serde_json::to_string(&GitlabError::ConfigDirUnresolved).unwrap(),
            r#"{"kind":"ConfigDirUnresolved"}"#
        );
        assert_eq!(GitlabError::ThreadDown.kind(), "ThreadDown");
        // An unresolved config dir is not a bad "gitlab" block.
        assert!(
            !GitlabError::ConfigDirUnresolved
                .message_pl()
                .contains("\"gitlab\"")
        );
    }

    #[test]
    fn thread_down_without_config_is_an_error_with_no_items() {
        let mut prev = GitlabSnapshot::starting();
        prev.refreshing = true;
        let (s, k) = next_snapshot(&prev, None, None, Err(GitlabError::ThreadDown), 9);
        assert!(matches!(
            s.state,
            GitlabState::Error { ref kind, .. } if kind == "ThreadDown"
        ));
        assert!(!s.refreshing);
        assert_eq!(s.items, None);
        assert!(s.seq > prev.seq);
        assert_eq!(k, None);
    }

    // ----- snapshot rule

    fn item(reference: &str) -> GitlabItem {
        GitlabItem {
            kind: ItemKind::Issue,
            id: 1,
            project_id: 1,
            iid: 1,
            reference: reference.to_string(),
            title: "T".into(),
            web_url: None,
            updated_at: "2026-03-02T10:00:00Z".into(),
            labels: vec![],
            draft: false,
            changed: None,
            milestone: None,
            is_new: false,
            hidden_at_ms: None,
        }
    }

    fn items(reference: &str) -> AssignedItems {
        AssignedItems {
            merge_requests: vec![],
            issues: vec![item(reference)],
            hidden_issues: vec![],
        }
    }

    fn other_host() -> GitlabHost {
        GitlabHost::parse("other.example.invalid").unwrap()
    }

    fn config() -> GitlabConfig {
        GitlabConfig {
            host: host(),
            user: Some(GitlabUser::parse("invented.user").unwrap()),
        }
    }

    fn other_config() -> GitlabConfig {
        GitlabConfig {
            host: other_host(),
            user: Some(GitlabUser::parse("invented.user").unwrap()),
        }
    }

    fn other_user_config() -> GitlabConfig {
        GitlabConfig {
            user: Some(GitlabUser::parse("other.invented").unwrap()),
            ..config()
        }
    }

    fn no_user_config() -> GitlabConfig {
        GitlabConfig {
            user: None,
            ..config()
        }
    }

    #[test]
    fn snapshot_first_success_sets_items_and_time() {
        let prev = GitlabSnapshot::starting();
        let (s, kept) = next_snapshot(&prev, None, Some(&config()), Ok(items("g/p#1")), 1000);
        assert_eq!(s.state, GitlabState::Ok);
        assert_eq!(s.items, Some(items("g/p#1")));
        assert_eq!(s.last_success_ms, Some(1000));
        assert_eq!(s.checked_at_ms, Some(1000));
        assert!(!s.refreshing);
        assert_eq!(s.seq, 1);
        assert_eq!(kept, Some(config()));
    }

    #[test]
    fn snapshot_failure_after_success_keeps_items_and_last_success() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&config()),
            Ok(items("g/p#1")),
            1000,
        );
        for (err, label) in [
            (GitlabError::LoggedOut, "LoggedOut"),
            (GitlabError::ToolMissing, "ToolMissing"),
            (GitlabError::Failed { exit_code: Some(1) }, "Failed"),
            (GitlabError::ResponseInvalid, "ResponseInvalid"),
        ] {
            let (s, k) = next_snapshot(&ok, kept.as_ref(), Some(&config()), Err(err.clone()), 2000);
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
            assert_eq!(k, Some(config()));
        }
    }

    #[test]
    fn snapshot_unavailable_keeps_items() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&config()),
            Ok(items("g/p#1")),
            1000,
        );
        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&config()),
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
        assert_eq!(k, Some(config()));
    }

    #[test]
    fn snapshot_failure_without_prior_success_has_no_items() {
        let (s, k) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&config()),
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
            Some(&config()),
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
            Some(&config()),
            Ok(items("g/p#1")),
            1000,
        );
        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&other_config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert!(matches!(s.state, GitlabState::Unavailable { .. }));
        assert!(s.items.is_none() && s.last_success_ms.is_none());
        assert!(k.is_none());

        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&other_config()),
            Ok(items("g/p#2")),
            3000,
        );
        assert_eq!(s.items, Some(items("g/p#2")));
        assert_eq!(s.last_success_ms, Some(3000));
        assert_eq!(k, Some(other_config()));
    }

    #[test]
    fn snapshot_user_change_drops_items_on_failure_and_replaces_on_success() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&config()),
            Ok(items("g/p#1")),
            1000,
        );
        for changed in [other_user_config(), no_user_config()] {
            let (s, k) = next_snapshot(
                &ok,
                kept.as_ref(),
                Some(&changed),
                Err(GitlabError::Unavailable),
                2000,
            );
            assert!(matches!(s.state, GitlabState::Unavailable { .. }));
            assert!(s.items.is_none() && s.last_success_ms.is_none());
            assert!(k.is_none());

            let (s, k) =
                next_snapshot(&ok, kept.as_ref(), Some(&changed), Ok(items("g/p#2")), 3000);
            assert_eq!(s.items, Some(items("g/p#2")));
            assert_eq!(k, Some(changed));
        }
        // Same host and user: still kept.
        let (s, k) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert_eq!(s.items, Some(items("g/p#1")));
        assert_eq!(k, Some(config()));
    }

    #[test]
    fn account_key_is_stable_opaque_and_differs_per_host_and_user() {
        let key = account_key(&config());
        assert_eq!(key, account_key(&config()));
        assert_eq!(key.len(), 64);
        assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(!key.contains("example") && !key.contains("gitlab"));
        for other in [other_config(), other_user_config(), no_user_config()] {
            assert_ne!(account_key(&other), key);
        }
    }

    #[test]
    fn snapshot_account_follows_the_items() {
        let (ok, kept) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            Some(&config()),
            Ok(items("g/p#1")),
            1000,
        );
        assert_eq!(ok.account, Some(account_key(&config())));
        // Kept items keep their account.
        let (s, _) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert_eq!(s.account, ok.account);
        // Dropped items drop it; a success under another user replaces it.
        let (s, _) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&other_user_config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert!(s.items.is_none() && s.account.is_none());
        let (s, _) = next_snapshot(
            &ok,
            kept.as_ref(),
            Some(&other_user_config()),
            Ok(items("g/p#2")),
            3000,
        );
        assert_eq!(s.account, Some(account_key(&other_user_config())));
        // Never sent to the webview.
        let json = serde_json::to_value(&s).unwrap();
        assert!(json.get("account").is_none());
    }

    #[test]
    fn settings_messages_mention_the_optional_user() {
        for msg in [
            GitlabError::NotConfigured.message_pl(),
            GitlabError::SettingsInvalid.message_pl(),
        ] {
            assert!(msg.contains("\"user\""), "{msg}");
        }
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
            let (s, k) = next_snapshot(&prev, kept.as_ref(), Some(&config()), outcome, n as u64);
            assert!(s.seq > last);
            last = s.seq;
            prev = s;
            kept = k;
        }
    }

    #[test]
    fn snapshot_keeps_hidden_state_with_the_items_on_unavailable_and_error_rounds() {
        let mut shown = items("g/p#1");
        shown.hidden_issues = vec![item("g/p#2")];
        let prev = GitlabSnapshot {
            items: Some(shown.clone()),
            last_success_ms: Some(1000),
            hidden_check_failed: 2,
            own_comments_known: true,
            hidden_error: Some("fixed text".into()),
            ..GitlabSnapshot::starting()
        };
        for err in [GitlabError::Unavailable, GitlabError::LoggedOut] {
            let (s, kept) = next_snapshot(&prev, Some(&config()), Some(&config()), Err(err), 2000);
            assert_eq!(s.items, Some(shown.clone()));
            assert_eq!(s.hidden_check_failed, 2);
            assert!(s.own_comments_known);
            assert_eq!(s.hidden_error.as_deref(), Some("fixed text"));
            assert_eq!(kept, Some(config()));
        }
        // A changed user drops the list and its hidden state with it.
        let (s, _) = next_snapshot(
            &prev,
            Some(&config()),
            Some(&other_user_config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert_eq!(s.items, None);
        assert_eq!(s.hidden_check_failed, 0);
        assert!(!s.own_comments_known);
        assert_eq!(s.hidden_error, None);
    }

    #[test]
    fn snapshot_keeps_refresh_setting_invalid_across_every_round_outcome() {
        let prev = GitlabSnapshot {
            refresh_setting_invalid: true,
            ..GitlabSnapshot::starting()
        };
        for err in [
            GitlabError::Unavailable,
            GitlabError::LoggedOut,
            GitlabError::NotConfigured,
            GitlabError::SettingsInvalid,
        ] {
            let (s, _) = next_snapshot(&prev, Some(&config()), Some(&config()), Err(err), 2);
            assert!(s.refresh_setting_invalid);
        }
        let (s, _) = next_snapshot(&prev, None, Some(&config()), Ok(items("g/p#1")), 2);
        assert!(s.refresh_setting_invalid);
        let (s, _) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            None,
            Err(GitlabError::Unavailable),
            2,
        );
        assert!(!s.refresh_setting_invalid);
    }

    #[test]
    fn snapshot_serializes_refresh_setting_invalid() {
        let s = GitlabSnapshot {
            refresh_setting_invalid: true,
            ..GitlabSnapshot::starting()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["refreshSettingInvalid"], true);
    }

    #[test]
    fn snapshot_keeps_filter_state_with_the_items_and_resets_it_when_they_drop() {
        let counts = FilteredOut {
            merge_requests: 1,
            issues: 2,
            hidden_issues: 3,
        };
        let prev = GitlabSnapshot {
            items: Some(items("g/p#1")),
            last_success_ms: Some(1000),
            filter_active: true,
            filtered_out: counts,
            ..GitlabSnapshot::starting()
        };
        let (s, _) = next_snapshot(
            &prev,
            Some(&config()),
            Some(&config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert!(s.filter_active);
        assert_eq!(s.filtered_out, counts);
        for (cfg, err) in [
            (Some(other_user_config()), GitlabError::Unavailable),
            (None, GitlabError::NotConfigured),
        ] {
            let (s, _) = next_snapshot(&prev, Some(&config()), cfg.as_ref(), Err(err), 2000);
            assert_eq!(s.items, None);
            assert!(!s.filter_active);
            assert_eq!(s.filtered_out, FilteredOut::default());
        }
        // A successful round starts clean: the round fills the filter fields.
        let (s, _) = next_snapshot(&prev, None, Some(&config()), Ok(items("g/p#1")), 3000);
        assert!(!s.filter_active);
        assert_eq!(s.filtered_out, FilteredOut::default());
    }

    #[test]
    fn snapshot_carries_filter_invalid_as_a_round_flag() {
        let prev = GitlabSnapshot {
            filter_invalid: true,
            ..GitlabSnapshot::starting()
        };
        for err in [
            GitlabError::Unavailable,
            GitlabError::LoggedOut,
            GitlabError::NotConfigured,
            GitlabError::SettingsInvalid,
        ] {
            let (s, _) = next_snapshot(&prev, Some(&config()), Some(&config()), Err(err), 2);
            assert!(s.filter_invalid);
        }
        let (s, _) = next_snapshot(&prev, None, Some(&config()), Ok(items("g/p#1")), 2);
        assert!(s.filter_invalid);
        let (s, _) = next_snapshot(
            &GitlabSnapshot::starting(),
            None,
            None,
            Err(GitlabError::Unavailable),
            2,
        );
        assert!(!s.filter_invalid);
    }

    fn invented_filter() -> GitlabFilter {
        let mut filter = GitlabFilter::default();
        filter.projects.include = vec!["acme/invented-project".to_string()];
        filter.closed_days = Some(5);
        filter
    }

    #[test]
    fn snapshot_keeps_the_filter_with_the_list_and_resets_it_without() {
        let prev = GitlabSnapshot {
            items: Some(items("g/p#1")),
            last_success_ms: Some(1000),
            filter: invented_filter(),
            ..GitlabSnapshot::starting()
        };
        let (s, _) = next_snapshot(
            &prev,
            Some(&config()),
            Some(&config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        assert_eq!(s.filter, invented_filter());
        for (cfg, err) in [
            (Some(other_user_config()), GitlabError::Unavailable),
            (None, GitlabError::NotConfigured),
        ] {
            let (s, _) = next_snapshot(&prev, Some(&config()), cfg.as_ref(), Err(err), 2000);
            assert_eq!(s.items, None);
            assert_eq!(s.filter, GitlabFilter::default());
        }
    }

    #[test]
    fn snapshot_carries_the_refresh_interval_fields_across_rounds() {
        let prev = GitlabSnapshot {
            items: Some(items("g/p#1")),
            refresh_minutes: 3,
            next_refresh_at_ms: Some(123_456),
            ..GitlabSnapshot::starting()
        };
        let (kept, _) = next_snapshot(
            &prev,
            Some(&config()),
            Some(&config()),
            Err(GitlabError::Unavailable),
            2000,
        );
        let (dropped, _) = next_snapshot(
            &prev,
            Some(&config()),
            None,
            Err(GitlabError::NotConfigured),
            2000,
        );
        for s in [kept, dropped] {
            assert_eq!(s.refresh_minutes, 3);
            assert_eq!(s.next_refresh_at_ms, Some(123_456));
        }
        let start = GitlabSnapshot::starting();
        assert_eq!(start.refresh_minutes, DEFAULT_REFRESH_MINUTES);
        assert_eq!(start.next_refresh_at_ms, None);
    }

    #[test]
    fn snapshot_debug_does_not_show_filter_values() {
        let s = GitlabSnapshot {
            filter: invented_filter(),
            ..GitlabSnapshot::starting()
        };
        assert!(!format!("{s:?}").contains("invented-project"));
    }

    #[test]
    fn snapshot_serializes_the_filter_and_the_interval_fields() {
        let s = GitlabSnapshot {
            filter: invented_filter(),
            refresh_minutes: 7,
            next_refresh_at_ms: Some(42),
            ..GitlabSnapshot::starting()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(
            v["filter"]["projects"]["include"],
            serde_json::json!(["acme/invented-project"])
        );
        assert_eq!(v["filter"]["projects"]["exclude"], serde_json::json!([]));
        assert_eq!(v["filter"]["closedDays"], 5);
        assert_eq!(v["refreshMinutes"], 7);
        assert_eq!(v["nextRefreshAtMs"], 42);
        let v = serde_json::to_value(GitlabSnapshot::starting()).unwrap();
        assert_eq!(v["filter"]["closedDays"], serde_json::Value::Null);
        assert_eq!(v["nextRefreshAtMs"], serde_json::Value::Null);
    }

    #[test]
    fn snapshot_success_reports_whether_own_comments_are_known() {
        let prev = GitlabSnapshot::starting();
        let (s, _) = next_snapshot(&prev, None, Some(&config()), Ok(items("g/p#1")), 1);
        assert!(s.own_comments_known);
        let (s, _) = next_snapshot(&prev, None, Some(&no_user_config()), Ok(items("g/p#1")), 1);
        assert!(!s.own_comments_known);
    }

    #[test]
    fn snapshot_serializes_for_the_webview() {
        let mut s = GitlabSnapshot::starting();
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"state":{"state":"starting"},"items":null,"lastSuccessMs":null,"checkedAtMs":null,"refreshing":false,"seq":0,"hiddenCheckFailed":0,"ownCommentsKnown":false,"hiddenError":null,"refreshSettingInvalid":false,"filterActive":false,"filter":{"projects":{"include":[],"exclude":[]},"labels":{"include":[],"exclude":[]},"milestones":{"include":[],"exclude":[]},"closedDays":null},"refreshMinutes":10,"nextRefreshAtMs":null,"filterInvalid":false,"filteredOut":{"mergeRequests":0,"issues":0,"hiddenIssues":0},"closed":null}"#
        );
        s.closed = Some(ClosedSection {
            days: 7,
            items: vec![],
            filtered_out: 2,
            error: Some("m".into()),
        });
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(
            v["closed"],
            serde_json::json!({"days":7,"items":[],"filteredOut":2,"error":"m"})
        );
        s.closed = None;
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

    #[test]
    fn item_serializes_new_and_hidden_date_for_the_webview() {
        let mut i = item("acme/invented#1");
        let v = serde_json::to_value(&i).unwrap();
        assert_eq!(v["isNew"], serde_json::json!(false));
        assert_eq!(v["hiddenAtMs"], serde_json::Value::Null);
        i.is_new = true;
        i.hidden_at_ms = Some(1_800_000_000_000);
        let v = serde_json::to_value(&i).unwrap();
        assert_eq!(v["isNew"], serde_json::json!(true));
        assert_eq!(v["hiddenAtMs"], serde_json::json!(1_800_000_000_000_i64));
        let closed = ClosedItem {
            item: i,
            closed_at: "2026-03-03T08:00:00Z".into(),
        };
        let v = serde_json::to_value(&closed).unwrap();
        assert_eq!(v["isNew"], serde_json::json!(true));
    }
}
