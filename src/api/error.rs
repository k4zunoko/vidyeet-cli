/// インフラ層のエラー定義
/// 
/// 外部システム（ファイルシステム、ネットワーク、API）との
/// やり取りで発生するエラーを構造化して定義。
/// #[from] / #[source] を使って原因連鎖を保持する。

use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum InfraError {
    /// ファイルシステムエラー
    #[error("file system error: {context}")]
    FileSystem {
        context: String,
        #[source]
        source: io::Error,
    },

    /// ネットワークエラー（将来実装）
    #[error("network error: {message}")]
    Network {
        message: String,
        // #[from]
        // source: reqwest::Error,
    },

    /// API通信エラー（将来実装）
    #[error("API error: {endpoint} - {message}")]
    Api {
        endpoint: String,
        message: String,
        status_code: Option<u16>,
    },

    /// 設定ファイルエラー（将来実装）
    #[error("configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// タイムアウトエラー
    #[error("operation timed out: {operation}")]
    Timeout { operation: String },

    /// その他のI/Oエラー
    #[error("I/O error")]
    Io(#[from] io::Error),
}

impl InfraError {
    /// ファイルシステムエラーを作成
    pub fn file_system(context: impl Into<String>, source: io::Error) -> Self {
        Self::FileSystem {
            context: context.into(),
            source,
        }
    }

    /// APIエラーを作成
    pub fn api(
        endpoint: impl Into<String>,
        message: impl Into<String>,
        status_code: Option<u16>,
    ) -> Self {
        Self::Api {
            endpoint: endpoint.into(),
            message: message.into(),
            status_code,
        }
    }

    /// ネットワークエラーを作成
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// リトライ可能かどうかを判定
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network { .. } => true,
            Self::Timeout { .. } => true,
            Self::Api { status_code, .. } => {
                // 5xx系のステータスコードはリトライ可能
                status_code.map(|code| code >= 500 && code < 600).unwrap_or(false)
            }
            _ => false,
        }
    }

    /// エラーの深刻度を返す
    pub fn severity(&self) -> crate::domain::error::ErrorSeverity {
        use crate::domain::error::ErrorSeverity;
        
        match self {
            Self::Config { .. } => ErrorSeverity::ConfigError,
            Self::FileSystem { .. } | Self::Io(_) => ErrorSeverity::SystemError,
            Self::Network { .. } | Self::Timeout { .. } | Self::Api { .. } => {
                ErrorSeverity::SystemError
            }
        }
    }
}
