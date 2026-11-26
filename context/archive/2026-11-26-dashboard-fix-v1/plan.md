# Full-width "Do zrobienia" layout Implementation Plan

## Overview

The "Do zrobienia" view puts its three blocks in one row (list, picker, side panels), so at a window of about 1250 px the list itself gets only about 515 px. This plan stacks the view vertically at full width: the list on top, "Do wyboru" below it with the three sources in three columns, and "Stan źródeł" next to "Dziennik zdarzeń" (1:2) at the bottom. The approved mockup is `context/changes/dashboard-fix-v1/mockup.html` (variant A).

## Current State Analysis

- `src/todo/TodoView.tsx:388-490` renders `.dn-columns` with three children: `.dn-col-main` (the "Do zrobienia" `Panel`), `.todo-col-pick` (`Picker`) and `.todo-col-side` (`SourcesPanel` + `EventLogPanel`).
- The flex values are `3 1 440px` (`src/styles/layout.css:27`), `2 1 320px` and `1 1 260px` (`src/todo/TodoView.css:3-14`). With `.container` at max 1360 px and 20 px padding, a 1250 px window gives about 1210 px of content: 440 + 320 + 260 + 2 × 20 gaps = 1060, and the remaining 150 px split 3:2:1 leaves the list at about 515 px, the picker at about 370 px and the side column at about 285 px.
- `Picker` (`src/todo/TodoView.tsx:527-710`) renders one `Panel` with three groups in sequence: a `GroupHead` and then the group's error, status, notices, empty message or rows, for GitLab, GitHub and "Moje zadania".
- The empty-list message says "Nic na dziś — dodaj z list obok" (`src/todo/TodoView.tsx:401`). It points to lists "next to" the list, which is no longer true after this change.
- `.dn-columns` / `.dn-col-main` / `.dn-col-side` in `src/styles/layout.css` are also used by `GithubView`, `HealthPanel`, `OwnTasksView` and `WorkTimeView`, so the shared layout must not change.
- The window is 1280 × 800 by default with `minWidth: 800` (`src-tauri/tauri.conf.json:15-18`), so the content can be as narrow as about 760 px.
- The design-system artifact holds component cards and two pattern cards, not full-view mockups (see the archived `design-system-artifact-sync` brief), so this layout change needs no artifact sync.

## Desired End State

At a window of about 1250 px, the "Do zrobienia" view shows, top to bottom:

1. the summary strip (unchanged);
2. notices and errors (unchanged);
3. the "Do zrobienia" panel at full content width;
4. the "Do wyboru" panel at full content width, with GitLab, GitHub and "Moje zadania" as three equal columns separated by a vertical divider, each column holding its own head, error, status, notices, empty message and rows;
5. a bottom row with "Stan źródeł" (about one third) next to "Dziennik zdarzeń" (about two thirds).

In a narrow window (down to the 800 px minimum) the picker's three columns fall back to one stacked list (GitLab, GitHub, "Moje zadania", as today) with no vertical dividers, and the bottom row stacks "Stan źródeł" above "Dziennik zdarzeń". The empty list says the lists are below.

Verify by comparing the built app with the mockup at about 1250 px and at 800 px.

### Key Discoveries:

- Three-column flex row with only ~515 px for the list at 1250 px: `src/todo/TodoView.tsx:388`, `src/todo/TodoView.css:3-14`, `src/styles/layout.css:20-31`.
- The picker's groups are siblings in one `Panel`, so a per-source wrapper is needed to place them in columns: `src/todo/TodoView.tsx:609-707`.
- Shared `.dn-columns` is used by four other views, so this change stays in `src/todo/`.
- The minimum window width is 800 px: `src-tauri/tauri.conf.json:17`.

## What We're NOT Doing

- No change to `src/styles/layout.css` or any other view.
- No change to data, hooks, Rust commands, the limit, flags or the "+" button behaviour.
- No change to the content of the "Stan źródeł" and "Dziennik zdarzeń" panels; the event log keeps its scroll after about eight rows.
- No variant B (one stacked list at full width); it was rejected in favour of three columns.
- No 1:1 or stacked bottom row at wide widths; the user chose side by side 1:2.
- No design-system artifact update (it has no full-view mockups).
- No new tokens: spacing and lines use existing tokens from `src/styles/tokens.css`.

## Implementation Approach

Replace the three-column wrapper in `TodoView` with a vertical sequence of the list panel, the picker panel and a bottom row. Inside `Picker`, wrap each source's block in a group element and lay the groups out in a three-column grid when the panel is wide enough, otherwise as today's single list. Lay the bottom row out as a wrapping flex row with a 1:2 ratio. All layout lives in `src/todo/TodoView.css` with the existing `todo-` prefix.

## Phase 1: Vertical layout of the "Do zrobienia" view

### Overview

Restructure the JSX and CSS of the "Do zrobienia" view to match mockup variant A, with fallbacks for a narrow window.

### Changes Required:

#### 1. View structure

**File**: `src/todo/TodoView.tsx`

**Intent**: Replace the `.dn-columns` row with a vertical stack: the "Do zrobienia" `Panel` at full width, then `Picker` at full width, then a bottom row holding `SourcesPanel` and `EventLogPanel`. Change the empty-list text from "dodaj z list obok" to point below (e.g. "Nic na dziś — dodaj z list poniżej").

**Contract**: The `main.container` children after the notices become: the list `Panel` (still `headingLevel={1}`, same meta and footer), `Picker` (same props), and a bottom-row wrapper (`todo-bottom`) with `SourcesPanel` (`todo-bottom-sources`) and `EventLogPanel` (`todo-bottom-events`) wrappers. `.dn-columns`, `.dn-col-main`, `.todo-col-pick` and `.todo-col-side` are no longer used in this view. The loading and error branches keep their current structure.

#### 2. Picker source columns

**File**: `src/todo/TodoView.tsx` (`Picker`)

**Intent**: Wrap each source's block (its `GroupHead` and everything rendered under it up to the next source) in its own group element, and the three groups in one grid wrapper, so they can be laid out as columns. Order stays GitLab, GitHub, "Moje zadania"; content and conditions stay unchanged.

**Contract**: Inside the `Panel` titled "Do wyboru": one wrapper `todo-pick-groups` containing three `todo-pick-group` elements. Each group's errors (`ErrorState`), status (`SourceStatusView`), notices (`Notice`) and empty messages stay inside that source's group.

#### 3. Layout CSS

**File**: `src/todo/TodoView.css`

**Intent**: Remove the old column rules and add the stacked layout: three equal picker columns with a vertical divider (`--line-subtle`) between them when the panel is wide enough, a single stacked list without dividers otherwise, and a wrapping bottom row with "Stan źródeł" to "Dziennik zdarzeń" at about 1:2 that stacks in a narrow window. Update the header comment that refers to the old 3/2/1 columns.

**Contract**:
- `.todo-col-pick` and `.todo-col-side` rules are removed.
- `.todo-pick-groups` is a three-column grid (`minmax(0, 1fr)` each) only when the picker panel is at least about 840 px wide (a container query on the picker panel, or an equivalent that keys on the panel's width), otherwise a single column; the vertical divider applies only in the three-column layout.
- `.todo-bottom` is a wrapping flex row with the `--space-5` gap; the sources item grows 1 and the events item grows 2 with bases chosen so that they sit side by side at a ~1250 px window and wrap below roughly 900 px.
- Existing rules for `.todo-pick .dn-row`, `.todo-group-head`, `.todo-sources` and `.todo-events` stay.
- Only tokens from `src/styles/tokens.css`; no literal colours.

### Success Criteria:

#### Automated Verification:

- Lint passes: `npm run lint`
- Formatting passes: `npm run format:check`
- Type check and build pass: `npm run build`
- `.dn-columns`, `todo-col-pick` and `todo-col-side` no longer appear in `src/todo/`: `grep -rn "dn-columns\|todo-col-pick\|todo-col-side" src/todo` returns nothing
- `src/styles/layout.css` is unchanged: `git diff --quiet -- src/styles/layout.css`

#### Manual Verification:

- At a ~1250 px window the view matches mockup variant A: full-width list, "Do wyboru" below in three columns, "Stan źródeł" next to "Dziennik zdarzeń" at about 1:2
- At the 800 px minimum window the picker shows one stacked list without dividers and the bottom panels stack, with no horizontal scroll
- Per-source states still show in their own column: GitLab unavailable (calm), a GitHub load error with retry, and an empty source message
- Adding from the picker, ticking and removing from the list work as before, and the empty list says the lists are below
- The other views (GitLab, GitHub, Moje zadania, Czas pracy, Źródła) look unchanged
- In the built app (`npm run tauri build -- --debug --no-bundle`) the DevTools console shows no CSP violation

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful.

---

## Testing Strategy

### Unit Tests:

- None: the UI has no test runner, and no Rust domain rule changes.

### Manual Testing Steps:

1. Run `npm run tauri dev`, set the window to about 1250 px wide and compare "Do zrobienia" with `context/changes/dashboard-fix-v1/mockup.html` (variant A).
2. Shrink the window to 800 px: the picker is one list without dividers, the bottom panels stack, and nothing scrolls sideways.
3. With the work GitLab unreachable, check that its calm "niedostępny" status sits in the GitLab column and the other columns are unaffected.
4. Add an item with "+", tick it, take it off; empty the list and read the empty message.
5. Click through the other five views and confirm they look as before.
6. Build with `npm run tauri build -- --debug --no-bundle` and check the DevTools console for CSP violations.

## Performance Considerations

None: only layout changes; no extra renders, hooks or data.

## References

- Approved mockup: `context/changes/dashboard-fix-v1/mockup.html` (variant A, bottom row 1:2)
- Current view: `src/todo/TodoView.tsx:353-493`, `src/todo/TodoView.tsx:527-710`
- Current layout CSS: `src/todo/TodoView.css:1-14`, `src/styles/layout.css:20-31`
- Original "Do zrobienia" change: `context/archive/2026-11-23-todo-list/`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Vertical layout of the "Do zrobienia" view

#### Automated

- [x] 1.1 Lint passes: `npm run lint` — 109d1ef
- [x] 1.2 Formatting passes: `npm run format:check` — 109d1ef
- [x] 1.3 Type check and build pass: `npm run build` — 109d1ef
- [x] 1.4 `.dn-columns`, `todo-col-pick` and `todo-col-side` no longer appear in `src/todo/` — 109d1ef
- [x] 1.5 `src/styles/layout.css` is unchanged — 109d1ef

#### Manual

- [x] 1.6 At a ~1250 px window the view matches mockup variant A — 109d1ef
- [x] 1.7 At the 800 px minimum window the picker stacks without dividers and the bottom panels stack, with no horizontal scroll — 109d1ef
- [x] 1.8 Per-source states still show in their own column — 109d1ef
- [x] 1.9 Adding, ticking and removing work as before, and the empty list says the lists are below — 109d1ef
- [x] 1.10 The other views look unchanged — 109d1ef
- [x] 1.11 The built app's DevTools console shows no CSP violation — 109d1ef
