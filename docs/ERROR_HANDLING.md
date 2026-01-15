# エラーハンドリング

## 概要

vidyeet-cliは、階層化されたエラー型と明確な終了コードによる体系的なエラーハンドリングを実装しています。このドキュメントでは、エラー処理の戦略、各エラー型の責務、終了コードの決定方法を説明します。

## エラー処理の原則

### 1. エラーの階層化

各アーキテクチャ層が独自のエラー型を持ち、責務を明確に分離：

```
DomainError   ← ビジネスルール違反
ConfigError   ← 設定・認証問題
InfraError    ← 外部通信エラー
    ↓
anyhow::Error ← アプリケーション層で統合
    ↓
ErrorSeverity ← 終了コード決定
```

### 2. Fail Fast原則

問題を早期に検出し、即座にエラーを返す：

```rust
// 良い例
pub fn validate_upload_file(path: &str) -> Result<FileInfo> {
    if !Path::new(path).exists() {
        return Err(DomainError::FileNotFound(path.to_string()));
    }
    // 続きの処理...
}

// 悪い例（エラーを無視）
pub fn validate_upload_file(path: &str) -> Option<FileInfo> {
    if !Path::new(path).exists() {
        return None; // 原因が不明確
    }
    // ...
}
```

### 3. エラーコンテキストの追加

`anyhow::Context` でエラーチェーンに詳細情報を追加：

```rust
validator::validate_upload_file(path)
    .context("File validation failed")?;

client.create_direct_upload().await
    .context("Failed to create Direct Upload")?;
```

## 終了コード

### 終了コード一覧

| コード | 分類 | 意味 | 例 |
|-------|------|------|---|
| 0 | 成功 | 処理が正常に完了 | - |
| 1 | ユーザーエラー | ユーザーの入力に問題 | ファイル不正、形式無効 |
| 2 | 設定エラー | システム設定に問題 | トークン無効、設定破損 |
| 3 | システムエラー | 外部要因のエラー | ネットワーク障害、I/O障害 |

### ErrorSeverity型

```rust
// src/error_severity.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    UserError,    // Exit Code: 1
    ConfigError,  // Exit Code: 2
    SystemError,  // Exit Code: 3
}

impl ErrorSeverity {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::UserError => 1,
            Self::ConfigError => 2,
            Self::SystemError => 3,
        }
    }
}
```

**設計判断:**
- 独立モジュールとして全層から参照可能
- 依存方向の例外として許可（終了コード決定のみの責務）

## エラー型の詳細

### 1. DomainError（ドメイン層）

**責務:** ビジネスルール違反の表現

```rust
// src/domain/error.rs
#[derive(thiserror::Error, Debug)]
pub enum DomainError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("File size {size} bytes exceeds maximum allowed size of {max} bytes")]
    FileTooLarge { size: u64, max: u64 },

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid file: {0}")]
    InvalidFile(String),
}

impl DomainError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::FileNotFound(_) 
            | Self::FileTooLarge { .. } 
            | Self::UnsupportedFormat(_) 
            | Self::InvalidFile(_) => ErrorSeverity::UserError,
        }
    }

    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::FileNotFound(_) => Some(
                "Check that the file path is correct and the file exists."
            ),
            Self::FileTooLarge { max, .. } => Some(
                &format!("Maximum file size is {} GB. Please use a smaller file.", 
                         max / 1_073_741_824)
            ),
            Self::UnsupportedFormat(_) => Some(
                "Supported formats: mp4, mov, avi, wmv, flv, mkv, webm"
            ),
            Self::InvalidFile(_) => Some(
                "The file may be corrupted or not a valid video file."
            ),
        }
    }
}
```

**使用例:**

```rust
// domain/validator.rs
pub fn validate_upload_file(path: &str) -> Result<FileInfo> {
    let path_obj = Path::new(path);
    
    if !path_obj.exists() {
        return Err(DomainError::FileNotFound(path.to_string()).into());
    }
    
    let metadata = std::fs::metadata(path)
        .context("Failed to read file metadata")?;
    
    let size = metadata.len();
    if size > APP_CONFIG.max_file_size {
        return Err(DomainError::FileTooLarge {
            size,
            max: APP_CONFIG.max_file_size,
        }.into());
    }
    
    // 拡張子チェック
    let extension = path_obj.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    if !APP_CONFIG.supported_formats.contains(&extension) {
        return Err(DomainError::UnsupportedFormat(extension.to_string()).into());
    }
    
    Ok(FileInfo { path: path.to_string(), size, format: extension.to_string() })
}
```

### 2. ConfigError（設定層）

**責務:** 設定ファイル、認証情報の問題を表現

```rust
// src/config/error.rs
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

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl ConfigError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::TokenNotFound 
            | Self::InvalidToken(_) 
            | Self::ParseError(_) 
            | Self::InvalidConfig(_) => ErrorSeverity::ConfigError,
            
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
            Self::InvalidConfig(_) => Some(
                "Check your configuration file for syntax errors."
            ),
        }
    }
}
```

**使用例:**

```rust
// config/user.rs
impl UserConfig {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        
        if !path.exists() {
            return Err(ConfigError::TokenNotFound.into());
        }
        
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileSystemError(e.to_string()))?;
        
        let config: UserConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        // 認証情報の検証
        if let Some(auth) = &config.auth {
            if auth.token_id.is_empty() || auth.token_secret.is_empty() {
                return Err(ConfigError::InvalidToken(
                    "Token ID or Secret is empty".to_string()
                ).into());
            }
        }
        
        Ok(config)
    }
}
```

### 3. InfraError（インフラ層）

**責務:** 外部システム（Mux API、ネットワーク）とのやり取りでのエラー

```rust
// src/api/error.rs
#[derive(thiserror::Error, Debug)]
pub enum InfraError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("API error (HTTP {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Request timeout")]
    Timeout,

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    #[error("Upload failed: {0}")]
    UploadFailed(String),
}

impl InfraError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::AuthenticationFailed(_) => ErrorSeverity::ConfigError,
            
            Self::NetworkError(_) 
            | Self::ApiError { .. } 
            | Self::Timeout 
            | Self::InvalidResponse(_) 
            | Self::UploadFailed(_) => ErrorSeverity::SystemError,
        }
    }
}
```

**使用例:**

```rust
// api/client.rs
impl MuxClient {
    pub async fn create_direct_upload(&self) -> Result<DirectUpload> {
        let response = self.client
            .post(&format!("{}/video/v1/uploads", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&json!({
                "cors_origin": "*",
                "new_asset_settings": {
                    "playback_policy": ["public"]
                }
            }))
            .timeout(Duration::from_secs(APP_CONFIG.timeout_seconds))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    InfraError::Timeout
                } else {
                    InfraError::NetworkError(e.to_string())
                }
            })?;
        
        if response.status() == 401 {
            return Err(InfraError::AuthenticationFailed(
                "Invalid token".to_string()
            ).into());
        }
        
        if !response.status().is_success() {
            return Err(InfraError::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            }.into());
        }
        
        let data: serde_json::Value = response.json().await
            .map_err(|e| InfraError::InvalidResponse(e.to_string()))?;
        
        // レスポンスをパース...
        Ok(upload)
    }
}
```

## エラーハンドリングフロー

### 1. エラー発生からユーザー表示まで

```
[各層でエラー発生]
    ↓
DomainError / ConfigError / InfraError
    ↓ Into<anyhow::Error>
    
[アプリケーション層]
    ↓ .context("追加情報")
    
anyhow::Error (エラーチェーン)
    ↓
    
[プレゼンテーション層] main.rs::handle_error()
    ↓
1. extract_error_info() でチェーン走査
    - 最初の定義エラーを発見
    - severity() で終了コード取得
    - hint() でヒント取得
    ↓
2. 出力
    - --machine: JSON形式でstdoutに出力
    - 通常: 人間向けにstderrに出力
    ↓
3. std::process::exit(exit_code)
```

### 2. extract_error_info() の実装

```rust
// src/main.rs
fn extract_error_info(error: &anyhow::Error) -> (i32, Option<String>) {
    // エラーチェーン全体を一度走査
    for cause in error.chain() {
        // DomainError の場合
        if let Some(domain_err) = cause.downcast_ref::<DomainError>() {
            let severity = domain_err.severity();
            let hint = domain_err.hint().map(|s| s.to_string());
            return (severity.exit_code(), hint);
        }

        // ConfigError の場合
        if let Some(config_err) = cause.downcast_ref::<ConfigError>() {
            let severity = config_err.severity();
            let hint = config_err.hint().map(|s| s.to_string());
            return (severity.exit_code(), hint);
        }

        // InfraError の場合
        if let Some(infra_err) = cause.downcast_ref::<InfraError>() {
            let severity = infra_err.severity();
            return (severity.exit_code(), None);
        }
    }

    // 不明なエラーの場合はデフォルトの終了コード
    (1, None)
}
```

**設計の利点:**
- 一度のチェーン走査で全情報を取得（効率的）
- 型判定の重複排除
- エラー型側に分類責務を委譲（各層が独立）

## エラー出力フォーマット

### 人間向け出力（stderr）

```
Error: Upload command failed

Caused by:
  1: File validation failed
  2: File not found: video.mp4

Hint: Check that the file path is correct and the file exists.
```

### 機械向け出力（stdout、--machineフラグ）

```json
{
  "success": false,
  "error": {
    "message": "Upload command failed",
    "exit_code": 1,
    "hint": "Check that the file path is correct and the file exists."
  }
}
```

## エラーハンドリングのベストプラクティス

### 1. エラーを無視しない

```rust
// 悪い例
let _ = file.write_all(data); // エラーを無視

// 良い例
file.write_all(data)
    .context("Failed to write data")?;
```

### 2. 適切なエラー型を選択

```rust
// ユーザーの入力ミス → DomainError
if size > MAX_SIZE {
    return Err(DomainError::FileTooLarge { size, max: MAX_SIZE });
}

// 設定の問題 → ConfigError
if config.auth.is_none() {
    return Err(ConfigError::TokenNotFound);
}

// ネットワークの問題 → InfraError
if response.status() != 200 {
    return Err(InfraError::ApiError { status, message });
}
```

### 3. コンテキストを追加

```rust
// 悪い例
let config = UserConfig::load()?; // どこで失敗したか不明確

// 良い例
let config = UserConfig::load()
    .context("Failed to load user configuration")?;
```

### 4. エラーメッセージに機密情報を含めない

```rust
// 悪い例
Err(format!("Authentication failed with token: {}", token))

// 良い例
Err(InfraError::AuthenticationFailed("Invalid token".to_string()))
```

## テスト戦略

### ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error_severity() {
        let err = DomainError::FileNotFound("test.mp4".to_string());
        assert_eq!(err.severity(), ErrorSeverity::UserError);
        assert_eq!(err.severity().exit_code(), 1);
    }

    #[test]
    fn test_config_error_hint() {
        let err = ConfigError::TokenNotFound;
        assert!(err.hint().unwrap().contains("vidyeet login"));
    }

    #[test]
    fn test_extract_error_info_domain_error() {
        let err: anyhow::Error = DomainError::FileNotFound("test.mp4".to_string()).into();
        let err = err.context("Command failed");
        
        let (exit_code, hint) = extract_error_info(&err);
        assert_eq!(exit_code, 1);
        assert!(hint.is_some());
    }
}
```

### 統合テスト

```rust
#[tokio::test]
async fn test_upload_with_invalid_file() {
    let result = commands::upload::execute("nonexistent.mp4", None).await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    // エラーチェーンを検証
    let domain_err = err
        .chain()
        .find_map(|e| e.downcast_ref::<DomainError>())
        .expect("Should contain DomainError");
    
    match domain_err {
        DomainError::FileNotFound(_) => {}, // OK
        _ => panic!("Expected FileNotFound error"),
    }
}
```

## 将来の拡張

### 1. エラーリトライ機能

```rust
pub async fn retry_on_network_error<F, T>(
    mut f: F,
    max_retries: u32,
) -> Result<T>
where
    F: FnMut() -> Future<Output = Result<T>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if let Some(infra_err) = e.downcast_ref::<InfraError>() {
                    if matches!(infra_err, InfraError::NetworkError(_)) 
                        && attempts < max_retries {
                        attempts += 1;
                        tokio::time::sleep(Duration::from_secs(2_u64.pow(attempts))).await;
                        continue;
                    }
                }
                return Err(e);
            }
        }
    }
}
```

### 2. 構造化ログ

```rust
use tracing::{error, warn, info};

error!(
    error_type = ?domain_err,
    exit_code = exit_code,
    "Upload failed"
);
```

### 3. エラーメトリクス収集

```rust
fn record_error_metric(error_type: &str, exit_code: i32) {
    // メトリクス収集サービスに送信
}
```

## 参考資料

- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [anyhow documentation](https://docs.rs/anyhow/)
- [thiserror documentation](https://docs.rs/thiserror/)
- [Error Handling in Rust - Andrew Gallant](https://blog.burntsushi.net/rust-error-handling/)