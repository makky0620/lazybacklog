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

**Terminal settings:**
- Output: `demo/demo.gif`
- Width: 220 columns, Height: 50 rows
- Font size: 14, Theme: Dracula (or similar dark theme)
- Framerate: 24

**Operation sequence:**

| Step | Action | VHS directive |
|------|--------|---------------|
| Launch app | `cargo run -- --demo` | `Type`, `Enter` |
| Wait for project select | — | `Sleep 3s` |
| Select project | `Enter` | `Enter`, `Sleep 1s` |
| Browse issues | `j` × 3 | `Down` × 3, `Sleep 500ms` each |
| Open issue detail | `Enter` | `Enter`, `Sleep 500ms` |
| Close detail | `Esc` | `Escape`, `Sleep 500ms` |
| Start search | `/` | `Type /` |
| Type query | e.g. `login` | `Type "login"`, `Sleep 500ms` |
| Show match | — | `Sleep 1s` |
| Quit | `q` | `Ctrl+C` or `Type q` |

## README Integration

Add to `README.md` and `README.ja.md` under the existing `## Demo` section:

```markdown
## Demo

![Demo](demo/demo.gif)

Try it without a Backlog account:
...
```

## Tooling

Install VHS: `brew install vhs`

Regenerate GIF: `vhs demo/demo.tape`
