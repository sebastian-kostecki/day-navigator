//! Reconciliation of hidden GitHub issues in one successful round, in the same
//! two `tauri`-free halves as the GitLab round:
//!
//! - `check_github_round` reads the records and runs the due comment calls
//!   (no lock held);
//! - `commit_github_round` plans the transitions per repo, applies them in one
//!   store write and partitions the repos from the rows that write returned.
//!
//! Only repos with an `Ok` outcome take part: a failed repo's records get no
//! transition and no call, so its absence is never judged. Logging: kinds and
//! plain counts only, never a repo name, number, login or comment author.

use std::collections::HashMap;

use chrono::Local;

use super::items::{HiddenItemRecord, HiddenKind, HiddenSource, apply_items_round, list_items};
use super::{
    Comment, CommentVerdict, HiddenIssueError, HideRecord, comment_verdict, needs_comment_check,
    plan_round, status_changed,
};
use crate::cli::{CliRunner, GithubConfig, GithubIssueNumber};
use crate::github::fetch::{RepoResult, fetch_issue_comments, fetch_login};
use crate::github::{GithubError, GithubIssue, RepoView, repo_view};
use crate::store::StoreHandle;

/// One record's key: the lowercase repo key and the issue number.
type Key = (String, u64);

/// What `check_github_round` found: the records read before the calls, one
/// verdict per checked issue, and the record whose failed call stopped the
/// round (if any).
#[derive(Debug)]
pub struct GithubRoundChecks {
    records: Result<Vec<HiddenItemRecord>, HiddenIssueError>,
    checks: HashMap<Key, Result<CommentVerdict, ()>>,
    stopped_at: Option<Key>,
}

impl GithubRoundChecks {
    /// The record whose comment call stopped the round (or the previous
    /// `resume_after` when the login call failed). The next round passes it to
    /// `check_github_round`, so a call that keeps timing out cannot starve the
    /// checks ordered after it.
    pub fn stopped_at(&self) -> Option<(String, u64)> {
        self.stopped_at.clone()
    }
}

/// What the round shows: the partitioned repos, how many comment checks failed
/// and the store failure (if any) that made the repos unpartitioned.
#[derive(Debug)]
pub struct GithubRoundOutcome {
    pub repos: Vec<RepoView>,
    pub check_failed: u32,
    pub store_error: Option<HiddenIssueError>,
}

/// A failure after which more calls are pointless this round.
fn stops_round(e: &GithubError) -> bool {
    matches!(
        e,
        GithubError::Unavailable | GithubError::LoggedOut | GithubError::ToolMissing
    )
}

/// The records in check order: those after `resume_after` first, then the rest
/// from the start; both parts in `(scope, number)` order.
fn check_order<'a>(
    records: &'a [HiddenItemRecord],
    resume_after: Option<&Key>,
) -> Vec<&'a HiddenItemRecord> {
    let mut sorted: Vec<&HiddenItemRecord> = records.iter().collect();
    sorted.sort_by(|a, b| (&a.scope, a.item_id).cmp(&(&b.scope, b.item_id)));
    let Some(cursor) = resume_after else {
        return sorted;
    };
    let (rest, first): (Vec<_>, Vec<_>) = sorted
        .into_iter()
        .partition(|r| (&r.scope, r.item_id) <= (&cursor.0, cursor.1));
    first.into_iter().chain(rest).collect()
}

/// A hidden record that is due a comment check, with the repo and issue it
/// belongs to.
struct Due<'a> {
    record: &'a HiddenItemRecord,
    repo: &'a crate::cli::GithubRepo,
}

/// Reads the records and runs the comment calls of the hidden issues that are
/// due. Holds no lock. The login is fetched once, before the first due check;
/// when that fails no comment call starts and every due check counts as
/// failed. After the first unavailable, logged-out or missing-tool failure of
/// a comment call no further call starts. A store read failure is carried in
/// the result and skips the calls.
///
/// `resume_after` is the previous round's `stopped_at`.
pub fn check_github_round(
    store: &StoreHandle,
    runner: &CliRunner,
    results: &[RepoResult],
    now_ms: i64,
    resume_after: Option<(String, u64)>,
) -> GithubRoundChecks {
    let today = Local::now().date_naive();
    let records = match store.with_store(today, |s| {
        list_items(s, HiddenSource::Github, HiddenKind::Issue)
    }) {
        Ok(records) => records,
        Err(e) => {
            return GithubRoundChecks {
                records: Err(e),
                checks: HashMap::new(),
                stopped_at: None,
            };
        }
    };
    let mut due: Vec<Due<'_>> = Vec::new();
    for rec in check_order(&records, resume_after.as_ref()) {
        let Some((repo, issues)) = results.iter().find_map(|r| {
            (r.repo.key() == rec.scope)
                .then(|| r.outcome.as_ref().ok().map(|i| (&r.repo, i)))
                .flatten()
        }) else {
            continue;
        };
        let Some(item) = issues.iter().find(|i| i.number == rec.item_id) else {
            continue;
        };
        if status_changed(rec, item) || !needs_comment_check(rec, item, now_ms) {
            continue;
        }
        due.push(Due { record: rec, repo });
    }
    let mut checks: HashMap<Key, Result<CommentVerdict, ()>> = HashMap::new();
    let mut stopped_at = None;
    if !due.is_empty() {
        let key_of = |d: &Due<'_>| (d.record.scope.clone(), d.record.item_id);
        match fetch_login(runner) {
            Err(e) => {
                log::debug!("github: hidden check login failed: {}", e.kind());
                stopped_at = resume_after.clone();
                for d in &due {
                    checks.insert(key_of(d), Err(()));
                }
            }
            Ok(login) => {
                let mut halted = false;
                for d in &due {
                    // An unreadable baseline cannot be compared: a failed
                    // check, never a guess.
                    let verdict = match (
                        d.record.baseline(),
                        GithubIssueNumber::new(d.record.item_id),
                    ) {
                        (Some(baseline), Some(number)) if !halted => {
                            match fetch_issue_comments(runner, d.repo, number) {
                                Ok(found) => {
                                    let comments: Vec<Comment> = found
                                        .into_iter()
                                        .map(|c| Comment {
                                            author: c.author,
                                            counts: true,
                                            created_at: c.created_at,
                                        })
                                        .collect();
                                    Ok(comment_verdict(&comments, baseline, Some(login.as_str())))
                                }
                                Err(e) => {
                                    if stops_round(&e) {
                                        halted = true;
                                        stopped_at = Some(key_of(d));
                                    }
                                    Err(())
                                }
                            }
                        }
                        _ => Err(()),
                    };
                    checks.insert(key_of(d), verdict);
                }
            }
        }
    }
    let failed = checks.values().filter(|v| v.is_err()).count();
    if failed > 0 {
        log::debug!("github: hidden comment checks failed: {failed}");
    }
    GithubRoundChecks {
        records: Ok(records),
        checks,
        stopped_at,
    }
}

/// Plans and applies the round's transitions in one store write, then splits
/// every repo's issues from the rows that write returned. When the store
/// cannot be read or written the repos are returned unpartitioned with
/// `store_error` set: never an empty list in place of a failed source.
pub fn commit_github_round(
    store: &StoreHandle,
    config: &GithubConfig,
    results: &[RepoResult],
    checks: GithubRoundChecks,
    now_ms: i64,
) -> GithubRoundOutcome {
    let unpartitioned = |error: HiddenIssueError| {
        log::warn!("github: hidden state unavailable: {}", error.kind());
        GithubRoundOutcome {
            repos: results.iter().cloned().map(repo_view).collect(),
            check_failed: 0,
            store_error: Some(error),
        }
    };
    let GithubRoundChecks {
        records,
        checks: verdicts,
        ..
    } = checks;
    let records = match records {
        Ok(records) => records,
        Err(e) => return unpartitioned(e),
    };
    let mut planned: Vec<(String, Vec<super::Transition>)> = Vec::new();
    for result in results {
        let Ok(issues) = &result.outcome else {
            continue;
        };
        let scope = result.repo.key();
        let own: Vec<HiddenItemRecord> = records
            .iter()
            .filter(|r| r.scope == scope)
            .cloned()
            .collect();
        let repo_verdicts: HashMap<u64, Result<CommentVerdict, ()>> = verdicts
            .iter()
            .filter(|((s, _), _)| *s == scope)
            .map(|((_, n), v)| (*n, *v))
            .collect();
        let transitions = plan_round(&own, issues, &repo_verdicts, now_ms);
        if !transitions.is_empty() {
            planned.push((scope, transitions));
        }
    }
    let active: Vec<String> = config.repos.iter().map(|r| r.key()).collect();
    let today = Local::now().date_naive();
    let rows = match store.with_store(today, |s| {
        apply_items_round(
            s,
            HiddenSource::Github,
            HiddenKind::Issue,
            &planned,
            &active,
            now_ms,
        )
    }) {
        Ok(rows) => rows,
        Err(e) => return unpartitioned(e),
    };
    let repos = partition_github(results, &rows);
    // Only issues still hidden after the write count: one that came back
    // without comments (returned) is visible and needs no re-check notice.
    let check_failed = verdicts
        .iter()
        .filter(|((scope, n), v)| {
            v.is_err()
                && rows
                    .iter()
                    .any(|r| r.scope == *scope && r.item_id == *n && r.is_hidden())
        })
        .count() as u32;
    GithubRoundOutcome {
        repos,
        check_failed,
        store_error: None,
    }
}

/// One view per result in settings order. A failed repo keeps its error view.
/// For an `Ok` repo, issues without a record are visible, hidden records go to
/// `hidden` (with `hidden_at_ms`) and marked records stay visible with
/// `changed`; list order is kept in both.
pub fn partition_github(results: &[RepoResult], records: &[HiddenItemRecord]) -> Vec<RepoView> {
    results
        .iter()
        .map(|result| {
            let Ok(issues) = &result.outcome else {
                return repo_view(result.clone());
            };
            let scope = result.repo.key();
            let mut visible: Vec<GithubIssue> = Vec::new();
            let mut hidden: Vec<GithubIssue> = Vec::new();
            for issue in issues {
                let record = records
                    .iter()
                    .find(|r| r.scope == scope && r.item_id == issue.number);
                match record {
                    None => visible.push(GithubIssue {
                        changed: None,
                        hidden_at_ms: None,
                        ..issue.clone()
                    }),
                    Some(r) if r.is_hidden() => hidden.push(GithubIssue {
                        changed: None,
                        hidden_at_ms: Some(r.hidden_at_ms),
                        ..issue.clone()
                    }),
                    Some(r) => visible.push(GithubIssue {
                        changed: r.changed_reason,
                        hidden_at_ms: None,
                        ..issue.clone()
                    }),
                }
            }
            RepoView {
                repo: result.repo.as_str().to_string(),
                issues: visible,
                hidden,
                error: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::GithubRepo;
    use crate::hidden_issues::ChangeReason;

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

    fn record(scope: &str, n: u64, reason: Option<ChangeReason>) -> HiddenItemRecord {
        HiddenItemRecord {
            source: HiddenSource::Github,
            scope: scope.to_string(),
            kind: HiddenKind::Issue,
            item_id: n,
            project_id: None,
            iid: None,
            status_fingerprint: vec![],
            baseline_updated_at: "2026-03-03T08:00:00Z".to_string(),
            settle_after_ms: None,
            hidden_at_ms: 1_700_000_000_000 + n as i64,
            absent_since_ms: None,
            changed_reason: reason,
            changed_at_ms: reason.map(|_| 1_700_000_100_000),
        }
    }

    fn ok(repo: &str, numbers: &[u64]) -> RepoResult {
        RepoResult {
            repo: GithubRepo::parse(repo).unwrap(),
            outcome: Ok(numbers.iter().map(|n| issue(repo, *n)).collect()),
        }
    }

    #[test]
    fn partition_keeps_order_and_sets_hidden_at_only_on_hidden_items() {
        let results = vec![ok("Octo-Test/Alpha", &[9, 3, 5, 1])];
        let records = vec![
            record("octo-test/alpha", 3, None),
            record("octo-test/alpha", 1, None),
            record("octo-test/alpha", 5, Some(ChangeReason::Comment)),
            // A record of another repo with the same number changes nothing.
            record("octo-test/beta", 9, None),
        ];
        let views = partition_github(&results, &records);
        assert_eq!(views.len(), 1);
        let v = &views[0];
        assert_eq!(v.repo, "Octo-Test/Alpha");
        let visible: Vec<u64> = v.issues.iter().map(|i| i.number).collect();
        let hidden: Vec<u64> = v.hidden.iter().map(|i| i.number).collect();
        assert_eq!(visible, [9, 5]);
        assert_eq!(hidden, [3, 1]);
        assert_eq!(v.issues[0].changed, None);
        assert_eq!(v.issues[1].changed, Some(ChangeReason::Comment));
        assert!(v.issues.iter().all(|i| i.hidden_at_ms.is_none()));
        assert!(v.hidden.iter().all(|i| i.changed.is_none()));
        assert_eq!(v.hidden[0].hidden_at_ms, Some(1_700_000_000_003));
        assert_eq!(v.hidden[1].hidden_at_ms, Some(1_700_000_000_001));
    }

    #[test]
    fn partition_keeps_the_error_view_of_a_failed_repo() {
        let results = vec![
            RepoResult {
                repo: GithubRepo::parse("octo-test/alpha").unwrap(),
                outcome: Err(GithubError::Failed { exit_code: Some(1) }),
            },
            ok("octo-test/beta", &[2]),
        ];
        let records = vec![record("octo-test/alpha", 3, None)];
        let views = partition_github(&results, &records);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].error.as_ref().unwrap().kind, "Failed");
        assert!(views[0].issues.is_empty() && views[0].hidden.is_empty());
        assert!(views[1].error.is_none());
        assert_eq!(views[1].issues.len(), 1);
    }

    #[test]
    fn check_order_rotates_after_the_resume_key() {
        let records = vec![
            record("octo-test/beta", 1, None),
            record("octo-test/alpha", 3, None),
            record("octo-test/alpha", 11, None),
        ];
        let keys = |order: Vec<&HiddenItemRecord>| -> Vec<(String, u64)> {
            order
                .into_iter()
                .map(|r| (r.scope.clone(), r.item_id))
                .collect()
        };
        let plain = keys(check_order(&records, None));
        assert_eq!(plain[0], ("octo-test/alpha".to_string(), 3));
        let cursor = ("octo-test/alpha".to_string(), 3);
        let rotated = keys(check_order(&records, Some(&cursor)));
        assert_eq!(
            rotated,
            [
                ("octo-test/alpha".to_string(), 11),
                ("octo-test/beta".to_string(), 1),
                ("octo-test/alpha".to_string(), 3),
            ]
        );
    }
}
