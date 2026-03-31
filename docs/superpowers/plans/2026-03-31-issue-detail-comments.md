# Issue Detail Comments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display Backlog issue comments below the description in the issue detail screen, fetched automatically on open.

**Architecture:** Add a `Comment` model and `fetch_comments` API method, introduce a `CommentsLoaded` event and `detail_comments` state field, spawn the comments fetch concurrently with `fetch_issue` on Enter, and render comments as a unified scrollable block beneath the description.

**Tech Stack:** Rust, ratatui, reqwest, tokio, serde, wiremock (tests)

---

### Task 1: Add Comment model

**Files:**
- Modify: `src/api/models.rs`

- [ ] **Step 1: Write failing deserialization tests**

Add inside the existing `#[cfg(test)]` block in `src/api/models.rs`:

```rust
#[test]
fn test_deserialize_comment() {
    let json = serde_json::json!({
        "id": 1,
        "content": "Hello",
        "createdUser": { "id": 10, "name": "Alice" },
        "created": "2026-03-31T12:00:00Z"
    });
    let comment: Comment = serde_json::from_value(json).unwrap();
    assert_eq!(comment.id, 1);
    assert_eq!(comment.content.as_deref(), Some("Hello"));
    assert_eq!(comment.created_user.unwrap().name, "Alice");
    assert_eq!(comment.created, "2026-03-31T12:00:00Z");
}

#[test]
fn test_deserialize_comment_null_fields() {
    let json = serde_json::json!({
        "id": 2,
        "content": null,
        "createdUser": null,
        "created": "2026-03-31T12:00:00Z"
    });
    let comment: Comment = serde_json::from_value(json).unwrap();
    assert!(comment.content.is_none());
    assert!(comment.created_user.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_deserialize_comment
```

Expected: compile error — `Comment` not defined.

- [ ] **Step 3: Add Comment struct**

Add after the `IssueType` struct in `src/api/models.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub content: Option<String>,
    #[serde(rename = "createdUser")]
    pub created_user: Option<User>,
    pub created: String,
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test test_deserialize_comment
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/api/models.rs
git commit -m "feat: add Comment model with createdUser deserialization"
```

---

### Task 2: Add fetch_comments API method

**Files:**
- Modify: `src/api/client.rs`

- [ ] **Step 1: Write failing wiremock tests**

Add inside the `#[cfg(test)]` block in `src/api/client.rs`:

```rust
#[tokio::test]
async fn test_fetch_comments_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/issues/PROJ-1/comments"))
        .and(query_param("apiKey", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 1,
                "content": "First comment",
                "createdUser": { "id": 10, "name": "Alice" },
                "created": "2026-03-31T12:00:00Z"
            }
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let comments = client.fetch_comments("PROJ-1").await.unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].content.as_deref(), Some("First comment"));
    assert_eq!(comments[0].created_user.as_ref().unwrap().name, "Alice");
}

#[tokio::test]
async fn test_fetch_comments_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/issues/PROJ-1/comments"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let err = client.fetch_comments("PROJ-1").await.unwrap_err();
    assert!(err.to_string().contains("401 Unauthorized"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_fetch_comments
```

Expected: compile error — `fetch_comments` not defined.

- [ ] **Step 3: Add Comment to imports and fetch_comments method**

Change the import line at the top of `src/api/client.rs`:

```rust
use super::models::{Comment, Issue, IssueStatus, Project, User};
```

Add the method after `fetch_issue` in `src/api/client.rs`:

```rust
pub async fn fetch_comments(&self, issue_id_or_key: &str) -> Result<Vec<Comment>> {
    let resp = self
        .http
        .get(format!("{}/issues/{}/comments", self.base_url, issue_id_or_key))
        .query(&[("apiKey", &self.api_key)])
        .send()
        .await
        .context("Failed to connect to Backlog API")?;
    if resp.status() == 401 {
        anyhow::bail!("401 Unauthorized - check your API key");
    }
    resp.error_for_status_ref()
        .context("Backlog API returned an error")?;
    resp.json::<Vec<Comment>>()
        .await
        .context("Failed to parse comments response")
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test test_fetch_comments
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/api/client.rs
git commit -m "feat: add fetch_comments API method"
```

---

### Task 3: CommentsLoaded event, state fields, and event handlers

**Files:**
- Modify: `src/event.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Write failing state tests**

Add inside the `#[cfg(test)]` block in `src/app.rs` (after existing tests):

```rust
#[test]
fn test_comments_loaded_sets_detail_comments() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config, false);
    state.detail_issue = Some(make_issue("PROJ-1"));
    state.handle_event(AppEvent::CommentsLoaded {
        issue_key: "PROJ-1".to_string(),
        comments: vec![],
    });
    assert!(state.detail_comments.is_some());
}

#[test]
fn test_comments_loaded_wrong_key_ignored() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config, false);
    state.detail_issue = Some(make_issue("PROJ-1"));
    state.handle_event(AppEvent::CommentsLoaded {
        issue_key: "PROJ-99".to_string(),
        comments: vec![],
    });
    assert!(state.detail_comments.is_none());
}

#[test]
fn test_issue_detail_loaded_resets_comments() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config, false);
    state.detail_comments = Some(vec![]);
    state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-1")));
    assert!(state.detail_comments.is_none());
}

#[test]
fn test_select_space_resets_comments() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config, false);
    state.detail_comments = Some(vec![]);
    state.select_space(0);
    assert!(state.detail_comments.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_comments_loaded test_issue_detail_loaded_resets_comments test_select_space_resets_comments
```

Expected: compile errors — `CommentsLoaded` not defined, `detail_comments` not a field.

- [ ] **Step 3: Add CommentsLoaded to event.rs**

Change the import line in `src/event.rs`:

```rust
use crate::api::models::{Comment, Issue, IssueStatus, Project, User};
```

Add the new variant to the `AppEvent` enum in `src/event.rs`:

```rust
/// Comments for a single issue fetched
CommentsLoaded { issue_key: String, comments: Vec<Comment> },
```

- [ ] **Step 4: Add detail_comments field to AppState**

Change the import line in `src/app.rs`:

```rust
use crate::api::models::{Comment, Issue, IssueStatus, Project, User};
```

Add the field to the `AppState` struct (after `detail_issue`):

```rust
pub detail_comments: Option<Vec<Comment>>,
```

Add initialisation in `AppState::new` (after `detail_issue: None`):

```rust
detail_comments: None,
```

Add reset in `select_space` (after `self.detail_issue = None;`):

```rust
self.detail_comments = None;
```

- [ ] **Step 5: Add CommentsLoaded arm and update IssueDetailLoaded in handle_event**

Update the `IssueDetailLoaded` arm in `handle_event`:

```rust
AppEvent::IssueDetailLoaded(issue) => {
    self.clear_search();
    self.detail_issue = Some(issue);
    self.detail_scroll_offset = 0;
    self.detail_comments = None;
    self.screen = Screen::IssueDetail;
}
```

Add a new arm after `IssueDetailLoaded` in `handle_event`:

```rust
AppEvent::CommentsLoaded { issue_key, comments } => {
    if self.detail_issue.as_ref().map(|i| &i.issue_key) == Some(&issue_key) {
        self.detail_comments = Some(comments);
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cargo test test_comments_loaded test_issue_detail_loaded_resets_comments test_select_space_resets_comments
```

Expected: 4 tests pass.

- [ ] **Step 7: Run full test suite**

```bash
cargo test
```

Expected: all tests pass (the new `CommentsLoaded` variant must be handled in all `match AppEvent` exhaustiveness checks — the compiler will flag any missing arms).

- [ ] **Step 8: Commit**

```bash
git add src/event.rs src/app.rs
git commit -m "feat: add CommentsLoaded event and detail_comments state"
```

---

### Task 4: Spawn fetch_comments on Enter; clear on Esc

**Files:**
- Modify: `src/handler.rs`

- [ ] **Step 1: Write failing test**

Add inside the `#[cfg(test)]` block in `src/handler.rs`:

```rust
#[test]
fn test_detail_key_esc_clears_comments() {
    let mut state = make_state();
    state.screen = Screen::IssueDetail;
    state.detail_issue = Some(crate::api::models::Issue {
        id: 1,
        issue_key: "PROJ-1".to_string(),
        summary: "test".to_string(),
        description: None,
        assignee: None,
        status: IssueStatus { id: 1, name: "Open".to_string() },
        priority: None,
        issue_type: None,
        due_date: None,
    });
    state.detail_comments = Some(vec![]);
    handle_detail_key(key(KeyCode::Esc), &mut state);
    assert!(state.detail_comments.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_detail_key_esc_clears_comments
```

Expected: compile error or assertion failure — `detail_comments` not cleared on Esc.

- [ ] **Step 3: Update handle_detail_key Esc arm**

Replace the `KeyCode::Esc` arm in `handle_detail_key`:

```rust
KeyCode::Esc => {
    state.screen = Screen::IssueList;
    state.detail_issue = None;
    state.detail_scroll_offset = 0;
    state.detail_comments = None;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test test_detail_key_esc_clears_comments
```

Expected: 1 test passes.

- [ ] **Step 5: Update Enter key handler to spawn fetch_comments concurrently**

Replace the entire `KeyCode::Enter` arm in `handle_list_key` (the one that spawns `tokio::spawn` for `fetch_issue`):

```rust
KeyCode::Enter => {
    if state.demo_mode {
        if let Some(issue) = state.selected_issue().cloned() {
            state.detail_comments = Some(vec![]);
            let _ = tx.send(AppEvent::IssueDetailLoaded(issue));
        }
        return;
    }
    if let Some(issue) = state.selected_issue() {
        let issue_key = issue.issue_key.clone();
        let space_name = state.current_space_name().to_string();
        let space_cfg = config
            .spaces
            .iter()
            .find(|s| s.name == space_name)
            .unwrap()
            .clone();
        state.detail_comments = None;
        // spawn fetch_issue
        let tx1 = tx.clone();
        let issue_key1 = issue_key.clone();
        let space_cfg1 = space_cfg.clone();
        let space_name1 = space_name.clone();
        tokio::spawn(async move {
            match api::client::BacklogClient::new(space_cfg1.host, space_cfg1.api_key) {
                Ok(client) => match client.fetch_issue(&issue_key1).await {
                    Ok(issue) => {
                        let _ = tx1.send(AppEvent::IssueDetailLoaded(issue));
                    }
                    Err(e) => {
                        let _ = tx1.send(AppEvent::ApiError {
                            space: space_name1,
                            message: e.to_string(),
                        });
                    }
                },
                Err(e) => {
                    let _ = tx1.send(AppEvent::ApiError {
                        space: space_name1,
                        message: e.to_string(),
                    });
                }
            }
        });
        // spawn fetch_comments
        let tx2 = tx.clone();
        let space_name2 = space_name.clone();
        tokio::spawn(async move {
            match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
                Ok(client) => match client.fetch_comments(&issue_key).await {
                    Ok(comments) => {
                        let _ = tx2.send(AppEvent::CommentsLoaded {
                            issue_key,
                            comments,
                        });
                    }
                    Err(e) => {
                        let _ = tx2.send(AppEvent::ApiError {
                            space: space_name2,
                            message: e.to_string(),
                        });
                    }
                },
                Err(e) => {
                    let _ = tx2.send(AppEvent::ApiError {
                        space: space_name2,
                        message: e.to_string(),
                    });
                }
            }
        });
    }
}
```

- [ ] **Step 6: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/handler.rs
git commit -m "feat: spawn fetch_comments concurrently with fetch_issue on Enter"
```

---

### Task 5: Render comments in issue detail UI

**Files:**
- Modify: `src/ui/issue_detail.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write failing UI tests**

Replace the existing `issue_detail_shows_description_block_title` test and add new ones in `src/ui/issue_detail.rs`:

```rust
#[test]
fn issue_detail_shows_description_and_comments_block_title() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let issue = make_issue();
    terminal
        .draw(|frame| render(frame, frame.area(), &issue, 0, None))
        .unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        content.contains("Description"),
        "Expected 'Description' in block title, got: {:?}",
        content
    );
}

#[test]
fn issue_detail_shows_loading_when_comments_none() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let issue = make_issue();
    terminal
        .draw(|frame| render(frame, frame.area(), &issue, 0, None))
        .unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        content.contains("loading comments"),
        "Expected loading indicator, got: {:?}",
        content
    );
}

#[test]
fn issue_detail_shows_comment_content() {
    use crate::api::models::Comment;
    let backend = TestBackend::new(60, 25);
    let mut terminal = Terminal::new(backend).unwrap();
    let issue = make_issue();
    let comments = vec![Comment {
        id: 1,
        content: Some("Great bug report".to_string()),
        created_user: Some(crate::api::models::User { id: 10, name: "Alice".to_string() }),
        created: "2026-03-31T12:00:00Z".to_string(),
    }];
    terminal
        .draw(|frame| render(frame, frame.area(), &issue, 0, Some(&comments)))
        .unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        content.contains("Great bug report"),
        "Expected comment content, got: {:?}",
        content
    );
}
```

Also update the existing render calls in tests that still pass 4 args — change:
```rust
render(frame, frame.area(), &issue, 0)
```
to:
```rust
render(frame, frame.area(), &issue, 0, None)
```
(applies to `issue_detail_shows_issue_key_in_details_title`, `issue_detail_no_cyan_title_bar_at_top`, and any other existing tests in this file)

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test '*' -- issue_detail
cargo test issue_detail
```

Expected: compile errors — `render` has wrong number of arguments.

- [ ] **Step 3: Update issue_detail.rs**

Add `Comment` to the imports at the top of `src/ui/issue_detail.rs`:

```rust
use crate::api::models::{Comment, Issue};
```

Change the `render` function signature:

```rust
pub fn render(frame: &mut Frame, area: Rect, issue: &Issue, scroll_offset: u16, comments: Option<&[Comment]>) {
```

Update the body of `render` to call the renamed function:

```rust
pub fn render(frame: &mut Frame, area: Rect, issue: &Issue, scroll_offset: u16, comments: Option<&[Comment]>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // details block (3 content lines + 2 border)
            Constraint::Min(0),    // description + comments block
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_details(frame, chunks[0], issue);
    render_description_and_comments(frame, chunks[1], issue, scroll_offset, comments);
    render_help_bar(frame, chunks[2]);
}
```

Replace the `render_description` function entirely with:

```rust
fn render_description_and_comments(
    frame: &mut Frame,
    area: Rect,
    issue: &Issue,
    scroll_offset: u16,
    comments: Option<&[Comment]>,
) {
    let description = issue.description.as_deref().unwrap_or("(no description)");
    let mut lines: Vec<Line> = description
        .lines()
        .map(|l| Line::from(l.to_string()))
        .collect();

    match comments {
        None => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "(loading comments...)",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Some(comments) => {
            for (i, comment) in comments.iter().enumerate() {
                let author = comment
                    .created_user
                    .as_ref()
                    .map(|u| u.name.as_str())
                    .unwrap_or("?");
                let date = comment.created.get(..10).unwrap_or(&comment.created);
                let separator = format!("── {}: {}  {} ──", i + 1, author, date);
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    separator,
                    Style::default().fg(Color::DarkGray),
                )));
                let content = comment.content.as_deref().unwrap_or("");
                for cl in content.lines() {
                    lines.push(Line::from(cl.to_string()));
                }
            }
        }
    }

    let block = Block::default()
        .title(" Description & Comments ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: Update ui/mod.rs to pass comments**

Change the `issue_detail::render` call in `src/ui/mod.rs`:

```rust
if state.screen == Screen::IssueDetail {
    if let Some(issue) = &state.detail_issue {
        issue_detail::render(
            frame,
            area,
            issue,
            state.detail_scroll_offset,
            state.detail_comments.as_deref(),
        );
    }
    render_status_message(frame, area, state);
    return;
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test issue_detail
```

Expected: all issue_detail tests pass.

- [ ] **Step 6: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 7: Build release to confirm no warnings**

```bash
cargo build --release
```

Expected: builds cleanly.

- [ ] **Step 8: Commit**

```bash
git add src/ui/issue_detail.rs src/ui/mod.rs
git commit -m "feat: render comments below description in issue detail"
```
