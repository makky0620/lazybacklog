# Open Issue in Browser — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Press `o` in the Issue Detail screen to open the Backlog issue page in the system default browser.

**Architecture:** Add the `open` crate, extend `handle_detail_key` with a new `'o'` match arm that builds the URL from the active space host and issue key, then calls `open::that()`. Guard the call with `#[cfg(not(test))]` to keep tests side-effect free.

**Tech Stack:** Rust, `open = "5"` crate, ratatui TUI

---

### Task 1: Add `open` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, add under `[dependencies]`:
```toml
open = "5"
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build
```
Expected: compiles without errors (no code changes yet, so no new functionality).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add open crate for browser integration"
```

---

### Task 2: Write baseline tests for the `'o'` key handler

**Files:**
- Modify: `src/handler.rs` (test module only)

The existing test module is at the bottom of `src/handler.rs`. It already has a `make_state()` helper, a `key(code)` helper, and tests for `handle_detail_key`. Add two new tests alongside them.

- [ ] **Step 1: Write the failing tests**

In the `#[cfg(test)]` module in `src/handler.rs`, add after the last `handle_detail_key` test:

```rust
#[test]
fn test_detail_key_o_with_issue_does_not_change_state() {
    let mut state = make_state();
    state.screen = Screen::IssueDetail;
    state.detail_scroll_offset = 2;
    state.detail_issue = Some(crate::api::models::Issue {
        id: 1,
        issue_key: "PROJ-1".to_string(),
        summary: "test".to_string(),
        description: None,
        assignee: None,
        status: IssueStatus {
            id: 1,
            name: "Open".to_string(),
        },
        priority: None,
        issue_type: None,
        due_date: None,
    });
    handle_detail_key(key(KeyCode::Char('o')), &mut state);
    assert_eq!(state.screen, Screen::IssueDetail);
    assert_eq!(state.detail_scroll_offset, 2);
    assert!(state.detail_issue.is_some());
}

#[test]
fn test_detail_key_o_without_issue_is_noop() {
    let mut state = make_state();
    state.screen = Screen::IssueDetail;
    state.detail_issue = None;
    handle_detail_key(key(KeyCode::Char('o')), &mut state);
    assert_eq!(state.screen, Screen::IssueDetail);
    assert!(state.detail_issue.is_none());
}
```

- [ ] **Step 2: Run tests to confirm baseline**

These are baseline regression tests. `'o'` currently falls through to `_ => {}` so state is already unchanged — the tests pass before implementation, defining the contract the implementation must preserve.

```bash
cargo test test_detail_key_o
```
Expected: both tests PASS.

- [ ] **Step 3: Commit the tests**

```bash
git add src/handler.rs
git commit -m "test: add baseline tests for 'o' key in issue detail"
```

---

### Task 3: Implement the `'o'` key handler

**Files:**
- Modify: `src/handler.rs` (production code)

- [ ] **Step 1: Add the match arm to `handle_detail_key`**

Find `handle_detail_key` in `src/handler.rs` (around line 162). The current match body looks like:

```rust
pub fn handle_detail_key(key: KeyEvent, state: &mut AppState) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => { ... }
        KeyCode::Char('k') | KeyCode::Up => { ... }
        KeyCode::Esc => { ... }
        _ => {}
    }
}
```

Add the new arm **before** `_ => {}`:

```rust
KeyCode::Char('o') => {
    if let Some(issue) = &state.detail_issue {
        let url = format!(
            "https://{}/view/{}",
            state.config.spaces[state.current_space_idx].host,
            issue.issue_key
        );
        #[cfg(not(test))]
        let _ = open::that(url);
    }
}
```

- [ ] **Step 2: Run tests to confirm they still pass**

```bash
cargo test test_detail_key_o
```
Expected: both tests PASS (state is unchanged, browser call is skipped in test cfg).

- [ ] **Step 3: Run all tests to confirm nothing is broken**

```bash
cargo test
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/handler.rs
git commit -m "feat: open issue in browser with 'o' key from detail screen"
```

---

### Task 4: Update the help bar

**Files:**
- Modify: `src/ui/issue_detail.rs`

- [ ] **Step 1: Update the help bar string**

In `src/ui/issue_detail.rs` (around line 93), find:

```rust
Paragraph::new(" [j/k] Scroll  [Esc] Back")
```

Change it to:

```rust
Paragraph::new(" [j/k] Scroll  [o] Open  [Esc] Back")
```

- [ ] **Step 2: Run the app to visually verify**

```bash
cargo run -- --demo
```

Navigate to any issue detail screen and confirm the help bar shows `[o] Open`.

- [ ] **Step 3: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/ui/issue_detail.rs
git commit -m "feat: add [o] Open hint to issue detail help bar"
```
