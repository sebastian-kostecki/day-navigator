# Hide and resurface GitHub issues (S-01) — Plan Brief

> Full plan: `context/changes/github-hide-resurface/plan.md`
> Research: `context/changes/github-hide-resurface/research.md`

## What & Why

In "Projekty (GitHub)" the user can hide an issue and show it again; a hidden issue comes back marked "Zmienione" when it leaves the open list and returns, or when someone other than the user comments. This is the GitLab behaviour behind "hidden issues do not clutter the list" (FR-003, FR-005, MS-31), and the M-4 north star: GitHub and GitLab should behave and look alike.

## Starting Point

GitLab hiding is complete (table `hidden_issues`, rules, a two-half round, commands, the "Pokaż ukryte (n)" toggle with dimmed "ukryte DD.MM" rows). GitHub has one `gh issue list` per repo and no hiding machinery; F-01 left the gate and kept list to this slice.

## Desired End State

GitHub rows have "Ukryj do zmiany", "Pokaż na liście" and "OK, zauważone"; the view head has "Pokaż ukryte (n)"; hidden rows are dimmed after the visible ones with "ukryte DD.MM"; returned issues carry "Zmienione" (tooltip "nowy komentarz" / "wróciło na listę"), also in "Do zrobienia". Records sit in a new source-neutral table; GitLab is untouched.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| User's login | new read-only `gh api user` (`CliCall::GhUser`, in `ALL`) | User's decision; `ALL` puts it in the real-CLI smoke test | User / Plan |
| What resurfaces | other person's comment, or leaving and re-joining the open list | User's decision (FR-003), same as GitLab; labels, edits, reactions never | User |
| Look | mirror GitLab: one view toggle "Pokaż ukryte (n)", dimmed rows with "ukryte DD.MM" | GitLab has no separate "Ukryte (n)" section; "(n)" is the toggle count | User / Research |
| Rules sharing (team Q1) | one copy: traits `HideItem`, `HideRecord`, struct `Comment`, `user: Option<&str>` | No copy; GitLab signatures and `HiddenRecord` fields stay | Research / Plan |
| Storage (team Q2) | option A: new `hidden_items` keyed `(source, scope, kind, item_id)`; GitLab issues stay in `hidden_issues` | S-07 needs no migration; no GitLab SQL change while S-06 runs in parallel | Plan |
| Comments call | `gh issue view <N> -R <repo> --json comments` | All comments (the list call stops at 100); nothing else is needed | Research / Plan |
| `gh api user` fails | every due check of the round counts as failed | Never resurface on the user's own comment | Plan (agent default) |
| Ghost / missing author | counts as someone else | Not the user; a parse failure would block checks | Plan (agent default) |
| Failed repo | its records are left out of the round | Otherwise they turn absent and later falsely "returned" | Research |
| Settle check | kept (`SETTLE_MS`, one extra call per hide) | Shared rules; GitHub throttling unverified | Plan (agent default) |
| Command key | `reference` (`owner/name#N`) | Existing stable key, same as "To do" | Plan (agent default) |
| Close + reopen inside one interval | not detected (no `stateReason`) | Same as GitLab; list arguments unchanged | Plan (agent default) |
| Login caching | none; one call per round with a due check | A cache goes stale after `gh auth switch` | Plan (agent default) |

## Scope

**In scope:** two `gh` calls with guard shapes and fake routes; migration v15 `hidden_items`; generic rules; GitHub round, gate, kept list, three commands; GitHub view rows, toggle, flags, notices; "zmienione" in "Do zrobienia"; `AGENTS.md` app-data line.

**Out of scope:** GitLab MR hiding (S-07), moving GitLab rows, any GitLab code change, GitHub refresh interval (S-02), GitHub NEW flags (S-03), a remembered GitHub toggle, `stateReason`, CSP changes.

## Architecture / Approach

The list fetch stays as it is. When a round succeeds, `check_github_round` (no lock) runs `gh api user` once and `gh issue view` for each hidden issue whose `updatedAt` moved or whose settle check is due; then, under a new `hidden_gate`, `commit_github_round` plans transitions per loaded repo with the shared rules, applies them in one store write, and partitions each repo into visible and hidden issues. Commands act on the kept full list under the same gate and publish after it.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. `gh` calls, guard, fake routes, parsers | `GhUser`, `GhIssueComments`, parsers that never read comment text | Guard counts conflict with S-06 (textual) |
| 2. Migration v15, store, shared rules | `hidden_items`, store ops, generic rules, `AGENTS.md` | GitLab behaviour must stay identical |
| 3. GitHub round and snapshot fields | check/commit/partition, TS mirror | Failed repos and login failures handled safely |
| 4. Refresh wiring and commands | gate, kept list, round in `refresh_round`, 3 commands | Overlaps S-02 in `refresh_round` |
| 5. UI | rows, toggle, flags, notices, `ItemRow` props, "Do zrobienia" flag | Overlaps S-02 in `GithubView.tsx` |

**Prerequisites:** F-01 merged (done, develop 00e6a19); a logged-in `gh` for the manual checks.
**Estimated effort:** ~5 implementer sessions, one per phase.

## Open Risks & Assumptions

- `gh` renders a deleted author as `null` or an empty login (unverified); the parser treats both, and a missing field, as someone else.
- GitHub `updatedAt` throttling is unverified; the settle check covers a short delay.
- A close and reopen inside one refresh interval without a comment by someone else is not detected (same as GitLab).
- Merge points with S-02 (`github_refresh.rs`, `GithubSnapshot`, `GithubView.tsx`) and S-06 (`cli/mod.rs`, `fake_cli.rs`) are textual; the plan's table says how to resolve each.

## Success Criteria (Summary)

- Hiding, showing and dismissing a GitHub issue works from its row, and hidden issues come back only on another person's comment or a return to the open list.
- GitLab hiding behaves exactly as before; no repo name, login, title or comment text reaches a log or an error.
- All gates are green after every phase; the user's manual checks in the built app pass.
