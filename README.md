# lazybacklog

A [lazygit](https://github.com/jesseduffield/lazygit)-inspired Terminal UI (TUI) for [Nulab's Backlog](https://backlog.com) project management service, written in Rust.

[日本語版 README](README.ja.md)

## Demo

![Demo](demo/demo.gif)

Try it without a Backlog account:

```bash
cargo run -- --demo
```

## Features

- Browse issues with keyboard navigation
- Issue detail popup with full description
- Filter by assignee or status
- Search issues, assignees, and statuses with `/`
- Project selection per space
- Multiple Backlog spaces with `[` / `]` switching
- API key authentication

## Installation

### From source

```bash
git clone https://github.com/makky0620/lazybacklog.git
cd lazybacklog
cargo build --release
cp target/release/lazybacklog /usr/local/bin/
```

### Requirements

- macOS or Linux
- Rust 2021 edition or later (`rustup` recommended)

## Configuration

Create `~/.config/lazybacklog/config.toml`:

```toml
default_space = "myspace"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "YOUR_API_KEY"

[[spaces]]
name = "work"
host = "work.backlog.jp"
api_key = "YOUR_API_KEY"
```

Protect your API key:

```bash
chmod 600 ~/.config/lazybacklog/config.toml
```

Get your API key: Backlog → Personal Settings → API → Add API Key

## Key Bindings

### Issue List

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Open issue detail |
| `f` | Open assignee filter |
| `s` | Open status filter |
| `/` | Start search |
| `n` / `N` | Next / previous search match |
| `r` | Refresh issues |
| `[` / `]` | Switch space |
| `q` | Quit |

### Popups (Filter / Status Filter)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` | Toggle selection (status filter) |
| `Enter` | Apply filter |
| `/` | Search within popup |
| `Esc` | Cancel |

### Issue Detail

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Esc` | Close |

## License

MIT — see [LICENSE](LICENSE)
