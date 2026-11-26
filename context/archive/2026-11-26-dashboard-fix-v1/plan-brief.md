# Full-width "Do zrobienia" layout — Plan Brief

> Full plan: `context/changes/dashboard-fix-v1/plan.md`
> Mockup: `context/changes/dashboard-fix-v1/mockup.html` (variant A)

## What & Why

At a window of about 1250 px the "Do zrobienia" list is too narrow: it shares one row with "Do wyboru" and the side panels and gets only about 515 px. The user wants the list at full width, "Do wyboru" below it, and "Stan źródeł" and "Dziennik zdarzeń" at the bottom.

## Starting Point

`TodoView` lays out three flex columns at 3:2:1 (minimum 440 / 320 / 260 px): the list, the picker with GitLab, GitHub and "Moje zadania" stacked, and a side column with "Stan źródeł" above "Dziennik zdarzeń". The shared `.dn-columns` layout is also used by four other views.

## Desired End State

At about 1250 px the view is a vertical stack: summary strip, notices, the "Do zrobienia" list at full width, "Do wyboru" at full width with the three sources in three columns, and at the bottom "Stan źródeł" (about 1/3) next to "Dziennik zdarzeń" (about 2/3). Down to the 800 px minimum window the picker falls back to one stacked list and the bottom panels stack. The empty list says the lists are below.

## Key Decisions Made

| Decision            | Choice                                                    | Why (1 sentence)                                                                                  |
| ------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| "Do wyboru" layout  | Variant A: GitLab / GitHub / Moje zadania in three columns | The panel is about half as tall, "+" stays next to the title, and the bottom panels stay nearer the top. |
| Bottom row          | "Stan źródeł" next to "Dziennik zdarzeń", 1:2             | The log gets room for full titles; the short source list does not waste width.                     |
| Narrow window       | Picker stacks below about 840 px of panel width; bottom row wraps below about 900 px | The window can be 800 px wide, where three columns would be about 240 px each.        |
| Where the change lives | Only `src/todo/TodoView.tsx` and `TodoView.css`        | `.dn-columns` in `layout.css` is shared with four other views.                                     |
| Per-source states   | Errors, statuses, notices and empty messages stay in their source's column | Each column reads as one self-contained source.                                    |

## Scope

**In scope:**

- The JSX structure of `TodoView` and `Picker` (wrappers only).
- Layout rules in `src/todo/TodoView.css`.
- The empty-list text ("obok" → "poniżej").

**Out of scope:**

- `src/styles/layout.css` and every other view.
- Data, hooks, Rust, the limit, flags and the "+" behaviour.
- The content of "Stan źródeł" and "Dziennik zdarzeń".
- Variant B, a 1:1 or stacked bottom row at wide widths.
- The design-system artifact (it has no full-view mockups).

## Architecture / Approach

`main.container` holds, in order: summary, notices, the list `Panel`, `Picker`, and a `todo-bottom` flex row with `SourcesPanel` and `EventLogPanel` (grow 1 : 2, wrapping). Inside `Picker`, each source's block is wrapped in `todo-pick-group`, and the three groups sit in `todo-pick-groups`: a three-column grid with vertical dividers when the panel is wide enough, otherwise one column as today.

## Phases at a Glance

| Phase                                         | What it delivers                                  | Key risk                                                          |
| --------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------- |
| 1. Vertical layout of the "Do zrobienia" view | List, picker and bottom row stacked as in the mockup | Uneven column heights or awkward wrapping at in-between widths |

**Prerequisites:** none.
**Estimated effort:** one short session, one commit.

## Open Risks & Assumptions

- The about 840 px and 900 px thresholds are starting values; the manual check at 800 px and 1250 px may move them.
- When one source has many candidates, its column is much taller than the others; accepted with variant A.

## Success Criteria (Summary)

- At about 1250 px the "Do zrobienia" list uses the full width and the view matches the mockup.
- At 800 px nothing scrolls sideways and the picker reads as one list.
- Every other view looks exactly as before.
