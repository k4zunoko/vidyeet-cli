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

    /// エラーの深刻度を返す
    pub fn severity(&self) -> ErrorSeverity {
        ErrorSeverity::SystemError
    }
}
