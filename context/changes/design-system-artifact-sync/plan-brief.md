# Design-system artifact sync — Plan Brief

> Full plan: `context/changes/design-system-artifact-sync/plan.md`

## What & Why

The "Day Navigator — Konsola" design-system artifact (linked from `AGENTS.md`) was written before the app and still describes the 2026-11-23 mockup. After S-01–S-08 it is out of date: components are missing, APIs have changed, the rules are stale and the icons are absent. F-05 makes it describe the shipped app again, so UI reviews check against a current source.

## Starting Point

The artifact holds 14 components, Google Fonts, "Brak logo" and a "one flag per row" rule. `develop` (66b9b1f) differs: 18 components (`Chip`, `DataTable`, `Notice` and `ErrorState` are new), local woff2 fonts, the hatch tokens, an app icon and three tray icons, numbered navigation with shortcuts, a START/STOP question, several flags per row, and the S-06 "Przypomnienia" panel and the S-07 "Dane aplikacji" card.

## Desired End State

The live artifact matches `develop`:

- tokens equal `tokens.css`, and the six local fonts are wired;
- `bundle.js`/`bundle.css` are faithful ports of `src/components/`, with the CSS copied verbatim;
- every card has a current README and a live preview with invented data;
- two "Wzorce widoków" cards show S-06 and S-07, and the `AppHeader` card shows the counter with its question;
- an "Ikony aplikacji" asset group holds the four SVGs;
- the README states the current rules.

## Key Decisions Made

| Decision                    | Choice                                                                 | Why (1 sentence)                                                                    |
| --------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Fonts                       | Upload the six woff2 files to `project/fonts/`, list them in `type.fonts` | The artifact renders the app's own fonts, offline, as the app does.                |
| App and tray icons          | Upload the four SVGs as the asset group "Ikony aplikacji", with a README | Real files copied 1:1 instead of a description; agents can see the mark.           |
| S-06/S-07 and the counter   | `page` pattern cards, plus the counter states in the `AppHeader` preview, plus a README section | The bundle keeps exactly `src/components/`, while the patterns stay visible. |
| Flag rule                   | Document several flags in a fixed order (NOWE, ZMIENIONE, DRAFT, W DO ZROBIENIA) | The app does this on purpose (S-02); F-05 documents what ships.              |
| Where edits happen          | A working copy in `context/changes/…/artifact/`, published once         | Edits are reviewable in git, and only one moment touches the live artifact.        |
| Parity checks               | Plain-Node scripts in `checks/` (tokens, CSS verbatim, class names via SSR, denylist) | Repeatable proof without new dependencies.                              |
| Publishing                  | Only after the user's explicit OK; uploads, then files, then the index last | Required by the task and by the artifact type's write order.                    |

## Scope

**In scope:**

- `project/` files of the artifact: `tokens.json`, the README, the bundle (js, css, d.ts), 18 component cards, 2 pattern cards and the Cover text;
- the font files;
- the icon asset group;
- the sync checklist and the check scripts in the change folder.

**Out of scope:**

- Any app code, CSS, Rust, icon or `AGENTS.md` change.
- New tokens or components beyond the app, and a light theme.
- Full-view mockups.
- A Cover redesign.
- Fixing the two findings (`.dn-chip` shared by `Chip`/`SourceChip`; the plain-`<p>` error in `OpenBackupsButton`), which are reported instead.

## Architecture / Approach

The work is pull, edit, verify, then publish. Phase 1 copies the live `project/` files into the change folder and records their hashes. Phases 2–4 edit that copy from the code (`src/components/`, `tokens.css`, the views), checked by scripts that compare tokens, CSS and rendered class names with the app. Phase 5 renders every card locally. After the user says yes, it uploads the SVGs, publishes the changed files (fonts straight from `src/assets/fonts/`) and writes the index last, after a fresh read.

## Phases at a Glance

| Phase                               | What it delivers                                      | Key risk                                                    |
| ----------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------- |
| 1. Working copy and sync checklist  | The local mirror, the hashes and an item-by-item list | Missing an item; mitigated by the coverage check            |
| 2. Tokens, fonts and README         | Current foundations and rules                         | `tokens.json` grammar drops a value silently                |
| 3. Bundle, stylesheet and types     | A faithful port of 18 components                      | Markup drift from the TSX; mitigated by the SSR class check |
| 4. Component cards and view patterns| Current READMEs and previews, plus S-06/S-07 cards    | Work data leaking into examples; mitigated by the denylist check |
| 5. Local render and publish         | The live artifact in sync                             | Overwriting a concurrent edit; mitigated by re-read and hash compare |

**Prerequisites:** F-04 done (it is). The user is signed in with edit rights on the artifact. The push denylist exists at `%USERPROFILE%\.day-navigator\push-denylist.txt`. Node is available.
**Estimated effort:** ~1–2 sessions across 5 phases, with phase 3 the largest.

## Open Risks & Assumptions

- The page generates `tokens.css` and `@font-face` from `tokens.json`. If the fonts don't load live, the fallback is `Segoe UI`/`Consolas`; this is checked in 5.5.
- Previews run React 18 while the app runs React 19. The port uses only `createElement` and basic hooks, and avoids `useId`.
- Someone may edit the artifact between phase 1 and phase 5. Re-reading it and comparing hashes before publishing catches this.

## Success Criteria (Summary)

- Every row of `sync-checklist.md` is done, and every check script passes.
- The live artifact renders every card with the app's fonts and shows the four icons.
- A reader of the README alone gets the rules the app follows today.
