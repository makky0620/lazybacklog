# lazybacklog Design Spec

**Date:** 2026-03-14
**Status:** Approved

---

## Overview

`lazybacklog` is a Rust-based TUI (Terminal User Interface) application for Nulab's Backlog project management service, inspired by the lazygit user experience. The initial version focuses on issue browsing with assignee filtering across multiple Backlog spaces.

---

## Goals & Scope (v1)

**In scope:**
- Issue list view with real-time navigation
- Issue detail popup
- Assignee-based filtering
- Multiple Backlog space support with keyboard switching
- API key authentication
- Config file at `~/.config/lazybacklog/config.toml`

**Out of scope (future versions):**
- Issue creation / editing
- Comment posting
- Status changes
- Wiki browsing
- OAuth 2.0 authentication

---

## Architecture

### Approach

**ratatui + tokio + reqwest with mpsc channels**

The UI thread runs the ratatui event loop. API calls are dispatched as tokio tasks and communicate results back via `mpsc` channels. This keeps the UI responsive during network requests.

```
キー入力
  ↓
app.rs (状態更新・イベント処理)
  ↓ tokio::spawn
api/client.rs (非同期API呼び出し)
  ↓ mpsc::Sender
app.rs (状態更新)
  ↓
ratatui (再描画)
```

### Project Structure

```
lazybacklog/
├── src/
│   ├── main.rs           # エントリーポイント、ターミナル初期化・クリーンアップ
│   ├── app.rs            # アプリ状態管理、メインイベントループ
│   ├── event.rs          # イベント型定義（キー入力・APIレスポンス・エラー）
│   ├── config.rs         # 設定ファイル読み込み・バリデーション
│   ├── api/
│   │   ├── mod.rs
│   │   ├── client.rs     # Backlog APIクライアント（reqwest）
│   │   └── models.rs     # APIレスポンス型（serde）
│   └── ui/
│       ├── mod.rs         # メインrender関数、レイアウト組み立て
│       ├── issue_list.rs  # 課題一覧ウィジェット
│       ├── issue_detail.rs # 詳細ポップアップウィジェット
│       └── filter.rs      # Assigneeフィルターポップアップ
├── Cargo.toml
└── docs/
    └── superpowers/
        └── specs/
            └── 2026-03-14-lazybacklog-design.md
```

### Key Crates

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI描画フレームワーク |
| `crossterm` | クロスプラットフォームターミナル操作 |
| `tokio` | 非同期ランタイム |
| `reqwest` | HTTP APIクライアント |
| `serde` / `serde_json` | JSONシリアライズ・デシリアライズ |
| `toml` | 設定ファイルパース |
| `anyhow` | エラーハンドリング |

---

## UI/UX Design

### Main Screen (Issue List)

```
┌─ lazybacklog ──────────────── [myspace] ──────────────────────┐
│ Project: ALL  Assignee: 田中太郎  Status: In Progress          │
├───────────────────────────────────────────────────────────────┤
│ #  │ Key      │ Summary              │ Assignee  │ Status      │
│────┼──────────┼──────────────────────┼───────────┼─────────────│
│ ▶  │ PROJ-123 │ ログイン画面の修正   │ 田中太郎  │ 処理中      │
│    │ PROJ-124 │ APIエラーハンドリング │ 山田花子  │ 未対応      │
│    │ PROJ-125 │ テスト追加           │ 田中太郎  │ 完了        │
├───────────────────────────────────────────────────────────────┤
│ [j/k] 移動  [Enter] 詳細  [f] フィルター  [r] 更新  [q] 終了  │
└───────────────────────────────────────────────────────────────┘
```

### Issue Detail Popup (Enter)

```
┌─ PROJ-123 ────────────────────────────────────┐
│ ログイン画面の修正                              │
│                                                │
│ 担当者: 田中太郎     優先度: 高                │
│ ステータス: 処理中   種別: バグ                │
│ 期限: 2026-03-20                               │
│                                                │
│ 詳細:                                          │
│ ログイン時にXXXのエラーが発生する。            │
│ ...                                            │
│                                               ↕│
│                              [Esc] 閉じる      │
└────────────────────────────────────────────────┘
```

### Key Bindings

| Key | Action |
|-----|--------|
| `j` / `↓` | 下へ移動 |
| `k` / `↑` | 上へ移動 |
| `Enter` | 詳細ポップアップを開く |
| `Esc` | ポップアップを閉じる |
| `f` | Assigneeフィルターを開く |
| `r` | 課題一覧をリフレッシュ |
| `[` / `]` | スペースを切り替え |
| `q` | アプリ終了 |

---

## Configuration

**Path:** `~/.config/lazybacklog/config.toml`

```toml
default_space = "myspace"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "xxxxxxxxxxxxxxxx"

[[spaces]]
name = "work"
host = "work.backlog.jp"
api_key = "yyyyyyyyyyyyyyyy"
```

---

## Backlog API Integration

**Base URL:** `https://{host}/api/v2`
**Authentication:** クエリパラメータ `?apiKey={api_key}`

### Endpoints Used

| Purpose | Endpoint |
|---------|----------|
| 課題一覧 | `GET /api/v2/issues` |
| 課題詳細 | `GET /api/v2/issues/:issueIdOrKey` |
| プロジェクト一覧 | `GET /api/v2/projects` |
| メンバー一覧 | `GET /api/v2/projects/:projectIdOrKey/users` |

### Filter Parameters (Issues)

- `assigneeId[]` — Assignee絞り込み
- `statusId[]` — ステータス絞り込み
- `projectId[]` — プロジェクト絞り込み
- `offset` / `count` — ページネーション（最大100件/回）

### Caching Strategy

- プロジェクト一覧・メンバー一覧: 起動時に1回取得してメモリキャッシュ
- 課題一覧: `r`キーで手動リフレッシュ（自動更新なし）

---

## Error Handling

| Error | Behavior |
|-------|----------|
| 設定ファイル未存在 | 起動時にメッセージ表示して終了 |
| API接続失敗・タイムアウト | ステータスバーにエラー表示、アプリ継続 |
| 401 Unauthorized | スペース名とともにエラー表示 |
| その他APIエラー | ステータスバーにHTTPステータスコードとメッセージ表示 |

エラーはアプリをクラッシュさせず、ステータスバーに表示する。

```
│ ⚠ [myspace] API error: 401 Unauthorized        │
```

---

## Testing Strategy

| Layer | Approach |
|-------|----------|
| `config.rs` | ユニットテスト（tomlパース・バリデーション） |
| `api/client.rs` | `wiremock`クレートによるモックサーバーとのインテグレーションテスト |
| `app.rs` | 状態遷移のユニットテスト（UIなし） |
| UIレイヤー | テスト対象外（過剰なため） |

---

## Future Considerations (v2+)

- 課題ステータス変更
- コメント追加
- 課題作成・編集
- OAuth 2.0対応
- キーバインドのカスタマイズ
