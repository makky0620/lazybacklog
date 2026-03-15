# Demo GIF Design

**Date:** 2026-03-15
**Status:** Approved

## Summary

Create a reproducible GIF demo of lazybacklog using [VHS](https://github.com/charmbracelet/vhs) for embedding in the README. The GIF is generated from a `.tape` script checked into the repository, making it easy to regenerate when the app changes.

## Goals

- Embed a GIF in `README.md` (and `README.ja.md`) that shows core usage
- Script-driven and reproducible (`demo/demo.tape` → `demo/demo.gif`)
- Uses `--demo` mode (no real Backlog credentials needed)

## Non-Goals

- Video (MP4) output
- asciinema recording
- Automated GIF regeneration in CI

## File Structure

```
demo/
  demo.tape   — VHS script (committed to repo)
  demo.gif    — Generated GIF (committed to repo)
```

## demo.tape Script

**Pre-requisite:** Run `cargo build --release` before recording so the binary is already compiled. Use `./target/release/lazybacklog --demo` in the tape script to avoid compile wait time.

**Terminal settings:**
- Output: `demo/demo.gif`
- Width: 220 columns, Height: 50 rows
- Font size: 14, Theme: Dracula (or similar dark theme)
- Framerate: 24

**Operation sequence:**

| Step | Action | VHS directive |
|------|--------|---------------|
| Launch app | `./target/release/lazybacklog --demo` | `Type "..."`, `Enter` |
| Wait for project select | — | `Sleep 1s` |
| Select project | `Enter` | `Enter`, `Sleep 500ms` |
| Browse issues | `j` × 3 (vi-style) | `Type j` × 3, `Sleep 300ms` each |
| Open issue detail | `Enter` | `Enter`, `Sleep 500ms` |
| Close detail | `Esc` | `Escape`, `Sleep 500ms` |
| Start search | `/` | `Type /`, `Sleep 300ms` |
| Type query | `login` (matches mock issue) | `Type "login"`, `Sleep 500ms` |
| Show match highlight | — | `Sleep 1s` |
| Exit search mode | `Esc` | `Escape`, `Sleep 300ms` |
| Quit | `q` | `Type q` |

Note: `Escape` before `q` is required — if search mode is still active, `q` appends to the query instead of quitting.

Note: A `Sleep 300ms` between `Type /` and `Type "login"` is required to let the UI re-render into search mode before characters arrive.

## README Integration

Insert `![Demo](demo/demo.gif)` *above* the existing `cargo run -- --demo` block in both `README.md` and `README.ja.md`. Do not replace existing content.

```markdown
## Demo

![Demo](demo/demo.gif)

Try it without a Backlog account:

```bash
cargo run -- --demo
```
```

## Tooling

Install VHS: `brew install vhs`

Pre-build binary: `cargo build --release`

Regenerate GIF: `vhs demo/demo.tape`
