# lazybacklog

[lazygit](https://github.com/jesseduffield/lazygit) にインスパイアされた、[Nulab の Backlog](https://backlog.com) 向け Terminal UI (TUI) アプリケーションです。Rust 製。

[English README](README.md)

## デモ

![Demo](demo/demo.gif)

Backlog アカウントなしで試せます:

```bash
cargo run -- --demo
```

## 機能

- キーボードナビゲーションで課題を閲覧
- 課題詳細ポップアップ（説明文フル表示）
- 担当者・ステータスでフィルタリング
- `/` で課題・担当者・ステータスをインクリメンタル検索
- スペースごとのプロジェクト選択
- `[` / `]` で複数 Backlog スペースを切り替え
- API キー認証

## インストール

### ソースからビルド

```bash
git clone https://github.com/makky0620/lazybacklog.git
cd lazybacklog
cargo build --release
cp target/release/lazybacklog /usr/local/bin/
```

### 動作環境

- macOS または Linux
- Rust 2021 edition 以降（`rustup` 推奨）

## 設定

`~/.config/lazybacklog/config.toml` を作成してください:

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

API キーを保護するため、パーミッションを設定してください:

```bash
chmod 600 ~/.config/lazybacklog/config.toml
```

API キーの取得: Backlog → 個人設定 → API → APIキーの登録

## キーバインド

### 課題一覧

| キー | 操作 |
|------|------|
| `j` / `↓` | 下へ移動 |
| `k` / `↑` | 上へ移動 |
| `Enter` | 課題詳細を開く |
| `f` | 担当者フィルターを開く |
| `s` | ステータスフィルターを開く |
| `/` | 検索開始 |
| `n` / `N` | 次 / 前の検索結果へ |
| `r` | 課題一覧を更新 |
| `[` / `]` | スペースを切り替え |
| `q` | 終了 |

### ポップアップ（フィルター / ステータスフィルター）

| キー | 操作 |
|------|------|
| `j` / `↓` | 下へ移動 |
| `k` / `↑` | 上へ移動 |
| `Space` | 選択切り替え（ステータスフィルター） |
| `Enter` | フィルターを適用 |
| `/` | ポップアップ内検索 |
| `Esc` | キャンセル |

### 課題詳細

| キー | 操作 |
|------|------|
| `j` / `↓` | 下にスクロール |
| `k` / `↑` | 上にスクロール |
| `Esc` | 閉じる |

## ライセンス

MIT — [LICENSE](LICENSE) を参照
