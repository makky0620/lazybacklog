# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run                # run the app
cargo test               # run all tests
cargo test <test_name>   # run a single test
cargo clippy             # lint
cargo fmt --check        # format check
cargo fmt                # auto-format
```

## Architecture

**lazybacklog** is a lazygit-inspired Terminal UI (TUI) for [Nulab's Backlog](https://backlog.com) project management service.

### Module Structure

```
src/
├── main.rs              # Entry point: event loop, keyboard handlers, tokio::main
├── app.rs               # AppState, SpaceState, Screen enum — all UI state
├── config.rs            # TOML config loading (~/.config/lazybacklog/config.toml)
├── event.rs             # AppEvent enum (all async events flow through here)
├── api/
│   ├── client.rs        # BacklogClient — all HTTP API calls via reqwest
│   └── models.rs        # Issue, User, Project, IssueStatus, etc.
└── ui/
    ├── mod.rs           # render() dispatcher by Screen
    ├── issue_list.rs    # Issue table
    ├── issue_detail.rs  # Issue detail popup
    ├── filter.rs        # Assignee filter popup
    ├── status_filter.rs # Status filter popup
    └── project_select.rs# Project selection screen
```

### Core Data Flow

1. Config loads multiple Backlog spaces (host + API key each)
2. Startup fetches projects & users for all spaces in parallel (tokio tasks → AppEvent channel)
3. User selects project → fetch statuses (auto-exclude "完了"/"Closed") → fetch issues
4. Keyboard events mutate `AppState` → re-render via ratatui

### State Management

- `AppState` is the single source of truth, held in `main.rs`
- `HashMap<String, SpaceState>` holds per-space state (projects, issues, filters, etc.)
- `Screen` enum drives which UI widget renders: `ProjectSelect | IssueList | IssueDetail | Filter | StatusFilter`
- Async tasks are spawned from the main event loop and send results back via an unbounded `mpsc::channel::<AppEvent>()`

### Key Patterns

- **Event-driven**: All async fetch results come back as `AppEvent` variants; state is only mutated in the main loop's `handle_event()`
- **Filter composition**: `assignee_id` and `filter_status_ids` are combined into query params in a single `fetch_issues()` call
- **Pending filter state**: Filter popups store changes in `*_pending` fields; only committed to real state on Enter
- **Multi-space**: `[` / `]` keys switch the active space; each space has independent state
- **Default status filter**: Statuses named "完了" or "Closed" are auto-excluded when first fetched

### Testing

- Unit tests live inline (config parsing, state logic, UI text)
- Integration tests for `BacklogClient` use `wiremock` to mock HTTP responses
- Config tests use `tempfile` for isolated config file creation
