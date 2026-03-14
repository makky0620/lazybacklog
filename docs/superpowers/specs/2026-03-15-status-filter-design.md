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

1. User selects a project → `fetch_issues` + `fetch_statuses` called concurrently
2. `StatusesLoaded { space, statuses }` event updates `SpaceState.statuses`
3. Default filter applied: all statuses except name == "完了" or "Closed"
4. User presses `s` → `Screen::StatusFilter` opens
5. User toggles statuses with `Space`, confirms with `Enter`
6. On confirm: `filter_status_ids` updated, `fetch_issues` called with new status IDs
7. Issues re-rendered with filtered results

### API

**Endpoint:** `GET /api/v2/projects/:projectIdOrKey/statuses`

Returns an array of `IssueStatus` objects (`{ id, name }`). Added as `fetch_statuses` in `src/api/client.rs`.

`fetch_issues` gains a `status_ids: &[i64]` parameter, sent as `statusId[]` query params.

### State Changes (`src/app.rs`)

**`SpaceState` additions:**
```rust
pub statuses: Option<Vec<IssueStatus>>,
pub loading_statuses: bool,
```

**`AppState` additions:**
```rust
pub filter_status_ids: Vec<i64>,     // Default: IDs of all non-closed statuses
pub status_filter_cursor_idx: usize,
pub status_filter_pending: Vec<i64>, // Temporary state while popup is open
```

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
- Serialize `status_ids` as repeated `statusId[]` query params

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

While `statuses` is `None` (loading), display "読み込み中..." inside the popup.

### `src/ui/mod.rs`

- Call `status_filter::render` when `screen == Screen::StatusFilter`
- Update filter bar display:
  - When all statuses selected: `Status: ALL`
  - When none selected: `Status: (なし)`
  - Otherwise: `Status: 未対応, 処理中, 処理済み` (comma-separated names)

### `src/main.rs`

**Key bindings on `IssueList` screen:**
- `s` → copy `filter_status_ids` to `status_filter_pending`, set `screen = StatusFilter`

**Key bindings on `StatusFilter` screen:**
- `j` / `Down` → increment `status_filter_cursor_idx`
- `k` / `Up` → decrement `status_filter_cursor_idx`
- `Space` → toggle cursor's status ID in `status_filter_pending`
- `Enter` → apply `status_filter_pending` to `filter_status_ids`, refetch issues, return to `IssueList`
- `Esc` → discard `status_filter_pending`, return to `IssueList`

**Help bar update:** Add `[s] ステータスフィルター` to `IssueList` help text.

### Default Filter Logic

When `StatusesLoaded` is received:
1. If `filter_status_ids` is empty (first load), set default: all status IDs where `name != "完了" && name != "Closed"`
2. Trigger `fetch_issues` with the default status IDs

## Error Handling

- If `fetch_statuses` fails: log error via `ApiError` event, leave `statuses` as `None`
- If `statuses` is `None` when user presses `s`: show popup with "読み込み中..."
- If `filter_status_ids` is empty (user deselected all): fetch with no `statusId[]` params (API returns all statuses)

## Testing

- Unit test: default filter excludes statuses named "完了" and "Closed"
- Unit test: `status_filter_pending` toggle logic (add/remove IDs)
- Unit test: filter bar display text generation (ALL / なし / comma-separated)
- Integration: `fetch_issues` correctly serializes `statusId[]` query params
