/// ドメインサービス: ファイルバリデーション
/// 
/// アップロード対象のファイルを検証する。
/// ドメイン層の責務として、ビジネスルールを適用する。

use std::path::Path;
use crate::domain::error::DomainError;

/// サポートされている動画形式
const SUPPORTED_FORMATS: &[&str] = &["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv"];

/// 最大ファイルサイズ（バイト）: 2GB
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// ファイルのバリデーション結果
pub struct ValidationResult {
    pub path: String,
    pub size: u64,
    pub extension: String,
}

/// アップロード対象のファイルをバリデーションする
/// 
/// # エラー
/// - ファイルが存在しない
/// - ディレクトリが指定された
/// - ファイルが空
/// - サポートされていない形式
/// - ファイルサイズが制限を超過
pub fn validate_upload_file(file_path: &str) -> Result<ValidationResult, DomainError> {
    let path = Path::new(file_path);
    
    // 存在確認
    if !path.exists() {
        return Err(DomainError::FileNotFound {
            path: file_path.to_string(),
        });
    }
    
    // メタデータ取得（InfraErrorに変換せず、ここではDomainErrorとして扱う）
    let metadata = std::fs::metadata(path).map_err(|_| DomainError::FileNotFound {
        path: file_path.to_string(),
    })?;
    
    // ディレクトリチェック
    if metadata.is_dir() {
        return Err(DomainError::NotAFile {
            path: file_path.to_string(),
        });
    }
    
    // 空ファイルチェック
    let size = metadata.len();
    if size == 0 {
        return Err(DomainError::EmptyFile {
            path: file_path.to_string(),
        });
    }
    
    // ファイルサイズチェック
    if size > MAX_FILE_SIZE {
        return Err(DomainError::FileTooLarge {
            size,
            max: MAX_FILE_SIZE,
        });
    }
    
    // 拡張子チェック
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| DomainError::InvalidFormat {
            path: file_path.to_string(),
            expected: format!("one of: {}", SUPPORTED_FORMATS.join(", ")),
            found: "no extension".to_string(),
        })?;
    
    if !SUPPORTED_FORMATS.contains(&extension.as_str()) {
        return Err(DomainError::InvalidFormat {
            path: file_path.to_string(),
            expected: format!("one of: {}", SUPPORTED_FORMATS.join(", ")),
            found: extension.clone(),
        });
    }
    
    Ok(ValidationResult {
        path: file_path.to_string(),
        size,
        extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supported_formats() {
        assert!(SUPPORTED_FORMATS.contains(&"mp4"));
        assert!(SUPPORTED_FORMATS.contains(&"mov"));
    }
}
