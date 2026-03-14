# Project Selection Screen Design

**Date:** 2026-03-15
**Status:** Approved

## Overview

Add a project selection screen that appears at startup, before the issue list. The user selects a project from the current Backlog space, then the issue list is filtered to that project.

## User Flow

```
App starts
  → fetch projects for current space (API)
  → Screen::ProjectSelect (loading → list)
  → user presses Enter on a project
  → fetch issues for selected project
  → Screen::IssueList
```

## State Changes

### `Screen` enum (`app.rs`)
Add `ProjectSelect` variant. Initial screen changes from `IssueList` to `ProjectSelect`.

### `SpaceState` (`app.rs`)
```rust
pub projects: Option<Vec<Project>>,
pub loading_projects: bool,
```

### `AppState` (`app.rs`)
```rust
pub selected_project_idx: usize,
pub selected_project: Option<Project>,
```

### `AppEvent` (`event.rs`)
```rust
ProjectsLoaded { space: String, projects: Vec<Project> }
```

## API Changes

### `fetch_issues()` (`api/client.rs`)
Add `project_id: Option<i64>` parameter. When `Some`, append `projectId[]` to query params.

## Startup Flow Changes (`main.rs`)

- Remove initial `fetch_issues()` call at startup.
- Instead, call `fetch_projects()` for the current space at startup.
- On `ProjectsLoaded` event: store projects in `SpaceState`, remain on `Screen::ProjectSelect`.
- On `Enter` in `ProjectSelect`: set `selected_project`, transition to `Screen::IssueList`, call `fetch_issues(project_id)`.

## Key Handling

New `handle_project_select_key()` in `main.rs`:
- `j` / `↓`: move cursor down
- `k` / `↑`: move cursor up
- `Enter`: confirm selection, fetch issues, go to `IssueList`
- `q`: quit

## UI (`src/ui/project_select.rs`)

New file following the same pattern as `filter.rs`. Renders a centered popup with:
- Title: `Select Project`
- Project list in `projectKey - name` format
- Selected row highlighted
- Loading state: show `Loading projects...`

`ui/mod.rs` renders this widget when `state.screen == Screen::ProjectSelect`.

Help bar shows `[j/k] 移動  [Enter] 選択  [q] 終了` on the `ProjectSelect` screen.

## Files to Change

| File | Change |
|------|--------|
| `src/app.rs` | Add `ProjectSelect` to `Screen`, fields to `SpaceState` and `AppState`, handle `ProjectsLoaded` event |
| `src/event.rs` | Add `ProjectsLoaded` variant |
| `src/api/client.rs` | Add `project_id` param to `fetch_issues()` |
| `src/main.rs` | Change startup flow, add `handle_project_select_key()`, update `fetch_issues` calls |
| `src/ui/mod.rs` | Render `ProjectSelect` screen, update help bar |
| `src/ui/project_select.rs` | New file: project selection widget |
