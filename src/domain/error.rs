/// ドメイン層のエラー定義
/// 
/// ビジネスロジックに関連するエラーを構造化して定義。
/// 外部クレートのエラーは含まず、純粋にドメインの制約違反を表現する。

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum DomainError {
    /// ファイルが見つからない
    #[error("file not found: {path}")]
    FileNotFound { path: String },

    /// ファイル形式が無効
    #[error("invalid file format: {path} (expected: {expected}, found: {found})")]
    InvalidFormat {
        path: String,
        expected: String,
        found: String,
    },

    /// ファイルサイズが制限を超過
    #[error("file too large: {size} bytes (maximum allowed: {max} bytes)")]
    FileTooLarge { size: u64, max: u64 },

    /// ファイルが空
    #[error("file is empty: {path}")]
    EmptyFile { path: String },

    /// ディレクトリが指定された（ファイルが期待される場所）
    #[error("'{path}' is a directory, not a file")]
    NotAFile { path: String },

    /// 認証エラー
    #[error("authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    /// バリデーションエラー
    #[error("validation failed: {message}")]
    ValidationFailed { message: String },
}

impl DomainError {
    /// エラーの深刻度を返す
    /// 
    /// 終了コードの決定に使用できる
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::FileNotFound { .. } => ErrorSeverity::UserError,
            Self::InvalidFormat { .. } => ErrorSeverity::UserError,
            Self::FileTooLarge { .. } => ErrorSeverity::UserError,
            Self::EmptyFile { .. } => ErrorSeverity::UserError,
            Self::NotAFile { .. } => ErrorSeverity::UserError,
            Self::AuthenticationFailed { .. } => ErrorSeverity::ConfigError,
            Self::ValidationFailed { .. } => ErrorSeverity::UserError,
        }
    }

    /// ユーザー向けのヒントメッセージを返す
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::FileNotFound { .. } => {
                Some("Please check the file path and ensure the file exists.")
            }
            Self::InvalidFormat { .. } => {
                Some("Supported formats: mp4, mov, avi, mkv, webm")
            }
            Self::FileTooLarge { .. } => {
                Some("Try compressing the video or use a smaller file.")
            }
            Self::EmptyFile { .. } => Some("The file appears to be empty or corrupted."),
            Self::NotAFile { .. } => Some("Please specify a file, not a directory."),
            Self::AuthenticationFailed { .. } => {
                Some("Check your API credentials in the configuration.")
            }
            Self::ValidationFailed { .. } => None,
        }
    }
}

/// エラーの深刻度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// ユーザーの入力エラー（exit code: 1）
    UserError,
    /// 設定エラー（exit code: 2）
    ConfigError,
    /// システムエラー（exit code: 3）
    SystemError,
}

impl ErrorSeverity {
    /// 終了コードを返す
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::UserError => 1,
            Self::ConfigError => 2,
            Self::SystemError => 3,
        }
    }
}
