/// Config層のエラー定義
///
/// 設定ファイルの読み込み、書き込み、パースに関するエラーを構造化して定義。
/// 外部エラー(std::io::Error, toml::de::Error等)の発信元を適切に保持する。
use crate::error_severity::ErrorSeverity;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    /// 設定ディレクトリの取得失敗
    #[error("failed to get config directory: {message}")]
    DirectoryNotFound { message: String },

    /// ファイルシステムエラー
    #[error("file system error: {context}")]
    FileSystem {
        context: String,
        #[source]
        source: io::Error,
    },

    /// 設定ファイルのパースエラー
    #[error("failed to parse config file: {context}")]
    ParseError {
        context: String,
        #[source]
        source: toml::de::Error,
    },

    /// 設定ファイルのシリアライズエラー
    #[error("failed to serialize config: {context}")]
    SerializeError {
        context: String,
        #[source]
        source: toml::ser::Error,
    },

    /// 設定の検証エラー
    #[error("configuration validation failed: {message}")]
    ValidationError { message: String },

    /// トークンが見つからない
    #[error("token not found: {message}")]
    TokenNotFound { message: String },
}

impl ConfigError {
    /// エラーの深刻度を返す
    ///
    /// 終了コードの決定に使用できる
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::DirectoryNotFound { .. } => ErrorSeverity::ConfigError,
            Self::FileSystem { .. } => ErrorSeverity::SystemError,
            Self::ParseError { .. } => ErrorSeverity::ConfigError,
            Self::SerializeError { .. } => ErrorSeverity::ConfigError,
            Self::ValidationError { .. } => ErrorSeverity::ConfigError,
            Self::TokenNotFound { .. } => ErrorSeverity::ConfigError,
        }
    }

    /// ユーザー向けのヒントメッセージを返す
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::DirectoryNotFound { .. } => {
                Some("Unable to locate the configuration directory. Check your system environment.")
            }
            Self::FileSystem { .. } => {
                Some("Check file permissions and ensure the config directory is writable.")
            }
            Self::ParseError { .. } => {
                Some("The config file may be corrupted. Try deleting it to regenerate defaults.")
            }
            Self::SerializeError { .. } => {
                Some("Failed to save configuration. Check for invalid characters or formatting.")
            }
            Self::ValidationError { .. } => {
                Some("Review your configuration settings and ensure all required fields are valid.")
            }
            Self::TokenNotFound { .. } => {
                Some("Please run 'vidyeet login' to authenticate with api.video.")
            }
        }
    }
}
