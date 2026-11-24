//! The production migration list and its validator.
//!
//! A shipped migration is never edited, only followed by a new one.

use super::Migration;
use super::error::StoreError;

/// Production migrations, oldest first. Each data slice appends its own.
pub const MIGRATIONS: &[Migration] = &[
    // S-08: work-time entries. Instants are UTC epoch milliseconds; a NULL end
    // is the running entry; removal is soft (`removed_at_ms`).
    Migration {
        version: 1,
        sql: "CREATE TABLE work_entries (
  id            INTEGER PRIMARY KEY,
  start_ms      INTEGER NOT NULL,
  end_ms        INTEGER,
  removed_at_ms INTEGER,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  CHECK (end_ms IS NULL OR end_ms > start_ms)
);
CREATE UNIQUE INDEX work_entries_one_running
  ON work_entries ((end_ms IS NULL)) WHERE end_ms IS NULL AND removed_at_ms IS NULL;
CREATE INDEX work_entries_start ON work_entries (start_ms);",
    },
    // S-06: own tasks. `done_at` is the single "done" state S-07 reuses.
    Migration {
        version: 2,
        sql: "CREATE TABLE own_tasks (
  id         INTEGER PRIMARY KEY,
  space      TEXT NOT NULL CHECK (space IN ('work','private')),
  title      TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
  note       TEXT NOT NULL DEFAULT '' CHECK (length(note) <= 2000),
  due        TEXT,
  priority   TEXT CHECK (priority IN ('high','medium','low')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  done_at    TEXT
);",
    },
    // S-07: "To do" picks. One row per pick; own entries take their ticked
    // state from `own_tasks.done_at`, GitLab entries keep their own tick and a
    // reference/title snapshot. `closed_at` is the day roll-over, `removed_at`
    // the user's removal; at most one live row per item.
    Migration {
        version: 3,
        sql: "CREATE TABLE todo_items (
  id           INTEGER PRIMARY KEY,
  source       TEXT NOT NULL CHECK (source IN ('own','gitlab')),
  own_task_id  INTEGER REFERENCES own_tasks(id),
  gitlab_kind  TEXT CHECK (gitlab_kind IN ('issue','mergeRequest')),
  gitlab_ref   TEXT,
  gitlab_title TEXT,
  gitlab_url   TEXT,
  picked_on    TEXT NOT NULL,
  picked_at    TEXT NOT NULL,
  ticked_at    TEXT,
  ticked_by    TEXT CHECK (ticked_by IN ('user','source')),
  removed_at   TEXT,
  closed_at    TEXT,
  CHECK (
    (source = 'own' AND own_task_id IS NOT NULL
      AND gitlab_kind IS NULL AND gitlab_ref IS NULL
      AND gitlab_title IS NULL AND gitlab_url IS NULL
      AND ticked_at IS NULL AND ticked_by IS NULL)
    OR
    (source = 'gitlab' AND own_task_id IS NULL
      AND gitlab_kind IS NOT NULL AND gitlab_ref IS NOT NULL
      AND gitlab_title IS NOT NULL
      AND ((ticked_at IS NULL) = (ticked_by IS NULL)))
  )
);
CREATE UNIQUE INDEX todo_items_live_own
  ON todo_items (own_task_id)
  WHERE own_task_id IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL;
CREATE UNIQUE INDEX todo_items_live_gitlab
  ON todo_items (gitlab_kind, gitlab_ref)
  WHERE gitlab_ref IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL;",
    },
    // S-07 review fix: the GitLab account a GitLab entry was picked under, as
    // the opaque key from `gitlab::account_key` (a hash, never a hostname or a
    // login). Reconcile touches only entries of the account its list came
    // from. NULL on own entries and on GitLab entries picked before v4, which
    // any account's list reconciles.
    Migration {
        version: 4,
        sql: "ALTER TABLE todo_items ADD COLUMN gitlab_account TEXT;",
    },
    // S-09: days off. One row per local date (`YYYY-MM-DD`); un-marking deletes
    // the row. A flag, not a user record. (Planned as v3; renumbered to v5 when
    // merged after S-07's v3/v4.)
    Migration {
        version: 5,
        sql: "CREATE TABLE days_off (
  day           TEXT PRIMARY KEY
                CHECK (day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
  created_at_ms INTEGER NOT NULL
) WITHOUT ROWID;",
    },
    // S-03: hidden GitLab issues. No titles, URLs, references or comment text:
    // ids, scoped labels and timestamps only. A NULL `changed_reason` is a
    // hidden issue; a reason is a visible issue carrying a "changed" mark.
    // (Planned as v3; renumbered to v6 when merged after S-07's v3/v4 and
    // S-09's v5.)
    Migration {
        version: 6,
        sql: "CREATE TABLE hidden_issues (
  host                TEXT    NOT NULL,
  issue_id            INTEGER NOT NULL,
  project_id          INTEGER NOT NULL,
  iid                 INTEGER NOT NULL,
  scoped_labels       TEXT    NOT NULL,
  baseline_updated_at TEXT    NOT NULL,
  settle_after_ms     INTEGER,
  hidden_at_ms        INTEGER NOT NULL,
  absent_since_ms     INTEGER,
  changed_reason      TEXT CHECK (changed_reason IN ('status','comment','returned')),
  changed_at_ms       INTEGER,
  CHECK ((changed_reason IS NULL) = (changed_at_ms IS NULL)),
  PRIMARY KEY (host, issue_id)
);",
    },
    // S-04: seen GitLab assignments, for new-assignment notifications. The
    // account is the opaque key from `gitlab::account_key` (a hash, never a
    // hostname or a login). A row holds kind, GitLab's numeric id and two
    // instants: no title, reference or URL. `assignment_accounts` is a per-
    // account seeding marker that is never pruned, so an account whose list
    // was empty for a long time still notifies its next item. (Shipped as v7.)
    Migration {
        version: 7,
        sql: "CREATE TABLE assignment_accounts (
  account      TEXT PRIMARY KEY,
  seeded_at_ms INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE seen_assignments (
  account       TEXT    NOT NULL REFERENCES assignment_accounts(account),
  kind          TEXT    NOT NULL CHECK (kind IN ('issue','mergeRequest')),
  item_id       INTEGER NOT NULL CHECK (item_id > 0),
  first_seen_ms INTEGER NOT NULL,
  last_seen_ms  INTEGER NOT NULL,
  PRIMARY KEY (account, kind, item_id)
) WITHOUT ROWID;",
    },
    // S-05: GitHub entries in "To do". SQLite cannot alter a CHECK, so the
    // table is rebuilt: same columns (copied by name, `gitlab_account`
    // included) plus `github_ref` (lowercase `owner/name#N`), `github_title`
    // and `github_url`, with `source` and the row-shape CHECK widened. Runs in
    // the migration's single transaction. Planned as v7, renumbered to v8 on
    // merge after S-04 shipped v7.
    Migration {
        version: 8,
        sql: "CREATE TABLE todo_items_v8 (
  id             INTEGER PRIMARY KEY,
  source         TEXT NOT NULL CHECK (source IN ('own','gitlab','github')),
  own_task_id    INTEGER REFERENCES own_tasks(id),
  gitlab_kind    TEXT CHECK (gitlab_kind IN ('issue','mergeRequest')),
  gitlab_ref     TEXT,
  gitlab_title   TEXT,
  gitlab_url     TEXT,
  gitlab_account TEXT,
  github_ref     TEXT,
  github_title   TEXT,
  github_url     TEXT,
  picked_on      TEXT NOT NULL,
  picked_at      TEXT NOT NULL,
  ticked_at      TEXT,
  ticked_by      TEXT CHECK (ticked_by IN ('user','source')),
  removed_at     TEXT,
  closed_at      TEXT,
  CHECK (
    (source = 'own' AND own_task_id IS NOT NULL
      AND gitlab_kind IS NULL AND gitlab_ref IS NULL
      AND gitlab_title IS NULL AND gitlab_url IS NULL
      AND github_ref IS NULL AND github_title IS NULL AND github_url IS NULL
      AND ticked_at IS NULL AND ticked_by IS NULL)
    OR
    (source = 'gitlab' AND own_task_id IS NULL
      AND gitlab_kind IS NOT NULL AND gitlab_ref IS NOT NULL
      AND gitlab_title IS NOT NULL
      AND github_ref IS NULL AND github_title IS NULL AND github_url IS NULL
      AND ((ticked_at IS NULL) = (ticked_by IS NULL)))
    OR
    (source = 'github' AND own_task_id IS NULL
      AND gitlab_kind IS NULL AND gitlab_ref IS NULL
      AND gitlab_title IS NULL AND gitlab_url IS NULL AND gitlab_account IS NULL
      AND github_ref IS NOT NULL AND github_title IS NOT NULL
      AND ((ticked_at IS NULL) = (ticked_by IS NULL)))
  )
);
INSERT INTO todo_items_v8
  (id, source, own_task_id, gitlab_kind, gitlab_ref, gitlab_title, gitlab_url,
   gitlab_account, picked_on, picked_at, ticked_at, ticked_by, removed_at, closed_at)
SELECT id, source, own_task_id, gitlab_kind, gitlab_ref, gitlab_title, gitlab_url,
   gitlab_account, picked_on, picked_at, ticked_at, ticked_by, removed_at, closed_at
FROM todo_items;
DROP TABLE todo_items;
ALTER TABLE todo_items_v8 RENAME TO todo_items;
CREATE UNIQUE INDEX todo_items_live_own
  ON todo_items (own_task_id)
  WHERE own_task_id IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL;
CREATE UNIQUE INDEX todo_items_live_gitlab
  ON todo_items (gitlab_kind, gitlab_ref)
  WHERE gitlab_ref IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL;
CREATE UNIQUE INDEX todo_items_live_github
  ON todo_items (github_ref)
  WHERE github_ref IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL;",
    },
    // S-02: the GitLab view's seen-until mark, for the NEW flag. `account` is
    // the opaque key from `gitlab::account_key` (a hash, never a hostname or a
    // login); the value is one UTC instant in ms. No title, reference or URL.
    // No REFERENCES to `assignment_accounts`: a mark can exist before the
    // account's first round seeded it.
    Migration {
        version: 9,
        sql: "CREATE TABLE gitlab_view_marks (
  account       TEXT    PRIMARY KEY,
  seen_until_ms INTEGER NOT NULL
) WITHOUT ROWID;",
    },
    // S-08: today's event log. Only the current day's rows are shown; every
    // write deletes past days. Holds only what the UI shows: the item's
    // reference and title, the source key and label, the error kind. No URL,
    // hostname or message text. `at` is local `YYYY-MM-DDTHH:MM:SS`.
    Migration {
        version: 10,
        sql: "CREATE TABLE events (
  id           INTEGER PRIMARY KEY,
  at           TEXT    NOT NULL,
  kind         TEXT    NOT NULL CHECK (kind IN
    ('assigned','ticked','removed','closedInSource',
     'workStarted','workStopped','sourceError','sourceRecovered')),
  item_source  TEXT    CHECK (item_source IN ('own','gitlab','github')),
  item_kind    TEXT    CHECK (item_kind IN ('issue','mergeRequest')),
  reference    TEXT,
  title        TEXT    CHECK (title IS NULL OR length(title) <= 300),
  source_id    TEXT    CHECK (source_id IN
    ('settings','app-data','gitlab','github','reminders')),
  source_label TEXT,
  error_kind   TEXT
);",
    },
];

/// Versions must start at 1 and rise by exactly 1.
pub fn validate(migrations: &[Migration]) -> Result<(), StoreError> {
    for (i, m) in migrations.iter().enumerate() {
        let expected = i as u32 + 1;
        if m.version != expected {
            return Err(StoreError::MigrationFailed {
                version: m.version,
                reason: format!("expected version {expected}"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(version: u32) -> Migration {
        Migration { version, sql: "" }
    }

    #[test]
    fn rejects_gap_duplicate_and_wrong_start() {
        assert!(validate(&[m(1), m(3)]).is_err());
        assert!(validate(&[m(1), m(1)]).is_err());
        assert!(validate(&[m(2), m(3)]).is_err());
        assert!(validate(&[m(0)]).is_err());
    }

    #[test]
    fn accepts_contiguous_and_production_list() {
        assert!(validate(&[]).is_ok());
        assert!(validate(&[m(1), m(2), m(3)]).is_ok());
        assert!(validate(MIGRATIONS).is_ok());
    }
}
