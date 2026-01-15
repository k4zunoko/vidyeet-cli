# 設定管理

## 概要

vidyeet-cliは、**静的設定（AppConfig）**と**動的設定（UserConfig）**の二層構造による設定管理を採用しています。このドキュメントでは、両者の責務、実装方法、設定ファイルの仕様を説明します。

## 設定の二層構造

### 設計原則

| 種類 | 格納場所 | 変更タイミング | 用途 |
|------|---------|--------------|------|
| **AppConfig** | `config/app.rs` (コード内) | コンパイル時 | アプリケーション固有の不変設定 |
| **UserConfig** | `~/.config/vidyeet/config.toml` | 実行時 | ユーザーごとに異なる設定 |

**なぜこの設計にしたのか:**

1. **AppConfig（静的設定）**
   - API endpoint、タイムアウト値など、アプリケーション全体で共通の設定
   - コンパイル時に確定し、実行時コストゼロ
   - 型安全性（コンパイラが検証）

2. **UserConfig（動的設定）**
   - 認証情報、ユーザー設定など、実行時に変更可能な設定
   - ファイルシステムに永続化
   - ユーザーごと・環境ごとに異なる値

## AppConfig（静的設定）

### 実装

```rust
// src/config/app.rs
pub struct AppConfig {
    // API設定
    pub endpoint: &'static str,
    pub timeout_seconds: u64,
    
    // アップロード設定
    pub max_file_size: u64,
    pub supported_formats: &'static [&'static str],
    pub chunk_size: usize,
    pub polling_interval_secs: u64,
    pub polling_max_attempts: u32,
    
    // プレゼンテーション設定
    pub progress_update_interval_secs: u64,
    pub file_size_display_precision: usize,
    pub token_display_mask_length: usize,
    
    // 単位変換定数
    pub bytes_per_kb: f64,
    pub bytes_per_mb: f64,
    pub bytes_per_gb: f64,
}

pub const APP_CONFIG: AppConfig = AppConfig {
    // API設定
    endpoint: "https://api.mux.com",
    timeout_seconds: 300,
    
    // アップロード設定
    max_file_size: 10_737_418_240, // 10GB
    supported_formats: &["mp4", "mov", "avi", "wmv", "flv", "mkv", "webm"],
    chunk_size: 33_554_432, // 32MB (256KiB * 128)
    polling_interval_secs: 2,
    polling_max_attempts: 150, // 300秒 / 2秒
    
    // プレゼンテーション設定
    progress_update_interval_secs: 10,
    file_size_display_precision: 2,
    token_display_mask_length: 3,
    
    // 単位変換定数
    bytes_per_kb: 1_024.0,
    bytes_per_mb: 1_048_576.0,
    bytes_per_gb: 1_073_741_824.0,
};
```

### 設定項目の詳細

#### API設定

| 項目 | 値 | 説明 |
|------|-----|------|
| `endpoint` | `"https://api.mux.com"` | Mux API のベースURL |
| `timeout_seconds` | `300` | HTTPリクエストのタイムアウト（5分） |

#### アップロード設定

| 項目 | 値 | 説明 |
|------|-----|------|
| `max_file_size` | `10_737_418_240` | 最大ファイルサイズ（10GB） |
| `supported_formats` | `["mp4", "mov", ...]` | サポートする動画形式 |
| `chunk_size` | `33_554_432` | チャンクサイズ（32MB）<br>※ 256KiBの倍数（Mux推奨） |
| `polling_interval_secs` | `2` | Asset作成完了確認の間隔（2秒） |
| `polling_max_attempts` | `150` | ポーリング最大試行回数（300秒相当） |

#### プレゼンテーション設定

| 項目 | 値 | 説明 |
|------|-----|------|
| `progress_update_interval_secs` | `10` | 進捗更新の最小間隔（10秒） |
| `file_size_display_precision` | `2` | ファイルサイズ表示の小数点以下桁数 |
| `token_display_mask_length` | `3` | Token IDマスキング時の表示文字数（前後3文字） |

#### 単位変換定数

| 項目 | 値 | 説明 |
|------|-----|------|
| `bytes_per_kb` | `1_024.0` | 1 KB = 1024 bytes |
| `bytes_per_mb` | `1_048_576.0` | 1 MB = 1024 * 1024 bytes |
| `bytes_per_gb` | `1_073_741_824.0` | 1 GB = 1024 * 1024 * 1024 bytes |

### 使用例

```rust
use crate::config::app::APP_CONFIG;

// ファイルサイズチェック
if file_size > APP_CONFIG.max_file_size {
    return Err(DomainError::FileTooLarge {
        size: file_size,
        max: APP_CONFIG.max_file_size,
    });
}

// タイムアウト設定
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(APP_CONFIG.timeout_seconds))
    .build()?;

// ファイルサイズ表示
let size_mb = file_size as f64 / APP_CONFIG.bytes_per_mb;
println!("File size: {:.2} MB", size_mb);
```

### 設計上の制約

**含めるべきもの:**
- アプリケーション全体で共通の値
- 変更頻度が低い値
- コンパイル時に確定できる値

**含めないもの:**
- ユーザーごとに異なる値（認証情報など）
- 実行時に変更する必要がある値
- プランやAPIレスポンスに依存する値（例: 動画上限数）

## UserConfig（動的設定）

### 設定ファイルパス

プラットフォームごとに標準的なパスを使用（`dirs`クレート利用）：

| OS | パス |
|----|------|
| Windows | `%APPDATA%\vidyeet\config.toml` |
| macOS | `~/Library/Application Support/vidyeet/config.toml` |
| Linux | `~/.config/vidyeet/config.toml` |

**パーミッション（Unix系）:**
- ファイル: `0600` (rw-------)
- ディレクトリ: `0700` (rwx------)

### 設定ファイル形式（TOML）

```toml
# ユーザー設定
default_title = "My Video"
auto_copy_url = true
show_notification = true

# 認証情報（ログイン後に自動追加）
[auth]
token_id = "your-access-token-id"
token_secret = "your-access-token-secret"
```

### 実装

```rust
// src/config/user.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfig {
    pub default_title: Option<String>,
    pub auto_copy_url: bool,
    pub show_notification: bool,
    pub auth: Option<AuthConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthConfig {
    pub token_id: String,
    pub token_secret: String,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            default_title: None,
            auto_copy_url: false,
            show_notification: false,
            auth: None,
        }
    }
}

impl UserConfig {
    /// 設定ファイルのパスを取得
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory")?;
        Ok(config_dir.join("vidyeet").join("config.toml"))
    }

    /// 設定ファイルを読み込む
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = std::fs::read_to_string(&path)
            .context("Failed to read config file")?;
        
        let config: UserConfig = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        // 認証情報の検証（Fail Fast）
        if let Some(auth) = &config.auth {
            if auth.token_id.is_empty() || auth.token_secret.is_empty() {
                return Err(ConfigError::InvalidToken(
                    "Token ID or Secret is empty".to_string()
                ).into());
            }
        }
        
        Ok(config)
    }

    /// 設定ファイルに保存
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        
        // ディレクトリ作成
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        // TOML形式でシリアライズ
        let toml_string = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        // ファイル書き込み
        std::fs::write(&path, toml_string)
            .context("Failed to write config file")?;
        
        // パーミッション設定（Unix系のみ）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600); // rw-------
            std::fs::set_permissions(&path, perms)?;
        }
        
        Ok(())
    }

    /// 設定ファイルが存在することを保証（存在しなければデフォルト作成）
    pub fn ensure_config_exists() -> Result<()> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            let config = Self::default();
            config.save()?;
        }
        
        Ok(())
    }

    /// 認証情報を設定
    pub fn set_auth(&mut self, token_id: String, token_secret: String) {
        self.auth = Some(AuthConfig {
            token_id,
            token_secret,
        });
    }

    /// 認証情報を削除
    pub fn clear_auth(&mut self) {
        self.auth = None;
    }

    /// 認証情報が存在するか確認
    pub fn has_auth(&self) -> bool {
        self.auth.is_some()
    }
}
```

### 設定項目の詳細

| 項目 | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `default_title` | `Option<String>` | `None` | アップロード時のデフォルトタイトル |
| `auto_copy_url` | `bool` | `false` | アップロード後にURLを自動コピー（将来機能） |
| `show_notification` | `bool` | `false` | デスクトップ通知を表示（将来機能） |
| `auth.token_id` | `String` | - | Mux Access Token ID |
| `auth.token_secret` | `String` | - | Mux Access Token Secret |

### 使用例

```rust
// 設定読込
let config = UserConfig::load()?;

// 認証情報の確認
if let Some(auth) = config.auth {
    let client = MuxClient::new(auth.token_id, auth.token_secret)?;
} else {
    return Err(ConfigError::TokenNotFound.into());
}

// 認証情報の保存
let mut config = UserConfig::load()?;
config.set_auth(token_id, token_secret);
config.save()?;

// 認証情報の削除
let mut config = UserConfig::load()?;
config.clear_auth();
config.save()?;
```

## 認証情報の管理

### HTTP Basic認証

Mux APIはHTTP Basic認証を使用します：

- **ユーザー名**: Access Token ID
- **パスワード**: Access Token Secret

```rust
// api/auth.rs
pub fn create_basic_auth_header(token_id: &str, token_secret: &str) -> String {
    let credentials = format!("{}:{}", token_id, token_secret);
    let encoded = base64::encode(credentials);
    format!("Basic {}", encoded)
}
```

### 認証情報の取得

Muxダッシュボードで取得：

1. [Mux Dashboard](https://dashboard.mux.com/) にログイン
2. **Settings → Access Tokens** へ移動
3. **Generate new token** をクリック
4. Token IDとSecretをコピー

**重要な注意事項:**

- Muxはシークレットキーのハッシュのみを保存（平文は保存しない）
- シークレットキーを紛失した場合は復元不可
- 漏洩した場合は即座に無効化が必要

### 認証フロー

```
[ユーザー]
    ↓ vidyeet login
[プレゼンテーション層] commands/login.rs
    ↓ Token ID & Secret 入力
[設定層] config/user.rs
    ↓ 認証情報を検証
[インフラ層] api/client.rs
    ↓ 認証テスト（GET /video/v1/assets）
    ↓ 成功
[設定層] config/user.rs
    ↓ config.toml に保存
    ↓ パーミッション設定（0600）
[ユーザー]
    ✓ ログイン完了
```

### セキュリティ対策

#### 1. ファイルパーミッション

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&path)?.permissions();
    perms.set_mode(0o600); // 所有者のみ読み書き可能
    std::fs::set_permissions(&path, perms)?;
}
```

#### 2. トークンマスキング

```rust
// presentation/output.rs
pub fn mask_token(token: &str) -> String {
    let len = APP_CONFIG.token_display_mask_length;
    if token.len() <= len * 2 {
        "*".repeat(token.len())
    } else {
        format!(
            "{}***{}",
            &token[..len],
            &token[token.len() - len..]
        )
    }
}

// 出力例: "abc***xyz"
```

#### 3. Gitignore

```gitignore
# .gitignore
config.toml
*.toml
!Cargo.toml
```

#### 4. ログ出力の制限

```rust
// エラーメッセージに認証情報を含めない
// 悪い例
eprintln!("Authentication failed with token: {}", token);

// 良い例
eprintln!("Authentication failed: Invalid token");
```

## 設定の検証

### Fail Fast原則

設定ファイル読み込み時に即座に検証：

```rust
impl UserConfig {
    pub fn load() -> Result<Self> {
        // ... ファイル読込 ...
        
        // 認証情報の検証
        if let Some(auth) = &config.auth {
            if auth.token_id.is_empty() {
                return Err(ConfigError::InvalidToken(
                    "Token ID is empty".to_string()
                ));
            }
            if auth.token_secret.is_empty() {
                return Err(ConfigError::InvalidToken(
                    "Token Secret is empty".to_string()
                ));
            }
        }
        
        Ok(config)
    }
}
```

### 検証項目

1. **TOML構文チェック**: `toml::from_str()` で自動検証
2. **認証情報の存在チェック**: 空文字列のチェック
3. **ファイルパーミッションチェック**: Unix系でのみ実施（将来実装）

## エラーハンドリング

### ConfigError定義

```rust
// config/error.rs
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Authentication token not found")]
    TokenNotFound,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Failed to parse configuration file: {0}")]
    ParseError(String),

    #[error("Configuration file error: {0}")]
    FileSystemError(String),
}

impl ConfigError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::TokenNotFound 
            | Self::InvalidToken(_) 
            | Self::ParseError(_) => ErrorSeverity::ConfigError,
            
            Self::FileSystemError(_) => ErrorSeverity::SystemError,
        }
    }

    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::TokenNotFound => Some(
                "Please run 'vidyeet login' to authenticate."
            ),
            Self::InvalidToken(_) => Some(
                "Your authentication token may have expired. Please run 'vidyeet login' again."
            ),
            Self::ParseError(_) => Some(
                "Your configuration file may be corrupted. Try deleting it and logging in again."
            ),
            Self::FileSystemError(_) => Some(
                "Check file permissions and available disk space."
            ),
        }
    }
}
```

## テスト戦略

### ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = UserConfig::default();
        assert_eq!(config.auto_copy_url, false);
        assert!(config.auth.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        // 保存
        let mut config = UserConfig::default();
        config.set_auth("test_id".to_string(), "test_secret".to_string());
        config.save_to_path(&config_path).unwrap();
        
        // 読込
        let loaded = UserConfig::load_from_path(&config_path).unwrap();
        assert!(loaded.has_auth());
        assert_eq!(loaded.auth.unwrap().token_id, "test_id");
    }

    #[test]
    fn test_invalid_token_validation() {
        let mut config = UserConfig::default();
        config.set_auth("".to_string(), "secret".to_string());
        
        let result = config.validate();
        assert!(result.is_err());
    }
}
```

## 将来の拡張

### 1. 環境変数サポート

```rust
impl UserConfig {
    pub fn load_with_env() -> Result<Self> {
        let mut config = Self::load()?;
        
        // 環境変数を優先
        if let Ok(token_id) = env::var("MUX_TOKEN_ID") {
            if let Ok(token_secret) = env::var("MUX_TOKEN_SECRET") {
                config.set_auth(token_id, token_secret);
            }
        }
        
        Ok(config)
    }
}
```

### 2. 複数プロファイル対応

```toml
[profiles.production]
token_id = "prod_token_id"
token_secret = "prod_token_secret"

[profiles.staging]
token_id = "staging_token_id"
token_secret = "staging_token_secret"
```

```bash
vidyeet --profile production upload video.mp4
```

### 3. OS Keyring統合

```rust
use keyring::Entry;

impl UserConfig {
    pub fn save_to_keyring(&self) -> Result<()> {
        if let Some(auth) = &self.auth {
            let entry = Entry::new("vidyeet", "mux_token")?;
            let json = serde_json::to_string(auth)?;
            entry.set_password(&json)?;
        }
        Ok(())
    }
    
    pub fn load_from_keyring() -> Result<Option<AuthConfig>> {
        let entry = Entry::new("vidyeet", "mux_token")?;
        match entry.get_password() {
            Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
            Err(_) => Ok(None),
        }
    }
}
```

### 4. 設定マイグレーション

```rust
impl UserConfig {
    pub fn migrate_from_v1(old_path: &Path) -> Result<Self> {
        // v1形式の設定ファイルを読み込み、v2形式に変換
        // ...
    }
}
```

## 参考資料

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html)
- [TOML Specification](https://toml.io/)
- [Secure Storage Best Practices - OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- [dirs crate documentation](https://docs.rs/dirs/)