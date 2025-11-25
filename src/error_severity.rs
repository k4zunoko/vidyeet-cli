//! プレゼンテーション層が使用するエラー深刻度
//!
//! このモジュールはアーキテクチャの最外層（プレゼンテーション層）に属し、
//! 終了コードの決定に使用される。
//!
//! **依存方向の原則:**
//! - 内側層（domain, infra, config）はこのモジュールに依存してOK
//! - このモジュールは他のモジュールに依存しない（独立）

use std::fmt;

/// エラーの深刻度と対応する終了コード
///
/// アーキテクチャ内の全レイヤーが共有する、最も抽象的なエラー分類。
/// **設計原則**: このモジュールは「プレゼンテーション層の関心」を表現し、
/// 他の層から独立して存在する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorSeverity {
    /// ユーザーの入力エラー
    ///
    /// ファイルが見つからない、形式が無効など、ユーザーが直す可能性がある。
    ///
    /// **Exit Code: 1**
    UserError,

    /// 設定エラー
    ///
    /// API キーが無効、設定ファイルが破損しているなど、
    /// システム設定に問題がある。
    ///
    /// **Exit Code: 2**
    ConfigError,

    /// システムエラー
    ///
    /// ネットワークエラー、ファイルシステム障害など、
    /// ユーザーが直せない外部要因。
    ///
    /// **Exit Code: 3**
    SystemError,
}

impl ErrorSeverity {
    /// 対応する Unix 終了コードを返す
    pub fn exit_code(self) -> i32 {
        match self {
            Self::UserError => 1,
            Self::ConfigError => 2,
            Self::SystemError => 3,
        }
    }
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserError => write!(f, "user error"),
            Self::ConfigError => write!(f, "configuration error"),
            Self::SystemError => write!(f, "system error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes() {
        assert_eq!(ErrorSeverity::UserError.exit_code(), 1);
        assert_eq!(ErrorSeverity::ConfigError.exit_code(), 2);
        assert_eq!(ErrorSeverity::SystemError.exit_code(), 3);
    }

    #[test]
    fn test_display() {
        assert_eq!(ErrorSeverity::UserError.to_string(), "user error");
        assert_eq!(
            ErrorSeverity::ConfigError.to_string(),
            "configuration error"
        );
        assert_eq!(ErrorSeverity::SystemError.to_string(), "system error");
    }

    #[test]
    fn test_equality() {
        assert_eq!(ErrorSeverity::UserError, ErrorSeverity::UserError);
        assert_ne!(ErrorSeverity::UserError, ErrorSeverity::ConfigError);
    }
}
