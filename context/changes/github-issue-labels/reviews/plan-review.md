<!-- PLAN-REVIEW-REPORT -->
# Plan Review: Etykiety zgłoszeń GitHub

- **Plan**: context/changes/github-issue-labels/plan.md
- **Mode**: Deep
- **Date**: 2026-11-24
- **Verdict**: SOUND
- **Findings**: 0 critical, 1 warning, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | WARNING |
| Blind Spots | PASS |
| Plan Completeness | PASS |

## Grounding

14/14 paths ✓, 9/9 symbols ✓ (gh_assigned_issues_args, GH_ISSUES_TAIL, gh_args, check_read_only, RawIssue, gh_issue_value, .dn-sr-only, .code, --ink-faint), brief↔plan ✓, Progress↔Fazy ✓ (13/13 before fixes).

Also checked, with no finding:
- the test helper `gh_args` builds its arguments from `GH_ISSUES_TAIL`, so there is no fifth copy of the argument list;
- `tokens.css` loads before the component CSS (`src/main.tsx:3-5`), so 11 px on `.dn-row-labels` overrides 12 px from `.code`, as it does in GitLab;
- no TS code builds a `GithubIssue` object;
- only `github_fetch.rs` uses the `gh-issues` scenario.

## Findings

### F1 — Nikt nie przekaże F-05 informacji o nowym polu ItemRow.labels

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architectural Fitness
- **Location**: What We're NOT Doing; brief → Open Risks
- **Detail**: The plan changes the shared `ItemRow` and says the prop goes to F-05 as input, but no plan step records that anywhere. The F-05 Outcome list (`roadmap.md:150`) does not name `ItemRow.labels`, so the design-system artifact would drift from the app.
- **Fix A ⭐ Recommended**: Phase 2 adds "`ItemRow` labels" to the F-05 Outcome list in roadmap.md
  - Strength: the information lands where F-05 takes its scope from; one line.
  - Tradeoff: edits another roadmap item; a small merge conflict is possible.
  - Confidence: HIGH — the F-05 Outcome is exactly the list of artifact↔app drifts.
  - Blind spot: if F-05 is planned before S-04 is implemented, its plan will not have it.
- **Fix B**: Record it only in this change's `change.md` Notes
  - Strength: does not touch another roadmap item.
  - Tradeoff: F-05 does not read this change.md; the information gets lost easily.
  - Confidence: MED — depends on someone remembering at archive time.
  - Blind spot: no one is named to carry the note over.
- **Decision**: FIXED (Fix A). Phase 2 gained change 5 (`context/foundation/roadmap.md`, F-05 Outcome) and criterion 2.9. The brief was updated in Scope, Phases and Open Risks.

### F2 — Założenie o formacie etykiet jest już sprawdzone

- **Severity**: OBSERVATION
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Blind Spots
- **Location**: brief → Open Risks; plan → Key Discoveries
- **Detail**: The brief listed as unverified that `gh` sends labels as objects with `name`. Checked on `gh` 2.105.0 in Windows with a read-only call on the public `cli/cli` repository: the shape is `[{"id","name","description","color"}]`.
- **Fix**: Turn the risk into a fact confirmed on 2026-11-24 in the brief, and add the `gh` version and date to Key Discoveries in the plan.
- **Decision**: FIXED
