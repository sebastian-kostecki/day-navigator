//! Hide records of the source-neutral table `hidden_items` (migration v15):
//! GitHub issues now, GitLab merge requests later. Key: source, scope (the
//! lowercase `owner/name` repo key for GitHub), kind and item id. Every
//! mutation is one `Store::write`. A record holds ids, a status fingerprint and
//! instants only: no title, URL, reference, comment text or login. Errors never
//! carry a row value.

use rusqlite::{Connection, Row};

use super::rules::{PRUNE_AFTER_MS, SETTLE_MS, Transition};
use super::{ChangeReason, HiddenIssueError, corrupt, reason_text};
use crate::store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenSource {
    Gitlab,
    Github,
}

impl HiddenSource {
    fn text(self) -> &'static str {
        match self {
            HiddenSource::Gitlab => "gitlab",
            HiddenSource::Github => "github",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "gitlab" => Some(HiddenSource::Gitlab),
            "github" => Some(HiddenSource::Github),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenKind {
    Issue,
    MergeRequest,
}

impl HiddenKind {
    fn text(self) -> &'static str {
        match self {
            HiddenKind::Issue => "issue",
            HiddenKind::MergeRequest => "mergeRequest",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "issue" => Some(HiddenKind::Issue),
            "mergeRequest" => Some(HiddenKind::MergeRequest),
            _ => None,
        }
    }
}

/// A stored row. `changed_reason == None` means hidden; a reason means the item
/// is visible and carries a mark.
#[derive(Clone, PartialEq, Eq)]
pub struct HiddenItemRecord {
    pub source: HiddenSource,
    pub scope: String,
    pub kind: HiddenKind,
    pub item_id: u64,
    pub project_id: Option<u64>,
    pub iid: Option<u64>,
    pub status_fingerprint: Vec<String>,
    pub baseline_updated_at: String,
    pub settle_after_ms: Option<i64>,
    pub hidden_at_ms: i64,
    pub absent_since_ms: Option<i64>,
    pub changed_reason: Option<ChangeReason>,
    pub changed_at_ms: Option<i64>,
}

/// The scope (a repo key) is work data: it is not printed.
impl std::fmt::Debug for HiddenItemRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HiddenItemRecord")
            .field("source", &self.source)
            .field("kind", &self.kind)
            .field("hidden_at_ms", &self.hidden_at_ms)
            .field("changed_reason", &self.changed_reason)
            .finish_non_exhaustive()
    }
}

/// What `hide_item` stores. `project_id` and `iid` are required for `Gitlab`
/// and forbidden for `Github` (the table refuses anything else).
#[derive(Clone, PartialEq, Eq)]
pub struct NewHiddenItem {
    pub source: HiddenSource,
    pub scope: String,
    pub kind: HiddenKind,
    pub item_id: u64,
    pub project_id: Option<u64>,
    pub iid: Option<u64>,
    pub status_fingerprint: Vec<String>,
    pub baseline_updated_at: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HiddenItemKey {
    pub source: HiddenSource,
    pub scope: String,
    pub kind: HiddenKind,
    pub item_id: u64,
}

const COLUMNS: &str = "source, scope, kind, item_id, project_id, iid, status_fingerprint, \
     baseline_updated_at, settle_after_ms, hidden_at_ms, absent_since_ms, changed_reason, \
     changed_at_ms";

/// Reads one row; `None` when a column holds something the rules cannot use.
fn read_row(r: &Row<'_>) -> rusqlite::Result<Option<HiddenItemRecord>> {
    let source: String = r.get(0)?;
    let kind: String = r.get(2)?;
    let fingerprint: String = r.get(6)?;
    let reason: Option<String> = r.get(11)?;
    let changed_reason = match reason.as_deref() {
        None => None,
        Some("status") => Some(ChangeReason::Status),
        Some("comment") => Some(ChangeReason::Comment),
        Some("returned") => Some(ChangeReason::Returned),
        Some(_) => return Ok(None),
    };
    let (Some(source), Some(kind), Ok(status_fingerprint)) = (
        HiddenSource::parse(&source),
        HiddenKind::parse(&kind),
        serde_json::from_str::<Vec<String>>(&fingerprint),
    ) else {
        return Ok(None);
    };
    Ok(Some(HiddenItemRecord {
        source,
        scope: r.get(1)?,
        kind,
        item_id: r.get::<_, i64>(3)? as u64,
        project_id: r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
        iid: r.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        status_fingerprint,
        baseline_updated_at: r.get(7)?,
        settle_after_ms: r.get(8)?,
        hidden_at_ms: r.get(9)?,
        absent_since_ms: r.get(10)?,
        changed_reason,
        changed_at_ms: r.get(12)?,
    }))
}

fn records(
    conn: &Connection,
    source: HiddenSource,
    kind: HiddenKind,
) -> Result<Vec<HiddenItemRecord>, HiddenIssueError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM hidden_items WHERE source = ?1 AND kind = ?2 \
         ORDER BY scope, item_id"
    ))?;
    let rows = stmt.query_map([source.text(), kind.text()], read_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?.ok_or_else(corrupt)?);
    }
    Ok(out)
}

/// The rows of one source and kind: hidden ones and ones carrying a mark.
pub fn list_items(
    store: &Store,
    source: HiddenSource,
    kind: HiddenKind,
) -> Result<Vec<HiddenItemRecord>, HiddenIssueError> {
    store.read(|c| records(c, source, kind))
}

/// Hides an item with a fresh snapshot; replaces a marked record. One settling
/// comment check is scheduled.
pub fn hide_item(
    store: &mut Store,
    item: &NewHiddenItem,
    now_ms: i64,
) -> Result<(), HiddenIssueError> {
    let fingerprint = serde_json::to_string(&item.status_fingerprint).map_err(|_| corrupt())?;
    store.write(|tx| {
        tx.execute(
            "INSERT INTO hidden_items(source, scope, kind, item_id, project_id, iid, \
                 status_fingerprint, baseline_updated_at, settle_after_ms, hidden_at_ms, \
                 absent_since_ms, changed_reason, changed_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL, NULL) \
             ON CONFLICT(source, scope, kind, item_id) DO UPDATE SET \
                 project_id = excluded.project_id, iid = excluded.iid, \
                 status_fingerprint = excluded.status_fingerprint, \
                 baseline_updated_at = excluded.baseline_updated_at, \
                 settle_after_ms = excluded.settle_after_ms, \
                 hidden_at_ms = excluded.hidden_at_ms, absent_since_ms = NULL, \
                 changed_reason = NULL, changed_at_ms = NULL",
            (
                item.source.text(),
                &item.scope,
                item.kind.text(),
                item.item_id as i64,
                item.project_id.map(|v| v as i64),
                item.iid.map(|v| v as i64),
                &fingerprint,
                &item.baseline_updated_at,
                now_ms + SETTLE_MS,
                now_ms,
            ),
        )?;
        Ok(())
    })
}

fn delete_where(
    store: &mut Store,
    key: &HiddenItemKey,
    marked: bool,
) -> Result<(), HiddenIssueError> {
    store.write(|tx| {
        let sql = if marked {
            "DELETE FROM hidden_items WHERE source = ?1 AND scope = ?2 AND kind = ?3 \
             AND item_id = ?4 AND changed_reason IS NOT NULL"
        } else {
            "DELETE FROM hidden_items WHERE source = ?1 AND scope = ?2 AND kind = ?3 \
             AND item_id = ?4 AND changed_reason IS NULL"
        };
        let n = tx.execute(
            sql,
            (
                key.source.text(),
                &key.scope,
                key.kind.text(),
                key.item_id as i64,
            ),
        )?;
        if n == 0 {
            return Err(HiddenIssueError::NotFound);
        }
        Ok(())
    })
}

/// Deletes a hidden record; anything else is `NotFound`.
pub fn unhide_item(store: &mut Store, key: &HiddenItemKey) -> Result<(), HiddenIssueError> {
    delete_where(store, key, false)
}

/// Deletes a marked record; anything else is `NotFound`.
pub fn dismiss_item(store: &mut Store, key: &HiddenItemKey) -> Result<(), HiddenIssueError> {
    delete_where(store, key, true)
}

/// Applies a round's transitions in one write. `planned` holds the transitions
/// per scope. Each transition is applied only if its row is still in the state
/// the plan assumed (hidden with the same `hidden_at_ms`, absent or not
/// absent); otherwise it is skipped. Also deletes the rows of `(source, kind)`
/// whose scope is not in `active_scopes` and that were hidden `PRUNE_AFTER_MS`
/// or more ago (their absence is never judged). Returns every `(source, kind)`
/// row as it is after the write.
pub fn apply_items_round(
    store: &mut Store,
    source: HiddenSource,
    kind: HiddenKind,
    planned: &[(String, Vec<Transition>)],
    active_scopes: &[String],
    now_ms: i64,
) -> Result<Vec<HiddenItemRecord>, HiddenIssueError> {
    let (s, k) = (source.text(), kind.text());
    store.write(|tx| {
        for (scope, transitions) in planned {
            for t in transitions {
                match t {
                    Transition::Resurface {
                        issue_id,
                        hidden_at_ms,
                        reason,
                    } => tx.execute(
                        "UPDATE hidden_items SET changed_reason = ?1, changed_at_ms = ?2, \
                             absent_since_ms = NULL, settle_after_ms = NULL \
                         WHERE source = ?3 AND scope = ?4 AND kind = ?5 AND item_id = ?6 \
                             AND changed_reason IS NULL AND hidden_at_ms = ?7",
                        (
                            reason_text(*reason),
                            now_ms,
                            s,
                            scope,
                            k,
                            *issue_id as i64,
                            *hidden_at_ms,
                        ),
                    )?,
                    Transition::AdvanceBaseline {
                        issue_id,
                        hidden_at_ms,
                        updated_at,
                        settle_after_ms,
                    } => tx.execute(
                        "UPDATE hidden_items SET baseline_updated_at = ?1, settle_after_ms = ?2 \
                         WHERE source = ?3 AND scope = ?4 AND kind = ?5 AND item_id = ?6 \
                             AND changed_reason IS NULL AND hidden_at_ms = ?7",
                        (
                            updated_at,
                            settle_after_ms,
                            s,
                            scope,
                            k,
                            *issue_id as i64,
                            *hidden_at_ms,
                        ),
                    )?,
                    Transition::MarkAbsent { issue_id } => tx.execute(
                        "UPDATE hidden_items SET absent_since_ms = ?1 \
                         WHERE source = ?2 AND scope = ?3 AND kind = ?4 AND item_id = ?5 \
                             AND absent_since_ms IS NULL",
                        (now_ms, s, scope, k, *issue_id as i64),
                    )?,
                    Transition::Prune { issue_id } => tx.execute(
                        "DELETE FROM hidden_items \
                         WHERE source = ?1 AND scope = ?2 AND kind = ?3 AND item_id = ?4 \
                             AND absent_since_ms IS NOT NULL AND ?5 - absent_since_ms >= ?6",
                        (s, scope, k, *issue_id as i64, now_ms, PRUNE_AFTER_MS),
                    )?,
                    Transition::ClearAbsent { issue_id } => tx.execute(
                        "UPDATE hidden_items SET absent_since_ms = NULL \
                         WHERE source = ?1 AND scope = ?2 AND kind = ?3 AND item_id = ?4 \
                             AND absent_since_ms IS NOT NULL",
                        (s, scope, k, *issue_id as i64),
                    )?,
                };
            }
        }
        let stale: Vec<(String, i64)> = {
            let mut stmt = tx.prepare(
                "SELECT scope, item_id FROM hidden_items \
                 WHERE source = ?1 AND kind = ?2 AND ?3 - hidden_at_ms >= ?4",
            )?;
            let rows = stmt.query_map((s, k, now_ms, PRUNE_AFTER_MS), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            out
        };
        for (scope, item_id) in stale {
            if active_scopes.contains(&scope) {
                continue;
            }
            tx.execute(
                "DELETE FROM hidden_items \
                 WHERE source = ?1 AND scope = ?2 AND kind = ?3 AND item_id = ?4",
                (s, &scope, k, item_id),
            )?;
        }
        records(tx, source, kind)
    })
}
