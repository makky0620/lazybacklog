# Open Issue in Browser — Design Spec

**Date:** 2026-03-23

## Overview

From the Issue Detail screen, pressing `o` opens the corresponding Backlog issue page in the system default browser.

## URL Format

```
https://{host}/view/{issue_key}
```

- `host` — from the active space's `SpaceConfig.host` (e.g. `myspace.backlog.com`)
- `issue_key` — from `AppState.detail_issue.issue_key` (e.g. `PROJ-42`)

## Changes

### `Cargo.toml`
Add dependency:
```toml
open = "5"
```

### `src/handler.rs` — `handle_detail_key`
Add a new match arm. Use direct index access via `current_space_idx` (always valid by invariant):
```rust
KeyCode::Char('o') => {
    if let Some(issue) = &state.detail_issue {
        let url = format!(
            "https://{}/view/{}",
            state.config.spaces[state.current_space_idx].host,
            issue.issue_key
        );
        #[cfg(not(test))]
        let _ = open::that(url);
    }
}
```

### `src/ui/issue_detail.rs` — help bar
Update the string literal (note the leading space):
```
" [j/k] Scroll  [o] Open  [Esc] Back"
```

## Error Handling

Errors from `open::that()` are silently ignored. The TUI remains active; the user stays on the Issue Detail screen.

## Import

No `use` statement is required; `open::that()` is called via fully-qualified path `open::that(url)`.

## Testing

The `open::that()` call is guarded with `#[cfg(not(test))]` to prevent launching a browser during `cargo test`. Tests can therefore call `handle_detail_key` with `'o'` safely.

Add unit tests in `src/handler.rs` alongside the existing `handle_detail_key` tests:
- `KeyCode::Char('o')` with `detail_issue = Some(...)` — verify state is unchanged (screen, scroll, issue) and no panic
- `KeyCode::Char('o')` with `detail_issue = None` — verify no-op / no panic

## Out of Scope

- No status message / feedback after opening
- No demo mode special-casing (demo issues have no real URL, but the behavior is harmless)
