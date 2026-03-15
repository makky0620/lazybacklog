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

## Architecture

### Entry Point (`src/main.rs`)

Detect `--demo` via `std::env::args()` before loading config:

```rust
let demo_mode = std::env::args().any(|a| a == "--demo");

let config = if demo_mode {
    mock::demo_config()
} else {
    config::load().unwrap_or_else(|e| { ... })
};
```

`mock::demo_config()` returns a synthetic `Config` with a single space (`name: "demo"`, `host: "mock"`, `api_key: "mock"`). No file I/O occurs.

### State (`src/app.rs`)

Add `demo_mode: bool` to `AppState`:

```rust
pub struct AppState {
    pub demo_mode: bool,
    // ... existing fields
}
```

`AppState::new()` accepts `demo_mode` and stores it. All fetch dispatchers check this flag.

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

### Fetch Function Branching (`src/main.rs`)

Each fetch function (`fetch_projects`, `fetch_statuses`, `fetch_issues`, startup loop) checks `state.demo_mode` at the top and sends mock `AppEvent`s directly, bypassing HTTP:

```rust
fn fetch_projects(state: &AppState, ...) {
    if state.demo_mode {
        let space = state.current_space_name().to_string();
        let _ = tx.send(AppEvent::ProjectsLoaded { space: space.clone(), projects: mock::projects() });
        let _ = tx.send(AppEvent::SpaceUsersLoaded { space, users: mock::users() });
        return;
    }
    // existing HTTP code ...
}
```

The same pattern applies to `fetch_statuses` and `fetch_issues`. For the issue detail fetch (Enter key in `handle_list_key`), mock data returns the selected issue directly from mock data by issue key.

The startup loop (per-space `tokio::spawn`) also branches on `demo_mode` to send `ProjectsLoaded` and `SpaceUsersLoaded` without spawning HTTP tasks.

## Data Flow (Demo Mode)

```
cargo run -- --demo
  → mock::demo_config() → AppState { demo_mode: true }
  → startup loop: send ProjectsLoaded + SpaceUsersLoaded (mock)
  → user selects project → fetch_statuses: send StatusesLoaded (mock)
  → auto-fetch: fetch_issues: send IssuesLoaded (mock)
  → user presses Enter → send IssueDetailLoaded (mock issue)
```

The existing event-driven flow (`StatusesLoaded` triggers `needs_issue_fetch`) is reused unchanged.

## Files Changed

| File | Change |
|---|---|
| `src/mock.rs` | New: all mock data functions |
| `src/main.rs` | `--demo` detection, `demo_mode` threading, fetch function branches, startup loop branch |
| `src/app.rs` | `demo_mode: bool` field on `AppState` |

## Testing

No new tests required. Existing unit tests for `AppState` and `BacklogClient` are unaffected. Manual verification: `cargo run -- --demo` navigates all screens.
