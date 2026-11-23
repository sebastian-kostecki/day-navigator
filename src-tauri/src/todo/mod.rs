//! "To do" list: today's picks from open own tasks and assigned GitLab items.
//!
//! No `tauri` import; `today` and `now` are parameters. The day is the local
//! calendar date. Visibility and roll-over are computed on every call, so no
//! scheduler is needed: an entry is on day D's list if it is live (not removed,
//! not closed) and either open or ticked on D. Every write first closes the
//! live entries ticked on any other day (`roll_over_in`), in the same
//! transaction.
//! Own entries take their ticked state from `own_tasks.done_at`.

pub mod commands;
pub mod error;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{OptionalExtension, Transaction};
use serde::Serialize;

use crate::github::GithubIssue;
use crate::gitlab::{AssignedItems, GitlabItem, ItemKind};
use crate::own_tasks;
use crate::space::Space;
use crate::store::{Store, StoreError};
pub use error::TodoError;

const TS_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const DATE_FORMAT: &str = "%Y-%m-%d";

/// Open entries allowed on a day's list unless `settings.json` says otherwise.
pub const DEFAULT_TODO_LIMIT: u32 = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoList {
    /// Local date the list is for, `YYYY-MM-DD`.
    pub day: String,
    pub limit: Option<u32>,
    pub limit_error: Option<TodoError>,
    pub open_count: u32,
    pub items: Vec<TodoItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: i64,
    pub picked_on: String,
    pub carried_over: bool,
    pub ticked: bool,
    pub picked_at: String,
    pub entry: TodoEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TodoEntry {
    Own {
        task_id: i64,
        space: Space,
        title: String,
    },
    Gitlab {
        item_kind: ItemKind,
        reference: String,
        title: String,
        web_url: Option<String>,
        closed_in_source: bool,
    },
    Github {
        /// Lowercase `owner/name#N`.
        reference: String,
        title: String,
        web_url: Option<String>,
        closed_in_source: bool,
    },
}

fn stamp(now: NaiveDateTime) -> String {
    now.format(TS_FORMAT).to_string()
}

fn day_text(day: NaiveDate) -> String {
    day.format(DATE_FORMAT).to_string()
}

fn kind_text(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Issue => "issue",
        ItemKind::MergeRequest => "mergeRequest",
    }
}

fn kind_from_text(s: &str) -> Option<ItemKind> {
    match s {
        "issue" => Some(ItemKind::Issue),
        "mergeRequest" => Some(ItemKind::MergeRequest),
        _ => None,
    }
}

fn corrupt() -> rusqlite::Error {
    // Carrier only: surfaces as a value-free `StoreError::Sql`.
    rusqlite::Error::InvalidQuery
}

/// Gives `closed_at` to every live entry ticked on a day other than `today`:
/// normally an earlier day, but also a later one after the clock moved back,
/// so no live entry is ever hidden from today's list while still blocking a
/// re-pick. Idempotent; returns the number of entries closed.
pub fn roll_over_in(tx: &Transaction<'_>, today: NaiveDate) -> Result<usize, StoreError> {
    let day = day_text(today);
    let closed_at = stamp(today.and_hms_opt(0, 0, 0).unwrap_or_default());
    let changed = tx.execute(
        "UPDATE todo_items SET closed_at = ?1 \
         WHERE removed_at IS NULL AND closed_at IS NULL AND ( \
           (source = 'own' AND EXISTS ( \
              SELECT 1 FROM own_tasks o WHERE o.id = todo_items.own_task_id \
                AND o.done_at IS NOT NULL AND substr(o.done_at, 1, 10) <> ?2)) \
           OR (source IN ('gitlab','github') AND ticked_at IS NOT NULL \
               AND substr(ticked_at, 1, 10) <> ?2))",
        (&closed_at, &day),
    )?;
    Ok(changed)
}

/// Raw live row joined with its own task, before the visibility rule.
struct Row {
    id: i64,
    source: String,
    own_task_id: Option<i64>,
    gitlab_kind: Option<String>,
    gitlab_ref: Option<String>,
    gitlab_title: Option<String>,
    gitlab_url: Option<String>,
    github_ref: Option<String>,
    github_title: Option<String>,
    github_url: Option<String>,
    picked_on: String,
    picked_at: String,
    ticked_at: Option<String>,
    ticked_by: Option<String>,
    own_space: Option<String>,
    own_title: Option<String>,
    own_done_at: Option<String>,
}

fn read_live(tx: &Transaction<'_>) -> Result<Vec<Row>, rusqlite::Error> {
    let mut stmt = tx.prepare(
        "SELECT t.id, t.source, t.own_task_id, t.gitlab_kind, t.gitlab_ref, t.gitlab_title, \
                t.gitlab_url, t.github_ref, t.github_title, t.github_url, \
                t.picked_on, t.picked_at, t.ticked_at, t.ticked_by, \
                o.space, o.title, o.done_at \
         FROM todo_items t LEFT JOIN own_tasks o ON o.id = t.own_task_id \
         WHERE t.removed_at IS NULL AND t.closed_at IS NULL",
    )?;
    stmt.query_map([], |r| {
        Ok(Row {
            id: r.get(0)?,
            source: r.get(1)?,
            own_task_id: r.get(2)?,
            gitlab_kind: r.get(3)?,
            gitlab_ref: r.get(4)?,
            gitlab_title: r.get(5)?,
            gitlab_url: r.get(6)?,
            github_ref: r.get(7)?,
            github_title: r.get(8)?,
            github_url: r.get(9)?,
            picked_on: r.get(10)?,
            picked_at: r.get(11)?,
            ticked_at: r.get(12)?,
            ticked_by: r.get(13)?,
            own_space: r.get(14)?,
            own_title: r.get(15)?,
            own_done_at: r.get(16)?,
        })
    })?
    .collect()
}

impl Row {
    /// The ticked timestamp that matters for the entry's source.
    fn tick(&self) -> Option<&str> {
        if self.source == "own" {
            self.own_done_at.as_deref()
        } else {
            self.ticked_at.as_deref()
        }
    }

    fn into_item(self, day: &str) -> Result<TodoItem, rusqlite::Error> {
        let ticked = self.tick().is_some();
        let carried_over = self.picked_on != day;
        let entry = match self.source.as_str() {
            "own" => TodoEntry::Own {
                task_id: self.own_task_id.ok_or_else(corrupt)?,
                space: match self.own_space.as_deref() {
                    Some("work") => Space::Work,
                    Some("private") => Space::Private,
                    _ => return Err(corrupt()),
                },
                title: self.own_title.ok_or_else(corrupt)?,
            },
            "gitlab" => TodoEntry::Gitlab {
                item_kind: self
                    .gitlab_kind
                    .as_deref()
                    .and_then(kind_from_text)
                    .ok_or_else(corrupt)?,
                reference: self.gitlab_ref.ok_or_else(corrupt)?,
                title: self.gitlab_title.ok_or_else(corrupt)?,
                web_url: self.gitlab_url,
                closed_in_source: self.ticked_by.as_deref() == Some("source"),
            },
            "github" => TodoEntry::Github {
                reference: self.github_ref.ok_or_else(corrupt)?,
                title: self.github_title.ok_or_else(corrupt)?,
                web_url: self.github_url,
                closed_in_source: self.ticked_by.as_deref() == Some("source"),
            },
            _ => return Err(corrupt()),
        };
        Ok(TodoItem {
            id: self.id,
            picked_on: self.picked_on,
            carried_over,
            ticked,
            picked_at: self.picked_at,
            entry,
        })
    }
}

/// Live entries on `day`'s list: open, or ticked on `day`. Open entries first
/// by `(picked_at, id)`, then ticked ones in the same order.
fn visible(tx: &Transaction<'_>, day: &str) -> Result<Vec<TodoItem>, rusqlite::Error> {
    let mut items = Vec::new();
    for row in read_live(tx)? {
        let on_list = match row.tick() {
            None => true,
            Some(t) => t.get(..10) == Some(day),
        };
        if on_list {
            items.push(row.into_item(day)?);
        }
    }
    items.sort_by(|a, b| (a.ticked, &a.picked_at, a.id).cmp(&(b.ticked, &b.picked_at, b.id)));
    Ok(items)
}

fn open_count(items: &[TodoItem]) -> u32 {
    items.iter().filter(|i| !i.ticked).count() as u32
}

/// Roll-over, then today's list. One write, so the roll-over is durable.
pub fn list(
    store: &mut Store,
    today: NaiveDate,
    _now: NaiveDateTime,
    limit: Result<u32, TodoError>,
) -> Result<TodoList, TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let day = day_text(today);
        let items = visible(tx, &day)?;
        let (limit, limit_error) = match limit {
            Ok(n) => (Some(n), None),
            Err(e) => (None, Some(e)),
        };
        Ok(TodoList {
            day,
            limit,
            limit_error,
            open_count: open_count(&items),
            items,
        })
    })
}

/// `LimitReached` when the open count has reached the limit.
fn check_limit(tx: &Transaction<'_>, today: NaiveDate, limit: u32) -> Result<(), TodoError> {
    let items = visible(tx, &day_text(today))?;
    if open_count(&items) >= limit {
        return Err(TodoError::LimitReached { limit });
    }
    Ok(())
}

/// Picks an open own task. Returns the new entry's id.
pub fn add_own(
    store: &mut Store,
    task_id: i64,
    today: NaiveDate,
    now: NaiveDateTime,
    limit: Result<u32, TodoError>,
) -> Result<i64, TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let limit = limit?;
        let done_at: Option<Option<String>> = tx
            .query_row(
                "SELECT done_at FROM own_tasks WHERE id = ?1",
                [task_id],
                |r| r.get(0),
            )
            .optional()?;
        match done_at {
            None => return Err(TodoError::NotFound),
            Some(Some(_)) => return Err(TodoError::TaskDone),
            Some(None) => {}
        }
        let live: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM todo_items WHERE own_task_id = ?1 \
               AND removed_at IS NULL AND closed_at IS NULL)",
            [task_id],
            |r| r.get(0),
        )?;
        if live {
            return Err(TodoError::AlreadyOnList);
        }
        check_limit(tx, today, limit)?;
        tx.execute(
            "INSERT INTO todo_items (source, own_task_id, picked_on, picked_at) \
             VALUES ('own', ?1, ?2, ?3)",
            (task_id, day_text(today), stamp(now)),
        )?;
        Ok(tx.last_insert_rowid())
    })
}

/// Picks a GitLab item (the caller took it from the live snapshot) under the
/// snapshot's `account` (`gitlab::account_key`). Returns the new entry's id.
pub fn add_gitlab(
    store: &mut Store,
    account: &str,
    item: &GitlabItem,
    today: NaiveDate,
    now: NaiveDateTime,
    limit: Result<u32, TodoError>,
) -> Result<i64, TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let limit = limit?;
        let kind = kind_text(item.kind);
        let live: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM todo_items WHERE gitlab_kind = ?1 AND gitlab_ref = ?2 \
               AND removed_at IS NULL AND closed_at IS NULL)",
            (kind, &item.reference),
            |r| r.get(0),
        )?;
        if live {
            return Err(TodoError::AlreadyOnList);
        }
        check_limit(tx, today, limit)?;
        tx.execute(
            "INSERT INTO todo_items \
               (source, gitlab_kind, gitlab_ref, gitlab_title, gitlab_url, gitlab_account, \
                picked_on, picked_at) \
             VALUES ('gitlab', ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                kind,
                &item.reference,
                &item.title,
                &item.web_url,
                account,
                day_text(today),
                stamp(now),
            ),
        )?;
        Ok(tx.last_insert_rowid())
    })
}

/// Picks a GitHub issue (the caller took it from the live snapshot). The
/// reference is stored lowercase (`owner/name#N`), so settings case never makes
/// two entries of one issue. Returns the new entry's id.
pub fn add_github(
    store: &mut Store,
    issue: &GithubIssue,
    today: NaiveDate,
    now: NaiveDateTime,
    limit: Result<u32, TodoError>,
) -> Result<i64, TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let limit = limit?;
        let reference = issue.reference.to_lowercase();
        let live: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM todo_items WHERE github_ref = ?1 \
               AND removed_at IS NULL AND closed_at IS NULL)",
            [&reference],
            |r| r.get(0),
        )?;
        if live {
            return Err(TodoError::AlreadyOnList);
        }
        check_limit(tx, today, limit)?;
        tx.execute(
            "INSERT INTO todo_items \
               (source, github_ref, github_title, github_url, picked_on, picked_at) \
             VALUES ('github', ?1, ?2, ?3, ?4, ?5)",
            (
                &reference,
                &issue.title,
                &issue.web_url,
                day_text(today),
                stamp(now),
            ),
        )?;
        Ok(tx.last_insert_rowid())
    })
}

/// Ticks or unticks an entry on today's list. An own entry changes the task's
/// `done_at` in the same transaction; a GitLab or GitHub entry keeps its own
/// tick.
pub fn set_ticked(
    store: &mut Store,
    id: i64,
    ticked: bool,
    today: NaiveDate,
    now: NaiveDateTime,
) -> Result<(), TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let entry: Option<(Option<i64>, Option<String>)> = tx
            .query_row(
                "SELECT own_task_id, ticked_at FROM todo_items \
                 WHERE id = ?1 AND removed_at IS NULL AND closed_at IS NULL",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((own_task_id, ticked_at)) = entry else {
            return Err(TodoError::NotFound);
        };
        if let Some(task_id) = own_task_id {
            own_tasks::set_done_in(tx, task_id, ticked, now)?;
        } else if ticked {
            if ticked_at.is_none() {
                tx.execute(
                    "UPDATE todo_items SET ticked_at = ?1, ticked_by = 'user' WHERE id = ?2",
                    (stamp(now), id),
                )?;
            }
        } else {
            tx.execute(
                "UPDATE todo_items SET ticked_at = NULL, ticked_by = NULL WHERE id = ?1",
                [id],
            )?;
        }
        Ok(())
    })
}

/// Takes an entry off the list. An own task's done state is untouched.
pub fn remove(
    store: &mut Store,
    id: i64,
    today: NaiveDate,
    now: NaiveDateTime,
) -> Result<(), TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let changed = tx.execute(
            "UPDATE todo_items SET removed_at = ?1 \
             WHERE id = ?2 AND removed_at IS NULL AND closed_at IS NULL",
            (stamp(now), id),
        )?;
        if changed == 0 {
            return Err(TodoError::NotFound);
        }
        Ok(())
    })
}

/// Call only after a successful GitLab round; `account` is the key of the
/// config `items` came from. Ticks (`ticked_by = 'source'`) every open live
/// GitLab entry of that account (or picked before accounts were recorded)
/// missing from `items`, and refreshes the stored title and URL of the ones
/// still present. Entries of another account are left alone: a missing item
/// there says nothing about its state. Own entries are never touched.
/// An issue the user hid (S-03) is still open and assigned, so it counts as
/// present: hiding it never ticks its "To do" entry.
/// Returns the number of entries changed (roll-over closings not counted).
pub fn reconcile_gitlab(
    store: &mut Store,
    account: &str,
    items: &AssignedItems,
    today: NaiveDate,
    now: NaiveDateTime,
) -> Result<usize, TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let present: HashMap<(&'static str, &str), &GitlabItem> = items
            .merge_requests
            .iter()
            .chain(items.issues.iter())
            .chain(items.hidden_issues.iter())
            .map(|i| ((kind_text(i.kind), i.reference.as_str()), i))
            .collect();

        let mut stmt = tx.prepare(
            "SELECT id, gitlab_kind, gitlab_ref, gitlab_title, gitlab_url FROM todo_items \
             WHERE source = 'gitlab' AND removed_at IS NULL AND closed_at IS NULL \
               AND ticked_at IS NULL \
               AND (gitlab_account IS NULL OR gitlab_account = ?1)",
        )?;
        let open = stmt
            .query_map([account], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        let ts = stamp(now);
        let mut changed = 0;
        for (id, kind, reference, title, url) in open {
            let key = (kind.as_str(), reference.as_str());
            match present.get(&key) {
                None => {
                    tx.execute(
                        "UPDATE todo_items SET ticked_at = ?1, ticked_by = 'source' WHERE id = ?2",
                        (&ts, id),
                    )?;
                    changed += 1;
                }
                Some(item) if item.title != title || item.web_url != url => {
                    tx.execute(
                        "UPDATE todo_items SET gitlab_title = ?1, gitlab_url = ?2 WHERE id = ?3",
                        (&item.title, &item.web_url, id),
                    )?;
                    changed += 1;
                }
                Some(_) => {}
            }
        }
        Ok(changed)
    })
}

/// Call after a GitHub round, with only the repos that loaded successfully
/// (`(lowercase repo key, its issues)`). Ticks (`ticked_by = 'source'`) every
/// open live GitHub entry of such a repo that is missing from that repo's
/// issues, and refreshes the stored title and URL of the ones still present.
/// An entry of a repo not in `loaded` (it failed this round, or it is no longer
/// in settings) is left alone: nothing is known about it. Own and GitLab
/// entries are never touched. A stored URL is kept when the fresh one was
/// dropped as foreign (for example after a repo rename), so a link never
/// disappears.
///
/// Single-account assumption: unlike GitLab, no account is recorded per entry
/// (out of scope for S-05). The list is "assigned to `@me`" for whichever
/// account `gh` uses now, so after `gh auth switch` to another account the
/// entries it is not assigned to are ticked as closed in the source. Switching
/// back does not untick them.
/// Returns the number of entries changed (roll-over closings not counted).
pub fn reconcile_github(
    store: &mut Store,
    loaded: &[(String, &[GithubIssue])],
    today: NaiveDate,
    now: NaiveDateTime,
) -> Result<usize, TodoError> {
    store.write(|tx| {
        roll_over_in(tx, today)?;
        let repos: HashMap<String, HashMap<&str, &GithubIssue>> = loaded
            .iter()
            .map(|(key, issues)| {
                (
                    key.to_lowercase(),
                    issues.iter().map(|i| (i.reference.as_str(), i)).collect(),
                )
            })
            .collect();

        let mut stmt = tx.prepare(
            "SELECT id, github_ref, github_title, github_url FROM todo_items \
             WHERE source = 'github' AND removed_at IS NULL AND closed_at IS NULL \
               AND ticked_at IS NULL",
        )?;
        let open = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        let ts = stamp(now);
        let mut changed = 0;
        for (id, reference, title, url) in open {
            let Some((repo, _)) = reference.rsplit_once('#') else {
                continue;
            };
            let Some(issues) = repos.get(&repo.to_lowercase()) else {
                continue;
            };
            match issues.get(reference.as_str()) {
                None => {
                    tx.execute(
                        "UPDATE todo_items SET ticked_at = ?1, ticked_by = 'source' WHERE id = ?2",
                        (&ts, id),
                    )?;
                    changed += 1;
                }
                Some(issue) => {
                    let fresh_url = issue.web_url.as_ref().or(url.as_ref());
                    if issue.title != title || fresh_url != url.as_ref() {
                        tx.execute(
                            "UPDATE todo_items SET github_title = ?1, github_url = ?2 WHERE id = ?3",
                            (&issue.title, fresh_url, id),
                        )?;
                        changed += 1;
                    }
                }
            }
        }
        Ok(changed)
    })
}
