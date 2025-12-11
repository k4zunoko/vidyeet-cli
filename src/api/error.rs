use std::io;
/// インフラ層のエラー定義
///
/// 外部システム（ファイルシステム、ネットワーク、API）との
/// やり取りで発生するエラーを構造化して定義。
/// #[from] / #[source] を使って原因連鎖を保持する。
use crate::error_severity::ErrorSeverity;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InfraError {
    /// ネットワークエラー
    #[error("network error: {message}")]
    Network {
        message: String,
    },

    /// API通信エラー
    #[error("API error: {endpoint} - {message}")]
    Api {
        endpoint: String,
        message: String,
        status_code: Option<u16>,
    },

    /// タイムアウトエラー
    #[error("operation timed out: {operation}")]
    Timeout { operation: String },

    /// その他のI/Oエラー
    #[error("I/O error")]
    Io(#[from] io::Error),
}

impl InfraError {
    /// ネットワークエラーを作成
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
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

    /// タイムアウトエラーを作成
    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// エラーの深刻度を返す
    pub fn severity(&self) -> ErrorSeverity {
        ErrorSeverity::SystemError
    }

    /// ユーザー向けのヒントメッセージを返す
    #[allow(dead_code)]
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::Network { .. } => Some("Check your internet connection and try again."),
            Self::Api { .. } => Some("Check your API credentials and permissions."),
            Self::Timeout { .. } => Some("The operation took too long. Try again or check your connection."),
            Self::Io(_) => Some("An I/O error occurred. Check file permissions and disk space."),
        }
    }
}
