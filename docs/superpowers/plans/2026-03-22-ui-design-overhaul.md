# UI Design Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update all TUI screens to use a Cyan-inverted title bar, DarkGray bordered panels, and a White/Black selected row across the entire app.

**Architecture:** Style-only changes to existing ratatui widgets (no logic changes). The biggest structural change is in `mod.rs` where the issue list layout gains a Block wrapper that contains the filter bar and table as one panel. All other changes are isolated color/style swaps within each file's existing structure.

**Tech Stack:** Rust, ratatui (Block, Borders, Style, Color, Modifier, Layout, Constraint)

---

## File Map

| File | What changes |
|------|-------------|
| `src/ui/project_select.rs` | `render_title` → Cyan bg / Black text; list highlight → White bg / Black text |
| `src/ui/space_select.rs` | `render_title` → Cyan bg / Black text; list highlight → White bg / Black text |
| `src/ui/filter.rs` | Both Block borders: Cyan → DarkGray; list highlight → White bg / Black text |
| `src/ui/status_filter.rs` | All three Block borders: Cyan → DarkGray; list highlight → White bg / Black text |
| `src/ui/issue_list.rs` | Table header: Yellow → Cyan; highlight → White bg / Black text |
| `src/ui/issue_detail.rs` | Title inline code → Cyan bg / Black text; content Block → DarkGray border; field labels: Yellow → Cyan |
| `src/ui/mod.rs` | `render_title` → Cyan bg / Black text; layout restructure (3-chunk, Block panel, inner layout for filter) |

---

## Task 1: Style select screens (project_select.rs, space_select.rs)

**Files:**
- Modify: `src/ui/project_select.rs`
- Modify: `src/ui/space_select.rs`

These two screens share the same pattern: a `render_title` private function and a list `highlight_style`. No layout changes needed.

- [ ] **Step 1: Update `project_select.rs` — title bar**

In `src/ui/project_select.rs`, find `render_title` (around line 24) and change:

```rust
fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}
```

to:

```rust
fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Update `project_select.rs` — list highlight**

In `render_content` (around line 57), change:

```rust
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

to:

```rust
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

- [ ] **Step 3: Update `space_select.rs` — title bar**

In `src/ui/space_select.rs`, find `render_title` (around line 24) and change:

```rust
fn render_title(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new(" lazybacklog").style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}
```

to:

```rust
fn render_title(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new(" lazybacklog").style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: Update `space_select.rs` — list highlight**

In `render_content` (around line 41), change:

```rust
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

to:

```rust
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

- [ ] **Step 5: Build and test**

```bash
cargo build 2>&1 | grep -E "^error"
cargo test 2>&1 | tail -5
```

Expected: no errors, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/project_select.rs src/ui/space_select.rs
git commit -m "style: update title bar and highlight for select screens"
```

---

## Task 2: Style filter popups (filter.rs, status_filter.rs)

**Files:**
- Modify: `src/ui/filter.rs`
- Modify: `src/ui/status_filter.rs`

Change border color from Cyan to DarkGray in all Block instances. Change list highlight to White bg / Black text.

- [ ] **Step 1: Update `filter.rs` — error-state Block border (line ~18)**

Find the Block in the `users_error` early return:

```rust
        Block::default()
            .title(" Assignee Filter ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
```

Change to:

```rust
        Block::default()
            .title(" Assignee Filter ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
```

- [ ] **Step 2: Update `filter.rs` — main list Block border and highlight (lines ~57-69)**

Change the main list Block border:

```rust
    let list = List::new(list_items)
        .block(
            Block::default()
                .title(" Assignee Filter ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

to:

```rust
    let list = List::new(list_items)
        .block(
            Block::default()
                .title(" Assignee Filter ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

- [ ] **Step 3: Update `status_filter.rs` — "Loading..." Block border (line ~23)**

```rust
            Block::default()
                .title(" Status Filter ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
```

Change `.fg(Color::Cyan)` to `.fg(Color::DarkGray)` (first early return block).

- [ ] **Step 4: Update `status_filter.rs` — "No statuses" Block border (line ~33)**

Same change for the second early return block — `.fg(Color::Cyan)` → `.fg(Color::DarkGray)`.

- [ ] **Step 5: Update `status_filter.rs` — main list Block border and highlight (lines ~66-78)**

```rust
    let list = List::new(list_items)
        .block(
            Block::default()
                .title(" Status Filter ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

to:

```rust
    let list = List::new(list_items)
        .block(
            Block::default()
                .title(" Status Filter ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
```

- [ ] **Step 6: Build and test**

```bash
cargo build 2>&1 | grep -E "^error"
cargo test 2>&1 | tail -5
```

Expected: no errors, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/ui/filter.rs src/ui/status_filter.rs
git commit -m "style: update popup borders and highlights to DarkGray/White"
```

---

## Task 3: Style issue list table (issue_list.rs)

**Files:**
- Modify: `src/ui/issue_list.rs`

Change table header color from Yellow to Cyan. Change highlight to White bg / Black text.

- [ ] **Step 1: Update table header color (line ~93)**

Find:

```rust
        .header(
            Row::new(vec!["Key", "Summary", "Assignee", "Status"])
                .style(Style::default().fg(Color::Yellow)),
        )
```

Change to:

```rust
        .header(
            Row::new(vec!["Key", "Summary", "Assignee", "Status"])
                .style(Style::default().fg(Color::Cyan)),
        )
```

- [ ] **Step 2: Update highlight style (lines ~97-101)**

Find:

```rust
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
```

Change to:

```rust
        .highlight_style(
            Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
```

- [ ] **Step 3: Build and test**

```bash
cargo build 2>&1 | grep -E "^error"
cargo test 2>&1 | tail -5
```

Expected: no errors, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/ui/issue_list.rs
git commit -m "style: update issue table header to Cyan, highlight to White/Black"
```

---

## Task 4: Style issue detail screen (issue_detail.rs)

**Files:**
- Modify: `src/ui/issue_detail.rs`

Three changes: title bar style, content block border, field label colors.

- [ ] **Step 1: Update title bar style (lines ~22-26)**

Find the inline title rendering in `render()`:

```rust
    let title_paragraph = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
```

Change to:

```rust
    let title_paragraph = Paragraph::new(title).style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
```

- [ ] **Step 2: Update field label colors (Yellow → Cyan)**

There are 6 `Span::styled(... Style::default().fg(Color::Yellow))` calls in `issue_detail.rs` (for Assignee, Priority, Status, Type, Due, Description). Change all of them from `Color::Yellow` to `Color::Cyan`.

Labels to update:
- `"Assignee: "` (line ~51)
- `"Priority: "` (line ~54)
- `"Status: "` (line ~58)
- `"Type: "` (line ~61)
- `"Due: "` (line ~65)
- `"Description:"` (line ~70)

In each case, change:

```rust
Span::styled("...", Style::default().fg(Color::Yellow))
```

to:

```rust
Span::styled("...", Style::default().fg(Color::Cyan))
```

- [ ] **Step 3: Update content block border (line ~80-83)**

Find:

```rust
    let content_paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
```

Change to:

```rust
    let content_paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
```

> Note: Adding `Borders::ALL` reduces the inner visible area by 2 rows. This is intentional and expected. `scroll_offset` continues to work as-is — no compensating logic needed.

- [ ] **Step 4: Build and test**

```bash
cargo build 2>&1 | grep -E "^error"
cargo test 2>&1 | tail -5
```

Expected: no errors, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/issue_detail.rs
git commit -m "style: update issue detail title bar, content border, and field label colors"
```

---

## Task 5: Restructure issue list layout (mod.rs)

**Files:**
- Modify: `src/ui/mod.rs`

This is the most structural change: title bar inversion, removing the top-level filter constraint, adding a Block panel, and rendering the filter bar inside the panel's inner area.

- [ ] **Step 1: Add `Block` and `Borders` to imports**

In `src/ui/mod.rs`, find the imports:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};
```

Change to:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
```

- [ ] **Step 2: Update `render_title` — Cyan bg / Black text**

Find `render_title` (around line 75):

```rust
fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}
```

Change to:

```rust
fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 3: Restructure the issue list layout**

Find the 4-constraint layout block (around line 44):

```rust
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Length(1), // filter bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0], state);
    render_filter_bar(frame, chunks[1], state);
    issue_list::render(frame, chunks[2], state);
    render_help_bar(frame, chunks[3], state);
```

Replace with:

```rust
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // block panel
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0], state);

    let panel_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let panel_inner = panel_block.inner(chunks[1]);
    frame.render_widget(panel_block, chunks[1]);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // filter bar
            Constraint::Min(0),    // issue list
        ])
        .split(panel_inner);

    render_filter_bar(frame, inner_chunks[0], state);
    issue_list::render(frame, inner_chunks[1], state);
    render_help_bar(frame, chunks[2], state);

// Note: `render_filter_bar`'s function signature does not change — only the area
// argument changes from top-level `chunks[1]` to `inner_chunks[0]`.
```

- [ ] **Step 4: Build**

```bash
cargo build 2>&1 | grep -E "^error"
```

Expected: no errors. If there are import errors, check that `Block` and `Borders` are in the use statement from Step 1.

- [ ] **Step 5: Run tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass. The layout change does not affect any unit tests (they test config parsing and state logic, not rendering).

- [ ] **Step 6: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: restructure issue list layout with bordered panel and inverted title bar"
```

---

## Final Check

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep -E "^error"
```

Expected: no errors. Warnings about unused imports or style are ok to note but not blocking.

- [ ] **Format check**

```bash
cargo fmt --check
```

If it reports differences, run `cargo fmt` and commit:

```bash
cargo fmt
git add -u
git commit -m "style: apply cargo fmt"
```
