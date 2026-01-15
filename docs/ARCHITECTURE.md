# アーキテクチャ

## 概要

vidyeet-cliは**Clean Architecture**に基づいた5層構成を採用しています。このドキュメントでは、各層の責務、データフロー、モジュール構成を説明します。

## レイヤー構造

### 全体図

```
┌─────────────────────────────────────────────────────────────┐
│                    プレゼンテーション層                        │
│  ┌─────────┐  ┌─────────┐  ┌──────────────┐                │
│  │ main.rs │  │ cli.rs  │  │presentation/ │                │
│  └─────────┘  └─────────┘  └──────────────┘                │
│  - CLI引数解析                                               │
│  - ユーザー入出力（対話/JSON）                                │
│  - 進捗表示・DTO変換                                          │
│  - 終了コード決定（error_severity.rs）                        │
└───────────────────────┬─────────────────────────────────────┘
                        ↓ 依存（外→内）
┌─────────────────────────────────────────────────────────────┐
│                    アプリケーション層                          │
│  ┌──────────────────────────────────────┐                   │
│  │         commands/                    │                   │
│  │  login, logout, status, list,        │                   │
│  │  show, delete, upload, help          │                   │
│  └──────────────────────────────────────┘                   │
│  - コマンド実装                                               │
│  - ユースケースの組み立て                                     │
│  - 各層の機能を統合                                           │
└──────────┬────────────┬────────────┬────────────────────────┘
           ↓            ↓            ↓
    ┌───────────┐ ┌──────────┐ ┌──────────┐
    │ドメイン層  │ │  設定層   │ │インフラ層 │
    ├───────────┤ ├──────────┤ ├──────────┤
    │domain/    │ │config/   │ │api/      │
    ├───────────┤ ├──────────┤ ├──────────┤
    │ビジネス    │ │静的設定   │ │HTTP通信  │
    │ルール      │ │動的設定   │ │Mux API  │
    │バリデー    │ │認証情報   │ │認証処理  │
    │ション      │ │管理       │ │          │
    └───────────┘ └──────────┘ └──────────┘
```

### 依存方向の原則

- **外側から内側への依存のみ許可**
- 内側の層は外側の層を知らない
- 例外: `error_severity.rs`は全層で使用される独立モジュール

## 各層の詳細

### 1. プレゼンテーション層

**責務:**
- CLI引数の解析とコマンドディスパッチ
- ユーザー入力の受付（対話的入力、標準入力）
- 実行結果の出力（人間向け/機械向け）
- 進捗情報のDTO変換と表示
- エラーハンドリングと終了コード決定

**モジュール構成:**

```
src/
├── main.rs                 # エントリーポイント、エラーハンドリング
├── cli.rs                  # CLI引数解析、コマンドディスパッチ
├── error_severity.rs       # 終了コード定義（独立モジュール）
└── presentation/
    ├── mod.rs
    ├── input.rs            # ユーザー入力処理
    ├── output.rs           # 結果出力フォーマット
    └── progress.rs         # 進捗DTO変換・表示
```

**主要な型:**

```rust
// presentation/progress.rs
pub struct DisplayProgress {
    pub phase: String,
    pub details: serde_json::Value,
}

pub trait ToDisplay {
    fn to_display(&self, last_update: Option<Instant>) 
        -> Option<DisplayProgress>;
}
```

**データフロー:**

```
ユーザー入力
    ↓
cli::parse_args()
    ↓
commands::*::execute() 呼び出し
    ↓
結果を取得
    ↓
output::output_result() でフォーマット
    ↓
stdout/stderr に出力
```

### 2. アプリケーション層

**責務:**
- 各コマンドのユースケース実装
- ドメイン層、設定層、インフラ層の機能を統合
- ビジネスロジックの組み立て

**モジュール構成:**

```
src/commands/
├── mod.rs
├── result.rs              # 共通の結果型
├── login.rs               # ログインコマンド
├── logout.rs              # ログアウトコマンド
├── status.rs              # ステータス確認コマンド
├── list.rs                # 動画一覧取得コマンド
├── show.rs                # 動画詳細表示コマンド
├── delete.rs              # 動画削除コマンド
├── upload.rs              # 動画アップロードコマンド
└── help.rs                # ヘルプ表示コマンド
```

**共通インターフェース:**

すべてのコマンドは以下のシグネチャを持つ：

```rust
pub async fn execute(...) -> Result<CommandResult>
```

**CommandResult構造:**

```rust
// commands/result.rs
pub enum CommandResult {
    Login { was_logged_in: bool },
    Logout { was_logged_in: bool },
    Status { is_authenticated: bool, token_id: Option<String> },
    List { videos: Vec<VideoInfo> },
    Show { video: VideoDetails },
    Delete { asset_id: String },
    Upload { asset_id: String, playback_id: String, /* ... */ },
    Help,
}
```

**コマンド実装例（upload）:**

```rust
pub async fn execute(
    file_path: &str,
    progress_tx: Option<mpsc::Sender<UploadProgress>>,
) -> Result<CommandResult> {
    // 1. ファイルバリデーション（ドメイン層）
    validator::validate_upload_file(file_path)?;
    
    // 2. 認証情報読込（設定層）
    let config = UserConfig::load()?;
    let auth = config.auth.context("Not logged in")?;
    
    // 3. API クライアント初期化（インフラ層）
    let client = MuxClient::new(auth.token_id, auth.token_secret)?;
    
    // 4. Direct Upload 作成
    let upload = client.create_direct_upload().await?;
    
    // 5. ファイルアップロード
    client.upload_file(file_path, &upload.url, progress_tx).await?;
    
    // 6. Asset 取得
    let asset = client.wait_for_asset(&upload.id).await?;
    
    // 7. 結果返却
    Ok(CommandResult::Upload { /* ... */ })
}
```

### 3. ドメイン層

**責務:**
- ビジネスルールの定義
- ファイルバリデーション
- 進捗イベントの定義
- ドメインエラーの定義

**モジュール構成:**

```
src/domain/
├── mod.rs
├── validator.rs           # ファイルバリデーションロジック
├── progress.rs            # 進捗イベント定義
├── formatter.rs           # ドメインオブジェクトのフォーマット
└── error.rs               # ドメインエラー定義
```

**主要な型:**

```rust
// domain/progress.rs
pub enum UploadProgress {
    Validating { file_path: String },
    CreatingUpload { file_name: String, size_bytes: u64, format: String },
    Uploading { file_name: String, current_chunk: u32, total_chunks: u32, 
                bytes_sent: u64, total_bytes: u64 },
    WaitingForAsset { upload_id: String, elapsed_secs: u64 },
    Completed { asset_id: String },
}

// domain/validator.rs
pub fn validate_upload_file(path: &str) -> Result<FileInfo> {
    // ファイル存在チェック
    // サイズ上限チェック
    // フォーマットチェック
}

// domain/error.rs
pub enum DomainError {
    FileNotFound(String),
    FileTooLarge { size: u64, max: u64 },
    UnsupportedFormat(String),
}
```

**ビジネスルール:**

1. **ファイルサイズ制限**: 10GB以下
2. **サポート形式**: mp4, mov, avi, wmv, flv, mkv, webm
3. **進捗更新頻度**: 10秒以上の間隔（プレゼンテーション層で制御）

### 4. 設定層

**責務:**
- 静的設定の管理（AppConfig）
- 動的設定の管理（UserConfig）
- 認証情報の保存・読込
- 設定ファイルの検証

**モジュール構成:**

```
src/config/
├── mod.rs
├── app.rs                 # 静的設定（コンパイル時定数）
├── user.rs                # 動的設定（実行時TOML）
└── error.rs               # 設定エラー定義
```

**AppConfig（静的設定）:**

```rust
// config/app.rs
pub struct AppConfig {
    // API設定
    pub endpoint: &'static str,
    pub timeout_seconds: u64,
    
    // アップロード設定
    pub max_file_size: u64,
    pub supported_formats: &'static [&'static str],
    pub chunk_size: usize,
    
    // プレゼンテーション設定
    pub progress_update_interval_secs: u64,
    pub file_size_display_precision: usize,
    
    // 単位変換定数
    pub bytes_per_mb: f64,
    pub bytes_per_gb: f64,
}

pub const APP_CONFIG: AppConfig = AppConfig {
    endpoint: "https://api.mux.com",
    timeout_seconds: 300,
    max_file_size: 10_737_418_240, // 10GB
    // ...
};
```

**UserConfig（動的設定）:**

```rust
// config/user.rs
#[derive(Serialize, Deserialize)]
pub struct UserConfig {
    pub default_title: Option<String>,
    pub auto_copy_url: bool,
    pub show_notification: bool,
    pub auth: Option<AuthConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthConfig {
    pub token_id: String,
    pub token_secret: String,
}

impl UserConfig {
    pub fn load() -> Result<Self>;
    pub fn save(&self) -> Result<()>;
    pub fn config_path() -> PathBuf;
}
```

**設定ファイルパス:**

| OS | パス |
|----|------|
| Windows | `%APPDATA%\vidyeet\config.toml` |
| macOS | `~/Library/Application Support/vidyeet/config.toml` |
| Linux | `~/.config/vidyeet/config.toml` |

詳細は [`CONFIGURATION.md`](./CONFIGURATION.md) を参照。

### 5. インフラ層

**責務:**
- Mux API との HTTP 通信
- HTTP Basic 認証処理
- API レスポンスの型変換
- ネットワークエラーハンドリング

**モジュール構成:**

```
src/api/
├── mod.rs
├── client.rs              # Mux API クライアント
├── auth.rs                # HTTP Basic 認証
├── types.rs               # API レスポンス型定義
└── error.rs               # インフラエラー定義
```

**主要な型:**

```rust
// api/client.rs
pub struct MuxClient {
    client: reqwest::Client,
    base_url: String,
    token_id: String,
    token_secret: String,
}

impl MuxClient {
    pub async fn create_direct_upload(&self) -> Result<DirectUpload>;
    pub async fn upload_file(&self, path: &str, url: &str, 
                             progress_tx: Option<mpsc::Sender<UploadProgress>>) 
                             -> Result<()>;
    pub async fn get_upload_status(&self, upload_id: &str) 
                                   -> Result<UploadStatus>;
    pub async fn get_asset(&self, asset_id: &str) -> Result<AssetData>;
    pub async fn list_assets(&self) -> Result<Vec<AssetData>>;
    pub async fn delete_asset(&self, asset_id: &str) -> Result<()>;
}

// api/types.rs
#[derive(Deserialize)]
pub struct DirectUpload {
    pub id: String,
    pub url: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct AssetData {
    pub id: String,
    pub status: String,
    pub playback_ids: Vec<PlaybackId>,
    pub duration: Option<f64>,
    pub created_at: String,
    // ...
}
```

**認証処理:**

```rust
// api/auth.rs
pub fn create_basic_auth_header(token_id: &str, token_secret: &str) 
    -> String {
    let credentials = format!("{}:{}", token_id, token_secret);
    let encoded = base64::encode(credentials);
    format!("Basic {}", encoded)
}
```

## データフロー

### アップロード処理の完全フロー

```
[ユーザー] vidyeet upload video.mp4
    ↓
[プレゼンテーション層] cli.rs
    - 引数解析
    - progress チャネル作成
    ↓
[アプリケーション層] commands/upload.rs
    ↓
    ├─→ [ドメイン層] domain/validator.rs
    │    - ファイル存在確認
    │    - サイズチェック (10GB以下)
    │    - 拡張子チェック (mp4, mov, etc.)
    │    ↓ Result<FileInfo>
    │
    ├─→ [設定層] config/user.rs
    │    - config.toml 読込
    │    - 認証情報取得
    │    ↓ Result<AuthConfig>
    │
    ├─→ [インフラ層] api/client.rs
    │    ├─→ create_direct_upload()
    │    │    - POST /video/v1/uploads
    │    │    - HTTP Basic認証ヘッダー付与
    │    │    ↓ DirectUpload { id, url }
    │    │
    │    ├─→ upload_file()
    │    │    - ファイルを32MBチャンクに分割
    │    │    - 各チャンクをPUT (Content-Range付き)
    │    │    - 進捗を progress_tx に送信
    │    │    ↓ Result<()>
    │    │
    │    └─→ wait_for_asset()
    │         - GET /video/v1/uploads/{UPLOAD_ID}
    │         - ポーリング（2秒間隔、最大300秒）
    │         - status == "asset_created" まで待機
    │         ↓ AssetData
    │
    ↓ CommandResult::Upload
    
[プレゼンテーション層] presentation/output.rs
    - 人間向け: stderr に整形出力
    - 機械向け: stdout に JSON出力
    ↓
[ユーザー] 結果表示
```

### 進捗通知のデータフロー

```
[アップロード処理]
    ↓ progress_tx.send(UploadProgress::Uploading { ... })
    
[プレゼンテーション層] presentation/progress.rs
    ├─→ progress_rx.recv()
    │    ↓ UploadProgress (ドメイン層の型)
    │
    ├─→ ToDisplay::to_display()
    │    - 10秒以内の更新は抑制
    │    - DisplayProgress に変換
    │    ↓ Option<DisplayProgress>
    │
    └─→ 出力
         - --machine: JSON を stdout
         - 通常: 進捗バーを stderr
```

### エラーハンドリングのデータフロー

```
[エラー発生]
    ↓
DomainError / ConfigError / InfraError
    ↓ anyhow::Context で詳細追加
    
[アプリケーション層] commands/*.rs
    - .context("Command failed")
    ↓ anyhow::Error
    
[プレゼンテーション層] main.rs
    ├─→ handle_error()
    │    ├─→ extract_error_info()
    │    │    - error.chain() を走査
    │    │    - 最初の定義エラーを発見
    │    │    - severity() で終了コード取得
    │    │    - hint() でヒント取得
    │    │    ↓ (exit_code, hint)
    │    │
    │    └─→ 出力
    │         - --machine: JSON エラー
    │         - 通常: 人間向けエラー
    │
    └─→ std::process::exit(exit_code)
```

## モジュール間通信

### 同期的な通信

```rust
// アプリケーション層 → ドメイン層
let file_info = validator::validate_upload_file(path)?;

// アプリケーション層 → 設定層
let config = UserConfig::load()?;

// アプリケーション層 → インフラ層
let assets = client.list_assets().await?;
```

### 非同期的な通信（進捗通知）

```rust
// インフラ層 → プレゼンテーション層
if let Some(tx) = &progress_tx {
    tx.send(UploadProgress::Uploading { /* ... */ })
      .await
      .ok();
}
```

## ファイルシステム構造

```
vidyeet-cli/
├── Cargo.toml
├── README.md
├── AGENTS.md                    # プロジェクト索引
├── MACHINE_API.md               # 機械可読API仕様
│
├── docs/                        # 詳細ドキュメント
│   ├── DESIGN_PHILOSOPHY.md
│   ├── ARCHITECTURE.md          # このファイル
│   ├── ERROR_HANDLING.md
│   ├── CLI_CONTRACT.md
│   ├── CONFIGURATION.md
│   ├── TESTING_STRATEGY.md
│   ├── API_VIDEO_SPECIFICATION.md
│   ├── API_UPLOAD_PROGRESS.md
│   └── AUTHENTICATION_DESIGN.md
│
└── src/
    ├── main.rs                  # エントリーポイント
    ├── cli.rs                   # CLI解析
    ├── error_severity.rs        # 終了コード定義
    │
    ├── presentation/            # プレゼンテーション層
    │   ├── mod.rs
    │   ├── input.rs
    │   ├── output.rs
    │   └── progress.rs
    │
    ├── commands/                # アプリケーション層
    │   ├── mod.rs
    │   ├── result.rs
    │   ├── login.rs
    │   ├── logout.rs
    │   ├── status.rs
    │   ├── list.rs
    │   ├── show.rs
    │   ├── delete.rs
    │   ├── upload.rs
    │   └── help.rs
    │
    ├── domain/                  # ドメイン層
    │   ├── mod.rs
    │   ├── validator.rs
    │   ├── progress.rs
    │   ├── formatter.rs
    │   └── error.rs
    │
    ├── config/                  # 設定層
    │   ├── mod.rs
    │   ├── app.rs
    │   ├── user.rs
    │   └── error.rs
    │
    └── api/                     # インフラ層
        ├── mod.rs
        ├── client.rs
        ├── auth.rs
        ├── types.rs
        └── error.rs
```

## パフォーマンス最適化

### メモリ効率

1. **チャンク分割アップロード**
   - ファイルを32MBチャンクに分割
   - メモリに一度に載せるのは1チャンクのみ

2. **ストリーミング処理**
   - `std::fs::File` + `std::io::Read` で順次読込
   - 大容量ファイルでもメモリ消費を抑制

### ネットワーク効率

1. **非同期I/O**
   - `tokio` + `reqwest` で非ブロッキング
   - ポーリング中も他の処理が可能

2. **タイムアウト設定**
   - 接続タイムアウト: 30秒
   - リクエストタイムアウト: 300秒

### コンパイル時最適化

1. **const評価**
   - `APP_CONFIG` の全フィールド
   - 実行時コストゼロ

2. **ゼロコスト抽象化**
   - トレイト境界のインライン化
   - ジェネリクスのモノモーフィズム

## 拡張性

### 新しいコマンドの追加

1. `src/commands/` に新しいファイルを作成
2. `pub async fn execute(...) -> Result<CommandResult>` を実装
3. `CommandResult` に新しいバリアントを追加
4. `cli.rs` にコマンドマッチを追加

### 新しいAPI機能の追加

1. `api/types.rs` にレスポンス型を定義
2. `api/client.rs` にメソッドを追加
3. 必要に応じてコマンド層で利用

### 新しいエラー型の追加

1. 該当する層（domain/config/api）の `error.rs` に追加
2. `severity()` と `hint()` を実装
3. `main.rs` の `extract_error_info()` は自動的に対応

## 参考資料

- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Domain-Driven Design](https://martinfowler.com/bliki/DomainDrivenDesign.html)