# Project Selection Screen Design

**Date:** 2026-03-15
**Status:** Approved

## Overview

Add a project selection screen that appears at startup, before the issue list. The user selects a project from the current Backlog space, then the issue list is filtered to that project.

## User Flow

```
App starts
  → fetch projects for current space (API), loading_projects = true for ALL spaces
  → Screen::ProjectSelect (loading → list)
  → user presses Enter on a project
  → loading_issues = true, fetch issues (projectId[], optional assigneeId[])
  → Screen::IssueList

Space switch (from IssueList only — suppressed from ProjectSelect):
  → switch_space_next/prev → screen = Screen::ProjectSelect
  → needs_projects_fetch() fires from event loop if projects not yet loaded
  → fetch_projects(), loading_projects = true
  → user selects project → fetch issues
```

## State Changes

### `Screen` enum (`app.rs`)
Add `ProjectSelect` variant. Initial screen changes from `IssueList` to `ProjectSelect`.

### `SpaceState` (`app.rs`)
```rust
pub projects: Option<Vec<Project>>,
pub loading_projects: bool,
pub selected_project: Option<Project>,  // per-space: each space retains its own selection
```

### `AppState` (`app.rs`)
```rust
pub project_cursor_idx: usize,  // cursor for project selection UI (follows filter_cursor_idx pattern)
```

Navigation is mutated inline in `handle_project_select_key()`, following the same pattern as `filter_cursor_idx` in `handle_filter_key()`. No new helper methods on `AppState`.

Add convenience accessor:
```rust
pub fn selected_project(&self) -> Option<&Project> {
    self.current_space_state().selected_project.as_ref()
}
```

Add `needs_projects_fetch()` (mirrors `needs_issue_fetch()`):
```rust
pub fn needs_projects_fetch(&self) -> bool {
    let state = self.current_space_state();
    state.projects.is_none() && !state.loading_projects
}
```

Update `app.rs` import: `use crate::api::models::{Issue, Project, User};`

Note on `filter_assignee_id`: stays on `AppState` (not per-space) and is reset on space switch. Assignee filters are not preserved across space switches, consistent with the reset already applied to `selected_issue_idx` and `detail_issue`.

### `AppEvent` (`event.rs`)
```rust
ProjectsLoaded { space: String, projects: Vec<Project> }
```

Update `event.rs` import: `use crate::api::models::{Issue, Project, User};`

### `handle_event` in `AppState` (`app.rs`)

Add `ProjectsLoaded` arm:
```rust
AppEvent::ProjectsLoaded { space, projects } => {
    if let Some(state) = self.spaces.get_mut(&space) {
        state.projects = Some(projects);
        state.loading_projects = false;
    }
}
```

Update `ApiError` arm to also reset `loading_projects`:
```rust
AppEvent::ApiError { space, message } => {
    ...
    if let Some(state) = self.spaces.get_mut(&space) {
        state.loading_issues = false;
        state.loading_projects = false;  // add this line
        if state.users.is_none() { state.users_error = true; }
    }
}
```

### `switch_space_next()` / `switch_space_prev()` (`app.rs`)
Change `self.screen = Screen::IssueList` to `self.screen = Screen::ProjectSelect`, and also reset:
```rust
self.selected_issue_idx = 0;
self.detail_issue = None;
self.project_cursor_idx = 0;
self.filter_assignee_id = None;
self.screen = Screen::ProjectSelect;
// selected_project lives on SpaceState — each space retains its own selection
```

## API Changes

### `fetch_issues()` in `api/client.rs`
Add `project_id: Option<i64>` parameter. When `Some`, append `projectId[]` to query params.

### `fetch_issues()` helper in `main.rs`
Add `project_id: Option<i64>` parameter. Pass it to the API call alongside `assignee_id`. Call sites:
- After project selection in `handle_project_select_key()`: `project_id = state.selected_project().map(|p| p.id)`
- `'r'` refresh in `handle_list_key()`: same
- `handle_filter_key()` Enter: same (retain existing `issues = None` clear before fetch)
- Space-switch handlers (`[`/`]`) in `handle_list_key()`: **remove** the `needs_issue_fetch()` auto-fetch block entirely

Add `fetch_projects()` helper in `main.rs` (mirrors `fetch_issues()`):
```rust
fn fetch_projects(state: &AppState, config: &config::Config, tx: mpsc::UnboundedSender<AppEvent>) {
    let space_name = state.current_space_name().to_string();
    let space_cfg = config.spaces.iter().find(|s| s.name == space_name).unwrap().clone();
    tokio::spawn(async move {
        match BacklogClient::new(space_cfg.host, space_cfg.api_key) {
            Ok(client) => match client.fetch_projects().await {
                Ok(projects) => { let _ = tx.send(AppEvent::ProjectsLoaded { space: space_name, projects }); }
                Err(e) => { let _ = tx.send(AppEvent::ApiError { space: space_name, message: e.to_string() }); }
            },
            Err(e) => { let _ = tx.send(AppEvent::ApiError { space: space_name, message: e.to_string() }); }
        }
    });
}
```

## Startup Flow Changes (`main.rs`)

**New startup:**
1. Remove the initial `fetch_issues()` call entirely.
2. Set `loading_projects = true` for **all** spaces before spawning any tasks. This prevents `needs_projects_fetch()` from spuriously triggering in the event loop while the startup spawns are still in flight:
   ```rust
   for space in &config.spaces {
       state.spaces.get_mut(&space.name).unwrap().loading_projects = true;
   }
   ```
3. The existing per-space user-fetching loop is retained. For **all spaces**, clone `projects` before the user-fetch iteration and send `ProjectsLoaded`. This ensures `loading_projects` is properly reset for all spaces and `needs_projects_fetch()` never spuriously fires on space switch:
   ```rust
   let projects = client.fetch_projects().await?;
   // Send ProjectsLoaded for every space (clone into event, borrow original for user iteration)
   let _ = tx.send(AppEvent::ProjectsLoaded { space: space_name.clone(), projects: projects.clone() });
   // IMPORTANT: iterate by reference (&projects), not by value — the existing code does
   // `for project in projects` (by move) which must be changed to `for project in &projects`
   // so that `projects` is not consumed before the clone above.
   for project in &projects { ... }
   let _ = tx.send(AppEvent::SpaceUsersLoaded { ... });
   ```
   The clone is of a small `Vec<Project>` (typically tens of items).

### Event loop guards (`other =>` arm)
Two independent `if` blocks (not `else if`) — they are logically mutually exclusive by screen state, but written as independent `if` for clarity and future-safety:
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
    if state.screen == Screen::ProjectSelect && state.needs_projects_fetch() {
        fetch_projects(&state, &config, tx.clone());
        state.current_space_state_mut().loading_projects = true;
    }
}
```

## Key Handling

### Key dispatch match in event loop (`main.rs`)
Add the new arm to the `match state.screen` block that dispatches key events:
```rust
match state.screen {
    Screen::IssueList => handle_list_key(key, &mut state, &config, tx.clone()),
    Screen::IssueDetail => handle_detail_key(key, &mut state),
    Screen::Filter => handle_filter_key(key, &mut state, &config, tx.clone()),
    Screen::ProjectSelect => handle_project_select_key(key, &mut state, &config, tx.clone()),
}
```

### `handle_project_select_key()` (new, `main.rs`)
- `j` / `↓`: move `project_cursor_idx` down (bounded by project list length − 1)
- `k` / `↑`: move `project_cursor_idx` up (bounded at 0)
- `Enter`:
  - If `projects` is `None` or empty: no-op (no project to select; user can only `q`)
  - Otherwise (following `handle_filter_key()` pattern — clear issues, set state, then fetch):
    1. Set `state.current_space_state_mut().selected_project` from `project_cursor_idx`
    2. Set `state.screen = Screen::IssueList`
    3. Set `state.current_space_state_mut().issues = None` ← clear stale issues (matches `handle_filter_key()` pattern)
    4. Set `state.current_space_state_mut().loading_issues = true` ← **before** calling `fetch_issues`, preventing `needs_issue_fetch()` double-fire
    5. Call `fetch_issues(&state, &config, tx, project_id, assignee_id)`
- `q`: quit
- `[` / `]`: **intentionally suppressed** — space switching is not available from the `ProjectSelect` screen. Design rationale: a project must be selected before entering the issue list; switching spaces from the selection screen would require additional UX (re-entering the selection for the new space) and is out of scope. Users who want to switch spaces must first complete project selection to reach `IssueList`, then use `[`/`]` there. The help bar on `ProjectSelect` does not show `[`/`]` hints, giving users no false affordance.

## UI

### `src/ui/project_select.rs` (new file)
`Screen::ProjectSelect` is a **full-screen takeover**, not a popup.

Renders a full 3-row layout (title bar, content, help bar):
- Title bar: `lazybacklog ──── [space_name]`
- Content: list of projects in `projectKey - name` format, selected row highlighted; or `Loading projects...` if `loading_projects`; or `No projects found.` if `projects == Some([])`
- Help bar: `[j/k] 移動  [Enter] 選択  [q] 終了`

### `src/ui/mod.rs`
Restructure `render()` with an early return for `ProjectSelect`:

```rust
pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if state.screen == Screen::ProjectSelect {
        // Full-screen takeover: render project select layout first,
        // then overlay status message on the bottom line (same as main layout).
        // Order matters: project_select::render paints the help bar,
        // render_status_message overlays it when an error is present.
        project_select::render(frame, area, state);
        render_status_message(frame, area, state);
        return;
    }

    let chunks = Layout::default()...split(area);
    render_title(frame, chunks[0], state);
    render_filter_bar(frame, chunks[1], state);
    issue_list::render(frame, chunks[2], state);
    render_help_bar(frame, chunks[3]);

    match state.screen {
        Screen::IssueDetail => { if let Some(issue) = &state.detail_issue { issue_detail::render(...) } }
        Screen::Filter => { filter::render(...) }
        Screen::IssueList => {}
        Screen::ProjectSelect => {}  // dead code — early return above handles this; satisfies exhaustiveness
    }

    render_status_message(frame, area, state);
}
```

Extract status message rendering into `render_status_message(frame, area, state)` helper to avoid duplication between the two branches.

## Tests

### `app.rs`
- `test_initial_state`: update assertion from `Screen::IssueList` to `Screen::ProjectSelect`
- Add test for `ProjectsLoaded` event: verify `projects` stored, `loading_projects = false`
- Add test for `ApiError` resetting `loading_projects`
- Add test for `switch_space_next/prev`: verify `project_cursor_idx = 0`, `filter_assignee_id = None`, `screen = Screen::ProjectSelect`
- Add test for `needs_projects_fetch()`: returns `true` when `projects == None && !loading_projects`, `false` otherwise

### `api/client.rs`
- Add `test_fetch_issues_with_project_filter`: verify `projectId[]` query param sent when `project_id = Some(id)`, following pattern of `test_fetch_issues_with_assignee_filter`

## Files to Change

| File | Change |
|------|--------|
| `src/app.rs` | Add `ProjectSelect` to `Screen`; update import to include `Project`; fields to `SpaceState`; `project_cursor_idx` to `AppState`; `selected_project()` accessor; `needs_projects_fetch()`; handle `ProjectsLoaded`; update `ApiError` arm; update `switch_space_*`; update tests |
| `src/event.rs` | Add `ProjectsLoaded` variant; update import to include `Project` |
| `src/api/client.rs` | Add `project_id` param to `fetch_issues()`; add test |
| `src/main.rs` | Startup: set `loading_projects = true` for all spaces, send `ProjectsLoaded` for all spaces (clone), no initial `fetch_issues`; add `fetch_projects()` helper; add `handle_project_select_key()` (with `issues = None` clear); add `Screen::ProjectSelect => handle_project_select_key(...)` to key dispatch match; add `project_id` to `fetch_issues()` helper and all call sites; add both event-loop guards; remove issue-fetch from `[`/`]` handlers |
| `src/ui/mod.rs` | Early return for `ProjectSelect` with status overlay; extract `render_status_message()`; add exhaustive `Screen::ProjectSelect => {}` arm |
| `src/ui/project_select.rs` | New file: full-screen project selection widget |
