# lazybacklog Design Spec

**Date:** 2026-03-14
**Status:** Approved

---

## Overview

`lazybacklog` is a Rust-based TUI (Terminal User Interface) application for Nulab's Backlog project management service, inspired by the lazygit user experience. The initial version focuses on issue browsing with assignee filtering across multiple Backlog spaces.

**Target platforms:** macOS and Linux only. Windows is out of scope for v1.

---

## Goals & Scope (v1)

**In scope:**
- Issue list view with navigation
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

The main thread runs a `tokio` event loop using `tokio::select!` to simultaneously wait on:
1. **キー入力:** `crossterm::event::poll` を別スレッド（`std::thread::spawn`）で実行し、キーイベントを `mpsc::Sender<AppEvent>` に送信
2. **APIレスポンス:** 非同期タスク（`tokio::spawn`）が結果を同じ `mpsc::Sender<AppEvent>` に送信

メインループは `mpsc::Receiver<AppEvent>` から `AppEvent` を受信して状態を更新し、ratatuiで再描画する。

```
[別スレッド] crossterm::event::read() → mpsc::Sender<AppEvent::Key>
                                                          ↓
[メインスレッド tokio] mpsc::Receiver<AppEvent> → app.rs(状態更新) → ratatui(再描画)
                                                          ↑
[tokio::spawn] api/client.rs(非同期API) → mpsc::Sender<AppEvent::IssuesLoaded等>
```

### Event Types (`event.rs`)

There is **one shared `mpsc` channel** (Sender cloned per task). All events flow through `AppEvent`:

```rust
pub enum AppEvent {
    // キー入力（キー読み取りスレッドから送信）
    Key(crossterm::event::KeyEvent),
    // API results（tokio::spawnタスクから送信）
    IssuesLoaded { space: String, issues: Vec<Issue> },
    IssueDetailLoaded(Issue),
    SpaceUsersLoaded { space: String, users: Vec<User> },
    // エラー
    ApiError { space: String, message: String },
}
```

**注:** `GET /projects/:id/users` の結果はプロジェクトをまたいで集約し、スペース全体のユーザーリストとして `SpaceUsersLoaded` に格納する。Backlogのユーザーはスペース内で重複するため、`user.id` で重複排除してからキャッシュする。

### Project Structure

```
lazybacklog/
├── src/
│   ├── main.rs           # エントリーポイント、ターミナル初期化・クリーンアップ
│   ├── app.rs            # アプリ状態管理、メインイベントループ、フィルター状態
│   ├── event.rs          # AppEvent enum定義
│   ├── config.rs         # 設定ファイル読み込み・バリデーション・パーミッション確認
│   ├── api/
│   │   ├── mod.rs
│   │   ├── client.rs     # Backlog APIクライアント（reqwest）
│   │   └── models.rs     # APIレスポンス型（serde）
│   └── ui/
│       ├── mod.rs         # メインrender関数、レイアウト組み立て
│       ├── issue_list.rs  # 課題一覧ウィジェット描画
│       ├── issue_detail.rs # 詳細ポップアップ描画
│       └── filter.rs      # Assigneeフィルターポップアップ描画
├── Cargo.toml
└── docs/
    └── superpowers/
        └── specs/
            └── 2026-03-14-lazybacklog-design.md
```

**責務の境界:**
- `ui/` 以下のファイルはすべて **描画のみ**。状態を持たない純粋な描画関数。
- フィルター選択状態（現在選択中のAssignee等）は `app.rs` の `AppState` が持つ。
- `ui/filter.rs` はフィルターポップアップのウィジェット描画のみ担当。

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
| `[` / `]` | スペースを切り替え（自動フェッチあり） |
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

**セキュリティ:** 設定ファイルはAPIキーを平文で保存するため、`config.rs` 読み込み時にファイルパーミッションを確認し、`0600` 以外の場合は警告を表示する。初回生成時は `0600` で作成する。

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

### Pagination

Backlog APIの `/issues` は最大100件/回（`count`パラメータ）。

**v1の方針: 最大100件を上限とし、一覧の末尾に「(表示: 100件 / 上限)" と件数を表示する。** ページネーションは v2 以降で実装。

### Startup Flow (メンバーキャッシュ)

起動時に以下の順序で非同期フェッチを行う（全スペース並列）:

1. `GET /projects` でプロジェクト一覧取得
2. 各プロジェクトに対して `GET /projects/:id/users` を並列実行（N+1リクエスト）
3. 全ユーザーをスペースごとにメモリキャッシュ

**タイムアウト:** 各リクエストは10秒でタイムアウト。ユーザーフェッチが失敗した場合はAssigneeフィルターを「取得失敗」として表示し、課題一覧の表示は妨げない。起動時のローディング状態はステータスバーに表示する。

### Space Switching

`[` / `]` でスペースを切り替えると以下の挙動をとる:

- **初回切り替え:** そのスペースの課題一覧をフェッチ。ローディング表示。
- **2回目以降:** キャッシュ済みの課題一覧を即座に表示（再フェッチしない）。
- **強制リフレッシュ:** `r` キーで現在のスペースの課題一覧を再フェッチ。

### Caching Strategy

| データ | キャッシュ戦略 |
|--------|--------------|
| ユーザー一覧（スペース別） | 起動時に取得、以降キャッシュ（`r`でもリフレッシュしない） |
| 課題一覧（スペース別） | 初回スペース表示時に取得、`r`で手動リフレッシュ |
| 課題詳細 | キャッシュなし（毎回フェッチ） |

---

## Error Handling

| Error | Behavior |
|-------|----------|
| 設定ファイル未存在 | 起動時にメッセージ表示して終了 |
| 設定ファイルのパーミッション不正 | 警告を表示して続行 |
| API接続失敗・タイムアウト | ステータスバーにエラー表示、アプリ継続 |
| 401 Unauthorized | スペース名とともにエラー表示 |
| 起動時ユーザーフェッチ失敗 | フィルターを「取得失敗」表示、課題表示は続行 |

エラーはアプリをクラッシュさせず、ステータスバーに表示する。

```
│ ⚠ [myspace] API error: 401 Unauthorized        │
```

---

## Testing Strategy

| Layer | Approach |
|-------|----------|
| `config.rs` | ユニットテスト（tomlパース・バリデーション・パーミッション確認） |
| `api/client.rs` | `wiremock` クレートによるモックサーバーとのインテグレーションテスト（`#[tokio::test]` 使用） |
| `app.rs` | 状態遷移のユニットテスト（UIなし、`AppEvent` を直接注入） |
| UIレイヤー | テスト対象外 |

**非同期テストの注意:** `api/client.rs` のテストはすべて `#[tokio::test]` アトリビュートを使用する。`reqwest` は非同期クライアントのため同期テストでは動作しない。

---

## Future Considerations (v2+)

- 課題ページネーション
- 課題ステータス変更
- コメント追加
- 課題作成・編集
- OAuth 2.0対応
- キーバインドのカスタマイズ
