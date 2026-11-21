---
change_id: work-tasks-from-template
title: Work tasks from template
status: archived
created: 2026-11-20
updated: 2026-11-21
archived_at: 2026-11-21T07:00:01Z
---

## Notes

- 2026-11-21: Phase 2 kept `chrono` (named in plan). `cargo audit` reports RUSTSEC-2026-0543 (unmaintained, no fix) as a warning, not a vulnerability; accepted on user "continue". Revisit if a maintained alternative is wanted.
- 2026-11-21: Impl review (`reviews/impl-review.md`) fixes landed beyond the plan's contract:
  - `set_status` refuses notes outside the tasks folder and the template (`TaskError::NotATask`).
  - `Vault::create_note_in_existing_folder` never recreates a vanished tasks folder.
  - `TaskRow.fieldProblems` reports Priority/Due read errors.
  - `DoneUpdate::NoWrite` is returned for re-selecting the same status.
  - Commands are async (`spawn_blocking`), and `reload()` keeps only the latest response.
  - Titles are NFC-normalised and length-capped.
  - Status and priority duplicates are checked case-insensitively.

<!-- Free-form notes for this change: links, ad-hoc context, decisions that don't belong in research/frame/plan. -->
