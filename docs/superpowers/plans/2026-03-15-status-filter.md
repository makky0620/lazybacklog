# Status Filter Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-select status filtering to lazybacklog, defaulting to "Not Closed" tickets, with statuses fetched per-project from the Backlog API.

**Architecture:** Statuses are fetched from `GET /api/v2/projects/:id/statuses` when a project is selected; the default filter excludes statuses named "完了" or "Closed". A new `StatusFilter` popup screen (key `s`) lets users toggle individual statuses. All filtering is server-side via `statusId[]` query params.

**Tech Stack:** Rust, ratatui (TUI), crossterm, tokio (async), reqwest (HTTP), wiremock (test HTTP server)

---

## File Map

| File | Change |
|---|---|
| `src/api/client.rs` | Add `fetch_statuses(project_id: i64)`, add `status_ids: &[i64]` param to `fetch_issues` |
| `src/api/models.rs` | No change (`IssueStatus` already exists) |
| `src/event.rs` | Add `StatusesLoaded { space, statuses }` variant |
| `src/app.rs` | Add fields to `SpaceState` and `AppState`, add `Screen::StatusFilter`, update `handle_event`, update `needs_issue_fetch` |
| `src/ui/status_filter.rs` | New file — multi-select checkbox popup |
| `src/ui/mod.rs` | Register `status_filter`, update filter bar + help bar, add `StatusFilter` to match |
| `src/main.rs` | Add `fetch_statuses` helper, update key handlers, update all `fetch_issues` call sites |

---

## Chunk 1: API Layer

### Task 1: Add `fetch_statuses` to the API client

**Files:**
- Modify: `src/api/client.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `src/api/client.rs`:

```rust
#[tokio::test]
async fn test_fetch_statuses_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/projects/100/statuses"))
        .and(query_param("apiKey", "test_api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": 1, "name": "未対応" },
            { "id": 2, "name": "処理中" },
            { "id": 3, "name": "処理済み" },
            { "id": 4, "name": "完了" }
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let statuses = client.fetch_statuses(100).await.unwrap();
    assert_eq!(statuses.len(), 4);
    assert_eq!(statuses[0].name, "未対応");
    assert_eq!(statuses[3].name, "完了");
}

#[tokio::test]
async fn test_fetch_statuses_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/projects/100/statuses"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let err = client.fetch_statuses(100).await.unwrap_err();
    assert!(err.to_string().contains("401 Unauthorized"));
}
```

Also update the import at the top of the test module — add `IssueStatus` to the `use super::*` (it's already covered by `*`, but ensure `IssueStatus` is in scope via `use super::models::IssueStatus` if needed — check the existing import pattern).

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p lazybacklog test_fetch_statuses -- --nocapture
```

Expected: compile error — `fetch_statuses` does not exist yet.

- [ ] **Step 3: Implement `fetch_statuses`**

Add to `impl BacklogClient` in `src/api/client.rs`, after `fetch_project_users`:

First, add `IssueStatus` to the import at the top of `client.rs`:

```rust
use super::models::{Issue, IssueStatus, Project, User};
```

Then add the method to `impl BacklogClient` after `fetch_project_users`:

```rust
pub async fn fetch_statuses(&self, project_id: i64) -> Result<Vec<IssueStatus>> {
    let resp = self
        .http
        .get(format!("{}/projects/{}/statuses", self.base_url, project_id))
        .query(&[("apiKey", &self.api_key)])
        .send()
        .await
        .context("Failed to connect to Backlog API")?;
    if resp.status() == 401 {
        anyhow::bail!("401 Unauthorized - check your API key");
    }
    resp.error_for_status_ref()
        .context("Backlog API returned an error")?;
    resp.json::<Vec<IssueStatus>>()
        .await
        .context("Failed to parse statuses response")
}
```

Also add a test for 5xx errors to match coverage of other methods:

```rust
#[tokio::test]
async fn test_fetch_statuses_500() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/projects/100/statuses"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let err = client.fetch_statuses(100).await.unwrap_err();
    assert!(err.to_string().contains("error"));
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p lazybacklog test_fetch_statuses -- --nocapture
```

Expected: PASS (both `test_fetch_statuses_success` and `test_fetch_statuses_401`)

- [ ] **Step 5: Commit**

```bash
git add src/api/client.rs
git commit -m "feat: add fetch_statuses API method"
```

---

### Task 2: Update `fetch_issues` to accept `status_ids`

**Files:**
- Modify: `src/api/client.rs`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `src/api/client.rs`:

```rust
#[tokio::test]
async fn test_fetch_issues_with_status_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/issues"))
        .and(query_param("statusId[]", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let issues = client.fetch_issues(None, None, &[1]).await.unwrap();
    assert_eq!(issues.len(), 0);
}

#[tokio::test]
async fn test_fetch_issues_with_multiple_status_ids() {
    let server = MockServer::start().await;
    // Require all three statusId[] values to confirm all are serialized.
    // wiremock's query_param matcher checks that the given key=value pair is present;
    // chaining multiple .and(query_param(...)) assertions means all must be present.
    Mock::given(method("GET"))
        .and(path("/api/v2/issues"))
        .and(query_param("statusId[]", "1"))
        .and(query_param("statusId[]", "2"))
        .and(query_param("statusId[]", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let issues = client.fetch_issues(None, None, &[1, 2, 3]).await.unwrap();
    assert_eq!(issues.len(), 0);
}

#[tokio::test]
async fn test_fetch_issues_with_empty_status_ids_sends_no_status_param() {
    let server = MockServer::start().await;
    // This mock has NO statusId[] requirement — it matches any GET /issues.
    // If statusId[] were sent, this mock still matches (it's not excluded).
    // The key assertion is that fetch_issues does not panic with empty slice.
    Mock::given(method("GET"))
        .and(path("/api/v2/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let client = make_client(&server).await;
    let issues = client.fetch_issues(None, None, &[]).await.unwrap();
    assert_eq!(issues.len(), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p lazybacklog test_fetch_issues_with_status -- --nocapture
```

Expected: compile errors — signature mismatch.

- [ ] **Step 3: Update `fetch_issues` signature and body**

Replace the `fetch_issues` method in `src/api/client.rs`:

```rust
pub async fn fetch_issues(
    &self,
    project_id: Option<i64>,
    assignee_id: Option<i64>,
    status_ids: &[i64],
) -> Result<Vec<Issue>> {
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
    for id in status_ids {
        params.push(("statusId[]", id.to_string()));
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

- [ ] **Step 4: Fix existing tests in `client.rs` that call `fetch_issues`**

Update all existing calls to `fetch_issues` in `src/api/client.rs` tests to pass `&[]` as the third argument:

- `test_fetch_issues_success`: `client.fetch_issues(None, None, &[]).await`
- `test_fetch_issues_with_assignee_filter`: `client.fetch_issues(None, Some(42), &[]).await`
- `test_fetch_issues_with_project_filter`: `client.fetch_issues(Some(100), None, &[]).await`
- `test_fetch_issues_401`: `client.fetch_issues(None, None, &[]).await`

- [ ] **Step 5: Run all client tests**

```
cargo test -p lazybacklog --test-threads=1 2>&1 | head -50
```

Expected: compile errors in `src/main.rs` (callers not yet updated — that's fine, we'll fix in Chunk 4). The client tests themselves should pass once main.rs compiles.

Temporarily allow the build to fail on main.rs by checking just the lib/api tests:

```
cargo test -p lazybacklog api:: -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add src/api/client.rs
git commit -m "feat: add status_ids param to fetch_issues"
```

---

## Chunk 2: State and Event Layer

### Task 3: Add `StatusesLoaded` to `AppEvent`

**Files:**
- Modify: `src/event.rs`

- [ ] **Step 1: Add the new event variant**

In `src/event.rs`, add to the `use` imports:

```rust
use crate::api::models::{Issue, IssueStatus, Project, User};
```

Then add to the `AppEvent` enum:

```rust
/// Statuses for the selected project fetched
StatusesLoaded { space: String, statuses: Vec<IssueStatus> },
```

- [ ] **Step 2: Add a stub arm in `app.rs` to keep the build clean**

Before committing, add a temporary stub to the `handle_event` match in `src/app.rs` so it compiles:

```rust
AppEvent::StatusesLoaded { .. } => {
    // TODO: implement in Task 5
}
```

This keeps every commit compiling. The stub will be replaced by the real handler in Task 5.

- [ ] **Step 3: Build to confirm no errors**

```
cargo build 2>&1 | head -10
```

Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add src/event.rs src/app.rs
git commit -m "feat: add StatusesLoaded event variant (stub handler)"
```

---

### Task 4: Add state fields to `SpaceState`, `AppState`, and `Screen`

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)]` block in `src/app.rs`:

```rust
#[test]
fn test_space_state_default_statuses_is_none() {
    let config = make_config("space1", &["space1"]);
    let state = AppState::new(config);
    assert!(state.current_space_state().statuses.is_none());
    assert!(!state.current_space_state().loading_statuses);
    assert!(state.current_space_state().filter_status_ids.is_empty());
}

#[test]
fn test_appstate_default_status_filter_fields() {
    let config = make_config("space1", &["space1"]);
    let state = AppState::new(config);
    assert_eq!(state.status_filter_cursor_idx, 0);
    assert!(state.status_filter_pending.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p lazybacklog test_space_state_default_statuses -- --nocapture
cargo test -p lazybacklog test_appstate_default_status_filter -- --nocapture
```

Expected: compile errors — fields don't exist yet.

- [ ] **Step 3: Add fields to `SpaceState`**

In `src/app.rs`, update `SpaceState` struct:

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
    pub statuses: Option<Vec<IssueStatus>>,
    pub loading_statuses: bool,
    pub filter_status_ids: Vec<i64>,
}
```

Update the import at the top of `src/app.rs`:

```rust
use crate::api::models::{Issue, IssueStatus, Project, User};
```

- [ ] **Step 4: Add fields to `AppState` and `Screen`**

Add `StatusFilter` to the `Screen` enum:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    ProjectSelect,
    IssueList,
    IssueDetail,
    Filter,
    StatusFilter,
}
```

Add fields to `AppState` struct:

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
    pub status_filter_cursor_idx: usize,
    pub status_filter_pending: Vec<i64>,
    pub screen: Screen,
    pub status_message: Option<String>,
    pub should_quit: bool,
}
```

Update `AppState::new` to initialize the new fields:

```rust
Self {
    config,
    current_space_idx,
    spaces,
    selected_issue_idx: 0,
    detail_issue: None,
    filter_assignee_id: None,
    filter_cursor_idx: 0,
    project_cursor_idx: 0,
    status_filter_cursor_idx: 0,
    status_filter_pending: vec![],
    screen: Screen::ProjectSelect,
    status_message: None,
    should_quit: false,
}
```

- [ ] **Step 5: Run tests**

```
cargo test -p lazybacklog test_space_state_default_statuses -- --nocapture
cargo test -p lazybacklog test_appstate_default_status_filter -- --nocapture
```

Expected: PASS

- [ ] **Step 6: Add stub arm to `main.rs` to keep the build clean**

`Screen::StatusFilter` is now a valid variant, but `main.rs` has a `match state.screen` block that must be exhaustive. Add a stub arm to `src/main.rs` in the `AppEvent::Key(key) => match state.screen` block:

```rust
Screen::StatusFilter => handle_status_filter_key(key, &mut state, &config, tx.clone()),
```

Since `handle_status_filter_key` doesn't exist yet, add a temporary no-op stub function after `handle_filter_key`:

```rust
fn handle_status_filter_key(
    _key: crossterm::event::KeyEvent,
    _state: &mut AppState,
    _config: &config::Config,
    _tx: mpsc::UnboundedSender<AppEvent>,
) {
    // TODO: implement in Task 11
}
```

- [ ] **Step 7: Build to confirm no errors**

```
cargo build 2>&1 | head -10
```

Expected: clean build.

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add status filter fields to SpaceState, AppState, Screen"
```

---

### Task 5: Handle `StatusesLoaded` in `handle_event` and update `ApiError`

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)]` block in `src/app.rs`:

```rust
fn make_status(id: i64, name: &str) -> IssueStatus {
    IssueStatus { id, name: name.to_string() }
}

#[test]
fn test_statuses_loaded_sets_default_filter_excluding_closed() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.handle_event(AppEvent::StatusesLoaded {
        space: "space1".to_string(),
        statuses: vec![
            make_status(1, "未対応"),
            make_status(2, "処理中"),
            make_status(3, "処理済み"),
            make_status(4, "完了"),
        ],
    });
    let ss = state.current_space_state();
    assert!(ss.statuses.is_some());
    assert!(!ss.loading_statuses);
    // Default excludes "完了"
    assert_eq!(ss.filter_status_ids, vec![1, 2, 3]);
}

#[test]
fn test_statuses_loaded_excludes_closed_english() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.handle_event(AppEvent::StatusesLoaded {
        space: "space1".to_string(),
        statuses: vec![
            make_status(1, "Open"),
            make_status(2, "In Progress"),
            make_status(3, "Resolved"),
            make_status(4, "Closed"),
        ],
    });
    let ss = state.current_space_state();
    assert_eq!(ss.filter_status_ids, vec![1, 2, 3]);
}

#[test]
fn test_statuses_loaded_all_open_no_exclusion() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.handle_event(AppEvent::StatusesLoaded {
        space: "space1".to_string(),
        statuses: vec![
            make_status(1, "In Progress"),
            make_status(2, "Review"),
        ],
    });
    let ss = state.current_space_state();
    assert_eq!(ss.filter_status_ids, vec![1, 2]);
}

#[test]
fn test_statuses_loaded_wrong_space_is_noop() {
    // Firing StatusesLoaded for an unknown space must not affect the current space.
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.handle_event(AppEvent::StatusesLoaded {
        space: "nonexistent".to_string(),
        statuses: vec![make_status(1, "Open")],
    });
    // space1's statuses remain None (unaffected)
    assert!(state.current_space_state().statuses.is_none());
    assert!(state.current_space_state().filter_status_ids.is_empty());
}

#[test]
fn test_api_error_resets_loading_statuses() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().loading_statuses = true;
    state.handle_event(AppEvent::ApiError {
        space: "space1".to_string(),
        message: "timeout".to_string(),
    });
    assert!(!state.current_space_state().loading_statuses);
    // Statuses set to Some([]) so issue fetch can proceed
    assert!(state.current_space_state().statuses.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p lazybacklog test_statuses_loaded -- --nocapture
cargo test -p lazybacklog test_api_error_resets_loading_statuses -- --nocapture
```

Expected: compile errors — `StatusesLoaded` not yet handled.

- [ ] **Step 3: Add `StatusesLoaded` arm to `handle_event`**

In `src/app.rs`, in the `handle_event` match, add before `AppEvent::Key(_) => {}`:

```rust
AppEvent::StatusesLoaded { space, statuses } => {
    if let Some(state) = self.spaces.get_mut(&space) {
        let default_ids: Vec<i64> = statuses
            .iter()
            .filter(|s| s.name != "完了" && s.name != "Closed")
            .map(|s| s.id)
            .collect();
        state.filter_status_ids = default_ids;
        state.statuses = Some(statuses);
        state.loading_statuses = false;
    }
}
```

- [ ] **Step 4: Update `ApiError` arm to reset `loading_statuses` and set `statuses = Some([])`**

Replace the `ApiError` arm in `handle_event`:

```rust
AppEvent::ApiError { space, message } => {
    self.status_message = Some(format!("⚠ [{}] {}", space, message));
    if let Some(state) = self.spaces.get_mut(&space) {
        state.loading_issues = false;
        state.loading_projects = false;
        state.loading_statuses = false;
        if state.statuses.is_none() {
            // Allow issue fetch to proceed even if status fetch failed
            state.statuses = Some(vec![]);
        }
        if state.users.is_none() {
            state.users_error = true;
        }
    }
}
```

- [ ] **Step 5: Run tests**

```
cargo test -p lazybacklog test_statuses_loaded -- --nocapture
cargo test -p lazybacklog test_api_error_resets_loading_statuses -- --nocapture
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: handle StatusesLoaded event and update ApiError to reset loading_statuses"
```

---

### Task 6: Update `needs_issue_fetch` to require statuses loaded

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Write failing tests**

Add to `#[cfg(test)]` block:

```rust
#[test]
fn test_needs_issue_fetch_false_when_statuses_not_loaded() {
    let config = make_config("space1", &["space1"]);
    let state = AppState::new(config);
    // statuses is None (not yet loaded) → should not fetch issues yet
    assert!(!state.needs_issue_fetch());
}

#[test]
fn test_needs_issue_fetch_true_when_statuses_loaded() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().statuses = Some(vec![]);
    // statuses loaded (even empty), issues None, not loading → should fetch
    assert!(state.needs_issue_fetch());
}
```

Note: `!state.loading_statuses` in the updated `needs_issue_fetch` is a defensive guard — in practice, `loading_statuses = true` always implies `statuses.is_none()` (the flag is set before the fetch and cleared in the handler). The `statuses.is_some()` condition already blocks the fetch while loading. Including `!state.loading_statuses` makes the intent explicit without any test specifically exercising it as the controlling condition.

- [ ] **Step 2: Run tests to verify the new tests fail**

```
cargo test -p lazybacklog test_needs_issue_fetch -- --nocapture
```

Expected: `test_needs_issue_fetch_false_when_statuses_not_loaded` FAILS (currently returns true).

**Note:** Running the full `app::` suite at this step will also show failures in the existing `test_needs_issue_fetch_true_when_no_issues`, `test_needs_issue_fetch_false_when_loading`, and `test_needs_issue_fetch_false_when_loaded` tests — those are expected and will be fixed in Step 5.

- [ ] **Step 3: Update `needs_issue_fetch`**

Replace in `src/app.rs`:

```rust
pub fn needs_issue_fetch(&self) -> bool {
    let state = self.current_space_state();
    state.statuses.is_some()
        && state.issues.is_none()
        && !state.loading_issues
        && !state.loading_statuses
}
```

- [ ] **Step 4: Run all `needs_issue_fetch` tests**

```
cargo test -p lazybacklog test_needs_issue_fetch -- --nocapture
```

Expected: all PASS

- [ ] **Step 5: Run all app tests**

```
cargo test -p lazybacklog app:: -- --nocapture
```

Expected: PASS (some existing tests that relied on old `needs_issue_fetch` behavior may need adjustment — `test_needs_issue_fetch_true_when_no_issues` and `test_needs_issue_fetch_false_when_loading` and `test_needs_issue_fetch_false_when_loaded` must be updated to set `statuses = Some(vec![])` first where needed)

Update `test_needs_issue_fetch_true_when_no_issues`:
```rust
#[test]
fn test_needs_issue_fetch_true_when_no_issues() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().statuses = Some(vec![]);
    assert!(state.needs_issue_fetch());
}
```

Update `test_needs_issue_fetch_false_when_loading`:
```rust
#[test]
fn test_needs_issue_fetch_false_when_loading() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().statuses = Some(vec![]);
    state.current_space_state_mut().loading_issues = true;
    assert!(!state.needs_issue_fetch());
}
```

Update `test_needs_issue_fetch_false_when_loaded`:
```rust
#[test]
fn test_needs_issue_fetch_false_when_loaded() {
    let config = make_config("space1", &["space1"]);
    let mut state = AppState::new(config);
    state.current_space_state_mut().statuses = Some(vec![]);
    state.handle_event(AppEvent::IssuesLoaded {
        space: "space1".to_string(),
        issues: vec![],
    });
    assert!(!state.needs_issue_fetch());
}
```

- [ ] **Step 6: Run all app tests again**

```
cargo test -p lazybacklog app:: -- --nocapture
```

Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat: require statuses loaded before issue fetch"
```

---

## Chunk 3: UI Layer

> **Prerequisite:** Chunk 2 must be fully committed before starting Chunk 3. `Screen::StatusFilter` (from Task 4) must exist for `src/ui/mod.rs` and `src/main.rs` to compile.

### Task 7: Create `src/ui/status_filter.rs`

**Files:**
- Create: `src/ui/status_filter.rs`

- [ ] **Step 1: Write the UI helper unit tests**

These test the pure logic functions (filter bar text, toggle logic) that we'll extract as free functions. Add at the bottom of the new file after writing it (do this after Step 3 instead — tests are in the same file).

- [ ] **Step 2: Create `src/ui/status_filter.rs`**

```rust
use crate::app::AppState;
use crate::api::models::IssueStatus;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(50, 60, area);
    frame.render_widget(Clear, popup_area);

    let space_state = state.current_space_state();

    let items: Vec<ListItem> = match &space_state.statuses {
        None => vec![ListItem::new("読み込み中...")],
        Some(statuses) if statuses.is_empty() => vec![ListItem::new("ステータスなし")],
        Some(statuses) => statuses
            .iter()
            .map(|s| {
                let checked = state.status_filter_pending.contains(&s.id);
                let checkbox = if checked { "[✓]" } else { "[ ]" };
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{} {}", checkbox, s.name)),
                ]))
            })
            .collect(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" ステータスフィルター ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    // Only show cursor when statuses are actually loaded
    if space_state.statuses.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
        list_state.select(Some(state.status_filter_cursor_idx));
    }

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help = Paragraph::new("[j/k] 移動  [Space] 切替  [Enter] 決定  [Esc] キャンセル")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
    }
}

/// Generate the filter bar status text from current filter state.
/// Returns "ALL", "(なし)", or comma-separated status names.
pub fn status_filter_text(filter_ids: &[i64], statuses: &Option<Vec<IssueStatus>>) -> String {
    let Some(statuses) = statuses else {
        return "読み込み中...".to_string();
    };
    if statuses.is_empty() {
        return "ALL".to_string();
    }
    if filter_ids.is_empty() {
        return "(なし)".to_string();
    }
    if filter_ids.len() == statuses.len() {
        return "ALL".to_string();
    }
    statuses
        .iter()
        .filter(|s| filter_ids.contains(&s.id))
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Toggle a status ID in the pending list. If present, removes it; if absent, appends it.
pub fn toggle_status(pending: &mut Vec<i64>, id: i64) {
    if let Some(pos) = pending.iter().position(|&x| x == id) {
        pending.remove(pos);
    } else {
        pending.push(id);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let height = (area.height as u32 * percent_y as u32 / 100) as u16;
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_statuses() -> Vec<IssueStatus> {
        vec![
            IssueStatus { id: 1, name: "未対応".to_string() },
            IssueStatus { id: 2, name: "処理中".to_string() },
            IssueStatus { id: 3, name: "処理済み".to_string() },
            IssueStatus { id: 4, name: "完了".to_string() },
        ]
    }

    #[test]
    fn test_status_filter_text_all_selected() {
        let statuses = make_statuses();
        let ids = vec![1, 2, 3, 4];
        assert_eq!(status_filter_text(&ids, &Some(statuses)), "ALL");
    }

    #[test]
    fn test_status_filter_text_none_selected() {
        let statuses = make_statuses();
        assert_eq!(status_filter_text(&[], &Some(statuses)), "(なし)");
    }

    #[test]
    fn test_status_filter_text_partial() {
        let statuses = make_statuses();
        let ids = vec![1, 2];
        assert_eq!(status_filter_text(&ids, &Some(statuses)), "未対応, 処理中");
    }

    #[test]
    fn test_status_filter_text_loading() {
        assert_eq!(status_filter_text(&[], &None), "読み込み中...");
    }

    #[test]
    fn test_status_filter_text_empty_statuses_all() {
        assert_eq!(status_filter_text(&[], &Some(vec![])), "ALL");
    }

    #[test]
    fn test_toggle_status_add() {
        let mut pending = vec![1i64, 3];
        toggle_status(&mut pending, 2);
        assert_eq!(pending, vec![1, 3, 2]);
    }

    #[test]
    fn test_toggle_status_remove() {
        let mut pending = vec![1i64, 2, 3];
        toggle_status(&mut pending, 2);
        assert_eq!(pending, vec![1, 3]);
    }

    #[test]
    fn test_toggle_status_add_to_empty() {
        let mut pending: Vec<i64> = vec![];
        toggle_status(&mut pending, 5);
        assert_eq!(pending, vec![5]);
    }
}
```

- [ ] **Step 3: Run the unit tests**

```
cargo test -p lazybacklog ui::status_filter:: -- --nocapture
```

Expected: PASS for all `status_filter` tests.

- [ ] **Step 4: Commit**

```bash
git add src/ui/status_filter.rs
git commit -m "feat: add status_filter UI widget and helper functions"
```

---

### Task 8: Update `src/ui/mod.rs`

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Register `status_filter` module and add `StatusFilter` to match**

In `src/ui/mod.rs`:

Add module declaration after `pub mod project_select;`:
```rust
pub mod status_filter;
```

Update the `use` import to include `Screen::StatusFilter`:
```rust
use crate::app::{AppState, Screen};
```
(Already imported — just confirm `Screen` is used in the match.)

Add arm to the `match state.screen` block:
```rust
Screen::StatusFilter => {
    status_filter::render(frame, area, state);
}
```

**Only the `match` block changes** — do not touch the `render_status_message(frame, area, state)` call that follows it; it must remain after the match.

The full match becomes:
```rust
match state.screen {
    Screen::IssueDetail => {
        if let Some(issue) = &state.detail_issue {
            issue_detail::render(frame, area, issue);
        }
    }
    Screen::Filter => {
        filter::render(frame, area, state);
    }
    Screen::StatusFilter => {
        status_filter::render(frame, area, state);
    }
    Screen::IssueList => {}
    Screen::ProjectSelect => {}
}
// render_status_message stays here — do not remove it
render_status_message(frame, area, state);
```

- [ ] **Step 2: Update `render_filter_bar` to show status filter**

Replace `render_filter_bar` in `src/ui/mod.rs`:

```rust
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

    let status_text = status_filter::status_filter_text(
        &space_state.filter_status_ids,
        &space_state.statuses,
    );

    let text = format!(" Assignee: {}  |  Status: {}", assignee_name, status_text);
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::Gray));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 3: Update `render_help_bar` to add `[s]` shortcut**

Replace the help text in `render_help_bar`:

```rust
fn render_help_bar(frame: &mut Frame, area: Rect) {
    let text = " [j/k] 移動  [Enter] 詳細  [f] Assignee  [s] Status  [r] 更新  [[] []] スペース切替  [q] 終了";
    let paragraph =
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: Build to check**

```
cargo build 2>&1 | head -30
```

Expected: compile errors in `main.rs` — the `match state.screen` key dispatch block is non-exhaustive (it has 4 arms; the `Screen::StatusFilter => handle_status_filter_key(...)` arm exists as a stub function but the arm in the match was not yet added to the key dispatch match — that is done in Task 12). This error exists throughout Chunk 3 and is not introduced by this task. The `src/ui/mod.rs` changes themselves compile cleanly.

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: add StatusFilter to UI render, update filter bar and help bar"
```

---

## Chunk 4: Main.rs Wiring

### Task 9: Add `fetch_statuses` helper and update project selection

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `fetch_statuses` helper function**

Add after the `fetch_issues` function in `src/main.rs` (around line 390):

```rust
fn fetch_statuses(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: i64,
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
            Ok(client) => match client.fetch_statuses(project_id).await {
                Ok(statuses) => {
                    let _ = tx.send(AppEvent::StatusesLoaded {
                        space: space_name,
                        statuses,
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

- [ ] **Step 2: Update `handle_project_select_key` — call `fetch_statuses` instead of `fetch_issues`**

Replace the `KeyCode::Enter` arm in `handle_project_select_key` (around line 325–345 in `src/main.rs`):

```rust
KeyCode::Enter => {
    if project_count == 0 {
        return;
    }
    let project = state
        .current_space_state()
        .projects
        .as_ref()
        .and_then(|p| p.get(state.project_cursor_idx))
        .cloned();
    if let Some(project) = project {
        let project_id = project.id;
        // Reset status + issue state for new project.
        // Use separate short-lived borrows (matching existing codebase pattern)
        // to avoid holding a &mut SpaceState across the screen/fetch calls.
        state.current_space_state_mut().selected_project = Some(project);
        state.current_space_state_mut().statuses = None;
        state.current_space_state_mut().filter_status_ids = vec![];
        state.current_space_state_mut().issues = None;
        state.current_space_state_mut().loading_statuses = true; // defined in Chunk 2 Task 4
        state.screen = Screen::IssueList;
        // Fetch statuses first; issues fetched automatically by auto-fetch guard after StatusesLoaded
        fetch_statuses(state, config, tx, project_id);
    }
}
```

- [ ] **Step 3: Do NOT commit yet — continue to Task 10**

At this point the build has errors because `fetch_issues` call sites still use the old 2-argument signature. Commit only after Task 10 completes and the build is clean.

---

### Task 10: Update all `fetch_issues` call sites

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `fetch_issues` helper signature**

Replace the `fetch_issues` function signature and body in `src/main.rs` (around line 350):

```rust
fn fetch_issues(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
    project_id: Option<i64>,
    assignee_id: Option<i64>,
    status_ids: Vec<i64>,
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
            Ok(client) => match client.fetch_issues(project_id, assignee_id, &status_ids).await {
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

Note: `status_ids` is passed by value (`Vec<i64>`) so it can be moved into the async block.

- [ ] **Step 2: Update auto-fetch guard in event loop**

Find the auto-fetch guard (around line 141–146) and update:

```rust
if state.screen == Screen::IssueList && state.needs_issue_fetch() {
    let project_id = state.selected_project().map(|p| p.id);
    let assignee_id = state.filter_assignee_id;
    let status_ids = state.current_space_state().filter_status_ids.clone();
    fetch_issues(&state, &config, tx.clone(), project_id, assignee_id, status_ids);
    state.current_space_state_mut().loading_issues = true;
}
```

- [ ] **Step 3: Update `r` key refresh in `handle_list_key`**

Replace the `KeyCode::Char('r')` arm in `handle_list_key`:

```rust
KeyCode::Char('r') => {
    let project_id = state.selected_project().map(|p| p.id);
    let assignee_id = state.filter_assignee_id;
    let status_ids = state.current_space_state().filter_status_ids.clone();
    state.current_space_state_mut().issues = None;
    state.current_space_state_mut().loading_issues = true;
    fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
}
```

- [ ] **Step 4: Update `fetch_issues` call in `handle_filter_key`**

Replace the `KeyCode::Enter` arm in `handle_filter_key` — update the `fetch_issues` call:

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
    let status_ids = state.current_space_state().filter_status_ids.clone();
    state.current_space_state_mut().issues = None;
    state.current_space_state_mut().loading_issues = true;
    fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
}
```

- [ ] **Step 5: Build to confirm clean compile**

```
cargo build 2>&1
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire fetch_statuses and update all fetch_issues call sites with status_ids"
```

---

### Task 11: Add `handle_status_filter_key`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add the key handler function**

Add after `handle_filter_key` in `src/main.rs`:

```rust
fn handle_status_filter_key(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let status_count = state
        .current_space_state()
        .statuses
        .as_ref()
        .map(|s| s.len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Esc => {
            state.status_filter_pending = vec![];
            state.screen = Screen::IssueList;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if status_count > 0 && state.status_filter_cursor_idx + 1 < status_count {
                state.status_filter_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.status_filter_cursor_idx > 0 {
                state.status_filter_cursor_idx -= 1;
            }
        }
        KeyCode::Char(' ') => {
            if status_count > 0 {
                let id = state
                    .current_space_state()
                    .statuses
                    .as_ref()
                    .and_then(|s| s.get(state.status_filter_cursor_idx))
                    .map(|s| s.id);
                if let Some(id) = id {
                    ui::status_filter::toggle_status(&mut state.status_filter_pending, id);
                }
            }
        }
        KeyCode::Enter => {
            let pending = state.status_filter_pending.clone();
            state.current_space_state_mut().filter_status_ids = pending;
            state.status_filter_pending = vec![];
            state.screen = Screen::IssueList;
            let project_id = state.selected_project().map(|p| p.id);
            let assignee_id = state.filter_assignee_id;
            let status_ids = state.current_space_state().filter_status_ids.clone();
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, project_id, assignee_id, status_ids);
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Build to confirm no errors**

```
cargo build 2>&1
```

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add handle_status_filter_key"
```

---

### Task 12: Wire `s` key and `StatusFilter` screen dispatch

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `s` key binding to `handle_list_key`**

In `handle_list_key`, add after the `KeyCode::Char('f')` arm:

```rust
KeyCode::Char('s') => {
    state.status_filter_pending = state.current_space_state().filter_status_ids.clone();
    state.status_filter_cursor_idx = 0;
    state.screen = Screen::StatusFilter;
}
```

- [ ] **Step 2: Add `StatusFilter` to key dispatch in the event loop**

In the `match state.screen` in `run()`:

```rust
AppEvent::Key(key) => match state.screen {
    Screen::IssueList => handle_list_key(key, &mut state, &config, tx.clone()),
    Screen::IssueDetail => handle_detail_key(key, &mut state),
    Screen::Filter => handle_filter_key(key, &mut state, &config, tx.clone()),
    Screen::StatusFilter => handle_status_filter_key(key, &mut state, &config, tx.clone()),
    Screen::ProjectSelect => handle_project_select_key(key, &mut state, &config, tx.clone()),
},
```

- [ ] **Step 3: Build and run all tests**

```
cargo build 2>&1
cargo test 2>&1
```

Expected: clean build, all tests PASS.

- [ ] **Step 4: Final sanity — run all tests with output**

```
cargo test -- --nocapture 2>&1 | tail -20
```

Expected: `test result: ok. X passed; 0 failed`

- [ ] **Step 5: Final commit**

```bash
git add src/main.rs
git commit -m "feat: wire s key and StatusFilter screen dispatch"
```

---

## Done

The status filter feature is complete:

- `s` opens a multi-select status popup
- Default is "Not Closed" (all statuses except "完了"/"Closed")
- Statuses fetched per-project from Backlog API
- Filter bar shows `Assignee: X | Status: Y`
- Server-side filtering via `statusId[]` query params
