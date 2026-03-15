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

### Cursor index model

**The existing cursor fields (`selected_issue_idx`, `filter_cursor_idx`, `status_filter_cursor_idx`) always index into the full unfiltered list.** Filtering is a rendering and navigation concern only — the underlying cursor is kept in full-list space at all times.

This means:
- Rendering: compute filtered indices dynamically, display only matching rows
- `navigate_down` / `navigate_up` / `handle_filter_key` j/k / `handle_status_filter_key` j/k: **when `search_query` is non-empty, skip non-matching items** — j moves to the next matching full-list index, k moves to the previous matching full-list index. When `search_query` is empty, behavior is unchanged (full list navigation).
- `n`/`N`: compute the filtered match list, find the n-th match's full-list index, then set `selected_issue_idx` (or equivalent) to that index
- Opening IssueDetail on Enter: `state.selected_issue()` still uses `selected_issue_idx` into the original issues slice — no change needed

`search_match_idx` tracks position within the *filtered* match list for `n`/`N` navigation. When the query changes, `search_match_idx` is reset to 0 and `selected_issue_idx` / cursor is set to the full-list index of the first match (if any).

### Match lists

Match lists are computed dynamically at render/key-handling time from `search_query` — not stored in state. This ensures they stay in sync automatically when the query changes.

**Search targets by screen:**
- `IssueList`: `issue_key` + `summary`, case-insensitive (`to_lowercase().contains(...)`)
- Assignee Filter popup: user `name` case-insensitive, plus the literal string `"all"` for the "ALL" row (index 0)
- Status Filter popup: status `name`, case-insensitive

## Key Handling

Each `handle_*_key` function checks `state.search_active` at the top. **Note: while `search_active = true`, ALL `Char(c)` inputs are captured by the search handler — including `f`, `s`, `r`, etc. There is no way to open a filter popup or trigger refresh while in search mode. This is intentional.**

### While `search_active = true` (all screens):
| Key | Action |
|-----|--------|
| `Char(c)` | Append to `search_query`, reset `search_match_idx = 0`, move cursor to first match's full-list index |
| `Backspace` | Remove last char from `search_query`. If query is still non-empty: reset `search_match_idx = 0`, move cursor to first match. If query becomes empty: keep cursor at current full-list position (do not reset to 0 — Esc is the explicit reset path). |
| `Enter` | `search_active = false` (query remains, n/N still works) |
| `Esc` | `search_active = false`, clear `search_query`, reset `search_match_idx = 0`, reset cursor to 0 |

### Normal mode additions (IssueList, Filter, StatusFilter):
| Key | Action |
|-----|--------|
| `/` | `search_active = true`, clear `search_query`, reset `search_match_idx = 0` |
| `n` (`KeyCode::Char('n')`) | If `search_query` non-empty: increment `search_match_idx` (wrap around), set cursor to match's full-list index |
| `N` (`KeyCode::Char('N')`, i.e. shift+N) | If `search_query` non-empty: decrement `search_match_idx` (wrap around), set cursor to match's full-list index |

**Note on `N`:** crossterm emits uppercase letters as `KeyCode::Char('N')` when shift is held. No explicit `KeyModifiers` check is needed — matching `KeyCode::Char('N')` is sufficient.

### n/N cursor movement logic:
1. Compute match list: indices (in full list) of items matching `search_query`
2. `n`: `search_match_idx = (search_match_idx + 1) % match_count`
3. `N`: `search_match_idx = (search_match_idx + match_count - 1) % match_count`
4. Set `selected_issue_idx` / `filter_cursor_idx` / `status_filter_cursor_idx` to `match_list[search_match_idx]`
5. Zero matches: no-op

### Clear search state triggers

Search state (`search_active = false`, `search_query = ""`, `search_match_idx = 0`, cursor reset to 0) is cleared when:
- Space switching (`[` / `]` keys)
- Opening IssueDetail (Enter in IssueList while not in search mode, or `IssueDetailLoaded` event)
- `handle_filter_key` Enter (applies assignee filter, transitions to IssueList)
- `handle_status_filter_key` Enter (applies status filter, transitions to IssueList)
- `r` refresh key in IssueList (new issues are fetched; re-applying old query to new results would be confusing)

## UI Rendering

### IssueList (`src/ui/issue_list.rs`)
When `search_active || !search_query.is_empty()`, replace the footer line (issue count) with a search bar:
```
/ proj-1█                    (3 matches)
```
The issue table shows only matching issues when `search_query` is non-empty (render pass filters the slice before building rows). The table still uses `selected_issue_idx` to determine the highlighted row; the `TableState::select` call uses the position of `selected_issue_idx` within the *filtered* rows slice.

### Assignee Filter popup (`src/ui/filter.rs`)
Add a search bar at the bottom of the popup. List is filtered to matching users only (plus "ALL" row if query matches "all"). `filter_cursor_idx` keeps its full-list semantics; the rendered list shows only matching rows.

### Status Filter popup (`src/ui/status_filter.rs`)
Same pattern as Assignee Filter. Filtered status list is rendered; `status_filter_cursor_idx` stays in full-list space.

### Help bar (`src/ui/mod.rs`)
- Normal mode (no active search): add `[/] 検索` to existing help text
- Typing mode (`search_active = true`): show `[Enter] 確定  [Esc] キャンセル`  *(n/N are NOT shown here — they don't work while typing)*
- Post-confirm mode (`search_active = false`, `search_query` non-empty): add `[n/N] 次/前のマッチ` to normal help text

## Data Flow

1. User presses `/` on IssueList → `search_active = true`, search query cleared, search bar appears
2. User types characters → `search_query` grows, list filters in real-time, cursor moves to first match
3. User presses `Enter` → `search_active = false`, filtered view remains, n/N available
4. User presses `n`/`N` → cursor moves through matches in full-list space
5. User presses `Esc` → clears query, all items visible again, cursor reset to 0

## Error Handling / Edge Cases

- Zero matches: show empty list (or "ALL" only for assignee popup), show `(0 matches)` in search bar
- `n`/`N` with zero matches: no-op
- Query non-empty but `search_active = false`: filtered view is still active until Esc is pressed
- Popup Enter (apply filter) clears search state so IssueList starts fresh

## Testing

- Unit test: filtered issue list with query matching `issue_key`
- Unit test: filtered issue list with query matching `summary`
- Unit test: empty query returns all issues
- Unit test: case-insensitive matching
- Unit test: `n`/`N` wrap-around at list boundaries (uses full-list indices)
- Unit test: Esc clears query and resets cursor to 0
- Unit test: Assignee filter — "ALL" row matches query "all"
- Unit test: Status filter — filtered list excludes non-matching statuses
- Unit test: `r` refresh clears search state
