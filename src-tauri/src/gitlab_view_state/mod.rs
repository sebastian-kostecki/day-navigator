//! The remembered type choice and "Ukryte" expansion of the GitLab view: one
//! fixed row in the app data store. Filter values are work data and never
//! stored.

pub mod commands;
pub mod error;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

pub use error::GitlabViewStateError;

use crate::store::{Store, StoreError};

/// The type choice. Mirrors `KindFilter` in `src/gitlab/types.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViewKind {
    #[default]
    All,
    Issue,
    MergeRequest,
}

impl ViewKind {
    pub fn as_db(self) -> &'static str {
        match self {
            ViewKind::All => "all",
            ViewKind::Issue => "issue",
            ViewKind::MergeRequest => "mergeRequest",
        }
    }

    pub fn from_db(text: &str) -> Option<ViewKind> {
        match text {
            "all" => Some(ViewKind::All),
            "issue" => Some(ViewKind::Issue),
            "mergeRequest" => Some(ViewKind::MergeRequest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabViewState {
    pub kind: ViewKind,
    pub show_hidden: bool,
}

/// Parses the two columns; one fixed reason, never the value.
fn from_row(kind: &str, show_hidden: i64) -> Result<GitlabViewState, GitlabViewStateError> {
    let invalid = || {
        GitlabViewStateError::Store(StoreError::Sql {
            reason: "invalid view state row".into(),
        })
    };
    let kind = ViewKind::from_db(kind).ok_or_else(invalid)?;
    let show_hidden = match show_hidden {
        0 => false,
        1 => true,
        _ => return Err(invalid()),
    };
    Ok(GitlabViewState { kind, show_hidden })
}

/// Reads the saved state; no row gives the defaults.
pub fn load(store: &Store) -> Result<GitlabViewState, GitlabViewStateError> {
    let row: Option<(String, i64)> = store.read::<_, GitlabViewStateError>(|c| {
        Ok(c.query_row(
            "SELECT kind, show_hidden FROM gitlab_view_state WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
    })?;
    match row {
        Some((kind, show_hidden)) => from_row(&kind, show_hidden),
        None => Ok(GitlabViewState::default()),
    }
}

/// Saves the state in one write (upsert of the single row).
pub fn save(store: &mut Store, view: GitlabViewState) -> Result<(), GitlabViewStateError> {
    store.write::<_, GitlabViewStateError>(|tx| {
        tx.execute(
            "INSERT INTO gitlab_view_state (id, kind, show_hidden) VALUES (1, ?1, ?2)
             ON CONFLICT (id) DO UPDATE SET kind = excluded.kind, show_hidden = excluded.show_hidden",
            rusqlite::params![view.kind.as_db(), view.show_hidden as i64],
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [(ViewKind, &str); 3] = [
        (ViewKind::All, "all"),
        (ViewKind::Issue, "issue"),
        (ViewKind::MergeRequest, "mergeRequest"),
    ];

    #[test]
    fn serde_shape_for_each_kind() {
        for (kind, text) in KINDS {
            for show_hidden in [false, true] {
                let state = GitlabViewState { kind, show_hidden };
                let json = format!(r#"{{"kind":"{text}","showHidden":{show_hidden}}}"#);
                assert_eq!(serde_json::to_string(&state).unwrap(), json);
                assert_eq!(
                    serde_json::from_str::<GitlabViewState>(&json).unwrap(),
                    state
                );
            }
        }
    }

    #[test]
    fn serde_refuses_an_unknown_kind() {
        assert!(
            serde_json::from_str::<GitlabViewState>(r#"{"kind":"epic","showHidden":false}"#)
                .is_err()
        );
    }

    #[test]
    fn error_shapes() {
        let busy = GitlabViewStateError::Store(StoreError::Busy);
        assert_eq!(
            serde_json::to_string(&busy).unwrap(),
            r#"{"kind":"Store","detail":{"kind":"Busy"}}"#
        );
        assert_eq!(
            serde_json::to_string(&GitlabViewStateError::Background).unwrap(),
            r#"{"kind":"Background"}"#
        );
    }

    #[test]
    fn db_text_round_trips() {
        for (kind, text) in KINDS {
            assert_eq!(kind.as_db(), text);
            assert_eq!(ViewKind::from_db(text), Some(kind));
            for (n, on) in [(0, false), (1, true)] {
                assert_eq!(
                    from_row(text, n).unwrap(),
                    GitlabViewState {
                        kind,
                        show_hidden: on
                    }
                );
            }
        }
    }

    #[test]
    fn invalid_rows_give_one_fixed_error() {
        for (kind, n) in [("epic", 0), ("all", 2)] {
            match from_row(kind, n) {
                Err(GitlabViewStateError::Store(StoreError::Sql { reason })) => {
                    assert_eq!(reason, "invalid view state row");
                }
                other => panic!("unexpected: {other:?}"),
            }
        }
    }
}
