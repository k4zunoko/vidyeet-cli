# 設計思想

## 目的

vidyeet-cliは、Mux Video APIを利用した動画アップロードCLIツールです。このドキュメントでは、プロジェクトの設計原則と判断の根拠を説明します。

## 設計原則

### 1. Clean Architectureによる責務分離

**なぜこの設計にしたのか:**
- レイヤー間の依存方向を明確にし、ビジネスロジックをインフラや表示から分離
- テスト容易性の向上（モックやスタブで外部依存を置き換え可能）
- 変更の影響範囲を局所化（例: Mux API仕様変更時はインフラ層のみ修正）

**5層構成:**

```
┌─────────────────────────────────────────┐
│  プレゼンテーション層                      │
│  - CLI引数解析                           │
│  - ユーザー入出力（対話/JSON）            │
│  - 進捗表示                              │
│  責務: main.rs, cli.rs, presentation/   │
└────────────┬────────────────────────────┘
             ↓ 依存
┌─────────────────────────────────────────┐
│  アプリケーション層                        │
│  - コマンド実装                           │
│  - ユースケースの組み立て                 │
│  責務: commands/                         │
└─────┬───────────┬──────────┬────────────┘
      ↓           ↓          ↓
┌──────────┐ ┌─────────┐ ┌──────────┐
│ドメイン層 │ │ 設定層   │ │インフラ層 │
│ビジネス   │ │ 静的/動的│ │ 外部通信  │
│ルール     │ │ 設定管理 │ │ HTTP/API │
│domain/    │ │config/  │ │api/      │
└──────────┘ └─────────┘ └──────────┘
```

**依存方向の厳守:**
- 外側から内側への依存のみ許可
- 内側の層は外側の層を知らない
- 例外: `error_severity.rs`は全層で使用される独立モジュール（終了コード決定のみを担当）

### 2. プレゼンテーション層のDTO変換

**課題:**
- ドメイン層の`UploadProgress`をそのままUI表示に使うと、依存方向が逆転する
- 表示ロジックがドメイン層に漏れ出す

**解決策:**
- `presentation/progress.rs`でDTO変換を実施
- `UploadProgress` (ドメイン層) → `DisplayProgress` (プレゼンテーション層)
- 自前トレイト`ToDisplay`で型安全な変換（標準`From`トレイトはオーファンルール違反）
- `Option<DisplayProgress>`で表示抑制を明示的に表現

**実装例:**
```rust
// domain/progress.rs (ドメイン層)
pub enum UploadProgress {
    Validating { file_path: String },
    CreatingUpload { file_name: String },
    Uploading { bytes_sent: u64, total_bytes: u64 },
    // ...
}

// presentation/progress.rs (プレゼンテーション層)
pub struct DisplayProgress {
    pub phase: String,
    pub details: serde_json::Value,
}

pub trait ToDisplay {
    fn to_display(&self, last_update: Option<std::time::Instant>) 
        -> Option<DisplayProgress>;
}

impl ToDisplay for UploadProgress {
    fn to_display(&self, last_update: Option<std::time::Instant>) 
        -> Option<DisplayProgress> {
        // 10秒以内の更新は抑制
        if let Some(last) = last_update {
            if last.elapsed().as_secs() < 10 {
                return None;
            }
        }
        // ドメイン層の型をプレゼンテーション層の型に変換
        Some(DisplayProgress { /* ... */ })
    }
}
```

**なぜこの設計にしたのか:**
- ドメイン層の独立性を保つ（UIの変更がビジネスロジックに影響しない）
- 表示更新頻度の制御をプレゼンテーション層に集約
- 依存逆転の原則（Dependency Inversion Principle）の実践

### 3. マジックナンバーの完全排除

**課題:**
- ハードコードされた定数がコード全体に散在すると保守性が低下
- 設定変更時に複数箇所を修正する必要がある

**解決策:**
- すべての定数を`config/app.rs`の`APP_CONFIG`に集約
- コンパイル時定数として定義（実行時コストゼロ）

**実装例:**
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
    supported_formats: &["mp4", "mov", "avi", "wmv", "flv", "mkv", "webm"],
    chunk_size: 33_554_432, // 32MB
    progress_update_interval_secs: 10,
    file_size_display_precision: 2,
    bytes_per_mb: 1_048_576.0,
    bytes_per_gb: 1_073_741_824.0,
};
```

**なぜこの設計にしたのか:**
- 一元管理により設定変更が容易
- 型安全性（コンパイル時に設定値を検証）
- 実行時コストゼロ（const評価）
- ドキュメント化（設定項目が一箇所に集約）

**例外:**
- ユーザープランに依存する値（動画上限数など）は含めない
  - 理由: プランごとに異なるため、静的定数として定義できない

### 4. エラーハンドリングの階層化

**3つのエラー型:**

```rust
// domain/error.rs
pub enum DomainError {
    FileNotFound(String),
    FileTooLarge { size: u64, max: u64 },
    UnsupportedFormat(String),
}

// config/error.rs
pub enum ConfigError {
    TokenNotFound,
    InvalidConfig(String),
    FileSystemError(String),
}

// api/error.rs
pub enum InfraError {
    NetworkError(String),
    ApiError { status: u16, message: String },
    Timeout,
}
```

**終了コード決定の委譲:**

各エラー型は`severity()`メソッドで終了コードを返す責務を持つ。
プレゼンテーション層（`main.rs`）はエラーチェーンを走査し、
最初に見つかったアプリケーション定義エラーから終了コードを取得。

```rust
// main.rs
fn extract_error_info(error: &anyhow::Error) -> (i32, Option<String>) {
    for cause in error.chain() {
        if let Some(domain_err) = cause.downcast_ref::<DomainError>() {
            return (domain_err.severity().exit_code(), domain_err.hint());
        }
        if let Some(config_err) = cause.downcast_ref::<ConfigError>() {
            return (config_err.severity().exit_code(), config_err.hint());
        }
        if let Some(infra_err) = cause.downcast_ref::<InfraError>() {
            return (infra_err.severity().exit_code(), None);
        }
    }
    (1, None) // デフォルト
}
```

**なぜこの設計にしたのか:**
- 各層が自身のエラー分類責務を持つ（単一責任原則）
- エラー型の追加時にmain.rsの修正が最小限
- エラーチェーンの一度の走査で情報を抽出（効率的）

詳細は [`ERROR_HANDLING.md`](./ERROR_HANDLING.md) を参照。

### 5. CLI出力の明示的制御

**UNIX哲学の実践:**

- **stdout**: 機械可読データのみ（`--machine`フラグ指定時のみ）
- **stderr**: 人間向けメッセージ（進捗、エラー、結果）
- **終了コード**: 成否を示す（0=成功、1/2/3=エラー）

**設計判断:**

TTY検出による自動切り替えではなく、`--machine`フラグで明示的に制御。

```rust
// 悪い例（自動切り替え）
if atty::is(Stream::Stdout) {
    // 人間向け出力
} else {
    // 機械向けJSON出力
}

// 良い例（明示的制御）
if machine_output {
    println!("{}", json); // stdout
} else {
    eprintln!("✓ Success!"); // stderr
}
```

**なぜこの設計にしたのか:**
- **予測可能性**: ユーザーが出力形式を明示的に指定できる
- **テスト容易性**: フラグベースで動作を確認しやすい
- **互換性**: 異なるTTY環境での予期しない動作変更を防ぐ

詳細は [`CLI_CONTRACT.md`](./CLI_CONTRACT.md) を参照。

### 6. 非同期処理の一貫性

**async/awaitの全面採用:**

すべてのコマンドを`async fn`として実装。

```rust
// commands/upload.rs
pub async fn execute(
    file_path: &str, 
    progress_tx: Option<mpsc::Sender<UploadProgress>>
) -> Result<CommandResult>
```

**なぜこの設計にしたのか:**
- ネットワークI/O（Mux API通信）の非ブロッキング実行
- ポーリング処理（Asset作成完了待機）の効率化
- 将来の並列処理拡張に対応（複数動画の同時アップロードなど）

**トレードオフ:**
- 実行ファイルサイズの増加（tokioランタイムを含む）
- 単純なI/O処理でも非同期ランタイムが必要
- **判断**: ネットワークI/Oが主要処理のため、非同期のメリットが上回る

### 7. 設定の二層構造

**静的設定（AppConfig）と動的設定（UserConfig）の分離:**

| 種類 | 格納場所 | 変更 | 用途 |
|------|---------|------|------|
| **AppConfig** | `config/app.rs` (コード内) | コンパイル時 | API endpoint, タイムアウト、サポート形式 |
| **UserConfig** | `~/.config/vidyeet/config.toml` | 実行時 | 認証情報、ユーザー設定 |

**なぜこの設計にしたのか:**
- AppConfig: アプリケーション固有の不変設定（型安全、実行時コストゼロ）
- UserConfig: ユーザーごとに異なる設定（柔軟性、実行時変更可能）

詳細は [`CONFIGURATION.md`](./CONFIGURATION.md) を参照。

## 変更許容性のガイドライン

### 変更してよいもの

1. **プレゼンテーション層の表示形式**
   - 進捗表示のフォーマット
   - エラーメッセージの文言
   - JSON出力の構造（後方互換性を保つ場合）

2. **AppConfig の設定値**
   - タイムアウト時間
   - チャンクサイズ
   - 表示精度

3. **コマンドの追加**
   - 新しいコマンドの実装（既存コマンドに影響しない）

### 変更してはいけないもの（要慎重検討）

1. **依存方向**
   - Clean Architectureの層間依存（外側→内側）
   - 内側の層が外側の層を知ることは禁止

2. **エラー型のseverity()の戻り値**
   - 終了コードの変更はスクリプトの互換性に影響

3. **--machineフラグのJSON構造**
   - 後方互換性を保つ必要がある
   - フィールド追加はOK、削除・型変更はNG

4. **認証情報の保存形式**
   - config.tomlのフォーマット変更は既存ユーザーに影響

## コーディング規約

### 言語使用

| 対象 | 言語 | 理由 |
|------|------|------|
| 実装コード | 英語 | 国際的な保守性、Rust標準 |
| ドキュメントコメント | 日本語 | 開発チームの効率化 |
| ユーザー向け出力 | 英語 | 国際的なユーザー対応 |

### テスト命名規則

```rust
#[test]
fn test_<function_name>_<scenario>() {
    // test_validate_upload_file_when_file_not_found
    // test_extract_error_info_returns_domain_error_code
}
```

### 実装途中の暫定対応

- `#[allow(unused_imports)]`や`#[allow(dead_code)]`は最小限に留める
- 実装完了後は必ず削除

## 使用しているRust機能

### 所有権とライフタイム
- 参照と借用を活用した安全なメモリ管理
- `&str` vs `String` の適切な使い分け
- `Option<T>`による null安全性

### 非同期プログラミング
- tokioランタイムによる`async/await`
- チャネル（`mpsc`）による進捗通知

### エラーハンドリング
- `thiserror`による構造化エラー定義
- `anyhow`によるエラーコンテキストの追加（`.context()`）
- エラーチェーンの走査（`error.chain()`）

### 型システム
- `Result<T, E>`による明示的なエラー処理
- コンパイル時定数（`const fn`）
- `derive`マクロによる自動実装（`Serialize`, `Deserialize`, `Error`）

### トレイトシステム
- 自前トレイト`ToDisplay`による型変換（オーファンルール回避）
- `Option<T>`による明示的なnullable表現

### モジュールシステム
- Clean Architectureに基づいた明確な責務分離
- `pub(crate)`による適切なカプセル化

## パフォーマンス考慮事項

### メモリ効率
- チャンク分割アップロード（32MB単位）により大容量ファイルでもメモリ消費を抑制
- ストリーミング処理（ファイル全体を一度にメモリに載せない）

### ネットワーク効率
- 非同期I/Oによるブロッキング回避
- タイムアウト設定によるハング防止
- チャンクごとの再試行機能（将来の拡張）

### コンパイル時最適化
- `const`評価による実行時コスト削減
- ゼロコスト抽象化（トレイト、ジェネリクス）

## 依存クレートの選定理由

| クレート | 理由 |
|---------|------|
| `anyhow` | エラーハンドリングの簡素化、コンテキスト追加 |
| `thiserror` | 構造化エラー定義の自動化（deriveマクロ） |
| `serde`/`serde_json` | JSON/TOMLシリアライゼーションの標準 |
| `reqwest` | 非同期HTTPクライアントの標準 |
| `tokio` | 非同期ランタイムの標準 |
| `base64` | HTTP Basic認証ヘッダー生成 |
| `chrono` | 日時処理とタイムゾーン変換 |
| `dirs` | プラットフォーム固有パス取得 |

すべて広く採用されている安定したクレートを選定（メンテナンス性重視）。

## 参考資料

- [Clean Architecture by Robert C. Martin](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Command Line Interface Guidelines](https://clig.dev/)
- [UNIX Philosophy](https://en.wikipedia.org/wiki/Unix_philosophy)