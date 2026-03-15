# Search Feature Design

**Date:** 2026-03-15
**Scope:** vim-like `/` search for IssueList, Assignee Filter popup, and Status Filter popup

## Overview

Add client-side incremental search triggered by `/`, with `n`/`N` navigation between matches and Esc/Enter for cancel/confirm. No API calls — search operates on already-loaded data.

## State Management

Add three fields to `AppState` in `src/app.rs`:

```rust
pub search_active: bool,      // true while user is typing the query
pub search_query: String,     // current search string
pub search_match_idx: usize,  // cursor position within the match list (for n/N)
```

Match lists are computed dynamically at render/key-handling time from `search_query` — not stored in state. This ensures they stay in sync automatically when the query changes.

**Search targets by screen:**
- `IssueList`: `issue_key` + `summary`, case-insensitive (`to_lowercase()`)
- Assignee Filter popup: user `name`, case-insensitive
- Status Filter popup: status `name`, case-insensitive

When `search_query` is non-empty, only matching items are shown in the list. `selected_issue_idx` / `filter_cursor_idx` / `status_filter_cursor_idx` index into the *filtered* list, not the original.

## Key Handling

Each `handle_*_key` function checks `state.search_active` at the top:

### While `search_active = true` (all screens):
| Key | Action |
|-----|--------|
| `Char(c)` | Append to `search_query`, reset `search_match_idx = 0`, move cursor to first match |
| `Backspace` | Remove last char from `search_query`, reset `search_match_idx = 0`, update cursor |
| `Enter` | `search_active = false` (query remains, n/N still works) |
| `Esc` | `search_active = false`, clear `search_query`, reset cursor to 0 |

### Normal mode additions (IssueList, Filter, StatusFilter):
| Key | Action |
|-----|--------|
| `/` | `search_active = true`, clear `search_query` |
| `n` | If `search_query` non-empty: increment `search_match_idx` (wrap around), update cursor |
| `N` | If `search_query` non-empty: decrement `search_match_idx` (wrap around), update cursor |

### n/N cursor movement logic:
1. Compute match list from current issues/users/statuses filtered by `search_query`
2. `n`: `search_match_idx = (search_match_idx + 1) % match_count`
3. `N`: `search_match_idx = (search_match_idx + match_count - 1) % match_count`
4. Update `selected_issue_idx` / `filter_cursor_idx` / `status_filter_cursor_idx` to match

## UI Rendering

### IssueList (`src/ui/issue_list.rs`)
When `search_active || !search_query.is_empty()`, replace the footer line (issue count) with a search bar:
```
/ proj-1█                    (3 matches)
```
The issue table shows only matching issues when `search_query` is non-empty.

### Assignee Filter popup (`src/ui/filter.rs`)
Add a search bar at the bottom of the popup. List is filtered to matching users only.

### Status Filter popup (`src/ui/status_filter.rs`)
Same pattern as Assignee Filter.

### Help bar (`src/ui/mod.rs`)
- Normal mode: add `[/] 検索` to existing help text
- Search mode: show `[Enter] 確定  [Esc] キャンセル  [n/N] 移動`

## Data Flow

1. User presses `/` on IssueList → `search_active = true`, search bar appears
2. User types characters → `search_query` grows, list filters in real-time
3. User presses `Enter` → `search_active = false`, filtered view remains, n/N available
4. User presses `n`/`N` → cursor moves through matches
5. User presses `Esc` (or `/` again) → clears query, all items visible again

## Error Handling / Edge Cases

- If `search_query` is non-empty but there are zero matches: show empty list with "(0 matches)"
- `n`/`N` with zero matches: no-op
- Switching screens (e.g. opening IssueDetail with Enter while searching): search state should be cleared (`search_active = false`, `search_query = ""`)
- Space switching (`[`/`]`): clear search state

## Testing

- Unit test: filtered issue list with query matching `issue_key`
- Unit test: filtered issue list with query matching `summary`
- Unit test: empty query returns all issues
- Unit test: case-insensitive matching
- Unit test: `n`/`N` wrap-around at list boundaries
- Unit test: Esc clears query and resets cursor
