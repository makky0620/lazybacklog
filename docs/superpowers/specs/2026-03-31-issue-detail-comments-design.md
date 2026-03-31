# Issue Detail Comments — Design Spec

Date: 2026-03-31

## Overview

Display Backlog issue comments in the issue detail screen, below the description, using a unified scroll. Comments are fetched automatically when the issue detail is opened.

## Data Model

**New struct in `src/api/models.rs`:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub content: Option<String>,
    #[serde(rename = "createdUser")]
    pub created_user: Option<User>,
    pub created: String,
}
```

Fields map directly to Backlog API response keys (`id`, `content`, `createdUser`, `created`).

## API Client

**New method in `src/api/client.rs`:**

```rust
pub async fn fetch_comments(&self, issue_id_or_key: &str) -> Result<Vec<Comment>>
```

Calls `GET /api/v2/issues/{issueIdOrKey}/comments` with `apiKey` query param.

## Events

**New variant in `src/event.rs`:**

```rust
CommentsLoaded { issue_key: String, comments: Vec<Comment> },
```

The `issue_key` is carried so the handler can verify the loaded comments belong to the currently displayed issue.

## State

**New fields in `AppState` (`src/app.rs`):**

```rust
pub detail_comments: Option<Vec<Comment>>,
pub loading_comments: bool,
```

- `IssueDetailLoaded` handler resets both fields: `detail_comments = None`, `loading_comments = false`.
- `CommentsLoaded` handler sets `detail_comments = Some(comments)`, `loading_comments = false`, only if `issue_key` matches `detail_issue`.
- `detail_comments` is cleared alongside `detail_issue` on Esc and space switch.

## Fetch Trigger

In `src/handler.rs`, the `Enter` key handler that opens issue detail spawns two independent `tokio::spawn` tasks:

1. `fetch_issue` → sends `IssueDetailLoaded`
2. `fetch_comments` → sends `CommentsLoaded`

Both run concurrently. The UI handles whichever arrives first.

In demo mode, comments fetch is skipped (no API key available).

## UI

**`src/ui/issue_detail.rs`:**

- `render_description` renamed to `render_description_and_comments`
- Block title changed from `" Description "` to `" Description & Comments "`
- Function signature gains `comments: Option<&Vec<Comment>>`

**Content layout inside the block:**

```
<description lines>

── 1: Alice  2026-03-31 ──
<comment content lines>

── 2: Bob  2026-04-01 ──
<comment content lines>
```

- If `comments` is `None` (loading): append a `"(loading comments...)"` line after description.
- If `comments` is `Some([])` (empty): no separator or extra lines appended.
- Separator style: `Color::DarkGray`, author + date in `Color::Cyan`.
- All lines concatenated into a single `Vec<Line>` fed to one `Paragraph` widget, so `detail_scroll_offset` works unchanged.

## Help Bar

No change. `[j/k] Scroll  [o] Open  [Esc] Back` remains sufficient.

## Testing

- `models.rs`: deserialize `Comment` with renamed field `createdUser`, optional `content`.
- `client.rs`: wiremock test for `fetch_comments` success and 401.
- `app.rs`: `CommentsLoaded` sets `detail_comments`; mismatched `issue_key` is ignored; `IssueDetailLoaded` resets `detail_comments`.
- `issue_detail.rs`: render test verifies "Description & Comments" block title; loading state shows "(loading comments...)".
