# UI Design Overhaul — Spec

**Date:** 2026-03-22
**Status:** Approved

## Overview

Improve the overall visual design of lazybacklog's TUI by introducing a bordered panel layout, a consistent color scheme, and better visual hierarchy across all screens.

## Goals

- Make the UI feel polished and professional (lazygit-inspired)
- Improve visual structure through borders and layout changes
- Make the selected row more visible (high contrast: white bg / black text)
- Unify the color palette across all screens

## Out of Scope

- Color-coding of statuses or priorities
- Multi-panel (left/right split) layout changes
- Any changes to keyboard bindings or data fetching logic

---

## Design Decisions

### Color Palette

| Element                        | Current                     | New                                        |
|--------------------------------|-----------------------------|--------------------------------------------|
| Title bar background           | None                        | Cyan (`Color::Cyan`)                       |
| Title bar text                 | Cyan, Bold                  | Black, Bold (on Cyan bg)                   |
| Table header (issue list)      | Yellow                      | Cyan (`Color::Cyan`)                       |
| Field labels (issue detail)    | Yellow                      | Cyan (`Color::Cyan`)                       |
| Selected row                   | Blue bg, white text, Bold   | White bg (`Color::White`), Black text, Bold |
| Panel / block border color     | Cyan (popups only)          | DarkGray (`Color::DarkGray`) — all panels  |
| Filter bar text                | Gray                        | Gray (unchanged)                           |
| Help bar text                  | DarkGray                    | DarkGray (unchanged)                       |
| Status message                 | Yellow                      | Yellow (unchanged)                         |

### Layout: Issue List Screen (`Screen::IssueList`)

**Before:**
```
[Title bar       ] — 1 line, Cyan text, no background
[Filter bar      ] — 1 line, Gray text
[Issue table     ] — remaining space
[Help bar        ] — 1 line, DarkGray
```

**After:**
```
[Title bar       ] — 1 line, Cyan background + Black Bold text (reversed)
[╔══════════════╗] — Block border, DarkGray
[║ Filter bar   ║] — Gray text; bottom separator line (DarkGray)
[║ Table header ║] — Cyan text; bottom separator line
[║ Table rows   ║] — normal rows; selected = White bg / Black text / Bold
[║ (count/search)║] — footer remains inside the block, managed by issue_list.rs
[╚══════════════╝]
[Help bar        ] — 1 line, DarkGray
```

**Layout constraints change in `mod.rs`:**
- Remove the top-level `Constraint::Length(1)` for filter bar (filter moves inside block)
- New top-level layout: `[Length(1) title] [Min(0) block-panel] [Length(1) help]`
- The block panel's inner area is split via an inner `Layout`:
  - `[Length(1) filter-bar] [Min(0) issue-list-area]`
- `issue_list.rs::render()` receives only the `issue-list-area` (below the filter bar)
- The footer (count/search bar) in `issue_list.rs` continues to carve `area.height - 1` as before — no change to that logic

### Layout: Issue Detail Screen (`Screen::IssueDetail`)

**Before:**
```
[Title bar       ] — Cyan text, no background
[Content         ] — plain, no border (fields + description)
[Help bar        ] — DarkGray
```

**After:**
```
[Title bar       ] — Cyan background + Black Bold text (same pattern)
[╔══════════════╗] — Block border, DarkGray
[║                   ║] — blank line (existing Line::from(""))
[║ Assignee  Priority ║]
[║ Status    Type     ║]
[║ Due                ║]
[║                   ║] — blank line (existing Line::from(""))
[║ Description:      ║]
[║                   ║] — blank line (existing)
[║ ...               ║]
[╚══════════════════╝]
[Help bar        ] — DarkGray
```

- Field labels (Assignee, Priority, Status, Type, Due, Description) change from Yellow to Cyan
- Implementation: change the existing `.block(Block::default().borders(Borders::NONE))` on the `Paragraph` to `.block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))`. The Paragraph is still rendered to `chunks[1]` — no additional frame render call needed.
- **Scroll regression note:** Adding `Borders::ALL` reduces usable inner height by 2 rows (top + bottom border). Since there is no hardcoded scroll-clamping logic tied to a fixed line count in `main.rs`, no adjustment is needed — `scroll_offset` continues to work as before.

### Popups: Filter (`filter.rs`)

Border color changes from `Color::Cyan` to `Color::DarkGray` in **both Block instances**:
1. Error-state block (`users_error` early return path)
2. Normal list block (main render path)

Selected row highlight: Blue bg → White bg / Black text / Bold

### Popups: Status Filter (`status_filter.rs`)

Border color changes from `Color::Cyan` to `Color::DarkGray` in **all three Block instances**:
1. "Loading..." early return block
2. "No statuses" early return block
3. Normal list block (main render path)

Selected row highlight: Blue bg → White bg / Black text / Bold

### Project Select Screen (`project_select.rs`)

- `render_title`: background → Cyan, text → Black Bold
- List highlight: Blue bg → White bg / Black text / Bold
- Title text format unchanged: `" lazybacklog ──── [space_name] "`

### Space Select Screen (`space_select.rs`)

- `render_title`: background → Cyan, text → Black Bold
- List highlight: Blue bg → White bg / Black text / Bold
- Note: title text is `" lazybacklog"` (no space name suffix) — shorter than other screens. The Paragraph fills the full terminal width so the Cyan background will extend across the row naturally.

---

## Implementation Notes

- `mod.rs`, `project_select.rs`, and `space_select.rs` each have their own private `render_title` function — update each independently. `issue_detail.rs` has no `render_title` helper; the title is rendered inline at the top of `render()` — update that inline code directly.
- In `mod.rs`, the filter bar rendering moves from the top-level layout into the Block panel. The `render_filter_bar` function signature changes to receive the inner filter-bar area instead of `chunks[1]`.
- `issue_list.rs::render()` requires no structural changes — only the highlight style changes. It continues to manage the footer by carving `area.height - 1`.
- The Block widget for the issue panel and detail panel has no title string (the title bar above is the label). `Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))`.

---

## Files Affected

| File | Changes |
|------|---------|
| `src/ui/mod.rs` | Title bar: Cyan bg / Black text; restructure layout (remove filter constraint, add Block panel, inner layout for filter); `render_filter_bar` area adjustment |
| `src/ui/issue_list.rs` | Highlight style: White bg / Black text / Bold; table header: Cyan |
| `src/ui/issue_detail.rs` | Title bar: Cyan bg / Black text; wrap content in Block (DarkGray border); field labels: Yellow → Cyan |
| `src/ui/filter.rs` | All Block border colors: Cyan → DarkGray (2 instances); highlight: White bg / Black text |
| `src/ui/status_filter.rs` | All Block border colors: Cyan → DarkGray (3 instances); highlight: White bg / Black text |
| `src/ui/project_select.rs` | Title bar: Cyan bg / Black text; highlight: White bg / Black text |
| `src/ui/space_select.rs` | Title bar: Cyan bg / Black text; highlight: White bg / Black text |
