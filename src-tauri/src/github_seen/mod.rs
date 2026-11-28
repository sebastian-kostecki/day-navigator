//! Seen GitHub issues (S-03): the durable seen set behind the NEW flag of the
//! GitHub view. No `tauri` import and no clock: `now_ms` is a parameter. A
//! round is one `Store::write`. Rows hold the lowercase repo key, issue numbers
//! and instants only. No toast, no event log.

pub mod error;
pub mod rules;

pub use error::GithubSeenError;
pub use rules::{FORGET_AFTER_MS, RepoPlan, SeenKey, StoredIssue, plan_repo};
