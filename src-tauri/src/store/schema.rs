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
