# Demo Mode Design

**Date:** 2026-03-15
**Status:** Approved

## Summary

Add a `--demo` CLI flag that launches lazybacklog with hardcoded mock data, requiring no config file or API key. Intended for development/debugging and demo/screenshot use cases.

## Goals

- `cargo run -- --demo` starts the app with realistic mock data
- No `config.toml` required in demo mode
- All UI screens (project select, issue list, issue detail, filters) work end-to-end with mock data
- Mock data is in English

## Non-Goals

- CI/automated UI testing (not in scope)
- Configurable/file-based mock data
- Trait abstraction for `BacklogClient`
- Client-side filtering of mock data (filters UI works, but issue list always returns full mock set)

## Architecture

### Entry Point (`src/main.rs` — `main()`)

Detect `--demo` via `std::env::args()` before loading config:

```rust
let demo_mode = std::env::args().any(|a| a == "--demo");

let config = if demo_mode {
    mock::demo_config()
} else {
    config::load().unwrap_or_else(|e| { ... })
};

// Skip the permissions check in demo mode (no config file exists)
#[cfg(unix)]
if !demo_mode {
    if let Some(warning) = config::check_permissions(&config::config_path()) {
        eprintln!("{}", warning);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

// Pass demo_mode into run()
let result = run(&mut terminal, config, demo_mode).await;
```

`mock::demo_config()` returns a synthetic `Config` with **exactly one space** (`name: "demo"`, `host: "mock"`, `api_key: "mock"`). One space avoids needing to handle multi-space switching in demo mode.

### `run()` signature update

`run()` gains a `demo_mode: bool` parameter:

```rust
async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: config::Config,
    demo_mode: bool,
) -> Result<()> { ... }
```

`demo_mode` is passed to `AppState::new(config.clone(), demo_mode)` and also used directly in the startup loop branch and `fetch_*` function calls.

### State (`src/app.rs`)

Add `demo_mode: bool` to `AppState` and update `new()` signature:

```rust
pub struct AppState {
    pub demo_mode: bool,
    // ... existing fields
}

impl AppState {
    pub fn new(config: Config, demo_mode: bool) -> Self { ... }
}
```

**Call sites to update:**
- `main.rs`: `AppState::new(config.clone())` → `AppState::new(config.clone(), demo_mode)`
- `app.rs` unit tests (~29 call sites): `AppState::new(config)` → `AppState::new(config, false)`

### Mock Data (`src/mock.rs` — new file)

Centralizes all mock data:

| Function | Returns |
|---|---|
| `demo_config()` | `Config` with one demo space |
| `projects()` | 2 projects: `DEMO`, `SAMPLE` |
| `users()` | 3 users: Alice, Bob, Charlie |
| `statuses()` | 4 statuses: Open, In Progress, Resolved, Closed |
| `issues()` | ~10 issues with varied assignees, due dates, statuses |

All content is in English.

### Startup Loop (`src/main.rs` — `run()`)

In demo mode, mock events are sent **synchronously before the event loop** (no `tokio::spawn` needed). The `loading_projects = true` pre-set loop runs unconditionally for both modes:

```rust
// Runs for both modes
for space in &config.spaces {
    state.spaces.get_mut(&space.name).unwrap().loading_projects = true;
}

if demo_mode {
    for space in &config.spaces {
        let _ = tx.send(AppEvent::ProjectsLoaded {
            space: space.name.clone(),
            projects: mock::projects(),
        });
        let _ = tx.send(AppEvent::SpaceUsersLoaded {
            space: space.name.clone(),
            users: mock::users(),
        });
    }
} else {
    // existing per-space tokio::spawn HTTP code
}
```

### Fetch Function Branching (`src/main.rs`)

`fetch_projects`, `fetch_statuses`, and `fetch_issues` each check `state.demo_mode` at the top. In demo mode they send mock events synchronously and return immediately.

**`fetch_projects`** (called on lazy space-switch only — not startup):
```rust
fn fetch_projects(state: &AppState, ...) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::ProjectsLoaded { space, projects: mock::projects() });
        // SpaceUsersLoaded is NOT sent here — sent at startup only.
        // Safe: demo_config() returns exactly one space, so [ / ] switching is a no-op.
        return;
    }
    // existing HTTP code ...
}
```

**`fetch_statuses`** (`project_id: i64` parameter is accepted but intentionally unused in demo mode — all projects share the same mock statuses):
```rust
fn fetch_statuses(state: &AppState, config: &config::Config, tx: ..., project_id: i64) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::StatusesLoaded { space, statuses: mock::statuses() });
        return;
    }
    // existing HTTP code ...
}
```

**`fetch_issues`** (`assignee_id`, `status_ids`, and `project_id` are accepted but intentionally unused in demo mode — always returns the full mock issue list regardless of filters):
```rust
fn fetch_issues(state: &AppState, ...) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::IssuesLoaded { space, issues: mock::issues() });
        return;
    }
    // existing HTTP code ...
}
```

### Issue Detail Fetch (Enter key in `handle_list_key`)

In demo mode, no HTTP call is needed. Clone the already-selected issue from `state.selected_issue()` and send as `IssueDetailLoaded`:

```rust
KeyCode::Enter => {
    if state.demo_mode {
        if let Some(issue) = state.selected_issue().cloned() {
            let _ = tx.send(AppEvent::IssueDetailLoaded(issue));
        }
        return;
    }
    // existing tokio::spawn HTTP code ...
}
```

## Data Flow (Demo Mode)

```
cargo run -- --demo
  → mock::demo_config() → run(..., demo_mode=true)
  → AppState::new(config, true) → state.demo_mode = true
  → startup: send ProjectsLoaded + SpaceUsersLoaded synchronously (mock)
  → user selects project → fetch_statuses: send StatusesLoaded (mock, sync, project_id ignored)
  → auto-fetch: fetch_issues: send IssuesLoaded (mock, sync, filters ignored)
  → user presses Enter → clone selected_issue(), send IssueDetailLoaded
```

The startup mock events only load projects and users. Issue loading still follows the normal project-select → statuses → issues chain: the user must select a project (Enter on `ProjectSelect`), which triggers `fetch_statuses`; on `StatusesLoaded`, `needs_issue_fetch()` becomes true and the event loop auto-calls `fetch_issues`. All guards are reused unchanged.

Assignee/status filter UI works (popup navigates, Enter commits) but `fetch_issues` always returns the full mock list.

## Files Changed

| File | Change |
|---|---|
| `src/mock.rs` | New: all mock data functions |
| `src/main.rs` | `--demo` detection; skip permissions check; `run()` gains `demo_mode` param; startup loop branch; `AppState::new(config, demo_mode)`; fetch function branches; Enter-key detail branch |
| `src/app.rs` | `demo_mode: bool` on `AppState`; `new()` gains `demo_mode` param; ~29 test call sites updated to `AppState::new(config, false)` |

## Testing

Existing unit tests in `app.rs` compile after updating all `AppState::new(config)` calls to `AppState::new(config, false)`. No new tests required. Manual verification: `cargo run -- --demo` navigates all screens end-to-end.
