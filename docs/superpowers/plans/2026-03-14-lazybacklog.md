# lazybacklog Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a lazygit-inspired TUI for Nulab's Backlog that shows issues with assignee filtering across multiple spaces.

**Architecture:** ratatui renders the UI; a separate `std::thread` reads crossterm keyboard events; `tokio::spawn` tasks call the Backlog REST API — all sending results through one `mpsc::UnboundedChannel<AppEvent>` to the tokio main loop which updates `AppState` and re-renders.

**Tech Stack:** Rust 2021, ratatui 0.28, crossterm 0.28, tokio 1 (full), reqwest 0.12 (json), serde/serde_json 1, toml 0.8, anyhow 1, wiremock 0.6 (dev)

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Dependencies |
| `src/main.rs` | Terminal setup/teardown, tokio entry point, event loop, key handlers |
| `src/config.rs` | Load & validate `~/.config/lazybacklog/config.toml`, Unix permission check |
| `src/event.rs` | `AppEvent` enum |
| `src/app.rs` | `AppState` struct, state mutation methods, `handle_event()` |
| `src/api/mod.rs` | Re-exports |
| `src/api/models.rs` | Serde types for Backlog API responses |
| `src/api/client.rs` | `BacklogClient` — async reqwest wrapper for Backlog API v2 |
| `src/ui/mod.rs` | Top-level `render()`, title/filter bar/help bar |
| `src/ui/issue_list.rs` | Issue table widget (drawing only) |
| `src/ui/issue_detail.rs` | Issue detail centered popup (drawing only) |
| `src/ui/filter.rs` | Assignee filter popup (drawing only) |

---

## Chunk 1: Project Scaffold + Config

### Task 1: Initialize Cargo project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Initialize project**

```bash
cd /Users/makinotakashi/Workspace/rust/lazybacklog
cargo init --name lazybacklog
```

Expected: creates `Cargo.toml` and `src/main.rs`

- [ ] **Step 2: Replace Cargo.toml with full dependencies**

Replace the entire `Cargo.toml` with:

```toml
[package]
name = "lazybacklog"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
anyhow = "1"

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 3: Replace src/main.rs with a placeholder that compiles**

```rust
fn main() {
    println!("lazybacklog");
}
```

- [ ] **Step 4: Verify project compiles**

```bash
cargo build
```

Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: initialize cargo project with dependencies"
```

---

### Task 2: Config module

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: Write failing tests for config parsing**

Create `src/config.rs` with only the test module first:

```rust
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct SpaceConfig {
    pub name: String,
    pub host: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub default_space: String,
    pub spaces: Vec<SpaceConfig>,
}

pub fn config_path() -> PathBuf {
    // Respect XDG_CONFIG_HOME on Linux; fall back to ~/.config on macOS
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    config_home.join("lazybacklog").join("config.toml")
}

pub fn load() -> Result<Config> {
    todo!()
}

/// Returns a warning string if permissions are wrong, None if OK.
#[cfg(unix)]
pub fn check_permissions(path: &std::path::Path) -> Option<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    fn load_from_path(path: &std::path::Path) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
        if config.spaces.is_empty() {
            bail!("No spaces defined in config.toml");
        }
        if !config.spaces.iter().any(|s| s.name == config.default_space) {
            bail!(
                "default_space '{}' not found in spaces",
                config.default_space
            );
        }
        Ok(config)
    }

    #[test]
    fn test_parse_valid_config() {
        let file = write_config(
            r#"
default_space = "myspace"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "abc123"
"#,
        );
        let config = load_from_path(file.path()).unwrap();
        assert_eq!(config.default_space, "myspace");
        assert_eq!(config.spaces.len(), 1);
        assert_eq!(config.spaces[0].host, "myspace.backlog.com");
        assert_eq!(config.spaces[0].api_key, "abc123");
    }

    #[test]
    fn test_parse_multiple_spaces() {
        let file = write_config(
            r#"
default_space = "work"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "abc123"

[[spaces]]
name = "work"
host = "work.backlog.jp"
api_key = "def456"
"#,
        );
        let config = load_from_path(file.path()).unwrap();
        assert_eq!(config.spaces.len(), 2);
        assert_eq!(config.default_space, "work");
    }

    #[test]
    fn test_invalid_default_space() {
        let file = write_config(
            r#"
default_space = "nonexistent"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "abc123"
"#,
        );
        let err = load_from_path(file.path()).unwrap_err();
        assert!(err.to_string().contains("default_space"));
    }

    #[test]
    fn test_empty_spaces() {
        let file = write_config(r#"default_space = "x""#);
        let err = load_from_path(file.path()).unwrap_err();
        assert!(err.to_string().contains("No spaces"));
    }

    #[cfg(unix)]
    #[test]
    fn test_permissions_ok() {
        use std::os::unix::fs::PermissionsExt;
        let file = write_config("x = 1");
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(check_permissions(file.path()).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_permissions_warn() {
        use std::os::unix::fs::PermissionsExt;
        let file = write_config("x = 1");
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        let warning = check_permissions(file.path());
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("chmod 600"));
    }
}
```

- [ ] **Step 2: Run tests to verify the permission tests panic**

```bash
cargo test config
```

Expected: the 4 parsing tests pass (they use the local `load_from_path` helper, not `load()`). The two `test_permissions_*` tests panic with `not yet implemented` — this confirms the implementation is still needed.

- [ ] **Step 3: Implement `load()` and `check_permissions()`**

Replace the `todo!()` bodies:

```rust
pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        bail!(
            "Config file not found: {}\nCreate it with your Backlog spaces and API keys.",
            path.display()
        );
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let config: Config =
        toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
    if config.spaces.is_empty() {
        bail!("No spaces defined in config.toml");
    }
    if !config.spaces.iter().any(|s| s.name == config.default_space) {
        bail!(
            "default_space '{}' not found in spaces",
            config.default_space
        );
    }
    Ok(config)
}

#[cfg(unix)]
pub fn check_permissions(path: &std::path::Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Some(format!(
                "Warning: {} has permissions {:04o}, expected 0600. Run: chmod 600 {}",
                path.display(),
                mode,
                path.display()
            ));
        }
    }
    None
}
```

- [ ] **Step 4: Add `mod config;` to `src/main.rs`**

```rust
mod config;

fn main() {
    println!("lazybacklog");
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test config
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add config module with toml parsing and permission check"
```

---

### Task 3: API models

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/models.rs`

- [ ] **Step 1: Create `src/api/mod.rs`** (models only for now; client is added in Task 4)

```rust
pub mod models;
```

- [ ] **Step 2: Create stub `src/api/client.rs`** (empty placeholder so the file exists)

```rust
// Implemented in Task 4
```

Then update `src/api/mod.rs` to include both:

```rust
pub mod client;
pub mod models;
```

- [ ] **Step 3: Create `src/api/models.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: i64,
    #[serde(rename = "projectKey")]
    pub project_key: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueStatus {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Priority {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueType {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub id: i64,
    #[serde(rename = "issueKey")]
    pub issue_key: String,
    pub summary: String,
    pub description: Option<String>,
    pub assignee: Option<User>,
    pub status: IssueStatus,
    pub priority: Option<Priority>,
    #[serde(rename = "issueType")]
    pub issue_type: Option<IssueType>,
    #[serde(rename = "dueDate")]
    pub due_date: Option<String>,
}
```

- [ ] **Step 3: Add serde roundtrip tests to `src/api/models.rs`**

Append to `src/api/models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_issue_with_renamed_fields() {
        let json = serde_json::json!({
            "id": 1,
            "issueKey": "PROJ-1",
            "summary": "Test",
            "description": "desc",
            "assignee": { "id": 10, "name": "Alice" },
            "status": { "id": 1, "name": "Open" },
            "priority": { "id": 2, "name": "Normal" },
            "issueType": { "id": 3, "name": "Bug" },
            "dueDate": "2026-04-01T00:00:00Z"
        });
        let issue: Issue = serde_json::from_value(json).unwrap();
        assert_eq!(issue.issue_key, "PROJ-1");
        assert_eq!(issue.assignee.unwrap().name, "Alice");
        assert_eq!(issue.issue_type.unwrap().name, "Bug");
        assert_eq!(issue.due_date.unwrap(), "2026-04-01T00:00:00Z");
    }

    #[test]
    fn test_deserialize_issue_with_null_optional_fields() {
        let json = serde_json::json!({
            "id": 2,
            "issueKey": "PROJ-2",
            "summary": "Minimal",
            "description": null,
            "assignee": null,
            "status": { "id": 1, "name": "Open" },
            "priority": null,
            "issueType": null,
            "dueDate": null
        });
        let issue: Issue = serde_json::from_value(json).unwrap();
        assert!(issue.assignee.is_none());
        assert!(issue.description.is_none());
        assert!(issue.due_date.is_none());
    }

    #[test]
    fn test_deserialize_project_key_renamed() {
        let json = serde_json::json!({ "id": 100, "projectKey": "PROJ", "name": "My Project" });
        let project: Project = serde_json::from_value(json).unwrap();
        assert_eq!(project.project_key, "PROJ");
    }
}
```

- [ ] **Step 4: Run model tests**

```bash
cargo test api::models
```

Expected: 3 tests pass.

- [ ] **Step 5: Add `mod api;` to `src/main.rs`**

```rust
mod api;
mod config;

fn main() {
    println!("lazybacklog");
}
```

- [ ] **Step 6: Verify compilation**

```bash
cargo build
```

Expected: `Finished` with no errors.

- [ ] **Step 7: Commit**

```bash
git add src/api/mod.rs src/api/models.rs src/main.rs
git commit -m "feat: add Backlog API response models with serde"
```

---

## Chunk 2: API Client + App State

### Task 4: API client

**Files:**
- Create: `src/api/client.rs`

The client uses a `base_url` field so tests can point it at a wiremock server.

- [ ] **Step 1: Write failing tests**

Create `src/api/client.rs` with test stubs:

```rust
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

use super::models::{Issue, Project, User};

pub struct BacklogClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl BacklogClient {
    pub fn new(host: String, api_key: String) -> Result<Self> {
        Self::with_base_url(format!("https://{}/api/v2", host), api_key)
    }

    pub fn with_base_url(base_url: String, api_key: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }

    pub async fn fetch_issues(&self, assignee_id: Option<i64>) -> Result<Vec<Issue>> {
        todo!()
    }

    pub async fn fetch_issue(&self, id_or_key: &str) -> Result<Issue> {
        todo!()
    }

    pub async fn fetch_projects(&self) -> Result<Vec<Project>> {
        todo!()
    }

    pub async fn fetch_project_users(&self, project_id: i64) -> Result<Vec<User>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn make_client(server: &MockServer) -> BacklogClient {
        BacklogClient::with_base_url(
            format!("{}/api/v2", server.uri()),
            "test_api_key".to_string(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_fetch_issues_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .and(query_param("apiKey", "test_api_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 1,
                    "issueKey": "PROJ-1",
                    "summary": "Test issue",
                    "description": "Some description",
                    "assignee": { "id": 10, "name": "Alice" },
                    "status": { "id": 1, "name": "In Progress" },
                    "priority": { "id": 2, "name": "Normal" },
                    "issueType": { "id": 1, "name": "Bug" },
                    "dueDate": null
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issues = client.fetch_issues(None).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_key, "PROJ-1");
        assert_eq!(issues[0].summary, "Test issue");
        assert_eq!(issues[0].assignee.as_ref().unwrap().name, "Alice");
    }

    #[tokio::test]
    async fn test_fetch_issues_with_assignee_filter() {
        let server = MockServer::start().await;
        // reqwest percent-encodes '[' and ']' as '%5B' and '%5D' in query keys.
        // wiremock 0.6's query_param matcher uses url::Url::query_pairs() which
        // percent-decodes keys before comparison, so "assigneeId[]" matches the
        // wire bytes "assigneeId%5B%5D". If this test fails to match, change to
        // query_param("assigneeId%5B%5D", "42").
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .and(query_param("assigneeId[]", "42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issues = client.fetch_issues(Some(42)).await.unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_issues_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let err = client.fetch_issues(None).await.unwrap_err();
        assert!(err.to_string().contains("401 Unauthorized"));
    }

    #[tokio::test]
    async fn test_fetch_issue_detail() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/issues/PROJ-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 1,
                "issueKey": "PROJ-1",
                "summary": "Detailed issue",
                "description": "Full description here",
                "assignee": null,
                "status": { "id": 1, "name": "Open" },
                "priority": null,
                "issueType": null,
                "dueDate": "2026-04-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let issue = client.fetch_issue("PROJ-1").await.unwrap();
        assert_eq!(issue.issue_key, "PROJ-1");
        assert_eq!(issue.description.unwrap(), "Full description here");
    }

    #[tokio::test]
    async fn test_fetch_projects_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/projects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": 100, "projectKey": "PROJ", "name": "My Project" }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let projects = client.fetch_projects().await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].project_key, "PROJ");
    }

    #[tokio::test]
    async fn test_fetch_project_users() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/projects/100/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "id": 10, "name": "Alice" },
                { "id": 20, "name": "Bob" }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let users = client.fetch_project_users(100).await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Alice");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test api::client
```

Expected: tests fail because methods have `todo!()`.

- [ ] **Step 3: Implement API client methods**

Replace the `todo!()` bodies in `BacklogClient`:

```rust
pub async fn fetch_issues(&self, assignee_id: Option<i64>) -> Result<Vec<Issue>> {
    let mut params: Vec<(&str, String)> = vec![
        ("apiKey", self.api_key.clone()),
        ("count", "100".to_string()),
    ];
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

pub async fn fetch_issue(&self, id_or_key: &str) -> Result<Issue> {
    let resp = self
        .http
        .get(format!("{}/issues/{}", self.base_url, id_or_key))
        .query(&[("apiKey", &self.api_key)])
        .send()
        .await
        .context("Failed to connect to Backlog API")?;
    if resp.status() == 401 {
        anyhow::bail!("401 Unauthorized - check your API key");
    }
    resp.error_for_status_ref()
        .context("Backlog API returned an error")?;
    resp.json::<Issue>()
        .await
        .context("Failed to parse issue response")
}

pub async fn fetch_projects(&self) -> Result<Vec<Project>> {
    let resp = self
        .http
        .get(format!("{}/projects", self.base_url))
        .query(&[("apiKey", &self.api_key)])
        .send()
        .await
        .context("Failed to connect to Backlog API")?;
    if resp.status() == 401 {
        anyhow::bail!("401 Unauthorized - check your API key");
    }
    resp.error_for_status_ref()
        .context("Backlog API returned an error")?;
    resp.json::<Vec<Project>>()
        .await
        .context("Failed to parse projects response")
}

pub async fn fetch_project_users(&self, project_id: i64) -> Result<Vec<User>> {
    let resp = self
        .http
        .get(format!("{}/projects/{}/users", self.base_url, project_id))
        .query(&[("apiKey", &self.api_key)])
        .send()
        .await
        .context("Failed to connect to Backlog API")?;
    resp.error_for_status_ref()
        .context("Backlog API returned an error")?;
    resp.json::<Vec<User>>()
        .await
        .context("Failed to parse users response")
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test api::client
```

Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/api/client.rs
git commit -m "feat: add Backlog API client with reqwest"
```

---

### Task 5: Event types

**Files:**
- Create: `src/event.rs`

- [ ] **Step 1: Create `src/event.rs`**

```rust
use crate::api::models::{Issue, User};
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
    /// Any API error
    ApiError { space: String, message: String },
}
```

- [ ] **Step 2: Add `mod event;` to `src/main.rs`**

```rust
mod api;
mod config;
mod event;

fn main() {
    println!("lazybacklog");
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/event.rs src/main.rs
git commit -m "feat: add AppEvent enum"
```

---

### Task 6: App state

**Files:**
- Create: `src/app.rs`

- [ ] **Step 1: Write failing tests**

Create `src/app.rs`:

```rust
use std::collections::HashMap;

use crate::api::models::{Issue, IssueStatus, User};
use crate::config::Config;
use crate::event::AppEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    IssueList,
    IssueDetail,
    Filter,
}

#[derive(Debug, Clone, Default)]
pub struct SpaceState {
    pub issues: Option<Vec<Issue>>,
    pub users: Option<Vec<User>>,
    pub users_error: bool,
    pub loading_issues: bool,
}

pub struct AppState {
    pub config: Config,
    pub current_space_idx: usize,
    pub spaces: HashMap<String, SpaceState>,
    pub selected_issue_idx: usize,
    pub detail_issue: Option<Issue>,
    pub filter_assignee_id: Option<i64>,
    pub filter_cursor_idx: usize,
    pub screen: Screen,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let mut spaces = HashMap::new();
        for space in &config.spaces {
            spaces.insert(space.name.clone(), SpaceState::default());
        }
        let current_space_idx = config
            .spaces
            .iter()
            .position(|s| s.name == config.default_space)
            .unwrap_or(0);
        Self {
            config,
            current_space_idx,
            spaces,
            selected_issue_idx: 0,
            detail_issue: None,
            filter_assignee_id: None,
            filter_cursor_idx: 0,
            screen: Screen::IssueList,
            status_message: None,
            should_quit: false,
        }
    }

    pub fn current_space_name(&self) -> &str {
        &self.config.spaces[self.current_space_idx].name
    }

    pub fn current_space_state(&self) -> &SpaceState {
        self.spaces.get(self.current_space_name()).unwrap()
    }

    pub fn current_space_state_mut(&mut self) -> &mut SpaceState {
        let name = self.current_space_name().to_string();
        self.spaces.get_mut(&name).unwrap()
    }

    pub fn needs_issue_fetch(&self) -> bool {
        let state = self.current_space_state();
        state.issues.is_none() && !state.loading_issues
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        self.current_space_state()
            .issues
            .as_ref()
            .and_then(|issues| issues.get(self.selected_issue_idx))
    }

    pub fn navigate_down(&mut self) {
        let len = self
            .current_space_state()
            .issues
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(0);
        if len > 0 && self.selected_issue_idx < len - 1 {
            self.selected_issue_idx += 1;
        }
    }

    pub fn navigate_up(&mut self) {
        if self.selected_issue_idx > 0 {
            self.selected_issue_idx -= 1;
        }
    }

    pub fn switch_space_next(&mut self) {
        self.current_space_idx = (self.current_space_idx + 1) % self.config.spaces.len();
        self.selected_issue_idx = 0;
        self.detail_issue = None;
        self.screen = Screen::IssueList;
    }

    pub fn switch_space_prev(&mut self) {
        if self.current_space_idx == 0 {
            self.current_space_idx = self.config.spaces.len() - 1;
        } else {
            self.current_space_idx -= 1;
        }
        self.selected_issue_idx = 0;
        self.detail_issue = None;
        self.screen = Screen::IssueList;
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::IssuesLoaded { space, issues } => {
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.issues = Some(issues);
                    state.loading_issues = false;
                }
                self.selected_issue_idx = 0;
                self.status_message = None;
            }
            AppEvent::IssueDetailLoaded(issue) => {
                self.detail_issue = Some(issue);
                self.screen = Screen::IssueDetail;
            }
            AppEvent::SpaceUsersLoaded { space, users } => {
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.users = Some(users);
                    state.users_error = false;
                }
            }
            AppEvent::ApiError { space, message } => {
                self.status_message = Some(format!("⚠ [{}] {}", space, message));
                if let Some(state) = self.spaces.get_mut(&space) {
                    state.loading_issues = false;
                    if state.users.is_none() {
                        state.users_error = true;
                    }
                }
            }
            AppEvent::Key(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(default: &str, names: &[&str]) -> Config {
        Config {
            default_space: default.to_string(),
            spaces: names
                .iter()
                .map(|n| crate::config::SpaceConfig {
                    name: n.to_string(),
                    host: format!("{}.backlog.com", n),
                    api_key: "key".to_string(),
                })
                .collect(),
        }
    }

    fn make_issue(key: &str) -> Issue {
        Issue {
            id: 1,
            issue_key: key.to_string(),
            summary: format!("Summary of {}", key),
            description: None,
            assignee: None,
            status: IssueStatus {
                id: 1,
                name: "Open".to_string(),
            },
            priority: None,
            issue_type: None,
            due_date: None,
        }
    }

    #[test]
    fn test_initial_state() {
        let config = make_config("space1", &["space1", "space2"]);
        let state = AppState::new(config);
        assert_eq!(state.current_space_name(), "space1");
        assert_eq!(state.current_space_idx, 0);
        assert_eq!(state.screen, Screen::IssueList);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_default_space_selection() {
        let config = make_config("space2", &["space1", "space2"]);
        let state = AppState::new(config);
        assert_eq!(state.current_space_idx, 1);
        assert_eq!(state.current_space_name(), "space2");
    }

    #[test]
    fn test_issues_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("PROJ-2")],
        });
        let issues = state.current_space_state().issues.as_ref().unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].issue_key, "PROJ-1");
        assert!(!state.current_space_state().loading_issues);
        assert_eq!(state.selected_issue_idx, 0);
    }

    #[test]
    fn test_issue_detail_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssueDetailLoaded(make_issue("PROJ-5")));
        assert_eq!(state.screen, Screen::IssueDetail);
        assert_eq!(state.detail_issue.unwrap().issue_key, "PROJ-5");
    }

    #[test]
    fn test_space_users_loaded_event() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::SpaceUsersLoaded {
            space: "space1".to_string(),
            users: vec![
                User { id: 1, name: "Alice".to_string() },
                User { id: 2, name: "Bob".to_string() },
            ],
        });
        let users = state.current_space_state().users.as_ref().unwrap();
        assert_eq!(users.len(), 2);
        assert!(!state.current_space_state().users_error);
    }

    #[test]
    fn test_api_error_sets_status_message() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "401 Unauthorized".to_string(),
        });
        let msg = state.status_message.unwrap();
        assert!(msg.contains("space1"));
        assert!(msg.contains("401 Unauthorized"));
    }

    #[test]
    fn test_api_error_sets_users_error_when_no_users() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::ApiError {
            space: "space1".to_string(),
            message: "timeout".to_string(),
        });
        assert!(state.current_space_state().users_error);
    }

    #[test]
    fn test_navigate_down() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("PROJ-2"), make_issue("PROJ-3")],
        });
        state.navigate_down();
        assert_eq!(state.selected_issue_idx, 1);
        state.navigate_down();
        assert_eq!(state.selected_issue_idx, 2);
        state.navigate_down(); // at end, should not go past
        assert_eq!(state.selected_issue_idx, 2);
    }

    #[test]
    fn test_navigate_up() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![make_issue("PROJ-1"), make_issue("PROJ-2")],
        });
        state.navigate_down();
        state.navigate_up();
        assert_eq!(state.selected_issue_idx, 0);
        state.navigate_up(); // at top, should not go negative
        assert_eq!(state.selected_issue_idx, 0);
    }

    #[test]
    fn test_switch_space_next() {
        let config = make_config("space1", &["space1", "space2", "space3"]);
        let mut state = AppState::new(config);
        state.switch_space_next();
        assert_eq!(state.current_space_name(), "space2");
        state.switch_space_next();
        assert_eq!(state.current_space_name(), "space3");
        state.switch_space_next(); // wraps around
        assert_eq!(state.current_space_name(), "space1");
    }

    #[test]
    fn test_switch_space_prev() {
        let config = make_config("space1", &["space1", "space2", "space3"]);
        let mut state = AppState::new(config);
        state.switch_space_prev(); // wraps around
        assert_eq!(state.current_space_name(), "space3");
        state.switch_space_prev();
        assert_eq!(state.current_space_name(), "space2");
    }

    #[test]
    fn test_needs_issue_fetch_true_when_no_issues() {
        let config = make_config("space1", &["space1"]);
        let state = AppState::new(config);
        assert!(state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loading() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.current_space_state_mut().loading_issues = true;
        assert!(!state.needs_issue_fetch());
    }

    #[test]
    fn test_needs_issue_fetch_false_when_loaded() {
        let config = make_config("space1", &["space1"]);
        let mut state = AppState::new(config);
        state.handle_event(AppEvent::IssuesLoaded {
            space: "space1".to_string(),
            issues: vec![],
        });
        assert!(!state.needs_issue_fetch());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test app
```

Expected: compile errors — `AppState` and related types not fully implemented yet (they are defined but `handle_event` needs to compile).

Actually all code is defined. Run:

```bash
cargo test app 2>&1 | head -20
```

Expected: tests compile and run; any failures are logic errors to fix.

- [ ] **Step 3: Add `mod app;` to `src/main.rs`**

```rust
mod api;
mod app;
mod config;
mod event;

fn main() {
    println!("lazybacklog");
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: all tests pass (config + api::client + app).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/event.rs src/main.rs
git commit -m "feat: add AppState with event handling and space/navigation logic"
```

---

## Chunk 3: UI Layer + Main Event Loop

### Task 7: UI — issue list widget

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/issue_list.rs`

- [ ] **Step 1: Create `src/ui/issue_list.rs`**

```rust
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let space_state = state.current_space_state();

    if space_state.loading_issues {
        let loading =
            Paragraph::new("Loading issues...").style(Style::default().fg(Color::Gray));
        frame.render_widget(loading, area);
        return;
    }

    let issues = match &space_state.issues {
        Some(issues) => issues,
        None => {
            let msg = Paragraph::new("No issues loaded. Press [r] to fetch.")
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(msg, area);
            return;
        }
    };

    let rows: Vec<Row> = issues
        .iter()
        .map(|issue| {
            let assignee = issue
                .assignee
                .as_ref()
                .map(|u| u.name.as_str())
                .unwrap_or("-");
            Row::new(vec![
                Cell::from(issue.issue_key.clone()),
                Cell::from(issue.summary.clone()),
                Cell::from(assignee.to_string()),
                Cell::from(issue.status.name.clone()),
            ])
        })
        .collect();

    let footer_msg = if issues.len() >= 100 {
        format!("(表示: {}件 / 上限100件)", issues.len())
    } else {
        format!("({}件)", issues.len())
    };

    // Reserve last line for count
    let table_area = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
    let footer_area = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Min(30),
        Constraint::Length(16),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Key", "Summary", "Assignee", "Status"])
                .style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut table_state = TableState::default();
    if !issues.is_empty() {
        table_state.select(Some(state.selected_issue_idx));
    }

    frame.render_stateful_widget(table, table_area, &mut table_state);

    let footer = Paragraph::new(footer_msg).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}
```

- [ ] **Step 2: Create `src/ui/mod.rs`**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, Screen};

pub mod filter;
pub mod issue_detail;
pub mod issue_list;

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

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
    }

    // Status message overlays the help bar when present
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
```

- [ ] **Step 3: Add stubs for `issue_detail` and `filter` so it compiles**

Create `src/ui/issue_detail.rs`:

```rust
use crate::api::models::Issue;
use ratatui::{layout::Rect, Frame};

pub fn render(_frame: &mut Frame, _area: Rect, _issue: &Issue) {
    // implemented in Task 8
}
```

Create `src/ui/filter.rs`:

```rust
use crate::app::AppState;
use ratatui::{layout::Rect, Frame};

pub fn render(_frame: &mut Frame, _area: Rect, _state: &AppState) {
    // implemented in Task 9
}
```

- [ ] **Step 4: Add `mod ui;` to `src/main.rs`**

```rust
mod api;
mod app;
mod config;
mod event;
mod ui;

fn main() {
    println!("lazybacklog");
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo build
```

Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add src/ui/
git commit -m "feat: add UI skeleton with issue list widget"
```

---

### Task 8: Issue detail popup

**Files:**
- Modify: `src/ui/issue_detail.rs`

- [ ] **Step 1: Implement `src/ui/issue_detail.rs`**

Replace the stub:

```rust
use crate::api::models::Issue;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, issue: &Issue) {
    let popup_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup_area);

    let assignee = issue
        .assignee
        .as_ref()
        .map(|u| u.name.as_str())
        .unwrap_or("-");
    let priority = issue
        .priority
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("-");
    let issue_type = issue
        .issue_type
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("-");
    let due_date = issue.due_date.as_deref().unwrap_or("-");
    let description = issue.description.as_deref().unwrap_or("(説明なし)");

    let mut lines = vec![
        Line::from(vec![Span::styled(
            issue.summary.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("担当者: ", Style::default().fg(Color::Yellow)),
            Span::raw(assignee),
            Span::raw("    "),
            Span::styled("優先度: ", Style::default().fg(Color::Yellow)),
            Span::raw(priority),
        ]),
        Line::from(vec![
            Span::styled("ステータス: ", Style::default().fg(Color::Yellow)),
            Span::raw(issue.status.name.as_str()),
            Span::raw("    "),
            Span::styled("種別: ", Style::default().fg(Color::Yellow)),
            Span::raw(issue_type),
        ]),
        Line::from(vec![
            Span::styled("期限: ", Style::default().fg(Color::Yellow)),
            Span::raw(due_date),
        ]),
        Line::from(""),
        Line::from(Span::styled("詳細:", Style::default().fg(Color::Yellow))),
        Line::from(""),
    ];

    for desc_line in description.lines() {
        lines.push(Line::from(desc_line.to_string()));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!(" {} ", issue.issue_key))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);

    // Help text
    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help =
            Paragraph::new("[Esc] 閉じる").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
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
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

- [ ] **Step 3: Commit**

```bash
git add src/ui/issue_detail.rs
git commit -m "feat: add issue detail popup widget"
```

---

### Task 9: Assignee filter popup

**Files:**
- Modify: `src/ui/filter.rs`

- [ ] **Step 1: Implement `src/ui/filter.rs`**

Replace the stub:

```rust
use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(40, 60, area);
    frame.render_widget(Clear, popup_area);

    let space_state = state.current_space_state();

    let items: Vec<ListItem> = if space_state.users_error {
        vec![ListItem::new("⚠ ユーザー取得失敗")]
    } else {
        let mut items = vec![ListItem::new("ALL (フィルターなし)")];
        if let Some(users) = &space_state.users {
            for user in users {
                items.push(ListItem::new(user.name.clone()));
            }
        }
        items
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Assigneeフィルター ")
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
    list_state.select(Some(state.filter_cursor_idx));

    frame.render_stateful_widget(list, popup_area, &mut list_state);

    if popup_area.height > 2 {
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width.saturating_sub(2),
            height: 1,
        };
        let help = Paragraph::new("[Enter] 選択  [Esc] キャンセル")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, help_area);
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
```

- [ ] **Step 2: Verify compilation**

```bash
cargo build
```

Expected: compiles cleanly.

- [ ] **Step 3: Commit**

```bash
git add src/ui/filter.rs
git commit -m "feat: add assignee filter popup widget"
```

---

### Task 10: Main event loop

**Files:**
- Modify: `src/main.rs`

This is the final integration step. Replace the placeholder `main.rs` with the full implementation.

- [ ] **Step 1: Replace `src/main.rs` with the full implementation**

```rust
use anyhow::Result;
// NOTE: Do NOT import `crossterm::event` with `{self, ...}` — the local `mod event`
// module creates an E0255 name conflict. Use only the specific types we need.
use crossterm::{
    event::KeyCode,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

mod api;
mod app;
mod config;
mod event;
mod ui;

use app::{AppState, Screen};
use event::AppEvent;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::load().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    #[cfg(unix)]
    if let Some(warning) = config::check_permissions(&config::config_path()) {
        eprintln!("{}", warning);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, config).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: config::Config,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Spawn key-reading thread — sends Key events to the shared channel.
    // IMPORTANT: Use fully-qualified crossterm::event::read() and
    // crossterm::event::Event::Key — NOT event::read() (which would look up
    // src/event.rs, which has no read() function).
    let key_tx = tx.clone();
    std::thread::spawn(move || loop {
        if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
            if key_tx.send(AppEvent::Key(key)).is_err() {
                break;
            }
        }
    });

    let mut state = AppState::new(config.clone());

    // Startup: fetch users for all spaces in parallel
    for space in &config.spaces {
        let space_name = space.name.clone();
        let host = space.host.clone();
        let api_key = space.api_key.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match api::client::BacklogClient::new(host, api_key) {
                Ok(client) => match client.fetch_projects().await {
                    Ok(projects) => {
                        let mut all_users: Vec<api::models::User> = Vec::new();
                        for project in projects {
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

    // Initial issue fetch for default space
    fetch_issues(&state, &config, tx.clone(), None);
    state.current_space_state_mut().loading_issues = true;

    loop {
        terminal.draw(|f| ui::render(f, &state))?;

        if let Some(ev) = rx.recv().await {
            match ev {
                AppEvent::Key(key) => match state.screen {
                    Screen::IssueList => handle_list_key(key, &mut state, &config, tx.clone()),
                    Screen::IssueDetail => handle_detail_key(key, &mut state),
                    Screen::Filter => handle_filter_key(key, &mut state, &config, tx.clone()),
                },
                other => {
                    state.handle_event(other);
                    if state.needs_issue_fetch() {
                        let assignee_id = state.filter_assignee_id;
                        fetch_issues(&state, &config, tx.clone(), assignee_id);
                        state.current_space_state_mut().loading_issues = true;
                    }
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_list_key(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => state.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => state.navigate_up(),
        KeyCode::Enter => {
            if let Some(issue) = state.selected_issue() {
                let issue_key = issue.issue_key.clone();
                let space_name = state.current_space_name().to_string();
                let space_cfg = config
                    .spaces
                    .iter()
                    .find(|s| s.name == space_name)
                    .unwrap()
                    .clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    match api::client::BacklogClient::new(space_cfg.host, space_cfg.api_key) {
                        Ok(client) => match client.fetch_issue(&issue_key).await {
                            Ok(issue) => {
                                let _ = tx.send(AppEvent::IssueDetailLoaded(issue));
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
        }
        KeyCode::Char('f') => {
            let assignee_id = state.filter_assignee_id;
            state.filter_cursor_idx = if assignee_id.is_none() {
                0
            } else {
                state
                    .current_space_state()
                    .users
                    .as_ref()
                    .and_then(|users| {
                        users
                            .iter()
                            .position(|u| Some(u.id) == assignee_id)
                            .map(|i| i + 1)
                    })
                    .unwrap_or(0)
            };
            state.screen = Screen::Filter;
        }
        KeyCode::Char('r') => {
            let assignee_id = state.filter_assignee_id;
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, assignee_id);
        }
        KeyCode::Char(']') => {
            state.switch_space_next();
            if state.needs_issue_fetch() {
                let assignee_id = state.filter_assignee_id;
                fetch_issues(state, config, tx, assignee_id);
                state.current_space_state_mut().loading_issues = true;
            }
        }
        KeyCode::Char('[') => {
            state.switch_space_prev();
            if state.needs_issue_fetch() {
                let assignee_id = state.filter_assignee_id;
                fetch_issues(state, config, tx, assignee_id);
                state.current_space_state_mut().loading_issues = true;
            }
        }
        _ => {}
    }
}

fn handle_detail_key(key: crossterm::event::KeyEvent, state: &mut AppState) {
    if key.code == KeyCode::Esc {
        state.screen = Screen::IssueList;
        state.detail_issue = None;
    }
}

fn handle_filter_key(
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    let user_count = state
        .current_space_state()
        .users
        .as_ref()
        .map(|u| u.len())
        .unwrap_or(0);
    let total = user_count + 1; // +1 for "ALL"

    match key.code {
        KeyCode::Esc => state.screen = Screen::IssueList,
        KeyCode::Char('j') | KeyCode::Down => {
            if state.filter_cursor_idx + 1 < total {
                state.filter_cursor_idx += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.filter_cursor_idx > 0 {
                state.filter_cursor_idx -= 1;
            }
        }
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
            let assignee_id = state.filter_assignee_id;
            state.current_space_state_mut().issues = None;
            state.current_space_state_mut().loading_issues = true;
            fetch_issues(state, config, tx, assignee_id);
        }
        _ => {}
    }
}

fn fetch_issues(
    state: &AppState,
    config: &config::Config,
    tx: mpsc::UnboundedSender<AppEvent>,
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
            Ok(client) => match client.fetch_issues(assignee_id).await {
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

- [ ] **Step 2: Verify the E0255 import conflict is avoided**

The import block already avoids `{self}` from `crossterm::event` (which would conflict with `mod event`). The key-reading thread uses fully-qualified paths:

```rust
std::thread::spawn(move || loop {
    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
        if key_tx.send(AppEvent::Key(key)).is_err() {
            break;
        }
    }
});
```

If the compiler reports `E0255: the name 'event' is defined multiple times`, the fix is to remove any `crossterm::event::{self, ...}` import and use only specific types like `KeyCode`.

- [ ] **Step 3: Build the project**

```bash
cargo build --release
```

Expected: compiles with no errors.

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5: Smoke test with real config (if available)**

If you have a real `~/.config/lazybacklog/config.toml`:

```bash
cargo run
```

Expected: TUI launches, shows loading, then issue list. Navigate with j/k, press f for filter, Enter for detail, q to quit.

If no config, verify the error message:

```bash
cargo run 2>&1
```

Expected: `Error: Config file not found: ...`

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement main event loop and key handlers"
```

---

### Task 11: Final cleanup and release build

- [ ] **Step 1: Run full test suite**

```bash
cargo test -- --nocapture
```

Expected: all tests pass, no warnings about unused code (fix any warnings).

- [ ] **Step 2: Check for warnings and fix them**

```bash
cargo build 2>&1 | grep "^warning"
```

Fix any unused variable or import warnings by prefixing with `_` or removing.

- [ ] **Step 3: Release build**

```bash
cargo build --release
```

Expected: binary at `target/release/lazybacklog`.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: lazybacklog v1 complete — issue browser TUI for Backlog"
```

---

## Quick Reference: Config File Setup

Before running, create `~/.config/lazybacklog/config.toml`:

```toml
default_space = "myspace"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "YOUR_API_KEY_HERE"
```

Then set permissions:

```bash
chmod 600 ~/.config/lazybacklog/config.toml
```

Get your API key from: Backlog → Personal Settings → API → Add API Key
