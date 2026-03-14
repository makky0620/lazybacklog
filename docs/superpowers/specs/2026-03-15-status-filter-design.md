# Status Filter Design

**Date:** 2026-03-15
**Status:** Approved

## Overview

Add status-based filtering to lazybacklog, a Backlog project management TUI client written in Rust. By default, show only "Not Closed" tickets (all statuses except those whose name is "完了" or "Closed"). Users can open a status filter popup to toggle individual statuses on/off.

## Goals

- Filter issues by status (multi-select, server-side)
- Default to "Not Closed" (exclude statuses named "完了" or "Closed")
- Fetch available statuses from Backlog API per project (to support custom statuses)
- Integrate naturally with the existing assignee filter UX

## Architecture

### Data Flow

1. User selects a project → `fetch_statuses(project.id)` is called **first**
2. `StatusesLoaded { space, statuses }` event updates `SpaceState.statuses`
3. Default filter applied: all status IDs where name != "完了" and name != "Closed", stored in `SpaceState.filter_status_ids`
4. `fetch_issues` is called with the default `filter_status_ids`
5. `IssuesLoaded` event updates `SpaceState.issues`
6. User presses `s` → `Screen::StatusFilter` opens, copying `filter_status_ids` to `status_filter_pending`
7. User toggles statuses with `Space`, confirms with `Enter`
8. On confirm: `SpaceState.filter_status_ids` updated, `fetch_issues` called with new status IDs

Note: `fetch_statuses` and `fetch_issues` are **sequential**, not concurrent — issues are fetched only after statuses are loaded and the default filter is computed.

### API

**Endpoint:** `GET /api/v2/projects/:projectIdOrKey/statuses`

Called with `project.id` (i64), consistent with existing `fetch_project_users` convention. Returns an array of `IssueStatus` objects (`{ id, name }`). Added as `fetch_statuses(project_id: i64)` in `src/api/client.rs`.

`fetch_issues` gains a `status_ids: &[i64]` parameter, sent as repeated `statusId[]` query params. If `status_ids` is empty, no `statusId[]` params are sent (API returns all statuses).

### State Changes (`src/app.rs`)

**`SpaceState` additions** (per-space, resets on project change):
```rust
pub statuses: Option<Vec<IssueStatus>>,  // None = not yet loaded
pub loading_statuses: bool,
pub filter_status_ids: Vec<i64>,         // Default: IDs of all non-closed statuses
```

**`AppState` additions:**
```rust
pub status_filter_cursor_idx: usize,
pub status_filter_pending: Vec<i64>,     // Temporary state while popup is open
```

`filter_status_ids` lives in `SpaceState` (not `AppState`) so each space/project has independent filter state and defaults are computed correctly on every project switch.

**`Screen` enum addition:**
```rust
StatusFilter,
```

**`AppEvent` addition:**
```rust
StatusesLoaded { space: String, statuses: Vec<IssueStatus> },
```

## Components

### `src/api/client.rs`

- Add `fetch_statuses(project_id: i64) -> Result<Vec<IssueStatus>>`
- Update `fetch_issues` signature: add `status_ids: &[i64]` parameter
- Serialize `status_ids` as repeated `statusId[]` query params (skip if empty)

### `src/ui/status_filter.rs` (new file)

Centered popup, similar structure to `src/ui/filter.rs`:

```
┌── ステータスフィルター ────────────────┐
│  [✓] 未対応                           │
│  [✓] 処理中                           │
│▶ [✓] 処理済み                         │
│  [ ] 完了                             │
│                                       │
│  [j/k] 移動  [Space] 切替  [Enter] 決定 │
└───────────────────────────────────────┘
```

Render order follows `SpaceState.statuses` Vec order (as returned by the API). Checkbox rendering iterates statuses in order, checking membership in `status_filter_pending` via `Vec::contains` (O(n), acceptable for typical status counts of 4–10).

While `statuses` is `None` (loading or load failed), display "読み込み中..." inside the popup.

### `src/ui/mod.rs`

- Call `status_filter::render` when `screen == Screen::StatusFilter`
- Update filter bar display:
  - When all statuses selected: `Status: ALL`
  - When none selected: `Status: (なし)`
  - Otherwise: `Status: 未対応, 処理中, 処理済み` (comma-separated names, looked up from `SpaceState.statuses`)

### `src/main.rs`

**Key bindings on `IssueList` screen:**
- `s` → copy `space_state.filter_status_ids` to `status_filter_pending`, reset `status_filter_cursor_idx` to 0, set `screen = StatusFilter`

**Key bindings on `StatusFilter` screen:**
- `j` / `Down` → increment `status_filter_cursor_idx`
- `k` / `Up` → decrement `status_filter_cursor_idx`
- `Space` → toggle cursor's status ID in `status_filter_pending`
- `Enter` → apply `status_filter_pending` to `space_state.filter_status_ids`, refetch issues, set `screen = IssueList`
- `Esc` → discard `status_filter_pending`, set `screen = IssueList`

**Help bar update:** Add `[s] ステータスフィルター` to `IssueList` help text.

### Default Filter Logic (in `handle_event`)

When `StatusesLoaded { space, statuses }` is received:
1. Store `statuses` in `SpaceState.statuses`, set `loading_statuses = false`
2. Compute default: `filter_status_ids` = all IDs where `name != "完了" && name != "Closed"`
3. Call `fetch_issues` with the computed `filter_status_ids`

### Project Change Reset

When the user returns to `ProjectSelect` screen and selects a new project, reset the current space's `SpaceState`:
```rust
space_state.statuses = None;
space_state.loading_statuses = false;
space_state.filter_status_ids = vec![];
space_state.issues = None;
```
This ensures stale statuses from a previous project are not shown.

## Error Handling

- If `fetch_statuses` fails: send `ApiError` event; `handle_event` for `ApiError` must reset `loading_statuses = false` (in addition to existing resets for `loading_issues`, `loading_projects`). Leave `statuses` as `None`.
- If `statuses` is `None` when user presses `s`: show popup with "読み込み中..."
- If `filter_status_ids` is empty (user deselected all): fetch with no `statusId[]` params (API returns all statuses)

## Testing

- Unit test: default filter logic excludes statuses named "完了" and "Closed", includes others
- Unit test: `status_filter_pending` toggle logic (Vec::contains + retain/push)
- Unit test: filter bar display text — ALL / なし / comma-separated names
- Unit test: `ApiError` resets `loading_statuses = false`
- Integration: `fetch_issues` correctly serializes `statusId[]` query params (multiple values)
- Integration: `fetch_statuses` calls correct endpoint with `project.id`
