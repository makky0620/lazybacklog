# lazybacklog

A lazygit-inspired TUI for [Nulab's Backlog](https://backlog.com) project management service, written in Rust.

## Features

- Issue list with keyboard navigation
- Issue detail popup
- Assignee-based filtering
- Multiple Backlog spaces with keyboard switching
- API key authentication

## Installation

```bash
cargo build --release
cp target/release/lazybacklog /usr/local/bin/
```

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

Set permissions to protect your API key:

```bash
chmod 600 ~/.config/lazybacklog/config.toml
```

Get your API key from: Backlog → Personal Settings → API → Add API Key

## Key Bindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Open issue detail |
| `Esc` | Close popup |
| `f` | Open assignee filter |
| `r` | Refresh issues |
| `[` / `]` | Switch space |
| `q` | Quit |

## Requirements

- macOS or Linux
- Rust 2021 edition or later
