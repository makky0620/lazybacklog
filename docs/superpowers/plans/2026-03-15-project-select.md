# Project Selection Screen Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a project selection screen that appears at startup before the issue list, filtering issues to the chosen project.

**Architecture:** New `Screen::ProjectSelect` variant drives a full-screen takeover UI; per-space `selected_project` lives on `SpaceState`; `fetch_projects()` helper mirrors `fetch_issues()`; event loop auto-fetches projects via `needs_projects_fetch()` guard.

**Tech Stack:** Rust, ratatui (TUI), crossterm (keyboard), tokio (async), reqwest (HTTP)

**Spec:** `docs/superpowers/specs/2026-03-15-project-select-design.md`

---

## Chunk 1: API and Event Layer

### Task 1: Add `project_id` filter to `fetch_issues()` in `api/client.rs`

**Files:**
- Modify: `src/api/client.rs`

- [ ] **Step 1: Write the failing test**

Add this test inside the `#[cfg(test)]` block in `src/api/client.rs`, after the existing `test_fetch_issues_with_assignee_filter` test:

```rust
#[tokio::test]
async fn test_fetch_issues_with_project_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/issues"))
        .and(query_param("projectId[]", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let issues = client.fetch_issues(Some(100), None).await.unwrap();
    assert_eq!(issues.len(), 0);
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test test_fetch_issues_with_project_filter 2>&1 | tail -20
```

Expected: compile error — `fetch_issues` does not yet accept two arguments.

- [ ] **Step 3: Update `fetch_issues` signature and implementation**

Change the `fetch_issues` method in `src/api/client.rs`:

```rust
pub async fn fetch_issues(&self, project_id: Option<i64>, assignee_id: Option<i64>) -> Result<Vec<Issue>> {
    let mut params: Vec<(&str, String)> = vec![
        ("apiKey", self.api_key.clone()),
        ("count", "100".to_string()),
    ];
    if let Some(id) = project_id {
        params.push(("projectId[]", id.to_string()));
    }
    if let Some(id) = assignee_id {
        params.push(("assigneeId[]", id.to_string()));
    }
    let resp = self
        .http
        .get(format!("{}/issues", self.base_url))
        .query(&params)
        .send()
        .await
        .context("Failed to connect to Backlog API")?;
    if resp.status() == 401 {
        anyhow::bail!("401 Unauthorized - check your API key");
    }
    resp.error_for_status_ref()
        .context("Backlog API returned an error")?;
    resp.json::<Vec<Issue>>()
        .await
        .context("Failed to parse issues response")
}
```

Also update the three existing call sites in the test module to pass `None` as first arg:
- `test_fetch_issues_success` (line ~147): `client.fetch_issues(None).await` → `client.fetch_issues(None, None).await`
- `test_fetch_issues_with_assignee_filter` (line ~169): `client.fetch_issues(Some(42)).await` → `client.fetch_issues(None, Some(42)).await`
- `test_fetch_issues_401` (line ~183): `client.fetch_issues(None).await` → `client.fetch_issues(None, None).await`

- [ ] **Step 4: Run all `client.rs` tests to verify they pass**

```bash
cargo test --lib api::client 2>&1 | tail -20
```

Expected: all tests pass including `test_fetch_issues_with_project_filter`.

- [ ] **Step 5: Commit**

```bash
git add src/api/client.rs
git commit -m "feat: add project_id filter param to fetch_issues"
```

---

### Task 2: Add `ProjectsLoaded` event to `event.rs`

**Files:**
- Modify: `src/event.rs`

- [ ] **Step 1: Update the import and add the variant**

Replace the contents of `src/event.rs`:

```rust
use crate::api::models::{Issue, Project, User};
use crossterm::event::KeyEvent;

pub enum AppEvent {
    /// Keyboard input from the crossterm reader thread
    Key(KeyEvent),
    /// Issue list fetched for a space
    IssuesLoaded { space: String, issues: Vec<Issue> },
    /// Single issue detail fetched
    IssueDetailLoaded(Issue),
    /// All users for a space fetched and deduplicated by user.id
    SpaceUsersLoaded { space: String, users: Vec<User> },
    /// Projects for a space fetched
    ProjectsLoaded { space: String, projects: Vec<Project> },
    /// Any API error
    ApiError { space: String, message: String },
}
```

- [ ] **Step 2: Verify it compiles (compile errors expected in app.rs/main.rs until Task 3)**

```bash
cargo check 2>&1 | grep "event.rs" | head -10
```

Expected: no errors in `event.rs` itself (errors in `app.rs` and `main.rs` are expected — fix in next tasks).

- [ ] **Step 3: Commit**

```bash
git add src/event.rs
git commit -m "feat: add ProjectsLoaded event variant"
```

---

## Chunk 2: App State

### Task 3: Update `AppState` and `SpaceState` in `app.rs`

**Files:**
- Modify: `src/app.rs`

This is the largest state change. Do it in sub-steps, running `cargo check` after each.

- [ ] **Step 1: Update import and `Screen` enum**

Change line 3 in `src/app.rs`:
```rust
use crate::api::models::{Issue, Project, User};
```

Add `ProjectSelect` to the `Screen` enum:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    ProjectSelect,
    IssueList,
    IssueDetail,
    Filter,
}
```

- [ ] **Step 2: Add fields to `SpaceState`**

Update `SpaceState`:
```rust
#[derive(Debug, Clone, Default)]
pub struct SpaceState {
    pub issues: Option<Vec<Issue>>,
    pub users: Option<Vec<User>>,
    pub users_error: bool,
    pub loading_issues: bool,
    pub projects: Option<Vec<Project>>,
    pub loading_projects: bool,
    pub selected_project: Option<Project>,
}
```

- [ ] **Step 3: Add `project_cursor_idx` to `AppState` and change initial screen**

In `AppState`:
```rust
pub struct AppState {
    pub config: Config,
    pub current_space_idx: usize,
    pub spaces: HashMap<String, SpaceState>,
    pub selected_issue_idx: usize,
    pub detail_issue: Option<Issue>,
    pub filter_assignee_id: Option<i64>,
    pub filter_cursor_idx: usize,
    pub project_cursor_idx: usize,
    pub screen: Screen,
    pub status_message: Option<String>,
    pub should_quit: bool,
}
```

In `AppState::new`, change:
```rust
screen: Screen::ProjectSelect,
```
and add:
```rust
project_cursor_idx: 0,
```

- [ ] **Step 4: Add `selected_project()` and `needs_projects_fetch()` methods**

After `needs_issue_fetch()`, add:
```rust
pub fn needs_projects_fetch(&self) -> bool {
    let state = self.current_space_state();
    state.projects.is_none() && !state.loading_projects
}

pub fn selected_project(&self) -> Option<&Project> {
    self.current_space_state().selected_project.as_ref()
}
```

- [ ] **Step 5: Add `ProjectsLoaded` arm and update `ApiError` arm in `handle_event`**

Add after the `SpaceUsersLoaded` arm:
```rust
AppEvent::ProjectsLoaded { space, projects } => {
    if let Some(state) = self.spaces.get_mut(&space) {
        state.projects = Some(projects);
        state.loading_projects = false;
    }
}
```

In the `ApiError` arm, inside the `if let Some(state)` block, add after `state.loading_issues = false;`:
```rust
state.loading_projects = false;
```

- [ ] **Step 6: Update `switch_space_next` and `switch_space_prev`**

In `switch_space_next`:
- Change `self.screen = Screen::IssueList;` to `self.screen = Screen::ProjectSelect;`
- Add before that line:
  ```rust
  self.project_cursor_idx = 0;
  self.filter_assignee_id = None;
  ```

In `switch_space_prev`: same changes.

- [ ] **Step 7: Run `cargo check` to confirm no errors in `app.rs`**

```bash
cargo check 2>&1 | grep "app.rs" | head -20
```

Expected: no errors in `app.rs` (errors in `main.rs` and `ui/` are expected).

- [ ] **Step 8: Write failing tests**

In the `#[cfg(test)]` block in `src/app.rs`:

1. Update `test_initial_state` — change:
   ```rust
   assert_eq!(state.screen, Screen::IssueList);
   ```
   to:
   ```rust
   assert_eq!(state.screen, Screen::ProjectSelect);
   ```

2. Add new tests after `test_needs_issue_fetch_false_when_loaded`:

```rust
#[test]
fn test_projects_loaded_event() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.handle_event(AppEvent::ProjectsLoaded {
        space: "space1".to_string(),
        projects: vec![
            crate::api::models::Project { id: 1, project_key: "PROJ".to_string(), name: "My Project".to_string() },
        ],
    });
    let projects = state.current_space_state().projects.as_ref().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project_key, "PROJ");
    assert!(!state.current_space_state().loading_projects);
}

#[test]
fn test_api_error_resets_loading_projects() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().loading_projects = true;
    state.handle_event(AppEvent::ApiError {
        space: "space1".to_string(),
        message: "timeout".to_string(),
    });
    assert!(!state.current_space_state().loading_projects);
}

#[test]
fn test_needs_projects_fetch_true_when_no_projects() {
    let config = make_config("space1", &["space1"]);
    let state = AppState::new(config);
    assert!(state.needs_projects_fetch());
}

#[test]
fn test_needs_projects_fetch_false_when_loading() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().loading_projects = true;
    assert!(!state.needs_projects_fetch());
}

#[test]
fn test_switch_space_resets_project_state() {
    let config = make_config("space1", &["space1", "space2"]);
    let mut state = AppState::new(config);
    state.filter_assignee_id = Some(42);
    state.project_cursor_idx = 3;
    state.switch_space_next();
    assert_eq!(state.screen, Screen::ProjectSelect);
    assert_eq!(state.project_cursor_idx, 0);
    assert!(state.filter_assignee_id.is_none());
}
```

- [ ] **Step 9: Run all `app.rs` tests**

```bash
cargo test --lib app 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 10: Commit**

```bash
git add src/app.rs
git commit -m "feat: add ProjectSelect screen and project state to AppState/SpaceState"
```

---

## Chunk 3: UI

### Task 4: Create `src/ui/project_select.rs`

**Files:**
- Create: `src/ui/project_select.rs`

- [ ] **Step 1: Create the file**

Create `src/ui/project_select.rs` with this content:

```rust
use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0], state);
    render_content(frame, chunks[1], state);
    render_help_bar(frame, chunks[2]);
}

fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(paragraph, area);
}

fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    let space_state = state.current_space_state();

    if space_state.loading_projects {
        let loading =
            Paragraph::new("Loading projects...").style(Style::default().fg(Color::Gray));
        frame.render_widget(loading, area);
        return;
    }

    let projects = match &space_state.projects {
        Some(p) if !p.is_empty() => p,
        Some(_) => {
            let msg = Paragraph::new("No projects found.")
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(msg, area);
            return;
        }
        None => {
            let msg = Paragraph::new("No projects found.")
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(msg, area);
            return;
        }
    };

    let items: Vec<ListItem> = projects
        .iter()
        .map(|p| ListItem::new(format!("{} - {}", p.project_key, p.name)))
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.project_cursor_idx));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let text = " [j/k] 移動  [Enter] 選択  [q] 終了";
    let paragraph =
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Verify compile**

```bash
cargo check 2>&1 | grep "project_select" | head -10
```

Expected: error about module not declared yet (will fix in Task 5).

---

### Task 5: Update `src/ui/mod.rs`

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Rewrite `mod.rs`**

Replace the entire contents of `src/ui/mod.rs`:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, Screen};

pub mod filter;
pub mod issue_detail;
pub mod issue_list;
pub mod project_select;

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if state.screen == Screen::ProjectSelect {
        // Full-screen takeover: render project select layout first,
        // then overlay status message on the bottom line.
        // Order matters: project_select::render paints the help bar,
        // render_status_message overlays it when an error is present.
        project_select::render(frame, area, state);
        render_status_message(frame, area, state);
        return;
    }

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
    render_help_bar(frame, chunks[3]);

    match state.screen {
        Screen::IssueDetail => {
            if let Some(issue) = &state.detail_issue {
                issue_detail::render(frame, area, issue);
            }
        }
        Screen::Filter => {
            filter::render(frame, area, state);
        }
        Screen::IssueList => {}
        Screen::ProjectSelect => {} // dead code — early return above handles this; satisfies exhaustiveness
    }

    render_status_message(frame, area, state);
}

fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = format!(" lazybacklog ──── [{}] ", state.current_space_name());
    let paragraph = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(paragraph, area);
}

fn render_filter_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let space_state = state.current_space_state();
    let assignee_name = if let Some(aid) = state.filter_assignee_id {
        space_state
            .users
            .as_ref()
            .and_then(|users| users.iter().find(|u| u.id == aid))
            .map(|u| u.name.clone())
            .unwrap_or_else(|| format!("ID:{}", aid))
    } else {
        "ALL".to_string()
    };

    let text = format!(" Assignee: {}", assignee_name);
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::Gray));
    frame.render_widget(paragraph, area);
}

fn render_help_bar(frame: &mut Frame, area: Rect) {
    let text = " [j/k] 移動  [Enter] 詳細  [f] フィルター  [r] 更新  [[] []] スペース切替  [q] 終了";
    let paragraph =
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

fn render_status_message(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(msg) = &state.status_message {
        let status_area = Rect {
            y: area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        let paragraph =
            Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow));
        frame.render_widget(paragraph, status_area);
    }
}
```

- [ ] **Step 2: Verify `ui/` compiles**

```bash
cargo check 2>&1 | grep "ui" | head -20
```

Expected: no errors in `ui/` files.

- [ ] **Step 3: Commit**

```bash
git add src/ui/project_select.rs src/ui/mod.rs
git commit -m "feat: add project_select full-screen UI widget"
```

---

## Chunk 4: Main Loop Wiring

### Task 6: Update `main.rs` — fetch helpers, startup, key handling, event loop

**Files:**
- Modify: `src/main.rs`

This task wires everything together. The existing `main.rs` has compile errors from the `fetch_issues` signature change. Do in sub-steps.

- [ ] **Step 1: Update `fetch_issues` helper signature and body**

Change the `fetch_issues` function at the bottom of `src/main.rs`:

```rust
fn fetch_issues(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: Option<i64>,
    assignee_id: Option<i64>,
) {
    let space_name = state.current_space_name().to_string();
    let space_cfg = config
        .spaces
        .iter()
        .find(|s| s.name == space_name)
        .unwrap()
        .clone();
    tokio::spawn(async move {
        match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client.fetch_issues(project_id, assignee_id).await {
                Ok(issues) => {
                    let _ = tx.send(AppEvent::IssuesLoaded {
                        space: space_name,
                        issues,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError {
                        space: space_name,
                        message: e.to_string(),
                    });
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::ApiError {
                    space: space_name,
                    message: e.to_string(),
                });
            }
        }
    });
}
```

- [ ] **Step 2: Add `fetch_projects` helper**

Add after `fetch_issues`:

```rust
fn fetch_projects(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let space_name = state.current_space_name().to_string();
    let space_cfg = config
        .spaces
        .iter()
        .find(|s| s.name == space_name)
        .unwrap()
        .clone();
    tokio::spawn(async move {
        match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client.fetch_projects().await {
                Ok(projects) => {
                    let _ = tx.send(AppEvent::ProjectsLoaded {
                        space: space_name,
                        projects,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError {
                        space: space_name,
                        message: e.to_string(),
                    });
                }
            },
            Err(e) => {
                let _ = tx.send(AppEvent::ApiError {
                    space: space_name,
                    message: e.to_string(),
                });
            }
        }
    });
}
```

- [ ] **Step 3: Update startup block in `run()`**

Replace lines 71–115 (the startup section in `run()`) with:

```rust
    // Set loading_projects = true for ALL spaces before spawning, to prevent
    // needs_projects_fetch() from firing while startup tasks are in flight.
    for space in &config.spaces {
        state.spaces.get_mut(&space.name).unwrap().loading_projects = true;
    }

    // Spawn per-space tasks: fetch projects (for ProjectsLoaded) and users.
    for space in &config.spaces {
        let space_name = space.name.clone();
        let host = space.host.clone();
        let api_key = space.api_key.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match api::client::BacklogClient::new(host, api_key) {
                Ok(client) => match client.fetch_projects().await {
                    Ok(projects) => {
                        // Send ProjectsLoaded for every space.
                        // Clone into the event; borrow original for user iteration below.
                        let _ = tx.send(AppEvent::ProjectsLoaded {
                            space: space_name.clone(),
                            projects: projects.clone(),
                        });
                        // Fetch users for each project (iterate by reference, not by move).
                        let mut all_users: Vec<api::models::User> = Vec::new();
                        for project in &projects {
                            if let Ok(users) = client.fetch_project_users(project.id).await {
                                for user in users {
                                    if !all_users.iter().any(|u| u.id == user.id) {
                                        all_users.push(user);
                                    }
                                }
                            }
                        }
                        let _ = tx.send(AppEvent::SpaceUsersLoaded {
                            space: space_name,
                            users: all_users,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::ApiError {
                            space: space_name,
                            message: e.to_string(),
                        });
                    }
                },
                Err(e) => {
                    let _ = tx.send(AppEvent::ApiError {
                        space: space_name,
                        message: e.to_string(),
                    });
                }
            }
        });
    }
    // No initial fetch_issues — user selects project first.
```

- [ ] **Step 4: Update key dispatch match in the event loop**

Find the `match ev` block in `run()` and update the key dispatch arm:

```rust
AppEvent::Key(key) => match state.screen {
    Screen::IssueList => handle_list_key(key, &mut state, &config, tx.clone()),
    Screen::IssueDetail => handle_detail_key(key, &mut state),
    Screen::Filter => handle_filter_key(key, &mut state, &config, tx.clone()),
    Screen::ProjectSelect => handle_project_select_key(key, &mut state, &config, tx.clone()),
},
```

- [ ] **Step 5: Update event loop `other =>` arm**

Replace the existing `other =>` arm:

```rust
other => {
    state.handle_event(other);
    // Guard 1: issue auto-fetch (only on IssueList screen)
    if state.screen == Screen::IssueList && state.needs_issue_fetch() {
        let project_id = state.selected_project().map(|p| p.id);
        let assignee_id = state.filter_assignee_id;
        fetch_issues(&state, &config, tx.clone(), project_id, assignee_id);
        state.current_space_state_mut().loading_issues = true;
    }
    // Guard 2: project auto-fetch (only on ProjectSelect screen)
    // Fires when user switches to a space whose projects were not yet loaded.
    if state.screen == Screen::ProjectSelect && state.needs_projects_fetch() {
        fetch_projects(&state, &config, tx.clone());
        state.current_space_state_mut().loading_projects = true;
    }
}
```

- [ ] **Step 6: Update `handle_list_key` — remove issue-fetch from `[`/`]` handlers and update `'r'`**

In `handle_list_key`:

**`'r'` key handler** — add `project_id`:
```rust
KeyCode::Char('r') => {
    let project_id = state.selected_project().map(|p| p.id);
    let assignee_id = state.filter_assignee_id;
    state.current_space_state_mut().issues = None;
    state.current_space_state_mut().loading_issues = true;
    fetch_issues(state, config, tx, project_id, assignee_id);
}
```

**`']'` key handler** — remove the `needs_issue_fetch` block, keep only the space switch:
```rust
KeyCode::Char(']') => {
    state.switch_space_next();
}
```

**`'['` key handler** — same:
```rust
KeyCode::Char('[') => {
    state.switch_space_prev();
}
```

- [ ] **Step 7: Update `handle_filter_key` — add `project_id` to fetch call**

In the `Enter` arm of `handle_filter_key`, update the `fetch_issues` call:
```rust
KeyCode::Enter => {
    if state.filter_cursor_idx == 0 {
        state.filter_assignee_id = None;
    } else {
        let users = state.current_space_state().users.clone();
        if let Some(users) = users {
            if let Some(user) = users.get(state.filter_cursor_idx - 1) {
                state.filter_assignee_id = Some(user.id);
            }
        }
    }
    state.screen = Screen::IssueList;
    let project_id = state.selected_project().map(|p| p.id);
    let assignee_id = state.filter_assignee_id;
    state.current_space_state_mut().issues = None;
    state.current_space_state_mut().loading_issues = true;
    fetch_issues(state, config, tx, project_id, assignee_id);
}
```

- [ ] **Step 8: Add `handle_project_select_key`**

Add this new function before `fetch_issues`:

```rust
fn handle_project_select_key(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let project_count = state
        .current_space_state()
        .projects
        .as_ref()
        .map(|p| p.len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            if project_count > 0 && state.project_cursor_idx + 1 < project_count {
                state.project_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.project_cursor_idx > 0 {
                state.project_cursor_idx -= 1;
            }
        }
        KeyCode::Enter => {
            if project_count == 0 {
                // No project to select — no-op
                return;
            }
            // Clone the selected project and store it on SpaceState
            let project = state
                .current_space_state()
                .projects
                .as_ref()
                .and_then(|p| p.get(state.project_cursor_idx))
                .cloned();
            if let Some(project) = project {
                let project_id = project.id;
                state.current_space_state_mut().selected_project = Some(project);
                state.screen = Screen::IssueList;
                state.current_space_state_mut().issues = None;
                state.current_space_state_mut().loading_issues = true;
                fetch_issues(state, config, tx, Some(project_id), state.filter_assignee_id);
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 9: Verify full compile**

```bash
cargo build 2>&1 | tail -20
```

Expected: compiles with no errors.

- [ ] **Step 10: Run all tests**

```bash
cargo test 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 11: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire project selection into startup, key handling, and event loop"
```

---

## Final Verification

- [ ] **Smoke test the binary**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: builds successfully.

- [ ] **Run full test suite one final time**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass, zero failures.
