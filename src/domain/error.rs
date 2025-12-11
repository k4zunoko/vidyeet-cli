/// ドメイン層のエラー定義
///
/// ビジネスロジックに関連するエラーを構造化して定義。
/// 外部クレートのエラーは含まず、純粋にドメインの制約違反を表現する。
use crate::error_severity::ErrorSeverity;
use thiserror::Error;

#[derive(Error, Debug)]
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
}

impl DomainError {
    /// ファイルが見つからないエラーを生成
    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// 無効なファイル形式エラーを生成
    pub fn invalid_format(
        path: impl Into<String>,
        supported_formats: &[&str],
        found: impl Into<String>,
    ) -> Self {
        Self::InvalidFormat {
            path: path.into(),
            expected: format!("one of: {}", supported_formats.join(", ")),
            found: found.into(),
        }
    }

    /// ファイルが空エラーを生成
    pub fn empty_file(path: impl Into<String>) -> Self {
        Self::EmptyFile { path: path.into() }
    }

    /// ディレクトリ指定エラーを生成
    pub fn not_a_file(path: impl Into<String>) -> Self {
        Self::NotAFile { path: path.into() }
    }

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
        }
    }

    /// ユーザー向けのヒントメッセージを返す
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::FileNotFound { .. } => {
                Some("Please check the file path and ensure the file exists.")
            }
            Self::InvalidFormat { .. } => Some("Supported formats: mp4, mov, avi, mkv, webm"),
            Self::FileTooLarge { .. } => Some("Try compressing the video or use a smaller file."),
            Self::EmptyFile { .. } => Some("The file appears to be empty or corrupted."),
            Self::NotAFile { .. } => Some("Please specify a file, not a directory."),
        }
    }
}
