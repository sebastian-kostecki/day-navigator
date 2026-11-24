<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Etykiety zgłoszeń GitHub

- **Plan**: context/changes/github-issue-labels/plan.md
- **Scope**: Full plan
- **Reviewed phases**: 1, 2
- **Date**: 2026-11-24
- **Verdict**: APPROVED
- **Findings**: 0 critical, 0 warnings, 3 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | PASS |
| Scope Discipline | PASS |
| Safety & Quality | PASS |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | PASS |

## Evidence

- Range: `46bacfe..HEAD` (2db6c3f p1, a0a9534 p2, 2003643 epilogue, 3f02e4d manual ticks). Every changed code file is in the plan; no EXTRA files.
- Drift check: 13/13 planned changes MATCH (Faza 1 changes 1–8, Faza 2 changes 1–5). Nothing from "What We're NOT Doing" appears in the diff.
- Automated criteria, re-run on HEAD:
  - `cargo test`: 534 passed, 0 failed.
  - `cargo fmt --check` OK; `cargo clippy --all-targets -- -D warnings` OK.
  - `npm run build`, `npm run lint` and `npm run format:check` OK.
  - The old `--json` field list (without `labels`) is absent from src-tauri (`rg` exit 1).
  - `dn-row-labels` appears only in ItemRow.tsx, ItemRow.css and GithubView.css.
  - The F-05 Outcome line names `ItemRow` labels.
- Manual criteria 2.4–2.8: confirmed by the user in the running app on 2026-11-24 (commit 3f02e4d).
- Safety and quality:
  - The `gh` call keeps fixed args and stays read-only.
  - Labels are rendered only as React text and a `title` attribute.
  - Label names never reach log lines (`github_refresh.rs`), tray status (`health::github_status`), error text or SQLite (`todo::add_github` stores reference, title and URL only). `tests/github_fetch.rs` asserts this with `LABEL_MARKER`.
  - Output is bounded by `STDOUT_CAP` and `--limit 500`.
- `ItemRow` without `labels` renders as before; no caller spreads props into it. 11 px on `.dn-row-labels` wins over `.code` (12 px) by load order: tokens.css is imported in main.tsx before App.

## Findings

### F1 — CSS widoku GitHuba ukrywa wewnętrzną klasę ItemRow

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src/github/GithubView.css:13-17
- **Detail**: `.github-repos .dn-row-labels { display: none }` reaches into a class inside the component, while GitLab hides only its own `gitlab-col-labels`. The plan chose this on purpose, and hiding at a breakpoint is layout, so AGENTS.md is respected.
- **Fix**: Leave as is. If a second view needs the same, add a prop or modifier class to `ItemRow`.
- **Decision**: ACCEPTED — left as planned; revisit only if a second view needs to hide labels

### F2 — Klucz `labels` sprawdza inny test, niż zapisano w planie

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/github/mod.rs:429
- **Detail**: The plan said the snapshot serialization test would check the `labels` key. A new test, `issue_serializes_its_labels`, checks it instead. The snapshot test builds only an errored repo with no issues, so it could not have checked it. The JSON contract is covered.
- **Fix**: No code change; already recorded as an adaptation in the implementation run report.
- **Decision**: ACCEPTED — no code change; the JSON contract is covered by issue_serializes_its_labels

### F3 — Marker GH_LABEL_MARKER nie ma komentarza dokumentującego

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Pattern Consistency
- **Location**: src-tauri/src/bin/fake_cli.rs:30
- **Detail**: The neighbouring `GH_TITLE_MARKER` has a `///` comment saying what the marker is for; the new label marker has none. Cosmetic.
- **Fix**: Add a one-line `///` comment in the style of `GH_TITLE_MARKER`.
- **Decision**: FIXED — /// comment added in src-tauri/src/bin/fake_cli.rs
