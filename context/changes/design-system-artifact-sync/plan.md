# Design-system artifact sync Implementation Plan

## Overview

We bring the "Day Navigator — Konsola" design-system artifact (https://claude.ai/artifact/RAuK11sMerfzyBMr2gkiYY, linked from `AGENTS.md`) in line with the app as it stands on `develop` after S-01–S-08 (roadmap F-05, issue #16, MS-16). The artifact's `project/` files are edited in a reviewable working copy inside this change folder, checked locally, and published once, only after the user's explicit OK. App code does not change; a real UI bug found on the way is reported, not fixed.

## Current State Analysis

The artifact is a "Design System" type artifact kept in files (index `project/design-system.json`, `v: 3`, `layout: files`, namespace `DayNav`, libraries react/react-dom 18, no asset groups, `lastChange` 2026-11-23). It holds `project/README.md`, `project/tokens.json`, `project/components/{bundle.js,bundle.css,index.d.ts}`, a `Cover` preview and 14 component folders (README + preview each). It was written before the app existed and was never updated.

Differences between the artifact and `develop` (66b9b1f):

- **Missing components** (exported from `src/components/index.ts`): `Chip` (`src/components/Chip/Chip.tsx`), `DataTable` + `DataTableRow` + `DataTableCell` (`src/components/DataTable/DataTable.tsx`), `Notice` (`src/components/Notice/Notice.tsx`), `ErrorState` (`src/components/ErrorState.tsx`).
- **Changed API / look**:
  - `Flag`: tones `change | info | new`, `text-transform: uppercase` in CSS (`src/components/Flag/Flag.tsx:4-9`, `Flag.css`).
  - `ItemRow`: a union type: with a checkbox, `id` and `onToggle` are required (no random id). Adds `checked`, `checkboxLabel`, `checkboxDisabled` and `labels`, shown comma-separated with an sr-only "Etykiety:" prefix. The title is a `span` when there is no checkbox (`src/components/ItemRow/ItemRow.tsx:21-45, 69-76`).
  - `HoursBar`: `norm` and `label` required, no default 8 h; the norm comes from Rust (`src/components/HoursBar/HoursBar.tsx:3-14`).
  - `AppHeader`: buttons with `onSelect`, items `{id,label,active,key}`, `aria-keyshortcuts`, `data-view`, and a `children` slot for the counter. The header no longer renders the clock itself (`src/components/AppHeader/AppHeader.tsx`). Digits 1–6 switch views: no modifier, no auto-repeat, not in editable fields, and focus moves to the chosen tab (`src/App.tsx:14-57`).
  - `Tabs`: generic, with roving focus (arrows, Home and End), `panelId` and `idPrefix` (`src/components/Tabs/Tabs.tsx`).
  - `SourceState`: `role=status` for `calm`, `lastOk` rendered as given (the artifact prefixes "ostatnie udane odświeżenie:"), and `actionDisabled` (aria-disabled + aria-busy) (`src/components/SourceState/SourceState.tsx`).
  - `Segmented` `size: sm | md`, `StatStrip` tone `soft` (with `muted` as an alias) and tiles from 180 px, `Field` `multiline` and disabled style, `Button` `aria-disabled` busy look and `ref`, and `Panel` `className`.
  - `Icon` adds `eye`, `eye-off`, `alert-triangle`, `clock` and `folder`; `hide`, `show` and `alert` stay as aliases (`src/components/Icon/Icon.tsx`).
- **Work-time counter** (`src/worktime/WorkTimerHeader.tsx`, `.css`):
  - START/STOP asks first: "Rozpocząć pracę?" / "Zakończyć pracę?", then [TAK, START | TAK, STOP] [Anuluj]. "Anuluj" sits where START/STOP was and gets focus.
  - The question gives up after 6 s or on Escape. Clicks within the first 400 ms are ignored.
  - Loading reads "Czas pracy: wczytywanie…". An error is a compact error `Notice` with "Spróbuj ponownie".
- **Tokens**:
  - `--hatch-stripe` and `--hatch-bg` (`src/styles/tokens.css:93-95`) are missing from the artifact.
  - The app bundles six woff2 files (`src/assets/fonts/`, OFL), while the artifact imports Google Fonts in `bundle.css`.
  - All other colour, spacing, radius and size values match.
- **Icons**: the README says "Brak logo". The app has an app icon (a compass needle in a day ring, amber) and three tray states (`src-tauri/icons/source/{app-icon,tray-ok,tray-error,tray-idle}.svg`; `src-tauri/src/runtime.rs:139-145`).
- **Flags**: the README says "Najwyżej jedna flaga na wiersz". The app shows several: GitLab renders NOWE, ZMIENIONE, DRAFT and W DO ZROBIENIA in that order (`src/gitlab/GitlabView.tsx:199-208`). "Do zrobienia" puts the first flag in `flag` and the rest in `meta` (`src/todo/TodoView.tsx:325-336`).
- **S-06 "Przypomnienia"** (`src/worktime/RemindersPanel.tsx`): a `Panel` holding a `dl`. Each row shows a label, a `code` value or "—", `StatusPill glyph={false}` WŁ./WYŁ., "domyślnie" as meta and a hint. The footer has a quiet "Ustawienia" button (icon `external`) and a compact error `Notice`. Load errors go through `ErrorState` with "ostatnio OK HH:MM".
- **S-07 "Dane aplikacji"** (`src/health/SourceCard.tsx`, `sourceView.ts:97-113`, `OpenBackupsButton.tsx`): the source card has a glyph, name, kind and pill, a message and a facts `dl`. The facts are Sprawdzono, Ostatnia kopia (tone `warn` when stale), Kopie "N z M" and Schemat `vN`. A quiet "Otwórz folder kopii" button with the `folder` icon sits under the facts.

## Desired End State

The live artifact describes the shipped app:

- `tokens.json` matches `src/styles/tokens.css` token for token, plus the six local fonts.
- `bundle.js` exports every component of `src/components/` with the same props, markup and class names.
- `bundle.css` carries the component CSS verbatim.
- Every component card has a README and a live preview with invented data.
- Two view-pattern cards show S-06 and S-07, and the `AppHeader` card shows the counter with its START/STOP question.
- An asset group "Ikony aplikacji" holds the four icon SVGs.
- The README brand book states the current rules: local fonts, several flags in a fixed order, numbered navigation with shortcuts, START/STOP confirmation, `Notice` vs `SourceState`, and the app and tray icons.

Verify it by checking every row of `context/changes/design-system-artifact-sync/sync-checklist.md`. Read the published files back and open the artifact page with every card rendering.

### Key Discoveries:

- The artifact type keeps content in files under `project/`. The index is sent once, in the last call, after a fresh read. `tokens.json` is always sent whole. `tokens.css`, `manifest.json` and `api/` are generated by the page and never written (type `SKILL.md`, `artifact-type/reference/format.md`).
- `tokens.json` colours accept hex, colour functions or `{alias}` only. `hatch-stripe` becomes the alias `{line-control}`. `hatch-bg` (a `repeating-linear-gradient` over `var()`) cannot be a token and goes into `bundle.css` `:root`.
- Fonts go to `project/fonts/<file>` and are listed in `tokens.json` `type.fonts[]` (`family`, `file`, `weight`); the page then emits `@font-face`.
- A preview's line 1 is `<!-- @dsCard group="…" height=N subtitle="…" -->`. Adding `page` marks a showcase card that is not a bundle export, which is how the two view patterns are added. Component folder names must be ASCII JS identifiers.
- Icons/images under `assets/<Group>/` are uploads (`asset: true` publish), recorded in the index under `assetGroups.<Group>.files.<name>` = `{name, blob, size, type}`, with `groups` ordering them. An SVG shows only via `<img>`, so the group README names each icon's inks.
- `Chip` and `SourceChip` share the `.dn-chip` class. `SourceChip.css` therefore also styles `Chip` (mono, bold, min-width, centred) whenever both are loaded, which in the app is always. The artifact documents the shipped look; the overlap is reported (see "Findings to report").

## What We're NOT Doing

- No change to app code, CSS, Rust, icons or `AGENTS.md`. The artifact link stays the same.
- No light theme, no new tokens beyond what `tokens.css` has, no new components beyond `src/components/`.
- The view patterns are not bundle exports, and `WorkTimerHeader`, `RemindersPanel` and `SourceCard` are not ported as components.
- No full-view mockups (GitLab, GitHub, days table, event log). They stay in the separate mockup artifact. `DataTable`'s preview shows a short invented days table only.
- The Cover is not redesigned; it is edited only where it contradicts the app.
- No removal of artifact files. Every existing path stays, so no `null` entries.
- No work data anywhere: invented titles, numbers, repos and hosts only.
- The two findings below are not fixed here.

## Implementation Approach

Work on a working copy of the artifact's `project/` files in `context/changes/design-system-artifact-sync/artifact/project/`. The copy mirrors served paths, so the publish can use `root` = `…/artifact` and git shows every edit for review. Binaries are not copied: fonts are published straight from `src/assets/fonts/`, and the icon SVGs are uploaded straight from `src-tauri/icons/source/`. Small Node check scripts in `context/changes/design-system-artifact-sync/checks/` (plain `node`, no new dependencies) make the parity rules repeatable. Phases 1–4 touch only the repo. Phase 5 is the only one that writes to the artifact, after the user says yes in that session.

## Critical Implementation Details

- **Publish ordering and conflicts.** Re-read each file you will change, and the index, right before publishing. Compare them with the hashes recorded in phase 1. If someone edited the artifact meanwhile, merge onto the newer text and do not overwrite it. Upload the SVGs first, then send one `files` publish, then the index in the last call (with `groups`, `assetGroups`, `lastChange`, and every other key kept as read).
- **Privacy.** `lastChange.by` is "Claude" (as in the existing index) and `via` is "Claude Code". Do not put the user's name, email or any work term into the artifact.
- **Bundle runtime.** `bundle.js` is one classic script with no `import` and no network. It uses `window.React.createElement` and contains no literal `</script` or `<!--`. Previews run React 18, so hooks such as `useId` must not be used; generate ids from props the way the app's callers pass them.

## Phase 1: Working copy and sync checklist

### Overview

Pull the live artifact into the change folder and write the item-by-item list that the later phases work through and the review checks against.

### Changes Required:

#### 1. Working copy

**File**: `context/changes/design-system-artifact-sync/artifact/project/**`

**Intent**: Read every `project/` file of the live artifact (index, README, `tokens.json`, `components/bundle.{js,css}`, `components/index.d.ts`, `components/Cover/preview.html`, the 14 component folders) with the Artifact tool and save them at their served paths, so all later edits show up as git diffs.

**Contract**: the folder mirrors the artifact's `project/` paths exactly. Record the read version id and each file's sha256 in `sync-checklist.md` (the base for the conflict check in phase 5).

#### 2. Sync checklist

**File**: `context/changes/design-system-artifact-sync/sync-checklist.md`

**Intent**: One table row per item to sync, so nothing in F-05, S-06 or S-07 is missed and the impl review can check coverage. Each row names the code source, the target artifact file and the change.

**Contract**: The rows cover:

- tokens (hatch, fonts), README sections and the asset group;
- each of the 18 cards (`AppHeader`, `Button`, `Chip`, `DataTable`, `ErrorState`, `Field`, `Flag`, `HoursBar`, `Icon`, `ItemRow`, `Notice`, `Panel`, `Segmented`, `SourceChip`, `SourceState`, `StatStrip`, `StatusPill`, `Tabs`);
- the pattern cards `RemindersPanel` and `SourceCard`, and the Cover review;
- a "Findings to report" section.

Columns: `#`, item, source (`file:line`), artifact path, change, status.

### Success Criteria:

#### Automated Verification:

- Every `project/` path from the artifact's file listing exists in the working copy: `node context/changes/design-system-artifact-sync/checks/copy-complete.mjs`
- `sync-checklist.md` has a row for each of the 18 components exported from `src/components/index.ts`, plus `RemindersPanel` and `SourceCard`: `node context/changes/design-system-artifact-sync/checks/checklist-coverage.mjs`

---

## Phase 2: Tokens, fonts and README

### Overview

Update the foundations: tokens and fonts as the app has them, and the brand book rules that changed.

### Changes Required:

#### 1. Tokens and fonts

**File**: `artifact/project/tokens.json`

**Intent**: Make the tokens equal `src/styles/tokens.css`, and wire the six local woff2 files so the artifact renders the app's own fonts offline.

**Contract**:
- Add the colour token `hatch-stripe` = `{line-control}`, with a usage note: the day-off stripes in `DataTable` rows, visible on `surface` and `surface-raised`.
- Change `src-gitlab` to `#FF9F5A` (the literal `tokens.css` uses) or keep the alias; the value must stay equal.
- `type.fonts` gets six entries: `IBM Plex Sans` 400/500/600 and `JetBrains Mono` 400/500/700, `file` `fonts/<name>.woff2`, `style` normal.
- `type.families` stays.
- All other values stay; re-verify each against `tokens.css` (names and values in source order).

#### 2. README (brand book)

**File**: `artifact/project/README.md`

**Intent**: The rules a consuming agent reads must describe the shipped app.

**Contract**: The README changes:

- **Typography:** fonts are bundled locally (woff2, OFL) with no Google Fonts.
- **Flags:** a row may carry several flags in a fixed order: NOWE, ZMIENIONE, DRAFT, W DO ZROBIENIA. In `ItemRow` the first one goes in `flag` and the rest in `meta`. `info` marks "closed in the source". The CSS sets flags in capitals.
- **Navigation and counter:**
  - Six numbered views. Keys 1–6 switch views: no modifier, no auto-repeat, not while typing, and focus moves to the chosen tab.
  - The counter is always visible and sits in the `AppHeader` slot.
  - START/STOP asks "Rozpocząć pracę?" / "Zakończyć pracę?" with [TAK, START|STOP] [Anuluj]. "Anuluj" sits in START/STOP's place and gets focus. The question gives up after 6 s or on Escape and ignores clicks in its first 400 ms.
- **Messages:** use `SourceState`/`ErrorState` for a whole source and `Notice` for in-view banners, with tones `calm`, `info`, `warn`, `error` and the `compact`/`inset` variants. `lastOk` is a ready sentence ("ostatnio OK 08:52").
- **Day off:** the hatch pattern lives in `DataTable` rows (`hatched`), and `current` marks the row for now.
- **Icons:** the Icon set gets the new names. Replace "Brak logo" with the app icon and the three tray states, by asset name and inks; the `DAY/NAV` wordmark stays in the header.
- **Components:** the full list of 18, plus a short "Wzorce widoków" section that points to the `RemindersPanel` and `SourceCard` cards and the counter in `AppHeader`.
- **Helpers:** one line on `plural`, the `format*` time helpers and `useNow`, as app-side helpers that are not in the bundle.

#### 3. Asset group README

**File**: `artifact/project/assets/Ikony aplikacji/README.md`

**Intent**: Usage notes for the uploaded icons, since `<img>` cannot inherit colour.

**Contract**:
- `app-icon.svg`: the app and installer icon. Ring and needle tip in `accent` `#F5A524`, needle tail in `ink-faint` `#8A93A3`, on a transparent background.
- `tray-ok.svg`: amber, the overall state OK.
- `tray-error.svg`: `danger` `#FF7B72` plus a badge dot, the overall state error.
- `tray-idle.svg`: greys, for the starting and not-running states.
- The note says the bundle PNGs are generated from these SVGs (`npx tauri icon`).

### Success Criteria:

#### Automated Verification:

- `tokens.json` parses and its colour, spacing, radius and size tokens equal `src/styles/tokens.css` by name and value (aliases resolved; `hatch-bg` the only allowed miss): `node context/changes/design-system-artifact-sync/checks/tokens-parity.mjs`
- Every `type.fonts[].file` exists in `src/assets/fonts/`: checked by `tokens-parity.mjs`
- No term from the push denylist appears in the working copy (prints counts and file names only, never the terms): `node context/changes/design-system-artifact-sync/checks/no-work-data.mjs`

#### Manual Verification:

- README reads as current rules, in Polish, with no stale claims ("Najwyżej jedna flaga", "Brak logo", Google Fonts URL)

---

## Phase 3: Bundle, stylesheet and types

### Overview

Port every component of `src/components/` into the artifact's runtime so the previews render the app's markup and CSS.

### Changes Required:

#### 1. `bundle.js`

**File**: `artifact/project/components/bundle.js`

**Intent**: Rewrite the `DayNav` namespace as a faithful port of the TSX components: same props, defaults, element tree, ARIA attributes and class names.

**Contract**:
- Header `/* @ds-bundle: {"format":4,"namespace":"DayNav","components":[…]} */` lists the 18 cards in alphabetical order.
- `window.DayNav` also exposes `DataTableRow` and `DataTableCell`.
- `AppHeader` takes `items`, `onSelect` and `children`, and no longer has `clock`, `running` or `onToggle`.
- `ErrorState` wraps `SourceState kind="error"`, with the action label "Spróbuj ponownie".
- `Tabs` keeps the arrow, Home and End handling.
- `ItemRow` renders a checkbox only when `onToggle` is set and uses the given `id`.
- Classic script, React 18 `createElement` only, no `useId`.

#### 2. `bundle.css`

**File**: `artifact/project/components/bundle.css`

**Intent**: The component styles exactly as shipped, plus the base and helper rules the previews need.

**Contract**:
- Drop the Google Fonts `@import`.
- Keep the body base and the `a` and `:focus-visible` rules from `tokens.css`.
- Add `:root { --hatch-bg: repeating-linear-gradient(135deg, var(--hatch-stripe) 0 2px, transparent 2px 8px); }`.
- Then copy each `src/components/*/*.css` verbatim, one section per component in alphabetical order, each with a comment naming its source file.
- Then add the helpers used by previews from `tokens.css` and `layout.css` (`.dn-loading`, `.dn-empty`, `.dn-sr-only`, `.dn-view-head*`, `.dn-spacer`, `.dn-columns`/`.dn-col-*`, `.link-button` if used).
- Then a "Wzorce widoków" section with the rules the pattern previews need, copied from `src/worktime/WorkTimerHeader.css`, the `wt-reminder*` rules of `src/worktime/WorkTime.css` and the `health-card*` rules of `src/health/Health.css`. Each block has a source comment.
- No hex literals anywhere; colours come only through `var(--…)`.

#### 3. `index.d.ts`

**File**: `artifact/project/components/index.d.ts`

**Intent**: Types as documentation, mirroring the TS sources so the page's API cards show the real props.

**Contract**:
- One `<Comp>Props` per card, plus `DataTableRowProps`, `DataTableCellProps`, `NoticeTone`, `SourceKind`, `PillState`, `Stat`, `SegmentedOption` and `IconName` (with its aliases).
- Keep the doc comments from the TSX.
- `DayNav` declares all exports.

### Success Criteria:

#### Automated Verification:

- `node --check` passes on `bundle.js`, and it contains no `</script`, `<!--`, `import` or `fetch`: `node context/changes/design-system-artifact-sync/checks/bundle-check.mjs`
- Loaded with the repo's `react` and `react-dom/server`, each of the 18 components plus `DataTableRow` and `DataTableCell` renders sample props without throwing. The class names in the output equal those the TSX component sets for the same props, compared against a per-component list kept in the check: `bundle-check.mjs`
- Each `src/components/*/*.css` file appears verbatim, normalised for whitespace, in `bundle.css`, and `bundle.css` holds no hex colour: `node context/changes/design-system-artifact-sync/checks/css-parity.mjs`
- Every type exported from `src/components/index.ts` is declared in `index.d.ts`: `css-parity.mjs` (types section)

#### Manual Verification:

- The props in `index.d.ts` match the TSX props of each component (spot-check `ItemRow`, `HoursBar`, `AppHeader`, `Notice`, `DataTableCell`)

---

## Phase 4: Component cards and view patterns

### Overview

Every card's README and preview describe the shipped component with invented data; three new cards and two pattern cards are added.

### Changes Required:

#### 1. New cards

**Files**: `artifact/project/components/{Chip,DataTable,Notice,ErrorState}/{README.md,preview.html}`

**Intent**: Document the components the artifact lacks.

**Contract**:
- **Chip:** the priority marker with tones `warn`, `soft` and `faint`; `Flag` stays the change signal.
- **DataTable:**
  - The view sets the grid template in `--dn-table-cols`.
  - A `head` row, plus `current` (raised background, accent edge, `aria-current`) and `hatched` (day off), which can combine.
  - Cell tones `muted`, `faint`, `info` and `warn`, and `truncate`.
  - The preview is a short invented month table with `HoursBar`.
- **Notice:**
  - Four tones with their icons (`wifi-off`, `check-circle`, `alert-triangle`).
  - `compact` (under a field, in a row or in the header) and `inset` (in a panel body).
  - `action`, and `role` defaults: `alert` for error, `status` otherwise.
  - When to use `SourceState` instead.
- **ErrorState:** the single error renderer. `lastSuccess` is a ready sentence and `onRetry` shows "Spróbuj ponownie".

#### 2. Updated cards

**Files**: README and preview of `AppHeader`, `Button`, `Field`, `Flag`, `HoursBar`, `Icon`, `ItemRow`, `Panel`, `Segmented`, `SourceChip`, `SourceState`, `StatStrip`, `StatusPill`, `Tabs`

**Intent**: Bring each card to the current API and the rules from phase 2.

**Contract**:
- **AppHeader:** the preview shows the numbered nav (keys 1–6 work inside the preview frame) and the counter slot in four states: running (● REC, `05:12:34`, STOP), the question "Zakończyć pracę?" [TAK, STOP] [Anuluj], stopped, and an error `Notice` compact.
- **Flag:** shows `change`, `info` and `new`, and several flags in a row in the fixed order.
- **HoursBar:** the preview passes `norm` and `label` explicitly, with a day-off row (`norm={0}`).
- **ItemRow:** the preview shows ids, `labels`, `checkboxLabel` and a row without a checkbox.
- **Icon:** lists all names.
- **SourceState:**
  - `lastOk` reads like "ostatnio OK 08:52".
  - `calm` has `role=status`.
  - It shows the `actionDisabled` state.
- **Segmented:** shows `md`.
- **StatStrip:** documents `soft` and 180 px tiles.
- **Field:** shows `multiline`.
- **Button:** shows the `aria-disabled` busy state.
- **StatusPill:** shows `glyph={false}` with custom text (WŁ./WYŁ.).
- **Tabs:** documents the keyboard behaviour.

#### 3. View-pattern cards

**Files**: `artifact/project/components/{RemindersPanel,SourceCard}/{README.md,preview.html}`

**Intent**: Show S-06 and S-07 as compositions of the components, not as new exports.

**Contract**: Line 1 of each preview carries `group="Wzorce widoków"` and the `page` flag.

- **RemindersPanel:** "Przypomnienia" with two rows ("Licznik biegnie dłużej niż" `9:00` WŁ.; "Brak START-u w dzień roboczy po" — WYŁ. "domyślnie"), the hints, the footer "Ustawienia" and one compact error `Notice` variant.
- **SourceCard:** "Dane aplikacji", "OK", with the facts Sprawdzono "2 min temu", Ostatnia kopia "wczoraj" (and one stale variant with tone `warn`), Kopie "5 z 7", Schemat "v11", and "Otwórz folder kopii" (icon `folder`).
- Each README says where the pattern lives in the app (`src/worktime/RemindersPanel.tsx`, `src/health/SourceCard.tsx`) and which components it is built from.

#### 4. Cover review

**File**: `artifact/project/components/Cover/preview.html`

**Intent**: Remove any claim that contradicts the app (component count, missing logo, Google Fonts). Leave everything else.

**Contract**: Edit only text that contradicts the app; no redesign.

### Success Criteria:

#### Automated Verification:

- Each of the 18 cards and the 2 pattern cards has `README.md` and `preview.html`. Each preview's line 1 is a valid `@dsCard` marker (pattern cards with `page`), and no preview has an iframe, an unpinned CDN URL or a hex colour: `node context/changes/design-system-artifact-sync/checks/cards-check.mjs`
- No term from the push denylist appears in the working copy: `node context/changes/design-system-artifact-sync/checks/no-work-data.mjs`

#### Manual Verification:

- All example data is invented (titles, numbers, repos), and none is copied from the user's real GitLab or GitHub

---

## Phase 5: Local render and publish

### Overview

Render every card locally, get the user's OK, then publish to the artifact in the order the type requires and check the live page.

### Changes Required:

#### 1. Local render harness

**File**: `context/changes/design-system-artifact-sync/checks/render-local.mjs` (output in the session scratchpad, not committed)

**Intent**: Build one static HTML page per preview to look at the cards before publishing.

**Contract**:
- Each page inlines a `:root` block generated from `tokens.json` (vars, plus `@font-face` from `type.fonts` pointing at `src/assets/fonts/`), then `bundle.css`, React and ReactDOM 18.3.1 UMD from jsDelivr (pinned), `bundle.js`, and the preview body.
- Open each page in the built-in browser and take screenshots.

#### 2. Publish (only after the user's explicit yes in this session)

**Intent**: Put the working copy live without overwriting anyone else's edits.

**Contract**: The order:

1. Re-read the index and every file that will change, and compare them with the phase-1 hashes. Merge if they differ.
2. Upload the four SVGs from `src-tauri/icons/source/` with `asset: true` and record their `/_blob/<id>`.
3. Send one publish with `url` = the artifact, `root` = `context/changes/design-system-artifact-sync/artifact`, `file_path` = the absolute path of `project/README.md`, and `files` = every changed `project/` path. Add `project/fonts/*.woff2` and the two OFL texts as `{from: <absolute path in src/assets/fonts/>}`.
4. Re-read the index, then publish it with `groups: ["Ikony aplikacji"]`, the `assetGroups` record (`tile: "m"`, `order` and `files` with `name`, `blob`, `size` and `type: image/svg+xml`) and `lastChange` `{by: "Claude", via: "Claude Code", at: <now>, note: "Zgodność z aplikacją po S-01–S-08 (F-05)."}`. Keep every other key and `createdOnFiles`.

#### 3. Read-back

**Intent**: Confirm what is live.

**Contract**:
- List the files, then read the index and README back.
- Open the artifact in the built-in browser and go through Overview, Colors, Typography, Components and Assets.
- Record the new version id in `sync-checklist.md`.

### Success Criteria:

#### Automated Verification:

- `render-local.mjs` builds a page for every preview without error
- The artifact's file listing after publishing contains every working-copy path and the six font files, and the index has the "Ikony aplikacji" group with four records

#### Manual Verification:

- Local renders reviewed: each card matches the app's look for the same component (spot-check AppHeader, ItemRow, DataTable, Notice against the running app)
- The user gave an explicit OK to publish before step 2 ran
- Live artifact: every component and pattern card renders (no static fallback), fonts are IBM Plex Sans / JetBrains Mono, the four icons show in Assets
- `sync-checklist.md` has every row marked done, and the findings are reported to the user

---

## Findings to report (not fixed here)

- `.dn-chip` is shared by `Chip` (`src/components/Chip/Chip.css`) and `SourceChip` (`src/components/SourceChip/SourceChip.css`). Since both stylesheets load globally, the priority `Chip` always picks up `SourceChip`'s mono/bold/min-width/centre rules. This may be intended; it should be confirmed or the class split in a separate change.
- `OpenBackupsButton` shows its error as a plain `<p class="health-card-note is-error" role="alert">` instead of a compact `Notice` like the "Przypomnienia" footer (`src/health/OpenBackupsButton.tsx:31-35`). This is a consistency nit.

## Testing Strategy

### Unit Tests:

- None in the app (no app code changes). Parity rules are enforced by the `checks/*.mjs` scripts.

### Integration Tests:

- `bundle-check.mjs` renders each ported component with React server rendering and compares class names with the TSX components.

### Manual Testing Steps:

1. Run `npm run tauri dev` and open the app beside the local renders; compare AppHeader (incl. START/STOP question), a GitLab row with several flags, the days table, "Przypomnienia" and the "Dane aplikacji" card.
2. After publishing, open the artifact and check each card and the Assets tab.
3. Check README for stale rules.

## Performance Considerations

None. The six woff2 files add about 0.3 MB to the artifact, well under its caps.

## Migration Notes

There are no data migrations. The publish replaces files in place. The previous artifact version stays in its history, so rolling back means republishing the phase-1 copy from git.

## References

- Roadmap item: `context/foundation/roadmap.md` (F-05, MS-16, issue #16)
- Original design-system change: `context/archive/2026-11-23-konsola-design-system/plan.md`
- S-06: `context/archive/2026-11-24-worktime-reminders-panel/`; S-07: `context/archive/2026-11-24-app-data-details/`
- Artifact type rules: `SKILL.md`, `artifact-type/reference/format.md`, `artifact-type/reference/craft.md` on the artifact
- Sources: `src/components/`, `src/styles/tokens.css`, `src/styles/layout.css`, `src/App.tsx`, `src/worktime/WorkTimerHeader.tsx`, `src/worktime/RemindersPanel.tsx`, `src/health/SourceCard.tsx`, `src-tauri/icons/source/`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Working copy and sync checklist

#### Automated

- [x] 1.1 Every artifact project path exists in the working copy
- [x] 1.2 sync-checklist.md covers the 18 components and 2 pattern cards

### Phase 2: Tokens, fonts and README

#### Automated

- [ ] 2.1 tokens.json equals tokens.css by name and value
- [ ] 2.2 Every type.fonts file exists in src/assets/fonts
- [ ] 2.3 No denylist term in the working copy

#### Manual

- [ ] 2.4 README states current rules with no stale claims

### Phase 3: Bundle, stylesheet and types

#### Automated

- [ ] 3.1 bundle.js passes node --check and runtime limits
- [ ] 3.2 Every bundle component renders with the TSX class names
- [ ] 3.3 Component CSS copied verbatim, no hex in bundle.css
- [ ] 3.4 Every exported type is declared in index.d.ts

#### Manual

- [ ] 3.5 index.d.ts props match the TSX props

### Phase 4: Component cards and view patterns

#### Automated

- [ ] 4.1 Every card has README and a valid preview
- [ ] 4.2 No denylist term in the working copy

#### Manual

- [ ] 4.3 All example data is invented

### Phase 5: Local render and publish

#### Automated

- [ ] 5.1 render-local.mjs builds every preview
- [ ] 5.2 Published listing has every path, fonts and the icon group

#### Manual

- [ ] 5.3 Local renders match the app
- [ ] 5.4 User gave explicit OK before publishing
- [ ] 5.5 Live artifact renders every card, fonts and icons
- [ ] 5.6 Checklist done and findings reported
