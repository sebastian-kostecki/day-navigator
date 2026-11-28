//! Seen GitHub issues (S-03): the durable seen set behind the NEW flag of the
//! GitHub view. No `tauri` import and no clock: `now_ms` is a parameter. A
//! round is one `Store::write`. Rows hold the lowercase repo key, issue numbers
//! and instants only. No toast, no event log.

pub mod error;
pub mod rules;

use std::collections::HashSet;
use std::fmt;

use rusqlite::{OptionalExtension, Transaction};

use crate::github::fetch::RepoResult;
use crate::store::{Store, StoreError};
pub use error::GithubSeenError;
pub use rules::{FORGET_AFTER_MS, RepoPlan, SeenKey, StoredIssue, plan_repo};

/// One successful repo round: the lowercase `owner/name` key and the issue
/// numbers in list order. `Debug` is opaque so a repo never reaches a log.
#[derive(Clone, PartialEq, Eq)]
pub struct RepoRound {
    pub repo: String,
    pub numbers: Vec<u64>,
}

impl fmt::Debug for RepoRound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RepoRound(..)")
    }
}

/// The round input of one refresh: one entry per repo whose fetch succeeded,
/// in input order. A failed repo (`Err`, `Truncated` included) is left out.
pub fn round_input(results: &[RepoResult]) -> Vec<RepoRound> {
    results
        .iter()
        .filter_map(|result| {
            let issues = result.outcome.as_ref().ok()?;
            let numbers = issues
                .iter()
                .map(|issue| issue.number)
                .filter(|n| (1..=i64::MAX as u64).contains(n))
                .collect();
            Some(RepoRound {
                repo: result.repo.key(),
                numbers,
            })
        })
        .collect()
}

fn damaged() -> GithubSeenError {
    GithubSeenError::Store(StoreError::Damaged {
        reason: "github seen record is unreadable".into(),
    })
}

fn stored_issues(tx: &Transaction<'_>, repo: &str) -> Result<Vec<StoredIssue>, GithubSeenError> {
    let mut stmt = tx.prepare(
        "SELECT issue_number, first_seen_ms, last_seen_ms FROM github_seen_issues
         WHERE repo = ?1",
    )?;
    let rows = stmt.query_map([repo], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (number, first_seen_ms, last_seen_ms) = row?;
        let number = match u64::try_from(number) {
            Ok(n) if n > 0 => n,
            _ => return Err(damaged()),
        };
        out.push(StoredIssue {
            number,
            first_seen_ms,
            last_seen_ms,
        });
    }
    Ok(out)
}

/// Records one successful refresh and returns the fresh keys: visible-or-hidden
/// issues of the loaded repos first seen after the view's mark. One write:
/// read the mark and the seeded repos, plan each loaded repo, insert the
/// seeding marker when it seeds, upsert every current number, delete the
/// forgotten ones, then prune the repos no longer in `active`. A repo that is
/// active but not loaded keeps its rows.
pub fn record_round(
    store: &mut Store,
    loaded: &[RepoRound],
    active: &[String],
    now_ms: i64,
) -> Result<Vec<(SeenKey, i64)>, GithubSeenError> {
    store.write::<_, GithubSeenError>(|tx| {
        let mark: Option<i64> = tx
            .query_row(
                "SELECT seen_until_ms FROM github_view_mark WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let seeded: HashSet<String> = {
            let mut stmt = tx.prepare("SELECT repo FROM github_seen_repos")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<HashSet<String>, _>>()?
        };

        let mut fresh = Vec::new();
        for round in loaded {
            let stored = stored_issues(tx, &round.repo)?;
            let plan = plan_repo(
                seeded.contains(&round.repo),
                &stored,
                &round.numbers,
                mark,
                now_ms,
            );
            if plan.seed {
                tx.execute(
                    "INSERT INTO github_seen_repos (repo, seeded_at_ms) VALUES (?1, ?2)",
                    rusqlite::params![round.repo, now_ms],
                )?;
            }
            {
                let mut upsert = tx.prepare(
                    "INSERT INTO github_seen_issues (repo, issue_number, first_seen_ms, last_seen_ms)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (repo, issue_number)
                     DO UPDATE SET last_seen_ms = excluded.last_seen_ms",
                )?;
                for &number in &round.numbers {
                    upsert.execute(rusqlite::params![
                        round.repo,
                        number as i64,
                        plan.insert_first_seen_ms,
                        now_ms
                    ])?;
                }
            }
            {
                let mut delete = tx.prepare(
                    "DELETE FROM github_seen_issues WHERE repo = ?1 AND issue_number = ?2",
                )?;
                for &number in &plan.forget {
                    delete.execute(rusqlite::params![round.repo, number as i64])?;
                }
            }
            fresh.extend(plan.fresh.into_iter().map(|(number, first)| {
                (SeenKey::new(round.repo.clone(), number), first)
            }));
        }

        let active: HashSet<&str> = active.iter().map(String::as_str).collect();
        for repo in seeded.iter().filter(|r| !active.contains(r.as_str())) {
            tx.execute("DELETE FROM github_seen_issues WHERE repo = ?1", [repo])?;
            tx.execute("DELETE FROM github_seen_repos WHERE repo = ?1", [repo])?;
        }
        Ok(fresh)
    })
}

/// Saves the GitHub view's seen-until mark, never moving it back.
pub fn mark_view_seen(store: &mut Store, seen_until_ms: i64) -> Result<(), GithubSeenError> {
    store.write::<_, GithubSeenError>(|tx| {
        tx.execute(
            "INSERT INTO github_view_mark (id, seen_until_ms) VALUES (1, ?1)
             ON CONFLICT (id)
             DO UPDATE SET seen_until_ms = max(seen_until_ms, excluded.seen_until_ms)",
            [seen_until_ms],
        )?;
        Ok(())
    })
}

/// The saved seen-until mark, `None` before the first save.
pub fn view_mark(store: &Store) -> Result<Option<i64>, GithubSeenError> {
    store.read::<_, GithubSeenError>(|c| {
        let mark: Option<i64> = c
            .query_row(
                "SELECT seen_until_ms FROM github_view_mark WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(mark)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::GithubRepo;
    use crate::github::{GithubError, GithubIssue};

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

    #[test]
    fn round_input_takes_only_ok_repos_by_lowercase_key() {
        let results = vec![
            RepoResult {
                repo: GithubRepo::parse("Octo-Test/Alpha").unwrap(),
                outcome: Ok(vec![
                    issue("Octo-Test/Alpha", 5),
                    issue("Octo-Test/Alpha", 3),
                ]),
            },
            RepoResult {
                repo: GithubRepo::parse("octo-test/beta").unwrap(),
                outcome: Err(GithubError::Truncated),
            },
            RepoResult {
                repo: GithubRepo::parse("octo-test/gamma").unwrap(),
                outcome: Ok(vec![]),
            },
        ];
        assert_eq!(
            round_input(&results),
            vec![
                RepoRound {
                    repo: "octo-test/alpha".to_string(),
                    numbers: vec![5, 3],
                },
                RepoRound {
                    repo: "octo-test/gamma".to_string(),
                    numbers: vec![],
                },
            ]
        );
    }
}
